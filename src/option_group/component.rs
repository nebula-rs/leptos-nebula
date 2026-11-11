//! The components behind [`OptionGroup`] and [`OptionGroupMultiple`]: one render path, three
//! kinds of native `<input>` (radio, checkbox, checkbox with `role="switch"`).

use leptos::prelude::*;

use super::{is_selected, toggle};
use crate::options::Item;
use crate::util::{cls, next_id};
use crate::{Color, Options};

/// What a multiple group renders for each option.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OptionGroupType {
    /// A checkbox per option.
    #[default]
    Checkbox,
    /// A switch per option (`<input type="checkbox" role="switch">`).
    Toggle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Radio,
    Checkbox,
    Toggle,
}

/// Shared look of a group.
#[derive(Clone, Copy)]
struct Look {
    color: Color,
    inline: bool,
    dense: bool,
    left_label: bool,
}

/// The group element with its options. `is_checked` says whether an option is selected and
/// `on_change` is called when the user flips one.
fn group_view<V>(
    kind: Kind,
    look: Look,
    options: Options<V>,
    disable: Signal<bool>,
    aria_label: MaybeProp<String>,
    is_checked: impl Fn(&V) -> bool + Clone + Send + Sync + 'static,
    on_change: impl Fn(&V) + Clone + Send + Sync + 'static,
) -> impl IntoView
where
    V: Clone + Send + Sync + 'static,
{
    let Look {
        color,
        inline,
        dense,
        left_label,
    } = look;
    let name = next_id("lx-option-group");
    let layout = match (inline, dense) {
        (false, false) => "lx-option-group-col",
        (false, true) => "lx-option-group-col-dense",
        (true, false) => "lx-option-group-inline",
        (true, true) => "lx-option-group-inline-dense",
    };
    let role = match kind {
        Kind::Radio => "radiogroup",
        Kind::Checkbox | Kind::Toggle => "group",
    };
    let items = options.items();

    let rows = move || {
        let name = name.clone();
        let is_checked = is_checked.clone();
        let on_change = on_change.clone();
        items
            .get()
            .into_iter()
            .map(|item: Item<V>| {
                let Item {
                    label,
                    value,
                    disable: item_disable,
                } = item;
                let checked = {
                    let value = value.clone();
                    let is_checked = is_checked.clone();
                    move || is_checked(&value)
                };
                let is_disabled = move || disable.get() || item_disable;
                let item_class = move || match (left_label, is_disabled()) {
                    (false, false) => "lx-option-item",
                    (true, false) => "lx-option-item-left",
                    (false, true) => "lx-option-item-disabled",
                    (true, true) => "lx-option-item-left-disabled",
                };
                let onchange = {
                    let on_change = on_change.clone();
                    move |_| on_change(&value)
                };
                let input = match kind {
                    Kind::Radio => view! {
                        <input
                            type="radio"
                            name=name.clone()
                            class=if dense { "lx-radio-dense" } else { "lx-radio" }
                            prop:checked=checked
                            disabled=is_disabled
                            on:change=onchange
                        />
                    }
                    .into_any(),
                    Kind::Checkbox => view! {
                        <input
                            type="checkbox"
                            class=if dense { "lx-checkbox-dense" } else { "lx-checkbox" }
                            prop:checked=checked
                            disabled=is_disabled
                            on:change=onchange
                        />
                    }
                    .into_any(),
                    Kind::Toggle => {
                        let class = {
                            let checked = checked.clone();
                            move || match (dense, checked()) {
                                (false, false) => "lx-toggle-off",
                                (false, true) => "lx-toggle-on",
                                (true, false) => "lx-toggle-off-dense",
                                (true, true) => "lx-toggle-on-dense",
                            }
                        };
                        let aria_checked = {
                            let checked = checked.clone();
                            move || if checked() { "true" } else { "false" }
                        };
                        view! {
                            <input
                                type="checkbox"
                                role="switch"
                                class=class
                                aria-checked=aria_checked
                                prop:checked=checked
                                disabled=is_disabled
                                on:change=onchange
                            />
                        }
                        .into_any()
                    }
                };
                view! {
                    <label class=item_class>
                        {input}
                        <span class="lx-option-label">{label}</span>
                    </label>
                }
            })
            .collect_view()
    };

    view! {
        <div
            class=cls([layout, color.tone_class()])
            role=role
            aria-label=move || aria_label.get()
        >
            {rows}
        </div>
    }
}

