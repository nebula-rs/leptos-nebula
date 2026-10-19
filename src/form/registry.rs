//! Registro de filhos e coordenação de validação do `Form` — **sem DOM e sem Leptos**.
//!
//! Porta da lógica do `QForm` do Quasar (`ui/src/components/form/QForm.js`): ordem de
//! registro, validação sequencial × `greedy`, descarte de resultados superados e escolha de
//! quem recebe o foco. O componente só liga isto ao `<form>` e ao DOM.
//!
//! Nenhum empréstimo do `RefCell` interno fica ativo durante um `await` nem durante a chamada
//! de uma closure de filho: um filho pode entrar ou sair do registro no meio de uma validação.

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use futures::future::{LocalBoxFuture, join_all};

/// Identificação de um filho registrado num `Form`.
///
/// Estável durante a vida do filho: sair do registro (ex.: ao ser desabilitado) e voltar com
/// [`Registry::bind`] mantém o mesmo id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormChildId(u64);

/// Valida o filho: `Ok(true)` = válido, `Ok(false)` = inválido, `Err(msg)` = falha inesperada,
/// que conta como inválido (≈ regra que lança no Quasar) e cuja mensagem o componente registra
/// no console. Regra síncrona é uma future já resolvida.
pub type ValidateFn = Rc<dyn Fn() -> LocalBoxFuture<'static, Result<bool, String>>>;

/// Limpa o estado de validação do filho (mensagem de erro, marcação de inválido).
pub type ResetValidationFn = Rc<dyn Fn()>;

/// Foca o filho; devolve `true` se conseguiu (montado e ativo).
pub type FocusFn = Rc<dyn Fn() -> bool>;

/// O que um filho informa ao se registrar.
#[derive(Clone)]
pub struct FormChild {
    /// Nome para diagnóstico.
    pub name: Option<String>,
    /// Validação do filho.
    pub validate: ValidateFn,
    /// Limpeza da validação; chamada por [`Registry::reset_validation`].
    pub reset_validation: Option<ResetValidationFn>,
    /// Foco; sem ela, o filho nunca recebe o foco de erro.
    pub focus: Option<FocusFn>,
}

/// Um filho que falhou na validação.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildFailure {
    /// Quem falhou.
    pub id: FormChildId,
    /// Mensagem do `Err`; `None` quando o filho respondeu `Ok(false)`.
    pub error: Option<String>,
}

/// Opções de uma validação: as props do `Form` e o argumento de `validate(should_focus)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ValidateOptions {
    /// Valida todos concorrentemente e coleta os inválidos, em vez de parar no primeiro.
    pub greedy: bool,
    /// Prop `no_error_focus`: não foca o primeiro inválido.
    pub no_error_focus: bool,
    /// Argumento explícito de `validate`; quando presente, sobrepõe `no_error_focus`.
    pub should_focus: Option<bool>,
}

impl ValidateOptions {
    /// Se esta validação foca o primeiro inválido que aceite foco.
    pub fn focus_on_error(&self) -> bool {
        self.should_focus.unwrap_or(!self.no_error_focus)
    }
}

/// Resultado de [`Registry::validate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationOutcome {
    /// Todos válidos: emitir `on_validation_success`.
    Valid,
    /// Há inválidos: emitir `on_validation_error` com o primeiro de `failures`.
    Invalid {
        /// Inválidos na ordem de registro (no máximo um sem `greedy`).
        failures: Vec<ChildFailure>,
        /// Quem recebeu o foco, se alguém.
        focused: Option<FormChildId>,
    },
    /// Uma validação mais nova (ou um `reset_validation`) começou antes desta terminar:
    /// nenhum evento, nenhum foco, nenhum submit. `valid` é o resultado desta validação.
    Superseded {
        /// Resultado desta validação, devolvido a quem a chamou.
        valid: bool,
    },
}

