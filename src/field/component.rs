//! `<Field>`, os slots e o `FieldContext` (≈ `QField` / `use-field` do Quasar).

use leptos::{ev, prelude::*};
use wasm_bindgen::JsCast;

use super::state::{
    FieldDesign, bottom_class, control_class, editable_class, hint_visible, is_floating,
    label_class, label_space_class, placeholder_class, size_class,
};
use crate::{Color, util::cls};

/// Slot `prepend` do [`Field`]: dentro da caixa, antes do controle.
/// `<FieldPrepend slot:prepend>…</FieldPrepend>`.
#[slot]
pub struct FieldPrepend {
    children: ChildrenFn,
}

/// Slot `append` do [`Field`]: dentro da caixa, depois do controle.
/// `<FieldAppend slot:append>…</FieldAppend>`.
#[slot]
pub struct FieldAppend {
    children: ChildrenFn,
}

/// Slot `before` do [`Field`]: fora da caixa, à esquerda. `<FieldBefore slot:before>…</FieldBefore>`.
#[slot]
pub struct FieldBefore {
    children: ChildrenFn,
}

/// Slot `after` do [`Field`]: fora da caixa, à direita. `<FieldAfter slot:after>…</FieldAfter>`.
#[slot]
pub struct FieldAfter {
    children: ChildrenFn,
}

/// Slot de hint do [`Field`]: no rodapé; tem precedência sobre a prop `hint`.
/// `<FieldHint slot:hint_slot>…</FieldHint>` (o nome da prop é `hint_slot`, porque `hint` já é
/// o texto).
#[slot]
pub struct FieldHint {
    children: ChildrenFn,
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
/// O label é definido pelo tema ([ADR-0010](../../../docs/decisions/0010-field-label-pelo-tema.md)):
/// um único elemento `<label for=…>` com dois estados (repouso × flutuando); no scandi fica fixo
/// acima da caixa. O padrão de `design` é `Outlined` (no Quasar, `standard`).
///
/// Sem `for_`, nenhum id é gerado e o label não se associa ao controle: passe `for_` com o `id`
/// do `<input>`.
///
/// Ainda não: validação (`rules`, `error`, `error_message`), `clearable`, `counter`, `loading`,
/// `rounded`, `square`, `bg_color`, `label_color`, `item_aligned`, `dark`, `autofocus`, os slots
/// `label`/`error`/`counter`/`loading`, designs `Standard`/`Standout`.
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
    #[prop(optional)] prepend: Option<FieldPrepend>,
    #[prop(optional)] append: Option<FieldAppend>,
    #[prop(optional)] before: Option<FieldBefore>,
    #[prop(optional)] after: Option<FieldAfter>,
    #[prop(optional)] hint_slot: Option<FieldHint>,
    /// O controle (slot default).
    children: Children,
) -> impl IntoView {
    let focused_raw = RwSignal::new(false);
    // Um campo desabilitado nunca conta como focado.
    let focused = Signal::derive(move || focused_raw.get() && !disable.get());
    let floating = Signal::derive(move || is_floating(focused.get(), has_value.get(), stack_label));

    let has_label = label.is_some();
    let has_hint = hint.is_some() || hint_slot.is_some();
    let has_bottom = bottom_slots || has_hint;
    let hint_id = (has_hint)
        .then(|| for_.as_ref().map(|f| format!("{f}_hint")))
        .flatten();

    provide_context(FieldContext {
        for_: for_.clone(),
        focused,
        disable,
        readonly,
        hint_id: hint_id.clone(),
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
    let box_class = move || cls(["lx-field-box", control_class(design, focused.get())]);

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
    let bottom_view = has_bottom.then(|| {
        view! {
            <div class=cls(["lx-field-bottom", bottom_class(hide_bottom_space)])>
                {move || {
                    (has_hint && hint_visible(hide_hint, focused.get()))
                        .then(|| {
                            let content = match (&hint_slot, &hint) {
                                (Some(slot), _) => (slot.children)().into_any(),
                                (None, Some(text)) => text.clone().into_any(),
                                (None, None) => ().into_any(),
                            };
                            view! {
                                <div class="lx-field-hint" id=hint_id.clone()>
                                    {content}
                                </div>
                            }
                        })
                }}
            </div>
        }
    });

    view! {
        <div class=root_class aria-disabled=move || disable.get().then_some("true")>
            {before.map(|b| view! { <div class="lx-field-side">{(b.children)()}</div> })}
            <div class="lx-field-inner">
                {label_view}
                <div
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
                    {append.map(|a| view! { <div class="lx-field-marginal">{(a.children)()}</div> })}
                </div>
                {bottom_view}
            </div>
            {after.map(|a| view! { <div class="lx-field-side">{(a.children)()}</div> })}
        </div>
    }
    .into_any()
}
