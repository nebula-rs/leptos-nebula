//! `Field` — o "quadro" do campo de formulário (≈ `QField` do Quasar): label, hint, prefix/suffix,
//! slots e designs. O controle (um `<input>` nativo, depois o `Input`) vai como `children`.
//!
//! Especificação: `docs/ui/forms.md#field`. A escolha de classes e o estado do label ficam em
//! [`state`], sem DOM e sem Leptos, testados no host; [`component`] liga isso ao DOM.

mod component;
mod state;
mod validate;
mod validation;

pub(crate) use component::FieldProps;
pub use component::{
    Field, FieldAfter, FieldAppend, FieldBefore, FieldContext, FieldError, FieldHint, FieldPrepend,
    use_field_context,
};
pub use state::FieldDesign;
pub use validate::{LazyRules, Rule, RuleResult};
pub use validation::{FieldValidation, ValidationOptions, use_validate};
