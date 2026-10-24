//! `BtnToggle`: grupo de botões com uma opção ativa ("segmented"). ≈ `QBtnToggle`.
//!
//! A regra de clique (escolher, limpar ou ignorar) e a posição de cada botão no grupo ficam em
//! funções puras ([`click_action`], [`position`]), testadas no host.

use leptos::prelude::*;

use super::{BtnDesign, Icon};
use crate::{Color, util::cls};

/// Uma opção de um [`BtnToggle`]. ≈ `{ label, icon, value, disable }` do Quasar.
#[derive(Clone, Debug, PartialEq)]
pub struct BtnToggleOption<T> {
    /// Texto do botão.
    pub label: Option<String>,
    /// Ícone lucide (classe literal `i-lucide-…`), antes do texto.
    pub icon: Option<&'static str>,
    /// O valor que a opção grava no modelo.
    pub value: T,
    /// Opção desabilitada: aparece, mas não é escolhida.
    pub disable: bool,
    /// Nome acessível; necessário quando a opção só tem ícone.
    pub aria_label: Option<String>,
}

impl<T> BtnToggleOption<T> {
    /// Opção com texto.
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: Some(label.into()),
            icon: None,
            value,
            disable: false,
            aria_label: None,
        }
    }

    /// Opção só com ícone. Dê um nome com [`with_aria_label`](Self::with_aria_label).
    pub fn icon_only(icon: &'static str, value: T) -> Self {
        Self {
            label: None,
            icon: Some(icon),
            value,
            disable: false,
            aria_label: None,
        }
    }

    /// Esta mesma opção, com um ícone.
    #[must_use]
    pub fn with_icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Esta mesma opção, com nome acessível.
    #[must_use]
    pub fn with_aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Esta mesma opção, desabilitada.
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.disable = true;
        self
    }
}

/// O que um clique num botão faz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClickAction {
    /// Grava o valor do botão.
    Select,
    /// Limpa o modelo (`clearable` e o botão já é o ativo).
    Clear,
    /// Nada (`readonly`, ou o botão já é o ativo sem `clearable`).
    Ignore,
}

/// Regra do clique: `readonly` ignora; clicar no ativo limpa só com `clearable`; senão escolhe.
pub(crate) fn click_action(is_active: bool, clearable: bool, readonly: bool) -> ClickAction {
    match (readonly, is_active, clearable) {
        (true, _, _) | (false, true, false) => ClickAction::Ignore,
        (false, true, true) => ClickAction::Clear,
        (false, false, _) => ClickAction::Select,
    }
}

/// Posição de um botão no grupo (decide em quais pontas há raio).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Position {
    Only,
    First,
    Middle,
    Last,
}

/// Posição do botão `index` num grupo de `len` botões.
pub(crate) fn position(index: usize, len: usize) -> Position {
    match (index == 0, index + 1 >= len) {
        (true, true) => Position::Only,
        (true, false) => Position::First,
        (false, true) => Position::Last,
        (false, false) => Position::Middle,
    }
}

