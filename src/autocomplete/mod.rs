//! `Autocomplete`: campo de **texto livre** com uma lista de **sugestões** (ADR-0012,
//! `docs/decisions/0012-select-autocomplete-and-options.md`).
//!
//! Especificação: `docs/ui/forms.md#autocomplete`. O filtro e as decisões de teclado ficam em
//! [`logic`] (sem DOM e sem Leptos, testados no host; o filtro é o [`search`](crate::search));
//! [`control`] liga isso ao DOM, ao popup e ao `Field`; [`component`] é o componente público.

mod component;
mod control;
mod logic;

pub use component::Autocomplete;
