//! The built-in Notify plugin (`Plugins::notify`): toasts, written only against the public
//! extension API of [`crate::plugins`] and public components.
//!
//! How it fits the host: the first `notify` opens one host entry (never settled) that renders a
//! `popover="manual"` layer with a stack per screen position. Each notification is a *ghost*
//! entry: its view renders nothing, it only registers a toast with the layer, which draws it.
//! That gives every toast a real [`PluginHandle`] (`update`, `hide`, `.await`) without one
//! popover per toast.
//!
//! The pure parts (option merging, group keys, timer and badge rules) are plain functions tested
//! on the host; everything that touches the DOM or timers runs in `Effect`s, so nothing happens on
//! the server.

use std::{cell::Cell, cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use leptos::{
    leptos_dom::helpers::{TimeoutHandle, set_timeout, set_timeout_with_handle},
    prelude::*,
};

use super::{PluginCtx, PluginHandle, Plugins};
use crate::{Color, Icon, Spinner, util::cls};

// ── Options ────────────────────────────────────────────────────────────────

/// Where on the screen a notification appears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NotifyPosition {
    /// Top left corner.
    TopLeft,
    /// Top edge, centered.
    Top,
    /// Top right corner.
    TopRight,
    /// Left edge, vertically centered.
    Left,
    /// The middle of the screen.
    Center,
    /// Right edge, vertically centered.
    Right,
    /// Bottom left corner.
    BottomLeft,
    /// Bottom edge, centered.
    #[default]
    Bottom,
    /// Bottom right corner.
    BottomRight,
}

impl NotifyPosition {
    /// Every position, in the order the layer renders its stacks.
    const ALL: [NotifyPosition; 9] = [
        NotifyPosition::TopLeft,
        NotifyPosition::Top,
        NotifyPosition::TopRight,
        NotifyPosition::Left,
        NotifyPosition::Center,
        NotifyPosition::Right,
        NotifyPosition::BottomLeft,
        NotifyPosition::Bottom,
        NotifyPosition::BottomRight,
    ];

    /// Whether toasts slide in from above (the top row) instead of from below.
    fn enters_from_above(self) -> bool {
        matches!(
            self,
            NotifyPosition::TopLeft | NotifyPosition::Top | NotifyPosition::TopRight
        )
    }

    /// Whether the stack hugs the left edge (the badge then goes on the right side).
    fn is_left(self) -> bool {
        matches!(
            self,
            NotifyPosition::TopLeft | NotifyPosition::Left | NotifyPosition::BottomLeft
        )
    }

    fn stack_class(self) -> &'static str {
        match self {
            NotifyPosition::TopLeft => "lx-notify-stack lx-notify-stack-top-left",
            NotifyPosition::Top => "lx-notify-stack lx-notify-stack-top",
            NotifyPosition::TopRight => "lx-notify-stack lx-notify-stack-top-right",
            NotifyPosition::Left => "lx-notify-stack lx-notify-stack-left",
            NotifyPosition::Center => "lx-notify-stack lx-notify-stack-center",
            NotifyPosition::Right => "lx-notify-stack lx-notify-stack-right",
            NotifyPosition::BottomLeft => "lx-notify-stack lx-notify-stack-bottom-left",
            NotifyPosition::Bottom => "lx-notify-stack lx-notify-stack-bottom",
            NotifyPosition::BottomRight => "lx-notify-stack lx-notify-stack-bottom-right",
        }
    }
}

/// A preset of look and behavior: color, icon, spinner and timeout come from it unless the
/// options say otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotifyType {
    /// Success: positive color and a check icon.
    Positive,
    /// Error: negative color and an alert icon. Announced as an alert (`role="alert"`).
    Negative,
    /// Warning: warning color and a triangle icon.
    Warning,
    /// Information: info color and an info icon.
    Info,
    /// Work in progress: a spinner and no timeout (dismiss it with `hide`).
    Ongoing,
    /// A preset added with [`NotifyPlugin::register_type`]. An unknown name adds nothing.
    Custom(&'static str),
}

/// How repeats of the same notification are folded into one toast.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum NotifyGroup {
    /// Same message, caption and position are the same toast.
    #[default]
    Auto,
    /// Never grouped.
    Off,
    /// Same key and position are the same toast, whatever the text.
    Key(String),
}

/// A button on a notification.
#[derive(Clone, Debug)]
pub struct NotifyAction {
    /// Button text.
    pub label: String,
    /// Icon class (a literal `i-lucide-…`), before the label.
    pub icon: Option<&'static str>,
    /// Theme color of the text; the notification's text color when unset.
    pub color: Option<Color>,
    /// Keeps the notification open after the click (it is dismissed by default).
    pub no_dismiss: bool,
    /// Runs on click. Create it where it will outlive the toast (a callback is dropped with the
    /// component that made it, and then does nothing).
    pub on_click: Callback<()>,
}

