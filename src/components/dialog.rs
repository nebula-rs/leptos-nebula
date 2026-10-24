//! `Dialog` sobre o elemento nativo `<dialog>` (`showModal()`/`close()`): camada superior,
//! foco preso, `inert` no resto da página e foco devolvido ao fechar vêm do navegador
//! (ver "Sobreposições" em `docs/ui/overview.md`).
//!
//! A lógica sem DOM (regras de `Esc`/clique fora, leitura de durações, contagem do bloqueio de
//! rolagem) fica em funções puras testadas no host; [`Ctl`] a liga ao elemento, aos sinais e aos
//! relógios — só em handlers e `Effect`s, nunca durante a renderização (SSR).

use std::{cell::Cell, time::Duration};

use leptos::leptos_dom::helpers::{TimeoutHandle, set_timeout_with_handle};
use leptos::{ev, html, prelude::*};
use wasm_bindgen::{JsCast, JsValue};

use crate::util::cls;

/// Lado em que o [`Dialog`] encosta. `Standard` (padrão) centraliza.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DialogPosition {
    #[default]
    Standard,
    Top,
    Bottom,
    Left,
    Right,
}

// ── Lógica pura ────────────────────────────────────────────────────────────

/// O que fazer com uma tentativa de dispensar o diálogo (`Esc` ou clique fora).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dismiss {
    /// Fechar.
    Hide,
    /// Recusar e "sacudir" o painel.
    Shake,
    /// Recusar sem sacudir (diálogo maximizado: não há por onde sacudir).
    Ignore,
}

/// `onEscapeKey` do `QDialog`: `persistent` ou `no_esc_dismiss` recusam; maximizado não sacode.
fn esc_action(persistent: bool, no_esc_dismiss: bool, maximized: bool) -> Dismiss {
    if persistent || no_esc_dismiss {
        if maximized {
            Dismiss::Ignore
        } else {
            Dismiss::Shake
        }
    } else {
        Dismiss::Hide
    }
}

/// `onBackdropPress` do `QDialog`: `persistent` ou `no_backdrop_dismiss` recusam e sacodem.
fn backdrop_action(persistent: bool, no_backdrop_dismiss: bool, maximized: bool) -> Dismiss {
    if persistent || no_backdrop_dismiss {
        if maximized {
            Dismiss::Ignore
        } else {
            Dismiss::Shake
        }
    } else {
        Dismiss::Hide
    }
}

/// Lê um valor de `transition-duration` (lista separada por vírgulas, em `s` ou `ms`) e devolve
/// a maior duração, em ms. Valor ilegível conta como 0.
fn parse_duration_ms(css: &str) -> u32 {
    css.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if let Some(ms) = part.strip_suffix("ms") {
                ms.trim().parse::<f64>().ok()
            } else if let Some(s) = part.strip_suffix('s') {
                s.trim().parse::<f64>().ok().map(|v| v * 1000.0)
            } else {
                None
            }
        })
        .fold(0.0_f64, f64::max)
        .round() as u32
}

/// O `close` nativo veio logo depois de um `cancel` recusado? O Chrome fecha o diálogo mesmo
/// com `preventDefault()` quando o `Esc` é repetido sem interação do usuário no meio.
fn is_forced_close(blocked_at: Option<f64>, now: f64) -> bool {
    blocked_at.is_some_and(|t| (0.0..200.0).contains(&(now - t)))
}

/// Contagem de diálogos abertos que seguram o bloqueio de rolagem. Devolve `true` quando a
/// mudança é a que liga (1º) ou desliga (último) o bloqueio.
fn lock_acquire(count: &mut u32) -> bool {
    *count += 1;
    *count == 1
}

fn lock_release(count: &mut u32) -> bool {
    if *count == 0 {
        return false;
    }
    *count -= 1;
    *count == 0
}

thread_local! {
    static SCROLL_LOCKS: Cell<u32> = const { Cell::new(0) };
}

fn set_scroll_lock(on: bool) {
    let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    else {
        return;
    };
    let _ = if on {
        root.class_list().add_1("lx-scroll-lock")
    } else {
        root.class_list().remove_1("lx-scroll-lock")
    };
}