/// Grupo de botões que escolhe uma opção. ≈ `QBtnToggle`.
///
/// `value` é o `v-model`: `Some(valor)` da opção ativa ou `None`. Cada botão leva `aria-pressed`;
/// o grupo é `role="group"` (dê um nome com `aria_label`). Teclado: cada botão é um `<button>`
/// (`Tab`, `Enter`, `Espaço`); não há navegação por setas.
///
/// **Cor:** sem `color` e `toggle_color`, as cores vêm dos tokens `--lx-btn-toggle-*` do tema (no
/// scandi, o aspecto "segmented": trilho `surface-muted` e o ativo em `surface` com sombra
/// leve). `color` pinta os botões inativos com um papel de cor; `toggle_color` pinta o ativo
/// (nos designs `Outline` e `Flat`, sem `toggle_color` o ativo usa `Primary`, e sem `color` os
/// inativos usam `Neutral`).
///
/// **Designs:** `Unelevated` (padrão) tem trilho; `Outline` e `Flat` colam os botões.
///
/// Ainda não: `text_color`/`toggle_text_color` (o texto vem do par `on-*` do papel de cor),
/// `outline`/`flat`/`push`/`glossy` como props (use `design`), `size`, `padding`, `no_caps`,
/// `no_wrap`, `align`, `stack`, `stretch`, `vertical`, `ripple`, `name` (campo oculto no
/// `<form>`), slots por opção, `attrs` por opção, `toggle_text_color`, evento `click`.
#[component]
pub fn BtnToggle<T>(
    /// A opção ativa (`v-model`).
    value: RwSignal<Option<T>>,
    /// As opções.
    #[prop(into)]
    options: Signal<Vec<BtnToggleOption<T>>>,
    /// Cor dos botões inativos. Sem ela, vale o tema.
    #[prop(optional)]
    color: Option<Color>,
    /// Cor do botão ativo. Sem ela, vale o tema (`Primary` em `Outline`/`Flat`).
    #[prop(optional)]
    toggle_color: Option<Color>,
    #[prop(optional)] design: BtnDesign,
    /// Ocupa toda a largura, com botões de larguras iguais.
    #[prop(optional)]
    spread: bool,
    #[prop(optional)] dense: bool,
    /// Cantos em pílula.
    #[prop(optional)]
    rounded: bool,
    /// Clicar no botão ativo limpa o modelo (`None`).
    #[prop(optional)]
    clearable: bool,
    #[prop(optional, into)] disable: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    /// Nome acessível do grupo.
    #[prop(optional, into)]
    aria_label: Option<String>,
    /// O botão ativo foi clicado com `clearable`; recebe o valor que havia.
    #[prop(optional, into)]
    on_clear: Option<Callback<Option<T>>>,
) -> impl IntoView
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    let width = if spread {
        "lx-btn-toggle-spread"
    } else {
        "lx-btn-toggle-fit"
    };
    let frame = match (design, rounded) {
        (BtnDesign::Unelevated, false) => "lx-btn-toggle-track",
        (BtnDesign::Unelevated, true) => "lx-btn-toggle-track-rounded",
        (BtnDesign::Outline, _) => "lx-btn-toggle-joined-outline",
        (BtnDesign::Flat, _) => "lx-btn-toggle-joined-flat",
    };
    // O grupo carrega a cor dos inativos (o item a herda em `--lx-tone`). `Outline`/`Flat` sempre
    // têm cor; o `Unelevated` sem `color` lê os tokens do tema.
    let group_tone = match (color, design) {
        (Some(c), _) => c.tone_class(),
        (None, BtnDesign::Unelevated) => "",
        (None, _) => Color::Neutral.tone_class(),
    };
    let size = if dense {
        "lx-btn-toggle-size-dense"
    } else {
        "lx-btn-toggle-size-normal"
    };
    let themed_off = color.is_none();
    let themed_on = toggle_color.is_none() && design == BtnDesign::Unelevated;
    let on_tone = if themed_on {
        ""
    } else {
        toggle_color.unwrap_or_default().tone_class()
    };

    let buttons = move || {
        let list = options.get();
        let len = list.len();
        list.into_iter()
            .enumerate()
            .map(|(index, opt)| {
                let BtnToggleOption {
                    label,
                    icon,
                    value: opt_value,
                    disable: opt_disable,
                    aria_label,
                } = opt;
                let pos = match (position(index, len), rounded) {
                    (Position::Only, false) => "lx-btn-toggle-pos-only",
                    (Position::First, false) => "lx-btn-toggle-pos-first",
                    (Position::Middle, false) => "lx-btn-toggle-pos-middle",
                    (Position::Last, false) => "lx-btn-toggle-pos-last",
                    (Position::Only, true) => "lx-btn-toggle-pos-only-rounded",
                    (Position::First, true) => "lx-btn-toggle-pos-first-rounded",
                    (Position::Middle, true) => "lx-btn-toggle-pos-middle-rounded",
                    (Position::Last, true) => "lx-btn-toggle-pos-last-rounded",
                };
                let is_active = {
                    let v = opt_value.clone();
                    move || value.with(|cur| cur.as_ref() == Some(&v))
                };
                let is_disabled = move || disable.get() || opt_disable;
                let class = {
                    let is_active = is_active.clone();
                    move || {
                        let state =
                            item_state(design, is_active(), is_disabled(), themed_off, themed_on);
                        let tone = if is_active() { on_tone } else { "" };
                        cls(["lx-btn-toggle-item", size, pos, state, tone])
                    }
                };
                let onclick = {
                    let is_active = is_active.clone();
                    move |_| match click_action(is_active(), clearable, readonly.get_untracked()) {
                        ClickAction::Select => value.set(Some(opt_value.clone())),
                        ClickAction::Clear => {
                            let previous = value.get_untracked();
                            value.set(None);
                            if let Some(cb) = on_clear {
                                cb.run(previous);
                            }
                        }
                        ClickAction::Ignore => {}
                    }
                };
                view! {
                    <button
                        type="button"
                        class=class
                        aria-pressed=move || if is_active() { "true" } else { "false" }
                        aria-label=aria_label
                        disabled=is_disabled
                        on:click=onclick
                    >
                        {icon.map(|name| view! { <Icon name=name /> })}
                        {label.map(|l| view! { <span>{l}</span> })}
                    </button>
                }
            })
            .collect_view()
    };

    view! {
        <div
            class=cls(["lx-btn-toggle", width, frame, group_tone])
            role="group"
            aria-label=aria_label
        >
            {buttons}
        </div>
    }
}

