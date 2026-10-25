use leptos::prelude::*;

use crate::{
    Color,
    util::{cls, next_id},
};

/// Grade responsiva de [`Stat`]: tantas colunas quantas couberem, cada uma com no mínimo
/// `--lx-stat-grid-min-width`. Sem equivalente no Quasar (API nossa).
#[component]
pub fn StatGrid(children: Children) -> impl IntoView {
    view! { <div class="lx-stat-grid">{children()}</div> }
}

/// Indicador: rótulo, valor em destaque, dica e ícone. Sem equivalente no Quasar (API nossa).
///
/// Não tem fundo nem borda: ponha dentro de um `Card` (ou direto na página). O valor vem de
/// `value` (texto fixo ou reativo) ou, para conteúdo rico, de `children`, que tem precedência.
/// `color` tinge o valor e o ícone; sem ele o valor usa a cor do texto e o ícone a `muted`.
/// O conjunto é um `role="group"` nomeado pelo rótulo e descrito pela dica.
///
/// Ainda não: tendência (seta/variação), `loading`, link/clique, formatação de número.
#[component]
pub fn Stat(
    #[prop(into)] label: String,
    #[prop(optional, into)] value: MaybeProp<String>,
    #[prop(optional, into)] hint: Option<String>,
    /// Classe **literal** de ícone lucide, ex.: `"i-lucide-database"`.
    #[prop(optional)]
    icon: Option<&'static str>,
    /// Tom do valor e do ícone.
    #[prop(optional)]
    color: Option<Color>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let label_id = next_id("lx-stat-label");
    let hint_id = next_id("lx-stat-hint");
    let (tone, value_class, icon_class) = match color {
        Some(c) => (c.tone_class(), "lx-stat-value-tone", "lx-stat-icon-tone"),
        None => ("", "lx-stat-value-plain", "lx-stat-icon-plain"),
    };
    let described_by = hint.as_ref().map(|_| hint_id.clone());
    let value = match children {
        Some(c) => c().into_any(),
        None => (move || value.get()).into_any(),
    };
    let labelled_by = label_id.clone();
    view! {
        <div
            class=cls(["lx-stat", tone])
            role="group"
            aria-labelledby=labelled_by
            aria-describedby=described_by
        >
            <div class="lx-stat-body">
                <span class="lx-stat-label" id=label_id>
                    {label}
                </span>
                <span class=cls(["lx-stat-value", value_class])>{value}</span>
                {hint.map(|h| view! { <span class="lx-stat-hint" id=hint_id>{h}</span> })}
            </div>
            {icon
                .map(|name| {
                    // A cor fica no invólucro: a classe do ícone lucide define `color: inherit`.
                    view! {
                        <span class=cls(["lx-stat-icon", icon_class])>
                            <span class=name aria-hidden="true"></span>
                        </span>
                    }
                })}
        </div>
    }
}
