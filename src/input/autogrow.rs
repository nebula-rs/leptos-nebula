//! `autogrow` do `Input` (≈ `use-autogrow.js` do Quasar): o `<textarea>` cresce e encolhe com o
//! conteúdo.
//!
//! Dois caminhos, como no Quasar: onde existe `field-sizing: content` o próprio navegador mede
//! (a classe `lx-input-textarea-autogrow` basta; [`needs_js`] devolve `false`); nos demais, a
//! rotina de medição reduz o `textarea` a 1px, lê o `scrollHeight` e o grava como `height`.
//!
//! A parte pura (decisões e a fila de quadros) fica no topo, testada no host; [`Autogrow`] a
//! liga ao DOM e ao `requestAnimationFrame`, só em handlers e `Effect`s.

use leptos::leptos_dom::helpers::{
    AnimationFrameRequestHandle, request_animation_frame_with_handle,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlTextAreaElement;

/// Firefox (`$q.platform.is.firefox`): lá o `overflow-y` não é tocado.
pub(super) fn is_firefox(user_agent: &str) -> bool {
    user_agent.contains("Firefox/")
}

/// A rotina mexe no `overflow-y` durante a medição (`overflowY !== void 0 && !== 'scroll'`):
/// `None` no Firefox ou sem estilo computado; `scroll` explícito (#14263, #14344) fica como está.
pub(super) fn changes_overflow(overflow_y: Option<&str>) -> bool {
    matches!(overflow_y, Some(o) if o != "scroll")
}

/// `Number.parseInt(s, 10)`: o prefixo inteiro, ou `None` (`NaN`, ex.: `"none"`).
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start();
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let n: i64 = digits.parse().ok()?;
    Some(if neg { -n } else { n })
}

/// `overflow-y` final: só há barra de rolagem se existe `max-height` e o conteúdo é mais alto
/// que ele (`Number.parseInt(maxHeight, 10) < scrollHeight`; `NaN` nunca é menor).
pub(super) fn final_overflow(max_height: &str, scroll_height: i32) -> &'static str {
    match parse_int(max_height) {
        Some(max) if max < i64::from(scroll_height) => "auto",
        _ => "hidden",
    }
}

/// `margin-bottom` provisório do pai (`scrollHeight - 1`): mantém o tamanho total do controle
/// enquanto o `textarea` é reduzido a 1px, para a página não pular.
pub(super) fn guard_margin(scroll_height: i32) -> String {
    format!("{}px", scroll_height - 1)
}

/// Fila de medições: no máximo **uma por quadro**, por mais que peçam (uma tecla pede duas vezes:
/// o `input` e a volta do modelo).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FrameGate {
    pending: bool,
}

impl FrameGate {
    /// Pede uma medição; `true` se é preciso agendar o quadro (não havia um pendente).
    pub fn request(&mut self) -> bool {
        !std::mem::replace(&mut self.pending, true)
    }

    /// O quadro chegou: a próxima pergunta agenda outro.
    pub fn done(&mut self) {
        self.pending = false;
    }

    /// Descarta a medição pendente (descarte do campo); `true` se havia uma a cancelar.
    pub fn cancel(&mut self) -> bool {
        std::mem::replace(&mut self.pending, false)
    }
}

/// `CSS.supports('field-sizing', 'content')`: sem ele, usa-se a medição em JS. Só no navegador.
pub(super) fn needs_js() -> bool {
    let supports = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"CSS".into()).ok())
        .and_then(|css| {
            let f = js_sys::Reflect::get(&css, &"supports".into()).ok()?;
            let f: js_sys::Function = f.dyn_into().ok()?;
            f.call2(&css, &"field-sizing".into(), &"content".into())
                .ok()
        })
        .and_then(|v| v.as_bool());
    supports == Some(false)
}

/// Medição em JS de um `<textarea>`. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
pub(super) struct Autogrow {
    gate: StoredValue<FrameGate>,
    frame: StoredValue<Option<AnimationFrameRequestHandle>, LocalStorage>,
    el: NodeRef<leptos::html::Textarea>,
}

impl Autogrow {
    pub fn new(el: NodeRef<leptos::html::Textarea>) -> Self {
        Self {
            gate: StoredValue::new(FrameGate::default()),
            frame: StoredValue::new_local(None),
            el,
        }
    }

    /// Pede uma medição (no próximo quadro, uma só por quadro).
    pub fn adjust(&self) {
        if self.gate.try_update_value(FrameGate::request) != Some(true) {
            return;
        }
        let this = *self;
        let handle = request_animation_frame_with_handle(move || {
            this.frame.try_set_value(None);
            this.gate.try_update_value(FrameGate::done);
            if let Some(el) = this.el.get_untracked() {
                measure(&el);
            }
        })
        .ok();
        self.frame.try_set_value(handle);
    }

