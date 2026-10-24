//! Base dos popups ancorados ([`Menu`](super::Menu), [`Tooltip`](super::Tooltip)).
//!
//! O popup é um `<div popover="manual">`: a **Popover API** põe o elemento na camada superior
//! (acima de qualquer `z-index` e fora do `overflow` dos ancestrais), e o motor puro de
//! `crate::position` calcula `top`/`left`.
//!
//! O popup é montado no `<body>` por um `Portal` (só no navegador; o servidor não o renderiza),
//! o equivalente do portal do Quasar. Dentro da âncora ele seria um problema: um `<button>` dentro
//! de um `<button>` (num `Menu` com botões) é reestruturado pelo analisador de HTML do servidor e
//! quebra a hidratação, e os filhos de um botão são "apresentacionais" para o leitor de tela. No
//! lugar do popup, a âncora recebe um `<span hidden>` (o *marcador*), cujo pai é a âncora.
//!
//! Tudo que toca o DOM roda só em `Effect`s e handlers, nunca durante a renderização (SSR).

use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

use leptos::leptos_dom::helpers::{TimeoutHandle, set_timeout_with_handle};
use leptos::{html, prelude::*};
use wasm_bindgen::{JsCast, closure::Closure};

use crate::position::{self, Input, Position, Rect, Size};
use crate::util::parse_duration_ms;

pub(crate) fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|w| w.document())
}

/// `focus({ preventScroll: true })`.
pub(crate) fn focus_quietly(el: &web_sys::HtmlElement) {
    let opts = web_sys::FocusOptions::new();
    opts.set_prevent_scroll(true);
    let _ = el.focus_with_options(&opts);
}

// ── Listener ───────────────────────────────────────────────────────────────

/// Opções de um [`Listener`].
#[derive(Clone, Copy, Default)]
pub(crate) struct ListenerOpts {
    pub capture: bool,
    pub passive: bool,
}

/// Ouvinte de evento que se remove ao ser descartado.
pub(crate) struct Listener {
    target: web_sys::EventTarget,
    name: &'static str,
    capture: bool,
    cb: Closure<dyn FnMut(web_sys::Event)>,
}

impl Listener {
    pub(crate) fn new(
        target: &web_sys::EventTarget,
        name: &'static str,
        opts: ListenerOpts,
        f: impl FnMut(web_sys::Event) + 'static,
    ) -> Self {
        let cb = Closure::<dyn FnMut(web_sys::Event)>::new(f);
        let o = web_sys::AddEventListenerOptions::new();
        o.set_capture(opts.capture);
        o.set_passive(opts.passive);
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            name,
            cb.as_ref().unchecked_ref(),
            &o,
        );
        Self {
            target: target.clone(),
            name,
            capture: opts.capture,
            cb,
        }
    }

    /// Ouvinte comum (fase de bolha, não passivo).
    pub(crate) fn on(
        target: &web_sys::EventTarget,
        name: &'static str,
        f: impl FnMut(web_sys::Event) + 'static,
    ) -> Self {
        Self::new(target, name, ListenerOpts::default(), f)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.name,
            self.cb.as_ref().unchecked_ref(),
            self.capture,
        );
    }
}

// ── Pilha de Esc ───────────────────────────────────────────────────────────

thread_local! {
    static ESC_STACK: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
}

fn next_overlay_id() -> u64 {
    NEXT_ID.with(|c| {
        let n = c.get();
        c.set(n + 1);
        n
    })
}

/// Empilha um popup aberto que reage ao `Esc` (só o do topo reage).
pub(crate) fn esc_push(id: u64) {
    ESC_STACK.with(|s| {
        let mut s = s.borrow_mut();
        s.retain(|x| *x != id);
        s.push(id);
    });
}

pub(crate) fn esc_remove(id: u64) {
    ESC_STACK.with(|s| s.borrow_mut().retain(|x| *x != id));
}

pub(crate) fn esc_is_top(id: u64) -> bool {
    ESC_STACK.with(|s| s.borrow().last() == Some(&id))
}

// ── Posicionamento no DOM ──────────────────────────────────────────────────

