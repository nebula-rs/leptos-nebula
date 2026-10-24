//! `Tooltip`: dica ancorada que abre com o mouse (hover) e com o foco do teclado.
//!
//! Mesma base do `Menu` (Popover API + motor de `crate::position`, ver `popup.rs`), mas sem
//! foco nem clique fora: o popup não recebe o ponteiro (`pointer-events: none`) e quem o dispara é
//! a âncora. Tudo que toca o DOM roda só em handlers e `Effect`s (SSR).

use std::time::Duration;

use leptos::leptos_dom::helpers::{TimeoutHandle, set_timeout_with_handle};
use leptos::{portal::Portal, prelude::*};
use wasm_bindgen::JsCast;

use super::popup::{
    Listener, Overlay, Params, Phase, add_id_ref, document, esc_is_top, esc_push, esc_remove,
    remove_id_ref,
};
use crate::position::{Horizontal, Position, Vertical};
use crate::util::{cls, next_id};

const BOTTOM_MIDDLE: Position = Position::new(Vertical::Bottom, Horizontal::Middle);
const TOP_MIDDLE: Position = Position::new(Vertical::Top, Horizontal::Middle);
/// `offset` padrão do `QTooltip`.
const DEFAULT_OFFSET: (f64, f64) = (14.0, 14.0);

/// Estado e ações do tooltip. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
struct Ctl {
    o: Overlay,
    /// Desejo de mostrar (resultado do hover/foco, depois dos atrasos).
    want: RwSignal<bool>,
    params: StoredValue<Params>,
    target: StoredValue<Option<String>>,
    id: StoredValue<String>,
    delay: u32,
    hide_delay: u32,
    no_parent_event: bool,
    on_show: Option<Callback<()>>,
    on_hide: Option<Callback<()>>,
    mounted: StoredValue<bool>,
    wired: StoredValue<bool>,
    /// Relógio do `delay`/`hide_delay` (separado do da transição, em `Overlay`).
    delay_timer: StoredValue<Option<TimeoutHandle>, LocalStorage>,
    anchor_listeners: StoredValue<Vec<Listener>, LocalStorage>,
    esc: StoredValue<Option<Listener>, LocalStorage>,
    /// `aria-describedby` foi acrescentado por nós (e deve sair ao desmontar).
    described: StoredValue<bool>,
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

