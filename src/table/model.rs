//! Lógica pura do `Table`: ordenação e paginação no cliente. Sem DOM e sem Leptos, testada no host.
//!
//! O ciclo de ordenação é o do Quasar (`columnSortOrder = 'ad'`): ascendente, descendente, sem
//! ordem; trocar de coluna recomeça no ascendente. Toda mudança de ordenação volta à página 1.

use std::cmp::Ordering;
use std::ops::Range;

/// Estado de ordenação e paginação de uma [`Table`](super::Table). ≈ `pagination` do `QTable`
/// (sem `rowsNumber`, que é do modo servidor).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TablePagination {
    /// `name` da coluna ordenada; `None` = na ordem original.
    pub sort_by: Option<String>,
    /// Ordem decrescente (só vale com `sort_by`).
    pub descending: bool,
    /// Página atual, a partir de 1.
    pub page: usize,
    /// Linhas por página; `0` = sem paginação (todas as linhas).
    pub rows_per_page: usize,
}

impl Default for TablePagination {
    fn default() -> Self {
        Self {
            sort_by: None,
            descending: false,
            page: 1,
            rows_per_page: 0,
        }
    }
}

impl TablePagination {
    /// Sem ordenação e na página 1, com `rows_per_page` linhas por página (`0` = todas).
    pub fn new(rows_per_page: usize) -> Self {
        Self {
            rows_per_page,
            ..Self::default()
        }
    }

    /// Ordenada por `name` desde o início.
    #[must_use]
    pub fn sorted_by(mut self, name: impl Into<String>, descending: bool) -> Self {
        self.sort_by = Some(name.into());
        self.descending = descending;
        self
    }
}

/// Estado da ordenação de uma coluna, para o `aria-sort`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortState {
    None,
    Ascending,
    Descending,
}

impl SortState {
    /// Valor de `aria-sort`.
    pub fn aria(self) -> &'static str {
        match self {
            SortState::None => "none",
            SortState::Ascending => "ascending",
            SortState::Descending => "descending",
        }
    }
}

/// Estado da coluna `name` dentro da paginação.
pub fn sort_state(pagination: &TablePagination, name: &str) -> SortState {
    match pagination.sort_by.as_deref() {
        Some(by) if by == name => {
            if pagination.descending {
                SortState::Descending
            } else {
                SortState::Ascending
            }
        }
        _ => SortState::None,
    }
}

/// Estado depois de acionar a ordenação da coluna `name`: outra coluna → ascendente; ascendente →
/// descendente; descendente → sem ordem. Sempre volta à página 1.
#[must_use]
pub fn toggle_sort(current: &TablePagination, name: &str) -> TablePagination {
    let mut next = current.clone();
    next.page = 1;
    match sort_state(current, name) {
        SortState::None => {
            next.sort_by = Some(name.to_string());
            next.descending = false;
        }
        SortState::Ascending => next.descending = true,
        SortState::Descending => {
            next.sort_by = None;
            next.descending = false;
        }
    }
    next
}

/// Comparador padrão: por texto, sem diferenciar maiúsculas de minúsculas (como o Quasar). Em
/// empate de texto, desempata pelo texto original, para a ordem ser total e estável.
pub fn compare_text(a: &str, b: &str) -> Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

/// Índices de `len` linhas na ordem de exibição. `cmp` compara duas linhas pelo índice; `None` =
/// ordem original. A ordenação é estável; `descending` inverte o comparador (empates seguem na
/// ordem original).
pub fn sorted_indices(
    len: usize,
    cmp: Option<&dyn Fn(usize, usize) -> Ordering>,
    descending: bool,
) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..len).collect();
    if let Some(cmp) = cmp {
        if descending {
            indices.sort_by(|&a, &b| cmp(b, a));
        } else {
            indices.sort_by(|&a, &b| cmp(a, b));
        }
    }
    indices
}

/// Total de páginas (no mínimo 1). `rows_per_page == 0` = uma página só.
pub fn page_count(total: usize, rows_per_page: usize) -> usize {
    if rows_per_page == 0 {
        1
    } else {
        total.div_ceil(rows_per_page).max(1)
    }
}

/// Página dentro de `1..=páginas`; `0` conta como 1 e uma página além do fim vira a última.
pub fn clamp_page(page: usize, total: usize, rows_per_page: usize) -> usize {
    page.clamp(1, page_count(total, rows_per_page))
}