/// O que o motor precisa além do que se mede no DOM.
#[derive(Clone)]
pub(crate) struct Params {
    pub anchor: Position,
    pub self_: Position,
    pub offset: (f64, f64),
    pub fit: bool,
    pub cover: bool,
    /// `max-height`/`max-width` por prop (qualquer comprimento CSS).
    pub max_height: Option<String>,
    pub max_width: Option<String>,
}

/// `"123.5px"` → `123.5`. `none`, vazio e outras unidades → `None`.
fn parse_px(v: &str) -> Option<f64> {
    v.trim().strip_suffix("px")?.trim().parse().ok()
}

fn set_px(style: &web_sys::CssStyleDeclaration, prop: &str, v: f64) {
    let _ = style.set_property(prop, &format!("{v}px"));
}

/// Mede o popup e a âncora, pergunta ao motor e escreve `top`/`left` e os tetos.
///
/// Primeiro tira os tetos de um passe anterior (mede o tamanho natural, com os tetos do CSS e das
/// props valendo); se há `fit`/`cover`, aplica o tamanho mínimo e mede de novo, porque a largura
/// mínima pode mudar a altura.
pub(crate) fn place(popup: &web_sys::HtmlElement, anchor: &web_sys::Element, p: &Params) {
    let Some(win) = web_sys::window() else { return };
    let Some(root) = win.document().and_then(|d| d.document_element()) else {
        return;
    };
    let style = popup.style();
    for prop in ["max-height", "max-width", "min-width", "min-height"] {
        let _ = style.remove_property(prop);
    }
    if let Some(v) = &p.max_height {
        let _ = style.set_property("max-height", v);
    }
    if let Some(v) = &p.max_width {
        let _ = style.set_property("max-width", v);
    }

    let r = anchor.get_bounding_client_rect();
    let computed = win.get_computed_style(popup).ok().flatten();
    let read = |name: &str| {
        computed
            .as_ref()
            .and_then(|c| c.get_property_value(name).ok())
    };
    let input = |popup_size: Size| Input {
        anchor: Rect {
            top: r.top(),
            left: r.left(),
            width: r.width(),
            height: r.height(),
        },
        popup: popup_size,
        viewport: Size {
            width: f64::from(root.client_width()),
            height: f64::from(root.client_height()),
        },
        anchor_origin: p.anchor,
        self_origin: p.self_,
        offset: p.offset,
        fit: p.fit,
        cover: p.cover,
        rtl: read("direction").is_some_and(|d| d == "rtl"),
        bound_max_height: read("max-height").and_then(|v| parse_px(&v)),
        bound_max_width: read("max-width").and_then(|v| parse_px(&v)),
    };
    let measure = || Size {
        width: f64::from(popup.offset_width()),
        height: f64::from(popup.offset_height()),
    };

    let mut placement = position::compute(&input(measure()));
    if p.fit || p.cover {
        if let Some(w) = placement.min_width {
            set_px(&style, "min-width", w);
        }
        if let Some(h) = placement.min_height {
            set_px(&style, "min-height", h);
        }
        placement = position::compute(&input(measure()));
    }
    if let Some(v) = placement.max_height {
        set_px(&style, "max-height", v);
    }
    if let Some(v) = placement.max_width {
        set_px(&style, "max-width", v);
    }
    set_px(&style, "top", placement.top);
    set_px(&style, "left", placement.left);
}

/// Mantém o popup colado à âncora: reposiciona em `scroll` (de qualquer ancestral), `resize` da
/// janela e quando o próprio popup muda de tamanho (conteúdo que cresce).
pub(crate) struct Tracker {
    _listeners: Vec<Listener>,
    observer: Option<web_sys::ResizeObserver>,
    _cb: Closure<dyn FnMut()>,
}

