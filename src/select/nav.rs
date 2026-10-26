//! Lógica pura do `Select` — **sem DOM e sem Leptos**, testada no host.
//!
//! Índice ativo (o `aria-activedescendant`) com setas/`Home`/`End` pulando as opções
//! desabilitadas, type-ahead por prefixo do label com janela de tempo e a alternância de seleção
//! do `SelectMultiple`.

/// Tecla de navegação entre as opções.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NavKey {
    Next,
    Prev,
    First,
    Last,
}

/// Tecla (`KeyboardEvent.key`) → movimento.
pub(crate) fn nav_key(key: &str) -> Option<NavKey> {
    match key {
        "ArrowDown" => Some(NavKey::Next),
        "ArrowUp" => Some(NavKey::Prev),
        "Home" => Some(NavKey::First),
        "End" => Some(NavKey::Last),
        _ => None,
    }
}

fn first_enabled(disabled: &[bool]) -> Option<usize> {
    disabled.iter().position(|d| !d)
}

fn last_enabled(disabled: &[bool]) -> Option<usize> {
    disabled.iter().rposition(|d| !d)
}

/// Próxima opção ativa. `disabled[i]` diz se a opção `i` está desabilitada.
///
/// Sem dar a volta (padrão "select-only combobox" do WAI-ARIA): `Next` no fim e `Prev` no começo
/// ficam onde estão. Sem opção ativa, `Next` vai à primeira habilitada e `Prev` à última.
/// `None` só quando não há opção habilitada.
pub(crate) fn move_active(disabled: &[bool], current: Option<usize>, key: NavKey) -> Option<usize> {
    match (key, current) {
        (NavKey::First, _) | (NavKey::Next, None) => first_enabled(disabled),
        (NavKey::Last, _) | (NavKey::Prev, None) => last_enabled(disabled),
        (NavKey::Next, Some(i)) => disabled
            .iter()
            .enumerate()
            .skip(i + 1)
            .find(|(_, d)| !**d)
            .map(|(j, _)| j)
            .or(Some(i)),
        (NavKey::Prev, Some(i)) => disabled
            .iter()
            .enumerate()
            .take(i)
            .rfind(|(_, d)| !**d)
            .map(|(j, _)| j)
            .or(Some(i)),
    }
}

/// Opção ativa ao abrir: a primeira selecionada e habilitada; senão a primeira habilitada
/// (ou, abrindo com `↑`, a última).
pub(crate) fn initial_active(
    disabled: &[bool],
    selected: &[bool],
    from_end: bool,
) -> Option<usize> {
    let picked = disabled.iter().zip(selected).position(|(d, s)| *s && !*d);
    picked.or_else(|| {
        if from_end {
            last_enabled(disabled)
        } else {
            first_enabled(disabled)
        }
    })
}

/// Navegação **dentro de um subconjunto visível** (`use_filter`): `visible` são os índices (na
/// lista completa, em ordem) que o filtro deixou; `disabled`/`selected` continuam indexados pela
/// lista completa. Devolve sempre um índice da lista completa — o id da opção não depende do
/// filtro. Uma opção ativa que o filtro escondeu conta como "sem ativa".
pub(crate) fn move_active_in(
    visible: &[usize],
    disabled: &[bool],
    current: Option<usize>,
    key: NavKey,
) -> Option<usize> {
    let sub = subset(visible, disabled);
    let cur = current.and_then(|c| visible.iter().position(|&i| i == c));
    move_active(&sub, cur, key).map(|k| visible[k])
}

/// [`initial_active`] dentro do subconjunto visível.
pub(crate) fn initial_active_in(
    visible: &[usize],
    disabled: &[bool],
    selected: &[bool],
    from_end: bool,
) -> Option<usize> {
    let (d, s) = (subset(visible, disabled), subset(visible, selected));
    initial_active(&d, &s, from_end).map(|k| visible[k])
}

fn subset(visible: &[usize], flags: &[bool]) -> Vec<bool> {
    visible
        .iter()
        .map(|&i| flags.get(i).copied().unwrap_or(true))
        .collect()
}

/// Janela do type-ahead, em ms: uma tecla depois disso começa uma busca nova.
pub(crate) const TYPEAHEAD_WINDOW_MS: f64 = 700.0;

