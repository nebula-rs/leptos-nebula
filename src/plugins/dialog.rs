//! The built-in Dialog plugin (`Plugins::dialog` / `dialog_with`), written only against the
//! public extension API of [`crate::plugins`] and public components — the same road an app's own
//! plugin takes.
//!
//! The pure parts (button resolution, when OK is disabled) are plain functions tested on the
//! host; the view is built once per entry by the plugin host.

use std::{cell::Cell, cell::RefCell, rc::Rc};

use leptos::{ev, prelude::*};

use super::{PluginCtx, PluginHandle, Plugins, use_plugin_ctx};
use crate::{
    Align, Btn, BtnDesign, Card, CardActions, CardSection, Color, Dialog, DialogPosition, Input,
    InputType, Spinner,
};

// ── Options ────────────────────────────────────────────────────────────────

/// How a dialog button looks and what it says.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum DialogBtn {
    /// No button.
    Hidden,
    /// The default button: the label and design from [`DialogSetup`].
    #[default]
    Default,
    /// The default button with another label.
    Label(String),
    /// A button with its own label, design and/or color.
    Props(DialogBtnProps),
}

impl From<bool> for DialogBtn {
    /// `true` is the default button, `false` hides it.
    fn from(shown: bool) -> Self {
        if shown {
            DialogBtn::Default
        } else {
            DialogBtn::Hidden
        }
    }
}

impl From<&str> for DialogBtn {
    fn from(label: &str) -> Self {
        DialogBtn::Label(label.to_string())
    }
}

impl From<String> for DialogBtn {
    fn from(label: String) -> Self {
        DialogBtn::Label(label)
    }
}

impl From<DialogBtnProps> for DialogBtn {
    fn from(props: DialogBtnProps) -> Self {
        DialogBtn::Props(props)
    }
}

/// A dialog button with its own look. Unset fields fall back to [`DialogSetup`] and the dialog's
/// `color`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DialogBtnProps {
    /// Text; the default label of the button when unset.
    pub label: Option<String>,
    /// Look; `DialogSetup::design` when unset.
    pub design: Option<BtnDesign>,
    /// Theme color; the dialog's `color`, then `DialogSetup::color`, when unset.
    pub color: Option<Color>,
}

/// Which button gets the initial focus (when the dialog has no form field that takes it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DialogFocus {
    /// The OK button.
    #[default]
    Ok,
    /// The cancel button.
    Cancel,
    /// Nothing in particular: the browser picks the first focusable element.
    None,
}

/// Options of [`Plugins::dialog`]. Plain data with chainable setters; `title`, `message`,
/// `progress`, `ok` and `cancel` can change while the dialog is open (`PluginHandle::update`),
/// the rest is read when it opens.
#[derive(Clone, Debug, PartialEq)]
pub struct DialogOptions {
    /// Heading; empty = none.
    pub title: String,
    /// Plain text under the heading (never HTML).
    pub message: String,
    /// The OK button; shown by default.
    pub ok: DialogBtn,
    /// The cancel button; hidden by default.
    pub cancel: DialogBtn,
    /// Which button starts focused.
    pub focus: DialogFocus,
    /// Stacks the buttons vertically.
    pub stack_buttons: bool,
    /// Theme color of the buttons and the spinner; `DialogSetup::color` when unset.
    pub color: Option<Color>,
    /// Shows a spinner.
    pub progress: bool,
    /// Neither `Esc` nor a click outside closes it (the panel shakes), and
    /// `Plugins::dismiss_all` skips it.
    pub persistent: bool,
    /// `Esc` does not close it.
    pub no_esc_dismiss: bool,
    /// A click outside does not close it.
    pub no_backdrop_dismiss: bool,
    /// Side the panel sticks to.
    pub position: DialogPosition,
    /// Fills the window.
    pub maximized: bool,
    /// Tries to take the window's width.
    pub full_width: bool,
    /// Tries to take the window's height.
    pub full_height: bool,
    /// Accessible name when there is no title.
    pub aria_label: Option<String>,
    /// Extra classes on the panel.
    pub class: String,
}

