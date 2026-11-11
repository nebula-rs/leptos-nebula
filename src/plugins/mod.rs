//! Programmatic components ("plugins", ADR-0019): open a view from code and get a result back.
//!
//! The app calls [`provide_plugins`] once at its root and places one [`PluginHost`] (inside the
//! `Router` when views need router contexts). Code that wants to open something calls
//! [`use_plugins`] — while rendering or in an event handler, **before** any `.await` — and then
//! [`Plugins::open`]. The host renders the opened views through a `Portal` at the app root
//! (the shadow root in web component mode), so they see the contexts visible at the host, not the
//! caller's local ones.
//!
//! Everything opens in the browser only; on the server `open` returns an unavailable handle.

mod dialog;
mod dialog_option_group;
mod notify;
mod settle;

use std::{
    any::Any, any::TypeId, cell::Cell, collections::HashMap, future::Future, pin::Pin, rc::Rc,
    task::Poll,
};

use futures::channel::oneshot;
use leptos::{portal::Portal, prelude::*};

pub use dialog::{
    DialogBtn, DialogBtnProps, DialogFocus, DialogForm, DialogFormParts, DialogHandle,
    DialogOptions, DialogPlugin, DialogPrompt, DialogSetup,
};
pub use dialog_option_group::{DialogOptionGroup, DialogOptionGroupMultiple};
pub use notify::{
    NotifyAction, NotifyGroup, NotifyHandle, NotifyOptions, NotifyPlugin, NotifyPosition,
    NotifySetup, NotifyType,
};
pub use settle::Dismissed;
use settle::Settlement;

// ── Host state ─────────────────────────────────────────────────────────────

/// Type-erased part of an entry, what the host needs without knowing `T`.
struct EntryState {
    /// The view builder; taken by the host row, which calls it once.
    view: Option<Box<dyn FnOnce() -> AnyView>>,
    /// Starts closing the entry (as [`Plugins::dismiss_all`] does).
    close: Rc<dyn Fn(Dismissed)>,
    /// Settles the entry as unavailable if still pending (the row went away).
    abandon: Rc<dyn Fn()>,
    /// Disposes the entry's reactive state.
    dispose: Rc<dyn Fn()>,
}

/// One entry in the host's stack.
#[derive(Clone, Copy)]
struct Row {
    id: u64,
    persistent: StoredValue<bool>,
    state: StoredValue<EntryState, LocalStorage>,
}

struct HostInner {
    /// Owner of `provide_plugins`: entry state is allocated under it, not under the caller's.
    owner: Owner,
    entries: RwSignal<Vec<Row>>,
    next_id: u64,
    /// Number of mounted `<PluginHost/>`.
    hosts: u32,
    warned_no_host: bool,
    extras: HashMap<TypeId, Box<dyn Any>>,
}

/// Handle to the plugin host of the surrounding app. `Copy`; get it with [`use_plugins`] and
/// capture it before an `.await`.
///
/// An *unavailable* handle (no [`provide_plugins`] context, or running on the server) is inert:
/// opening returns an unavailable [`PluginHandle`] that settles with [`Dismissed::Unavailable`].
#[derive(Clone, Copy)]
pub struct Plugins(Option<StoredValue<HostInner, LocalStorage>>);

fn in_browser() -> bool {
    cfg!(target_arch = "wasm32") && Owner::current_shared_context().is_none_or(|c| c.is_browser())
}

thread_local! {
    static WARNED_NO_CONTEXT: Cell<bool> = const { Cell::new(false) };
}

/// Creates the plugin host state and provides it as context. Call it once, at the root of the app
/// (before [`PluginHost`]).
pub fn provide_plugins() -> Plugins {
    let owner = Owner::current().unwrap_or_default();
    let plugins = Plugins(Some(StoredValue::new_local(HostInner {
        owner,
        entries: RwSignal::new(Vec::new()),
        next_id: 0,
        hosts: 0,
        warned_no_host: false,
        extras: HashMap::new(),
    })));
    provide_context(plugins);
    plugins
}

