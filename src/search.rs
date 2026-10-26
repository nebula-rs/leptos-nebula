//! Busca AICI (*accent-insensitive, case-insensitive*), o único lugar onde o `leptos-nebula`
//! normaliza texto para filtrar (ADR-0013, `docs/decisions/0013-busca-aici-e-use-filter.md`).
//!
//! Sem DOM e sem Leptos, testado no host. Usado pelo [`Autocomplete`](crate::Autocomplete) e
//! pelo `use_filter` do [`Select`](crate::Select) (R-46); é público para o app que filtra sozinho
//! (`Autocomplete` com `filter=false`) usar **a mesma** regra.
//!
//! O contrato é em duas etapas, para não normalizar a lista a cada tecla:
//!
//! 1. [`normalize`] cada rótulo **uma vez** (quando a lista muda) e a agulha uma vez por texto
//!    digitado;
//! 2. [`matches`] (ou [`filter_indices`]) compara os já normalizados.
//!
//! ```
//! use leptos_nebula::search::{filter_indices, matches, normalize};
//!
//! let labels: Vec<String> = ["São Paulo", "Santos", "Salvador"].iter().map(|l| normalize(l)).collect();
//! let needle = normalize("SAO");
//! assert!(matches(&labels[0], &needle));
//! assert_eq!(filter_indices(&labels, &needle), vec![0]);
//! ```

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Texto sem acentos e em minúsculas: decomposição **NFD**, remoção das marcas combinantes
/// (acentos, cedilha, til…) e `to_lowercase` (Unicode).
///
/// Equivale ao JS `text.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase()` para letras
/// acentuadas. Símbolos diacríticos **isolados** (`^`, `` ` ``, `´`, `¨`) permanecem — diferença
/// aceita. Não faz `trim`: o espaço conta.
///
/// ```
/// use leptos_nebula::search::normalize;
/// assert_eq!(normalize("São Paulo"), "sao paulo");
/// assert_eq!(normalize("ÜBER"), "uber");
/// ```
pub fn normalize(text: &str) -> String {
    text.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// O rótulo normalizado **contém** a agulha normalizada. Agulha vazia aceita tudo.
///
/// Os dois argumentos já vêm de [`normalize`]; comparar texto cru diferencia acentos e caixa.
///
/// ```
/// use leptos_nebula::search::{matches, normalize};
/// assert!(matches(&normalize("Ação"), &normalize("acao")));
/// assert!(matches(&normalize("Ação"), ""));
/// assert!(!matches(&normalize("Ação"), &normalize("xyz")));
/// ```
pub fn matches(normalized_label: &str, normalized_needle: &str) -> bool {
    normalized_label.contains(normalized_needle)
}

/// Índices (na ordem original) dos rótulos normalizados que casam com a agulha normalizada.
/// Agulha vazia devolve todos.
pub fn filter_indices<S: AsRef<str>>(
    normalized_labels: &[S],
    normalized_needle: &str,
) -> Vec<usize> {
    normalized_labels
        .iter()
        .enumerate()
        .filter(|(_, l)| matches(l.as_ref(), normalized_needle))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(label: &str, needle: &str) -> bool {
        matches(&normalize(label), &normalize(needle))
    }

    #[test]
    fn strips_accents_and_lowercases() {
        assert_eq!(normalize("São Paulo"), "sao paulo");
        assert_eq!(normalize("Ação"), "acao");
        assert_eq!(normalize("ÜBER"), "uber");
        assert_eq!(normalize("Paçoca"), "pacoca");
        assert_eq!(normalize("ÀÉÎÕÜ àéîõü"), "aeiou aeiou");
    }

    #[test]
    fn works_with_precomposed_and_decomposed_input() {
        // "é" como um caractere (U+00E9) e como "e" + acento combinante (U+0065 U+0301)
        assert_eq!(normalize("caf\u{e9}"), "cafe");
        assert_eq!(normalize("cafe\u{301}"), "cafe");
        assert!(hit("caf\u{e9}", "cafe\u{301}"));
    }

    #[test]
    fn keeps_isolated_diacritic_symbols_and_spaces() {
        assert_eq!(normalize("a^b `c´ ¨"), "a^b `c´ ¨");
        assert_eq!(normalize(" a b "), " a b ");
    }

    #[test]
    fn contains_not_prefix() {
        assert!(hit("São Paulo", "sao"));
        assert!(hit("São Paulo", "paulo"));
        assert!(hit("São Paulo", "o pa"));
        assert!(hit("Ação", "acao"));
        assert!(hit("Ação", "ÇÃ"));
        assert!(hit("Paçoca", "pacoca"));
        assert!(hit("ÜBER", "uber"));
        assert!(hit("uber", "ÜBER"));
    }

    #[test]
    fn accented_needle_and_label_in_every_combination() {
        // o caso do `Select` com `use_filter`: rótulo e agulha, com e sem acento, nas duas direções
        for (label, needle) in [
            ("Ação", "acao"),
            ("Ação", "AÇÃO"),
            ("acao", "Ação"),
            ("ÜBER", "uber"),
            ("uber", "ÜBER"),
            ("Paçoca", "pacoca"),
            ("PAÇOCA", "çoc"),
            ("São Paulo", "SAO"),
            ("São Paulo", "são p"),
        ] {
            assert!(hit(label, needle), "{label:?} deveria casar com {needle:?}");
        }
        assert!(!hit("Ação", "acai"));
        assert!(!hit("Paçoca", "pacocas"));
    }

    #[test]
    fn empty_needle_matches_everything_and_unrelated_nothing() {
        assert!(hit("São Paulo", ""));
        assert!(hit("", ""));
        assert!(!hit("São Paulo", "rio"));
        assert!(!hit("", "a"));
    }

    #[test]
    fn filter_indices_keeps_the_original_order() {
        let labels: Vec<String> = ["São Paulo", "Santos", "Rio", "Osasco"]
            .iter()
            .map(|l| normalize(l))
            .collect();
        assert_eq!(filter_indices(&labels, "sa"), vec![0, 1, 3]);
        assert_eq!(filter_indices(&labels, "sao"), vec![0]);
        assert_eq!(filter_indices(&labels, ""), vec![0, 1, 2, 3]);
        assert!(filter_indices(&labels, "zzz").is_empty());
        assert!(filter_indices::<String>(&[], "a").is_empty());
    }
}
