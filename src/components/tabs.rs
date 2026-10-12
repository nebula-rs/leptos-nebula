use leptos::prelude::*;

use super::Icon;
use crate::{Align, util::cls};

#[derive(Clone, Copy)]
struct TabsCtx {
    value: RwSignal<String>,
    dense: bool,
}

/// Barra de abas. ≈ `QTabs`. `value` é o `name` da aba ativa (`v-model`).
///
/// Ainda não: `vertical`, `inline_label`, `indicator_color`, `active_color`, `no_caps`, `shrink`,
/// `stretch`, `switch_indicator`, setas de rolagem, `on_update:model_value` (observe o sinal).
#[component]
pub fn Tabs(
    value: RwSignal<String>,
    #[prop(optional)] align: Align,
    #[prop(optional)] dense: bool,
    children: Children,
) -> impl IntoView {
    provide_context(TabsCtx { value, dense });
    let align = match align {
        Align::Left => "lx-tabs-align-left",
        Align::Center => "lx-tabs-align-center",
        Align::Right => "lx-tabs-align-right",
        Align::Between => "lx-tabs-align-between",
        Align::Around => "lx-tabs-align-around",
        Align::Evenly => "lx-tabs-align-evenly",
        Align::Justify => "lx-tabs-align-justify",
    };
    view! { <div class=cls(["lx-tabs", align]) role="tablist">{children()}</div> }
}

/// Aba. ≈ `QTab`.
///
/// Ainda não: `alert`, `content_class`, `ripple`, `no_caps`, `disable`.
#[component]
pub fn Tab(
    #[prop(into)] name: String,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] icon: Option<&'static str>,
) -> impl IntoView {
    let ctx = expect_context::<TabsCtx>();
    let size = if ctx.dense {
        "lx-tab-size-dense"
    } else {
        "lx-tab-size-normal"
    };
    let is_active = {
        let name = name.clone();
        move || ctx.value.get() == name
    };
    let class = {
        let is_active = is_active.clone();
        move || {
            cls([
                "lx-tab",
                size,
                if is_active() {
                    "lx-tab-active"
                } else {
                    "lx-tab-idle"
                },
            ])
        }
    };
    view! {
        <button
            class=class
            type="button"
            role="tab"
            aria-selected=move || is_active().to_string()
            on:click=move |_| ctx.value.set(name.clone())
        >
            {icon.map(|n| view! { <Icon name=n /> })}
            {label}
        </button>
    }
}

#[derive(Clone, Copy)]
struct PanelsCtx {
    value: RwSignal<String>,
}

/// Contêiner dos painéis. ≈ `QTabPanels`. Mostra só o [`TabPanel`] cujo `name` é o `value`.
///
/// Ainda não: `animated`, `swipeable`, `keep_alive`, `vertical`, `infinite`.
#[component]
pub fn TabPanels(value: RwSignal<String>, children: Children) -> impl IntoView {
    provide_context(PanelsCtx { value });
    view! { <div class="lx-tab-panels">{children()}</div> }
}

/// Painel. ≈ `QTabPanel`. O conteúdo é recriado a cada exibição.
#[component]
pub fn TabPanel(#[prop(into)] name: String, children: ChildrenFn) -> impl IntoView {
    let ctx = expect_context::<PanelsCtx>();
    view! {
        <Show when=move || ctx.value.get() == name>
            <div class="lx-tab-panel" role="tabpanel">
                {children()}
            </div>
        </Show>
    }
}