impl ValidationOutcome {
    /// O `bool` que `validate` devolve a quem chamou (vale também para uma superada).
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Valid => true,
            Self::Invalid { .. } => false,
            Self::Superseded { valid } => *valid,
        }
    }

    /// Se o submit que disparou esta validação deve prosseguir: só válida **e** não superada.
    pub fn should_submit(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    entries: Vec<(FormChildId, FormChild)>,
    /// Contador de validações; a validação compara o valor capturado ao começar com o atual.
    validate_index: u64,
}

/// Registro de filhos de um `Form` e coordenação das validações.
///
/// Clonar é barato e compartilha o mesmo registro (`Rc`). `!Send`: no componente, vai num
/// `StoredValue::new_local`.
#[derive(Clone, Default)]
pub struct Registry(Rc<RefCell<Inner>>);

impl Registry {
    /// Registro vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserva um id para um filho, sem registrá-lo (um filho criado desabilitado já tem id).
    pub fn new_id(&self) -> FormChildId {
        let mut inner = self.0.borrow_mut();
        inner.next_id += 1;
        FormChildId(inner.next_id)
    }

    /// Registra o filho. Id já registrado: troca o filho no mesmo lugar; senão entra no fim.
    pub fn bind(&self, id: FormChildId, child: FormChild) {
        let mut inner = self.0.borrow_mut();
        match inner.entries.iter_mut().find(|(i, _)| *i == id) {
            Some(entry) => entry.1 = child,
            None => inner.entries.push((id, child)),
        }
    }

    /// Tira o filho do registro; devolve se ele estava registrado.
    pub fn unbind(&self, id: FormChildId) -> bool {
        let mut inner = self.0.borrow_mut();
        let before = inner.entries.len();
        inner.entries.retain(|(i, _)| *i != id);
        inner.entries.len() != before
    }

    /// Se o filho está registrado.
    pub fn is_bound(&self, id: FormChildId) -> bool {
        self.0.borrow().entries.iter().any(|(i, _)| *i == id)
    }

    /// Nome (diagnóstico) com que o filho se registrou, se ainda estiver registrado.
    pub fn name_of(&self, id: FormChildId) -> Option<String> {
        let inner = self.0.borrow();
        let (_, child) = inner.entries.iter().find(|(i, _)| *i == id)?;
        child.name.clone()
    }

    /// Ids registrados, na ordem de registro.
    pub fn children(&self) -> Vec<FormChildId> {
        self.0.borrow().entries.iter().map(|(i, _)| *i).collect()
    }

    /// Supera as validações em voo e limpa a validação de todos os filhos.
    pub fn reset_validation(&self) {
        let resets: Vec<ResetValidationFn> = {
            let mut inner = self.0.borrow_mut();
            inner.validate_index += 1;
            inner
                .entries
                .iter()
                .filter_map(|(_, c)| c.reset_validation.clone())
                .collect()
        };
        for reset in resets {
            reset();
        }
    }

    /// Valida os filhos registrados agora (filhos que entram depois de começar ficam de fora).
    ///
    /// Em erro, e se [`ValidateOptions::focus_on_error`], foca o primeiro inválido que ainda
    /// esteja registrado e cuja [`FocusFn`] devolva `true`.
    pub async fn validate(&self, opts: ValidateOptions) -> ValidationOutcome {
        let (ticket, children) = {
            let mut inner = self.0.borrow_mut();
            inner.validate_index += 1;
            (inner.validate_index, inner.entries.clone())
        };

        let failures = run_validation(&children, opts.greedy).await;

        if self.0.borrow().validate_index != ticket {
            return ValidationOutcome::Superseded {
                valid: failures.is_empty(),
            };
        }
        if failures.is_empty() {
            return ValidationOutcome::Valid;
        }
        let focused = if opts.focus_on_error() {
            self.focus_first(&failures)
        } else {
            None
        };
        ValidationOutcome::Invalid { failures, focused }
    }

    /// Reset do `<form>`: `on_reset` roda **antes** de limpar a validação, para o app zerar os
    /// valores primeiro; `next_tick` é a espera entre os dois (no navegador, o próximo quadro).
    pub async fn reset(&self, on_reset: impl FnOnce(), next_tick: impl Future<Output = ()>) {
        on_reset();
        next_tick.await;
        self.reset_validation();
    }

