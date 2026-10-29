//! Busca AICI (*accent-insensitive, case-insensitive*), o único lugar onde o `leptos-nebula`
//! normaliza texto para filtrar (ADR-0013, `docs/decisions/0013-aici-search-and-use-filter.md`).
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
//! Para destacar o trecho achado no texto **original**, [`normalize_with_map`] devolve também a
//! posição de origem de cada byte do normalizado (usado pela busca do `leptos-press`).
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

/// Como [`normalize`], mas devolve também onde cada byte do texto normalizado veio do original
/// — para destacar, no texto **original**, um trecho achado no normalizado.
///
/// O texto devolvido é **igual** ao de [`normalize`]. O mapa tem `normalizado.len() + 1`
/// posições, todas em **bytes**: `map[i]` é o início, no original, do caractere que gerou o byte
/// `i` do normalizado (todos os bytes vindos do mesmo caractere apontam para o mesmo início), e
/// `map[normalizado.len()] == text.len()`. É não decrescente. Um caractere que some (marca
/// combinante solta) não aparece no mapa; um que muda de tamanho (`ã` → `a`, `ẞ` → `ß`,
/// `K` Kelvin → `k`) aponta todos os seus bytes para o mesmo início.
///
/// Trecho `[s, e)` do normalizado (em limites de caractere) → original: começa em `map[s]`;
/// termina no primeiro `map[j]`, `j >= e`, maior que `map[e - 1]` (em geral o próprio `map[e]`).
///
/// ```
/// use leptos_nebula::search::normalize_with_map;
/// let (norm, map) = normalize_with_map("São Paulo");
/// assert_eq!(norm, "sao paulo");
/// let s = norm.find("sao").unwrap();
/// assert_eq!(&"São Paulo"[map[s]..map[s + 3]], "São");
/// ```
pub fn normalize_with_map(text: &str) -> (String, Vec<usize>) {
    let mut norm = String::with_capacity(text.len());
    let mut map = Vec::with_capacity(text.len() + 1);
    // Caractere a caractere: as marcas combinantes são removidas, então a reordenação canônica
    // do NFD (que só move marcas) não muda o resultado — é o mesmo texto de `normalize`.
    for (start, c) in text.char_indices() {
        let before = norm.len();
        norm.extend(
            c.nfd()
                .filter(|m| !is_combining_mark(*m))
                .flat_map(char::to_lowercase),
        );
        map.resize(map.len() + (norm.len() - before), start);
    }
    map.push(text.len());
    (norm, map)
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

    /// Original do trecho `[s, e)` do normalizado, pela regra do doc comment.
    fn original<'a>(text: &'a str, map: &[usize], s: usize, e: usize) -> &'a str {
        let end = map[e..]
            .iter()
            .copied()
            .find(|&o| o > map[e - 1])
            .unwrap_or(text.len());
        &text[map[s]..end]
    }

    #[test]
    fn map_text_is_the_same_as_normalize() {
        for t in [
            "",
            "São Paulo",
            "Ação",
            "ÜBER",
            "Paçoca",
            "Straße",
            "STRAẞE",
            "İstanbul",
            "\u{212A}elvin",
            "cafe\u{301}",
            "\u{301}solta",
            "a^b `c´ ¨",
            "key_as_value",
        ] {
            let (norm, map) = normalize_with_map(t);
            assert_eq!(norm, normalize(t), "{t:?}");
            assert_eq!(map.len(), norm.len() + 1, "{t:?}");
            assert_eq!(map.last(), Some(&t.len()), "{t:?}");
            assert!(map.windows(2).all(|par| par[0] <= par[1]), "{t:?}");
        }
    }

    #[test]
    fn map_points_back_to_accented_letters() {
        let t = "Em São Paulo";
        let (norm, map) = normalize_with_map(t);
        assert_eq!(norm, "em sao paulo");
        assert_eq!(map[..7], [0, 1, 2, 3, 4, 6, 7]);
        let s = norm.find("sao").unwrap();
        assert_eq!(original(t, &map, s, s + 3), "São");
        // termina no meio da palavra, logo depois do "ã"
        assert_eq!(original(t, &map, s, s + 2), "Sã");
    }

    #[test]
    fn map_with_cedilla_and_tilde() {
        let t = "Ação!";
        let (norm, map) = normalize_with_map(t);
        assert_eq!(norm, "acao!");
        assert_eq!(map, [0, 1, 3, 5, 6, 7]);
        assert_eq!(original(t, &map, 1, 3), "çã");
        assert_eq!(original(t, &map, 0, 4), "Ação");
    }

    #[test]
    fn map_with_letters_that_change_size() {
        // ß fica ß (2 bytes); ẞ (3 bytes) vira ß (2 bytes); İ (2 bytes) vira i (1 byte);
        // o sinal Kelvin (3 bytes) vira k (1 byte).
        let t = "STRAẞE";
        let (norm, map) = normalize_with_map(t);
        assert_eq!(norm, "straße");
        let s = norm.find("ße").unwrap();
        assert_eq!(original(t, &map, s, s + "ß".len()), "ẞ");
        assert_eq!(original(t, &map, s, norm.len()), "ẞE");

        let t = "Straße";
        let (norm, map) = normalize_with_map(t);
        assert_eq!(norm, "straße");
        assert_eq!(original(t, &map, 4, 6), "ß");

        let t = "İstanbul";
        let (norm, map) = normalize_with_map(t);
        assert_eq!(norm, "istanbul");
        assert_eq!(original(t, &map, 0, 3), "İst");

        let t = "1 \u{212A}m";
        let (norm, map) = normalize_with_map(t);
        assert_eq!(norm, "1 km");
        assert_eq!(original(t, &map, 2, 4), "\u{212A}m");
    }

    #[test]
    fn map_skips_loose_combining_marks() {
        // "e" + acento combinante: os dois bytes do acento somem; o "e" aponta para o início.
        let t = "cafe\u{301} bom";
        let (norm, map) = normalize_with_map(t);
        assert_eq!(norm, "cafe bom");
        let s = norm.find("cafe").unwrap();
        assert_eq!(original(t, &map, s, s + 4), "cafe\u{301}");
        assert_eq!(original(t, &map, 5, 8), "bom");
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