impl NotifyAction {
    /// An action with a label that does nothing and dismisses the notification.
    pub fn new(label: impl Into<String>) -> Self {
        NotifyAction {
            label: label.into(),
            icon: None,
            color: None,
            no_dismiss: false,
            on_click: Callback::new(|()| {}),
        }
    }

    /// Sets [`icon`](Self::icon).
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets [`color`](Self::color).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets [`no_dismiss`](Self::no_dismiss).
    pub fn no_dismiss(mut self, no_dismiss: bool) -> Self {
        self.no_dismiss = no_dismiss;
        self
    }

    /// Sets [`on_click`](Self::on_click).
    pub fn on_click(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Callback::new(move |()| f());
        self
    }
}

/// Options of [`Plugins::notify`]. Plain data with chainable setters. `message`, `caption`,
/// `icon`, `spinner` and `timeout` can change while the notification is open
/// (`PluginHandle::update`); the rest is read when it opens.
///
/// Unset (`None`) fields come from the [`NotifyType`] preset, then from [`NotifySetup`].
#[derive(Clone, Debug, Default)]
pub struct NotifyOptions {
    /// Main text (plain text, never HTML).
    pub message: String,
    /// Smaller text under the message.
    pub caption: String,
    /// Preset of color, icon and behavior.
    pub type_: Option<NotifyType>,
    /// Theme color of the toast; the plain dark/light toast when unset.
    pub color: Option<Color>,
    /// Icon class (a literal `i-lucide-…`).
    pub icon: Option<&'static str>,
    /// Image URL shown before the text.
    pub avatar: Option<String>,
    /// Shows a spinner in place of the icon.
    pub spinner: bool,
    /// Where it appears; `NotifySetup::position` (bottom by default) when unset.
    pub position: Option<NotifyPosition>,
    /// How repeats are folded; `NotifySetup::group` (automatic by default) when unset.
    pub group: Option<NotifyGroup>,
    /// Shows how many times a grouped notification was repeated.
    pub badge: bool,
    /// Shows a bar for the remaining time; `NotifySetup::progress` when unset.
    pub progress: Option<bool>,
    /// Extra classes on the toast.
    pub class: Option<String>,
    /// Milliseconds until it closes; `0` = never; `NotifySetup::timeout` (5000 by default) when
    /// unset.
    pub timeout: Option<u32>,
    /// Buttons after the text.
    pub actions: Vec<NotifyAction>,
    /// A close button with this label; an empty label means `NotifySetup::close_btn_label`.
    pub close_btn: Option<String>,
    /// Actions go on their own row. Unset: true with more than one action.
    pub multi_line: Option<bool>,
    /// Skips [`NotifySetup`] (the type preset still applies).
    pub ignore_defaults: bool,
}

