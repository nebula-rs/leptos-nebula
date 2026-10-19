//! `use_form_child` — contrato que qualquer campo usa para se registrar no `Form` mais próximo
//! (≈ `useFormChild` do Quasar).

use leptos::prelude::*;

use super::registry::{FocusFn, FormChild, FormChildId, Registry, ResetValidationFn, ValidateFn};

/// Contexto fornecido pelo `<Form>`. `Copy`: o registro é `!Send` e vive num
/// `StoredValue::new_local` (ver `docs/guide/pitfalls.md`).
#[derive(Clone, Copy)]
pub(crate) struct FormContext {
    pub(crate) registry: StoredValue<Registry, LocalStorage>,
}

/// O que um campo informa ao `Form` ao se registrar com [`use_form_child`].
#[derive(Clone)]
pub struct FormChildOptions {
    /// Nome para diagnóstico.
    pub name: Option<String>,
    /// Validação: `Ok(true)` válido, `Ok(false)` inválido, `Err(msg)` falha inesperada (conta
    /// como inválido e a mensagem vai para o console). Regra síncrona é uma future já resolvida.
    pub validate: ValidateFn,
    /// Limpa o estado de validação do campo.
    pub reset_validation: Option<ResetValidationFn>,
    /// Foca o campo; devolve `true` se conseguiu. Sem ela, o campo nunca recebe o foco de erro.
    pub focus: Option<FocusFn>,
    /// Campo desabilitado não fica registrado e tem a validação limpa ao ser desabilitado.
    pub disabled: Signal<bool>,
    /// Registra um erro no console se não houver `Form` acima.
    pub requires_form: bool,
}

impl FormChildOptions {
    /// Opções com a validação dada; o restante vem desligado (habilitado, sem foco nem reset).
    pub fn new(validate: ValidateFn) -> Self {
        Self {
            name: None,
            validate,
            reset_validation: None,
            focus: None,
            disabled: Signal::stored(false),
            requires_form: false,
        }
    }
}

/// Registra o campo no `Form` mais próximo e o tira ao ser descartado.
///
/// Devolve o id no `Form`, ou `None` se não há `Form` acima (nada é feito; com
/// `requires_form`, registra um erro no console). Enquanto `disabled` for `true` o campo fica
/// fora do registro; ao ser desabilitado, sua validação é limpa. O id é o mesmo ao longo de
/// toda a vida do campo.
pub fn use_form_child(opts: FormChildOptions) -> Option<FormChildId> {
    let Some(ctx) = use_context::<FormContext>() else {
        if opts.requires_form {
            leptos::logging::error!(
                "use_form_child: o componente precisa estar dentro de um <Form>"
            );
        }
        return None;
    };

    let reset_validation = opts.reset_validation;
    let registry = ctx.registry.get_value();
    let id = registry.new_id();
    let child = FormChild {
        name: opts.name,
        validate: opts.validate,
        reset_validation: reset_validation.clone(),
        focus: opts.focus,
    };
    let disabled = opts.disabled;

    // Registra já na criação (a ordem de registro é a ordem do documento); o Effect só reage
    // às mudanças de `disabled`.
    if !disabled.get_untracked() {
        registry.bind(id, child.clone());
    }
    let registry_sv = ctx.registry;
    Effect::new(move |prev: Option<()>| {
        let is_disabled = disabled.get();
        if prev.is_none() {
            return;
        }
        let registry = registry_sv.get_value();
        if is_disabled {
            if registry.unbind(id)
                && let Some(reset) = &reset_validation
            {
                reset();
            }
        } else if !registry.is_bound(id) {
            registry.bind(id, child.clone());
        }
    });
    on_cleanup(move || {
        if let Some(registry) = registry_sv.try_get_value() {
            registry.unbind(id);
        }
    });
    Some(id)
}
