use leptos::{ev, prelude::*};
use wasm_bindgen::JsCast;

use crate::util::cls;

/// Estrutura da página: cabeçalho em cima, [`Drawer`] ao lado, [`PageContainer`] no meio.
/// ≈ `QLayout`, mas com estrutura fixa (**sem** `view`).
///
/// Filhos esperados: [`Header`], [`Drawer`]s e [`PageContainer`].
/// O drawer fixo se posiciona abaixo de um [`Header`]; sem cabeçalho, ajuste `--lx-header-height`.
#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! { <div class="lx-layout">{children()}</div> }
}

/// Cabeçalho fixo no topo. ≈ `QHeader`.
///
/// Ainda não: `reveal`, `model_value`, `height_hint`.
#[component]
pub fn Header(
    #[prop(optional)] bordered: bool,
    #[prop(optional)] elevated: bool,
    children: Children,
) -> impl IntoView {
    let class = cls([
        "lx-header",
        if bordered { "lx-header-bordered" } else { "" },
        if elevated { "lx-header-elevated" } else { "" },
    ]);
    view! { <header class=class>{children()}</header> }
}

/// Barra de ferramentas. ≈ `QToolbar`.
///
/// Ainda não: `inset`.
#[component]
pub fn Toolbar(children: Children) -> impl IntoView {
    view! { <div class="lx-toolbar" role="toolbar">{children()}</div> }
}

/// Título dentro da [`Toolbar`] (ocupa o espaço restante). ≈ `QToolbarTitle`.
///
/// Ainda não: `shrink`.
#[component]
pub fn ToolbarTitle(children: Children) -> impl IntoView {
    view! { <div class="lx-toolbar-title">{children()}</div> }
}

/// Lado do [`Drawer`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawerSide {
    #[default]
    Left,
    Right,
}

/// Gaveta lateral. ≈ `QDrawer`.
///
/// Acima do `breakpoint` fica fixa na coluna; abaixo, sobrepõe a página com fundo escuro e
/// fecha ao clicar fora ou com `Esc`. Como o breakpoint só é medido no navegador, a renderização
/// no servidor assume tela larga: passe `show = true` para a gaveta já vir aberta no SSG e
/// use `show_if_above` para ela fechar sozinha em tela estreita.
///
/// Ainda não: `overlay`, `behavior`, `mini`, `persistent`, `no_swipe_*`, `elevated`, `mini_width`.
#[component]
pub fn Drawer(
    /// Estado aberto/fechado (`v-model` do Quasar).
    show: RwSignal<bool>,
    #[prop(optional)] side: DrawerSide,
    /// Largura em px.
    #[prop(default = 300)]
    width: u32,
    /// Largura de tela (px) acima da qual a gaveta é fixa.
    #[prop(default = 1023)]
    breakpoint: u32,
    #[prop(optional)] bordered: bool,
    /// Abre ao cruzar o breakpoint para cima e fecha ao cruzar para baixo.
    #[prop(optional)]
    show_if_above: bool,
    children: Children,
) -> impl IntoView {
    // Assume tela larga até o navegador medir (SSR).
    let wide = RwSignal::new(true);

    Effect::new(move |_| {
        let query = format!("(min-width: {}px)", breakpoint + 1);
        if let Some(mql) = web_sys::window().and_then(|w| w.match_media(&query).ok().flatten()) {
            wide.set(mql.matches());
            if show_if_above {
                show.set(mql.matches());
            }
            let on_change =
                wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MediaQueryListEvent)>::new(
                    move |e: web_sys::MediaQueryListEvent| {
                        wide.set(e.matches());
                        if show_if_above {
                            show.set(e.matches());
                        }
                    },
                );
            let _ =
                mql.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
            // O layout vive o tempo todo do app; o listener também.
            on_change.forget();
        }
        // Esc fecha a gaveta sobreposta.
        let _ = window_event_listener(ev::keydown, move |e| {
            if e.key() == "Escape" && !wide.get_untracked() && show.get_untracked() {
                show.set(false);
            }
        });
    });

    let class = move || {
        let mode = match (show.get(), wide.get(), side) {
            (false, _, _) => "lx-drawer-closed",
            (true, true, DrawerSide::Left) => "lx-drawer-fixed-left",
            (true, true, DrawerSide::Right) => "lx-drawer-fixed-right",
            (true, false, DrawerSide::Left) => "lx-drawer-overlay-left",
            (true, false, DrawerSide::Right) => "lx-drawer-overlay-right",
        };
        let border = match (bordered, side) {
            (false, _) => "",
            (true, DrawerSide::Left) => "lx-drawer-border-r",
            (true, DrawerSide::Right) => "lx-drawer-border-l",
        };
        cls([mode, border])
    };

    view! {
        <Show when=move || show.get() && !wide.get()>
            <div class="lx-drawer-backdrop" on:click=move |_| show.set(false)></div>
        </Show>
        <aside class=class style=format!("width:{width}px")>
            {children()}
        </aside>
    }
}

/// Área da página (ao lado das gavetas). ≈ `QPageContainer`.
#[component]
pub fn PageContainer(children: Children) -> impl IntoView {
    view! { <main class="lx-page-container">{children()}</main> }
}

/// Uma página. ≈ `QPage`.
///
/// Ainda não: `style_fn`, `padding` como tamanho (aqui é `bool`).
#[component]
pub fn Page(#[prop(optional)] padding: bool, children: Children) -> impl IntoView {
    let class = if padding {
        "lx-page-padded"
    } else {
        "lx-page-flush"
    };
    view! { <div class=class>{children()}</div> }
}