/// The [`Plugins`] from the context. Without one it returns an unavailable handle and warns once
/// in the console: capture `use_plugins()` while rendering or in an event handler, before an
/// `.await` — the context is gone afterwards.
pub fn use_plugins() -> Plugins {
    match use_context::<Plugins>() {
        Some(p) => p,
        None => {
            if in_browser() && !WARNED_NO_CONTEXT.with(|w| w.replace(true)) {
                leptos::logging::warn!(
                    "use_plugins(): no `provide_plugins()` context here, so the plugin is \
                     unavailable. Call `use_plugins()` while rendering or in an event handler \
                     and capture the result before an `.await`."
                );
            }
            Plugins(None)
        }
    }
}

/// The [`Plugins`] from the context, if any (no warning).
pub fn try_use_plugins() -> Option<Plugins> {
    use_context::<Plugins>()
}

impl Plugins {
    /// Whether this handle is connected to a host.
    pub fn is_available(&self) -> bool {
        self.0.is_some()
    }

    /// Opens `view` on the host and returns its handle. `view` runs once, when the host renders the
    /// entry, with the entry's [`PluginCtx`]; it sees the contexts visible at the host. The entry
    /// stays until the view calls [`PluginCtx::finish`] (or a bound `Dialog` hides).
    ///
    /// On the server, or without a host, the handle is unavailable and settles with
    /// [`Dismissed::Unavailable`].
    pub fn open<T, V>(&self, view: impl FnOnce(PluginCtx<T>) -> V + 'static) -> PluginHandle<T>
    where
        T: Clone + 'static,
        V: IntoView + 'static,
    {
        self.open_with_props((), move |ctx, _| view(ctx))
    }

    /// Like [`open`](Self::open), with data the caller can change later through
    /// [`PluginHandle::update`]. The view receives it as a read-only signal.
    pub fn open_with_props<T, P, V>(
        &self,
        props: P,
        view: impl FnOnce(PluginCtx<T>, ReadSignal<P, LocalStorage>) -> V + 'static,
    ) -> PluginHandle<T, P>
    where
        T: Clone + 'static,
        P: 'static,
        V: IntoView + 'static,
    {
        let unavailable = PluginHandle {
            ctx: None,
            props: None,
        };
        if !in_browser() {
            return unavailable;
        }
        let Some(handle) = self.open_entry(props, view) else {
            return unavailable;
        };
        let (plugins, id) = match handle.ctx {
            Some(ctx) => (ctx.plugins, ctx.id),
            None => return handle,
        };
        // A host that is not mounted by the next frame never will render this entry.
        request_animation_frame(move || {
            let Some(ctx) = handle.ctx else { return };
            let no_host = plugins.0.is_some_and(|h| {
                h.try_update_value(|h| {
                    let none = h.hosts == 0;
                    if none && !h.warned_no_host {
                        h.warned_no_host = true;
                        leptos::logging::warn!(
                            "Plugins: no `<PluginHost/>` is mounted, so the opened plugin cannot \
                             render. Place `<PluginHost/>` once in the app, inside the `Router`."
                        );
                    }
                    none
                })
                .unwrap_or(false)
            });
            if no_host && !ctx.built.try_get_value().unwrap_or(true) {
                ctx.settle(Err(Dismissed::Unavailable));
                plugins.remove(id);
            }
        });
        handle
    }

    /// Creates the entry and queues it on the host; `None` without a host. Does not check the
    /// environment (that is `open_with_props`), so the host tests can use it.
    fn open_entry<T, P, V>(
        &self,
        props: P,
        view: impl FnOnce(PluginCtx<T>, ReadSignal<P, LocalStorage>) -> V + 'static,
    ) -> Option<PluginHandle<T, P>>
    where
        T: Clone + 'static,
        P: 'static,
        V: IntoView + 'static,
    {
        let host = self.0?;
        let (owner, id, entries) = host.try_update_value(|h| {
            h.next_id += 1;
            (h.owner.clone(), h.next_id, h.entries)
        })?;
        let plugins = *self;
        let (handle, row) = owner.with(|| {
            let ctx = PluginCtx {
                plugins,
                id,
                show: RwSignal::new(true),
                reason: StoredValue::new(Dismissed::User),
                built: StoredValue::new(false),
                persistent: StoredValue::new(false),
                settlement: StoredValue::new_local(Settlement::<T>::new()),
            };
            let props = RwSignal::new_local(props);
            let build = move || -> AnyView {
                ctx.built.try_set_value(true);
                provide_context(ctx);
                view(ctx, props.read_only()).into_any()
            };
            let state = StoredValue::new_local(EntryState {
                view: Some(Box::new(build)),
                close: Rc::new(move |why| ctx.request_close(why)),
                abandon: Rc::new(move || ctx.settle(Err(Dismissed::Unavailable))),
                dispose: Rc::new(move || {
                    ctx.show.dispose();
                    ctx.reason.dispose();
                    ctx.built.dispose();
                    ctx.persistent.dispose();
                    ctx.settlement.dispose();
                    props.dispose();
                }),
            });
            let row = Row {
                id,
                persistent: ctx.persistent,
                state,
            };
            (
                PluginHandle {
                    ctx: Some(ctx),
                    props: Some(props),
                },
                row,
            )
        });
        entries.update(|rows| rows.push(row));
        Some(handle)
    }

    /// Per-host state of a plugin (e.g. a toast container): created by `init` the first time any
    /// caller asks for an `S` and shared afterwards. `init` runs under the host's owner, so
    /// signals it creates live as long as the host. On an unavailable handle it returns a fresh,
    /// unshared `init()`.
    pub fn get_or_init<S: Clone + 'static>(&self, init: impl FnOnce() -> S) -> S {
        let Some(host) = self.0 else { return init() };
        let existing = host.try_with_value(|h| {
            h.extras
                .get(&TypeId::of::<S>())
                .and_then(|b| b.downcast_ref::<S>())
                .cloned()
        });
        if let Some(Some(found)) = existing {
            return found;
        }
        let Some(owner) = host.try_with_value(|h| h.owner.clone()) else {
            return init();
        };
        let created = owner.with(init);
        host.try_update_value(|h| {
            h.extras
                .entry(TypeId::of::<S>())
                .or_insert_with(|| Box::new(created.clone()))
                .downcast_ref::<S>()
                .cloned()
        })
        .flatten()
        .unwrap_or(created)
    }

