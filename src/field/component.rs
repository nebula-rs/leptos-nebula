//! `<Field>`, os slots e o `FieldContext` (≈ `QField` / `use-field` do Quasar).

use std::rc::Rc;

use leptos::{ev, html, prelude::*};
use wasm_bindgen::JsCast;

use super::state::{
    Bottom, FieldDesign, bottom_class, bottom_content, control_class, counter_text, editable_class,
    has_bottom, is_floating, label_class, label_space_class, placeholder_class, size_class,
};
use super::validate::{has_error, pick_message};
use super::validation::FieldValidation;
use crate::{Color, Icon, Spinner, util::cls};

/// Slot `prepend` do [`Field`]: dentro da caixa, antes do controle.
/// `<FieldPrepend slot:prepend>…</FieldPrepend>`.
#[slot]
pub struct FieldPrepend {
    pub(crate) children: ChildrenFn,
}

/// Slot `append` do [`Field`]: dentro da caixa, depois do controle.
/// `<FieldAppend slot:append>…</FieldAppend>`.
#[slot]
pub struct FieldAppend {
    pub(crate) children: ChildrenFn,
}

/// Slot `before` do [`Field`]: fora da caixa, à esquerda. `<FieldBefore slot:before>…</FieldBefore>`.
#[slot]
pub struct FieldBefore {
    pub(crate) children: ChildrenFn,
}

/// Slot `after` do [`Field`]: fora da caixa, à direita. `<FieldAfter slot:after>…</FieldAfter>`.
#[slot]
pub struct FieldAfter {
    pub(crate) children: ChildrenFn,
}

/// Slot de hint do [`Field`]: no rodapé; tem precedência sobre a prop `hint`.
/// `<FieldHint slot:hint_slot>…</FieldHint>` (o nome da prop é `hint_slot`, porque `hint` já é
/// o texto).
#[slot]
pub struct FieldHint {
    pub(crate) children: ChildrenFn,
}

/// Slot de erro do [`Field`]: no rodapé, no lugar do hint quando há erro; tem precedência sobre
/// a mensagem. `<FieldError slot:error_slot>…</FieldError>` (o nome da prop é `error_slot`,
/// porque `error` já é o sinal de erro).
#[slot]
pub struct FieldError {
    pub(crate) children: ChildrenFn,
}

/// Slot do contador do [`Field`]: no canto direito do rodapé, no lugar do texto `n / max`.
/// `<FieldCounter slot:counter_slot>…</FieldCounter>` (o nome da prop é `counter_slot`, porque
/// `counter` já é o booleano). Com o slot, o rodapé existe mesmo sem `counter`.
#[slot]
pub struct FieldCounter {
    pub(crate) children: ChildrenFn,
}

/// Slot de carregamento do [`Field`]: no lugar do `Spinner`, antes do `append`, enquanto
/// `loading` (ou uma validação assíncrona) está ativo.
/// `<FieldLoading slot:loading_slot>…</FieldLoading>` (a prop é `loading_slot`: `loading` já é o
/// sinal).
#[slot]
pub struct FieldLoading {
    pub(crate) children: ChildrenFn,
}

/// O que o [`Field`] entrega ao controle que ele contém (o `Input` da R-16 o usa).
///
/// Obtenha com [`use_field_context`] dentro dos `children` do `Field`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FieldContext {
    /// `id` do controle nativo (o `for` do label). `None` se o `Field` não recebeu `for_`.
    pub for_: Option<String>,
    /// O campo tem foco (`focusin` na caixa; nunca `true` com `disable`).
    pub focused: Signal<bool>,
    /// Campo desabilitado.
    pub disable: Signal<bool>,
    /// Campo somente leitura.
    pub readonly: Signal<bool>,
    /// `id` do hint no rodapé, para o controle usar em `aria-describedby`. `None` sem `for_` ou
    /// sem hint.
    pub hint_id: Option<String>,
    /// Há erro visível (regras ou `error` externo): o controle pode marcar `aria-invalid`.
    pub has_error: Signal<bool>,
    /// `id` da mensagem de erro no rodapé, para `aria-describedby`. `None` sem `for_`.
    pub error_id: Option<String>,
    /// Classe do controle nativo (`<input>`, `<select>`, `<textarea>`): sem borda nem fundo,
    /// herda fonte e cor, e aplica a opacidade do `placeholder` conforme o label.
    pub control_class: &'static str,
}

