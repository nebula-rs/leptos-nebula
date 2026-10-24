//! Cola entre a máscara pura ([`MaskState`]) e o `Input`: o controle nativo como [`MaskControl`],
//! o sinal do texto exibido (`inner`) e os handlers. Só roda no navegador (os handlers e os
//! `Effect`s); a renderização no servidor só lê o valor inicial.

use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};

use super::component::Native;
use super::mask::{
    Direction, MaskControl, MaskKey, MaskKeyEvent, MaskOutput, MaskProps, MaskState, char_to_utf16,
    utf16_to_char,
};

/// O `<input>` ou `<textarea>` do `Input` como [`MaskControl`]. Posições em caracteres; o DOM
/// conta em unidades UTF-16.
pub(super) enum DomEl {
    Input(HtmlInputElement),
    Textarea(HtmlTextAreaElement),
}

impl DomEl {
    fn raw_value(&self) -> String {
        match self {
            DomEl::Input(e) => e.value(),
            DomEl::Textarea(e) => e.value(),
        }
    }

    fn raw_selection(&self) -> (usize, usize, Direction) {
        let (s, e, d) = match self {
            DomEl::Input(el) => (
                el.selection_start(),
                el.selection_end(),
                el.selection_direction(),
            ),
            DomEl::Textarea(el) => (
                el.selection_start(),
                el.selection_end(),
                el.selection_direction(),
            ),
        };
        let dir = match d.ok().flatten().as_deref() {
            Some("forward") => Direction::Forward,
            Some("backward") => Direction::Backward,
            _ => Direction::None,
        };
        (
            s.ok().flatten().unwrap_or(0) as usize,
            e.ok().flatten().unwrap_or(0) as usize,
            dir,
        )
    }

    fn element(&self) -> &web_sys::Element {
        match self {
            DomEl::Input(e) => e.as_ref(),
            DomEl::Textarea(e) => e.as_ref(),
        }
    }
}

impl MaskControl for DomEl {
    fn value(&self) -> String {
        self.raw_value()
    }

    fn set_value(&mut self, v: &str) {
        match self {
            DomEl::Input(e) => e.set_value(v),
            DomEl::Textarea(e) => e.set_value(v),
        }
    }

    fn selection(&self) -> (usize, usize, Direction) {
        let v = self.raw_value();
        let (s, e, d) = self.raw_selection();
        (utf16_to_char(&v, s), utf16_to_char(&v, e), d)
    }

    fn set_selection(&mut self, start: usize, end: usize, dir: Direction) {
        let v = self.raw_value();
        let (s, e) = (
            char_to_utf16(&v, start) as u32,
            char_to_utf16(&v, end) as u32,
        );
        let dir = match dir {
            Direction::Forward => "forward",
            Direction::Backward => "backward",
            Direction::None => "none",
        };
        let _ = match self {
            DomEl::Input(el) => el.set_selection_range_with_direction(s, e, dir),
            DomEl::Textarea(el) => el.set_selection_range_with_direction(s, e, dir),
        };
    }

    fn is_focused(&self) -> bool {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .is_some_and(|a| &a == self.element())
    }
}

/// Estado reativo da máscara de um `Input`. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
pub(super) struct Masking {
    state: StoredValue<MaskState, LocalStorage>,
    /// O texto exibido no controle (≈ `innerValue`). Sem máscara espelha `value`.
    pub inner: RwSignal<String>,
    /// Último valor que o próprio `Input` escreveu em `value` (para o `Effect` do modelo ignorar o
    /// eco: ≈ `stopValueWatcher` + `emitCachedValue`).
    emitted: StoredValue<Option<String>>,
    /// IME em curso com máscara ativa (≈ `qComposing`).
    composing: StoredValue<bool>,
    value: RwSignal<String>,
    native: Native,
}

impl Masking {
    pub fn new(value: RwSignal<String>, native: Native, props: MaskProps) -> Self {
        let state = MaskState::new(MaskProps {
            model: value.get_untracked(),
            ..props
        });
        let inner = RwSignal::new(state.inner_value().to_string());
        Self {
            state: StoredValue::new_local(state),
            inner,
            emitted: StoredValue::new(None),
            composing: StoredValue::new(false),
            value,
            native,
        }
    }

    pub fn has_mask(&self) -> bool {
        self.state.with_value(MaskState::has_mask)
    }

    /// `state` + o controle (se já montado).
    fn run<R>(
        &self,
        f: impl FnOnce(&mut MaskState, Option<&mut dyn MaskControl>) -> R,
    ) -> Option<R> {
        let mut dom = self.native.dom();
        self.state.try_update_value(|st| {
            st.set_model(&self.value.get_untracked());
            f(st, dom.as_mut().map(|d| d as &mut dyn MaskControl))
        })
    }

    fn sync_inner(&self) {
        let text = self.state.with_value(|s| s.inner_value().to_string());
        if self.inner.with_untracked(|i| *i != text) {
            self.inner.set(text);
        }
    }

