//! `Input` — `<input>`/`<textarea>` dentro de um [`Field`](crate::Field) (≈ `QInput` do Quasar).
//!
//! Especificação: `docs/ui/forms.md#input`. Aqui ficam o [`InputType`] (lógica pura, testada no
//! host); o componente, o `InputHandle` e a geração do `id` estão em [`component`].

mod autogrow;
mod component;
pub(crate) mod emit;
mod mask;
mod masking;

pub use component::{Input, InputHandle};
pub use mask::FillMask;

/// Tipo do controle nativo. `Textarea` troca o `<input>` por um `<textarea>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputType {
    /// `type="text"`.
    #[default]
    Text,
    /// `type="password"`.
    Password,
    /// `type="email"`.
    Email,
    /// `type="search"`.
    Search,
    /// `type="tel"`.
    Tel,
    /// `type="url"`.
    Url,
    /// `type="number"`. O valor continua sendo `String` (o app converte).
    Number,
    /// `<textarea>`.
    Textarea,
    /// `type="date"`: o label fica sempre flutuando.
    Date,
    /// `type="time"`: o label fica sempre flutuando.
    Time,
    /// `type="datetime-local"`: o label fica sempre flutuando.
    DatetimeLocal,
}

impl InputType {
    /// Valor do atributo `type` (`None` para `Textarea`, que não tem atributo `type`).
    pub(crate) fn attr(self) -> Option<&'static str> {
        match self {
            InputType::Text => Some("text"),
            InputType::Password => Some("password"),
            InputType::Email => Some("email"),
            InputType::Search => Some("search"),
            InputType::Tel => Some("tel"),
            InputType::Url => Some("url"),
            InputType::Number => Some("number"),
            InputType::Textarea => None,
            InputType::Date => Some("date"),
            InputType::Time => Some("time"),
            InputType::DatetimeLocal => Some("datetime-local"),
        }
    }

    /// O controle é um `<textarea>`.
    pub(crate) fn is_textarea(self) -> bool {
        matches!(self, InputType::Textarea)
    }

    /// O navegador sempre mostra algo no controle (`dd/mm/aaaa`…), então o label não pode ficar
    /// em repouso por cima dele: flutua mesmo sem valor (ADR-0010).
    pub(crate) fn always_floats(self) -> bool {
        matches!(
            self,
            InputType::Date | InputType::Time | InputType::DatetimeLocal
        )
    }
}

#[cfg(test)]
mod tests {
    use super::InputType;

    #[test]
    fn default_is_text() {
        assert_eq!(InputType::default(), InputType::Text);
        assert_eq!(InputType::default().attr(), Some("text"));
    }

    #[test]
    fn attr_names() {
        assert_eq!(InputType::Password.attr(), Some("password"));
        assert_eq!(InputType::Email.attr(), Some("email"));
        assert_eq!(InputType::Search.attr(), Some("search"));
        assert_eq!(InputType::Tel.attr(), Some("tel"));
        assert_eq!(InputType::Url.attr(), Some("url"));
        assert_eq!(InputType::Number.attr(), Some("number"));
        assert_eq!(InputType::Date.attr(), Some("date"));
        assert_eq!(InputType::Time.attr(), Some("time"));
        assert_eq!(InputType::DatetimeLocal.attr(), Some("datetime-local"));
    }

    #[test]
    fn textarea_has_no_type_attr() {
        assert_eq!(InputType::Textarea.attr(), None);
        assert!(InputType::Textarea.is_textarea());
        assert!(!InputType::Text.is_textarea());
    }

    #[test]
    fn only_date_and_time_types_always_float() {
        for t in [InputType::Date, InputType::Time, InputType::DatetimeLocal] {
            assert!(t.always_floats());
        }
        for t in [
            InputType::Text,
            InputType::Password,
            InputType::Email,
            InputType::Search,
            InputType::Tel,
            InputType::Url,
            InputType::Number,
            InputType::Textarea,
        ] {
            assert!(!t.always_floats());
        }
    }
}
