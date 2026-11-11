//! Biblioteca de componentes Leptos com temas por tokens — o que o Quasar é para o Vue.
//!
//! Componentes **parciais** (API com os nomes do Quasar, só o que o `leptos-press` precisa);
//! ver `docs/ui/overview.md`. O CSS dos componentes vem pronto ([`CSS`] / [`UiStyles`]);
//! um app que quiser os mesmos shortcuts no próprio markup usa o preset `leptos-nebula/preset`.
//!
//! Seguro para SSR: nenhum componente toca `window`/`document` durante a renderização, e a
//! lib não fixa `csr`/`hydrate`/`ssr` — quem decide é o app (ADR-0007).

mod autocomplete;
mod color;
mod field;
mod form;
mod input;
mod list_popup;
mod option_group;
mod options;
mod root;
mod select;
mod styles;
mod table;
mod theme;
mod util;

pub mod components;
pub mod plugins;
pub mod position;
pub mod search;

pub use autocomplete::Autocomplete;
pub use color::{Align, Color};
pub use components::*;
pub use field::{
    Field, FieldAfter, FieldAppend, FieldBefore, FieldContext, FieldCounter, FieldDesign,
    FieldError, FieldHint, FieldLoading, FieldPrepend, FieldValidation, LazyRules, Rule,
    RuleResult, ValidationOptions, use_field_context, use_validate,
};
pub use form::{
    FocusFn, Form, FormChildId, FormChildOptions, FormHandle, ResetValidationFn, ValidateFn,
    use_form_child,
};
pub use input::{FillMask, Input, InputHandle, InputType};
pub use option_group::{OptionGroup, OptionGroupMultiple, OptionGroupType};
pub use options::{HasKey, HasLabel, NoKey, NoLabel, ObjectOptions, Opt, Options};
pub use plugins::{
    DialogBtn, DialogBtnProps, DialogFocus, DialogForm, DialogFormParts, DialogHandle,
    DialogOptionGroup, DialogOptionGroupMultiple, DialogOptions, DialogPlugin, DialogPrompt,
    DialogSetup, Dismissed, NotifyAction, NotifyGroup, NotifyHandle, NotifyOptions, NotifyPlugin,
    NotifyPosition, NotifySetup, NotifyType, PluginCtx, PluginFuture, PluginHandle, PluginHost,
    Plugins, provide_plugins, try_use_plugins, use_plugin_ctx, use_plugins,
};
pub use position::{Horizontal, ParsePositionError, Position, Vertical};
pub use root::{NebulaRoot, provide_root, use_root};
pub use select::{Select, SelectMultiple};
pub use styles::{CSS, UiStyles};
pub use table::{Table, TableColumn, TablePagination, TablePaginationLabels};
pub use theme::{Mode, Theme, ThemeStore, provide_theme, use_theme};