    fn apply(&self, out: MaskOutput) {
        if let Some(v) = out.emit {
            if out.stop_watcher {
                self.emitted.set_value(Some(v.clone()));
            }
            self.value.set(v);
        }
        self.sync_inner();
    }

    /// O "próximo ciclo" do Vue: depois do handler, antes do próximo evento.
    fn schedule_caret(&self) {
        let this = *self;
        spawn_local(async move {
            this.run(|st, ctl| st.flush_caret(ctl));
        });
    }

    /// `onInput` com máscara.
    pub fn on_input(&self, text: &str, input_type: Option<String>) {
        if self.composing.get_value() {
            return;
        }
        self.process(text, input_type);
    }

    fn process(&self, text: &str, input_type: Option<String>) {
        let it = input_type.filter(|t| !t.is_empty());
        if let Some(out) = self.run(|st, ctl| st.update_mask_value(ctl, text, false, it.as_deref()))
        {
            self.apply(out);
        }
        self.schedule_caret();
    }

    pub fn on_composition_start(&self) {
        if self.has_mask() {
            self.composing.set_value(true);
        }
    }

    /// `compositionend`/`change`: processa o texto final da composição.
    pub fn on_composition_end(&self, text: &str) {
        if self.composing.get_value() {
            self.composing.set_value(false);
            self.process(text, None);
        }
    }

    pub fn on_click(&self) {
        self.state.update_value(MaskState::on_masked_click);
    }

    /// `onMaskedKeydown`; `true` = chamar `preventDefault`.
    pub fn on_keydown(&self, ev: &web_sys::KeyboardEvent) -> bool {
        let key = match ev.key().as_str() {
            "ArrowLeft" => MaskKey::Left,
            "ArrowRight" => MaskKey::Right,
            "Backspace" => MaskKey::Backspace,
            "Delete" => MaskKey::Delete,
            _ => MaskKey::Other,
        };
        let event = MaskKeyEvent {
            key,
            shift: ev.shift_key(),
            alt: ev.alt_key(),
            default_prevented: ev.default_prevented(),
            composing: ev.is_composing(),
        };
        self.run(|st, ctl| ctl.map(|c| st.on_masked_keydown(c, &event)))
            .flatten()
            .unwrap_or(false)
    }

    /// `onPaste` com máscara (sem `reverse_fill_mask`): ancora a colagem no primeiro slot livre.
    pub fn on_paste(&self) {
        self.run(|st, ctl| {
            if st.reverse_fill_mask() {
                return;
            }
            if let Some(c) = ctl {
                let (s, e, _) = c.selection();
                st.move_cursor_for_paste(c, s, e);
            }
        });
    }

    /// `watch(modelValue)`: o modelo mudou por fora (ou foi o eco do próprio `Input`).
    pub fn on_model(&self, v: String) {
        if self.has_mask() {
            let own = self.emitted.try_update_value(Option::take).flatten();
            if own.as_deref() == Some(v.as_str()) {
                return;
            }
            if let Some(out) = self.run(|st, ctl| st.update_mask_value(ctl, &v, false, None)) {
                self.apply(out);
            }
            self.schedule_caret();
        } else {
            self.emitted.set_value(None);
            self.state.update_value(|s| s.set_inner_plain(&v));
            self.sync_inner();
        }
    }

    /// `watch` das props de máscara (mudaram `mask`, `fill_mask`/`reverse_fill_mask` ou
    /// `unmasked_value`). O que mudar no modelo entra em `value`.
    pub fn on_props(&self, change: PropsChange) {
        let mut emit: Option<MaskOutput> = None;
        self.run(|st, mut ctl| {
            let mut outs = Vec::new();
            if let Some(mask) = change.mask {
                outs.push(st.set_mask(reborrow(&mut ctl), mask));
            }
            if let Some((fill, reverse)) = change.fill {
                outs.push(st.set_fill(reborrow(&mut ctl), fill, reverse));
            }
            if let Some(unmasked) = change.unmasked {
                outs.push(st.set_unmasked_value(reborrow(&mut ctl), unmasked));
            }
            emit = outs.into_iter().rfind(|o| o.emit.is_some());
        });
        self.apply(emit.unwrap_or_default());
        // Sem máscara (nunca teve ou acabou de sair), o texto exibido é o próprio modelo.
        if !self.has_mask() {
            let v = self.value.get_untracked();
            self.state.update_value(|s| s.set_inner_plain(&v));
            self.sync_inner();
        }
        self.schedule_caret();
    }
}

/// Reempresta o controle para mais de uma chamada (o `as_deref_mut` prende o tempo de vida do
/// `dyn`).
fn reborrow<'a>(c: &'a mut Option<&mut dyn MaskControl>) -> Option<&'a mut dyn MaskControl> {
    match c {
        Some(x) => Some(&mut **x),
        None => None,
    }
}

/// O que mudou nas props de máscara entre dois ciclos.
#[derive(Default)]
pub(super) struct PropsChange {
    pub mask: Option<Option<String>>,
    pub fill: Option<(super::mask::FillMask, bool)>,
    pub unmasked: Option<bool>,
}
