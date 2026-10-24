//! `Menu`: popup ancorado que abre por clique na âncora (o elemento pai, ou o `target`).
//!
//! A camada superior vem da Popover API (`popover="manual"`) e a posição, do motor puro de
//! `crate::position`; ver "Sobreposições" em `docs/ui/overview.md` e `popup.rs`.
//!
//! A lógica sem DOM (navegação por teclado, quando o `Tab` sai do menu, quem abre sozinho com
//! `Enter`/`Espaço`) fica em funções puras testadas no host; [`Ctl`] a liga ao elemento, aos
//! sinais e aos relógios — só em handlers e `Effect`s, nunca durante a renderização (SSR).

use std::time::Duration;

use leptos::leptos_dom::helpers::set_timeout;
use leptos::{ev, portal::Portal, prelude::*};
use wasm_bindgen::JsCast;

use super::popup::{
    Listener, ListenerOpts, Overlay, Params, Phase, document, esc_is_top, esc_push, esc_remove,
    focus_quietly, is_expandable_control,
};
use crate::position::{Horizontal, Position, Vertical};
use crate::util::cls;

/// Contexto que o `Menu` fornece ao conteúdo: uma [`List`](super::List) dentro dele vira o
/// papel `menu` (e os `Item` clicáveis, `menuitem`).
#[derive(Clone, Copy)]
pub(crate) struct MenuCtx {
    pub(crate) is_menu: RwSignal<bool>,
}

// ── Lógica pura ────────────────────────────────────────────────────────────

/// Tecla de navegação entre os itens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavKey {
    Next,
    Prev,
    First,
    Last,
}

fn nav_key(key: &str) -> Option<NavKey> {
    match key {
        "ArrowDown" => Some(NavKey::Next),
        "ArrowUp" => Some(NavKey::Prev),
        "Home" => Some(NavKey::First),
        "End" => Some(NavKey::Last),
        _ => None,
    }
}

/// Próximo item a focar (WAI-ARIA, padrão "menu"): setas dão a volta; sem item focado,
/// `Next`/`First` vão ao primeiro e `Prev`/`Last` ao último.
fn next_item(current: Option<usize>, len: usize, key: NavKey) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match (key, current) {
        (NavKey::First, _) | (NavKey::Next, None) => 0,
        (NavKey::Last, _) | (NavKey::Prev, None) => len - 1,
        (NavKey::Next, Some(i)) => (i + 1) % len,
        (NavKey::Prev, Some(i)) => (i + len - 1) % len,
    })
}

/// O `Tab` vai tirar o foco do menu? Então o menu fecha e o foco volta à âncora antes do `Tab`
/// agir (o `Tab` padrão sairia da página, porque o menu fica fora da ordem do documento).
/// `tabbables` = quantos focáveis por `Tab` há dentro; `active` = índice do que tem foco entre
/// eles; `on_popup` = o foco está no próprio popup.
fn tab_leaves(tabbables: usize, active: Option<usize>, on_popup: bool, shift: bool) -> bool {
    if tabbables == 0 {
        return true;
    }
    let edge = if shift { 0 } else { tabbables - 1 };
    active == Some(edge) || (shift && on_popup)
}

/// Elementos que o `Enter`/`Espaço` não ativam sozinhos precisam do nosso tratamento.
fn activates_natively(tag: &str, has_href: bool) -> bool {
    match tag.to_ascii_uppercase().as_str() {
        "BUTTON" | "SUMMARY" | "INPUT" | "SELECT" | "TEXTAREA" => true,
        "A" => has_href,
        _ => false,
    }
}

const ITEM: &str = "[role=menuitem]";
const TABBABLE: &str = "a[href], button:not([disabled]), input:not([disabled]), \
    select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex^=\"-\"])";

const BOTTOM_START: Position = Position::new(Vertical::Bottom, Horizontal::Start);
const TOP_START: Position = Position::new(Vertical::Top, Horizontal::Start);
const CENTER_MIDDLE: Position = Position::new(Vertical::Center, Horizontal::Middle);

// ── Ligação ao DOM ─────────────────────────────────────────────────────────

/// ARIA que escrevemos na âncora (só o que o app não declarou), para desfazer ao desmontar.
#[derive(Default)]
struct Aria {
    el: Option<web_sys::Element>,
    expanded: bool,
    haspopup: bool,
}

