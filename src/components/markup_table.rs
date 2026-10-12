use leptos::prelude::*;

use crate::util::cls;

/// Separadores de células (Quasar: `separator`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkupSeparator {
    #[default]
    Horizontal,
    Vertical,
    Cell,
    None,
}

/// Moldura para um `<table>` nativo. ≈ `QMarkupTable`.
///
/// Ainda não: `dark`, `square`, `wrap_cells` só como `bool` (sem tamanho).
#[component]
pub fn MarkupTable(
    #[prop(optional)] dense: bool,
    #[prop(optional)] flat: bool,
    #[prop(optional)] bordered: bool,
    #[prop(optional)] separator: MarkupSeparator,
    #[prop(optional)] wrap_cells: bool,
    /// Um `<table>` nativo.
    children: Children,
) -> impl IntoView {
    let cells = if dense {
        "lx-mt-cells-dense"
    } else {
        "lx-mt-cells"
    };
    let separator = match separator {
        MarkupSeparator::Horizontal => "lx-mt-sep-horizontal",
        MarkupSeparator::Vertical => "lx-mt-sep-vertical",
        MarkupSeparator::Cell => "lx-mt-sep-cell",
        MarkupSeparator::None => "",
    };
    let elevation = if flat { "lx-mt-flat" } else { "lx-mt-raised" };
    let border = if bordered {
        "lx-mt-bordered"
    } else {
        "lx-mt-unbordered"
    };
    let wrap = if wrap_cells {
        "lx-mt-wrap"
    } else {
        "lx-mt-nowrap"
    };
    view! { <div class=cls(["lx-mt", cells, separator, elevation, border, wrap])>{children()}</div> }
}
