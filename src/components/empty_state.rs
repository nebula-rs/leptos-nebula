use leptos::prelude::*;

use crate::util::cls;

/// Slot `actions` do [`EmptyState`]: `<EmptyStateActions slot:actions>…</EmptyStateActions>`.
#[slot]
pub struct EmptyStateActions {
    children: ChildrenFn,
}

/// Estado vazio: ícone, título, dica e ações, centralizados. Sem equivalente no Quasar (API nossa).
///
/// As cores são as `muted` do tema (tokens `--lx-empty-state-*`).
///
/// Ainda não: `Color`, tamanhos, imagem no lugar do ícone.
#[component]
pub fn EmptyState(
    /// Classe **literal** de ícone lucide, ex.: `"i-lucide-inbox"`.
    #[prop(optional)]
    icon: Option<&'static str>,
    #[prop(into)] title: String,
    /// Texto de apoio, abaixo do título.
    #[prop(optional, into)]
    hint: Option<String>,
    #[prop(optional)] actions: Option<EmptyStateActions>,
) -> impl IntoView {
    view! {
        <div class="lx-empty-state">
            {icon
                .map(|name| {
                    view! { <span class=cls(["lx-empty-state-icon", name]) aria-hidden="true"></span> }
                })}
            <strong class="lx-empty-state-title">{title}</strong>
            {hint.map(|h| view! { <span class="lx-empty-state-hint">{h}</span> })}
            {actions
                .map(|a| view! { <div class="lx-empty-state-actions">{(a.children)()}</div> })}
        </div>
    }
}