// ── Ligação ao DOM ─────────────────────────────────────────────────────────

/// Fase visual: só `Open` mostra o painel; `Leaving` espera a transição de saída antes do
/// `close()`. `Closed` = o `<dialog>` não está aberto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Closed,
    Open,
    Leaving,
}

/// Estado e ações do diálogo. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
struct Ctl {
    dialog: NodeRef<html::Dialog>,
    body: NodeRef<html::Div>,
    show: RwSignal<bool>,
    phase: RwSignal<Phase>,
    shaking: RwSignal<bool>,
    persistent: bool,
    no_esc_dismiss: bool,
    no_backdrop_dismiss: bool,
    maximized: bool,
    on_show: Option<Callback<()>>,
    on_hide: Option<Callback<()>>,
    on_escape_key: Option<Callback<()>>,
    timer: StoredValue<Option<TimeoutHandle>, LocalStorage>,
    /// Elemento com foco quando o diálogo abriu (o gatilho).
    trigger: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
    /// Este diálogo segura o bloqueio de rolagem.
    locked: StoredValue<bool>,
    /// Fomos nós que chamamos `close()` (o evento `close` que vem depois é nosso).
    closing: StoredValue<bool>,
    /// Instante (ms) do último `cancel` recusado.
    blocked_at: StoredValue<Option<f64>>,
}

impl Ctl {
    fn clear_timer(&self) {
        if let Some(h) = self.timer.try_update_value(Option::take).flatten() {
            h.clear();
        }
    }