/// O que o usuário digitou há pouco (o relógio entra como argumento, para testar).
#[derive(Debug, Default)]
pub(crate) struct TypeAhead {
    buffer: String,
    last: f64,
}

impl TypeAhead {
    /// A busca em curso ainda vale (há texto e a janela não passou)?
    pub(crate) fn is_active(&self, now: f64) -> bool {
        !self.buffer.is_empty() && now - self.last <= TYPEAHEAD_WINDOW_MS
    }

    /// Acrescenta uma tecla; fora da janela, recomeça. Devolve o texto buscado (em minúsculas).
    pub(crate) fn push(&mut self, ch: char, now: f64) -> &str {
        if !self.is_active(now) {
            self.buffer.clear();
        }
        self.last = now;
        self.buffer.extend(ch.to_lowercase());
        &self.buffer
    }

    pub(crate) fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// A tecla é um caractere imprimível que alimenta o type-ahead? (`key` de um só caractere; o
/// espaço só conta no meio de uma busca em curso — fora dela ele abre/escolhe.)
pub(crate) fn typeahead_char(key: &str, searching: bool) -> Option<char> {
    let mut chars = key.chars();
    let c = chars.next()?;
    if chars.next().is_some() || c.is_control() {
        return None;
    }
    if c == ' ' && !searching {
        return None;
    }
    Some(c)
}

/// Opção que casa com o texto digitado: prefixo do label, sem diferenciar caixa, pulando as
/// desabilitadas.
///
/// Uma letra só (ou a mesma letra repetida: `aaa`) anda pelas opções que começam com ela, a
/// partir da seguinte à ativa e dando a volta; várias letras diferentes buscam a partir da
/// própria opção ativa (que continua valendo se ainda casa).
pub(crate) fn find_prefix<S: AsRef<str>>(
    labels: &[S],
    disabled: &[bool],
    buffer: &str,
    current: Option<usize>,
) -> Option<usize> {
    let mut chars = buffer.chars();
    let first = chars.next()?;
    let repeated = chars.all(|c| c == first);
    let (needle, cycle) = if repeated {
        (first.to_string(), true)
    } else {
        (buffer.to_string(), false)
    };
    let len = labels.len();
    if len == 0 {
        return None;
    }
    let start = match (current, cycle) {
        (Some(i), true) => (i + 1) % len,
        (Some(i), false) => i,
        (None, _) => 0,
    };
    (0..len).map(|k| (start + k) % len).find(|&i| {
        !disabled.get(i).copied().unwrap_or(false)
            && labels[i]
                .as_ref()
                .trim_start()
                .to_lowercase()
                .starts_with(&needle)
    })
}

/// Alterna `v` na lista (ordem de escolha). `true` se foi acrescentado.
pub(crate) fn toggle<T>(list: &mut Vec<T>, v: T, same: impl Fn(&T, &T) -> bool) -> bool {
    if let Some(i) = list.iter().position(|x| same(x, &v)) {
        list.remove(i);
        false
    } else {
        list.push(v);
        true
    }
}

/// Texto exibido para a seleção: os labels separados por vírgula.
pub(crate) fn join_labels<S: AsRef<str>>(labels: &[S]) -> String {
    labels
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO: bool = false;
    const DIS: bool = true;
    const YES: bool = true;

    #[test]
    fn maps_navigation_keys() {
        assert_eq!(nav_key("ArrowDown"), Some(NavKey::Next));
        assert_eq!(nav_key("ArrowUp"), Some(NavKey::Prev));
        assert_eq!(nav_key("Home"), Some(NavKey::First));
        assert_eq!(nav_key("End"), Some(NavKey::Last));
        assert_eq!(nav_key("a"), None);
        assert_eq!(nav_key("Enter"), None);
    }

    #[test]
    fn arrows_skip_disabled_options() {
        let d = [NO, DIS, DIS, NO, NO];
        assert_eq!(move_active(&d, Some(0), NavKey::Next), Some(3));
        assert_eq!(move_active(&d, Some(3), NavKey::Next), Some(4));
        assert_eq!(move_active(&d, Some(3), NavKey::Prev), Some(0));
        assert_eq!(move_active(&d, Some(4), NavKey::Prev), Some(3));
    }

    #[test]
    fn arrows_stop_at_the_ends() {
        let d = [NO, NO, NO];
        assert_eq!(move_active(&d, Some(2), NavKey::Next), Some(2));
        assert_eq!(move_active(&d, Some(0), NavKey::Prev), Some(0));
        // só desabilitadas depois: fica onde está
        let d = [NO, NO, DIS, DIS];
        assert_eq!(move_active(&d, Some(1), NavKey::Next), Some(1));
    }

    #[test]
    fn home_and_end_pick_the_first_and_last_enabled() {
        let d = [DIS, NO, NO, DIS];
        assert_eq!(move_active(&d, Some(2), NavKey::First), Some(1));
        assert_eq!(move_active(&d, Some(1), NavKey::Last), Some(2));
    }

    #[test]
    fn without_an_active_option_the_arrows_enter_from_the_ends() {
        let d = [DIS, NO, NO, DIS];
        assert_eq!(move_active(&d, None, NavKey::Next), Some(1));
        assert_eq!(move_active(&d, None, NavKey::Prev), Some(2));
    }

    #[test]
    fn nothing_to_navigate() {
        assert_eq!(move_active(&[], None, NavKey::Next), None);
        assert_eq!(move_active(&[DIS, DIS], None, NavKey::First), None);
        assert_eq!(move_active(&[DIS, DIS], None, NavKey::Last), None);
    }

    #[test]
    fn opening_lands_on_the_selection_or_the_first_enabled() {
        let d = [DIS, NO, NO, NO];
        assert_eq!(
            initial_active(&d, &[false, false, true, false], false),
            Some(2)
        );
        assert_eq!(initial_active(&d, &[false; 4], false), Some(1));
        assert_eq!(initial_active(&d, &[false; 4], true), Some(3));
        // a selecionada está desabilitada: cai na primeira habilitada
        assert_eq!(
            initial_active(&d, &[true, false, false, false], false),
            Some(1)
        );
        assert_eq!(initial_active(&[], &[], false), None);
    }

    #[test]
    fn filtered_navigation_walks_only_the_visible_options() {
        // lista completa de 6; o filtro deixou 1, 3 e 4 (a 3 desabilitada)
        let disabled = [NO, NO, NO, DIS, NO, NO];
        let visible = [1, 3, 4];
        let nav = |cur, key| move_active_in(&visible, &disabled, cur, key);
        assert_eq!(nav(None, NavKey::Next), Some(1));
        assert_eq!(nav(Some(1), NavKey::Next), Some(4)); // pula a 3 (desabilitada)
        assert_eq!(nav(Some(4), NavKey::Next), Some(4)); // sem dar a volta
        assert_eq!(nav(Some(4), NavKey::Prev), Some(1));
        assert_eq!(nav(Some(1), NavKey::Prev), Some(1));
        assert_eq!(nav(Some(1), NavKey::Last), Some(4));
        assert_eq!(nav(Some(4), NavKey::First), Some(1));
        // a ativa que o filtro escondeu (0) vale como "sem ativa"
        assert_eq!(nav(Some(0), NavKey::Next), Some(1));
        assert_eq!(nav(Some(0), NavKey::Prev), Some(4));
    }

    #[test]
    fn filtered_navigation_with_no_visible_enabled_option() {
        assert_eq!(move_active_in(&[], &[NO, NO], None, NavKey::Next), None);
        assert_eq!(move_active_in(&[1], &[NO, DIS], None, NavKey::First), None);
    }

    #[test]
    fn filtered_opening_prefers_the_visible_selection_then_the_first_enabled() {
        let disabled = [NO, NO, DIS, NO];
        let selected = [NO, NO, NO, YES];
        // a escolhida (3) está visível
        assert_eq!(
            initial_active_in(&[1, 3], &disabled, &selected, false),
            Some(3)
        );
        // a escolhida foi escondida: a primeira habilitada visível
        assert_eq!(
            initial_active_in(&[1, 2], &disabled, &selected, false),
            Some(1)
        );
        assert_eq!(
            initial_active_in(&[1, 2], &disabled, &selected, true),
            Some(1)
        );
        // tudo escondido
        assert_eq!(initial_active_in(&[], &disabled, &selected, false), None);
    }

    #[test]
    fn typeahead_buffer_resets_after_the_window() {
        let mut t = TypeAhead::default();
        assert!(!t.is_active(0.0));
        assert_eq!(t.push('B', 1000.0), "b");
        assert!(t.is_active(1500.0));
        assert_eq!(t.push('a', 1500.0), "ba");
        // passou da janela: recomeça
        assert!(!t.is_active(2300.0));
        assert_eq!(t.push('c', 2300.0), "c");
        t.reset();
        assert!(!t.is_active(2301.0));
    }

    #[test]
    fn only_single_printable_characters_feed_the_typeahead() {
        assert_eq!(typeahead_char("a", false), Some('a'));
        assert_eq!(typeahead_char("Ç", false), Some('Ç'));
        assert_eq!(typeahead_char("Enter", false), None);
        assert_eq!(typeahead_char("ArrowDown", false), None);
        assert_eq!(typeahead_char("\u{7}", false), None);
        assert_eq!(typeahead_char("", false), None);
        // o espaço só vale no meio de uma busca
        assert_eq!(typeahead_char(" ", false), None);
        assert_eq!(typeahead_char(" ", true), Some(' '));
    }

    #[test]
    fn prefix_search_ignores_case_and_skips_disabled() {
        let labels = ["Maçã", "Banana", "Beterraba", "Cereja"];
        let d = [NO, NO, DIS, NO];
        assert_eq!(find_prefix(&labels, &d, "b", None), Some(1));
        assert_eq!(find_prefix(&labels, &d, "be", None), None); // Beterraba está desabilitada
        assert_eq!(find_prefix(&labels, &d, "c", Some(0)), Some(3));
        assert_eq!(find_prefix(&labels, &d, "maç", None), Some(0));
        assert_eq!(find_prefix(&labels, &d, "z", None), None);
        assert_eq!(find_prefix(&labels, &d, "", None), None);
    }

    #[test]
    fn one_letter_cycles_through_the_matches_wrapping_around() {
        let labels = ["Abacate", "Banana", "Abacaxi", "Acerola"];
        let d = [NO; 4];
        assert_eq!(find_prefix(&labels, &d, "a", None), Some(0));
        assert_eq!(find_prefix(&labels, &d, "a", Some(0)), Some(2));
        assert_eq!(find_prefix(&labels, &d, "a", Some(2)), Some(3));
        assert_eq!(find_prefix(&labels, &d, "a", Some(3)), Some(0));
        // a mesma letra repetida também anda
        assert_eq!(find_prefix(&labels, &d, "aa", Some(0)), Some(2));
    }

    #[test]
    fn longer_prefixes_keep_the_active_option_when_it_still_matches() {
        let labels = ["Abacate", "Abacaxi", "Acerola"];
        let d = [NO; 3];
        assert_eq!(find_prefix(&labels, &d, "aba", Some(1)), Some(1));
        assert_eq!(find_prefix(&labels, &d, "ace", Some(1)), Some(2));
        assert_eq!(find_prefix(&labels, &d, "abaca", Some(2)), Some(0));
    }

    #[test]
    fn leading_spaces_in_the_label_do_not_count() {
        assert_eq!(find_prefix(&["  Zebra"], &[NO], "z", None), Some(0));
    }

    #[test]
    fn toggle_adds_and_removes_in_choice_order() {
        let mut l = vec![3, 1];
        assert!(toggle(&mut l, 2, |a, b| a == b));
        assert_eq!(l, vec![3, 1, 2]);
        assert!(!toggle(&mut l, 1, |a, b| a == b));
        assert_eq!(l, vec![3, 2]);
        assert!(!toggle(&mut l, 3, |a, b| a == b));
        assert!(!toggle(&mut l, 2, |a, b| a == b));
        assert!(l.is_empty());
    }

    #[test]
    fn labels_are_joined_by_comma() {
        assert_eq!(join_labels(&["A", "B", "C"]), "A, B, C");
        assert_eq!(join_labels(&["A"]), "A");
        assert_eq!(join_labels::<&str>(&[]), "");
    }
}
