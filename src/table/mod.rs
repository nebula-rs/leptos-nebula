//! `Table`: tabela de dados com ordenação e paginação no cliente. ≈ `QTable` (parcial).
//!
//! [`model`] é a lógica pura (ordenação, páginas, faixa "1–10 de 42"), testada no host;
//! [`component`] liga isso ao DOM.

mod component;
mod model;

pub use component::{Table, TableColumn, TablePaginationLabels};
pub use model::TablePagination;
