//! `Select` e `SelectMultiple` (≈ `QSelect` do Quasar): um `combobox` com lista de opções.
//!
//! Especificação: `docs/ui/forms.md#select`. A navegação por teclado, o type-ahead e a
//! alternância da seleção ficam em [`nav`], sem DOM e sem Leptos, testados no host; [`control`]
//! liga isso ao DOM e ao popup; [`component`] são os dois componentes públicos.

mod component;
mod control;
mod nav;

pub use component::{Select, SelectMultiple};
