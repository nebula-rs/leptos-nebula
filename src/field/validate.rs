//! Lógica pura da validação do `Field` — **sem DOM e sem Leptos**.
//!
//! Porta do `use-validate.js` do Quasar: regras síncronas e assíncronas, primeira regra que
//! falha define a mensagem, validação superada (o resultado de uma validação antiga não
//! sobrescreve a nova) e `lazy_rules`. O hook [`use_validate`](super::use_validate) só liga
//! isto aos sinais do Leptos.
//!
//! Especificação: `docs/ui/forms.md#validacao`.

use std::rc::Rc;

use futures::future::{FutureExt, LocalBoxFuture, join_all};

/// Resultado de uma regra: `Ok(())` válido; `Err(mensagem)` inválido (mensagem vazia = inválido
/// sem texto, o `false` do Quasar).
pub type RuleResult = Result<(), String>;

/// Quando validar numa mudança de valor e no blur (`lazy-rules` do Quasar).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LazyRules {
    /// Valida a cada mudança de valor e no blur (`lazy-rules: false`). Padrão.
    #[default]
    Off,
    /// Valida só no blur; depois que há erro visível, revalida a cada mudança até o erro
    /// sumir (`lazy-rules: true`).
    OnBlur,
    /// Nunca valida sozinho: só por `validate()` (ex.: pelo `Form`) (`lazy-rules: "ondemand"`).
    OnDemand,
}

/// Uma regra de validação sobre `&T`. Síncrona ([`Rule::new`]) ou assíncrona
/// ([`Rule::new_async`]).
pub struct Rule<T>(Rc<dyn Fn(&T) -> Outcome>);

impl<T> Clone for Rule<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

enum Outcome {
    Now(RuleResult),
    Later(LocalBoxFuture<'static, RuleResult>),
}

impl<T: 'static> Rule<T> {
    /// Regra síncrona: `Rule::new(|v: &String| if v.is_empty() { Err("Obrigatório".into()) } else { Ok(()) })`.
    pub fn new(f: impl Fn(&T) -> RuleResult + 'static) -> Self {
        Self(Rc::new(move |v| Outcome::Now(f(v))))
    }

    /// Regra assíncrona. A future não pode emprestar o valor: clone o que precisar antes do
    /// `async move`. Pode chamar uma `#[server] fn`.
    pub fn new_async<F>(f: impl Fn(&T) -> F + 'static) -> Self
    where
        F: std::future::Future<Output = RuleResult> + 'static,
    {
        Self(Rc::new(move |v| Outcome::Later(f(v).boxed_local())))
    }
}

/// Resultado de rodar o conjunto de regras.
pub(crate) enum Run {
    /// Resolvido sem esperar (todas síncronas, ou uma síncrona falhou).
    Now(RuleResult),
    /// Há regras assíncronas pendentes.
    Later(BoxedLater),
}

/// Future das regras assíncronas pendentes.
pub(crate) type BoxedLater = LocalBoxFuture<'static, RuleResult>;

/// Roda as regras em ordem. Uma regra síncrona que falha resolve na hora (mesmo havendo
/// assíncronas antes dela, como no Quasar); as assíncronas rodam juntas e a primeira que falha
/// (na ordem das regras) define a mensagem.
pub(crate) fn run_rules<T>(rules: &[Rule<T>], value: &T) -> Run {
    let mut pending = Vec::new();
    for rule in rules {
        match (rule.0)(value) {
            Outcome::Now(Err(msg)) => return Run::Now(Err(msg)),
            Outcome::Now(Ok(())) => {}
            Outcome::Later(fut) => pending.push(fut),
        }
    }
    if pending.is_empty() {
        return Run::Now(Ok(()));
    }
    Run::Later(
        async move {
            join_all(pending)
                .await
                .into_iter()
                .find(Result::is_err)
                .unwrap_or(Ok(()))
        }
        .boxed_local(),
    )
}

/// Há erro visível: `error` externo `Some(true)` ou erro interno (como no Quasar, `Some(false)`
/// não suprime o erro interno).
pub(crate) fn has_error(external: Option<bool>, inner: bool) -> bool {
    external == Some(true) || inner
}

/// A mensagem externa não vazia vence a interna.
pub(crate) fn pick_message(external: &str, inner: Option<&str>) -> Option<String> {
    if !external.is_empty() {
        Some(external.to_owned())
    } else {
        inner.map(str::to_owned)
    }
}

/// Máquina de estados da validação de um campo (sem timers, sem valor, sem regras).
///
/// Quem a usa passa `disabled` a cada chamada (muda no tempo) e executa o que ela decide.
#[derive(Clone, Debug)]
pub(crate) struct ValidationState {
    lazy: LazyRules,
    has_rules: bool,
    /// Erro produzido pelas regras.
    pub(crate) inner_error: bool,
    /// Mensagem do erro interno.
    pub(crate) message: Option<String>,
    /// Há validação assíncrona em andamento.
    pub(crate) loading: bool,
    index: u64,
    /// Um blur chegou durante uma validação assíncrona; vale quando ela terminar.
    pending_blur: bool,
}

impl ValidationState {
    pub(crate) fn new(lazy: LazyRules, has_rules: bool) -> Self {
        Self {
            lazy,
            has_rules,
            inner_error: false,
            message: None,
            loading: false,
            index: 0,
            pending_blur: false,
        }
    }

