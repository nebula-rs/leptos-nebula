use leptos::prelude::*;

use crate::util::cls;

/// Linha separadora. ≈ `QSeparator`.
///
/// Ainda não: `color`, `size`, `spaced` com tamanho, `inset` como `"item"`/`"item-thumbnail"`.
#[component]
pub fn Separator(
    #[prop(optional)] vertical: bool,
    #[prop(optional)] spaced: bool,
    #[prop(optional)] inset: bool,
) -> impl IntoView {
    let (base, spaced_class, inset_class) = if vertical {
        (
            "lx-separator-v",
            "lx-separator-spaced-v",
            "lx-separator-inset-v",
        )
    } else {
        (
            "lx-separator-h",
            "lx-separator-spaced-h",
            "lx-separator-inset-h",
        )
    };
    let class = cls([
        base,
        if spaced { spaced_class } else { "" },
        if inset { inset_class } else { "" },
    ]);
    let orientation = if vertical { "vertical" } else { "horizontal" };
    view! { <div class=class role="separator" aria-orientation=orientation></div> }
}
