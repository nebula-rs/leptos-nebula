//! `Field` — o "quadro" do campo de formulário (≈ `QField` do Quasar): label, hint, prefix/suffix,
//! slots e designs. O controle (um `<input>` nativo, depois o `Input`) vai como `children`.
//!
//! Especificação: `docs/ui/forms.md#field`. A escolha de classes e o estado do label ficam em
//! [`state`], sem DOM e sem Leptos, testados no host; [`component`] liga isso ao DOM.

mod component;
mod state;

pub use component::{
    Field, FieldAfter, FieldAppend, FieldBefore, FieldContext, FieldHint, FieldPrepend,
    use_field_context,
};
pub use state::FieldDesign;