/// A group of radios: exactly one option chosen at a time (or none).
///
/// `value` is the model: `Some(value)` of the chosen option or `None`. `options` comes in the
/// three formats of [`Options`] (`Vec<P>`, `Vec<Opt<K>>`, `ObjectOptions`); the value is
/// compared by the options' identity, so it needs no `PartialEq`.
///
/// Each option is a native `<input type="radio">` inside a `<label>`, all sharing one `name`: the
/// browser gives the keyboard behaviour (`Tab` enters the group, the arrows move and choose,
/// disabled options are skipped). The group is a `role="radiogroup"`; name it with `aria_label`.
///
/// Not yet: per-option `color`/`keep_color`, `size`, `name` (native submit value), option
/// slots, `attrs` per option, a `readonly` mode.
#[component]
pub fn OptionGroup<V>(
    /// The chosen option's value (`v-model`).
    value: RwSignal<Option<V>>,
    /// The options.
    #[prop(into)]
    options: Options<V>,
    /// Color of the controls.
    #[prop(optional)]
    color: Color,
    /// Lay the options out in a row (wrapping) instead of a column.
    #[prop(optional)]
    inline: bool,
    /// Compact controls and spacing.
    #[prop(optional)]
    dense: bool,
    /// Label before the control.
    #[prop(optional)]
    left_label: bool,
    /// Disables every option.
    #[prop(optional, into)]
    disable: Signal<bool>,
    /// Accessible name of the group.
    #[prop(optional, into)]
    aria_label: MaybeProp<String>,
) -> impl IntoView
where
    V: Clone + Send + Sync + 'static,
{
    let look = Look {
        color,
        inline,
        dense,
        left_label,
    };
    let same = options.clone();
    let same_set = options.clone();
    group_view(
        Kind::Radio,
        look,
        options,
        disable,
        aria_label,
        move |v: &V| {
            value.with(|cur| match cur {
                Some(cur) => same.same(cur, v),
                None => false,
            })
        },
        move |v: &V| {
            // Only the radio that became checked fires `change`.
            if !value.with_untracked(|cur| cur.as_ref().is_some_and(|c| same_set.same(c, v))) {
                value.set(Some(v.clone()));
            }
        },
    )
}

/// A group of checkboxes or switches: any number of options chosen.
///
/// `value` is the list of chosen values, in the order they were chosen. `type_` picks the
/// control: [`OptionGroupType::Checkbox`] (default) or [`OptionGroupType::Toggle`] (each is a
/// native checkbox with `role="switch"` and `aria-checked`). The group is a `role="group"`;
/// name it with `aria_label`. Every other prop is as in [`OptionGroup`].
#[component]
pub fn OptionGroupMultiple<V>(
    /// The chosen values (`v-model`).
    value: RwSignal<Vec<V>>,
    /// The options.
    #[prop(into)]
    options: Options<V>,
    /// Checkbox or switch.
    #[prop(optional)]
    type_: OptionGroupType,
    /// Color of the controls.
    #[prop(optional)]
    color: Color,
    /// Lay the options out in a row (wrapping) instead of a column.
    #[prop(optional)]
    inline: bool,
    /// Compact controls and spacing.
    #[prop(optional)]
    dense: bool,
    /// Label before the control.
    #[prop(optional)]
    left_label: bool,
    /// Disables every option.
    #[prop(optional, into)]
    disable: Signal<bool>,
    /// Accessible name of the group.
    #[prop(optional, into)]
    aria_label: MaybeProp<String>,
) -> impl IntoView
where
    V: Clone + Send + Sync + 'static,
{
    let kind = match type_ {
        OptionGroupType::Checkbox => Kind::Checkbox,
        OptionGroupType::Toggle => Kind::Toggle,
    };
    let look = Look {
        color,
        inline,
        dense,
        left_label,
    };
    let same = options.clone();
    let same_toggle = options.clone();
    group_view(
        kind,
        look,
        options,
        disable,
        aria_label,
        move |v: &V| value.with(|list| is_selected(list, v, |a, b| same.same(a, b))),
        move |v: &V| {
            value.update(|list| {
                toggle(list, v, |a, b| same_toggle.same(a, b));
            });
        },
    )
}
