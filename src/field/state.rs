//! Lógica pura do `Field`: estado do label e escolha das classes (sem DOM, sem Leptos).
//!
//! Toda classe é literal e escolhida por `match` (o UnoCSS lê o `.rs` como texto); cada estado
//! exclusivo é um shortcut **completo** em `uno/shortcuts.ts`.

/// Aparência do quadro do campo (exclusivas). `Outlined` é o padrão — desvio do Quasar, cujo
/// padrão é `standard` (ADR-0010). `Standard` e `Standout` entram depois sem quebrar quem usa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldDesign {
    /// Caixa com borda.
    #[default]
    Outlined,
    /// Caixa com fundo e linha na base.
    Filled,
    /// Sem caixa nem borda (o foco aparece como linha na base).
    Borderless,
}

/// O label "flutua" (ou fica na posição de repouso, conforme o tema) quando o campo tem foco,
/// tem valor ou `stack_label`. Calculado aqui — não por CSS (`:focus-within`,
/// `:placeholder-shown`) — para valer também com `stack_label` e tipos que sempre mostram o
/// controle (`date`, `time`): ADR-0010.
pub(crate) fn is_floating(focused: bool, has_value: bool, stack_label: bool) -> bool {
    focused || has_value || stack_label
}

/// O hint aparece sempre, ou só com foco quando `hide_hint`.
pub(crate) fn hint_visible(hide_hint: bool, focused: bool) -> bool {
    !hide_hint || focused
}

/// Classe da caixa do controle: design × foco (shortcuts completos).
pub(crate) fn control_class(design: FieldDesign, focused: bool) -> &'static str {
    match (design, focused) {
        (FieldDesign::Outlined, false) => "lx-field-outlined-idle",
        (FieldDesign::Outlined, true) => "lx-field-outlined-focused",
        (FieldDesign::Filled, false) => "lx-field-filled-idle",
        (FieldDesign::Filled, true) => "lx-field-filled-focused",
        (FieldDesign::Borderless, false) => "lx-field-borderless-idle",
        (FieldDesign::Borderless, true) => "lx-field-borderless-focused",
    }
}

/// Densidade (altura e recuo).
pub(crate) fn size_class(dense: bool) -> &'static str {
    if dense {
        "lx-field-size-dense"
    } else {
        "lx-field-size-normal"
    }
}

/// Estado de edição do campo inteiro.
pub(crate) fn editable_class(disable: bool, readonly: bool) -> &'static str {
    match (disable, readonly) {
        (true, _) => "lx-field-disabled",
        (false, true) => "lx-field-readonly",
        (false, false) => "lx-field-editable",
    }
}

/// Com label, o campo reserva o espaço do tema acima da caixa (`--lx-field-label-space`).
pub(crate) fn label_space_class(has_label: bool) -> &'static str {
    if has_label {
        "lx-field-labeled"
    } else {
        "lx-field-plain"
    }
}

/// Posição do label (dois estados exclusivos; as posições vêm dos tokens do tema).
pub(crate) fn label_class(floating: bool) -> &'static str {
    if floating {
        "lx-field-label-float"
    } else {
        "lx-field-label-rest"
    }
}

/// Opacidade do `placeholder`: some só enquanto há label em repouso (token do tema); sem label,
/// ou com o label flutuando, o placeholder aparece.
pub(crate) fn placeholder_class(has_label: bool, floating: bool) -> &'static str {
    if has_label && !floating {
        "lx-field-placeholder-rest"
    } else {
        "lx-field-placeholder-shown"
    }
}

/// Rodapé: reserva a altura de uma linha (para o layout não pular quando o hint aparece),
/// salvo `hide_bottom_space`.
pub(crate) fn bottom_class(hide_bottom_space: bool) -> &'static str {
    if hide_bottom_space {
        "lx-field-bottom-free"
    } else {
        "lx-field-bottom-reserved"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floating_when_focused_filled_or_stacked() {
        assert!(!is_floating(false, false, false));
        assert!(is_floating(true, false, false));
        assert!(is_floating(false, true, false));
        assert!(is_floating(false, false, true));
        assert!(is_floating(true, true, true));
    }

    #[test]
    fn hint_hidden_only_without_focus_when_hide_hint() {
        assert!(hint_visible(false, false));
        assert!(hint_visible(false, true));
        assert!(!hint_visible(true, false));
        assert!(hint_visible(true, true));
    }

    #[test]
    fn default_design_is_outlined() {
        assert_eq!(FieldDesign::default(), FieldDesign::Outlined);
    }

    #[test]
    fn control_class_covers_every_design_and_focus() {
        let designs = [
            FieldDesign::Outlined,
            FieldDesign::Filled,
            FieldDesign::Borderless,
        ];
        let mut seen = std::collections::HashSet::new();
        for d in designs {
            for f in [false, true] {
                assert!(seen.insert(control_class(d, f)), "classe repetida");
            }
        }
        assert_eq!(seen.len(), 6);
        assert_eq!(
            control_class(FieldDesign::Outlined, true),
            "lx-field-outlined-focused"
        );
        assert_eq!(
            control_class(FieldDesign::Filled, false),
            "lx-field-filled-idle"
        );
    }

    #[test]
    fn size_and_label_classes() {
        assert_eq!(size_class(false), "lx-field-size-normal");
        assert_eq!(size_class(true), "lx-field-size-dense");
        assert_eq!(label_class(false), "lx-field-label-rest");
        assert_eq!(label_class(true), "lx-field-label-float");
    }

    #[test]
    fn disable_wins_over_readonly() {
        assert_eq!(editable_class(false, false), "lx-field-editable");
        assert_eq!(editable_class(false, true), "lx-field-readonly");
        assert_eq!(editable_class(true, false), "lx-field-disabled");
        assert_eq!(editable_class(true, true), "lx-field-disabled");
    }

    #[test]
    fn label_space_only_with_label() {
        assert_eq!(label_space_class(true), "lx-field-labeled");
        assert_eq!(label_space_class(false), "lx-field-plain");
    }

    #[test]
    fn placeholder_hidden_only_with_resting_label() {
        assert_eq!(placeholder_class(true, false), "lx-field-placeholder-rest");
        assert_eq!(placeholder_class(true, true), "lx-field-placeholder-shown");
        assert_eq!(
            placeholder_class(false, false),
            "lx-field-placeholder-shown"
        );
        assert_eq!(placeholder_class(false, true), "lx-field-placeholder-shown");
    }

    #[test]
    fn bottom_reserves_space_unless_hidden() {
        assert_eq!(bottom_class(false), "lx-field-bottom-reserved");
        assert_eq!(bottom_class(true), "lx-field-bottom-free");
    }
}
