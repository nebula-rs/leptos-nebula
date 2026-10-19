//! `Form` — coordena validação, submit e reset dos campos registrados (≈ `QForm` do Quasar).
//!
//! Especificação: `docs/ui/forms.md`. A coordenação fica em [`registry`], sem DOM e sem
//! Leptos, testada no host; [`component`] liga isso ao `<form>`.

mod child;
mod component;
mod focus;
pub(crate) mod registry;

pub use child::{FormChildOptions, use_form_child};
pub use component::{Form, FormHandle};
pub use registry::{FocusFn, FormChildId, ResetValidationFn, ValidateFn};
