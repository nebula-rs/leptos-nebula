use leptos::prelude::*;

use crate::util::cls;

/// Disposição do termo em relação ao valor na [`DetailList`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DetailListLayout {
    /// Termo à esquerda, numa coluna de largura fixa (`--lx-detail-list-term-width`); o valor
    /// ocupa o resto. Se não couber, o valor desce para baixo do termo (empilha).
    #[default]
    Horizontal,
    /// Termo acima do valor, sempre.
    Vertical,
}

/// Lista de pares termo/valor para o modo leitura (`<dl>`). Sem equivalente no Quasar (API nossa).
///
/// Cada [`DetailItem`] é uma linha (`<div>` com `<dt>` e `<dd>`, válido dentro de `<dl>`). No
/// layout `Horizontal` a linha quebra quando o contêiner fica estreito (não depende da largura
/// da janela). A coluna do termo e a largura mínima do valor vêm dos tokens
/// `--lx-detail-list-term-width` e `--lx-detail-list-value-min-width`.
///
/// Ainda não: colunas múltiplas, termo com conteúdo rico, valor vazio com marcador.
#[component]
pub fn DetailList(
    #[prop(optional)] layout: DetailListLayout,
    /// Linhas mais compactas.
    #[prop(optional)]
    dense: bool,
    /// Linha entre os itens.
    #[prop(optional)]
    separator: bool,
    children: Children,
) -> impl IntoView {
    let layout = match layout {
        DetailListLayout::Horizontal => "lx-detail-list-horizontal",
        DetailListLayout::Vertical => "lx-detail-list-vertical",
    };
    let density = if dense {
        "lx-detail-list-dense"
    } else {
        "lx-detail-list-normal"
    };
    let separator = if separator {
        "lx-detail-list-separated"
    } else {
        ""
    };
    view! { <dl class=cls(["lx-detail-list", layout, density, separator])>{children()}</dl> }
}

/// Item da [`DetailList`]: `term` e o valor (`children`).
#[component]
pub fn DetailItem(#[prop(into)] term: String, children: Children) -> impl IntoView {
    view! {
        <div>
            <dt class="lx-detail-term">{term}</dt>
            <dd class="lx-detail-value">{children()}</dd>
        </div>
    }
}
