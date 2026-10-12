use leptos::prelude::*;

use crate::util::cls;

/// Ícone lucide em CSS puro. ≈ `QIcon`.
///
/// `name` é a classe **literal** `i-lucide-<nome>` (o UnoCSS lê o fonte como texto).
///
/// Ainda não: `left`/`right`, `color` (herda `currentColor`), ícones de outras coleções, imagens.
#[component]
pub fn Icon(
    /// Classe literal do ícone, ex.: `"i-lucide-menu"`.
    name: &'static str,
    /// Tamanho CSS (ex.: `"24px"`, `"1.5em"`); padrão: o do texto.
    #[prop(optional, into)]
    size: Option<String>,
) -> impl IntoView {
    let style = size.map(|s| format!("font-size:{s}"));
    view! { <i class=cls(["lx-icon", name]) style=style aria-hidden="true"></i> }
}
