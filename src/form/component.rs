//! `<Form>` e `FormHandle` (≈ `QForm` do Quasar).

use futures::channel::oneshot;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::{HtmlFormElement, SubmitEvent};

use super::child::FormContext;
use super::focus::focus_form;
use super::registry::{FormChildId, Registry, ValidateOptions, ValidationOutcome};
use crate::util::cls;

/// Tudo que o `<Form>` conecta ao seu `FormHandle`.
#[derive(Clone)]
struct Connected {
    registry: Registry,
    form: NodeRef<html::Form>,
    greedy: bool,
    no_error_focus: bool,
    no_reset_focus: bool,
    autofocus: bool,
    on_submit: Option<Callback<SubmitEvent>>,
    on_reset: Option<Callback<()>>,
    on_validation_success: Option<Callback<()>>,
    on_validation_error: Option<Callback<FormChildId>>,
}

impl Connected {
    /// Valida e emite `on_validation_success`/`on_validation_error`; uma validação superada
    /// não emite nada. Falhas inesperadas (`Err`) vão para o console.
    async fn validate(&self, should_focus: Option<bool>) -> ValidationOutcome {
        let outcome = self
            .registry
            .validate(ValidateOptions {
                greedy: self.greedy,
                no_error_focus: self.no_error_focus,
                should_focus,
            })
            .await;
        match &outcome {
            ValidationOutcome::Valid => {
                if let Some(cb) = self.on_validation_success {
                    cb.try_run(());
                }
            }
            ValidationOutcome::Invalid { failures, .. } => {
                for f in failures {
                    if let Some(error) = &f.error {
                        match self.registry.name_of(f.id) {
                            Some(name) => leptos::logging::error!(
                                "Form: falha inesperada na validação de `{name}`: {error}"
                            ),
                            None => {
                                leptos::logging::error!(
                                    "Form: falha inesperada na validação: {error}"
                                )
                            }
                        }
                    }
                }
                if let (Some(cb), Some(first)) = (self.on_validation_error, failures.first()) {
                    cb.try_run(first.id);
                }
            }
            ValidationOutcome::Superseded { .. } => {}
        }
        outcome
    }

    /// Submit: valida; se válida e não superada, chama `on_submit` ou faz o envio nativo.
    async fn submit(&self, ev: SubmitEvent) {
        if !self.validate(None).await.should_submit() {
            return;
        }
        match self.on_submit {
            Some(cb) => {
                cb.try_run(ev);
            }
            None => {
                if let Some(form) = self.form.get_untracked() {
                    // `submit()` não dispara o evento `submit` de novo.
                    let _ = HtmlFormElement::submit(&form);
                }
            }
        }
    }

    /// Reset: `on_reset` → próximo quadro → limpa as validações → foco (`autofocus`).
    async fn reset(&self) {
        let on_reset = self.on_reset;
        self.registry
            .reset(
                move || {
                    if let Some(cb) = on_reset {
                        cb.try_run(());
                    }
                },
                next_frame(),
            )
            .await;
        if self.autofocus && !self.no_reset_focus {
            self.focus();
        }
    }

    fn focus(&self) {
        if let Some(form) = self.form.get_untracked() {
            focus_form(&form);
        }
    }
}

/// Espera o próximo quadro de animação (o "próximo tick" do reset).
async fn next_frame() {
    let (tx, rx) = oneshot::channel();
    request_animation_frame(move || {
        let _ = tx.send(());
    });
    let _ = rx.await;
}

/// Controle de um [`Form`] por código (≈ os métodos do `ref` do `QForm`).
///
/// `Copy`. Crie com [`FormHandle::new`] e passe em `handle=`; o `<Form>` o conecta ao seu
/// registro ao ser criado. Antes disso (ou depois de o `<Form>` ser descartado) os métodos não
/// fazem nada: `validate` devolve `false` e `children` devolve uma lista vazia.
#[derive(Clone, Copy)]
pub struct FormHandle(StoredValue<Option<Connected>, LocalStorage>);

impl Default for FormHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl FormHandle {
    /// Handle ainda sem `<Form>`.
    pub fn new() -> Self {
        Self(StoredValue::new_local(None))
    }

    fn connected(&self) -> Option<Connected> {
        self.0.try_get_value().flatten()
    }