impl Default for DialogOptions {
    fn default() -> Self {
        DialogOptions {
            title: String::new(),
            message: String::new(),
            ok: DialogBtn::Default,
            cancel: DialogBtn::Hidden,
            focus: DialogFocus::Ok,
            stack_buttons: false,
            color: None,
            progress: false,
            persistent: false,
            no_esc_dismiss: false,
            no_backdrop_dismiss: false,
            position: DialogPosition::Standard,
            maximized: false,
            full_width: false,
            full_height: false,
            aria_label: None,
            class: String::new(),
        }
    }
}

impl DialogOptions {
    /// Sets [`title`](Self::title).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets [`message`](Self::message).
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Sets [`ok`](Self::ok): `true`/`false`, a label, or a [`DialogBtnProps`].
    pub fn ok(mut self, ok: impl Into<DialogBtn>) -> Self {
        self.ok = ok.into();
        self
    }

    /// Sets [`cancel`](Self::cancel): `true`/`false`, a label, or a [`DialogBtnProps`].
    pub fn cancel(mut self, cancel: impl Into<DialogBtn>) -> Self {
        self.cancel = cancel.into();
        self
    }

    /// Sets [`focus`](Self::focus).
    pub fn focus(mut self, focus: DialogFocus) -> Self {
        self.focus = focus;
        self
    }

    /// Sets [`stack_buttons`](Self::stack_buttons).
    pub fn stack_buttons(mut self, stack: bool) -> Self {
        self.stack_buttons = stack;
        self
    }

    /// Sets [`color`](Self::color).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets [`progress`](Self::progress).
    pub fn progress(mut self, progress: bool) -> Self {
        self.progress = progress;
        self
    }

    /// Sets [`persistent`](Self::persistent).
    pub fn persistent(mut self, persistent: bool) -> Self {
        self.persistent = persistent;
        self
    }

    /// Sets [`no_esc_dismiss`](Self::no_esc_dismiss).
    pub fn no_esc_dismiss(mut self, yes: bool) -> Self {
        self.no_esc_dismiss = yes;
        self
    }

    /// Sets [`no_backdrop_dismiss`](Self::no_backdrop_dismiss).
    pub fn no_backdrop_dismiss(mut self, yes: bool) -> Self {
        self.no_backdrop_dismiss = yes;
        self
    }

