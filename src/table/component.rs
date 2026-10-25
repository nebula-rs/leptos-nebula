use std::cmp::Ordering;
use std::sync::Arc;

use leptos::prelude::*;

use super::model::{
    SortState, TablePagination, clamp_page, compare_text, page_count, page_range, range_label,
    sort_state, sorted_indices, toggle_sort,
};
use crate::{
    Align, Btn, BtnDesign, EmptyState, LinearProgress, MarkupSeparator, MarkupTable, Spinner,
    util::{cls, next_id},
};

type FieldFn<T> = Arc<dyn Fn(&T) -> String + Send + Sync>;
type SortFn<T> = Arc<dyn Fn(&T, &T) -> Ordering + Send + Sync>;
type CellFn<T> = Arc<dyn Fn(&T) -> AnyView + Send + Sync>;

/// Uma coluna da [`Table`]. ≈ um item de `columns` do `QTable`.
///
/// Monte com [`TableColumn::new`] e os métodos encadeados. `name` identifica a coluna na
/// ordenação (`TablePagination::sort_by`) e precisa ser único.
pub struct TableColumn<T> {
    name: String,
    label: String,
    field: FieldFn<T>,
    align: Align,
    sortable: bool,
    sort: Option<SortFn<T>>,
    cell: Option<CellFn<T>>,
}

impl<T> Clone for TableColumn<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            label: self.label.clone(),
            field: self.field.clone(),
            align: self.align,
            sortable: self.sortable,
            sort: self.sort.clone(),
            cell: self.cell.clone(),
        }
    }
}

impl<T> TableColumn<T> {
    /// Coluna `name` com o rótulo `label` e o texto da célula devolvido por `field`.
    pub fn new(
        name: impl Into<String>,
        label: impl Into<String>,
        field: impl Fn(&T) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            field: Arc::new(field),
            align: Align::Left,
            sortable: false,
            sort: None,
            cell: None,
        }
    }

    /// Alinhamento do texto: só `Left`, `Center` e `Right` valem (os demais contam como `Left`).
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Cabeçalho ordenável (botão com `aria-sort`).
    #[must_use]
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Comparador próprio (crescente) da coluna ordenável; sem ele, compara o texto de `field`
    /// sem diferenciar maiúsculas de minúsculas.
    #[must_use]
    pub fn sort(mut self, cmp: impl Fn(&T, &T) -> Ordering + Send + Sync + 'static) -> Self {
        self.sort = Some(Arc::new(cmp));
        self
    }

    /// Célula customizada (≈ slot `body-cell-<name>`); sem ela, a célula mostra o texto de
    /// `field`. Chame `.into_any()` na view devolvida.
    #[must_use]
    pub fn cell(mut self, cell: impl Fn(&T) -> AnyView + Send + Sync + 'static) -> Self {
        self.cell = Some(Arc::new(cell));
        self
    }
}

/// Textos fixos do rodapé de paginação (para traduzir ou trocar).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TablePaginationLabels {
    /// Palavra entre os números da faixa ("1–10 **de** 42").
    pub of: &'static str,
    /// `aria-label` da navegação.
    pub navigation: &'static str,
    pub first_page: &'static str,
    pub previous_page: &'static str,
    pub next_page: &'static str,
    pub last_page: &'static str,
}

impl Default for TablePaginationLabels {
    fn default() -> Self {
        Self {
            of: "de",
            navigation: "Paginação",
            first_page: "Primeira página",
            previous_page: "Página anterior",
            next_page: "Próxima página",
            last_page: "Última página",
        }
    }
}

fn align_class(align: Align) -> &'static str {
    match align {
        Align::Center => "lx-table-align-center",
        Align::Right => "lx-table-align-right",
        _ => "lx-table-align-left",
    }
}

/// Cabeçalho de uma coluna (`<th scope="col">`), com botão de ordenação quando `sortable`.
fn header_cell<T: 'static>(col: &TableColumn<T>, pag: RwSignal<TablePagination>) -> AnyView {
    let align = align_class(col.align);
    if !col.sortable {
        return view! { <th scope="col" class=align>{col.label.clone()}</th> }.into_any();
    }
    let name = col.name.clone();
    let state = {
        let name = name.clone();
        move || pag.with(|p| sort_state(p, &name))
    };
    let icon_class = {
        let state = state.clone();
        move || match state() {
            SortState::None => "lx-table-sort-icon lx-table-sort-icon-idle i-lucide-arrow-up",
            SortState::Ascending => "lx-table-sort-icon lx-table-sort-icon-asc i-lucide-arrow-up",
            SortState::Descending => "lx-table-sort-icon lx-table-sort-icon-desc i-lucide-arrow-up",
        }
    };
    view! {
        <th scope="col" class=align aria-sort=move || state().aria()>
            <button
                type="button"
                class="lx-table-sort"
                on:click=move |_| pag.update(|p| *p = toggle_sort(p, &name))
            >
                {col.label.clone()}
                <span class=icon_class aria-hidden="true"></span>
            </button>
        </th>
    }
    .into_any()
}