/// Shortcut completo do estado do botão (design × ativo × desabilitado × cor do tema ou do papel).
fn item_state(
    design: BtnDesign,
    active: bool,
    disabled: bool,
    themed_off: bool,
    themed_on: bool,
) -> &'static str {
    match (design, active, disabled) {
        (BtnDesign::Unelevated, false, false) if themed_off => "lx-btn-toggle-item-unelevated-off",
        (BtnDesign::Unelevated, false, true) if themed_off => {
            "lx-btn-toggle-item-unelevated-off-disabled"
        }
        (BtnDesign::Unelevated, false, false) => "lx-btn-toggle-item-unelevated-off-tone",
        (BtnDesign::Unelevated, false, true) => "lx-btn-toggle-item-unelevated-off-tone-disabled",
        (BtnDesign::Unelevated, true, false) if themed_on => "lx-btn-toggle-item-unelevated-on",
        (BtnDesign::Unelevated, true, true) if themed_on => {
            "lx-btn-toggle-item-unelevated-on-disabled"
        }
        (BtnDesign::Unelevated, true, false) => "lx-btn-toggle-item-unelevated-on-tone",
        (BtnDesign::Unelevated, true, true) => "lx-btn-toggle-item-unelevated-on-tone-disabled",
        (BtnDesign::Outline, false, false) => "lx-btn-toggle-item-outline-off",
        (BtnDesign::Outline, false, true) => "lx-btn-toggle-item-outline-off-disabled",
        (BtnDesign::Outline, true, false) => "lx-btn-toggle-item-outline-on",
        (BtnDesign::Outline, true, true) => "lx-btn-toggle-item-outline-on-disabled",
        (BtnDesign::Flat, false, false) => "lx-btn-toggle-item-flat-off",
        (BtnDesign::Flat, false, true) => "lx-btn-toggle-item-flat-off-disabled",
        (BtnDesign::Flat, true, false) => "lx-btn-toggle-item-flat-on",
        (BtnDesign::Flat, true, true) => "lx-btn-toggle-item-flat-on-disabled",
    }
}

#[cfg(test)]
mod tests {
    use super::{BtnToggleOption, ClickAction, Position, click_action, position};

    #[test]
    fn click_selects_clears_or_ignores() {
        assert_eq!(click_action(false, false, false), ClickAction::Select);
        assert_eq!(click_action(false, true, false), ClickAction::Select);
        assert_eq!(click_action(true, false, false), ClickAction::Ignore);
        assert_eq!(click_action(true, true, false), ClickAction::Clear);
        for (active, clearable) in [(false, false), (false, true), (true, false), (true, true)] {
            assert_eq!(click_action(active, clearable, true), ClickAction::Ignore);
        }
    }

    #[test]
    fn positions_mark_the_ends() {
        assert_eq!(position(0, 1), Position::Only);
        assert_eq!(position(0, 3), Position::First);
        assert_eq!(position(1, 3), Position::Middle);
        assert_eq!(position(2, 3), Position::Last);
        assert_eq!(position(1, 2), Position::Last);
    }

    #[test]
    fn option_builders() {
        let o = BtnToggleOption::new("Dia", 1)
            .with_icon("i-lucide-sun")
            .disabled();
        assert_eq!(
            (o.label.as_deref(), o.icon, o.value, o.disable),
            (Some("Dia"), Some("i-lucide-sun"), 1, true)
        );
        let i = BtnToggleOption::icon_only("i-lucide-moon", 2).with_aria_label("Noite");
        assert_eq!((i.label, i.aria_label.as_deref()), (None, Some("Noite")));
    }
}
