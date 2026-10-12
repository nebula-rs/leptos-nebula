//! Entrega do CSS (ADR-0005): o crate carrega o próprio `ui.css`.

use leptos::prelude::*;
use leptos_meta::Style;

/// CSS pronto do crate (reset, tokens, tema, base e shortcuts dos componentes).
/// Gerado por `moon run leptos-nebula:css` — nunca editar `style/dist/ui.css` à mão.
pub const CSS: &str = include_str!("../style/dist/ui.css");

/// Injeta [`CSS`] no `<head>` via `leptos_meta`. Coloque uma vez na raiz do app
/// (requer `provide_meta_context()`). Um tema próprio do app é um CSS de tokens extra,
/// carregado **depois** deste.
#[component]
pub fn UiStyles() -> impl IntoView {
    view! { <Style>{CSS}</Style> }
}