    /// Pode validar automaticamente: habilitado, com regras e sem outra validação em curso (o
    /// `canDebounceValidate` do Quasar — um submit por ENTER, por exemplo, já pode ter começado).
    pub(crate) fn can_auto(&self, disabled: bool) -> bool {
        !disabled && self.has_rules && !self.loading
    }

    /// O valor mudou: deve validar agora?
    pub(crate) fn value_changed(&self, disabled: bool) -> bool {
        self.can_auto(disabled)
            && match self.lazy {
                LazyRules::Off => true,
                LazyRules::OnBlur => self.inner_error,
                LazyRules::OnDemand => false,
            }
    }

    /// O campo perdeu o foco: deve validar agora? Com validação assíncrona em curso, guarda o
    /// blur para quando ela terminar.
    pub(crate) fn blurred(&mut self, disabled: bool) -> bool {
        if self.lazy == LazyRules::OnDemand {
            return false;
        }
        if self.can_auto(disabled) {
            true
        } else {
            if !disabled && self.has_rules && self.loading {
                self.pending_blur = true;
            }
            false
        }
    }

    /// Limpa tudo; supera qualquer validação em voo.
    pub(crate) fn reset(&mut self) {
        self.index += 1;
        self.loading = false;
        self.inner_error = false;
        self.message = None;
        self.pending_blur = false;
    }

    /// Começa uma validação. `None`: nada a validar (desabilitado ou sem regras → válido, sem
    /// tocar no estado).
    pub(crate) fn begin(&mut self, disabled: bool) -> Option<u64> {
        if disabled || !self.has_rules {
            return None;
        }
        self.index += 1;
        self.pending_blur = false;
        Some(self.index)
    }

    /// Aplica um veredito.
    fn update(&mut self, result: &RuleResult) {
        match result {
            Ok(()) => {
                self.inner_error = false;
                self.message = None;
            }
            Err(msg) => {
                self.inner_error = true;
                self.message = (!msg.is_empty()).then(|| msg.clone());
            }
        }
        self.loading = false;
    }

    /// Veredito síncrono da validação `index`.
    pub(crate) fn finish_now(&mut self, result: &RuleResult) {
        self.update(result);
    }

    /// A validação passou a esperar regras assíncronas.
    pub(crate) fn start_async(&mut self) {
        self.loading = true;
    }

    /// A validação assíncrona `index` terminou. Superada (`index` velho): ignorada, devolve
    /// `false`. Senão aplica o veredito e diz se deve revalidar porque o valor mudou durante a
    /// espera (e as regras do `lazy` permitem: sem erro e sem blur pendente, o `OnBlur` espera o
    /// próximo blur).
    pub(crate) fn settle(
        &mut self,
        index: u64,
        result: &RuleResult,
        value_changed: bool,
        disabled: bool,
    ) -> bool {
        if index != self.index {
            return false;
        }
        self.update(result);
        let blur_arrived = std::mem::take(&mut self.pending_blur);
        value_changed
            && self.can_auto(disabled)
            && self.lazy != LazyRules::OnDemand
            && (self.lazy == LazyRules::Off || self.inner_error || blur_arrived)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::oneshot;
    use futures::executor::block_on;
    use std::cell::RefCell;

    fn required() -> Rule<String> {
        Rule::new(|v: &String| {
            if v.is_empty() {
                Err("Required".into())
            } else {
                Ok(())
            }
        })
    }

    fn now(run: Run) -> RuleResult {
        match run {
            Run::Now(r) => r,
            Run::Later(_) => panic!("esperava resultado imediato"),
        }
    }

    fn later(run: Run) -> BoxedLater {
        match run {
            Run::Later(f) => f,
            Run::Now(_) => panic!("esperava future pendente"),
        }
    }

    // -- run_rules ---------------------------------------------------------------------------

    #[test]
    fn passes_when_every_rule_agrees() {
        let rules = vec![required(), Rule::new(|_: &String| Ok(()))];
        assert_eq!(now(run_rules(&rules, &"ok".to_string())), Ok(()));
    }

    #[test]
    fn no_rules_is_valid() {
        let rules: Vec<Rule<String>> = vec![];
        assert_eq!(now(run_rules(&rules, &String::new())), Ok(()));
    }

    #[test]
    fn hands_the_value_to_the_rules() {
        let seen = Rc::new(RefCell::new(None));
        let s = seen.clone();
        let rules = vec![Rule::new(move |v: &String| {
            *s.borrow_mut() = Some(v.clone());
            Ok(())
        })];
        run_rules(&rules, &"abc".to_string());
        assert_eq!(seen.borrow().as_deref(), Some("abc"));
    }

    #[test]
    fn stops_at_the_first_failing_rule() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mk = |n: &'static str, r: RuleResult| {
            let c = calls.clone();
            Rule::new(move |_: &String| {
                c.borrow_mut().push(n);
                r.clone()
            })
        };
        let rules = vec![
            mk("a", Ok(())),
            mk("b", Err("B".into())),
            mk("c", Err("C".into())),
        ];
        assert_eq!(now(run_rules(&rules, &String::new())), Err("B".into()));
        assert_eq!(*calls.borrow(), vec!["a", "b"]);
    }