    /// Liga a âncora: `pointerenter`/`pointerleave` (mouse, caneta e o toque) e `focusin`/
    /// `focusout` (só o foco de teclado). A descrição (`aria-describedby`) fica sempre: o texto do
    /// tooltip escondido entra na descrição acessível mesmo antes de ele aparecer.
    fn wire(&self, anchor: &web_sys::Element) {
        if self.wired.get_value() {
            return;
        }
        self.wired.set_value(true);
        let id = self.id.get_value();
        self.described
            .set_value(add_id_ref(anchor, "aria-describedby", &id));
        if self.no_parent_event {
            return;
        }
        let this = *self;
        let t: &web_sys::EventTarget = anchor.as_ref();
        let mut ls = vec![
            Listener::on(t, "pointerenter", move |_| this.schedule(true)),
            Listener::on(t, "pointerleave", move |_| this.schedule(false)),
        ];
        ls.push(Listener::on(t, "focusin", move |e| {
            // só o foco do teclado: clicar no alvo ou devolver o foco depois de fechar um
            // diálogo/menu não faz o tooltip aparecer
            let keyboard = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|el| el.matches(":focus-visible").unwrap_or(false));
            if keyboard {
                this.schedule(true);
            }
        }));
        let el = anchor.clone();
        ls.push(Listener::on(t, "focusout", move |e| {
            // o foco mudando DENTRO da âncora não é sair dela
            let moved_inside = e
                .dyn_ref::<web_sys::FocusEvent>()
                .and_then(web_sys::FocusEvent::related_target)
                .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                .is_some_and(|n| el.contains(Some(&n)));
            if !moved_inside {
                this.schedule(false);
            }
        }));
        self.anchor_listeners.set_value(ls);
    }

    fn clear_delay(&self) {
        if let Some(h) = self.delay_timer.try_update_value(Option::take).flatten() {
            h.clear();
        }
    }

    /// Pede para mostrar/esconder, depois de `delay`/`hide_delay` ms (o último pedido vale).
    fn schedule(&self, show: bool) {
        self.clear_delay();
        let ms = if show { self.delay } else { self.hide_delay };
        if ms == 0 {
            self.want.try_set(show);
            return;
        }
        let this = *self;
        let handle = set_timeout_with_handle(
            move || {
                this.delay_timer.try_set_value(None);
                this.want.try_set(show);
            },
            Duration::from_millis(u64::from(ms)),
        )
        .ok();
        self.delay_timer.try_set_value(handle);
    }

    fn sync(&self) {
        // `get` (e não `get_untracked`): roda de novo quando o `Portal` montar o popup
        if self.o.el.get().is_none() {
            return;
        }
        match (self.want.get(), self.o.phase.get_untracked()) {
            (true, Phase::Closed) => self.open(),
            (true, Phase::Leaving) => {
                self.o.revive();
                self.arm_esc();
            }
            (false, Phase::Open) => self.leave(),
            _ => {}
        }
    }

    fn open(&self) {
        let target = self.target.get_value();
        let Some(anchor) = self.o.anchor_for_open(target.as_deref()) else {
            leptos::logging::warn!("Tooltip: âncora não encontrada");
            self.want.set(false);
            return;
        };
        self.wire(&anchor);
        let on_show = self.on_show;
        let shown = self.o.open(&self.params.get_value(), move || {
            if let Some(cb) = on_show {
                cb.run(());
            }
        });
        if !shown {
            leptos::logging::warn!("Tooltip: o navegador não suporta a Popover API");
            self.want.set(false);
            return;
        }
        self.arm_esc();
    }

    fn leave(&self) {
        self.disarm_esc();
        let on_hide = self.on_hide;
        self.o.leave(move || {
            if let Some(cb) = on_hide {
                cb.run(());
            }
        });
    }

    /// `Esc` esconde o tooltip sem mexer no foco (WCAG 1.4.13).
    fn arm_esc(&self) {
        self.disarm_esc();
        let Some(doc) = document() else { return };
        let t: &web_sys::EventTarget = doc.as_ref();
        let (this, id) = (*self, self.o.id);
        esc_push(id);
        self.esc
            .set_value(Some(Listener::on(t, "keydown", move |e| {
                let is_esc = e
                    .dyn_ref::<web_sys::KeyboardEvent>()
                    .is_some_and(|k| k.key() == "Escape");
                if is_esc && esc_is_top(id) {
                    // dentro de um `Dialog`, o mesmo `Esc` não pode fechar o diálogo também
                    e.prevent_default();
                    this.clear_delay();
                    this.want.set(false);
                }
            })));
    }

    fn disarm_esc(&self) {
        self.esc.set_value(None);
        esc_remove(self.o.id);
    }

    fn cleanup(&self) {
        self.clear_delay();
        self.disarm_esc();
        self.anchor_listeners.set_value(Vec::new());
        if self.described.get_value()
            && let Some(a) = self.o.anchor()
        {
            remove_id_ref(&a, "aria-describedby", &self.id.get_value());
        }
        self.o.cleanup();
    }
}