impl NotifyOptions {
    /// Sets [`message`](Self::message).
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Sets [`caption`](Self::caption).
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = caption.into();
        self
    }

    /// Sets [`type_`](Self::type_).
    pub fn type_(mut self, type_: NotifyType) -> Self {
        self.type_ = Some(type_);
        self
    }

    /// Sets [`color`](Self::color).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets [`icon`](Self::icon).
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets [`avatar`](Self::avatar).
    pub fn avatar(mut self, url: impl Into<String>) -> Self {
        self.avatar = Some(url.into());
        self
    }

    /// Sets [`spinner`](Self::spinner).
    pub fn spinner(mut self, spinner: bool) -> Self {
        self.spinner = spinner;
        self
    }

    /// Sets [`position`](Self::position).
    pub fn position(mut self, position: NotifyPosition) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets [`group`](Self::group).
    pub fn group(mut self, group: NotifyGroup) -> Self {
        self.group = Some(group);
        self
    }

    /// Sets [`badge`](Self::badge).
    pub fn badge(mut self, badge: bool) -> Self {
        self.badge = badge;
        self
    }

    /// Sets [`progress`](Self::progress).
    pub fn progress(mut self, progress: bool) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Sets [`class`](Self::class).
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Sets [`timeout`](Self::timeout), in milliseconds (`0` = never closes).
    pub fn timeout(mut self, ms: u32) -> Self {
        self.timeout = Some(ms);
        self
    }

    /// Adds one action to [`actions`](Self::actions).
    pub fn action(mut self, action: NotifyAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Sets [`actions`](Self::actions).
    pub fn actions(mut self, actions: Vec<NotifyAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Sets [`close_btn`](Self::close_btn) to a button with this label.
    pub fn close_btn(mut self, label: impl Into<String>) -> Self {
        self.close_btn = Some(label.into());
        self
    }

    /// Sets [`close_btn`](Self::close_btn) to a button with the default label
    /// (`NotifySetup::close_btn_label`).
    pub fn close_btn_default(mut self) -> Self {
        self.close_btn = Some(String::new());
        self
    }

    /// Sets [`multi_line`](Self::multi_line).
    pub fn multi_line(mut self, multi_line: bool) -> Self {
        self.multi_line = Some(multi_line);
        self
    }

    /// Sets [`ignore_defaults`](Self::ignore_defaults).
    pub fn ignore_defaults(mut self, ignore: bool) -> Self {
        self.ignore_defaults = ignore;
        self
    }
}

// ── Global setup and custom types ──────────────────────────────────────────

/// Page-global defaults of the Notify plugin, set once at boot with [`NotifyPlugin::setup`]. They
/// are merged into every notification that leaves the matching option unset, unless it has
/// `ignore_defaults`.
#[derive(Clone, Debug, PartialEq)]
pub struct NotifySetup {
    /// Where notifications appear. Default [`NotifyPosition::Bottom`].
    pub position: NotifyPosition,
    /// Milliseconds until a notification closes; `0` = never. Default `5000`.
    pub timeout: u32,
    /// Shows the remaining-time bar. Default `false`.
    pub progress: bool,
    /// How repeats are folded. Default [`NotifyGroup::Auto`].
    pub group: NotifyGroup,
    /// Label of a close button that asks for the default one. Default `"Fechar"`.
    pub close_btn_label: String,
}

impl Default for NotifySetup {
    fn default() -> Self {
        NotifySetup {
            position: NotifyPosition::Bottom,
            timeout: 5000,
            progress: false,
            group: NotifyGroup::Auto,
            close_btn_label: "Fechar".to_string(),
        }
    }
}

thread_local! {
    static SETUP: RefCell<NotifySetup> = RefCell::new(NotifySetup::default());
    static TYPES: RefCell<HashMap<&'static str, NotifyOptions>> = RefCell::new(HashMap::new());
}

/// Namespace of the Notify plugin's page-global configuration.
pub struct NotifyPlugin;

impl NotifyPlugin {
    /// Sets the page-global defaults of every notification. Call it once at boot; it is read when
    /// a notification opens, and calling it again replaces it. Being page-global, several app
    /// instances of the same wasm share it.
    pub fn setup(setup: NotifySetup) {
        SETUP.with(|s| *s.borrow_mut() = setup);
    }

    /// Adds (or replaces) a preset used by [`NotifyType::Custom`]`(name)`. Page-global, like
    /// [`setup`](Self::setup).
    pub fn register_type(name: &'static str, options: NotifyOptions) {
        TYPES.with(|t| {
            t.borrow_mut().insert(name, options);
        });
    }

    fn current() -> NotifySetup {
        SETUP.with(|s| s.borrow().clone())
    }
}

// ── Pure logic ─────────────────────────────────────────────────────────────

/// The preset of a type, as options whose set fields win over the setup.
fn preset(type_: Option<&NotifyType>) -> NotifyOptions {
    let base = NotifyOptions::default();
    match type_ {
        None => base,
        Some(NotifyType::Positive) => NotifyOptions {
            color: Some(Color::Positive),
            icon: Some("i-lucide-circle-check"),
            ..base
        },
        Some(NotifyType::Negative) => NotifyOptions {
            color: Some(Color::Negative),
            icon: Some("i-lucide-circle-alert"),
            ..base
        },
        Some(NotifyType::Warning) => NotifyOptions {
            color: Some(Color::Warning),
            icon: Some("i-lucide-triangle-alert"),
            ..base
        },
        Some(NotifyType::Info) => NotifyOptions {
            color: Some(Color::Info),
            icon: Some("i-lucide-info"),
            ..base
        },
        Some(NotifyType::Ongoing) => NotifyOptions {
            spinner: true,
            timeout: Some(0),
            ..base
        },
        Some(NotifyType::Custom(name)) => TYPES
            .with(|t| t.borrow().get(name).cloned())
            .unwrap_or(base),
    }
}

/// A notification after merging setup, type preset and options.
#[derive(Clone, Debug)]
struct Resolved {
    message: String,
    caption: String,
    color: Option<Color>,
    icon: Option<&'static str>,
    avatar: Option<String>,
    spinner: bool,
    position: NotifyPosition,
    group: NotifyGroup,
    badge: bool,
    progress: bool,
    class: String,
    timeout: u32,
    actions: Vec<NotifyAction>,
    close_btn: Option<String>,
    multi_line: bool,
    alert: bool,
}

/// Merges, from the weakest: built-in defaults (or the setup), the type preset, the options.
fn resolve(o: &NotifyOptions, setup: &NotifySetup) -> Resolved {
    let preset = preset(o.type_.as_ref());
    let defaults = if o.ignore_defaults {
        NotifySetup::default()
    } else {
        setup.clone()
    };
    let text = |own: &str, from_preset: String| {
        if own.is_empty() {
            from_preset
        } else {
            own.to_string()
        }
    };
    let actions = if o.actions.is_empty() {
        preset.actions
    } else {
        o.actions.clone()
    };
    let multi_line = o
        .multi_line
        .or(preset.multi_line)
        .unwrap_or(actions.len() > 1);
    let close_btn = o
        .close_btn
        .clone()
        .or(preset.close_btn)
        .map(|label| match label.is_empty() {
            true => defaults.close_btn_label.clone(),
            false => label,
        });
    Resolved {
        message: text(&o.message, preset.message),
        caption: text(&o.caption, preset.caption),
        color: o.color.or(preset.color),
        icon: o.icon.or(preset.icon),
        avatar: o.avatar.clone().or(preset.avatar),
        spinner: o.spinner || preset.spinner,
        position: o.position.or(preset.position).unwrap_or(defaults.position),
        group: o
            .group
            .clone()
            .or(preset.group)
            .unwrap_or_else(|| defaults.group.clone()),
        badge: o.badge || preset.badge,
        progress: o.progress.or(preset.progress).unwrap_or(defaults.progress),
        class: cls([
            preset.class.as_deref().unwrap_or_default(),
            o.class.as_deref().unwrap_or_default(),
        ]),
        timeout: o.timeout.or(preset.timeout).unwrap_or(defaults.timeout),
        actions,
        close_btn,
        multi_line,
        alert: o.type_ == Some(NotifyType::Negative),
    }
}

impl Resolved {
    /// Identity of the toast for grouping; `None` = never grouped.
    fn group_key(&self) -> Option<String> {
        match &self.group {
            NotifyGroup::Off => None,
            NotifyGroup::Auto => Some(format!(
                "auto|{}|{}|{:?}",
                self.message, self.caption, self.position
            )),
            NotifyGroup::Key(key) => Some(format!("key|{key}|{:?}", self.position)),
        }
    }
}

/// A repeat of a grouped notification: the count goes up and the timeout starts over.
/// Returns the new `(count, restart)`.
fn repeated(count: u32, restart: u32) -> (u32, u32) {
    (count.saturating_add(1), restart.wrapping_add(1))
}

/// The badge number to show: only for `badge` toasts that were repeated.
fn badge_count(badge: bool, count: u32) -> Option<u32> {
    (badge && count > 1).then_some(count)
}

/// Delay of the close timer, or `None` for "never" (`0`) or a toast that is already closing.
fn timer_ms(timeout: u32, open: bool) -> Option<u32> {
    (open && timeout > 0).then_some(timeout)
}

// ── Host state ─────────────────────────────────────────────────────────────

/// A toast the layer draws.
#[derive(Clone)]
struct ToastItem {
    key: u64,
    position: NotifyPosition,
    ctx: PluginCtx<()>,
    props: ReadSignal<NotifyOptions, LocalStorage>,
    /// How many times the grouped notification was shown.
    count: RwSignal<u32>,
    /// Bumped on every repeat: the timer and the progress bar start over.
    restart: RwSignal<u32>,
    setup: NotifySetup,
}

/// A live notification that later repeats fold into.
#[derive(Clone)]
struct Group {
    id: u64,
    key: String,
    handle: NotifyHandle,
    count: RwSignal<u32>,
    restart: RwSignal<u32>,
}

/// Per-host state, kept with `Plugins::get_or_init`.
#[derive(Clone)]
struct NotifyState {
    /// The host's owner: signals that outlive the caller are created under it.
    owner: Owner,
    items: RwSignal<Vec<ToastItem>, LocalStorage>,
    /// Bumped when a toast is added (the layer then makes sure it is above any modal).
    added: RwSignal<u32>,
    groups: Rc<RefCell<Vec<Group>>>,
    /// The layer entry is open.
    layer: Rc<Cell<bool>>,
    next: Rc<Cell<u64>>,
}

impl NotifyState {
    fn new() -> Self {
        NotifyState {
            owner: Owner::current().unwrap_or_default(),
            items: RwSignal::new_local(Vec::new()),
            added: RwSignal::new(0),
            groups: Rc::new(RefCell::new(Vec::new())),
            layer: Rc::new(Cell::new(false)),
            next: Rc::new(Cell::new(0)),
        }
    }

    fn next_id(&self) -> u64 {
        self.next.set(self.next.get() + 1);
        self.next.get()
    }

    /// Folds a repeat into the live toast of `key`, if any.
    fn repeat(&self, key: &str) -> Option<NotifyHandle> {
        let found = self
            .groups
            .borrow()
            .iter()
            .find(|g| g.key == key && g.handle.is_open())
            .cloned()?;
        let (count, restart) = repeated(found.count.get_untracked(), found.restart.get_untracked());
        found.count.set(count);
        found.restart.set(restart);
        Some(found.handle)
    }

    /// Opens the layer entry the first time (and again if the host dropped it).
    fn ensure_layer(&self, plugins: Plugins) {
        if self.layer.replace(true) {
            return;
        }
        let state = self.clone();
        let handle = plugins.open(move |_ctx: PluginCtx<()>| layer_view(state));
        handle.set_persistent(true);
        let flag = self.layer.clone();
        handle.on_dismiss(move || flag.set(false));
    }
}

// ── The plugin ─────────────────────────────────────────────────────────────

/// Handle of an opened notification. Its props are the [`NotifyOptions`]: change `message`,
/// `caption`, `icon`, `spinner` or `timeout` while it is open with `update`; `hide` dismisses it.
///
/// `.await`ing it gives `Ok(())` when the user used an action or the close button,
/// `Err(Dismissed::Cancel)` when the timeout ran out and `Err(Dismissed::Programmatic)` after
/// `hide`. A repeat of a grouped notification returns the handle of the toast it joined.
pub type NotifyHandle = PluginHandle<(), NotifyOptions>;

impl Plugins {
    /// Shows a notification. Needs the layer of this host, so the app must have
    /// `provide_plugins()` and a `<PluginHost/>`.
    ///
    /// Notifications are not dismissed by [`Plugins::dismiss_all`] (they survive navigation). The
    /// toasts sit in the top layer above any modal `Dialog` open when they appear, but content
    /// outside a modal is inert: while the modal is open its actions cannot be clicked.
    pub fn notify(&self, options: NotifyOptions) -> NotifyHandle {
        let setup = NotifyPlugin::current();
        let resolved = resolve(&options, &setup);
        let state = self.get_or_init(NotifyState::new);
        let group_key = resolved.group_key();
        if let Some(handle) = group_key.as_deref().and_then(|k| state.repeat(k)) {
            return handle;
        }
        state.ensure_layer(*self);

        let id = state.next_id();
        let count = state.owner.with(|| RwSignal::new(1_u32));
        let restart = state.owner.with(|| RwSignal::new(0_u32));
        let (items, added) = (state.items, state.added);
        let position = resolved.position;
        let handle = self.open_with_props(options, move |ctx, props| {
            // The ghost view: register the toast with the layer and take it out again with the
            // row (the host drops the row when the entry is removed).
            items.update(|list| {
                list.push(ToastItem {
                    key: id,
                    position,
                    ctx,
                    props,
                    count,
                    restart,
                    setup,
                })
            });
            added.update(|n| *n = n.wrapping_add(1));
            on_cleanup(move || {
                items.try_update(|list| list.retain(|t| t.key != id));
            });
        });
        handle.set_persistent(true);
        if let Some(key) = group_key {
            state.groups.borrow_mut().push(Group {
                id,
                key,
                handle,
                count,
                restart,
            });
            let groups = state.groups.clone();
            handle.on_dismiss(move || groups.borrow_mut().retain(|g| g.id != id));
        }
        handle
    }
}

// ── Views ──────────────────────────────────────────────────────────────────

/// Enter and leave of one toast.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Enter,
    Open,
    Leave,
}