    /// Starts closing every entry that is not persistent; each settles with
    /// [`Dismissed::Programmatic`] after its exit animation.
    pub fn dismiss_all(&self) {
        let Some(host) = self.0 else { return };
        let Some(entries) = host.try_with_value(|h| h.entries) else {
            return;
        };
        for row in entries.get_untracked() {
            if row.persistent.try_get_value() != Some(false) {
                continue;
            }
            if let Some(close) = row.state.try_with_value(|s| s.close.clone()) {
                close(Dismissed::Programmatic);
            }
        }
    }

    fn remove(&self, id: u64) {
        if let Some(entries) = self.0.and_then(|h| h.try_with_value(|h| h.entries)) {
            entries.try_update(|rows| rows.retain(|r| r.id != id));
        }
    }
}

// ── Entry context and handle ───────────────────────────────────────────────

/// What an opened view uses to talk to its entry: bind a `Dialog` with [`show`](Self::show) and
/// [`on_hide`](Self::on_hide), or settle it with [`ok`](Self::ok) / [`cancel`](Self::cancel).
/// `Copy`; also provided as context inside the view ([`use_plugin_ctx`]).
///
/// After the entry is removed the methods do nothing.
pub struct PluginCtx<T: 'static> {
    plugins: Plugins,
    id: u64,
    show: RwSignal<bool>,
    /// Why the entry closes, if it closes through `show` going to `false`.
    reason: StoredValue<Dismissed>,
    /// The host row already called the view builder.
    built: StoredValue<bool>,
    persistent: StoredValue<bool>,
    settlement: StoredValue<Settlement<T>, LocalStorage>,
}

impl<T: 'static> Clone for PluginCtx<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for PluginCtx<T> {}

/// The [`PluginCtx<T>`] of the entry this view is rendered for, if any.
pub fn use_plugin_ctx<T: Clone + 'static>() -> Option<PluginCtx<T>> {
    use_context::<PluginCtx<T>>()
}

