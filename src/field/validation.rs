//! `use_validate` e `FieldValidation`: liga a lógica pura de [`validate`](super::validate) aos
//! sinais do Leptos e ao `Form` (≈ `use-validate.js` + `useFormChild` do Quasar).

use std::rc::Rc;
use std::sync::Arc;

use futures::future::{FutureExt, LocalBoxFuture};
use leptos::prelude::*;

use super::validate::{LazyRules, Rule, Run, ValidationState, has_error, pick_message, run_rules};
use crate::{FocusFn, FormChildOptions, ResetValidationFn, ValidateFn, use_form_child};

/// Opções de [`use_validate`].
pub struct ValidationOptions<T> {
    /// Regras, avaliadas em ordem; a primeira que falha define a mensagem. Lidas uma vez (não
    /// há `reactive_rules` ainda).
    pub rules: Vec<Rule<T>>,
    /// Quando validar sozinho (padrão [`LazyRules::Off`]). Lido uma vez.
    pub lazy_rules: LazyRules,
    /// Campo desabilitado: não valida, sai do `Form` e tem o erro limpo.
    pub disable: Signal<bool>,
    /// Erro externo (`error` do Quasar). `Some(true)` mostra erro; `Some(false)` e `None` não
    /// suprimem o erro das regras.
    pub error: Signal<Option<bool>>,
    /// Mensagem externa (`error_message`): não vazia, vence a das regras.
    pub error_message: Signal<String>,
    /// Nome do campo no `Form`, para diagnóstico.
    pub name: Option<String>,
}

impl<T> Default for ValidationOptions<T> {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            lazy_rules: LazyRules::Off,
            disable: Signal::stored(false),
            error: Signal::stored(None),
            error_message: Signal::stored(String::new()),
            name: None,
        }
    }
}

impl<T> ValidationOptions<T> {
    /// Opções só com as regras; o restante usa os padrões.
    pub fn new(rules: Vec<Rule<T>>) -> Self {
        Self {
            rules,
            ..Self::default()
        }
    }
}

#[derive(Clone)]
struct Actions {
    validate: Rc<dyn Fn() -> LocalBoxFuture<'static, bool>>,
    reset: ResetValidationFn,
    blur: Rc<dyn Fn()>,
}

/// O estado de validação de um campo, devolvido por [`use_validate`] e passado ao
/// [`Field`](crate::Field) (`validation=`). `Copy`.
#[derive(Clone, Copy)]
pub struct FieldValidation {
    has_error: Signal<bool>,
    error_message: Signal<Option<String>>,
    loading: Signal<bool>,
    actions: StoredValue<Actions, LocalStorage>,
    focus: StoredValue<Option<FocusFn>, LocalStorage>,
}

impl FieldValidation {
    /// Há erro visível (das regras ou o `error` externo).
    pub fn has_error(&self) -> Signal<bool> {
        self.has_error
    }

    /// Mensagem do erro: a externa não vazia, senão a da regra que falhou.
    pub fn error_message(&self) -> Signal<Option<String>> {
        self.error_message
    }

    /// Há uma validação assíncrona em andamento.
    pub fn loading(&self) -> Signal<bool> {
        self.loading
    }

    /// Valida agora, com o valor atual; `true` se válido. Sem regras ou desabilitado: `true`,
    /// sem tocar no estado. Pode esperar regras assíncronas.
    pub fn validate(&self) -> LocalBoxFuture<'static, bool> {
        match self.actions.try_get_value() {
            Some(a) => (a.validate)(),
            None => async { true }.boxed_local(),
        }
    }

    /// Limpa o erro e supera qualquer validação em andamento.
    pub fn reset_validation(&self) {
        if let Some(a) = self.actions.try_get_value() {
            (a.reset)();
        }
    }

    /// O campo perdeu o foco: valida, salvo `lazy_rules` em `OnDemand`. O [`Field`](crate::Field)
    /// chama isto no `focusout`.
    pub fn on_blur(&self) {
        if let Some(a) = self.actions.try_get_value() {
            (a.blur)();
        }
    }

    /// Define como focar o campo quando o `Form` pede o foco do primeiro inválido. O
    /// [`Field`](crate::Field) faz isto sozinho; use só com um controle sem `Field`.
    pub fn set_focus(&self, focus: FocusFn) {
        self.focus.try_set_value(Some(focus));
    }

    /// Desfaz [`set_focus`](Self::set_focus).
    pub fn clear_focus(&self) {
        self.focus.try_set_value(None);
    }
}

/// Tudo o que as operações precisam; `Copy` (só ids de armazenamento e sinais).
struct Ctl<T: Send + Sync + 'static> {
    state: StoredValue<ValidationState>,
    rules: StoredValue<Vec<Rule<T>>, LocalStorage>,
    value: Signal<T>,
    /// Compara dois valores (o `PartialEq` do tipo, ou a identidade das options do `Select`).
    same: StoredValue<Same<T>>,
    disable: Signal<bool>,
    inner_error: RwSignal<bool>,
    message: RwSignal<Option<String>>,
    loading: RwSignal<bool>,
}

impl<T: Send + Sync + 'static> Clone for Ctl<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Send + Sync + 'static> Copy for Ctl<T> {}

