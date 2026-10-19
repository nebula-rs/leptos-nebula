//! Biblioteca de componentes Leptos com temas por tokens — o que o Quasar é para o Vue.
//!
//! Componentes **parciais** (API com os nomes do Quasar, só o que o `leptos-press` precisa);
//! ver `docs/ui/overview.md`. O CSS dos componentes vem pronto ([`CSS`] / [`UiStyles`]);
//! um app que quiser os mesmos shortcuts no próprio markup usa o preset `leptos-nebula/preset`.
//!
//! Seguro para SSR: nenhum componente toca `window`/`document` durante a renderização, e a
//! lib não fixa `csr`/`hydrate`/`ssr` — quem decide é o app (ADR-0007).

mod color;
// Só a coordenação pura por enquanto; o `<Form>` que a consome vem na R-11.
#[allow(dead_code)]
mod form;
mod styles;
mod theme;
mod util;

pub mod components;

pub use color::{Align, Color};
pub use components::*;
pub use styles::{CSS, UiStyles};
pub use theme::{Mode, Theme, ThemeStore, provide_theme, use_theme};