/// The `popover="manual"` layer with one stack per position.
fn layer_view(state: NotifyState) -> AnyView {
    let layer = NodeRef::<leptos::html::Div>::new();
    let (items, added) = (state.items, state.added);
    Effect::new(move |previous: Option<u32>| {
        // Tracked reads first, so the effect has dependencies even before the node exists.
        let node = layer.get();
        let count = items.with(Vec::len);
        let added_now = added.get();
        let Some(node) = node else {
            return added_now;
        };
        let open = node.matches(":popover-open").unwrap_or(false);
        if count == 0 {
            if open {
                let _ = node.hide_popover();
            }
        } else if !open {
            let _ = node.show_popover();
        } else if previous.is_some_and(|p| p != added_now)
            && crate::root::query_near(&node, "dialog:modal").is_some()
        {
            // A modal opened after the layer would cover it: showing it again puts it last in the
            // top layer.
            let _ = node.hide_popover();
            let _ = node.show_popover();
        }
        added_now
    });

    view! {
        <div node_ref=layer popover="manual" class="lx-notify-layer">
            {NotifyPosition::ALL.map(|position| stack_view(position, items))}
        </div>
    }
    .into_any()
}

fn stack_view(position: NotifyPosition, items: RwSignal<Vec<ToastItem>, LocalStorage>) -> AnyView {
    view! {
        <div class=position.stack_class() aria-live="polite">
            <For
                each=move || {
                    items
                        .get()
                        .into_iter()
                        .filter(|t| t.position == position)
                        .collect::<Vec<_>>()
                }
                key=|t| t.key
                children=toast_view
            />
        </div>
    }
    .into_any()
}