    /// Sets [`position`](Self::position).
    pub fn position(mut self, position: DialogPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets [`maximized`](Self::maximized).
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Sets [`full_width`](Self::full_width).
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Sets [`full_height`](Self::full_height).
    pub fn full_height(mut self, full_height: bool) -> Self {
        self.full_height = full_height;
        self
    }

    /// Sets [`aria_label`](Self::aria_label).
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets [`class`](Self::class).
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

// ── Global setup ───────────────────────────────────────────────────────────

/// Page-global defaults of the Dialog plugin, set once at boot with [`DialogPlugin::setup`].
#[derive(Clone, Debug, PartialEq)]
pub struct DialogSetup {
    /// Closes the non-persistent plugin dialogs when the path changes. Needs the `router`
    /// feature of `leptos-nebula` and a `<PluginHost/>` inside the `Router`; without the
    /// feature call [`Plugins::dismiss_all`] on navigation yourself. Default `false`.
    pub close_on_navigation: bool,
    /// Label of the default OK button. Default `"OK"`.
    pub ok_label: String,
    /// Label of the default cancel button. Default `"Cancelar"`.
    pub cancel_label: String,
    /// Look of the default buttons. Default [`BtnDesign::Flat`].
    pub design: BtnDesign,
    /// Color of the default buttons and the spinner. Default [`Color::Primary`].
    pub color: Color,
}

impl Default for DialogSetup {
    fn default() -> Self {
        DialogSetup {
            close_on_navigation: false,
            ok_label: "OK".to_string(),
            cancel_label: "Cancelar".to_string(),
            design: BtnDesign::Flat,
            color: Color::Primary,
        }
    }
}

thread_local! {
    static SETUP: RefCell<DialogSetup> = RefCell::new(DialogSetup::default());
}

/// Namespace of the Dialog plugin's page-global configuration.
pub struct DialogPlugin;

impl DialogPlugin {
    /// Sets the page-global defaults of every plugin dialog. Call it once at boot; it is read
    /// when an entry opens (and, for `close_on_navigation`, on navigation), and calling it again
    /// replaces it. Being page-global, several app instances of the same wasm share it.
    pub fn setup(setup: DialogSetup) {
        SETUP.with(|s| *s.borrow_mut() = setup);
    }

    /// The current setup.
    pub(crate) fn current() -> DialogSetup {
        SETUP.with(|s| s.borrow().clone())
    }

    /// Whether navigation should dismiss the dialogs (read by the plugin host).
    #[cfg(any(feature = "router", test))]
    pub(crate) fn close_on_navigation() -> bool {
        SETUP.with(|s| s.borrow().close_on_navigation)
    }
}

// ── Pure logic ─────────────────────────────────────────────────────────────

/// A button after applying the defaults.
#[derive(Clone, Debug, PartialEq)]
struct ResolvedBtn {
    label: String,
    design: BtnDesign,
    color: Color,
}

/// Turns a [`DialogBtn`] into what to render, or `None` for no button.
fn resolve_btn(
    btn: &DialogBtn,
    default_label: &str,
    dialog_color: Option<Color>,
    setup: &DialogSetup,
) -> Option<ResolvedBtn> {
    let (label, design, color) = match btn {
        DialogBtn::Hidden => return None,
        DialogBtn::Default => (None, None, None),
        DialogBtn::Label(l) => (Some(l.clone()), None, None),
        DialogBtn::Props(p) => (p.label.clone(), p.design, p.color),
    };
    Some(ResolvedBtn {
        label: label.unwrap_or_else(|| default_label.to_string()),
        design: design.unwrap_or(setup.design),
        color: color.or(dialog_color).unwrap_or(setup.color),
    })
}

/// OK is disabled while the form says its value is not valid.
fn ok_disabled(has_form: bool, valid: bool) -> bool {
    has_form && !valid
}

// ── Forms ──────────────────────────────────────────────────────────────────

/// What a [`DialogForm`] gives the dialog.
pub struct DialogFormParts<V: 'static> {
    /// The form's view, shown between the message and the buttons; `None` = no form.
    pub view: Option<AnyView>,
    /// The current value; OK settles the dialog with it.
    pub value: Signal<V, LocalStorage>,
    /// Whether the value is valid; OK is disabled while it is not.
    pub valid: Signal<bool, LocalStorage>,
    /// The form focuses one of its fields itself, so the dialog does not focus a button.
    pub autofocus: bool,
}

/// The body of a dialog that produces a value (`Plugins::dialog_with`). `build` runs once, when
/// the plugin host renders the dialog, so signals it creates belong to that entry. `()` is the
/// "no form" form.
pub trait DialogForm: 'static {
    /// What OK settles the dialog with.
    type Value: Clone + 'static;

    /// Creates the view and the signals.
    fn build(self) -> DialogFormParts<Self::Value>;
}

impl DialogForm for () {
    type Value = ();

    fn build(self) -> DialogFormParts<()> {
        DialogFormParts {
            view: None,
            value: Signal::stored_local(()),
            valid: Signal::stored_local(true),
            autofocus: false,
        }
    }
}

type Validity = Rc<dyn Fn(&str) -> bool>;

/// A single text field. OK settles with the text; `Enter` submits (except in a `Textarea`) and
/// [`is_valid`](Self::is_valid) keeps OK disabled while it returns `false`.
pub struct DialogPrompt {
    model: String,
    type_: InputType,
    label: Option<String>,
    placeholder: String,
    is_valid: Option<Validity>,
}

impl DialogPrompt {
    /// A prompt that starts with `model` as its text.
    pub fn new(model: impl Into<String>) -> Self {
        DialogPrompt {
            model: model.into(),
            type_: InputType::Text,
            label: None,
            placeholder: String::new(),
            is_valid: None,
        }
    }

