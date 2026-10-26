//! Lógica pura do `Autocomplete` — **sem DOM e sem Leptos**, testada no host.
//!
//! Quais sugestões aparecem (o filtro AICI de [`crate::search`], ou nenhum), o que cada tecla faz
//! e como o índice ativo se comporta quando a lista muda. A navegação entre as sugestões reaproveita
//! [`move_active`] do `Select` (aqui nenhuma sugestão é desabilitada).

use crate::search::{filter_indices, normalize};
use crate::select::nav::{NavKey, move_active};

/// As sugestões visíveis para o texto digitado.
///
/// `normalized` são as `options` já passadas por [`normalize`] (calculadas uma vez por mudança da
/// lista); a agulha é normalizada aqui. Com `filter == false` as `options` saem como vieram.
pub(crate) fn visible(
    options: &[String],
    normalized: &[String],
    text: &str,
    filter: bool,
) -> Vec<String> {
    if !filter {
        return options.to_vec();
    }
    let needle = normalize(text);
    filter_indices(normalized, &needle)
        .into_iter()
        .filter_map(|i| options.get(i).cloned())
        .collect()
}

/// O que uma tecla pressionada no campo pede ao `Autocomplete`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyAction {
    /// Nada: o navegador trata a tecla (digitar, `Home`/`End`, `Enter` que envia o `Form`…).
    Nothing,
    /// `↓`/`↑` com a lista fechada: abre e ativa a primeira (`↓`) ou a última (`↑`) sugestão.
    /// Com `preventDefault`.
    Open(NavKey),
    /// `↓`/`↑` com a lista aberta: move a sugestão ativa. Com `preventDefault`.
    Move(NavKey),
    /// `Enter` com uma sugestão ativa: preenche o texto e fecha. Com `preventDefault`.
    Pick(usize),
    /// `Enter` sem sugestão ativa, ou `Tab`: só fecha a lista; o comportamento nativo segue
    /// (envia o `Form`, passa o foco), sem `preventDefault`.
    Close,
}

/// Decisão para a tecla `key` (`KeyboardEvent.key`) com a lista `shown` (aberta e com sugestões),
/// a sugestão `active` e `len` sugestões visíveis.
///
/// `Esc` não aparece aqui: ele é tratado pelo ouvinte do `document` com a pilha de `Esc` (fecha
/// só o popup do topo, e só com `preventDefault` se havia lista aberta). `Home`/`End` ficam para
/// o campo de texto (movem o cursor).
pub(crate) fn key_action(key: &str, shown: bool, active: Option<usize>, len: usize) -> KeyAction {
    let nav = match key {
        "ArrowDown" => NavKey::Next,
        "ArrowUp" => NavKey::Prev,
        "Enter" if shown => {
            return match active.filter(|i| *i < len) {
                Some(i) => KeyAction::Pick(i),
                None => KeyAction::Close,
            };
        }
        "Tab" if shown => return KeyAction::Close,
        _ => return KeyAction::Nothing,
    };
    if len == 0 {
        KeyAction::Nothing
    } else if shown {
        KeyAction::Move(nav)
    } else {
        KeyAction::Open(nav)
    }
}

