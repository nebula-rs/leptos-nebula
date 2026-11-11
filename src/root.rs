//! Where the app lives in the DOM: the whole `document` (CSR/SSR) or a shadow root (web component
//! mode, ADR-0018).
//!
//! Components never reach for `document` directly to look up elements, read focus or read an
//! event's target — they use the helpers here, which work the same with and without a shadow root:
//!
//! - [`event_origin`]: the real origin of an event (`composedPath()[0]`). A listener on `document`
//!   sees `e.target()` **retargeted to the shadow host** for events from inside a shadow root.
//! - [`active_element`]: the focused element, descending into (open) shadow roots —
//!   `document.activeElement` stops at the host.
//! - [`by_id_near`] / [`query_near`]: lookups in the root (document or shadow root) of a given node.
//!
//! [`NebulaRoot`] (context, [`provide_root`]) only says which element receives the theme
//! attributes and the portaled popups; without it, `<html>` and `<body>` as before.
//!
//! Everything here runs only in the browser (`Effect`s and handlers), never while rendering.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// The element the app was mounted in when it is not the whole document (e.g. the `.lx-root`
/// inside a shadow root). `Copy`; provided with [`provide_root`].
#[derive(Clone, Copy)]
pub struct NebulaRoot {
    element: StoredValue<web_sys::Element, LocalStorage>,
}

impl NebulaRoot {
    /// The element element.
    pub fn element(&self) -> web_sys::Element {
        self.element.get_value()
    }
}

/// Declares `element` as the app's root: it receives `data-theme`/`data-mode` and the popups
/// (`Menu`, `Tooltip`, `Select`, `Autocomplete`). Call it before [`provide_theme`](crate::provide_theme),
/// at the top of the app (the `wc` mount does it).
pub fn provide_root(element: web_sys::Element) -> NebulaRoot {
    let root = NebulaRoot {
        element: StoredValue::new_local(element),
    };
    provide_context(root);
    root
}

/// The root from the context, if one was provided.
pub fn use_root() -> Option<NebulaRoot> {
    use_context::<NebulaRoot>()
}

/// Element that receives `data-theme`/`data-mode`: the root element, or `<html>`.
pub(crate) fn theme_target(root: Option<NebulaRoot>) -> Option<web_sys::Element> {
    match root {
        Some(r) => Some(r.element()),
        None => document()?.document_element(),
    }
}

/// Mount point for portaled popups: the root element; `None` = the `Portal` default (`<body>`).
/// Call it while rendering (it reads the context); it does not touch the DOM.
pub(crate) fn portal_mount() -> Option<web_sys::Element> {
    use_root().map(|r| r.element())
}

pub(crate) fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|w| w.document())
}

/// Where the event really started: `composedPath()[0]`, falling back to `target()`.
pub(crate) fn event_origin(e: &web_sys::Event) -> Option<web_sys::EventTarget> {
    let first = e.composed_path().get(0);
    first
        .dyn_into::<web_sys::EventTarget>()
        .ok()
        .or_else(|| e.target())
}

/// [`event_origin`] as a `Node`.
pub(crate) fn origin_node(e: &web_sys::Event) -> Option<web_sys::Node> {
    event_origin(e).and_then(|t| t.dyn_into::<web_sys::Node>().ok())
}

/// [`event_origin`] as an `Element`.
pub(crate) fn origin_element(e: &web_sys::Event) -> Option<web_sys::Element> {
    event_origin(e).and_then(|t| t.dyn_into::<web_sys::Element>().ok())
}

/// The focused element, descending into open shadow roots.
pub(crate) fn active_element() -> Option<web_sys::Element> {
    let mut active = document()?.active_element()?;
    while let Some(inner) = active.shadow_root().and_then(|s| s.active_element()) {
        active = inner;
    }
    Some(active)
}

/// `getElementById` in the root (shadow root or document) that contains `near`.
pub(crate) fn by_id_near(near: &web_sys::Node, id: &str) -> Option<web_sys::Element> {
    match near.get_root_node().dyn_into::<web_sys::ShadowRoot>() {
        Ok(shadow) => shadow.get_element_by_id(id),
        Err(_) => document()?.get_element_by_id(id),
    }
}

/// `querySelector` in the root (shadow root or document) that contains `near`.
pub(crate) fn query_near(near: &web_sys::Node, selector: &str) -> Option<web_sys::Element> {
    match near.get_root_node().dyn_into::<web_sys::ShadowRoot>() {
        Ok(shadow) => shadow.query_selector(selector).ok().flatten(),
        Err(_) => document()?.query_selector(selector).ok().flatten(),
    }
}