    /// The kind of field.
    pub fn type_(mut self, type_: InputType) -> Self {
        self.type_ = type_;
        self
    }

    /// The field's label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The field's placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// OK stays disabled while this returns `false` for the current text.
    pub fn is_valid(mut self, is_valid: impl Fn(&str) -> bool + 'static) -> Self {
        self.is_valid = Some(Rc::new(is_valid));
        self
    }
}

impl DialogForm for DialogPrompt {
    type Value = String;

    fn build(self) -> DialogFormParts<String> {
        let DialogPrompt {
            model,
            type_,
            label,
            placeholder,
            is_valid,
        } = self;
        let text = RwSignal::new(model);
        let valid = Signal::derive_local(move || match &is_valid {
            Some(check) => text.with(|t| check(t)),
            None => true,
        });
        let ctx = use_plugin_ctx::<String>();
        let on_keydown = Callback::new(move |e: ev::KeyboardEvent| {
            if e.key() == "Enter"
                && !e.is_composing()
                && type_ != InputType::Textarea
                && valid.get_untracked()
                && let Some(ctx) = ctx
            {
                e.prevent_default();
                ctx.ok(text.get_untracked());
            }
        });
        let view = match label {
            Some(label) => view! {
                <Input
                    value=text
                    type_=type_
                    label=label
                    placeholder=placeholder
                    dense=true
                    autofocus=true
                    on_keydown=on_keydown
                />
            }
            .into_any(),
            None => view! {
                <Input
                    value=text
                    type_=type_
                    placeholder=placeholder
                    dense=true
                    autofocus=true
                    on_keydown=on_keydown
                />
            }
            .into_any(),
        };
        DialogFormParts {
            view: Some(view),
            value: Signal::derive_local(move || text.get()),
            valid,
            autofocus: true,
        }
    }
}

// ── The plugin ─────────────────────────────────────────────────────────────

/// Handle of an opened dialog. Its props are the [`DialogOptions`]: change them while it is open
/// with `update`.
pub type DialogHandle<T = ()> = PluginHandle<T, DialogOptions>;

impl Plugins {
    /// Opens a dialog (alert, confirm, progress…). `.await` the handle, or use its `on_ok` /
    /// `on_cancel`; the value is `()`.
    pub fn dialog(&self, options: DialogOptions) -> DialogHandle<()> {
        self.dialog_with(options, ())
    }

    /// Opens a dialog with a body that produces a value, such as [`DialogPrompt`].
    pub fn dialog_with<F: DialogForm>(
        &self,
        options: DialogOptions,
        form: F,
    ) -> DialogHandle<F::Value> {
        let setup = DialogPlugin::current();
        let persistent = options.persistent;
        let handle = self.open_with_props(options, move |ctx, props| {
            dialog_view(ctx, props, form, setup)
        });
        handle.set_persistent(persistent);
        handle
    }
}

thread_local! {
    static NEXT_TITLE_ID: Cell<u32> = const { Cell::new(0) };
}

/// A fresh `id` for a dialog title (only called in the browser, when an entry renders).
fn next_title_id() -> String {
    let n = NEXT_TITLE_ID.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    format!("lx-dialog-plugin-title-{n}")
}