impl<T: Clone + 'static> PluginCtx<T> {
    /// Open/closed state to bind to a `Dialog`'s `show`. Starts `true`; the entry closes when it
    /// goes to `false`.
    pub fn show(&self) -> RwSignal<bool> {
        self.show
    }

    /// Callback to bind to a `Dialog`'s `on_hide`: after the exit animation it settles the entry
    /// (with the reason it closed) and removes it. Same as [`finish`](Self::finish).
    pub fn on_hide(&self) -> Callback<()> {
        let ctx = *self;
        Callback::new(move |()| ctx.finish())
    }

    /// Settles with `Ok(value)` right away and starts closing.
    pub fn ok(&self, value: T) {
        self.settle(Ok(value));
        self.begin_close();
    }

    /// Starts closing; the entry settles with [`Dismissed::Cancel`] after the exit animation.
    pub fn cancel(&self) {
        self.request_close(Dismissed::Cancel);
    }

    /// Starts closing; the entry settles with [`Dismissed::Programmatic`] after the exit
    /// animation.
    pub fn hide(&self) {
        self.request_close(Dismissed::Programmatic);
    }

    /// Settles the entry now (with the reason it was closing, [`Dismissed::User`] if none) and
    /// removes it from the host. For views that do not use a `Dialog`.
    pub fn finish(&self) {
        let why = self
            .reason
            .try_get_value()
            .unwrap_or(Dismissed::Unavailable);
        self.settle(Err(why));
        self.show.try_set(false);
        self.plugins.remove(self.id);
    }

    /// The host this entry runs on.
    pub fn plugins(&self) -> Plugins {
        self.plugins
    }

    fn settle(&self, result: Result<T, Dismissed>) {
        if let Some(flush) = self.settlement.try_update_value(|s| s.settle(result)) {
            flush.run();
        }
    }

    fn request_close(&self, why: Dismissed) {
        if self.show.try_get_untracked() != Some(true) {
            return;
        }
        self.reason.try_set_value(why);
        self.begin_close();
    }

    fn begin_close(&self) {
        if self.built.try_get_value() == Some(false) {
            // Never rendered, so no `Dialog` will report `on_hide`: end it here.
            self.finish();
        } else {
            self.show.try_set(false);
        }
    }
}

/// Handle to an opened plugin entry. `Copy`; it expires when the entry is removed, so attach
/// listeners (or `.await` it) right after opening.
///
/// `.await`ing it gives `Result<T, Dismissed>`. An unavailable handle settles with
/// [`Dismissed::Unavailable`] (listeners for that fire immediately).
pub struct PluginHandle<T: 'static, P: 'static = ()> {
    ctx: Option<PluginCtx<T>>,
    props: Option<RwSignal<P, LocalStorage>>,
}

impl<T: 'static, P: 'static> Clone for PluginHandle<T, P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, P: 'static> Copy for PluginHandle<T, P> {}

impl<T: Clone + 'static, P: 'static> PluginHandle<T, P> {
    /// Calls `f` with the value when the entry settles with `Ok`.
    pub fn on_ok(self, f: impl FnOnce(T) + 'static) -> Self {
        if let Some(ctx) = self.ctx
            && let Some(flush) = ctx.settlement.try_update_value(|s| s.on_ok(f))
        {
            flush.run();
        }
        self
    }

    /// Calls `f` with the reason when the entry settles without a value. On an unavailable
    /// handle it runs right away with [`Dismissed::Unavailable`].
    pub fn on_cancel(self, f: impl FnOnce(Dismissed) + 'static) -> Self {
        match self.ctx {
            Some(ctx) => {
                if let Some(flush) = ctx.settlement.try_update_value(|s| s.on_cancel(f)) {
                    flush.run();
                }
            }
            None => f(Dismissed::Unavailable),
        }
        self
    }

    /// Calls `f` when the entry settles, whichever way. On an unavailable handle it runs right
    /// away.
    pub fn on_dismiss(self, f: impl FnOnce() + 'static) -> Self {
        match self.ctx {
            Some(ctx) => {
                if let Some(flush) = ctx.settlement.try_update_value(|s| s.wait(move |_| f())) {
                    flush.run();
                }
            }
            None => f(),
        }
        self
    }

