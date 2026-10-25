//! `ExpansionItem`: item de lista que abre e fecha um conteúdo. ≈ `QExpansionItem`.
//!
//! A altura anima só com CSS (`grid-template-rows: 0fr → 1fr`, ver os shortcuts
//! `lx-expansion-content-*`); o conteúdo é sempre renderizado (também no SSR), e o fechado fica
//! `visibility: hidden`, fora da ordem de foco e da árvore de acessibilidade. A lógica sem DOM
//! (modo sanfona, teclas) fica em funções puras testadas no host.

use std::{cell::RefCell, collections::HashMap};

use leptos::{ev, prelude::*};

use super::{Icon, Item, ItemLabel, ItemSection};
use crate::util::{cls, next_id};

/// Slot `header` do [`ExpansionItem`]: `<ExpansionItemHeader slot:header>…</ExpansionItemHeader>`.
/// Substitui o rótulo, a legenda e o ícone do cabeçalho; o ícone de expandir continua.
#[slot]
pub struct ExpansionItemHeader {
    children: ChildrenFn,
}

// ── Lógica pura ────────────────────────────────────────────────────────────

/// Modo sanfona: o item `me` acabou de abrir/fechar; `active` é quem o grupo registra como
/// aberto. Devolve o novo registro: abrir toma o registro (os outros itens abertos fecham ao ver
/// a mudança, ver [`must_close`]); fechar solta o registro, mas só se era o próprio.
fn group_claim(open: bool, active: Option<&str>, me: &str) -> Option<String> {
    match (open, active) {
        (true, _) => Some(me.to_string()),
        (false, Some(a)) if a == me => None,
        (false, a) => a.map(str::to_string),
    }
}

/// Um item aberto fecha quando o grupo registra outro item.
fn must_close(open: bool, active: Option<&str>, me: &str) -> bool {
    open && active.is_some_and(|a| a != me)
}

/// `Enter` e `Espaço` alternam o cabeçalho.
fn key_toggles(key: &str) -> bool {
    matches!(key, "Enter" | " ")
}

// ── Registro de grupos ─────────────────────────────────────────────────────

thread_local! {
    /// Por nome de grupo, o `id` do item que o grupo considera aberto. `ArcRwSignal`: não
    /// pertence a nenhum `Owner`, então sobrevive a quem a criou. Só é tocado em `Effect`s
    /// (navegador).
    static GROUPS: RefCell<HashMap<String, ArcRwSignal<Option<String>>>> =
        RefCell::new(HashMap::new());
}

fn group_signal(name: &str) -> ArcRwSignal<Option<String>> {
    GROUPS.with(|g| {
        g.borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| ArcRwSignal::new(None))
            .clone()
    })
}