/// The measured length of the toast's animation, in ms (`0` when animations are off).
fn animation_ms(el: &web_sys::Element) -> u32 {
    web_sys::window()
        .and_then(|w| w.get_computed_style(el).ok().flatten())
        .and_then(|s| s.get_property_value("transition-duration").ok())
        .map(|d| crate::util::parse_duration_ms(&d))
        .unwrap_or(0)
}

fn toast_view(t: ToastItem) -> AnyView {
    let ToastItem {
        position,
        ctx,
        props,
        count,
        restart,
        setup,
        ..
    } = t;

    // Parts read once, when the toast opens.
    let first = resolve(&props.get_untracked(), &setup);
    // Parts `update` may change, each behind a `Memo` so the rest of the DOM stays put.
    let message = {
        let setup = setup.clone();
        Memo::new(move |_| {
            props
                .try_with(|o| resolve(o, &setup).message)
                .unwrap_or_default()
        })
    };
    let caption = {
        let setup = setup.clone();
        Memo::new(move |_| {
            props
                .try_with(|o| resolve(o, &setup).caption)
                .unwrap_or_default()
        })
    };
    let icon = {
        let setup = setup.clone();
        Memo::new(move |_| props.try_with(|o| resolve(o, &setup).icon).flatten())
    };
    let spinner = {
        let setup = setup.clone();
        Memo::new(move |_| {
            props
                .try_with(|o| resolve(o, &setup).spinner)
                .unwrap_or(false)
        })
    };
    let timeout = {
        let setup = setup.clone();
        Memo::new(move |_| props.try_with(|o| resolve(o, &setup).timeout).unwrap_or(0))
    };

    let el = NodeRef::<leptos::html::Div>::new();
    let phase = RwSignal::new(Phase::Enter);
    let timer = StoredValue::new_local(None::<TimeoutHandle>);
    let clear_timer = move || {
        if let Some(Some(handle)) = timer.try_update_value(|t| t.take()) {
            handle.clear();
        }
    };
    on_cleanup(clear_timer);

    // Enter: two frames after mounting, so the browser has painted the start state.
    Effect::new(move |_| {
        request_animation_frame(move || {
            request_animation_frame(move || {
                if phase.try_get_untracked() == Some(Phase::Enter) {
                    phase.try_set(Phase::Open);
                }
            })
        });
    });

    // The close timer: starts again on a repeat or when `update` changes the timeout.
    Effect::new(move |_| {
        let ms = timeout.get();
        restart.get();
        let open = ctx.show().try_get().unwrap_or(false);
        clear_timer();
        if let Some(ms) = timer_ms(ms, open)
            && let Ok(handle) =
                set_timeout_with_handle(move || ctx.cancel(), Duration::from_millis(u64::from(ms)))
        {
            timer.try_set_value(Some(handle));
        }
    });

    // Leave: when the entry starts closing, wait for the animation and then end the entry.
    Effect::new(move |_| {
        if ctx.show().try_get().unwrap_or(true) || phase.get_untracked() == Phase::Leave {
            return;
        }
        phase.set(Phase::Leave);
        let ms = el.get_untracked().map(|e| animation_ms(&e)).unwrap_or(0);
        if ms == 0 {
            ctx.finish();
        } else {
            set_timeout(
                move || ctx.finish(),
                Duration::from_millis(u64::from(ms) + 40),
            );
        }
    });

    let from_above = position.enters_from_above();
    let multi = first.multi_line;
    let color = first.color;
    let extra_class = first.class.clone();
    let class = move || {
        let look = match color {
            Some(c) => cls(["lx-notify-toned", c.tone_class()]),
            None => "lx-notify-plain".to_string(),
        };
        let layout = match multi {
            true => "lx-notify-multi",
            false => "lx-notify-single",
        };
        let state = match (phase.get(), from_above) {
            (Phase::Open, _) => "lx-notify-open",
            (_, true) => "lx-notify-enter-up",
            (_, false) => "lx-notify-enter-down",
        };
        cls(["lx-notify-item", &look, layout, state, &extra_class])
    };
    let role = match first.alert {
        true => "alert",
        false => "status",
    };

    let media = move || {
        if spinner.get() {
            Some(
                view! {
                    <span class="lx-notify-icon">
                        <Spinner size="1em" />
                    </span>
                }
                .into_any(),
            )
        } else {
            icon.get().map(|name| {
                view! {
                    <span class="lx-notify-icon">
                        <Icon name=name />
                    </span>
                }
                .into_any()
            })
        }
    };
    let avatar = first
        .avatar
        .clone()
        .map(|src| view! { <img class="lx-notify-avatar" src=src alt="" /> });
    let caption_view = move || {
        let text = caption.get();
        (!text.is_empty()).then(|| view! { <div class="lx-notify-caption">{text}</div> })
    };

    let actions_class = match multi {
        true => "lx-notify-actions lx-notify-actions-multi",
        false => "lx-notify-actions",
    };
    let action_views = first
        .actions
        .iter()
        .cloned()
        .map(|action| {
            let NotifyAction {
                label,
                icon,
                color,
                no_dismiss,
                on_click,
            } = action;
            let class = match color {
                Some(c) => cls(["lx-notify-action", "lx-notify-action-tone", c.tone_class()]),
                None => "lx-notify-action".to_string(),
            };
            view! {
                <button
                    type="button"
                    class=class
                    on:click=move |_| {
                        let _ = on_click.try_run(());
                        if !no_dismiss {
                            ctx.ok(());
                        }
                    }
                >
                    {icon.map(|name| view! { <Icon name=name /> })}
                    <span>{label}</span>
                </button>
            }
        })
        .collect_view();
    let close_view = first.close_btn.clone().map(|label| {
        view! {
            <button type="button" class="lx-notify-action" on:click=move |_| ctx.ok(())>
                {label}
            </button>
        }
    });
    let has_buttons = !first.actions.is_empty() || first.close_btn.is_some();
    let actions_view = has_buttons.then(|| {
        view! {
            <div class=actions_class>
                {action_views}
                {close_view}
            </div>
        }
    });

    let want_badge = first.badge;
    let badge_class = match position.is_left() {
        true => "lx-notify-badge lx-notify-badge-end",
        false => "lx-notify-badge lx-notify-badge-start",
    };
    let badge_view = move || {
        badge_count(want_badge, count.get())
            .map(|n| view! { <span class=badge_class aria-hidden="true">{n}</span> })
    };
    let want_progress = first.progress;
    let progress_view = move || {
        if !want_progress {
            return None;
        }
        // Re-rendered when the timeout or the repeat count changes, which restarts the bar.
        restart.get();
        let ms = timeout.get();
        (ms > 0).then(|| {
            view! {
                <div
                    class="lx-notify-progress"
                    aria-hidden="true"
                    style=format!("--lx-notify-progress-duration:{ms}ms")
                ></div>
            }
        })
    };

    view! {
        <div node_ref=el class=class role=role>
            {avatar}
            {media}
            <div class="lx-notify-text">
                <div class="lx-notify-message">{move || message.get()}</div>
                {caption_view}
            </div>
            {actions_view}
            {badge_view}
            {progress_view}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> NotifyOptions {
        NotifyOptions::default().message("Salvo")
    }

    #[test]
    fn defaults_come_from_the_built_in_setup() {
        let r = resolve(&opts(), &NotifySetup::default());
        assert_eq!(r.position, NotifyPosition::Bottom);
        assert_eq!(r.timeout, 5000);
        assert!(!r.progress);
        assert_eq!(r.group, NotifyGroup::Auto);
        assert!(!r.alert);
    }

    #[test]
    fn the_setup_fills_what_the_options_leave_unset() {
        let setup = NotifySetup {
            position: NotifyPosition::TopRight,
            timeout: 1200,
            progress: true,
            group: NotifyGroup::Off,
            close_btn_label: "Dispensar".into(),
        };
        let r = resolve(&opts(), &setup);
        assert_eq!(
            (r.position, r.timeout, r.progress, r.group),
            (NotifyPosition::TopRight, 1200, true, NotifyGroup::Off)
        );
        // What the options say wins, including `0` (never) over a non-zero default.
        let own = opts()
            .position(NotifyPosition::Left)
            .timeout(0)
            .progress(false)
            .group(NotifyGroup::Auto);
        let r = resolve(&own, &setup);
        assert_eq!(
            (r.position, r.timeout, r.progress, r.group),
            (NotifyPosition::Left, 0, false, NotifyGroup::Auto)
        );
    }

    #[test]
    fn ignore_defaults_skips_the_setup_but_not_the_type_preset() {
        let setup = NotifySetup {
            position: NotifyPosition::Top,
            timeout: 99,
            ..NotifySetup::default()
        };
        let o = opts().type_(NotifyType::Positive).ignore_defaults(true);
        let r = resolve(&o, &setup);
        assert_eq!(r.position, NotifyPosition::Bottom);
        assert_eq!(r.timeout, 5000);
        assert_eq!(r.color, Some(Color::Positive));
        assert_eq!(r.icon, Some("i-lucide-circle-check"));
    }

    #[test]
    fn options_beat_the_type_preset_which_beats_the_setup() {
        let setup = NotifySetup {
            timeout: 7000,
            ..NotifySetup::default()
        };
        let ongoing = resolve(&opts().type_(NotifyType::Ongoing), &setup);
        assert!(ongoing.spinner);
        assert_eq!(ongoing.timeout, 0);
        let own = resolve(&opts().type_(NotifyType::Ongoing).timeout(300), &setup);
        assert_eq!(own.timeout, 300);
        let colored = resolve(
            &opts().type_(NotifyType::Negative).color(Color::Warning),
            &setup,
        );
        assert_eq!(colored.color, Some(Color::Warning));
        assert_eq!(colored.icon, Some("i-lucide-circle-alert"));
        assert!(colored.alert);
        assert!(!resolve(&opts().type_(NotifyType::Info), &setup).alert);
    }

    #[test]
    fn a_registered_type_is_a_preset_and_an_unknown_one_adds_nothing() {
        NotifyPlugin::register_type(
            "tests-saved",
            NotifyOptions::default()
                .color(Color::Secondary)
                .icon("i-lucide-check")
                .timeout(1500)
                .class("extra"),
        );
        let r = resolve(
            &opts()
                .type_(NotifyType::Custom("tests-saved"))
                .class("mine"),
            &NotifySetup::default(),
        );
        assert_eq!(r.color, Some(Color::Secondary));
        assert_eq!(r.timeout, 1500);
        assert_eq!(r.class, "extra mine");
        let none = resolve(
            &opts().type_(NotifyType::Custom("tests-unknown")),
            &NotifySetup::default(),
        );
        assert_eq!((none.color, none.icon, none.timeout), (None, None, 5000));
    }

    #[test]
    fn the_close_button_label_falls_back_to_the_setup() {
        let setup = NotifySetup {
            close_btn_label: "Dispensar".into(),
            ..NotifySetup::default()
        };
        assert_eq!(
            resolve(&opts().close_btn_default(), &setup).close_btn,
            Some("Dispensar".to_string())
        );
        assert_eq!(
            resolve(&opts().close_btn("Ok"), &setup).close_btn,
            Some("Ok".to_string())
        );
        assert_eq!(resolve(&opts(), &setup).close_btn, None);
    }

    #[test]
    fn multi_line_is_automatic_with_more_than_one_action() {
        let setup = NotifySetup::default();
        let one = opts().action(NotifyAction::new("Desfazer"));
        let two = one.clone().action(NotifyAction::new("Ver"));
        assert!(!resolve(&opts(), &setup).multi_line);
        assert!(!resolve(&one, &setup).multi_line);
        assert!(resolve(&two, &setup).multi_line);
        assert!(!resolve(&two.clone().multi_line(false), &setup).multi_line);
        assert!(resolve(&one.multi_line(true), &setup).multi_line);
    }

    #[test]
    fn auto_group_key_is_message_caption_and_position() {
        let setup = NotifySetup::default();
        let key = |o: NotifyOptions| resolve(&o, &setup).group_key();
        let base = opts().caption("c");
        assert_eq!(key(base.clone()), key(base.clone().timeout(10)));
        assert_ne!(key(base.clone()), key(opts()));
        assert_ne!(key(base.clone()), key(base.clone().caption("d")));
        assert_ne!(
            key(base.clone()),
            key(base.clone().position(NotifyPosition::Top))
        );
    }

    #[test]
    fn explicit_group_keys_and_off() {
        let setup = NotifySetup::default();
        let key = |o: NotifyOptions| resolve(&o, &setup).group_key();
        let a = opts().group(NotifyGroup::Key("sync".into()));
        let b = NotifyOptions::default()
            .message("outro texto")
            .group(NotifyGroup::Key("sync".into()));
        assert_eq!(key(a.clone()), key(b.clone()));
        assert_ne!(key(a.clone()), key(a.position(NotifyPosition::Top)));
        assert_eq!(key(opts().group(NotifyGroup::Off)), None);
        // The setup can turn grouping off for everything.
        let off = NotifySetup {
            group: NotifyGroup::Off,
            ..NotifySetup::default()
        };
        assert_eq!(resolve(&opts(), &off).group_key(), None);
    }

    #[test]
    fn a_repeat_bumps_the_count_and_restarts_the_timer() {
        assert_eq!(repeated(1, 0), (2, 1));
        assert_eq!(repeated(2, 1), (3, 2));
        assert_eq!(repeated(u32::MAX, u32::MAX), (u32::MAX, 0));
    }

    #[test]
    fn the_badge_shows_only_for_repeated_badge_toasts() {
        assert_eq!(badge_count(true, 1), None);
        assert_eq!(badge_count(true, 3), Some(3));
        assert_eq!(badge_count(false, 3), None);
    }

    #[test]
    fn the_timer_runs_only_for_a_positive_timeout_on_an_open_toast() {
        assert_eq!(timer_ms(5000, true), Some(5000));
        assert_eq!(timer_ms(0, true), None);
        assert_eq!(timer_ms(5000, false), None);
    }

    #[test]
    fn setup_is_global_and_replaceable() {
        let before = NotifyPlugin::current();
        NotifyPlugin::setup(NotifySetup {
            timeout: 123,
            ..NotifySetup::default()
        });
        assert_eq!(NotifyPlugin::current().timeout, 123);
        NotifyPlugin::setup(before);
    }
}
