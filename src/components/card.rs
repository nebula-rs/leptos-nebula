use leptos::prelude::*;

use crate::{Align, util::cls};

/// Cartão. ≈ `QCard`.
///
/// Ainda não: `dark`, `tag` (renderiza sempre `<div>`).
#[component]
pub fn Card(
    #[prop(optional)] flat: bool,
    #[prop(optional)] bordered: bool,
    #[prop(optional)] square: bool,
    children: Children,
) -> impl IntoView {
    let elevation = if flat {
        "lx-card-flat"
    } else {
        "lx-card-raised"
    };
    let border = if bordered {
        "lx-card-bordered"
    } else {
        "lx-card-unbordered"
    };
    let shape = if square {
        "lx-card-square"
    } else {
        "lx-card-round"
    };
    view! { <div class=cls(["lx-card", elevation, border, shape])>{children()}</div> }
}

/// Seção do cartão. ≈ `QCardSection`.
///
/// Ainda não: `tag`.
#[component]
pub fn CardSection(#[prop(optional)] horizontal: bool, children: Children) -> impl IntoView {
    let class = if horizontal {
        "lx-card-section-horizontal"
    } else {
        "lx-card-section-vertical"
    };
    view! { <div class=class>{children()}</div> }
}

/// Ações do cartão. ≈ `QCardActions`.
#[component]
pub fn CardActions(
    #[prop(optional)] align: Align,
    #[prop(optional)] vertical: bool,
    children: Children,
) -> impl IntoView {
    let direction = if vertical {
        "lx-card-actions-column"
    } else {
        "lx-card-actions-row"
    };
    let align = match align {
        Align::Left => "lx-card-actions-align-left",
        Align::Center => "lx-card-actions-align-center",
        Align::Right => "lx-card-actions-align-right",
        Align::Between => "lx-card-actions-align-between",
        Align::Around => "lx-card-actions-align-around",
        Align::Evenly => "lx-card-actions-align-evenly",
        Align::Justify => "lx-card-actions-align-justify",
    };
    view! { <div class=cls([direction, align])>{children()}</div> }
}
