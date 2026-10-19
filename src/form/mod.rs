//! `Form` — coordena validação, submit e reset dos campos registrados (≈ `QForm` do Quasar).
//!
//! Especificação: `docs/ui/forms.md`. A coordenação fica em [`registry`], sem DOM e sem
//! Leptos, testada no host.

pub(crate) mod registry;