    #[test]
    fn empty_message_is_an_error_without_text() {
        let rules = vec![Rule::new(|_: &String| Err(String::new()))];
        assert_eq!(now(run_rules(&rules, &String::new())), Err(String::new()));
    }

    #[test]
    fn reports_the_first_failing_among_async_rules() {
        let rules = vec![
            Rule::new_async(|_: &String| async { Ok(()) }),
            Rule::new_async(|_: &String| async { Err("first".to_string()) }),
            Rule::new_async(|_: &String| async { Err("second".to_string()) }),
        ];
        let fut = later(run_rules(&rules, &String::new()));
        assert_eq!(block_on(fut), Err("first".into()));
    }

    #[test]
    fn async_rules_all_ok_is_valid() {
        let rules = vec![Rule::new_async(|_: &String| async { Ok(()) })];
        assert_eq!(block_on(later(run_rules(&rules, &String::new()))), Ok(()));
    }

    #[test]
    fn a_failing_sync_rule_wins_immediately_over_an_earlier_async_one() {
        let rules = vec![
            Rule::new_async(|_: &String| async { Ok(()) }),
            Rule::new(|_: &String| Err("sync".to_string())),
        ];
        assert_eq!(now(run_rules(&rules, &String::new())), Err("sync".into()));
    }

    #[test]
    fn async_rule_receives_the_value_at_call_time() {
        let (tx, rx) = oneshot::channel::<()>();
        let tx = Rc::new(RefCell::new(Some(tx)));
        let rules = vec![Rule::new_async(move |v: &String| {
            let v = v.clone();
            let rx_done = tx.borrow_mut().take();
            async move {
                if let Some(t) = rx_done {
                    let _ = t.send(());
                }
                if v == "bad" {
                    Err("bad".to_string())
                } else {
                    Ok(())
                }
            }
        })];
        let fut = later(run_rules(&rules, &"bad".to_string()));
        assert_eq!(block_on(fut), Err("bad".into()));
        assert!(block_on(rx).is_ok());
    }

    // -- external error ----------------------------------------------------------------------

    #[test]
    fn external_error_true_or_inner_error() {
        assert!(!has_error(None, false));
        assert!(!has_error(Some(false), false));
        assert!(has_error(Some(true), false));
        assert!(has_error(None, true));
        assert!(has_error(Some(false), true));
    }

    #[test]
    fn external_message_wins_over_inner() {
        assert_eq!(pick_message("ext", Some("inner")), Some("ext".into()));
        assert_eq!(pick_message("", Some("inner")), Some("inner".into()));
        assert_eq!(pick_message("", None), None);
    }

    // -- state machine -----------------------------------------------------------------------

    fn state(lazy: LazyRules) -> ValidationState {
        ValidationState::new(lazy, true)
    }

    #[test]
    fn validates_a_model_change_when_not_lazy() {
        assert!(state(LazyRules::Off).value_changed(false));
    }

    #[test]
    fn lazy_holds_back_the_validation_on_change() {
        assert!(!state(LazyRules::OnBlur).value_changed(false));
        assert!(!state(LazyRules::OnDemand).value_changed(false));
    }

    #[test]
    fn lazy_validates_once_it_loses_focus() {
        let mut s = state(LazyRules::OnBlur);
        assert!(s.blurred(false));
    }