/// Tabela de dados. ≈ `QTable` (parcial).
///
/// `rows` são as linhas; cada coluna ([`TableColumn`]) diz como extrair o texto (`field`) ou como
/// desenhar a célula (`cell`). `row_key` devolve uma chave estável da linha (reaproveita o DOM ao
/// reordenar; uma mudança de dado que não altere a chave nem o texto das colunas não redesenha a
/// linha).
///
/// Ordenação e paginação são no cliente e vivem em `pagination` (`TablePagination`: coluna,
/// sentido, página, linhas por página; `0` = sem paginação). Sem `pagination`, a tabela mostra
/// todas as linhas e ainda ordena. O cabeçalho ordenável é um `<button>` dentro do `<th>` (teclado:
/// `Enter`/`Espaço`) com `aria-sort`; o ciclo é ascendente, descendente, sem ordem.
///
/// Reaproveita o [`MarkupTable`] para o estilo das células (`dense`, `separator`, `wrap_cells`) e
/// a moldura (`flat`, `bordered`) fica no quadro externo. `loading` mostra um
/// [`LinearProgress`] indeterminado no topo e `aria-busy` na tabela; sem linhas, o quadro mostra
/// um [`EmptyState`] (ou o [`Spinner`], se carregando).
///
/// Ainda não: seleção, filtro, rolagem virtual, grid mode, paginação no servidor, cabeçalho fixo,
/// expansão de linha, seletor de linhas por página, slots `top`/`bottom`.
#[component]
pub fn Table<T, K>(
    /// As linhas.
    #[prop(into)]
    rows: Signal<Vec<T>>,
    /// As colunas, na ordem de exibição.
    columns: Vec<TableColumn<T>>,
    /// Chave estável de uma linha.
    row_key: K,
    /// Título do quadro; também é o nome acessível da tabela.
    #[prop(optional, into)]
    title: Option<String>,
    /// Nome acessível da tabela quando não há `title`.
    #[prop(optional, into)]
    aria_label: Option<String>,
    #[prop(optional)] dense: bool,
    #[prop(optional)] flat: bool,
    #[prop(optional)] bordered: bool,
    /// Linhas entre as células (o mesmo tipo do [`MarkupTable`]).
    #[prop(optional)]
    separator: MarkupSeparator,
    /// Permite quebra de linha no texto das células.
    #[prop(optional)]
    wrap_cells: bool,
    /// Carregando: barra indeterminada no topo e `aria-busy`.
    #[prop(optional, into)]
    loading: Signal<bool>,
    /// Texto do estado sem linhas.
    #[prop(default = "Nenhum dado".to_string(), into)]
    no_data_label: String,
    /// Nome acessível da barra e do indicador de carregamento.
    #[prop(default = "Carregando")]
    loading_label: &'static str,
    /// Ordenação e paginação; o componente grava nele ao ordenar e ao trocar de página.
    #[prop(optional)]
    pagination: Option<RwSignal<TablePagination>>,
    #[prop(default = TablePaginationLabels::default())] pagination_labels: TablePaginationLabels,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
    K: Fn(&T) -> String + Send + Sync + 'static,
{
    let pag = pagination.unwrap_or_else(|| RwSignal::new(TablePagination::default()));
    let cols = StoredValue::new(columns);
    let key_fn = Arc::new(row_key);
    let title_id = next_id("lx-table-title");
    let labelled_by = title.is_some().then(|| title_id.clone());

    let total = move || rows.with(Vec::len);
    let empty = Memo::new(move |_| total() == 0);
    let per_page = move || pag.with(|p| p.rows_per_page);
    let paged = move || per_page() > 0 && !empty.get();
    let page = move || clamp_page(pag.with(|p| p.page), total(), per_page());
    let pages = move || page_count(total(), per_page());

    // Índices das linhas na ordem da coluna ordenada (só recalcula ao mudar a ordem ou as linhas).
    let sort_spec = Memo::new(move |_| pag.with(|p| (p.sort_by.clone(), p.descending)));
    let order = Memo::new(move |_| {
        let (by, descending) = sort_spec.get();
        let col = by.and_then(|by| {
            cols.with_value(|cols| cols.iter().find(|c| c.sortable && c.name == by).cloned())
        });
        rows.with(|rows| match col {
            None => sorted_indices(rows.len(), None, false),
            Some(col) => match col.sort.as_ref() {
                Some(cmp) => {
                    let cmp = |i: usize, j: usize| cmp(&rows[i], &rows[j]);
                    sorted_indices(rows.len(), Some(&cmp), descending)
                }
                None => {
                    let texts: Vec<String> = rows.iter().map(|r| (col.field)(r)).collect();
                    let cmp = |i: usize, j: usize| compare_text(&texts[i], &texts[j]);
                    sorted_indices(rows.len(), Some(&cmp), descending)
                }
            },
        })
    });
    // Linhas da página atual.
    let visible = move || {
        let order = order.get();
        let range = page_range(order.len(), page(), per_page());
        rows.with(|rows| {
            order[range]
                .iter()
                .filter_map(|&i| rows.get(i).cloned())
                .collect::<Vec<T>>()
        })
    };
    let key = move |row: &T| {
        let mut k = key_fn(row);
        cols.with_value(|cols| {
            for c in cols {
                k.push('\u{1f}');
                k.push_str(&(c.field)(row));
            }
        });
        k
    };
    let render_row = move |row: T| {
        let cells = cols.with_value(|cols| {
            cols.iter()
                .map(|c| {
                    let content = match c.cell.as_ref() {
                        Some(cell) => cell(&row),
                        None => (c.field)(&row).into_any(),
                    };
                    view! { <td class=align_class(c.align)>{content}</td> }
                })
                .collect_view()
        });
        view! { <tr>{cells}</tr> }
    };

    let headers = cols.with_value(|cols| cols.iter().map(|c| header_cell(c, pag)).collect_view());

    let elevation = if flat { "lx-mt-flat" } else { "lx-mt-raised" };
    let border = if bordered {
        "lx-mt-bordered"
    } else {
        "lx-mt-unbordered"
    };
    let go = move |target: usize| {
        // `pages()` lê `pag`: calcula antes de escrever (ler dentro do `update` panica).
        let target = target.clamp(1, pages());
        pag.update(|p| p.page = target);
    };
    let labels = pagination_labels;
    let no_data_label = StoredValue::new(no_data_label);

    view! {
        <div class=cls(["lx-table", elevation, border])>
            {title
                .map(|t| view! { <div class="lx-table-title" id=title_id.clone()>{t}</div> })}
            <div class="lx-table-progress">
                <Show when=move || loading.get()>
                    <LinearProgress
                        indeterminate=true
                        size="var(--lx-table-progress-height)"
                        aria_label=loading_label
                    />
                </Show>
            </div>
            <MarkupTable dense=dense flat=true separator=separator wrap_cells=wrap_cells>
                <table
                    aria-labelledby=labelled_by
                    aria-label=aria_label
                    aria-busy=move || loading.get().then_some("true")
                >
                    <thead>
                        <tr>{headers}</tr>
                    </thead>
                    <tbody>
                        <For each=visible key=key children=render_row />
                    </tbody>
                </table>
            </MarkupTable>
            <Show when=move || empty.get()>
                <div class="lx-table-empty">
                    {move || {
                        if loading.get() {
                            view! {
                                <div class="lx-table-loading">
                                    <Spinner size="1.5em" aria_label=loading_label />
                                    <span>{loading_label}</span>
                                </div>
                            }
                                .into_any()
                        } else {
                            view! {
                                <EmptyState
                                    icon="i-lucide-inbox"
                                    title=no_data_label.get_value()
                                />
                            }
                                .into_any()
                        }
                    }}
                </div>
            </Show>
            <Show when=paged>
                <div class="lx-table-footer">
                    <span class="lx-table-range" aria-live="polite">
                        {move || range_label(total(), page(), per_page(), labels.of)}
                    </span>
                    <nav class="lx-table-pages" aria-label=labels.navigation>
                        <Btn
                            round=true
                            dense=true
                            design=BtnDesign::Flat
                            icon="i-lucide-chevrons-left"
                            aria_label=labels.first_page
                            disable=Signal::derive(move || page() <= 1)
                            on_click=Callback::new(move |_| go(1))
                        />
                        <Btn
                            round=true
                            dense=true
                            design=BtnDesign::Flat
                            icon="i-lucide-chevron-left"
                            aria_label=labels.previous_page
                            disable=Signal::derive(move || page() <= 1)
                            on_click=Callback::new(move |_| go(page().saturating_sub(1)))
                        />
                        <Btn
                            round=true
                            dense=true
                            design=BtnDesign::Flat
                            icon="i-lucide-chevron-right"
                            aria_label=labels.next_page
                            disable=Signal::derive(move || page() >= pages())
                            on_click=Callback::new(move |_| go(page() + 1))
                        />
                        <Btn
                            round=true
                            dense=true
                            design=BtnDesign::Flat
                            icon="i-lucide-chevrons-right"
                            aria_label=labels.last_page
                            disable=Signal::derive(move || page() >= pages())
                            on_click=Callback::new(move |_| go(pages()))
                        />
                    </nav>
                </div>
            </Show>
        </div>
    }
}
