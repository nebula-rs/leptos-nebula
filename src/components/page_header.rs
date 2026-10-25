use leptos::prelude::*;

use crate::{Btn, BtnDesign, Color};

/// Slot `actions` do [`PageHeader`]: `<PageHeaderActions slot:actions>…</PageHeaderActions>`.
#[slot]
pub struct PageHeaderActions {
    children: ChildrenFn,
}

/// Cabeçalho de página: título, subtítulo, ações à direita e botão de voltar opcional.
/// Sem equivalente no Quasar (API nossa).
///
/// O título é um `<h1>` (`level` 2 ou 3 para páginas com outro `<h1>`; fora de 1–3 vale o mais
/// próximo). Título e subtítulo aceitam texto fixo ou reativo. `back` mostra um botão redondo
/// com a seta; seu nome acessível é `back_label` ("Voltar"). Em tela estreita as ações descem
/// para baixo do título.
///
/// Ainda não: ícone/avatar no título, `back` como link (`href`), migalhas (breadcrumbs).
#[component]
pub fn PageHeader(
    #[prop(into)] title: Signal<String>,
    #[prop(optional, into)] subtitle: MaybeProp<String>,
    /// Nível do título: 1 (padrão) a 3.
    #[prop(default = 1)]
    level: u8,
    /// Mostra o botão voltar e chama o callback ao clicar.
    #[prop(optional, into)]
    back: Option<Callback<()>>,
    /// Nome acessível do botão voltar.
    #[prop(default = "Voltar".to_string(), into)]
    back_label: String,
    #[prop(optional)] actions: Option<PageHeaderActions>,
) -> impl IntoView {
    let heading = match level.clamp(1, 3) {
        1 => view! { <h1 class="lx-page-header-title">{move || title.get()}</h1> }.into_any(),
        2 => view! { <h2 class="lx-page-header-title">{move || title.get()}</h2> }.into_any(),
        _ => view! { <h3 class="lx-page-header-title">{move || title.get()}</h3> }.into_any(),
    };
    let back = back.map(|cb| {
        view! {
            <Btn
                round=true
                design=BtnDesign::Flat
                color=Color::Neutral
                icon="i-lucide-arrow-left"
                aria_label=back_label
                on_click=Callback::new(move |_| cb.run(()))
            />
        }
    });
    view! {
        <header class="lx-page-header">
            {back}
            <div class="lx-page-header-text">
                {heading}
                {move || {
                    subtitle
                        .get()
                        .map(|s| view! { <p class="lx-page-header-subtitle">{s}</p> })
                }}
            </div>
            {actions
                .map(|a| view! { <div class="lx-page-header-actions">{(a.children)()}</div> })}
        </header>
    }
}
