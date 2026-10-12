use leptos::prelude::*;

use crate::{Color, util::cls};

/// Slot `avatar` do [`Banner`]: `<BannerAvatar slot:avatar>…</BannerAvatar>`.
#[slot]
pub struct BannerAvatar {
    children: ChildrenFn,
}

/// Slot `action` do [`Banner`]: `<BannerAction slot:action>…</BannerAction>`.
#[slot]
pub struct BannerAction {
    children: ChildrenFn,
}

/// Faixa de aviso. ≈ `QBanner`.
///
/// Ainda não: `inline_actions`, `dark`.
/// `color` padrão é `Neutral` (o Quasar não tem cor padrão).
#[component]
pub fn Banner(
    #[prop(default = Color::Neutral)] color: Color,
    #[prop(optional)] dense: bool,
    #[prop(optional)] rounded: bool,
    #[prop(optional)] avatar: Option<BannerAvatar>,
    #[prop(optional)] action: Option<BannerAction>,
    children: Children,
) -> impl IntoView {
    let density = if dense {
        "lx-banner-dense"
    } else {
        "lx-banner-normal"
    };
    let shape = if rounded {
        "lx-banner-rounded"
    } else {
        "lx-banner-square"
    };
    view! {
        <div class=cls(["lx-banner", density, shape, color.tone_class()]) role="alert">
            {avatar.map(|a| view! { <div class="lx-banner-avatar">{(a.children)()}</div> })}
            <div class="lx-banner-content">{children()}</div>
            {action.map(|a| view! { <div class="lx-banner-actions">{(a.children)()}</div> })}
        </div>
    }
}
