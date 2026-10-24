use leptos::prelude::*;

use crate::{Color, util::cls};

/// Indicador de carregamento circular. ≈ `QSpinner` (só o padrão; sem `QSpinnerDots` etc.).
///
/// Sem `color`, herda `currentColor`; sem `size`, mede `1em`: dentro de um `Btn`, de um `Field`
/// ou de um texto, ele acompanha a cor e o tamanho do entorno. `role="status"` com
/// `aria_label` (padrão "Carregando"). A animação respeita `prefers-reduced-motion`
/// (rotação mais lenta, não parada).
///
/// Ainda não: variantes (`dots`, `bars`…) e `size` como enum de tamanhos.
#[component]
pub fn Spinner(
    /// Tamanho CSS (largura e altura), ex.: `"2em"`, `"32px"`.
    #[prop(default = "1em")]
    size: &'static str,
    /// Papel de cor do tema; sem ele, `currentColor`.
    #[prop(optional)]
    color: Option<Color>,
    /// Espessura do traço, em unidades do `viewBox` (50×50).
    #[prop(default = 5.0)]
    thickness: f32,
    /// Texto para leitores de tela.
    #[prop(default = "Carregando")]
    aria_label: &'static str,
) -> impl IntoView {
    let class = match color {
        Some(c) => cls(["lx-spinner", "lx-spinner-tone", c.tone_class()]),
        None => cls(["lx-spinner"]),
    };
    view! {
        <svg
            class=class
            width=size
            height=size
            viewBox="0 0 50 50"
            role="status"
            aria-label=aria_label
        >
            <circle
                class="lx-spinner-path"
                cx="25"
                cy="25"
                r="20"
                fill="none"
                stroke="currentColor"
                stroke-width=thickness.to_string()
                stroke-miterlimit="10"
            ></circle>
        </svg>
    }
}