    /// Starts closing; the entry settles with [`Dismissed::Programmatic`] after the exit
    /// animation.
    pub fn hide(&self) {
        if let Some(ctx) = self.ctx {
            ctx.request_close(Dismissed::Programmatic);
        }
    }

    /// Changes the props the entry was opened with (see [`Plugins::open_with_props`]).
    pub fn update(&self, f: impl FnOnce(&mut P)) {
        if let Some(props) = self.props {
            props.try_update(f);
        }
    }

    /// Whether the entry is open (not yet closing, not removed).
    pub fn is_open(&self) -> bool {
        self.ctx
            .and_then(|c| c.show.try_get_untracked())
            .unwrap_or(false)
    }

    /// The result, once settled.
    pub fn result(&self) -> Option<Result<T, Dismissed>> {
        match self.ctx {
            Some(ctx) => ctx.settlement.try_with_value(|s| s.result()).flatten(),
            None => Some(Err(Dismissed::Unavailable)),
        }
    }

    /// Makes [`Plugins::dismiss_all`] skip this entry (or not). Call it right after opening.
    pub fn set_persistent(&self, persistent: bool) {
        if let Some(ctx) = self.ctx {
            ctx.persistent.try_set_value(persistent);
        }
    }
}

/// The future of a [`PluginHandle`]: resolves when the entry settles.
pub struct PluginFuture<T>(FutureState<T>);

enum FutureState<T> {
    Ready(Option<Result<T, Dismissed>>),
    Waiting(oneshot::Receiver<Result<T, Dismissed>>),
}

impl<T> Unpin for PluginFuture<T> {}