    #[test]
    fn lazy_revalidates_on_change_while_an_error_is_displayed() {
        let mut s = state(LazyRules::OnBlur);
        assert!(s.blurred(false));
        s.begin(false);
        s.finish_now(&Err("Required".into()));
        assert!(s.value_changed(false));
        s.begin(false);
        s.finish_now(&Ok(()));
        // sem erro, volta a ser preguiçoso até o próximo blur
        assert!(!s.value_changed(false));
    }

    #[test]
    fn error_from_a_form_submit_is_cleared_while_typing() {
        let mut s = state(LazyRules::OnBlur);
        s.begin(false);
        s.finish_now(&Err("Required".into()));
        assert!(s.inner_error);
        assert!(s.value_changed(false));
    }

    #[test]
    fn on_demand_never_auto_validates_even_with_error() {
        let mut s = state(LazyRules::OnDemand);
        s.begin(false);
        s.finish_now(&Err("Required".into()));
        assert!(!s.value_changed(false));
        assert!(!s.blurred(false));
    }

    #[test]
    fn skips_the_auto_validation_while_another_one_is_running() {
        let mut s = state(LazyRules::Off);
        s.begin(false);
        s.start_async();
        assert!(!s.value_changed(false));
    }

    #[test]
    fn a_disabled_field_ignores_changes_and_blur() {
        let mut s = state(LazyRules::Off);
        assert!(!s.value_changed(true));
        assert!(!s.blurred(true));
        assert_eq!(s.begin(true), None);
    }

    #[test]
    fn without_rules_nothing_validates() {
        let mut s = ValidationState::new(LazyRules::Off, false);
        assert!(!s.value_changed(false));
        assert!(!s.blurred(false));
        assert_eq!(s.begin(false), None);
    }

    #[test]
    fn reset_clears_everything() {
        let mut s = state(LazyRules::Off);
        s.begin(false);
        s.finish_now(&Err("x".into()));
        s.reset();
        assert!(!s.inner_error);
        assert_eq!(s.message, None);
        assert!(!s.loading);
    }

    #[test]
    fn error_without_text_has_no_message() {
        let mut s = state(LazyRules::Off);
        s.begin(false);
        s.finish_now(&Err(String::new()));
        assert!(s.inner_error);
        assert_eq!(s.message, None);
    }

    #[test]
    fn discards_the_outcome_of_a_superseded_async_validation() {
        let mut s = state(LazyRules::Off);
        let old = s.begin(false).unwrap();
        s.start_async();
        let new = s.begin(false).unwrap();
        s.finish_now(&Ok(()));
        assert!(!s.settle(old, &Err("old".into()), false, false));
        assert!(!s.inner_error, "o veredito velho não sobrescreve o novo");
        assert_ne!(old, new);
    }

    #[test]
    fn reset_supersedes_a_pending_async_validation() {
        let mut s = state(LazyRules::Off);
        let i = s.begin(false).unwrap();
        s.start_async();
        s.reset();
        assert!(!s.settle(i, &Err("late".into()), true, false));
        assert!(!s.inner_error);
        assert!(!s.loading);
    }

    #[test]
    fn revalidates_when_the_value_changed_during_a_pending_async_validation() {
        let mut s = state(LazyRules::Off);
        let i = s.begin(false).unwrap();
        s.start_async();
        assert!(s.settle(i, &Err("Invalid".into()), true, false));
        assert!(s.inner_error);
    }

    #[test]
    fn honors_a_blur_that_happened_during_a_pending_async_validation() {
        let mut s = state(LazyRules::OnBlur);
        let i = s.begin(false).unwrap();
        s.start_async();
        assert!(!s.blurred(false), "adiado: há validação em curso");
        // veredito limpo, mas o blur pendente manda revalidar o valor novo
        assert!(s.settle(i, &Ok(()), true, false));
    }

    #[test]
    fn lazy_without_error_or_blur_stays_lazy_across_an_async_settle() {
        let mut s = state(LazyRules::OnBlur);
        let i = s.begin(false).unwrap();
        s.start_async();
        assert!(!s.settle(i, &Ok(()), true, false));
        assert!(!s.inner_error);
    }

    #[test]
    fn on_demand_never_revalidates_after_an_async_settle() {
        let mut s = state(LazyRules::OnDemand);
        let i = s.begin(false).unwrap();
        s.start_async();
        assert!(!s.blurred(false));
        assert!(!s.settle(i, &Err("Invalid".into()), true, false));
        assert!(s.inner_error);
    }

    #[test]
    fn no_revalidation_when_the_value_did_not_change() {
        let mut s = state(LazyRules::Off);
        let i = s.begin(false).unwrap();
        s.start_async();
        assert!(!s.settle(i, &Err("x".into()), false, false));
    }
}