impl Tracker {
    fn start(popup: &web_sys::HtmlElement, anchor: &web_sys::Element, params: &Params) -> Self {
        let run = {
            let (popup, anchor, params) = (popup.clone(), anchor.clone(), params.clone());
            move || {
                if popup.is_connected() && anchor.is_connected() {
                    place(&popup, &anchor, &params);
                }
            }
        };
        let mut listeners = Vec::new();
        if let Some(win) = web_sys::window() {
            let target: &web_sys::EventTarget = win.as_ref();
            let opts = ListenerOpts {
                capture: true,
                passive: true,
            };
            let on_scroll = {
                let (popup, run) = (popup.clone(), run.clone());
                move |e: web_sys::Event| {
                    // rolar o próprio popup não move a âncora
                    let inside = e
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                        .is_some_and(|n| popup.contains(Some(&n)));
                    if !inside {
                        run();
                    }
                }
            };
            listeners.push(Listener::new(target, "scroll", opts, on_scroll));
            let on_resize = {
                let run = run.clone();
                move |_| run()
            };
            listeners.push(Listener::new(
                target,
                "resize",
                ListenerOpts {
                    capture: false,
                    passive: true,
                },
                on_resize,
            ));
        }
        let cb = Closure::<dyn FnMut()>::new(run);
        let observer = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()).ok();
        if let Some(o) = &observer {
            o.observe(popup);
        }
        Self {
            _listeners: listeners,
            observer,
            _cb: cb,
        }
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        if let Some(o) = &self.observer {
            o.disconnect();
        }
    }
}

// ── Estado do popup ────────────────────────────────────────────────────────

/// Fase visual: só `Open` mostra o popup; `Leaving` espera a transição de saída antes de esconder
/// o elemento (`hidePopover()`). `Closed` = o popover não está aberto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Closed,
    Open,
    Leaving,
}

/// Elemento, âncora, relógios e rastreio de um popup. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
pub(crate) struct Overlay {
    /// O popup (existe só depois de o `Portal` montar, no navegador).
    pub(crate) el: NodeRef<html::Div>,
    /// O marcador `<span hidden>` que fica na posição do componente; seu pai é a âncora.
    pub(crate) marker: NodeRef<html::Span>,
    pub(crate) phase: RwSignal<Phase>,
    pub(crate) id: u64,
    anchor: StoredValue<Option<web_sys::Element>, LocalStorage>,
    timer: StoredValue<Option<TimeoutHandle>, LocalStorage>,
    tracker: StoredValue<Option<Tracker>, LocalStorage>,
}

impl Overlay {
    pub(crate) fn new() -> Self {
        Self {
            el: NodeRef::new(),
            marker: NodeRef::new(),
            phase: RwSignal::new(Phase::Closed),
            id: next_overlay_id(),
            anchor: StoredValue::new_local(None),
            timer: StoredValue::new_local(None),
            tracker: StoredValue::new_local(None),
        }
    }

    pub(crate) fn popup(&self) -> Option<web_sys::HtmlElement> {
        self.el
            .get_untracked()
            .map(|e| e.unchecked_into::<web_sys::HtmlElement>())
    }

    pub(crate) fn anchor(&self) -> Option<web_sys::Element> {
        self.anchor.get_value()
    }

    /// Acha a âncora: o pai do marcador, ou o primeiro elemento que casa com o seletor `target`.
    pub(crate) fn attach(&self, target: Option<&str>) -> Option<web_sys::Element> {
        let found = match target {
            Some(sel) => document()?.query_selector(sel).ok().flatten(),
            None => self.marker.get_untracked()?.parent_element(),
        };
        if let Some(a) = &found {
            self.anchor.set_value(Some(a.clone()));
        }
        found
    }

    /// Âncora para abrir: a guardada, ou (com `target`) uma nova busca pelo seletor.
    pub(crate) fn anchor_for_open(&self, target: Option<&str>) -> Option<web_sys::Element> {
        if let Some(a) = self.anchor().filter(|a| a.is_connected()) {
            return Some(a);
        }
        let found = document()?.query_selector(target?).ok().flatten()?;
        self.anchor.set_value(Some(found.clone()));
        Some(found)
    }

    pub(crate) fn clear_timer(&self) {
        if let Some(h) = self.timer.try_update_value(Option::take).flatten() {
            h.clear();
        }
    }

    pub(crate) fn after(&self, ms: u32, f: impl FnOnce() + 'static) {
        self.clear_timer();
        let this = *self;
        let handle = set_timeout_with_handle(
            move || {
                this.timer.try_set_value(None);
                f();
            },
            Duration::from_millis(u64::from(ms)),
        )
        .ok();
        self.timer.try_set_value(handle);
    }