fn dialog_view<F: DialogForm>(
    ctx: PluginCtx<F::Value>,
    props: ReadSignal<DialogOptions, LocalStorage>,
    form: F,
    setup: DialogSetup,
) -> impl IntoView {
    let o = props.get_untracked();
    let parts = form.build();
    let has_form = parts.view.is_some();
    let (value, valid) = (parts.value, parts.valid);
    let form_focuses = parts.autofocus;

    // Reactive pieces, each behind a `Memo` so an `update` that does not touch it leaves its
    // DOM (and the focus in it) alone.
    let title = Memo::new(move |_| props.with(|o| o.title.clone()));
    let message = Memo::new(move |_| props.with(|o| o.message.clone()));
    let progress = Memo::new(move |_| props.with(|o| o.progress));
    let spinner_color = {
        let fallback = setup.color;
        Memo::new(move |_| props.with(|o| o.color).unwrap_or(fallback))
    };
    let ok_btn = {
        let setup = setup.clone();
        Memo::new(move |_| props.with(|o| resolve_btn(&o.ok, &setup.ok_label, o.color, &setup)))
    };
    let cancel_btn = {
        let setup = setup.clone();
        Memo::new(move |_| {
            props.with(|o| resolve_btn(&o.cancel, &setup.cancel_label, o.color, &setup))
        })
    };

    let title_id = (!o.title.is_empty()).then(next_title_id);
    let aria_label = match (&title_id, &o.aria_label) {
        (Some(_), _) => None,
        (None, Some(l)) => Some(l.clone()),
        (None, None) => (!o.message.is_empty()).then(|| o.message.clone()),
    };
    let focus_ok = o.focus == DialogFocus::Ok && !form_focuses;
    let focus_cancel = o.focus == DialogFocus::Cancel && !form_focuses;
    let stack = o.stack_buttons;
    let ok_off = Signal::derive(move || ok_disabled(has_form, valid.get()));
    let ok_click = Callback::new(move |_: ev::MouseEvent| {
        if !ok_disabled(has_form, valid.get_untracked()) {
            ctx.ok(value.get_untracked());
        }
    });
    let cancel_click = Callback::new(move |_: ev::MouseEvent| ctx.cancel());

    let title_view = {
        let title_id = title_id.clone();
        move || {
            let text = title.get();
            let id = title_id.clone();
            (!text.is_empty()).then(move || {
                view! {
                    <CardSection>
                        <div id=id class="lx-dialog-title">
                            {text}
                        </div>
                    </CardSection>
                }
            })
        }
    };
    let progress_view = move || {
        progress.get().then(|| {
            view! {
                <CardSection>
                    <div class="lx-dialog-progress">
                        <Spinner size="2.5rem" color=spinner_color.get() />
                    </div>
                </CardSection>
            }
        })
    };
    let message_view = move || {
        let text = message.get();
        (!text.is_empty()).then(|| {
            view! {
                <CardSection>
                    <div class="lx-dialog-message">{text}</div>
                </CardSection>
            }
        })
    };
    let form_view = parts.view.map(|body| {
        view! {
            <CardSection>
                <div class="lx-dialog-form">{body}</div>
            </CardSection>
        }
    });
    let buttons_view = move || {
        let cancel = cancel_btn.get();
        let ok = ok_btn.get();
        (cancel.is_some() || ok.is_some()).then(|| {
            let cancel = cancel.map(|b| {
                view! {
                    <Btn
                        label=b.label
                        design=b.design
                        color=b.color
                        on_click=cancel_click
                        attr:autofocus=focus_cancel
                    />
                }
            });
            let ok = ok.map(|b| {
                view! {
                    <Btn
                        label=b.label
                        design=b.design
                        color=b.color
                        disable=ok_off
                        on_click=ok_click
                        attr:autofocus=focus_ok
                    />
                }
            });
            view! {
                <CardActions align=Align::Right vertical=stack>
                    {cancel}
                    {ok}
                </CardActions>
            }
        })
    };

    let class = crate::util::cls(["lx-dialog-plugin", &o.class]);
    let panel = move || {
        view! {
            <div class=class>
                <Card flat=true>
                    {title_view}
                    {progress_view}
                    {message_view}
                    {form_view}
                    {buttons_view}
                </Card>
            </div>
        }
    };

    let (show, on_hide) = (ctx.show(), ctx.on_hide());
    let (persistent, no_esc, no_backdrop) = (o.persistent, o.no_esc_dismiss, o.no_backdrop_dismiss);
    let (position, maximized, full_width, full_height) =
        (o.position, o.maximized, o.full_width, o.full_height);
    match (title_id, aria_label) {
        (Some(id), _) => view! {
            <Dialog
                show=show
                on_hide=on_hide
                persistent=persistent
                no_esc_dismiss=no_esc
                no_backdrop_dismiss=no_backdrop
                position=position
                maximized=maximized
                full_width=full_width
                full_height=full_height
                aria_labelledby=id
            >
                {panel()}
            </Dialog>
        }
        .into_any(),
        (None, Some(label)) => view! {
            <Dialog
                show=show
                on_hide=on_hide
                persistent=persistent
                no_esc_dismiss=no_esc
                no_backdrop_dismiss=no_backdrop
                position=position
                maximized=maximized
                full_width=full_width
                full_height=full_height
                aria_label=label
            >
                {panel()}
            </Dialog>
        }
        .into_any(),
        (None, None) => view! {
            <Dialog
                show=show
                on_hide=on_hide
                persistent=persistent
                no_esc_dismiss=no_esc
                no_backdrop_dismiss=no_backdrop
                position=position
                maximized=maximized
                full_width=full_width
                full_height=full_height
            >
                {panel()}
            </Dialog>
        }
        .into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_buttons_take_label_design_and_color_from_the_setup() {
        let setup = DialogSetup::default();
        let ok = resolve_btn(&DialogBtn::Default, &setup.ok_label, None, &setup).unwrap();
        assert_eq!(ok.label, "OK");
        assert_eq!(ok.design, BtnDesign::Flat);
        assert_eq!(ok.color, Color::Primary);
    }

    #[test]
    fn setup_changes_the_defaults() {
        let setup = DialogSetup {
            ok_label: "Sim".into(),
            cancel_label: "Não".into(),
            design: BtnDesign::Outline,
            color: Color::Secondary,
            ..DialogSetup::default()
        };
        let cancel = resolve_btn(&DialogBtn::Default, &setup.cancel_label, None, &setup).unwrap();
        assert_eq!(cancel.label, "Não");
        assert_eq!(cancel.design, BtnDesign::Outline);
        assert_eq!(cancel.color, Color::Secondary);
    }

    #[test]
    fn hidden_has_no_button_and_a_label_keeps_the_rest_default() {
        let setup = DialogSetup::default();
        assert_eq!(resolve_btn(&DialogBtn::Hidden, "OK", None, &setup), None);
        let b = resolve_btn(&DialogBtn::from("Entendi"), "OK", None, &setup).unwrap();
        assert_eq!(b.label, "Entendi");
        assert_eq!(b.design, BtnDesign::Flat);
    }

    #[test]
    fn props_override_the_dialog_color_which_overrides_the_setup() {
        let setup = DialogSetup::default();
        let props = DialogBtn::Props(DialogBtnProps {
            label: None,
            design: Some(BtnDesign::Unelevated),
            color: Some(Color::Negative),
        });
        let b = resolve_btn(&props, "OK", Some(Color::Info), &setup).unwrap();
        assert_eq!(b.label, "OK");
        assert_eq!(b.design, BtnDesign::Unelevated);
        assert_eq!(b.color, Color::Negative);
        let plain = resolve_btn(&DialogBtn::Default, "OK", Some(Color::Info), &setup).unwrap();
        assert_eq!(plain.color, Color::Info);
    }

    #[test]
    fn bool_and_str_convert_to_buttons() {
        assert_eq!(DialogBtn::from(true), DialogBtn::Default);
        assert_eq!(DialogBtn::from(false), DialogBtn::Hidden);
        assert_eq!(DialogBtn::from("x"), DialogBtn::Label("x".into()));
        let o = DialogOptions::default();
        assert_eq!(o.ok, DialogBtn::Default);
        assert_eq!(o.cancel, DialogBtn::Hidden);
    }

    #[test]
    fn ok_is_disabled_only_for_a_form_with_an_invalid_value() {
        assert!(!ok_disabled(false, true));
        assert!(!ok_disabled(false, false));
        assert!(!ok_disabled(true, true));
        assert!(ok_disabled(true, false));
    }

    #[test]
    fn setup_is_global_and_replaceable() {
        assert!(!DialogPlugin::close_on_navigation());
        DialogPlugin::setup(DialogSetup {
            close_on_navigation: true,
            ..DialogSetup::default()
        });
        assert!(DialogPlugin::close_on_navigation());
        assert!(DialogPlugin::current().close_on_navigation);
        DialogPlugin::setup(DialogSetup::default());
        assert!(!DialogPlugin::close_on_navigation());
    }
}