    /// Primeiro inválido ainda registrado (quem saiu foi desmontado) que aceite o foco.
    fn focus_first(&self, failures: &[ChildFailure]) -> Option<FormChildId> {
        let candidates: Vec<(FormChildId, FocusFn)> = {
            let inner = self.0.borrow();
            failures
                .iter()
                .filter_map(|f| {
                    let (_, child) = inner.entries.iter().find(|(i, _)| *i == f.id)?;
                    Some((f.id, child.focus.clone()?))
                })
                .collect()
        };
        candidates
            .into_iter()
            .find_map(|(id, focus)| focus().then_some(id))
    }
}

/// Roda as validações de `children` e devolve os inválidos, na ordem de registro.
///
/// Sem `greedy`, em sequência: o próximo só é chamado depois que o anterior resolve, e para no
/// primeiro inválido. Com `greedy`, chama todos antes de esperar qualquer um.
pub async fn run_validation(
    children: &[(FormChildId, FormChild)],
    greedy: bool,
) -> Vec<ChildFailure> {
    if greedy {
        let results = join_all(children.iter().map(|(_, c)| (c.validate)())).await;
        children
            .iter()
            .zip(results)
            .filter_map(|((id, _), r)| failure(*id, r))
            .collect()
    } else {
        for (id, child) in children {
            if let Some(f) = failure(*id, (child.validate)().await) {
                return vec![f];
            }
        }
        Vec::new()
    }
}

fn failure(id: FormChildId, result: Result<bool, String>) -> Option<ChildFailure> {
    match result {
        Ok(true) => None,
        Ok(false) => Some(ChildFailure { id, error: None }),
        Err(msg) => Some(ChildFailure {
            id,
            error: Some(msg),
        }),
    }
}