/// Estado e ações do menu. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
struct Ctl {
    o: Overlay,
    show: RwSignal<bool>,
    params: StoredValue<Params>,
    target: StoredValue<Option<String>>,
    persistent: bool,
    auto_close: bool,
    no_parent_event: bool,
    is_menu: RwSignal<bool>,
    on_show: Option<Callback<()>>,
    on_hide: Option<Callback<()>>,
    mounted: StoredValue<bool>,
    wired: StoredValue<bool>,
    /// Elemento com foco quando o menu abriu (o gatilho).
    trigger: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
    anchor_listeners: StoredValue<Vec<Listener>, LocalStorage>,
    dismiss: StoredValue<Vec<Listener>, LocalStorage>,
    aria: StoredValue<Aria, LocalStorage>,
    /// O fechamento veio de um clique fora: o foco só volta se o clique não o levou a outro lugar.
    defer_restore: StoredValue<bool>,
}

impl Ctl {
    fn mount(&self) {
        if self.mounted.get_value() || self.o.marker.get_untracked().is_none() {
            return;
        }
        self.mounted.set_value(true);
        let target = self.target.get_value();
        if let Some(anchor) = self.o.attach(target.as_deref()) {
            self.wire(&anchor);
        }
    }

    /// Liga a âncora: clique alterna; `Enter`/`Espaço` também, se o elemento não os trata sozinho.
    fn wire(&self, anchor: &web_sys::Element) {
        if self.wired.get_value() {
            return;
        }
        self.wired.set_value(true);
        self.configure_aria(anchor);
        if self.no_parent_event {
            return;
        }
        let show = self.show;
        let t: &web_sys::EventTarget = anchor.as_ref();
        let mut ls = vec![Listener::on(t, "click", move |_| {
            show.update(|v| *v = !*v);
        })];
        let el = anchor.clone();
        ls.push(Listener::on(t, "keydown", move |e| {
            let Ok(e) = e.dyn_into::<web_sys::KeyboardEvent>() else {
                return;
            };
            let on_anchor = e
                .target()
                .is_some_and(|t| t.dyn_ref::<web_sys::Element>().is_some_and(|x| x == &el));
            if on_anchor
                && matches!(e.key().as_str(), "Enter" | " ")
                && !activates_natively(&el.tag_name(), el.has_attribute("href"))
            {
                e.prevent_default();
                show.update(|v| *v = !*v);
            }
        }));
        self.anchor_listeners.set_value(ls);
    }

    fn configure_aria(&self, anchor: &web_sys::Element) {
        if !is_expandable_control(anchor) {
            return;
        }
        self.aria.set_value(Aria {
            el: Some(anchor.clone()),
            expanded: !anchor.has_attribute("aria-expanded"),
            haspopup: !anchor.has_attribute("aria-haspopup"),
        });
    }

    /// `aria-expanded` e `aria-haspopup` da âncora acompanham `show` e o conteúdo.
    fn update_aria(&self) {
        let open = self.show.get();
        let menu = self.is_menu.get();
        self.aria.with_value(|a| {
            let Some(el) = &a.el else { return };
            if a.expanded {
                let _ = el.set_attribute("aria-expanded", if open { "true" } else { "false" });
            }
            if a.haspopup {
                if menu {
                    let _ = el.set_attribute("aria-haspopup", "menu");
                } else {
                    let _ = el.remove_attribute("aria-haspopup");
                }
            }
        });
    }

    fn sync(&self) {
        // `get` (e não `get_untracked`): roda de novo quando o `Portal` montar o popup
        if self.o.el.get().is_none() {
            return;
        }
        match (self.show.get(), self.o.phase.get_untracked()) {
            (true, Phase::Closed) => self.open(),
            (true, Phase::Leaving) => {
                self.o.revive();
                self.arm_dismiss();
            }
            (false, Phase::Open) => self.leave(),
            _ => {}
        }
    }

    fn open(&self) {
        let target = self.target.get_value();
        let Some(anchor) = self.o.anchor_for_open(target.as_deref()) else {
            leptos::logging::warn!("Menu: âncora não encontrada");
            self.show.set(false);
            return;
        };
        self.wire(&anchor);
        let active = document()
            .and_then(|d| d.active_element())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
        self.trigger.set_value(active);
        let on_show = self.on_show;
        let shown = self.o.open(&self.params.get_value(), move || {
            if let Some(cb) = on_show {
                cb.run(());
            }
        });
        if !shown {
            leptos::logging::warn!("Menu: o navegador não suporta a Popover API");
            self.show.set(false);
            return;
        }
        self.arm_dismiss();
        self.focus_in();
    }