/// Dica ancorada. ≈ `QTooltip`. Vai dentro da âncora (o elemento pai) ou aponta para ela com
/// `target` (um seletor CSS): `<Btn label="Salvar"><Tooltip>"Salva o rascunho"</Tooltip></Btn>`.
///
/// Abre com o ponteiro sobre a âncora (hover) **e** com o foco do teclado (`Tab`, nunca com um
/// clique), depois de `delay` ms; fecha ao sair, depois de `hide_delay` ms, e com `Esc` (sem mexer
/// no foco). O popup não recebe o ponteiro.
///
/// **Posição:** abaixo da âncora, centralizado (`anchor` `bottom middle`, `self_` `top middle`),
/// com `offset` de `(14, 14)`; perto da borda da janela vira para o outro lado
/// ([`crate::position`]).
///
/// **Acessibilidade:** o popup tem `role="tooltip"` e um `id` (o seu, ou um gerado) que a âncora
/// referencia em `aria-describedby` enquanto o `Tooltip` existe, então o texto é lido como a
/// descrição da âncora. Um tooltip descreve, não dá nome: um botão só com ícone precisa de
/// `aria_label`.
///
/// **Renderização:** o popup vive no `<body>` (um `Portal`, só no navegador); o servidor só
/// renderiza um `<span hidden>` na posição do `Tooltip`, cujo pai é a âncora.
///
/// Ainda não: `cursor_position`, `persistent`, `show` (`v-model`), `transition_*`, os gestos de
/// toque (arrastar/segurar).
#[component]
pub fn Tooltip(
    /// Seletor CSS da âncora; sem ele, a âncora é o elemento pai.
    #[prop(optional, into)]
    target: Option<String>,
    /// Ponto da âncora a que o tooltip se prende (padrão `bottom middle`).
    #[prop(optional)]
    anchor: Option<Position>,
    /// Ponto do tooltip que encosta no da âncora (padrão `top middle`).
    #[prop(optional)]
    self_: Option<Position>,
    /// `(x, y)` em pixels: expande a caixa da âncora para fora (padrão `(14.0, 14.0)`).
    #[prop(optional)]
    offset: Option<(f64, f64)>,
    /// Espera, em ms, antes de aparecer.
    #[prop(optional)]
    delay: u32,
    /// Espera, em ms, antes de sumir.
    #[prop(optional)]
    hide_delay: u32,
    /// Qualquer comprimento CSS (ex.: `"12rem"`); o padrão vem do token `--lx-tooltip-max-width`.
    #[prop(optional, into)]
    max_width: Option<String>,
    /// Qualquer comprimento CSS.
    #[prop(optional, into)]
    max_height: Option<String>,
    /// A âncora não dispara o tooltip.
    #[prop(optional)]
    no_parent_event: bool,
    /// `id` do popup (o que a âncora referencia em `aria-describedby`); gerado se omitido.
    #[prop(optional, into)]
    id: Option<String>,
    /// Depois da transição de entrada.
    #[prop(optional, into)]
    on_show: Option<Callback<()>>,
    /// Depois da transição de saída.
    #[prop(optional, into)]
    on_hide: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let id = id.unwrap_or_else(|| next_id("lx-tooltip"));
    let o = Overlay::new();
    let params = Params {
        anchor: anchor.unwrap_or(BOTTOM_MIDDLE),
        self_: self_.unwrap_or(TOP_MIDDLE),
        offset: offset.unwrap_or(DEFAULT_OFFSET),
        fit: false,
        cover: false,
        max_height,
        max_width,
    };
    let ctl = Ctl {
        o,
        want: RwSignal::new(false),
        params: StoredValue::new(params),
        target: StoredValue::new(target),
        id: StoredValue::new(id.clone()),
        delay,
        hide_delay,
        no_parent_event,
        on_show,
        on_hide,
        mounted: StoredValue::new(false),
        wired: StoredValue::new(false),
        delay_timer: StoredValue::new_local(None),
        anchor_listeners: StoredValue::new_local(Vec::new()),
        esc: StoredValue::new_local(None),
        described: StoredValue::new(false),
    };

    Effect::new(move |_| {
        if o.marker.get().is_some() {
            ctl.mount();
        }
    });
    Effect::new(move |_| ctl.sync());
    on_cleanup(move || ctl.cleanup());

    let phase = o.phase;
    // `children` é `FnOnce`, mas o `Portal` pede uma função que pode ser chamada de novo: o
    // conteúdo sai do `slot` na primeira chamada (a única que acontece).
    let slot = StoredValue::new_local(Some(children));
    let popup = move || {
        let class = move || {
            cls([
                "lx-tooltip",
                if phase.get() == Phase::Open {
                    "lx-tooltip-open"
                } else {
                    "lx-tooltip-enter"
                },
            ])
        };
        let content = slot.try_update_value(Option::take).flatten();
        view! {
            <div node_ref=o.el popover="manual" id=id.clone() role="tooltip" class=class>
                {content.map(|c| c())}
            </div>
        }
    };

    view! {
        <span hidden=true node_ref=o.marker></span>
        <Portal>{popup()}</Portal>
    }
}
