use leptos::{ev, prelude::*};

use super::menu::MenuCtx;
use crate::util::cls;

#[derive(Clone, Copy)]
struct ListCtx {
    dense: bool,
}

/// Lista. ≈ `QList`. Os [`Item`]s herdam `dense`.
///
/// Dentro de um [`Menu`](super::Menu) a lista é o conteúdo do menu: ela deixa de ser `role="list"`
/// (o `Menu` passa a ser o `role="menu"`) e os `Item`s clicáveis viram `menuitem`s.
///
/// Ainda não: `dark`, `tag`.
#[component]
pub fn List(
    #[prop(optional)] bordered: bool,
    #[prop(optional)] separator: bool,
    #[prop(optional)] dense: bool,
    #[prop(optional)] padding: bool,
    children: Children,
) -> impl IntoView {
    provide_context(ListCtx { dense });
    let in_menu = use_context::<MenuCtx>();
    if let Some(menu) = in_menu {
        menu.is_menu.set(true);
    }
    let class = cls([
        "lx-list",
        if bordered { "lx-list-bordered" } else { "" },
        if separator { "lx-list-separator" } else { "" },
        if padding { "lx-list-padding" } else { "" },
    ]);
    // `none`: o papel `menu` está no popup do `Menu`; esta caixa só repassa os `menuitem`s
    let role = if in_menu.is_some() { "none" } else { "list" };
    view! { <div class=class role=role>{children()}</div> }
}

/// Item de lista. ≈ `QItem`.
///
/// Com `href` renderiza `<a>` (e é clicável). Navegação interna funciona pelo `Router` do app,
/// que intercepta cliques em `<a>` (não depende de `leptos_router` aqui).
///
/// Ainda não: `to`, `tag`, `focused`, `manual_focus`, `dark`, `disable`, `target`.
#[component]
pub fn Item(
    #[prop(optional, into)] href: Option<String>,
    #[prop(optional, into)] active: Signal<bool>,
    #[prop(optional)] clickable: bool,
    #[prop(optional)] dense: bool,
    /// Recuo em níveis de 56px (Quasar: `inset-level`).
    #[prop(optional)]
    inset_level: f32,
    #[prop(optional, into)] on_click: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let dense = dense || use_context::<ListCtx>().is_some_and(|c| c.dense);
    let interactive = clickable || href.is_some() || on_click.is_some();
    // Num `Menu`, o item clicável é um `menuitem` que o menu alcança com as setas (`tabindex=-1`).
    let (role, tabindex) = if interactive && use_context::<MenuCtx>().is_some() {
        (Some("menuitem"), Some("-1"))
    } else {
        (None, None)
    };
    let size = if dense {
        "lx-item-size-dense"
    } else {
        "lx-item-size-normal"
    };
    let class = move || {
        let state = if active.get() {
            "lx-item-active"
        } else if interactive {
            "lx-item-clickable"
        } else {
            ""
        };
        cls(["lx-item", size, state])
    };
    let style = (inset_level > 0.0).then(|| format!("padding-left:{}px", 56.0 * inset_level));
    let on_click = move |e: ev::MouseEvent| {
        if let Some(cb) = on_click {
            cb.run(e);
        }
    };
    match href {
        Some(href) => view! {
            <a class=class style=style href=href role=role tabindex=tabindex on:click=on_click>
                {children()}
            </a>
        }
        .into_any(),
        None => view! {
            <div class=class style=style role=role tabindex=tabindex on:click=on_click>
                {children()}
            </div>
        }
        .into_any(),
    }
}

/// Seção de um item. ≈ `QItemSection`. Sem flags: ocupa o espaço restante.
///
/// Ainda não: `thumbnail`, `top`, `no_wrap`.
#[component]
pub fn ItemSection(
    #[prop(optional)] avatar: bool,
    #[prop(optional)] side: bool,
    children: Children,
) -> impl IntoView {
    let class = if avatar {
        "lx-item-section-avatar"
    } else if side {
        "lx-item-section-side"
    } else {
        "lx-item-section-main"
    };
    view! { <div class=class>{children()}</div> }
}

/// Texto de um item. ≈ `QItemLabel`.
#[component]
pub fn ItemLabel(
    #[prop(optional)] header: bool,
    #[prop(optional)] caption: bool,
    /// Limita o texto a N linhas.
    #[prop(optional)]
    lines: Option<u32>,
    children: Children,
) -> impl IntoView {
    let kind = if header {
        "lx-item-label-header"
    } else if caption {
        "lx-item-label-caption"
    } else {
        "lx-item-label"
    };
    let class = cls([
        kind,
        if lines.is_some() {
            "lx-item-label-clamp"
        } else {
            ""
        },
    ]);
    let style = lines.map(|n| format!("-webkit-line-clamp:{n}"));
    view! {
        <div class=class style=style>
            {children()}
        </div>
    }
}