/// Item expansível. ≈ `QExpansionItem`.
///
/// `value` é o estado aberto (`v-model`); sem ele o componente guarda o estado. `group` liga
/// vários itens no modo sanfona (abrir um fecha os outros do mesmo nome). O cabeçalho é um
/// [`Item`] com `role="button"`, `aria-expanded` e `aria-controls`; `Enter` e `Espaço` alternam.
///
/// `on_show`/`on_hide` rodam quando o estado muda (não na primeira renderização).
///
/// Ainda não: `expanded_icon`, `expand_icon_toggle`, `hide_expand_icon`, `popup`, `duration`
/// (vem do token `--lx-expansion-item-duration`), `label_lines`, `caption_lines`,
/// `header_inset_level`, `content_inset_level`, `dense_toggle`, `href`, `dark`, `toggle_aria_label`.
#[component]
pub fn ExpansionItem(
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional, into)] caption: Option<String>,
    /// Ícone lucide antes do rótulo (classe literal `i-lucide-…`).
    #[prop(optional)]
    icon: Option<&'static str>,
    /// Ícone de expandir (classe literal); padrão `i-lucide-chevron-down`, que gira ao abrir.
    #[prop(default = "i-lucide-chevron-down")]
    expand_icon: &'static str,
    /// Estado aberto controlado de fora.
    #[prop(optional)]
    value: Option<RwSignal<bool>>,
    #[prop(optional)] default_opened: bool,
    /// Nome do grupo (sanfona).
    #[prop(optional, into)]
    group: Option<String>,
    #[prop(optional)] dense: bool,
    /// Ícone de expandir antes do conteúdo do cabeçalho, em vez de depois.
    #[prop(optional)]
    switch_toggle_side: bool,
    /// Linhas acima e abaixo do item enquanto está aberto.
    #[prop(optional)]
    expand_separator: bool,
    #[prop(optional, into)] disable: Signal<bool>,
    #[prop(optional, into)] on_show: Option<Callback<()>>,
    #[prop(optional, into)] on_hide: Option<Callback<()>>,
    #[prop(optional)] header: Option<ExpansionItemHeader>,
    children: Children,
) -> impl IntoView {
    let open = value.unwrap_or_else(|| RwSignal::new(default_opened));
    let id = next_id("lx-expansion");
    let header_id = format!("{id}-header");
    let content_id = format!("{id}-content");

    // Eventos e sanfona: só no navegador (`Effect` não roda no SSR).
    let me = id.clone();
    let group_sig = group.as_deref().map(group_signal);
    let group_sig2 = group_sig.clone();
    Effect::new(move |prev: Option<bool>| {
        let now = open.get();
        if let Some(sig) = &group_sig {
            let current = sig.get_untracked();
            let next = group_claim(now, current.as_deref(), &me);
            if next != current {
                sig.set(next);
            }
        }
        if let Some(prev) = prev
            && prev != now
        {
            let cb = if now { on_show } else { on_hide };
            if let Some(cb) = cb {
                cb.run(());
            }
        }
        now
    });
    // Outro item do grupo abriu: o registro mudou e este fecha.
    if let Some(sig) = group_sig2 {
        let me = id.clone();
        Effect::new(move |_| {
            let active = sig.get();
            if must_close(open.get_untracked(), active.as_deref(), &me) {
                open.set(false);
            }
        });
    }

    let toggle = move || {
        if !disable.get_untracked() {
            open.update(|o| *o = !*o);
        }
    };
    let on_key = move |e: ev::KeyboardEvent| {
        if key_toggles(&e.key()) {
            e.prevent_default();
            toggle();
        }
    };

    let root_class = move || {
        let sep = match (expand_separator, open.get()) {
            (false, _) => "lx-expansion-item-sep-none",
            (true, true) => "lx-expansion-item-sep-open",
            (true, false) => "lx-expansion-item-sep-closed",
        };
        cls(["lx-expansion-item", sep])
    };
    let icon_class = move || {
        if open.get() {
            "lx-expansion-icon-open"
        } else {
            "lx-expansion-icon-closed"
        }
    };
    let header_wrap = move || {
        if disable.get() {
            "lx-expansion-header-disabled"
        } else {
            "lx-expansion-header-enabled"
        }
    };
    let content_class = move || {
        if open.get() {
            "lx-expansion-content-open"
        } else {
            "lx-expansion-content-closed"
        }
    };

    let toggle_section = move || {
        view! {
            <ItemSection side=true>
                <span class=icon_class>
                    <Icon name=expand_icon />
                </span>
            </ItemSection>
        }
    };
    let main = match header {
        Some(h) => (h.children)().into_any(),
        None => view! {
            {icon.map(|i| view! { <ItemSection avatar=true><Icon name=i /></ItemSection> })}
            <ItemSection>
                <ItemLabel>{label}</ItemLabel>
                {caption.map(|c| view! { <ItemLabel caption=true>{c}</ItemLabel> })}
            </ItemSection>
        }
        .into_any(),
    };
    let (before, after) = if switch_toggle_side {
        (Some(toggle_section()), None)
    } else {
        (None, Some(toggle_section()))
    };

    view! {
        <div class=root_class>
            <div class=header_wrap>
                <Item
                    dense=dense
                    clickable=true
                    on_click=move |_| toggle()
                    attr:id=header_id.clone()
                    attr:role="button"
                    attr:tabindex=move || if disable.get() { "-1" } else { "0" }
                    attr:aria-expanded=move || open.get().to_string()
                    attr:aria-controls=content_id.clone()
                    attr:aria-disabled=move || disable.get().then_some("true")
                    on:keydown=on_key
                >
                    {before}
                    {main}
                    {after}
                </Item>
            </div>
            <div class=content_class id=content_id role="region" aria-labelledby=header_id>
                <div class="lx-expansion-content-clip">{children()}</div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_takes_the_group() {
        assert_eq!(group_claim(true, None, "a"), Some("a".into()));
        assert_eq!(group_claim(true, Some("b"), "a"), Some("a".into()));
    }

    #[test]
    fn closing_releases_only_my_own_claim() {
        assert_eq!(group_claim(false, Some("a"), "a"), None);
        assert_eq!(group_claim(false, Some("b"), "a"), Some("b".into()));
        assert_eq!(group_claim(false, None, "a"), None);
    }

    #[test]
    fn open_item_closes_when_another_is_registered() {
        assert!(must_close(true, Some("b"), "a"));
        assert!(!must_close(true, Some("a"), "a"));
        assert!(!must_close(true, None, "a"));
        assert!(!must_close(false, Some("b"), "a"));
    }

    #[test]
    fn enter_and_space_toggle() {
        assert!(key_toggles("Enter") && key_toggles(" "));
        assert!(!key_toggles("a") && !key_toggles("Tab"));
    }
}