/// O [`FieldContext`] do `Field` mais próximo, se houver.
pub fn use_field_context() -> Option<FieldContext> {
    use_context::<FieldContext>()
}

/// Seletor dos elementos que podem receber o foco ao clicar no quadro.
const FOCUSABLE: &str =
    "input:not([disabled]):not([type=hidden]), select:not([disabled]), textarea:not([disabled])";

/// Quadro do campo: label, hint, prefix/suffix, slots e design. ≈ `QField`. O controle (um
/// `<input>` nativo, depois o `Input`) entra como `children`; o `has_value` diz ao `Field` se o
/// controle tem valor (quem guarda o valor é o controle).
///
/// O label é definido pelo tema ([ADR-0010](../../../docs/decisions/0010-field-label-by-theme.md)):
/// um único elemento `<label for=…>` com dois estados (repouso × flutuando); no scandi fica fixo
/// acima da caixa. O padrão de `design` é `Outlined` (no Quasar, `standard`).
///
/// Sem `for_`, nenhum id é gerado e o label não se associa ao controle: passe `for_` com o `id`
/// do `<input>` (o [`Input`](crate::Input) já faz isso, e gera o id se faltar).
///
/// Validação: `validation` (de [`use_validate`](crate::use_validate)) e/ou o `error` e
/// `error_message` externos. Com erro, o rodapé mostra a mensagem (ou o slot `error_slot`) no
/// lugar do hint, a caixa fica na cor `negative` e um ícone aparece no fim (`no_error_icon` o
/// esconde). Com `validation`, o `focusout` valida (conforme `lazy_rules`) e o `Form` foca o
/// campo inválido.
///
/// `counter` mostra `n` (ou `n / maxlength`) no canto direito do rodapé; quem sabe o tamanho do
/// valor é o controle (`value_length`, em unidades UTF-16, como o atributo `maxlength`). `loading`
/// (ou uma validação assíncrona em curso) mostra um [`Spinner`] depois do ícone de erro, ou o slot
/// `loading_slot`.
///
/// Ainda não: `reactive_rules`, `max_values`, `rounded`, `square`, `bg_color`, `label_color`,
/// `item_aligned`, `dark`, `autofocus`, o slot `label` (`clearable` está no
/// [`Input`](crate::Input)), designs `Standard`/`Standout`.
#[component]
pub fn Field(
    /// Texto do label.
    #[prop(optional, into)]
    label: Option<String>,
    /// Mantém o label sempre na posição "flutuando".
    #[prop(optional)]
    stack_label: bool,
    /// Texto de ajuda no rodapé.
    #[prop(optional, into)]
    hint: Option<String>,
    /// O hint só aparece com foco.
    #[prop(optional)]
    hide_hint: bool,
    /// Texto antes do controle, dentro da caixa.
    #[prop(optional, into)]
    prefix: Option<String>,
    /// Texto depois do controle, dentro da caixa.
    #[prop(optional, into)]
    suffix: Option<String>,
    #[prop(optional)] design: FieldDesign,
    #[prop(optional)] dense: bool,
    /// Cor do foco.
    #[prop(optional)]
    color: Color,
    #[prop(optional, into)] disable: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    /// `id` do controle nativo (o `for` do label).
    #[prop(optional, into)]
    for_: Option<String>,
    /// O controle tem valor (o "filled" do Quasar): faz o label flutuar.
    #[prop(optional, into)]
    has_value: Signal<bool>,
    /// Reserva o rodapé mesmo sem hint (evita o layout pular quando um texto aparece).
    #[prop(optional)]
    bottom_slots: bool,
    /// Não reserva a altura do rodapé.
    #[prop(optional)]
    hide_bottom_space: bool,
    /// Validação do campo ([`use_validate`](crate::use_validate)).
    #[prop(optional)]
    validation: Option<FieldValidation>,
    /// Erro externo: `Some(true)` mostra o estado de erro (soma-se ao das regras).
    #[prop(optional, into)]
    error: Signal<Option<bool>>,
    /// Mensagem de erro externa; não vazia, vence a das regras.
    #[prop(optional, into)]
    error_message: Signal<String>,
    /// Não mostra o ícone de erro.
    #[prop(optional)]
    no_error_icon: bool,
    /// Contador no rodapé: `n` ou `n / maxlength` (≈ `counter` do Quasar).
    #[prop(optional)]
    counter: bool,
    /// Tamanho do valor para o contador, em unidades UTF-16 (quem guarda o valor é o controle).
    #[prop(optional, into)]
    value_length: Signal<usize>,
    /// Limite mostrado no contador (`n / max`).
    #[prop(optional)]
    maxlength: Option<u32>,
    /// Mostra o `Spinner` (ou o slot `loading_slot`) no fim da caixa. Soma-se à validação
    /// assíncrona em curso.
    #[prop(optional, into)]
    loading: Signal<bool>,
    #[prop(optional)] counter_slot: Option<FieldCounter>,
    #[prop(optional)] loading_slot: Option<FieldLoading>,
    #[prop(optional)] prepend: Option<FieldPrepend>,
    #[prop(optional)] append: Option<FieldAppend>,
    #[prop(optional)] before: Option<FieldBefore>,
    #[prop(optional)] after: Option<FieldAfter>,
    #[prop(optional)] hint_slot: Option<FieldHint>,
    #[prop(optional)] error_slot: Option<FieldError>,
    /// O controle (slot default).
    children: Children,
) -> impl IntoView {
    let focused_raw = RwSignal::new(false);
    // Um campo desabilitado nunca conta como focado.
    let focused = Signal::derive(move || focused_raw.get() && !disable.get());
    let floating = Signal::derive(move || is_floating(focused.get(), has_value.get(), stack_label));

    let has_label = label.is_some();
    let has_hint = hint.is_some() || hint_slot.is_some();
    // O rodapé existe com hint, `bottom_slots`, `validation`, slot de erro ou contador. Com só o
    // `error` externo, use `bottom_slots` para ele aparecer.
    let has_counter = counter || counter_slot.is_some();
    let has_bottom = has_bottom(
        bottom_slots,
        has_hint,
        validation.is_some(),
        error_slot.is_some(),
        has_counter,
    );
    let is_loading =
        Signal::derive(move || loading.get() || validation.is_some_and(|v| v.loading().get()));
    let hint_id = (has_hint)
        .then(|| for_.as_ref().map(|f| format!("{f}_hint")))
        .flatten();
    let error_id = for_.as_ref().map(|f| format!("{f}_error"));
    let has_err = Signal::derive(move || {
        has_error(error.get(), validation.is_some_and(|v| v.has_error().get()))
    });
    let err_message = Signal::derive(move || {
        pick_message(
            &error_message.get(),
            validation.and_then(|v| v.error_message().get()).as_deref(),
        )
    });
    let box_ref = NodeRef::<html::Div>::new();
    if let Some(v) = validation {
        v.set_focus(Rc::new(move || {
            box_ref
                .get_untracked()
                .and_then(|el| el.query_selector(FOCUSABLE).ok().flatten())
                .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
                .is_some_and(|el| el.focus().is_ok())
        }));
        on_cleanup(move || v.clear_focus());
    }

    provide_context(FieldContext {
        for_: for_.clone(),
        focused,
        disable,
        readonly,
        hint_id: hint_id.clone(),
        has_error: has_err,
        error_id: error_id.clone(),
        control_class: "lx-field-native",
    });
    let control = children();

    let root_class = move || {
        cls([
            "lx-field",
            size_class(dense),
            label_space_class(has_label),
            editable_class(disable.get(), readonly.get()),
            placeholder_class(has_label, floating.get()),
            color.tone_class(),
        ])
    };
    let box_class = move || {
        cls([
            "lx-field-box",
            control_class(design, focused.get(), has_err.get()),
        ])
    };

    let on_focusin = move |_: ev::FocusEvent| {
        if !disable.get_untracked() {
            focused_raw.set(true);
        }
    };
    // Perde o foco só se o novo alvo está fora da caixa (prepend/append contam como dentro).
    let on_focusout = move |e: ev::FocusEvent| {
        let inside = e
            .related_target()
            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
            .zip(
                e.current_target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok()),
            )
            .is_some_and(|(related, boxed)| boxed.contains(Some(&related)));
        if !inside {
            focused_raw.set(false);
            if let Some(v) = validation {
                v.on_blur();
            }
        }
    };
    // Clicar no recuo da caixa (fora de qualquer controle) foca o controle nativo, como o
    // `<label>` raiz do Quasar.
    let on_click = move |e: ev::MouseEvent| {
        if disable.get_untracked() {
            return;
        }
        let Some(boxed) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        let on_interactive = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|t| {
                t.closest("input, select, textarea, button, a[href], [tabindex]")
                    .ok()
                    .flatten()
            })
            .is_some();
        if on_interactive {
            return;
        }
        if let Ok(Some(el)) = boxed.query_selector(FOCUSABLE)
            && let Ok(el) = el.dyn_into::<web_sys::HtmlElement>()
        {
            let _ = el.focus();
        }
    };

    let label_view = label.map(|text| {
        view! {
            <label
                for=for_
                class=move || label_class(floating.get())
            >
                {text}
            </label>
        }
    });
    let counter_view = has_counter.then(|| {
        let content = match &counter_slot {
            Some(slot) => (slot.children)().into_any(),
            None => (move || counter_text(value_length.get(), maxlength)).into_any(),
        };
        view! { <div class="lx-field-counter">{content}</div> }
    });
    let bottom_view = has_bottom.then(|| {
        view! {
            <div class=cls(["lx-field-bottom", bottom_class(hide_bottom_space)])>
                <div class="lx-field-messages">
                {move || match bottom_content(has_err.get(), has_hint, hide_hint, focused.get()) {
                    Bottom::Error => {
                        let content = match (&error_slot, err_message.get()) {
                            (Some(slot), _) => Some((slot.children)().into_any()),
                            (None, Some(text)) => Some(text.into_any()),
                            (None, None) => None,
                        };
                        content
                            .map(|content| {
                                view! {
                                    <div class="lx-field-error" role="alert" id=error_id.clone()>
                                        {content}
                                    </div>
                                }
                                    .into_any()
                            })
                    }
                    Bottom::Hint => {
                        let content = match (&hint_slot, &hint) {
                            (Some(slot), _) => (slot.children)().into_any(),
                            (None, Some(text)) => text.clone().into_any(),
                            (None, None) => ().into_any(),
                        };
                        Some(
                            view! {
                                <div class="lx-field-hint" id=hint_id.clone()>
                                    {content}
                                </div>
                            }
                                .into_any(),
                        )
                    }
                    Bottom::Nothing => None,
                }}
                </div>
                {counter_view}
            </div>
        }
    });
    let error_icon = move || {
        (has_err.get() && !no_error_icon).then(|| {
            view! {
                <div class="lx-field-error-icon">
                    <Icon name="i-lucide-circle-alert" />
                </div>
            }
        })
    };

    let loading_view = move || {
        is_loading.get().then(|| {
            let content = match &loading_slot {
                Some(slot) => (slot.children)().into_any(),
                None => view! { <Spinner color=color /> }.into_any(),
            };
            view! { <div class="lx-field-marginal">{content}</div> }
        })
    };

    view! {
        <div class=root_class aria-disabled=move || disable.get().then_some("true")>
            {before.map(|b| view! { <div class="lx-field-side">{(b.children)()}</div> })}
            <div class="lx-field-inner">
                {label_view}
                <div
                    node_ref=box_ref
                    class=box_class
                    on:focusin=on_focusin
                    on:focusout=on_focusout
                    on:click=on_click
                >
                    {prepend.map(|p| view! { <div class="lx-field-marginal">{(p.children)()}</div> })}
                    <div class="lx-field-control-container">
                        {prefix.map(|t| view! { <div class="lx-field-prefix">{t}</div> })}
                        {control}
                        {suffix.map(|t| view! { <div class="lx-field-suffix">{t}</div> })}
                    </div>
                    {error_icon}
                    {loading_view}
                    {append.map(|a| view! { <div class="lx-field-marginal">{(a.children)()}</div> })}
                </div>
                {bottom_view}
            </div>
            {after.map(|a| view! { <div class="lx-field-side">{(a.children)()}</div> })}
        </div>
    }
    .into_any()
}