impl<T> Future for PluginFuture<T> {
    type Output = Result<T, Dismissed>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match &mut self.0 {
            FutureState::Ready(r) => Poll::Ready(r.take().unwrap_or(Err(Dismissed::Unavailable))),
            FutureState::Waiting(rx) => match Pin::new(rx).poll(cx) {
                Poll::Ready(Ok(r)) => Poll::Ready(r),
                // The listener was dropped unrun: the host went away.
                Poll::Ready(Err(_)) => Poll::Ready(Err(Dismissed::Unavailable)),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

impl<T: Clone + 'static, P: 'static> std::future::IntoFuture for PluginHandle<T, P> {
    type Output = Result<T, Dismissed>;
    type IntoFuture = PluginFuture<T>;

    fn into_future(self) -> PluginFuture<T> {
        let Some(ctx) = self.ctx else {
            return PluginFuture(FutureState::Ready(Some(Err(Dismissed::Unavailable))));
        };
        let (tx, rx) = oneshot::channel();
        let flush = ctx.settlement.try_update_value(|s| {
            s.wait(move |r| {
                let _ = tx.send(r);
            })
        });
        match flush {
            Some(flush) => {
                flush.run();
                PluginFuture(FutureState::Waiting(rx))
            }
            None => PluginFuture(FutureState::Ready(Some(Err(Dismissed::Unavailable)))),
        }
    }
}

// ── Host component ─────────────────────────────────────────────────────────

/// Renders the opened plugin entries. Place it once, inside the `Router` when the views need
/// router contexts; it needs [`provide_plugins`] above it. Views are mounted through a `Portal`
/// at the app root and see the contexts visible here. Nothing renders on the server.
///
/// With the `router` feature, and [`DialogPlugin::setup`] asking for it
/// (`close_on_navigation`), it also dismisses the dialogs when the path changes.
#[component]
pub fn PluginHost() -> impl IntoView {
    let Some(plugins) = try_use_plugins() else {
        leptos::logging::warn!(
            "<PluginHost/>: no `provide_plugins()` above it, so it renders nothing."
        );
        return ().into_any();
    };
    let Some(host) = plugins.0 else {
        return ().into_any();
    };
    host.update_value(|h| h.hosts += 1);
    on_cleanup(move || {
        host.try_update_value(|h| h.hosts = h.hosts.saturating_sub(1));
    });
    let entries = host.with_value(|h| h.entries);

    // Navigation: with the `router` feature, a path change dismisses the dialogs if
    // `DialogPlugin::setup` asked for it. The browser `Router` provides `BrowserUrl`, which is
    // how we know there is a `Router` to ask (there is none on the server).
    #[cfg(feature = "router")]
    if use_context::<leptos_router::location::BrowserUrl>().is_some() {
        let path = leptos_router::hooks::use_location().pathname;
        Effect::new(move |previous: Option<String>| {
            let now = path.get();
            if previous.as_ref().is_some_and(|p| *p != now) && DialogPlugin::close_on_navigation() {
                plugins.dismiss_all();
            }
            now
        });
    }

    let rows = move || {
        view! {
            <For
                each=move || entries.get()
                key=|row| row.id
                children=move |row| {
                    // Row owner: the entry state goes away with the row, never inside `on_hide`.
                    on_cleanup(move || {
                        if let Some((abandon, dispose)) = row
                            .state
                            .try_with_value(|s| (s.abandon.clone(), s.dispose.clone()))
                        {
                            abandon();
                            dispose();
                        }
                        row.state.dispose();
                    });
                    let build = row.state.try_update_value(|s| s.view.take()).flatten();
                    build.map(|build| untrack(build))
                }
            />
        }
    };
    match crate::root::portal_mount() {
        Some(m) => view! { <Portal mount=m>{rows()}</Portal> }.into_any(),
        None => view! { <Portal>{rows()}</Portal> }.into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `open_entry` skips the browser check, so the host logic runs on the test thread. The
    /// entries never render (there is no `<PluginHost/>`), so closing one settles it at once.
    fn host() -> (Owner, Plugins) {
        let owner = Owner::new();
        let plugins = owner.with(provide_plugins);
        (owner, plugins)
    }

    fn open(plugins: &Plugins) -> PluginHandle<u32> {
        plugins
            .open_entry((), |_ctx: PluginCtx<u32>, _| ())
            .expect("host is available")
    }

    #[test]
    fn dismiss_all_closes_entries_except_persistent_ones() {
        let (owner, plugins) = host();
        owner.with(|| {
            let normal = open(&plugins);
            let pinned = open(&plugins);
            pinned.set_persistent(true);
            plugins.dismiss_all();
            assert_eq!(normal.result(), Some(Err(Dismissed::Programmatic)));
            assert!(!normal.is_open());
            assert_eq!(pinned.result(), None);
            assert!(pinned.is_open());
            // A persistent entry still closes when asked directly.
            pinned.hide();
            assert_eq!(pinned.result(), Some(Err(Dismissed::Programmatic)));
        });
    }

    #[test]
    fn first_close_wins_and_ok_beats_a_later_hide() {
        let (owner, plugins) = host();
        owner.with(|| {
            let a = open(&plugins);
            a.hide();
            a.hide();
            assert_eq!(a.result(), Some(Err(Dismissed::Programmatic)));
            let b = open(&plugins);
            let ctx = b.ctx.expect("ctx");
            ctx.ok(3);
            b.hide();
            assert_eq!(b.result(), Some(Ok(3)));
        });
    }

    #[test]
    fn an_unavailable_handle_is_inert_and_settles_unavailable() {
        let handle = PluginHandle::<u32> {
            ctx: None,
            props: None,
        };
        let seen = Rc::new(Cell::new(None));
        let s = seen.clone();
        handle.on_cancel(move |why| s.set(Some(why)));
        assert_eq!(seen.get(), Some(Dismissed::Unavailable));
        assert_eq!(handle.result(), Some(Err(Dismissed::Unavailable)));
        assert!(!handle.is_open());
        handle.hide();
    }

    #[test]
    fn get_or_init_shares_the_first_value() {
        let (owner, plugins) = host();
        owner.with(|| {
            let first = plugins.get_or_init(|| 7_u32);
            let second = plugins.get_or_init(|| 9_u32);
            assert_eq!((first, second), (7, 7));
        });
    }

    #[test]
    fn use_plugins_without_a_context_is_unavailable() {
        let owner = Owner::new();
        owner.with(|| {
            assert!(try_use_plugins().is_none());
            assert!(!Plugins(None).is_available());
        });
    }
}
