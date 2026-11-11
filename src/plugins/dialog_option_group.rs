//! Option forms for the Dialog plugin: a radio group ([`DialogOptionGroup`]) and a checkbox or
//! switch group ([`DialogOptionGroupMultiple`]). They implement [`DialogForm`] over the public
//! [`OptionGroup`] components.

use std::rc::Rc;

use leptos::prelude::*;

use super::{DialogForm, DialogFormParts};
use crate::{Color, OptionGroup, OptionGroupMultiple, OptionGroupType, Options};

type Check<T> = Rc<dyn Fn(&T) -> bool>;

/// A radio group. OK settles with the chosen value (`Option<V>`, `None` = nothing chosen);
/// [`is_valid`](Self::is_valid) keeps OK disabled while it returns `false`.
pub struct DialogOptionGroup<V: Clone + Send + Sync + 'static> {
    model: Option<V>,
    options: Options<V>,
    color: Color,
    inline: bool,
    dense: bool,
    left_label: bool,
    aria_label: Option<String>,
    is_valid: Option<Check<Option<V>>>,
}

impl<V: Clone + Send + Sync + 'static> DialogOptionGroup<V> {
    /// A group that starts with `model` chosen (`None` = nothing).
    pub fn new(model: Option<V>, options: impl Into<Options<V>>) -> Self {
        DialogOptionGroup {
            model,
            options: options.into(),
            color: Color::Primary,
            inline: false,
            dense: false,
            left_label: false,
            aria_label: None,
            is_valid: None,
        }
    }

    /// Color of the controls.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Lay the options out in a row.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Compact controls and spacing.
    pub fn dense(mut self, dense: bool) -> Self {
        self.dense = dense;
        self
    }

    /// Label before the control.
    pub fn left_label(mut self, left_label: bool) -> Self {
        self.left_label = left_label;
        self
    }

    /// Accessible name of the group.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// OK stays disabled while this returns `false` for the current choice.
    pub fn is_valid(mut self, is_valid: impl Fn(&Option<V>) -> bool + 'static) -> Self {
        self.is_valid = Some(Rc::new(is_valid));
        self
    }
}

impl<V: Clone + Send + Sync + 'static> DialogForm for DialogOptionGroup<V> {
    type Value = Option<V>;

    fn build(self) -> DialogFormParts<Option<V>> {
        let DialogOptionGroup {
            model,
            options,
            color,
            inline,
            dense,
            left_label,
            aria_label,
            is_valid,
        } = self;
        let value = RwSignal::new(model);
        let valid = Signal::derive_local(move || match &is_valid {
            Some(check) => value.with(|v| check(v)),
            None => true,
        });
        let view = view! {
            <OptionGroup
                value=value
                options=options
                color=color
                inline=inline
                dense=dense
                left_label=left_label
                aria_label=aria_label
            />
        }
        .into_any();
        DialogFormParts {
            view: Some(view),
            value: Signal::derive_local(move || value.get()),
            valid,
            // The dialog focuses its OK button; Tab reaches the group.
            autofocus: false,
        }
    }
}

/// A group of checkboxes or switches. OK settles with the chosen values (`Vec<V>`, in the order
/// they were chosen); [`is_valid`](Self::is_valid) keeps OK disabled while it returns `false`.
pub struct DialogOptionGroupMultiple<V: Clone + Send + Sync + 'static> {
    model: Vec<V>,
    options: Options<V>,
    type_: OptionGroupType,
    color: Color,
    inline: bool,
    dense: bool,
    left_label: bool,
    aria_label: Option<String>,
    is_valid: Option<Check<Vec<V>>>,
}

impl<V: Clone + Send + Sync + 'static> DialogOptionGroupMultiple<V> {
    /// A group that starts with `model` chosen.
    pub fn new(model: Vec<V>, options: impl Into<Options<V>>) -> Self {
        DialogOptionGroupMultiple {
            model,
            options: options.into(),
            type_: OptionGroupType::Checkbox,
            color: Color::Primary,
            inline: false,
            dense: false,
            left_label: false,
            aria_label: None,
            is_valid: None,
        }
    }

    /// Checkbox or switch.
    pub fn type_(mut self, type_: OptionGroupType) -> Self {
        self.type_ = type_;
        self
    }

    /// Color of the controls.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Lay the options out in a row.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Compact controls and spacing.
    pub fn dense(mut self, dense: bool) -> Self {
        self.dense = dense;
        self
    }

    /// Label before the control.
    pub fn left_label(mut self, left_label: bool) -> Self {
        self.left_label = left_label;
        self
    }

    /// Accessible name of the group.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// OK stays disabled while this returns `false` for the current choices.
    pub fn is_valid(mut self, is_valid: impl Fn(&Vec<V>) -> bool + 'static) -> Self {
        self.is_valid = Some(Rc::new(is_valid));
        self
    }
}

impl<V: Clone + Send + Sync + 'static> DialogForm for DialogOptionGroupMultiple<V> {
    type Value = Vec<V>;

    fn build(self) -> DialogFormParts<Vec<V>> {
        let DialogOptionGroupMultiple {
            model,
            options,
            type_,
            color,
            inline,
            dense,
            left_label,
            aria_label,
            is_valid,
        } = self;
        let value = RwSignal::new(model);
        let valid = Signal::derive_local(move || match &is_valid {
            Some(check) => value.with(|v| check(v)),
            None => true,
        });
        let view = view! {
            <OptionGroupMultiple
                value=value
                options=options
                type_=type_
                color=color
                inline=inline
                dense=dense
                left_label=left_label
                aria_label=aria_label
            />
        }
        .into_any();
        DialogFormParts {
            view: Some(view),
            value: Signal::derive_local(move || value.get()),
            valid,
            autofocus: false,
        }
    }
}