/// Próxima sugestão ativa: sem dar a volta (`↓` no fim e `↑` no começo ficam onde estão); sem
/// ativa, `↓` vai à primeira e `↑` à última.
pub(crate) fn step(len: usize, current: Option<usize>, nav: NavKey) -> Option<usize> {
    move_active(&vec![false; len], current.filter(|i| *i < len), nav)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cities() -> (Vec<String>, Vec<String>) {
        let options: Vec<String> = [
            "São Paulo",
            "Santos",
            "Rio de Janeiro",
            "Brasília",
            "Ação Social",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let normalized = options.iter().map(|o| normalize(o)).collect();
        (options, normalized)
    }

    #[test]
    fn filter_ignores_case_and_accents() {
        let (o, n) = cities();
        assert_eq!(visible(&o, &n, "sao", true), vec!["São Paulo"]);
        assert_eq!(visible(&o, &n, "SÃO", true), vec!["São Paulo"]);
        assert_eq!(visible(&o, &n, "acao", true), vec!["Ação Social"]);
        assert_eq!(visible(&o, &n, "brasilia", true), vec!["Brasília"]);
        assert_eq!(visible(&o, &n, "san", true), vec!["Santos"]);
    }

    #[test]
    fn filter_is_contains_in_original_order() {
        let (o, n) = cities();
        assert_eq!(
            visible(&o, &n, "s", true),
            vec!["São Paulo", "Santos", "Brasília", "Ação Social"]
        );
        assert_eq!(visible(&o, &n, "de ja", true), vec!["Rio de Janeiro"]);
    }

    #[test]
    fn empty_text_shows_everything() {
        let (o, n) = cities();
        assert_eq!(visible(&o, &n, "", true), o);
    }

    #[test]
    fn no_match_is_empty() {
        let (o, n) = cities();
        assert!(visible(&o, &n, "zzz", true).is_empty());
        assert!(visible(&[], &[], "a", true).is_empty());
    }

    #[test]
    fn without_filter_the_options_come_as_they_are() {
        let (o, n) = cities();
        assert_eq!(visible(&o, &n, "zzz", false), o);
        assert_eq!(visible(&o, &[], "sao", false), o);
    }

    #[test]
    fn arrows_open_a_closed_list_and_move_an_open_one() {
        assert_eq!(
            key_action("ArrowDown", false, None, 3),
            KeyAction::Open(NavKey::Next)
        );
        assert_eq!(
            key_action("ArrowUp", false, None, 3),
            KeyAction::Open(NavKey::Prev)
        );
        assert_eq!(
            key_action("ArrowDown", true, Some(0), 3),
            KeyAction::Move(NavKey::Next)
        );
        assert_eq!(
            key_action("ArrowUp", true, None, 3),
            KeyAction::Move(NavKey::Prev)
        );
        // sem sugestões, as setas são do navegador
        assert_eq!(key_action("ArrowDown", false, None, 0), KeyAction::Nothing);
        assert_eq!(key_action("ArrowUp", true, None, 0), KeyAction::Nothing);
    }

    #[test]
    fn enter_picks_the_active_suggestion() {
        assert_eq!(key_action("Enter", true, Some(2), 3), KeyAction::Pick(2));
    }

    #[test]
    fn enter_without_an_active_suggestion_keeps_the_native_behavior() {
        // fecha a lista, mas não é "Pick" (sem preventDefault: o Form é enviado)
        assert_eq!(key_action("Enter", true, None, 3), KeyAction::Close);
        // índice velho, fora da lista atual
        assert_eq!(key_action("Enter", true, Some(5), 3), KeyAction::Close);
        // lista fechada: nada a fazer
        assert_eq!(key_action("Enter", false, None, 3), KeyAction::Nothing);
        assert_eq!(key_action("Enter", false, Some(1), 3), KeyAction::Nothing);
    }

    #[test]
    fn tab_closes_without_picking() {
        assert_eq!(key_action("Tab", true, Some(1), 3), KeyAction::Close);
        assert_eq!(key_action("Tab", false, None, 3), KeyAction::Nothing);
    }

    #[test]
    fn text_keys_home_end_and_escape_are_not_handled_here() {
        for k in [
            "a",
            "Ç",
            " ",
            "Backspace",
            "Home",
            "End",
            "Escape",
            "ArrowLeft",
            "ArrowRight",
        ] {
            assert_eq!(key_action(k, true, Some(0), 3), KeyAction::Nothing, "{k}");
            assert_eq!(key_action(k, false, None, 3), KeyAction::Nothing, "{k}");
        }
    }

    #[test]
    fn stepping_does_not_wrap() {
        assert_eq!(step(3, None, NavKey::Next), Some(0));
        assert_eq!(step(3, None, NavKey::Prev), Some(2));
        assert_eq!(step(3, Some(0), NavKey::Next), Some(1));
        assert_eq!(step(3, Some(2), NavKey::Next), Some(2));
        assert_eq!(step(3, Some(0), NavKey::Prev), Some(0));
        assert_eq!(step(3, Some(2), NavKey::Prev), Some(1));
        assert_eq!(step(0, None, NavKey::Next), None);
    }

    #[test]
    fn a_stale_active_index_counts_as_none() {
        // a lista encolheu de 5 para 2: o índice 4 não existe mais
        assert_eq!(step(2, Some(4), NavKey::Next), Some(0));
        assert_eq!(step(2, Some(4), NavKey::Prev), Some(1));
    }
}
