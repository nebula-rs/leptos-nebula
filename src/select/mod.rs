//! `Select` e `SelectMultiple` (≈ `QSelect` do Quasar): um `combobox` com lista de opções.
//!
//! Especificação: `docs/ui/forms.md#select`. A navegação por teclado, o type-ahead e a
//! alternância da seleção ficam em [`nav`], sem DOM e sem Leptos, testados no host; [`control`]
//! liga isso ao DOM e ao popup; [`component`] são os dois componentes públicos.

mod component;
mod control;
mod nav;

pub use component::{Select, SelectMultiple};

/// Uma opção de um [`Select`] ou [`SelectMultiple`]. ≈ `{ label, value, disable }` do Quasar
/// (já com `emit_value` e `map_options`: `value` é o que vai para o modelo).
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption<T> {
    /// Texto da opção (e o que o campo exibe quando ela está escolhida).
    pub label: String,
    /// O valor que a opção grava no modelo.
    pub value: T,
    /// Opção desabilitada: aparece, mas não é escolhida nem recebe a navegação.
    pub disable: bool,
}

impl<T> SelectOption<T> {
    /// Opção habilitada.
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
            disable: false,
        }
    }

    /// Esta mesma opção, desabilitada.
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.disable = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::SelectOption;

    #[test]
    fn options_are_enabled_unless_disabled() {
        let o = SelectOption::new("Maçã", 1);
        assert_eq!((o.label.as_str(), o.value, o.disable), ("Maçã", 1, false));
        assert!(SelectOption::new("Pera", 2).disabled().disable);
    }
}