/// Intervalo (índices) das linhas da página `page` (já limitada com [`clamp_page`]).
pub fn page_range(total: usize, page: usize, rows_per_page: usize) -> Range<usize> {
    if rows_per_page == 0 {
        return 0..total;
    }
    let page = clamp_page(page, total, rows_per_page);
    let start = ((page - 1) * rows_per_page).min(total);
    start..(start + rows_per_page).min(total)
}

/// Faixa do rodapé: `"1–10 de 42"` (`"0 de 0"` sem linhas). `of` é a palavra entre os números.
pub fn range_label(total: usize, page: usize, rows_per_page: usize, of: &str) -> String {
    if total == 0 {
        return format!("0 {of} 0");
    }
    let range = page_range(total, page, rows_per_page);
    format!("{}–{} {of} {total}", range.start + 1, range.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_cycles_ascending_descending_none() {
        let p = TablePagination::new(10);
        let p = toggle_sort(&p, "name");
        assert_eq!(sort_state(&p, "name"), SortState::Ascending);
        let p = toggle_sort(&p, "name");
        assert_eq!(sort_state(&p, "name"), SortState::Descending);
        let p = toggle_sort(&p, "name");
        assert_eq!(sort_state(&p, "name"), SortState::None);
        assert_eq!(p.sort_by, None);
        assert!(!p.descending);
    }

    #[test]
    fn switching_column_restarts_ascending_and_goes_to_page_one() {
        let mut p = TablePagination::new(5).sorted_by("name", true);
        p.page = 3;
        let p = toggle_sort(&p, "age");
        assert_eq!(p.sort_by.as_deref(), Some("age"));
        assert!(!p.descending);
        assert_eq!(p.page, 1);
        assert_eq!(sort_state(&p, "name"), SortState::None);
    }

    #[test]
    fn aria_values() {
        assert_eq!(SortState::None.aria(), "none");
        assert_eq!(SortState::Ascending.aria(), "ascending");
        assert_eq!(SortState::Descending.aria(), "descending");
    }

    #[test]
    fn text_comparison_ignores_case_and_is_total() {
        assert_eq!(compare_text("ana", "Bia"), Ordering::Less);
        assert_eq!(compare_text("Bia", "ana"), Ordering::Greater);
        assert_eq!(compare_text("ana", "ana"), Ordering::Equal);
        assert_ne!(compare_text("ana", "Ana"), Ordering::Equal);
    }

    #[test]
    fn sorts_stably_and_reverses() {
        let data = ["b", "a", "c", "a"];
        let cmp = |i: usize, j: usize| data[i].cmp(data[j]);
        assert_eq!(sorted_indices(4, Some(&cmp), false), vec![1, 3, 0, 2]);
        // descendente: os empates (1 e 3) mantêm a ordem original
        assert_eq!(sorted_indices(4, Some(&cmp), true), vec![2, 0, 1, 3]);
        assert_eq!(sorted_indices(4, None, true), vec![0, 1, 2, 3]);
        assert!(sorted_indices(0, Some(&cmp), false).is_empty());
    }

    #[test]
    fn page_count_and_clamp() {
        assert_eq!(page_count(42, 10), 5);
        assert_eq!(page_count(40, 10), 4);
        assert_eq!(page_count(0, 10), 1);
        assert_eq!(page_count(42, 0), 1);
        assert_eq!(clamp_page(0, 42, 10), 1);
        assert_eq!(clamp_page(9, 42, 10), 5);
        assert_eq!(clamp_page(3, 42, 0), 1);
    }

    #[test]
    fn page_ranges() {
        assert_eq!(page_range(42, 1, 10), 0..10);
        assert_eq!(page_range(42, 5, 10), 40..42);
        assert_eq!(page_range(42, 99, 10), 40..42);
        assert_eq!(page_range(42, 2, 0), 0..42);
        assert_eq!(page_range(0, 1, 10), 0..0);
    }

    #[test]
    fn range_text() {
        assert_eq!(range_label(42, 1, 10, "de"), "1–10 de 42");
        assert_eq!(range_label(42, 5, 10, "de"), "41–42 de 42");
        assert_eq!(range_label(7, 1, 0, "de"), "1–7 de 7");
        assert_eq!(range_label(0, 1, 10, "de"), "0 de 0");
    }
}
