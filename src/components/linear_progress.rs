//! `LinearProgress`: barra de progresso linear. ≈ `QLinearProgress`.
//!
//! Largura por `width` em porcentagem (determinado, buffer, segmentos); o indeterminado e o
//! `query` usam duas barras animadas por `@keyframes` (em `style/base.css`). A conta das larguras
//! dos segmentos é uma função pura, testada no host. O componente tem altura própria (`size`) e
//! largura total do pai: não depende de altura do pai.

use leptos::prelude::*;

use crate::{Color, util::cls};

/// Um trecho do `LinearProgress` segmentado: `value` é a fração (0..1) do **total** que o trecho
/// ocupa; `color` é o papel de cor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressSegment {
    pub value: f64,
    pub color: Color,
}

impl ProgressSegment {
    pub fn new(value: f64, color: Color) -> Self {
        Self { value, color }
    }
}

// ── Lógica pura ────────────────────────────────────────────────────────────

/// Limita a 0..1; `NaN` conta como 0.
fn clamp01(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

/// Fração ocupada por cada segmento, na ordem dada. Valores negativos ou `NaN` viram 0 e a soma é
/// limitada a 1: o segmento que ultrapassa o total é cortado e os seguintes ficam em 0.
fn segment_fractions(segments: &[ProgressSegment]) -> Vec<f64> {
    let mut left = 1.0_f64;
    segments
        .iter()
        .map(|s| {
            let f = clamp01(s.value).min(left);
            left -= f;
            f
        })
        .collect()
}

/// Fração como largura CSS (`"35.000%"`).
fn pct(fraction: f64) -> String {
    format!("{:.3}%", clamp01(fraction) * 100.0)
}

/// Valor de `aria-valuenow`: a fração, sem zeros à direita.
fn now_attr(fraction: f64) -> String {
    let s = format!("{:.4}", clamp01(fraction));
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Barra de progresso linear. ≈ `QLinearProgress`.
///
/// `value` e `buffer` vão de 0 a 1. Com `segments` (extensão nossa, sem equivalente no Quasar) as
/// barras são empilhadas na ordem dada, a soma é limitada a 1 e `value` é **ignorado**;
/// `indeterminate` e `query` têm precedência sobre `value` e `segments`. `role="progressbar"`
/// com `aria-valuemin`/`max`/`now`; sem `aria-valuenow` quando indeterminado ou `query`.
///
/// Diferenças do Quasar: `animation_speed` (ms) regula só as animações indeterminadas, e a
/// transição do valor usa o token `--lx-linear-progress-transition-duration`; `buffer` desenha
/// uma faixa sobre a trilha (a trilha continua inteira). Mais `aria_label` (nosso).
#[component]
pub fn LinearProgress(
    /// Valor, de 0 a 1.
    #[prop(optional, into)]
    value: Signal<f64>,
    /// Faixa de buffer, de 0 a 1, entre a trilha e a barra.
    #[prop(optional, into)]
    buffer: Option<Signal<f64>>,
    /// Papel de cor da barra.
    #[prop(optional)]
    color: Color,
    /// Papel de cor da trilha; sem ele, o token `--lx-linear-progress-track-bg`.
    #[prop(optional)]
    track_color: Option<Color>,
    /// Altura CSS da barra.
    #[prop(default = "4px")]
    size: &'static str,
    /// Pontas arredondadas.
    #[prop(optional, into)]
    rounded: Signal<bool>,
    /// Cresce da direita para a esquerda.
    #[prop(optional, into)]
    reverse: Signal<bool>,
    /// Animação contínua, sem valor.
    #[prop(optional, into)]
    indeterminate: Signal<bool>,
    /// Como `indeterminate`, mas da direita para a esquerda.
    #[prop(optional, into)]
    query: Signal<bool>,
    /// Listras sobre a barra (não aparecem no indeterminado).
    #[prop(optional, into)]
    stripe: Signal<bool>,
    /// Sem transição ao mudar o valor.
    #[prop(optional, into)]
    instant_feedback: Signal<bool>,
    /// Duração, em ms, de um ciclo da animação indeterminada.
    #[prop(default = 2100)]
    animation_speed: u32,
    /// Barras empilhadas (acertos + erros…); ignora `value`.
    #[prop(optional, into)]
    segments: Option<Signal<Vec<ProgressSegment>>>,
    /// Texto para leitores de tela.
    #[prop(optional)]
    aria_label: Option<&'static str>,
) -> impl IntoView {
    let motion = move || indeterminate.get() || query.get();
    let mirrored = move || reverse.get() || query.get();
    // Fração concluída (valor ou soma dos segmentos).
    let fractions = move || {
        segments
            .map(|s| s.with(|v| segment_fractions(v)))
            .unwrap_or_default()
    };
    let total = move || match segments {
        Some(_) => fractions().iter().sum::<f64>(),
        None => clamp01(value.get()),
    };

    let root_class = move || {
        cls([
            "lx-linear-progress",
            if rounded.get() {
                "lx-linear-progress-rounded"
            } else {
                "lx-linear-progress-square"
            },
            color.tone_class(),
        ])
    };
    let anim = move || {
        if instant_feedback.get() {
            ""
        } else {
            "lx-linear-progress-animated"
        }
    };
    let side = move || {
        if mirrored() {
            "lx-linear-progress-end"
        } else {
            "lx-linear-progress-start"
        }
    };
    let track_class = match track_color {
        Some(c) => cls([
            "lx-linear-progress-layer",
            "lx-linear-progress-track-tone",
            c.tone_class(),
        ]),
        None => cls(["lx-linear-progress-layer", "lx-linear-progress-track"]),
    };

    view! {
        <div
            class=root_class
            style=format!("height: {size}; --lx-linear-progress-speed: {animation_speed}ms")
            role="progressbar"
            aria-label=aria_label
            aria-valuemin="0"
            aria-valuemax="1"
            aria-valuenow=move || (!motion()).then(|| now_attr(total()))
        >
            <div class=track_class></div>
            {move || {
                buffer
                    .filter(|_| !motion())
                    .map(|b| {
                        view! {
                            <div
                                class=move || {
                                    cls(["lx-linear-progress-buffer", anim(), side()])
                                }
                                style:width=move || pct(b.get())
                            ></div>
                        }
                    })
            }}
            {move || {
                if motion() {
                    let dir = if mirrored() {
                        "lx-linear-progress-motion-reverse"
                    } else {
                        "lx-linear-progress-motion"
                    };
                    view! {
                        <div class=dir>
                            <div class="lx-linear-progress-indeterminate"></div>
                            <div class="lx-linear-progress-indeterminate-short"></div>
                        </div>
                    }
                        .into_any()
                } else if let Some(segs) = segments {
                    let dir = if mirrored() {
                        "lx-linear-progress-segments-reverse"
                    } else {
                        "lx-linear-progress-segments"
                    };
                    view! {
                        <div class=dir>
                            {move || {
                                let fr = fractions();
                                segs.with(|v| {
                                    v.iter()
                                        .zip(fr)
                                        .map(|(s, f)| {
                                            view! {
                                                <div
                                                    class=cls([
                                                        "lx-linear-progress-segment",
                                                        anim(),
                                                        s.color.tone_class(),
                                                    ])
                                                    style:width=pct(f)
                                                ></div>
                                            }
                                        })
                                        .collect_view()
                                })
                            }}
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div
                            class=move || cls(["lx-linear-progress-bar", anim(), side()])
                            style:width=move || pct(total())
                        ></div>
                    }
                        .into_any()
                }
            }}
            {move || {
                (stripe.get() && !motion())
                    .then(|| {
                        view! {
                            <div
                                class=move || cls(["lx-linear-progress-stripe", anim(), side()])
                                style:width=move || pct(total())
                            ></div>
                        }
                    })
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(v: f64) -> ProgressSegment {
        ProgressSegment::new(v, Color::Positive)
    }

    #[test]
    fn segments_are_stacked_in_order() {
        assert_eq!(segment_fractions(&[seg(0.2), seg(0.3)]), vec![0.2, 0.3]);
    }

    #[test]
    fn sum_is_capped_at_one() {
        let f = segment_fractions(&[seg(0.7), seg(0.6), seg(0.2)]);
        assert_eq!(f[0], 0.7);
        assert!((f[1] - 0.3).abs() < 1e-12);
        assert!(f[2].abs() < 1e-12);
        assert!((f.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_values_count_as_zero() {
        assert_eq!(
            segment_fractions(&[seg(f64::NAN), seg(-0.5), seg(0.4)]),
            vec![0.0, 0.0, 0.4]
        );
        assert_eq!(segment_fractions(&[seg(5.0)]), vec![1.0]);
        assert!(segment_fractions(&[]).is_empty());
    }

    #[test]
    fn formats_width_and_aria_value() {
        assert_eq!(pct(0.35), "35.000%");
        assert_eq!(pct(2.0), "100.000%");
        assert_eq!(pct(f64::NAN), "0.000%");
        assert_eq!(now_attr(0.35), "0.35");
        assert_eq!(now_attr(0.0), "0");
        assert_eq!(now_attr(1.5), "1");
    }
}