/// Casos portados de `ui/src/components/form/QForm.test.js` (Quasar).
#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::Cell;

    use futures::FutureExt;
    use futures::channel::oneshot;
    use futures::executor::{LocalPool, block_on};
    use futures::task::LocalSpawnExt;

    /// Conta as chamadas feitas a um filho de teste.
    #[derive(Default)]
    struct Probe {
        validate: Cell<usize>,
        reset: Cell<usize>,
        focus: Cell<usize>,
    }

    /// Filho com resultado fixo. `focus`: `None` = sem `FocusFn`; `Some(ok)` = devolve `ok`.
    fn child(result: Result<bool, String>, focus: Option<bool>) -> (FormChild, Rc<Probe>) {
        let probe = Rc::new(Probe::default());
        let p = probe.clone();
        let validate: ValidateFn = Rc::new(move || {
            p.validate.set(p.validate.get() + 1);
            futures::future::ready(result.clone()).boxed_local()
        });
        let p = probe.clone();
        let reset: ResetValidationFn = Rc::new(move || p.reset.set(p.reset.get() + 1));
        let focus = focus.map(|ok| {
            let p = probe.clone();
            Rc::new(move || {
                p.focus.set(p.focus.get() + 1);
                ok
            }) as FocusFn
        });
        let child = FormChild {
            name: None,
            validate,
            reset_validation: Some(reset),
            focus,
        };
        (child, probe)
    }

    fn valid() -> (FormChild, Rc<Probe>) {
        child(Ok(true), Some(true))
    }

    fn invalid() -> (FormChild, Rc<Probe>) {
        child(Ok(false), Some(true))
    }

    type Resolver = oneshot::Sender<Result<bool, String>>;

    /// Filho cujo `validate` fica pendente até `resolve(n, …)` (n = n-ésima chamada).
    struct Deferred {
        probe: Rc<Probe>,
        senders: Rc<RefCell<Vec<Option<Resolver>>>>,
    }

    impl Deferred {
        fn new() -> (FormChild, Self) {
            let probe = Rc::new(Probe::default());
            let senders: Rc<RefCell<Vec<_>>> = Rc::default();
            let (p, s) = (probe.clone(), senders.clone());
            let validate: ValidateFn = Rc::new(move || {
                p.validate.set(p.validate.get() + 1);
                let (tx, rx) = oneshot::channel();
                s.borrow_mut().push(Some(tx));
                async move { rx.await.unwrap_or_else(|_| Err("canceled".into())) }.boxed_local()
            });
            let p = probe.clone();
            let focus: FocusFn = Rc::new(move || {
                p.focus.set(p.focus.get() + 1);
                true
            });
            let child = FormChild {
                name: None,
                validate,
                reset_validation: None,
                focus: Some(focus),
            };
            (child, Self { probe, senders })
        }

        fn resolve(&self, call: usize, result: Result<bool, String>) {
            let tx = self.senders.borrow_mut()[call]
                .take()
                .expect("já resolvido");
            tx.send(result).expect("validação descartada");
        }
    }

    fn register(reg: &Registry, child: FormChild) -> FormChildId {
        let id = reg.new_id();
        reg.bind(id, child);
        id
    }

    /// Roda `fut` no `pool` e devolve onde o resultado aparece quando terminar.
    fn spawn<T: 'static>(
        pool: &LocalPool,
        fut: impl Future<Output = T> + 'static,
    ) -> Rc<RefCell<Option<T>>> {
        let slot = Rc::new(RefCell::new(None));
        let s = slot.clone();
        pool.spawner()
            .spawn_local(async move { *s.borrow_mut() = Some(fut.await) })
            .expect("spawn");
        slot
    }

    fn validate_in(
        pool: &LocalPool,
        reg: &Registry,
        opts: ValidateOptions,
    ) -> Rc<RefCell<Option<ValidationOutcome>>> {
        let reg = reg.clone();
        spawn(pool, async move { reg.validate(opts).await })
    }

    const NO_FOCUS: ValidateOptions = ValidateOptions {
        greedy: false,
        no_error_focus: false,
        should_focus: Some(false),
    };

    const GREEDY_NO_FOCUS: ValidateOptions = ValidateOptions {
        greedy: true,
        ..NO_FOCUS
    };

    const GREEDY: ValidateOptions = ValidateOptions {
        greedy: true,
        no_error_focus: false,
        should_focus: None,
    };

    // validates async fields one at a time when not greedy
    #[test]
    fn validates_one_at_a_time_when_not_greedy() {
        let reg = Registry::new();
        let (c1, first) = Deferred::new();
        let (c2, second) = Deferred::new();
        register(&reg, c1);
        register(&reg, c2);

        let mut pool = LocalPool::new();
        let out = validate_in(&pool, &reg, NO_FOCUS);
        pool.run_until_stalled();

        // o segundo espera o primeiro resolver
        assert_eq!(first.probe.validate.get(), 1);
        assert_eq!(second.probe.validate.get(), 0);

        first.resolve(0, Ok(true));
        pool.run_until_stalled();
        assert_eq!(second.probe.validate.get(), 1);
        assert!(out.borrow().is_none());

        second.resolve(0, Ok(true));
        pool.run_until_stalled();
        assert_eq!(*out.borrow(), Some(ValidationOutcome::Valid));
    }

    #[test]
    fn a_failure_stops_the_sequential_chain() {
        let reg = Registry::new();
        let (c1, first) = Deferred::new();
        let (c2, second) = Deferred::new();
        let id1 = register(&reg, c1);
        register(&reg, c2);

        let mut pool = LocalPool::new();
        let out = validate_in(&pool, &reg, NO_FOCUS);
        pool.run_until_stalled();
        first.resolve(0, Ok(false));
        pool.run_until_stalled();

        assert_eq!(second.probe.validate.get(), 0);
        assert_eq!(
            *out.borrow(),
            Some(ValidationOutcome::Invalid {
                failures: vec![ChildFailure {
                    id: id1,
                    error: None
                }],
                focused: None,
            })
        );
    }

    // validates async fields concurrently when greedy
    #[test]
    fn validates_concurrently_when_greedy() {
        let reg = Registry::new();
        let (c1, first) = Deferred::new();
        let (c2, second) = Deferred::new();
        let id1 = register(&reg, c1);
        register(&reg, c2);

        let mut pool = LocalPool::new();
        let out = validate_in(&pool, &reg, GREEDY_NO_FOCUS);
        pool.run_until_stalled();

        // todos chamados antes de qualquer um resolver
        assert_eq!(first.probe.validate.get(), 1);
        assert_eq!(second.probe.validate.get(), 1);

        // resolvem fora de ordem; o resultado segue a ordem de registro
        second.resolve(0, Ok(true));
        pool.run_until_stalled();
        assert!(out.borrow().is_none());
        first.resolve(0, Ok(false));
        pool.run_until_stalled();

        let out = out.borrow().clone().expect("terminou");
        assert!(!out.is_valid());
        assert!(
            matches!(out, ValidationOutcome::Invalid { ref failures, .. }
            if failures.iter().map(|f| f.id).collect::<Vec<_>>() == [id1])
        );
    }

    // (prop)greedy type Boolean has effect
    #[test]
    fn greedy_collects_every_failure_and_sequential_stops_at_first() {
        let reg = Registry::new();
        let (c1, p1) = invalid();
        let (c2, p2) = valid();
        let (c3, p3) = invalid();
        let id1 = register(&reg, c1);
        register(&reg, c2);
        let id3 = register(&reg, c3);

        let out = block_on(reg.validate(NO_FOCUS));
        assert!(
            matches!(out, ValidationOutcome::Invalid { ref failures, .. }
            if failures.iter().map(|f| f.id).collect::<Vec<_>>() == [id1])
        );
        assert_eq!(
            (p1.validate.get(), p2.validate.get(), p3.validate.get()),
            (1, 0, 0)
        );

        let out = block_on(reg.validate(GREEDY_NO_FOCUS));
        assert!(
            matches!(out, ValidationOutcome::Invalid { ref failures, .. }
            if failures.iter().map(|f| f.id).collect::<Vec<_>>() == [id1, id3])
        );
        assert_eq!(
            (p1.validate.get(), p2.validate.get(), p3.validate.get()),
            (2, 1, 1)
        );
    }

    // a superseded validation emits no events
    #[test]
    fn a_superseded_validation_reports_superseded() {
        let reg = Registry::new();
        let (c, deferred) = Deferred::new();
        register(&reg, c);

        let mut pool = LocalPool::new();
        let stale = validate_in(&pool, &reg, ValidateOptions::default());
        let fresh = validate_in(&pool, &reg, ValidateOptions::default());
        pool.run_until_stalled();

        deferred.resolve(1, Ok(true));
        pool.run_until_stalled();
        assert_eq!(*fresh.borrow(), Some(ValidationOutcome::Valid));

        deferred.resolve(0, Ok(false));
        pool.run_until_stalled();
        // a antiga devolve o próprio resultado, mas não foca nem reporta erro
        assert_eq!(
            *stale.borrow(),
            Some(ValidationOutcome::Superseded { valid: false })
        );
        assert_eq!(deferred.probe.focus.get(), 0);
    }

    // drops a submission superseded while validating
    #[test]
    fn drops_a_submission_superseded_while_validating() {
        let reg = Registry::new();
        let (c, deferred) = Deferred::new();
        register(&reg, c);

        let mut pool = LocalPool::new();
        let submit = validate_in(&pool, &reg, ValidateOptions::default());
        pool.run_until_stalled();

        // o estado do form é resetado com a validação em voo
        reg.reset_validation();

        deferred.resolve(0, Ok(true));
        pool.run_until_stalled();
        let out = submit.borrow().clone().expect("terminou");
        assert_eq!(out, ValidationOutcome::Superseded { valid: true });
        assert!(out.is_valid());
        assert!(!out.should_submit());
    }

    #[test]
    fn a_current_valid_validation_submits() {
        let reg = Registry::new();
        register(&reg, valid().0);
        let out = block_on(reg.validate(ValidateOptions::default()));
        assert!(out.should_submit());

        register(&reg, invalid().0);
        assert!(!block_on(reg.validate(ValidateOptions::default())).should_submit());
    }

    // focuses the first failing component by default
    #[test]
    fn focuses_the_first_failing_child_by_default() {
        let reg = Registry::new();
        let (c1, p1) = valid();
        let (c2, p2) = invalid();
        register(&reg, c1);
        let id2 = register(&reg, c2);

        let out = block_on(reg.validate(ValidateOptions::default()));
        assert!(matches!(out, ValidationOutcome::Invalid { focused, .. } if focused == Some(id2)));
        assert_eq!((p1.focus.get(), p2.focus.get()), (0, 1));
    }

    // skips failing components that cannot receive focus
    #[test]
    fn skips_failing_children_that_cannot_take_focus() {
        let reg = Registry::new();
        let (focusless, p_focusless) = child(Ok(false), None);
        let (unmounted, p_unmounted) = child(Ok(false), Some(false));
        let (focusable, p_focusable) = invalid();
        let id_focusless = register(&reg, focusless);
        register(&reg, unmounted);
        let id_focusable = register(&reg, focusable);

        let out = block_on(reg.validate(GREEDY));
        let ValidationOutcome::Invalid { failures, focused } = out else {
            panic!("esperava Invalid, veio {out:?}");
        };
        // o evento reporta o PRIMEIRO inválido...
        assert_eq!(failures[0].id, id_focusless);
        // ...mas o foco vai para o primeiro que consegue recebê-lo
        assert_eq!(focused, Some(id_focusable));
        assert_eq!(p_focusless.focus.get(), 0);
        assert_eq!(p_unmounted.focus.get(), 1);
        assert_eq!(p_focusable.focus.get(), 1);
    }

    #[test]
    fn does_not_focus_a_child_unbound_while_validating() {
        let reg = Registry::new();
        let (c1, gone) = Deferred::new();
        let (c2, p2) = invalid();
        let id1 = register(&reg, c1);
        let id2 = register(&reg, c2);

        let mut pool = LocalPool::new();
        let out = validate_in(&pool, &reg, GREEDY);
        pool.run_until_stalled();
        assert!(reg.unbind(id1));
        gone.resolve(0, Ok(false));
        pool.run_until_stalled();

        let out = out.borrow().clone().expect("terminou");
        assert!(
            matches!(out, ValidationOutcome::Invalid { ref failures, focused }
            if failures[0].id == id1 && focused == Some(id2))
        );
        assert_eq!((gone.probe.focus.get(), p2.focus.get()), (0, 1));
    }

    // (prop)no-error-focus type Boolean has effect
    #[test]
    fn no_error_focus_prevents_the_focus() {
        let reg = Registry::new();
        let (c, p) = invalid();
        register(&reg, c);

        let opts = ValidateOptions {
            no_error_focus: true,
            ..Default::default()
        };
        let out = block_on(reg.validate(opts));
        assert!(matches!(
            out,
            ValidationOutcome::Invalid { focused: None, .. }
        ));
        assert_eq!(p.focus.get(), 0);
    }

    // (prop)no-error-focus — an explicit validate(true) overrides it
    #[test]
    fn an_explicit_should_focus_overrides_no_error_focus() {
        let reg = Registry::new();
        let (c, p) = invalid();
        let id = register(&reg, c);

        let opts = ValidateOptions {
            no_error_focus: true,
            should_focus: Some(true),
            ..Default::default()
        };
        let out = block_on(reg.validate(opts));
        assert!(matches!(out, ValidationOutcome::Invalid { focused, .. } if focused == Some(id)));
        assert_eq!(p.focus.get(), 1);

        // e validate(false) não foca mesmo sem a prop
        block_on(reg.validate(NO_FOCUS));
        assert_eq!(p.focus.get(), 1);
    }

    #[test]
    fn focus_on_error_rule() {
        let cases = [
            (None, false, true),
            (None, true, false),
            (Some(true), true, true),
            (Some(true), false, true),
            (Some(false), false, false),
            (Some(false), true, false),
        ];
        for (should_focus, no_error_focus, expected) in cases {
            let opts = ValidateOptions {
                greedy: false,
                no_error_focus,
                should_focus,
            };
            assert_eq!(opts.focus_on_error(), expected, "{opts:?}");
        }
    }

    // unregisters removed fields from validation
    #[test]
    fn unbound_children_leave_the_next_validation() {
        let reg = Registry::new();
        let (c1, _) = valid();
        let (c2, p2) = invalid();
        register(&reg, c1);
        let id2 = register(&reg, c2);

        assert_eq!(reg.children().len(), 2);
        assert!(!block_on(reg.validate(NO_FOCUS)).is_valid());

        assert!(reg.unbind(id2));
        assert!(!reg.unbind(id2));
        assert_eq!(reg.children().len(), 1);
        assert!(block_on(reg.validate(NO_FOCUS)).is_valid());
        assert_eq!(p2.validate.get(), 1);
    }

    // lets the reset listener run before validations are reset
    #[test]
    fn reset_runs_the_listener_before_resetting_validation() {
        let reg = Registry::new();
        let order: Rc<RefCell<Vec<&str>>> = Rc::default();
        let o = order.clone();
        let (mut c, _) = valid();
        c.reset_validation = Some(Rc::new(move || o.borrow_mut().push("validation")));
        register(&reg, c);

        let o = order.clone();
        let tick = order.clone();
        block_on(
            reg.reset(move || o.borrow_mut().push("listener"), async move {
                tick.borrow_mut().push("tick")
            }),
        );
        assert_eq!(*order.borrow(), ["listener", "tick", "validation"]);
    }

    // reports a child whose validate() throws (greedy e não greedy)
    #[test]
    fn err_counts_as_invalid() {
        for greedy in [false, true] {
            let reg = Registry::new();
            let (c, p) = child(Err("Rule blew up".into()), Some(true));
            let id = register(&reg, c);

            let opts = ValidateOptions {
                greedy,
                ..Default::default()
            };
            let out = block_on(reg.validate(opts));
            assert_eq!(
                out,
                ValidationOutcome::Invalid {
                    failures: vec![ChildFailure {
                        id,
                        error: Some("Rule blew up".into())
                    }],
                    focused: Some(id),
                },
                "greedy = {greedy}"
            );
            assert_eq!(p.focus.get(), 1);
        }
    }

    // (event)validation-success is emitting — sem filhos
    #[test]
    fn an_empty_registry_is_valid() {
        let reg = Registry::new();
        assert_eq!(
            block_on(reg.validate(ValidateOptions::default())),
            ValidationOutcome::Valid
        );
        assert_eq!(block_on(reg.validate(GREEDY)), ValidationOutcome::Valid);
    }

    // (method)resetValidation should be callable
    #[test]
    fn reset_validation_reaches_every_child() {
        let reg = Registry::new();
        let (c1, p1) = valid();
        let (mut c2, _) = valid();
        c2.reset_validation = None;
        let (c3, p3) = invalid();
        register(&reg, c1);
        register(&reg, c2);
        register(&reg, c3);

        reg.reset_validation();
        assert_eq!((p1.reset.get(), p3.reset.get()), (1, 1));
    }

    // (method)getValidationComponents + re-registro de um filho reabilitado
    #[test]
    fn ids_are_stable_and_keep_registration_order() {
        let reg = Registry::new();
        let a = register(&reg, valid().0);
        let b = register(&reg, valid().0);
        let reserved = reg.new_id();
        assert_ne!(a, b);
        assert_eq!(reg.children(), [a, b]);
        assert!(!reg.is_bound(reserved));

        // bind de um id já registrado troca o filho no lugar
        let (replacement, p) = invalid();
        reg.bind(a, replacement);
        assert_eq!(reg.children(), [a, b]);
        assert!(!block_on(reg.validate(NO_FOCUS)).is_valid());
        assert_eq!(p.validate.get(), 1);

        // sair e voltar (disabled → enabled) mantém o id e vai para o fim, como no Quasar
        reg.unbind(a);
        reg.bind(a, valid().0);
        assert_eq!(reg.children(), [b, a]);

        reg.bind(reserved, valid().0);
        assert_eq!(reg.children(), [b, a, reserved]);
    }
}