impl<T: Clone + Send + Sync + 'static> Ctl<T> {
    /// Copia o estado puro para os sinais.
    fn publish(self) {
        let Some((e, m, l)) = self
            .state
            .try_with_value(|s| (s.inner_error, s.message.clone(), s.loading))
        else {
            return;
        };
        self.inner_error.try_set(e);
        self.message.try_set(m);
        self.loading.try_set(l);
    }

    fn spawn_validate(self) {
        leptos::task::spawn_local(async move {
            self.validate().await;
        });
    }

    fn validate(self) -> LocalBoxFuture<'static, bool> {
        let disabled = self.disable.get_untracked();
        let Some(index) = self.state.try_update_value(|s| s.begin(disabled)).flatten() else {
            return async { true }.boxed_local();
        };
        let start = self.value.get_untracked();
        let Some(run) = self.rules.try_with_value(|r| run_rules(r, &start)) else {
            return async { true }.boxed_local();
        };
        match run {
            Run::Now(res) => {
                self.state.update_value(|s| s.finish_now(&res));
                self.publish();
                async move { res.is_ok() }.boxed_local()
            }
            Run::Later(fut) => {
                self.state.update_value(|s| s.start_async());
                self.publish();
                async move {
                    let res = fut.await;
                    let changed = self.value.try_get_untracked().is_some_and(|now| {
                        !self
                            .same
                            .try_with_value(|same| same(&now, &start))
                            .unwrap_or(true)
                    });
                    let disabled = self.disable.try_get_untracked().unwrap_or(true);
                    let again = self
                        .state
                        .try_update_value(|s| s.settle(index, &res, changed, disabled))
                        .unwrap_or(false);
                    self.publish();
                    if again {
                        self.spawn_validate();
                    }
                    res.is_ok()
                }
                .boxed_local()
            }
        }
    }

    fn on_value_change(self) {
        let disabled = self.disable.get_untracked();
        if self
            .state
            .try_with_value(|s| s.value_changed(disabled))
            .unwrap_or(false)
        {
            self.spawn_validate();
        }
    }

    fn on_blur(self) {
        let disabled = self.disable.get_untracked();
        if self
            .state
            .try_update_value(|s| s.blurred(disabled))
            .unwrap_or(false)
        {
            self.spawn_validate();
        }
    }

    fn reset(self) {
        self.state.try_update_value(ValidationState::reset);
        self.publish();
    }
}

/// Validação de um campo: aplica `rules` a `value`, registra o campo no `Form` mais próximo
/// (validação, limpeza, foco e `disable`) e devolve o [`FieldValidation`] para o
/// [`Field`](crate::Field). ≈ `use-validate` do Quasar.
///
/// ```ignore
/// let value = RwSignal::new(String::new());
/// let validation = use_validate(
///     value.into(),
///     ValidationOptions::new(vec![Rule::new(|v: &String| {
///         if v.trim().is_empty() { Err("Obrigatório".into()) } else { Ok(()) }
///     })]),
/// );
/// view! { <Field validation=validation …>…</Field> }
/// ```
///
/// Chame dentro do componente do campo (precisa de um dono reativo), antes do `view!`.
pub fn use_validate<T>(value: Signal<T>, opts: ValidationOptions<T>) -> FieldValidation
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    use_validate_with(value, opts, Arc::new(|a: &T, b: &T| a == b))
}

/// Comparador de valores (ver `use_validate_with`).
pub(crate) type Same<T> = Arc<dyn Fn(&T, &T) -> bool + Send + Sync>;

/// [`use_validate`] para modelos sem `PartialEq`: `same` diz se dois valores são o mesmo (o
/// `Select` usa a identidade das options). Só é consultado ao fim de uma regra assíncrona, para
/// saber se o valor mudou enquanto ela rodava.
pub(crate) fn use_validate_with<T>(
    value: Signal<T>,
    opts: ValidationOptions<T>,
    same: Same<T>,
) -> FieldValidation
where
    T: Clone + Send + Sync + 'static,
{
    let ValidationOptions {
        rules,
        lazy_rules,
        disable,
        error,
        error_message,
        name,
    } = opts;
    let inner_error = RwSignal::new(false);
    let message = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let ctl = Ctl {
        state: StoredValue::new(ValidationState::new(lazy_rules, !rules.is_empty())),
        rules: StoredValue::new_local(rules),
        value,
        same: StoredValue::new(same),
        disable,
        inner_error,
        message,
        loading,
    };

    // Mudança de valor (a primeira execução do Effect é o valor inicial: não valida).
    Effect::new(move |prev: Option<()>| {
        value.track();
        if prev.is_some() {
            ctl.on_value_change();
        }
    });

    let actions = Actions {
        validate: Rc::new(move || ctl.validate()),
        reset: Rc::new(move || ctl.reset()),
        blur: Rc::new(move || ctl.on_blur()),
    };
    let focus: StoredValue<Option<FocusFn>, LocalStorage> = StoredValue::new_local(None);

    let validate_fn: ValidateFn = {
        let validate = actions.validate.clone();
        Rc::new(move || {
            let fut = validate();
            async move { Ok(fut.await) }.boxed_local()
        })
    };
    let mut child = FormChildOptions::new(validate_fn);
    child.name = name;
    child.reset_validation = Some(actions.reset.clone());
    child.focus = Some(Rc::new(move || {
        focus
            .try_get_value()
            .flatten()
            .is_some_and(|focus_fn| focus_fn())
    }));
    child.disabled = disable;
    use_form_child(child);

    FieldValidation {
        has_error: Signal::derive(move || has_error(error.get(), inner_error.get())),
        error_message: Signal::derive(move || {
            pick_message(&error_message.get(), message.get().as_deref())
        }),
        loading: loading.into(),
        actions: StoredValue::new_local(actions),
        focus,
    }
}
