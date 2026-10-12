use leptos::{ev, prelude::*};

use super::Icon;
use crate::{Color, util::cls};

/// Aparência do botão (exclusivas). `Unelevated` é o padrão.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BtnDesign {
    #[default]
    Unelevated,
    Outline,
    Flat,
}

/// Atributo `type` do `<button>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BtnType {
    #[default]
    Button,
    Submit,
    Reset,
}

/// Botão. ≈ `QBtn`. Com `href` renderiza `<a>`.
///
/// Ainda não: `size`, `loading`, `percentage`, `stack`, `no_caps`, `ripple`, `to`/`replace`,
/// `padding`, `glossy`, `push`, `square`, `fab`, `no_wrap`, `align`, `tabindex`, `repeat`.
/// `type` é `type_` (palavra reservada).
#[component]
pub fn Btn(
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] icon: Option<&'static str>,
    #[prop(optional)] icon_right: Option<&'static str>,
    #[prop(optional)] color: Color,
    #[prop(optional)] design: BtnDesign,
    #[prop(optional)] round: bool,
    #[prop(optional)] dense: bool,
    #[prop(optional, into)] href: Option<String>,
    #[prop(optional, into)] target: Option<String>,
    #[prop(optional)] type_: BtnType,
    #[prop(optional, into)] disable: Signal<bool>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] on_click: Option<Callback<ev::MouseEvent>>,
    /// Conteúdo (slot default), depois do `label`.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let size = match (round, dense) {
        (false, false) => "lx-btn-size-normal",
        (false, true) => "lx-btn-size-dense",
        (true, false) => "lx-btn-size-round",
        (true, true) => "lx-btn-size-round-dense",
    };
    let design = match design {
        BtnDesign::Unelevated => "lx-btn-unelevated",
        BtnDesign::Outline => "lx-btn-outline",
        BtnDesign::Flat => "lx-btn-flat",
    };
    let tone = color.tone_class();
    let class = move || {
        cls([
            "lx-btn",
            size,
            design,
            tone,
            if disable.get() { "lx-btn-disabled" } else { "" },
        ])
    };
    let content = view! {
        {icon.map(|name| view! { <Icon name=name /> })}
        {label.map(|l| view! { <span>{l}</span> })}
        {children.map(|c| c())}
        {icon_right.map(|name| view! { <Icon name=name /> })}
    };
    let on_click = move |e: ev::MouseEvent| {
        if let Some(cb) = on_click {
            cb.run(e);
        }
    };
    match href {
        Some(href) => view! {
            <a
                class=class
                href=href
                target=target
                aria-label=aria_label
                aria-disabled=move || disable.get().then_some("true")
                on:click=on_click
            >
                {content}
            </a>
        }
        .into_any(),
        None => {
            let kind = match type_ {
                BtnType::Button => "button",
                BtnType::Submit => "submit",
                BtnType::Reset => "reset",
            };
            view! {
                <button
                    class=class
                    type=kind
                    aria-label=aria_label
                    disabled=move || disable.get()
                    on:click=on_click
                >
                    {content}
                </button>
            }
            .into_any()
        }
    }
}