    /// Duração da transição do popup, lida do estilo computado (tokens `--lx-*-duration`).
    pub(crate) fn transition_ms(&self) -> u32 {
        let Some(popup) = self.popup() else { return 0 };
        web_sys::window()
            .and_then(|w| w.get_computed_style(&popup).ok().flatten())
            .and_then(|s| s.get_property_value("transition-duration").ok())
            .map(|v| parse_duration_ms(&v))
            .unwrap_or(0)
    }

    /// Âncora dentro de um `<dialog>` modal: o resto da página (o `<body>` inclusive) fica inerte,
    /// e um popover montado fora do `<dialog>` não receberia o ponteiro nem o foco. O popup passa a
    /// ser filho do `<dialog>`; continua na camada superior, e o `position: fixed` do motor segue
    /// valendo (a camada superior ignora o `transform` dos ancestrais).
    fn nest_in_dialog(popup: &web_sys::HtmlElement, anchor: &web_sys::Element) {
        let Ok(Some(dialog)) = anchor.closest("dialog") else {
            return;
        };
        if !popup
            .parent_element()
            .is_some_and(|p| p.is_same_node(Some(&dialog)))
        {
            let _ = dialog.append_child(popup);
        }
    }

    /// Mostra o popup na camada superior, posiciona, passa para `Open` (inicia a transição) e,
    /// ao fim dela, reposiciona e chama `on_shown`. `false` se não há como abrir (sem âncora ou
    /// sem Popover API).
    pub(crate) fn open(&self, params: &Params, on_shown: impl FnOnce() + 'static) -> bool {
        let (Some(popup), Some(anchor)) = (self.popup(), self.anchor()) else {
            return false;
        };
        self.clear_timer();
        Self::nest_in_dialog(&popup, &anchor);
        let already = popup.matches(":popover-open").unwrap_or(false);
        if !already && popup.show_popover().is_err() {
            return false;
        }
        place(&popup, &anchor, params);
        self.tracker
            .try_set_value(Some(Tracker::start(&popup, &anchor, params)));
        // fixa o estado inicial da transição antes de trocar para o aberto
        let _ = popup.offset_width();
        self.phase.set(Phase::Open);
        let ms = self.transition_ms();
        let (this, params) = (*self, params.clone());
        self.after(ms, move || {
            if let (Some(p), Some(a)) = (this.popup(), this.anchor())
                && p.is_connected()
            {
                place(&p, &a, &params);
            }
            on_shown();
        });
        true
    }

    /// Começa a sair: a transição roda e, depois dela, o popover é escondido e `on_hidden` chama.
    pub(crate) fn leave(&self, on_hidden: impl FnOnce() + 'static) {
        let ms = self.transition_ms();
        self.phase.set(Phase::Leaving);
        let this = *self;
        self.after(ms, move || {
            this.finish();
            on_hidden();
        });
    }

    /// Esconde já (sem transição).
    pub(crate) fn finish(&self) {
        self.tracker.try_set_value(None);
        if let Some(p) = self.popup() {
            let _ = p.hide_popover();
        }
        self.phase.set(Phase::Closed);
    }

    /// Cancela uma saída em curso (o `show` voltou a `true` durante a transição).
    pub(crate) fn revive(&self) {
        self.clear_timer();
        self.phase.set(Phase::Open);
    }

    /// Desmontagem: solta relógios e ouvintes (o `Portal` tira o popup do `<body>`).
    pub(crate) fn cleanup(&self) {
        self.clear_timer();
        self.tracker.try_set_value(None);
        esc_remove(self.id);
        if let Some(p) = self.popup() {
            let _ = p.hide_popover();
            // o `Portal` tira o contêiner do `<body>`; o popup pode ter ido para um `<dialog>`
            p.remove();
        }
    }
}

// ── ARIA da âncora ─────────────────────────────────────────────────────────