    /// Valida os campos registrados e devolve se são todos válidos. Emite
    /// `on_validation_success`/`on_validation_error` (nada, se for superada por uma validação
    /// mais nova). `should_focus` explícito sobrepõe `no_error_focus`.
    pub fn validate(&self, should_focus: Option<bool>) -> impl Future<Output = bool> + use<> {
        let connected = self.connected();
        async move {
            match connected {
                Some(c) => c.validate(should_focus).await.is_valid(),
                None => false,
            }
        }
    }

    /// Limpa a validação de todos os campos e supera as validações em andamento.
    pub fn reset_validation(&self) {
        if let Some(c) = self.connected() {
            c.registry.reset_validation();
        }
    }

    /// Pede o envio do `<form>` (dispara o evento `submit`, que valida e chama `on_submit`).
    pub fn submit(&self) {
        if let Some(form) = self.connected().and_then(|c| c.form.get_untracked()) {
            let _ = form.request_submit();
        }
    }

    /// Pede o reset do `<form>` (dispara o evento `reset`).
    pub fn reset(&self) {
        if let Some(form) = self.connected().and_then(|c| c.form.get_untracked()) {
            form.reset();
        }
    }

    /// Foca o primeiro elemento adequado do formulário (ver `autofocus`).
    pub fn focus(&self) {
        if let Some(c) = self.connected() {
            c.focus();
        }
    }

    /// Ids dos campos registrados, na ordem de registro.
    pub fn children(&self) -> Vec<FormChildId> {
        self.connected()
            .map(|c| c.registry.children())
            .unwrap_or_default()
    }
}

/// Formulário que coordena validação, submit e reset dos campos registrados. ≈ `QForm`.
///
/// Renderiza um `<form>`; atributos extras (`attr:action`, `attr:method`,
/// `attr:autocomplete`…) vão para ele. Os campos se registram com [`use_form_child`](crate::use_form_child).
///
/// - `autofocus`: foca o formulário ao montar e depois de um reset (salvo `no_reset_focus`).
/// - `no_error_focus`: não foca o primeiro campo inválido.
/// - `greedy`: valida todos os campos de uma vez, em vez de parar no primeiro inválido.
/// - `on_submit`: chamado se a validação passar; sem ele, faz o envio nativo do `<form>`.
/// - `on_reset`: chamado **antes** de limpar as validações (para o app zerar os valores).
/// - `on_validation_success`, `on_validation_error` (recebe o primeiro campo inválido).
/// - `handle`: [`FormHandle`] para controlar o formulário por código.
///
/// O envio e o reset nativos são interceptados (`preventDefault`).
#[component]
pub fn Form(
    #[prop(optional)] autofocus: bool,
    #[prop(optional)] no_error_focus: bool,
    #[prop(optional)] no_reset_focus: bool,
    #[prop(optional)] greedy: bool,
    #[prop(optional, into)] on_submit: Option<Callback<SubmitEvent>>,
    #[prop(optional, into)] on_reset: Option<Callback<()>>,
    #[prop(optional, into)] on_validation_success: Option<Callback<()>>,
    #[prop(optional, into)] on_validation_error: Option<Callback<FormChildId>>,
    #[prop(optional)] handle: Option<FormHandle>,
    children: Children,
) -> impl IntoView {
    let registry = Registry::new();
    let form = NodeRef::<html::Form>::new();
    let connected = Connected {
        registry: registry.clone(),
        form,
        greedy,
        no_error_focus,
        no_reset_focus,
        autofocus,
        on_submit,
        on_reset,
        on_validation_success,
        on_validation_error,
    };
    provide_context(FormContext {
        registry: StoredValue::new_local(registry),
    });

    let state = StoredValue::new_local(connected.clone());
    if let Some(handle) = handle {
        handle.0.set_value(Some(connected));
        on_cleanup(move || {
            handle.0.try_set_value(None);
        });
    }

    // Só no navegador: Effects não rodam na renderização no servidor.
    Effect::new(move |_| {
        if autofocus && form.get().is_some() {
            state.get_value().focus();
        }
    });

    let on_submit_ev = move |ev: SubmitEvent| {
        ev.prevent_default();
        let state = state.get_value();
        spawn_local(async move { state.submit(ev).await });
    };
    let on_reset_ev = move |ev: ev::Event| {
        ev.prevent_default();
        let state = state.get_value();
        spawn_local(async move { state.reset().await });
    };

    view! {
        <form node_ref=form class=cls(["lx-form"]) on:submit=on_submit_ev on:reset=on_reset_ev>
            {children()}
        </form>
    }
}
