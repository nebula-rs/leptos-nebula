use leptos::prelude::*;

use super::{Btn, BtnDesign, Icon};
use crate::{Color, Mode, use_theme};

/// Alterna claro → escuro → automático pela store de tema (`provide_theme()` na raiz do app).
/// Componente nosso (não existe no Quasar).
#[component]
pub fn ThemeSwitcher() -> impl IntoView {
    let store = use_theme();
    view! {
        <Btn
            round=true
            dense=true
            design=BtnDesign::Flat
            color=Color::Neutral
            aria_label="Alternar tema (claro, escuro, automático)"
            on_click=move |_| store.mode.update(|m| *m = m.next())
        >
            {move || match store.mode.get() {
                Mode::Light => view! { <Icon name="i-lucide-sun" /> }.into_any(),
                Mode::Dark => view! { <Icon name="i-lucide-moon" /> }.into_any(),
                Mode::Auto => view! { <Icon name="i-lucide-monitor" /> }.into_any(),
            }}
        </Btn>
    }
}
