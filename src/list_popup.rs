//! A máquina do popup de lista dos controles `combobox` (`Select` e `Autocomplete`): âncora na
//! caixa do campo, abertura/saída com transição, rolagem da opção ativa e o fechamento por `Esc`
//! e por clique fora. Extraída do `select/control.rs`, que o `autocomplete/control.rs` havia
//! duplicado (R-45); `docs/ui/forms.md#select`.
//!
//! Reaproveita o [`Overlay`] (`components::popup`: Popover API, `Portal`, motor de posição, pilha
//! de `Esc`). O que muda entre os controles entra por parâmetro: o que quer a lista aberta
//! (`want`), os ganchos (`on_leave`, `after_open`, `on_show`, `on_hide`) e a tag nos avisos.
//!
//! Tudo que toca o DOM roda só em handlers e `Effect`s, nunca durante a renderização (SSR).

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::popup::{
    Listener, ListenerOpts, Overlay, Params, Phase, document, esc_is_top, esc_push, esc_remove,
};
use crate::position::{Horizontal, Position, Vertical};
use crate::util::cls;

const BOTTOM_START: Position = Position::new(Vertical::Bottom, Horizontal::Start);
const TOP_START: Position = Position::new(Vertical::Top, Horizontal::Start);

/// Ganchos opcionais do popup (todos `Callback`, logo `Copy`).
#[derive(Clone, Copy, Default)]
pub(crate) struct Hooks {
    /// Depois da transição de entrada.
    pub on_show: Option<Callback<()>>,
    /// Depois da transição de saída.
    pub on_hide: Option<Callback<()>>,
    /// Ao começar a sair (antes da transição).
    pub on_leave: Option<Callback<()>>,
    /// Logo depois de o popup ficar visível (ainda sem a transição concluída).
    pub after_open: Option<Callback<()>>,
}

/// Estado e ações do popup de uma lista. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
pub(crate) struct ListPopup {
    pub o: Overlay,
    /// O usuário quer a lista aberta (o que o controle liga e desliga).
    pub open: RwSignal<bool>,
    /// Se a lista aparece de fato (o `Select` = `open`; o `Autocomplete` = `open` e com itens).
    pub want: Signal<bool>,
    /// Índice da opção ativa (o `aria-activedescendant`).
    pub active: RwSignal<Option<usize>>,
    params: StoredValue<Params>,
    dismiss: StoredValue<Vec<Listener>, LocalStorage>,
    mounted: StoredValue<bool>,
    id: StoredValue<String>,
    /// Nome do componente, para os avisos do console.
    tag: &'static str,
    hooks: Hooks,
}

impl ListPopup {
    /// `open`/`want`/`active` vêm do controle (ele os lê e escreve em seus handlers).
    pub(crate) fn new(
        tag: &'static str,
        id: &str,
        open: RwSignal<bool>,
        want: Signal<bool>,
        active: RwSignal<Option<usize>>,
        hooks: Hooks,
    ) -> Self {
        Self {
            o: Overlay::new(),
            open,
            want,
            active,
            params: StoredValue::new(Params {
                anchor: BOTTOM_START,
                self_: TOP_START,
                offset: (0.0, 4.0),
                fit: true,
                cover: false,
                max_height: None,
                max_width: None,
            }),
            dismiss: StoredValue::new_local(Vec::new()),
            mounted: StoredValue::new(false),
            id: StoredValue::new(id.to_string()),
            tag,
            hooks,
        }
    }

    /// `id` da opção `i`: `<id>_opt_<i>` (posicional na lista **completa**, nunca na filtrada).
    pub(crate) fn option_id(&self, i: usize) -> String {
        format!("{}_opt_{i}", self.id.get_value())
    }

    /// Liga a âncora: a caixa do campo (`.lx-field-box`), achada com `closest` a partir do
    /// marcador. Devolve a âncora **só na primeira vez** que acha (para o controle pôr os seus
    /// ouvintes nela).
    pub(crate) fn mount(&self) -> Option<web_sys::Element> {
        if self.mounted.get_value() {
            return None;
        }
        let marker = self.o.marker.get_untracked()?;
        let Ok(Some(anchor)) = marker.closest(".lx-field-box") else {
            return None;
        };
        self.mounted.set_value(true);
        self.o.set_anchor(anchor.clone());
        Some(anchor)
    }