    /// Descarta a medição pendente.
    pub fn cancel(&self) {
        if self.gate.try_update_value(FrameGate::cancel) == Some(true)
            && let Some(h) = self.frame.try_update_value(Option::take).flatten()
        {
            h.cancel();
        }
    }
}

/// O corpo de `adjustHeight` (o quadro já chegou).
fn measure(el: &HtmlTextAreaElement) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let parent = el
        .parent_element()
        .and_then(|p| p.dyn_into::<web_sys::HtmlElement>().ok());
    // O Chrome não mantém a rolagem (#15498).
    let scroll_top = el.scroll_top();
    let ua = window.navigator().user_agent().unwrap_or_default();
    let computed = if is_firefox(&ua) {
        None
    } else {
        window.get_computed_style(el).ok().flatten()
    };
    let (overflow_y, max_height) = match &computed {
        Some(c) => (
            c.get_property_value("overflow-y").ok(),
            c.get_property_value("max-height").unwrap_or_default(),
        ),
        None => (None, String::new()),
    };
    let change = changes_overflow(overflow_y.as_deref());
    let style = web_sys::HtmlElement::style(el);
    let parent_style = parent.as_ref().map(|p| p.style());

    // Reduz o controle para ler a altura real, sem mudar o tamanho total.
    if change {
        let _ = style.set_property("overflow-y", "hidden");
    }
    if let Some(ps) = &parent_style {
        let _ = ps.set_property("margin-bottom", &guard_margin(el.scroll_height()));
    }
    let _ = style.set_property("height", "1px");
    let height = el.scroll_height();
    let _ = style.set_property("height", &format!("{height}px"));
    // Só há barra de rolagem se existe `max-height` e o conteúdo passa dele.
    if change {
        let _ = style.set_property("overflow-y", final_overflow(&max_height, height));
    }
    if let Some(ps) = &parent_style {
        let _ = ps.remove_property("margin-bottom");
    }
    el.set_scroll_top(scroll_top);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_most_one_measurement_per_frame() {
        // `coalesces the measurements requested within one frame`
        let mut g = FrameGate::default();
        assert!(g.request(), "a primeira pede o quadro");
        assert!(!g.request(), "a segunda no mesmo quadro é absorvida");
        assert!(!g.request());
        g.done();
        assert!(
            g.request(),
            "depois do quadro, um novo pedido agenda de novo"
        );
    }

    #[test]
    fn cancel_drops_a_pending_measurement() {
        // `drops a pending measurement on unmount`: cancela exatamente uma vez
        let mut g = FrameGate::default();
        assert!(!g.cancel(), "nada a cancelar");
        assert!(g.request());
        assert!(g.cancel());
        assert!(!g.cancel(), "só uma vez");
        assert!(g.request(), "e o campo pode pedir de novo");
    }

    #[test]
    fn overflow_is_left_alone_when_scroll_or_unknown() {
        assert!(changes_overflow(Some("auto")));
        assert!(changes_overflow(Some("visible")));
        assert!(changes_overflow(Some("hidden")));
        assert!(!changes_overflow(Some("scroll")));
        assert!(!changes_overflow(None), "Firefox / sem estilo computado");
    }

    #[test]
    fn scrollbar_only_when_taller_than_max_height() {
        // `sizes the textarea on mount`: sem max-height o controle fica `hidden`
        assert_eq!(final_overflow("none", 400), "hidden");
        assert_eq!(final_overflow("", 400), "hidden");
        assert_eq!(final_overflow("120px", 80), "hidden");
        assert_eq!(final_overflow("120px", 120), "hidden");
        assert_eq!(final_overflow("120px", 121), "auto");
        assert_eq!(final_overflow(" 120.5px", 200), "auto");
        assert_eq!(final_overflow("-5px", 1), "auto");
    }

    #[test]
    fn guard_margin_keeps_the_total_height() {
        assert_eq!(guard_margin(80), "79px");
        assert_eq!(guard_margin(1), "0px");
    }

    #[test]
    fn firefox_detection() {
        assert!(is_firefox(
            "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0"
        ));
        assert!(!is_firefox(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120 Safari/537.36"
        ));
    }

    #[test]
    fn parse_int_follows_js() {
        assert_eq!(parse_int("120px"), Some(120));
        assert_eq!(parse_int("  7"), Some(7));
        assert_eq!(parse_int("-3px"), Some(-3));
        assert_eq!(parse_int("none"), None);
        assert_eq!(parse_int(""), None);
    }
}