/// `aria-expanded` só é válido em alguns papéis: uma âncora que é um `<div>` solto fica de fora
/// (mesma regra do `useAnchor` do Quasar). `role` é o atributo `role`, se houver.
fn expandable(role: Option<&str>, tag: &str, has_href: bool, input_type: Option<&str>) -> bool {
    const ROLES: [&str; 17] = [
        "application",
        "button",
        "checkbox",
        "columnheader",
        "combobox",
        "gridcell",
        "link",
        "listbox",
        "menuitem",
        "menuitemcheckbox",
        "menuitemradio",
        "row",
        "rowheader",
        "switch",
        "tab",
        "treeitem",
        "textbox",
    ];
    if let Some(role) = role.filter(|r| !r.trim().is_empty()) {
        return role
            .split_whitespace()
            .next()
            .is_some_and(|r| ROLES.contains(&r));
    }
    match tag.to_ascii_uppercase().as_str() {
        "BUTTON" => true,
        "A" => has_href,
        "INPUT" => matches!(input_type, Some("button" | "image" | "reset" | "submit")),
        _ => false,
    }
}

pub(crate) fn is_expandable_control(el: &web_sys::Element) -> bool {
    expandable(
        el.get_attribute("role").as_deref(),
        &el.tag_name(),
        el.has_attribute("href"),
        el.get_attribute("type").as_deref(),
    )
}

/// Acrescenta `id` à lista de ids de `current`. `None` se já constava (então não é nosso e não
/// deve ser removido depois).
fn with_id_ref(current: &str, id: &str) -> Option<String> {
    if current.split_whitespace().any(|t| t == id) {
        return None;
    }
    Some(if current.trim().is_empty() {
        id.to_string()
    } else {
        format!("{} {id}", current.trim())
    })
}

/// Tira `id` da lista; `None` quando sobra a lista vazia (o atributo sai).
fn without_id_ref(current: &str, id: &str) -> Option<String> {
    let rest: Vec<&str> = current.split_whitespace().filter(|t| *t != id).collect();
    (!rest.is_empty()).then(|| rest.join(" "))
}

/// Acrescenta um `id` ao atributo de referências (`aria-describedby`). `true` se foi acrescentado.
pub(crate) fn add_id_ref(el: &web_sys::Element, attr: &str, id: &str) -> bool {
    match with_id_ref(&el.get_attribute(attr).unwrap_or_default(), id) {
        Some(next) => {
            let _ = el.set_attribute(attr, &next);
            true
        }
        None => false,
    }
}

pub(crate) fn remove_id_ref(el: &web_sys::Element, attr: &str, id: &str) {
    match without_id_ref(&el.get_attribute(attr).unwrap_or_default(), id) {
        Some(next) => {
            let _ = el.set_attribute(attr, &next);
        }
        None => {
            let _ = el.remove_attribute(attr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_values_only() {
        assert_eq!(parse_px("520px"), Some(520.0));
        assert_eq!(parse_px(" 12.5px "), Some(12.5));
        assert_eq!(parse_px("none"), None);
        assert_eq!(parse_px("50%"), None);
        assert_eq!(parse_px(""), None);
    }

    #[test]
    fn only_controls_are_expandable() {
        assert!(expandable(None, "BUTTON", false, None));
        assert!(expandable(None, "button", false, None));
        assert!(expandable(None, "A", true, None));
        assert!(!expandable(None, "A", false, None));
        assert!(expandable(None, "INPUT", false, Some("submit")));
        assert!(!expandable(None, "INPUT", false, Some("text")));
        assert!(!expandable(None, "DIV", false, None));
        // o papel manda: um div com `role=button` expande; um botão com papel estranho, não
        assert!(expandable(Some("button"), "DIV", false, None));
        assert!(!expandable(Some("presentation"), "BUTTON", false, None));
        assert!(expandable(Some("tab extra"), "DIV", false, None));
    }

    #[test]
    fn id_refs_are_added_once_and_removed_cleanly() {
        assert_eq!(with_id_ref("", "t"), Some("t".to_string()));
        assert_eq!(with_id_ref("a b", "t"), Some("a b t".to_string()));
        assert_eq!(with_id_ref("a t", "t"), None);
        assert_eq!(without_id_ref("a t b", "t"), Some("a b".to_string()));
        assert_eq!(without_id_ref("t", "t"), None);
        assert_eq!(without_id_ref("", "t"), None);
    }

    #[test]
    fn only_the_top_popup_answers_to_escape() {
        esc_push(1001);
        esc_push(1002);
        assert!(esc_is_top(1002));
        assert!(!esc_is_top(1001));
        esc_remove(1002);
        assert!(esc_is_top(1001));
        esc_remove(1001);
        assert!(!esc_is_top(1001));
    }
}