    /// Faz a fase do popup seguir `want`.
    pub(crate) fn sync(&self) {
        // `get` (e não `get_untracked`): roda de novo quando o `Portal` montar o popup
        if self.o.el.get().is_none() {
            return;
        }
        match (self.want.get(), self.o.phase.get_untracked()) {
            (true, Phase::Closed) => self.do_open(),
            (true, Phase::Leaving) => {
                self.o.revive();
                self.arm_dismiss();
            }
            (false, Phase::Open) => self.leave(),
            _ => {}
        }
    }

    fn do_open(&self) {
        let on_show = self.hooks.on_show;
        let shown = self.o.open(&self.params.get_value(), move || {
            if let Some(cb) = on_show {
                cb.run(());
            }
        });
        if !shown {
            leptos::logging::warn!("{}: sem âncora ou sem suporte à Popover API", self.tag);
            self.open.set(false);
            return;
        }
        self.arm_dismiss();
        if let Some(cb) = self.hooks.after_open {
            cb.run(());
        }
        // o popup acabou de ficar visível: a opção ativa já pode ser rolada para a vista
        self.scroll_to(self.active.get_untracked());
    }

    fn leave(&self) {
        self.disarm_dismiss();
        self.active.set(None);
        if let Some(cb) = self.hooks.on_leave {
            cb.run(());
        }
        let on_hide = self.hooks.on_hide;
        self.o.leave(move || {
            if let Some(cb) = on_hide {
                cb.run(());
            }
        });
    }

    /// Rola a opção `i` para a vista dentro da lista.
    pub(crate) fn scroll_to(&self, i: Option<usize>) {
        let Some(i) = i else { return };
        // in the popup's own root (document or shadow root)
        let near: Option<web_sys::Node> = self
            .o
            .popup()
            .map(Into::into)
            .or_else(|| self.o.anchor().map(Into::into));
        let Some(el) = near.and_then(|n| crate::root::by_id_near(&n, &self.option_id(i))) else {
            return;
        };
        let opts = web_sys::ScrollIntoViewOptions::new();
        opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
        el.scroll_into_view_with_scroll_into_view_options(&opts);
    }

    /// Passa o mouse sobre a opção `i` (já sabendo que é habilitada): torna-a ativa.
    pub(crate) fn hover(&self, i: usize) {
        if self.active.get_untracked() != Some(i) {
            self.active.set(Some(i));
        }
    }

    /// `Esc` (na pilha de `Esc`, com `preventDefault`, para não fechar um `Dialog` junto) e
    /// `pointerdown` fora do popup e do campo fecham a lista.
    fn arm_dismiss(&self) {
        self.disarm_dismiss();
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
                this.open.set(false);
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

    /// `pointerdown` fora do popup e do campo fecha (o clique dentro do campo é do controle).
    fn on_outside(&self, e: &web_sys::Event) {
        let Some(node) = crate::root::origin_node(e) else {
            return;
        };
        if !self.contains(&node) && self.o.phase.get_untracked() == Phase::Open {
            self.open.set(false);
        }
    }

    /// O nó está dentro do popup?
    pub(crate) fn in_popup(&self, node: &web_sys::Node) -> bool {
        self.o.popup().is_some_and(|p| p.contains(Some(node)))
    }

    /// O nó está dentro do popup ou da âncora (a caixa do campo)?
    fn contains(&self, node: &web_sys::Node) -> bool {
        self.in_popup(node) || self.o.anchor().is_some_and(|a| a.contains(Some(node)))
    }

    /// Desmontagem: solta os ouvintes e o `Overlay`.
    pub(crate) fn cleanup(&self) {
        self.disarm_dismiss();
        self.o.cleanup();
    }

    /// Classe do contêiner do popup (`lx-menu*`): visível só em `Open`.
    pub(crate) fn popup_class(&self) -> impl Fn() -> String + Copy + use<> {
        let phase = self.o.phase;
        move || {
            cls([
                "lx-menu",
                if phase.get() == Phase::Open {
                    "lx-menu-open"
                } else {
                    "lx-menu-enter"
                },
            ])
        }
    }
}
