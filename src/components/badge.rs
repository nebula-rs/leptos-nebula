use leptos::prelude::*;

use crate::{Color, util::cls};

/// Alinhamento vertical do selo (Quasar: `align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeAlign {
    #[default]
    Middle,
    Top,
    Bottom,
}

/// Selo. ≈ `QBadge`.
///
/// Ainda não: `floating`, `multi_line`, `transparent`, `rounded`, `text_color`.
#[component]
pub fn Badge(
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] color: Color,
    #[prop(optional)] outline: bool,
    #[prop(optional)] align: BadgeAlign,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let design = if outline {
        "lx-badge-outline"
    } else {
        "lx-badge-filled"
    };
    let align = match align {
        BadgeAlign::Middle => "lx-badge-align-middle",
        BadgeAlign::Top => "lx-badge-align-top",
        BadgeAlign::Bottom => "lx-badge-align-bottom",
    };
    view! {
        <span class=cls(["lx-badge", design, align, color.tone_class()])>
            {label} {children.map(|c| c())}
        </span>
    }
}
