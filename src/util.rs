//! Utilitários internos.

use std::sync::atomic::{AtomicUsize, Ordering};

use leptos::prelude::Owner;

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

/// Contador só para o navegador sem SSR (`csr`), onde não há `SharedContext`.
static CSR_IDS: AtomicUsize = AtomicUsize::new(0);

/// `id` gerado (`<prefixo>-<n>`) para elementos que precisam de um `id` estável (`<label for>`,
/// `aria-describedby`…).
///
/// Seguro para hidratação: com SSR/SSG o número vem do `SharedContext` do Leptos (`next_id`),
/// uma sequência por requisição/página que o servidor e o cliente percorrem na mesma ordem (a
/// mesma que o Leptos usa para os `Resource`); um contador global não serviria, pois o SSG
/// renderiza várias páginas no mesmo processo. Sem `SharedContext` (`csr` puro) usa um contador
/// atômico, que não precisa casar com nada.
pub(crate) fn next_id(prefix: &str) -> String {
    let n = match Owner::current_shared_context() {
        Some(ctx) => ctx.next_id().into_inner(),
        None => CSR_IDS.fetch_add(1, Ordering::Relaxed),
    };
    format!("{prefix}-{n}")
}

/// Lê um valor de `transition-duration` (lista separada por vírgulas, em `s` ou `ms`) e devolve
/// a maior duração, em ms. Valor ilegível conta como 0.
pub(crate) fn parse_duration_ms(css: &str) -> u32 {
    css.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if let Some(ms) = part.strip_suffix("ms") {
                ms.trim().parse::<f64>().ok()
            } else if let Some(s) = part.strip_suffix('s') {
                s.trim().parse::<f64>().ok().map(|v| v * 1000.0)
            } else {
                None
            }
        })
        .fold(0.0_f64, f64::max)
        .round() as u32
}

#[cfg(test)]
mod tests {
    use super::{cls, parse_duration_ms};

    #[test]
    fn parses_the_longest_transition_duration() {
        assert_eq!(parse_duration_ms("0.25s"), 250);
        assert_eq!(parse_duration_ms("250ms"), 250);
        assert_eq!(parse_duration_ms("0.15s, 0.3s"), 300);
        assert_eq!(parse_duration_ms("0s"), 0);
        assert_eq!(parse_duration_ms(""), 0);
        assert_eq!(parse_duration_ms("auto"), 0);
    }

    #[test]
    fn joins_non_empty() {
        assert_eq!(cls(["a", "", "b"]), "a b");
        assert_eq!(cls(["", ""]), "");
    }
}
