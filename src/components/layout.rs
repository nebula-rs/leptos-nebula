use leptos::{ev, prelude::*};
use wasm_bindgen::JsCast;

use crate::util::cls;

/// Estrutura da página: cabeçalho em cima, [`Drawer`] ao lado, [`PageContainer`] no meio.
/// ≈ `QLayout`, mas com estrutura fixa (**sem** `view`).
///
/// Filhos esperados: [`Header`], [`Drawer`]s, [`PageContainer`] e, opcionalmente, [`Footer`].
/// O drawer fixo se posiciona abaixo de um [`Header`]; sem cabeçalho, ajuste `--lx-header-height`.
/// Com [`Footer`], a gaveta fixa encurta pela altura dele (`--lx-footer-height`).
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

/// Rodapé fixo embaixo, dentro do [`Layout`]. ≈ `QFooter`.
///
/// Ocupa a última linha da grade do `Layout`, então o [`PageContainer`] já reserva o espaço
/// dele: o conteúdo nunca fica escondido. Fica colado na borda inferior da tela (ou da caixa
/// que rola) enquanto o conteúdo é mais alto, e respeita `env(safe-area-inset-bottom)`.
/// Altura mínima por `--lx-footer-height`. Para uma barra fora do `Layout`, veja [`BottomBar`].
///
/// Ainda não: `reveal`, `model_value`, `height_hint`.
#[component]
pub fn Footer(
    #[prop(optional)] bordered: bool,
    #[prop(optional)] elevated: bool,
    children: Children,
) -> impl IntoView {
    let class = cls([
        "lx-footer",
        if bordered { "lx-footer-bordered" } else { "" },
        if elevated { "lx-footer-elevated" } else { "" },
    ]);
    view! { <footer class=class>{children()}</footer> }
}

/// Classe que reserva embaixo o espaço do [`BottomBar`] (`--lx-bottom-bar-height` + área segura
/// + respiro). Aplique ao contêiner do conteúdo: `class=BOTTOM_BAR_SPACE`.
///
/// É uma constante literal para o gerador de CSS sempre enxergar o shortcut
/// `lx-bottom-bar-space`.
pub const BOTTOM_BAR_SPACE: &str = "lx-bottom-bar-space";

/// Barra de navegação/ações fixa no rodapé da tela, para uso **fora** do [`Layout`]
/// (apps mobile). Sem equivalente no Quasar.
///
/// O conteúdo (`children`) fica centralizado até `--lx-bottom-bar-max-width` e a barra respeita
/// `env(safe-area-inset-bottom)`. Por ser `position: fixed`, ela cobre o fim da página: dê ao
/// contêiner do conteúdo a classe [`BOTTOM_BAR_SPACE`] (`lx-bottom-bar-space`, padding de baixo =
/// `--lx-bottom-bar-height` + área segura). Para conter a barra numa caixa (exemplos), a caixa
/// precisa de `transform: translateZ(0)`.
#[component]
pub fn BottomBar(
    /// Linha no topo da barra.
    #[prop(default = true)]
    bordered: bool,
    #[prop(optional)] elevated: bool,
    /// Nome acessível da navegação.
    #[prop(default = "Navegação")]
    aria_label: &'static str,
    children: Children,
) -> impl IntoView {
    let class = cls([
        "lx-bottom-bar",
        if bordered {
            "lx-bottom-bar-bordered"
        } else {
            ""
        },
        if elevated {
            "lx-bottom-bar-elevated"
        } else {
            ""
        },
    ]);
    view! {
        <nav class=class aria-label=aria_label>
            <div class="lx-bottom-bar-inner">{children()}</div>
        </nav>
    }
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
