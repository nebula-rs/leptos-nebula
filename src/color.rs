//! Cores semânticas, com os nomes do Quasar (`color="primary"`), mais `Neutral` (nosso).

/// Papel de cor de um componente.
///
/// A classe de cor só define variáveis locais (`--lx-tone`, `--lx-tone-contrast`,
/// `--lx-tone-soft`); o design (`unelevated`, `outline`, `flat`…) lê essas variáveis.
/// Assim cor e design nunca disputam a mesma propriedade.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Color {
    #[default]
    Primary,
    Secondary,
    /// ≈ `success` do tema.
    Positive,
    /// ≈ `danger` do tema.
    Negative,
    Info,
    Warning,
    /// Não existe no Quasar: cinza/tinta do tema.
    Neutral,
}

impl Color {
    pub(crate) fn tone_class(self) -> &'static str {
        match self {
            Color::Primary => "lx-tone-primary",
            Color::Secondary => "lx-tone-secondary",
            Color::Positive => "lx-tone-positive",
            Color::Negative => "lx-tone-negative",
            Color::Info => "lx-tone-info",
            Color::Warning => "lx-tone-warning",
            Color::Neutral => "lx-tone-neutral",
        }
    }
}

/// Alinhamento horizontal (Quasar: `align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Between,
    Around,
    Evenly,
    Justify,
}