    fn leave(&self) {
        self.disarm_dismiss();
        if self.defer_restore.get_value() {
            self.defer_restore.set_value(false);
            let this = *self;
            // o clique que fechou ainda vai mover o foco (para o que foi clicado, ou para o
            // `<body>`): confere depois
            set_timeout(move || this.restore_focus(), Duration::ZERO);
        } else {
            self.restore_focus();
        }
        let on_hide = self.on_hide;
        self.o.leave(move || {
            if let Some(cb) = on_hide {
                cb.run(());
            }
        });
    }

    /// Foco para dentro: o primeiro item (menu), o `[autofocus]` ou o próprio popup.
    fn focus_in(&self) {
        let Some(popup) = self.o.popup() else { return };
        let pick = |sel: &str| {
            popup
                .query_selector(sel)
                .ok()
                .flatten()
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        };
        let el = pick(ITEM).or_else(|| pick("[autofocus]")).unwrap_or(popup);
        focus_quietly(&el);
    }

    /// Devolve o foco ao gatilho (ou à âncora), mas só se ele ainda está no menu ou em lugar
    /// nenhum: quem moveu o foco para outro controle não é interrompido.
    fn restore_focus(&self) {
        let trigger = self.trigger.try_update_value(Option::take).flatten();
        let Some(doc) = document() else { return };
        let popup = self.o.popup();
        let in_limbo = match doc.active_element() {
            None => true,
            Some(a) => {
                doc.body().is_some_and(|b| a == *b) || popup.is_some_and(|p| p.contains(Some(&a)))
            }
        };
        if !in_limbo {
            return;
        }
        let anchor = self
            .o
            .anchor()
            .and_then(|a| a.dyn_into::<web_sys::HtmlElement>().ok());
        let body_is_trigger = trigger
            .as_ref()
            .is_some_and(|t| doc.body().is_some_and(|b| *t == b));
        let target = trigger
            .filter(|t| t.is_connected() && !body_is_trigger)
            .or(anchor);
        if let Some(t) = target {
            focus_quietly(&t);
        }
    }

    fn arm_dismiss(&self) {
        self.disarm_dismiss();
        if self.persistent {
            return;
        }
        let Some(doc) = document() else { return };
        let t: &web_sys::EventTarget = doc.as_ref();
        let this = *self;
        let id = self.o.id;
        esc_push(id);
        let esc = Listener::on(t, "keydown", move |e| {
            let Ok(e) = e.dyn_into::<web_sys::KeyboardEvent>() else {
                return;
            };
            if e.key() == "Escape" && !e.default_prevented() && esc_is_top(id) {
                // dentro de um `Dialog`, o mesmo `Esc` não pode fechar o diálogo também
                e.prevent_default();
                this.show.set(false);
            }
        });
        let outside = Listener::new(
            t,
            "pointerdown",
            ListenerOpts {
                capture: true,
                passive: true,
            },
            move |e| this.on_outside(&e),
        );
        self.dismiss.set_value(vec![esc, outside]);
    }

    fn disarm_dismiss(&self) {
        self.dismiss.set_value(Vec::new());
        esc_remove(self.o.id);
    }