    fn after(&self, ms: u32, f: impl FnOnce() + 'static) {
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

    /// Duração da transição do painel (tokens `--lx-dialog-duration`; 0 com reduced-motion
    /// se o tema assim definir).
    fn transition_ms(&self) -> u32 {
        let Some(body) = self.body.get_untracked() else {
            return 0;
        };
        web_sys::window()
            .and_then(|w| w.get_computed_style(&body).ok().flatten())
            .and_then(|s| s.get_property_value("transition-duration").ok())
            .map(|v| parse_duration_ms(&v))
            .unwrap_or(0)
    }

    fn acquire_lock(&self) {
        if self.locked.try_get_value().unwrap_or(true) {
            return;
        }
        self.locked.try_set_value(true);
        if SCROLL_LOCKS.with(|c| {
            let mut n = c.get();
            let first = lock_acquire(&mut n);
            c.set(n);
            first
        }) {
            set_scroll_lock(true);
        }
    }

    fn release_lock(&self) {
        if !self.locked.try_get_value().unwrap_or(false) {
            return;
        }
        self.locked.try_set_value(false);
        if SCROLL_LOCKS.with(|c| {
            let mut n = c.get();
            let last = lock_release(&mut n);
            c.set(n);
            last
        }) {
            set_scroll_lock(false);
        }
    }

    /// Reage ao `show`: abre, fecha (com a transição) ou cancela um fechamento em curso.
    fn sync(&self) {
        if self.dialog.get_untracked().is_none() {
            return;
        }
        let want = self.show.get();
        match (want, self.phase.get_untracked()) {
            (true, Phase::Closed) => self.open(),
            (true, Phase::Leaving) => {
                self.clear_timer();
                self.phase.set(Phase::Open);
            }
            (false, Phase::Open) => self.leave(),
            _ => {}
        }
    }

    fn open(&self) {
        let Some(dialog) = self.dialog.get_untracked() else {
            return;
        };
        self.clear_timer();
        let trigger = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
        self.trigger.try_set_value(trigger);
        if !dialog.open() && dialog.show_modal().is_err() {
            // Fora do documento: não há como abrir; o sinal volta ao que o DOM mostra.
            self.show.set(false);
            return;
        }
        self.acquire_lock();
        self.shaking.set(false);
        // Fixa o estado inicial da transição antes de trocar para o aberto.
        let _ = dialog.offset_width();
        let ms = self.transition_ms();
        self.phase.set(Phase::Open);
        let on_show = self.on_show;
        self.after(ms, move || {
            if let Some(cb) = on_show {
                cb.run(());
            }
        });
    }

    fn leave(&self) {
        let ms = self.transition_ms();
        self.phase.set(Phase::Leaving);
        let this = *self;
        self.after(ms, move || this.finish_hide());
    }

    fn finish_hide(&self) {
        if let Some(dialog) = self.dialog.get_untracked()
            && dialog.open()
        {
            self.closing.try_set_value(true);
            dialog.close();
        }
        self.release_lock();
        self.phase.set(Phase::Closed);
        self.shaking.set(false);
        // O navegador devolve o foco ao fechar; repete à mão porque o Safari não foca botões
        // ao clicar (o elemento ativo na abertura seria o `<body>`).
        if let Some(el) = self.trigger.try_update_value(Option::take).flatten()
            && el.is_connected()
        {
            let opts = web_sys::FocusOptions::new();
            opts.set_prevent_scroll(true);
            let _ = el.focus_with_options(&opts);
        }
        if let Some(cb) = self.on_hide {
            cb.run(());
        }
    }

    fn shake(&self) {
        if self.phase.get_untracked() == Phase::Open {
            self.shaking.set(true);
        }
    }

    fn apply(&self, action: Dismiss, escape: bool) {
        match action {
            Dismiss::Hide => {
                if escape && let Some(cb) = self.on_escape_key {
                    cb.run(());
                }
                self.show.set(false);
            }
            Dismiss::Shake => self.shake(),
            Dismiss::Ignore => {}
        }
    }

    /// Evento `cancel` nativo (`Esc`, ou o gesto de voltar do sistema).
    fn on_cancel(&self, e: web_sys::Event) {
        // Quem fecha é o sinal (com a transição); o fechamento nativo é sempre impedido.
        e.prevent_default();
        if self.phase.get_untracked() != Phase::Open {
            return;
        }
        let action = esc_action(self.persistent, self.no_esc_dismiss, self.maximized);
        if action != Dismiss::Hide {
            self.blocked_at.try_set_value(Some(js_sys::Date::now()));
        }
        self.apply(action, true);
    }

    /// `mousedown` no `<dialog>` (o painel e a área ao redor têm `pointer-events` próprios, então
    /// só o que está fora do painel chega com `target` = o `<dialog>`).
    fn on_mousedown(&self, e: ev::MouseEvent) {
        if e.button() != 0 || self.phase.get_untracked() != Phase::Open {
            return;
        }
        let Some(dialog) = self.dialog.get_untracked() else {
            return;
        };
        let on_backdrop = e.target().is_some_and(|t| {
            let t: &JsValue = t.as_ref();
            let d: &JsValue = dialog.as_ref();
            t == d
        });
        if on_backdrop {
            self.apply(
                backdrop_action(self.persistent, self.no_backdrop_dismiss, self.maximized),
                false,
            );
        }
    }

    /// Evento `close` nativo: o que não foi nosso é um fechamento forçado.
    fn on_close(&self) {
        if self.closing.try_get_value().unwrap_or(false) {
            self.closing.try_set_value(false);
            return;
        }
        let Some(dialog) = self.dialog.get_untracked() else {
            return;
        };
        let blocked_at = self.blocked_at.try_get_value().flatten();
        if self.show.get_untracked() && is_forced_close(blocked_at, js_sys::Date::now()) {
            // Recusamos o `Esc`, mas o Chrome fechou assim mesmo: reabre.
            let _ = dialog.show_modal();
            return;
        }
        // Fechado por fora (ex.: `<form method="dialog">`): o sinal acompanha.
        self.clear_timer();
        self.release_lock();
        self.phase.set(Phase::Closed);
        self.show.set(false);
        if let Some(cb) = self.on_hide {
            cb.run(());
        }
    }

    fn cleanup(&self) {
        self.clear_timer();
        self.release_lock();
    }
}

/// Diálogo modal. ≈ `QDialog`, mas sobre o `<dialog>` nativo com `showModal()`: a camada
/// superior, o foco preso, o `inert` no resto da página e o foco devolvido ao fechar são do
/// navegador, não de um portal. O foco inicial vai para o primeiro elemento focável (ou para o
/// que tiver `autofocus`), não para o painel.
///
/// `Esc` e clique fora fecham, salvo `persistent` (ou `no_esc_dismiss` / `no_backdrop_dismiss`
/// para cada um): então o painel "sacode". `show` é o `v-model`; a página rola de novo quando o
/// último diálogo fecha. O painel tem fundo próprio (surface), então qualquer conteúdo serve;
/// um `Card` dentro dele costuma ser `flat`.
///
/// O `<dialog>` não tem nome acessível por si: passe `aria_labelledby` (o `id` do título) ou
/// `aria_label`.
///
/// O servidor renderiza o `<dialog>` fechado; `showModal()` só roda no navegador.
///
/// Ainda não: `Dialog` como plugin (`$q.dialog`), `seamless`, `auto_close`, `no_route_dismiss`,
/// `no_refocus`, `no_focus`, `no_shake`, `allow_focus_outside`, `square`, `backdrop_filter`,
/// `transition_show`/`transition_hide`, os métodos `focus()`/`shake()`, o evento `shake`.
#[component]
pub fn Dialog(
    /// Estado aberto/fechado (`v-model` do Quasar).
    show: RwSignal<bool>,
    /// Não fecha com `Esc` nem com clique fora (o painel sacode).
    #[prop(optional)]
    persistent: bool,
    /// Não fecha com `Esc`.
    #[prop(optional)]
    no_esc_dismiss: bool,
    /// Não fecha com clique fora.
    #[prop(optional)]
    no_backdrop_dismiss: bool,
    #[prop(optional)] position: DialogPosition,
    /// Ocupa a janela inteira.
    #[prop(optional)]
    maximized: bool,
    /// Tenta ter a largura da janela.
    #[prop(optional)]
    full_width: bool,
    /// Tenta ter a altura da janela.
    #[prop(optional)]
    full_height: bool,
    /// Nome acessível do diálogo (use quando não há um título visível).
    #[prop(optional, into)]
    aria_label: Option<String>,
    /// `id` do elemento que dá nome ao diálogo (em geral o título dentro do painel).
    #[prop(optional, into)]
    aria_labelledby: Option<String>,
    /// Depois da transição de entrada.
    #[prop(optional, into)]
    on_show: Option<Callback<()>>,
    /// Depois da transição de saída (o `<dialog>` já fechou).
    #[prop(optional, into)]
    on_hide: Option<Callback<()>>,
    /// `Esc` que vai fechar o diálogo (não dispara com `persistent` nem `no_esc_dismiss`).
    #[prop(optional, into)]
    on_escape_key: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    let body = NodeRef::<html::Div>::new();
    let phase = RwSignal::new(Phase::Closed);
    let shaking = RwSignal::new(false);
    let ctl = Ctl {
        dialog,
        body,
        show,
        phase,
        shaking,
        persistent,
        no_esc_dismiss,
        no_backdrop_dismiss,
        maximized,
        on_show,
        on_hide,
        on_escape_key,
        timer: StoredValue::new_local(None),
        trigger: StoredValue::new_local(None),
        locked: StoredValue::new(false),
        closing: StoredValue::new(false),
        blocked_at: StoredValue::new(None),
    };

    Effect::new(move |_| ctl.sync());
    on_cleanup(move || ctl.cleanup());

    let dialog_class = move || {
        cls([
            "lx-dialog",
            if phase.get() == Phase::Open {
                "lx-dialog-scrim-shown"
            } else {
                "lx-dialog-scrim-hidden"
            },
        ])
    };
    let inner_class = match (maximized, position) {
        (true, _) => "lx-dialog-inner-max",
        (false, DialogPosition::Standard) => "lx-dialog-inner-standard",
        (false, DialogPosition::Top) => "lx-dialog-inner-top",
        (false, DialogPosition::Bottom) => "lx-dialog-inner-bottom",
        (false, DialogPosition::Left) => "lx-dialog-inner-left",
        (false, DialogPosition::Right) => "lx-dialog-inner-right",
    };
    // Geometria do painel (exclusiva): maximizado ignora posição, largura e altura.
    let geometry = match (maximized, position) {
        (true, _) => "lx-dialog-body-max",
        (false, DialogPosition::Standard) => "lx-dialog-body-standard",
        (false, DialogPosition::Top) => "lx-dialog-body-top",
        (false, DialogPosition::Bottom) => "lx-dialog-body-bottom",
        (false, DialogPosition::Left) => "lx-dialog-body-left",
        (false, DialogPosition::Right) => "lx-dialog-body-right",
    };
    let width = match (maximized, full_width) {
        (true, _) => "",
        (false, true) => "lx-dialog-w-full",
        (false, false) => "lx-dialog-w-normal",
    };
    let height = match (maximized, full_height, position) {
        (true, _, _) => "",
        (false, true, _) => "lx-dialog-h-full",
        (false, false, DialogPosition::Top | DialogPosition::Bottom) => "lx-dialog-h-edge",
        (false, false, _) => "lx-dialog-h-pad",
    };
    let body_class = move || {
        let state = if phase.get() == Phase::Open {
            "lx-dialog-open"
        } else {
            match position {
                DialogPosition::Standard => "lx-dialog-enter-standard",
                DialogPosition::Top => "lx-dialog-enter-top",
                DialogPosition::Bottom => "lx-dialog-enter-bottom",
                DialogPosition::Left => "lx-dialog-enter-left",
                DialogPosition::Right => "lx-dialog-enter-right",
            }
        };
        cls([
            "lx-dialog-body",
            geometry,
            width,
            height,
            state,
            if shaking.get() { "lx-dialog-shake" } else { "" },
        ])
    };

    view! {
        <dialog
            node_ref=dialog
            class=dialog_class
            aria-label=aria_label
            aria-labelledby=aria_labelledby
            on:cancel=move |e| ctl.on_cancel(e)
            on:close=move |_| ctl.on_close()
            on:mousedown=move |e| ctl.on_mousedown(e)
        >
            <div class=inner_class>
                <div
                    node_ref=body
                    class=body_class
                    on:animationend=move |e| {
                        if e.animation_name() == "lx-dialog-shake" {
                            shaking.set(false);
                        }
                    }
                >
                    {children()}
                </div>
            </div>
        </dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_closes_unless_persistent_or_no_esc_dismiss() {
        assert_eq!(esc_action(false, false, false), Dismiss::Hide);
        assert_eq!(esc_action(true, false, false), Dismiss::Shake);
        assert_eq!(esc_action(false, true, false), Dismiss::Shake);
        // `no_backdrop_dismiss` não afeta o Esc (nem entra na função)
        assert_eq!(esc_action(false, false, true), Dismiss::Hide);
    }

    #[test]
    fn maximized_never_shakes() {
        assert_eq!(esc_action(true, false, true), Dismiss::Ignore);
        assert_eq!(backdrop_action(true, false, true), Dismiss::Ignore);
    }

    #[test]
    fn backdrop_closes_unless_persistent_or_no_backdrop_dismiss() {
        assert_eq!(backdrop_action(false, false, false), Dismiss::Hide);
        assert_eq!(backdrop_action(true, false, false), Dismiss::Shake);
        assert_eq!(backdrop_action(false, true, false), Dismiss::Shake);
    }

    #[test]
    fn parses_the_longest_transition_duration() {
        assert_eq!(parse_duration_ms("0.25s"), 250);
        assert_eq!(parse_duration_ms("250ms"), 250);
        assert_eq!(parse_duration_ms("0.15s, 0.3s"), 300);
        assert_eq!(parse_duration_ms("0s"), 0);
        assert_eq!(parse_duration_ms(""), 0);
        assert_eq!(parse_duration_ms("auto"), 0);
    }

    #[test]
    fn a_close_right_after_a_blocked_cancel_is_forced() {
        assert!(is_forced_close(Some(1000.0), 1010.0));
        assert!(!is_forced_close(Some(1000.0), 1500.0));
        assert!(!is_forced_close(None, 1000.0));
        // relógio andando para trás não conta
        assert!(!is_forced_close(Some(1000.0), 900.0));
    }

    #[test]
    fn scroll_lock_toggles_on_first_and_last() {
        let mut n = 0;
        assert!(lock_acquire(&mut n));
        assert!(!lock_acquire(&mut n));
        assert!(!lock_release(&mut n));
        assert!(lock_release(&mut n));
        // soltar sem ter segurado não derruba a contagem
        assert!(!lock_release(&mut n));
        assert_eq!(n, 0);
    }
}
