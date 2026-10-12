//! Utilitários internos.

/// Junta pedaços de classe **literais** ignorando os vazios.
///
/// As classes continuam literais nos componentes (o UnoCSS lê o `.rs` como texto);
/// aqui só se concatena o que já foi escolhido com `match`.
pub(crate) fn cls<const N: usize>(parts: [&str; N]) -> String {
    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::cls;

    #[test]
    fn joins_non_empty() {
        assert_eq!(cls(["a", "", "b"]), "a b");
        assert_eq!(cls(["", ""]), "");
    }
}