    /// `pointerdown` na página: fora do menu e da âncora, fecha.
    fn on_outside(&self, e: &web_sys::Event) {
        let Some(node) = e.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()) else {
            return;
        };
        let in_popup = self.o.popup().is_some_and(|p| p.contains(Some(&node)));
        let in_anchor = self.o.anchor().is_some_and(|a| a.contains(Some(&node)));
        if !in_popup && !in_anchor && self.o.phase.get_untracked() == Phase::Open {
            self.defer_restore.set_value(true);
            self.show.set(false);
        }
    }

    fn items(&self) -> Vec<web_sys::HtmlElement> {
        let Some(popup) = self.o.popup() else {
            return Vec::new();
        };
        let Ok(list) = popup.query_selector_all(ITEM) else {
            return Vec::new();
        };
        (0..list.length())
            .filter_map(|i| list.item(i))
            .filter_map(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
            .collect()
    }

    fn on_keydown(&self, e: ev::KeyboardEvent) {
        let key = e.key();
        if key == "Tab" {
            self.on_tab(&e);
        } else if let Some(nav) = nav_key(&key) {
            self.navigate(&e, nav);
        } else if key == "Enter" || key == " " {
            self.activate(&e, &key);
        }
    }

    fn navigate(&self, e: &ev::KeyboardEvent, nav: NavKey) {
        let items = self.items();
        if items.is_empty() {
            return;
        }
        let active = document().and_then(|d| d.active_element());
        let popup = self.o.popup();
        let current = active
            .as_ref()
            .and_then(|a| items.iter().position(|i| i.is_same_node(Some(a))));
        let on_popup = matches!((&active, &popup), (Some(a), Some(p)) if a == &**p);
        // setas só andam entre os itens (não roubam a seta de um campo dentro do menu)
        if current.is_none() && !on_popup {
            return;
        }
        if let Some(i) = next_item(current, items.len(), nav) {
            e.prevent_default();
            focus_quietly(&items[i]);
        }
    }

    /// `Enter`/`Espaço` num item que não é `<button>`/`<a>`: clica nele.
    fn activate(&self, e: &ev::KeyboardEvent, key: &str) {
        let Some(el) = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return;
        };
        if el.get_attribute("role").as_deref() != Some("menuitem") {
            return;
        }
        // `<a href>` já trata o Enter; o Espaço não
        if key == "Enter" && activates_natively(&el.tag_name(), el.has_attribute("href")) {
            return;
        }
        e.prevent_default();
        el.click();
    }

    fn on_tab(&self, e: &ev::KeyboardEvent) {
        if e.default_prevented() || self.persistent {
            return;
        }
        let Some(popup) = self.o.popup() else { return };
        let Ok(list) = popup.query_selector_all(TABBABLE) else {
            return;
        };
        let tabbables: Vec<web_sys::Node> =
            (0..list.length()).filter_map(|i| list.item(i)).collect();
        let active = document().and_then(|d| d.active_element());
        let idx = active
            .as_ref()
            .and_then(|a| tabbables.iter().position(|n| n.is_same_node(Some(a))));
        let on_popup = active.as_ref().is_some_and(|a| a == &*popup);
        if tab_leaves(tabbables.len(), idx, on_popup, e.shift_key()) {
            // devolve o foco à âncora já, antes do `Tab` padrão agir
            if let Some(a) = self
                .o
                .anchor()
                .and_then(|a| a.dyn_into::<web_sys::HtmlElement>().ok())
            {
                focus_quietly(&a);
            }
            self.show.set(false);
        }
    }

    fn cleanup(&self) {
        self.disarm_dismiss();
        self.anchor_listeners.set_value(Vec::new());
        self.aria.with_value(|a| {
            if let Some(el) = &a.el {
                if a.expanded {
                    let _ = el.remove_attribute("aria-expanded");
                }
                if a.haspopup {
                    let _ = el.remove_attribute("aria-haspopup");
                }
            }
        });
        self.o.cleanup();
    }
}

/// Menu ancorado. ≈ `QMenu`. Vai dentro da âncora (o elemento pai, como no Quasar) ou aponta
/// para ela com `target` (um seletor CSS): clicar na âncora abre e fecha. Quem usa `Btn` põe o
/// `Menu` dentro dele: `<Btn label="Abrir"><Menu show=show>…</Menu></Btn>`.
///
/// `show` é o `v-model`. Fecha com `Esc` e com clique fora (salvo `persistent`) e devolve o foco ao
/// que o tinha ao abrir. Ao abrir, o foco vai para o primeiro item (se o conteúdo é uma
/// [`List`](super::List)), para o elemento com `autofocus`, ou para o próprio menu. Reposiciona em
/// `scroll` e `resize`.
///
/// **Posição:** o menu nasce no ponto `anchor` da âncora (padrão `bottom start`) e encosta nele
/// com o ponto `self_` do popup (padrão `top start`); perto da borda da janela ele **vira** para o
/// lado com mais espaço e ganha `max-height`/`max-width` que cabem ([`crate::position`]).
/// `offset` (`(x, y)` em pixels) expande a caixa da âncora para fora; `fit` dá ao menu pelo menos
/// a largura da âncora; `cover` o põe sobre a âncora (ignora `self_` e `offset`, e o padrão de
/// `anchor` vira `center middle`).
///
/// **Acessibilidade:** se o conteúdo tem uma [`List`](super::List), o menu ganha `role="menu"` e
/// a lista e os itens clicáveis viram `menuitem`s, navegáveis com `↑`/`↓`/`Home`/`End`, ativados
/// com `Enter`/`Espaço`; a âncora recebe `aria-haspopup="menu"` e `aria-expanded`. Conteúdo
/// livre não ganha papel. `Tab` no fim do menu fecha-o e segue a partir da âncora. Dê um nome ao
/// menu com `aria_label` ou `aria_labelledby`.
///
/// **Renderização:** o popup vive no `<body>` (um `Portal`, só no navegador); o servidor só
/// renderiza um `<span hidden>` na posição do `Menu`, cujo pai é a âncora. Por isso o conteúdo pode
/// ter botões, links e formulários mesmo que a âncora seja um `<button>`.
///
/// Ainda não: `context_menu`, `touch_position`, `hover`, `scroll_target`, `no_esc_dismiss`,
/// `no_focus`, `no_refocus`, `no_route_dismiss`, `separate_close_popup` (menus aninhados),
/// `square`, `transition_*`.
#[component]
pub fn Menu(
    /// Estado aberto/fechado (`v-model` do Quasar).
    show: RwSignal<bool>,
    /// Seletor CSS da âncora; sem ele, a âncora é o elemento pai.
    #[prop(optional, into)]
    target: Option<String>,
    /// Ponto da âncora a que o menu se prende (padrão `bottom start`; `center middle` com `cover`).
    #[prop(optional)]
    anchor: Option<Position>,
    /// Ponto do menu que encosta no da âncora (padrão `top start`).
    #[prop(optional)]
    self_: Option<Position>,
    /// `(x, y)` em pixels: expande a caixa da âncora para fora.
    #[prop(optional)]
    offset: Option<(f64, f64)>,
    /// Pelo menos a largura da âncora.
    #[prop(optional)]
    fit: bool,
    /// Cobre a âncora.
    #[prop(optional)]
    cover: bool,
    /// Qualquer clique dentro do menu o fecha.
    #[prop(optional)]
    auto_close: bool,
    /// Não fecha com `Esc`, clique fora nem `Tab` para fora.
    #[prop(optional)]
    persistent: bool,
    /// A âncora não abre o menu ao clicar (só o `show` abre).
    #[prop(optional)]
    no_parent_event: bool,
    /// Qualquer comprimento CSS (ex.: `"240px"`).
    #[prop(optional, into)]
    max_height: Option<String>,
    /// Qualquer comprimento CSS (ex.: `"320px"`).
    #[prop(optional, into)]
    max_width: Option<String>,
    /// Nome acessível do menu (use com um menu de `List`).
    #[prop(optional, into)]
    aria_label: Option<String>,
    /// `id` do elemento que dá nome ao menu.
    #[prop(optional, into)]
    aria_labelledby: Option<String>,
    /// Depois da transição de entrada.
    #[prop(optional, into)]
    on_show: Option<Callback<()>>,
    /// Depois da transição de saída (o popup já saiu da camada superior).
    #[prop(optional, into)]
    on_hide: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let o = Overlay::new();
    let is_menu = RwSignal::new(false);
    let params = Params {
        anchor: anchor.unwrap_or(if cover { CENTER_MIDDLE } else { BOTTOM_START }),
        self_: self_.unwrap_or(TOP_START),
        offset: offset.unwrap_or((0.0, 0.0)),
        fit,
        cover,
        max_height,
        max_width,
    };
    let ctl = Ctl {
        o,
        show,
        params: StoredValue::new(params),
        target: StoredValue::new(target),
        persistent,
        auto_close,
        no_parent_event,
        is_menu,
        on_show,
        on_hide,
        mounted: StoredValue::new(false),
        wired: StoredValue::new(false),
        trigger: StoredValue::new_local(None),
        anchor_listeners: StoredValue::new_local(Vec::new()),
        dismiss: StoredValue::new_local(Vec::new()),
        aria: StoredValue::new_local(Aria::default()),
        defer_restore: StoredValue::new(false),
    };

    Effect::new(move |_| {
        if o.marker.get().is_some() {
            ctl.mount();
        }
    });
    Effect::new(move |_| ctl.sync());
    Effect::new(move |_| ctl.update_aria());
    on_cleanup(move || ctl.cleanup());

    // `children` é `FnOnce`, mas o `Portal` pede uma função que pode ser chamada de novo: o
    // conteúdo sai do `slot` na primeira chamada (a única que acontece).
    let slot = StoredValue::new_local(Some(children));
    let phase = o.phase;
    let popup = move || {
        let class = move || {
            cls([
                "lx-menu",
                if phase.get() == Phase::Open {
                    "lx-menu-open"
                } else {
                    "lx-menu-enter"
                },
            ])
        };
        // Aqui (e não no corpo do componente): vários `Menu`s dividem o dono do componente, e o
        // contexto de um sobrescreveria o do outro; o dono do `Portal` é só deste menu.
        provide_context(MenuCtx { is_menu });
        let content = slot.try_update_value(Option::take).flatten();
        view! {
            <div
                node_ref=o.el
                popover="manual"
                tabindex="-1"
                class=class
                role=move || is_menu.get().then_some("menu")
                aria-label=aria_label.clone()
                aria-labelledby=aria_labelledby.clone()
                on:click=move |_| {
                    if ctl.auto_close {
                        ctl.show.set(false);
                    }
                }
                on:keydown=move |e| ctl.on_keydown(e)
            >
                {content.map(|c| c())}
            </div>
        }
    };

    view! {
        <span hidden=true node_ref=o.marker></span>
        <Portal>{popup()}</Portal>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_wrap_around() {
        assert_eq!(next_item(Some(0), 3, NavKey::Next), Some(1));
        assert_eq!(next_item(Some(2), 3, NavKey::Next), Some(0));
        assert_eq!(next_item(Some(0), 3, NavKey::Prev), Some(2));
        assert_eq!(next_item(Some(2), 3, NavKey::Prev), Some(1));
    }

    #[test]
    fn home_and_end_jump() {
        assert_eq!(next_item(Some(1), 4, NavKey::First), Some(0));
        assert_eq!(next_item(Some(1), 4, NavKey::Last), Some(3));
    }

    #[test]
    fn without_a_focused_item_the_arrows_enter_from_the_ends() {
        assert_eq!(next_item(None, 3, NavKey::Next), Some(0));
        assert_eq!(next_item(None, 3, NavKey::Prev), Some(2));
        assert_eq!(next_item(None, 3, NavKey::First), Some(0));
        assert_eq!(next_item(None, 3, NavKey::Last), Some(2));
    }

    #[test]
    fn no_items_no_navigation() {
        assert_eq!(next_item(None, 0, NavKey::Next), None);
        assert_eq!(next_item(Some(0), 0, NavKey::Last), None);
    }

    #[test]
    fn maps_navigation_keys() {
        assert_eq!(nav_key("ArrowDown"), Some(NavKey::Next));
        assert_eq!(nav_key("ArrowUp"), Some(NavKey::Prev));
        assert_eq!(nav_key("Home"), Some(NavKey::First));
        assert_eq!(nav_key("End"), Some(NavKey::Last));
        assert_eq!(nav_key("a"), None);
    }

    #[test]
    fn tab_leaves_at_the_edges_or_with_nothing_to_tab_to() {
        // sem focáveis por Tab (menu de itens): qualquer Tab sai
        assert!(tab_leaves(0, None, true, false));
        assert!(tab_leaves(0, None, false, true));
        // 3 focáveis: Tab sai do último, Shift+Tab do primeiro
        assert!(tab_leaves(3, Some(2), false, false));
        assert!(!tab_leaves(3, Some(1), false, false));
        assert!(tab_leaves(3, Some(0), false, true));
        assert!(!tab_leaves(3, Some(1), false, true));
        // foco no popup: Tab entra no primeiro; Shift+Tab sai
        assert!(!tab_leaves(3, None, true, false));
        assert!(tab_leaves(3, None, true, true));
    }

    #[test]
    fn only_non_native_anchors_need_key_handling() {
        assert!(activates_natively("BUTTON", false));
        assert!(activates_natively("button", false));
        assert!(activates_natively("A", true));
        assert!(!activates_natively("A", false));
        assert!(!activates_natively("DIV", false));
        assert!(!activates_natively("SPAN", false));
    }
}
