//! Escolha do elemento que recebe o foco do `<form>` (porta de `QForm.focus` do Quasar).

use wasm_bindgen::JsCast;
use web_sys::{FocusOptions, HtmlElement, HtmlFormElement};

/// Foca o primeiro elemento adequado dentro do formulário, por ordem de preferência:
/// `[autofocus][tabindex]`/`[data-autofocus][tabindex]`; um `tabindex` dentro de
/// `[autofocus]`/`[data-autofocus]`; o próprio `[autofocus]`/`[data-autofocus]`; o primeiro
/// elemento com `tabindex` diferente de -1. Sem nenhum, não faz nada. Não rola a página.
pub(crate) fn focus_form(form: &HtmlFormElement) {
    let selectors = [
        "[autofocus][tabindex], [data-autofocus][tabindex]",
        "[autofocus] [tabindex], [data-autofocus] [tabindex]",
        "[autofocus], [data-autofocus]",
    ];
    let target = selectors
        .into_iter()
        .find_map(|sel| form.query_selector(sel).ok().flatten())
        .and_then(|el| el.dyn_into::<HtmlElement>().ok())
        .or_else(first_focusable_by_tabindex(form));

    if let Some(el) = target {
        let opts = FocusOptions::new();
        opts.set_prevent_scroll(true);
        let _ = el.focus_with_options(&opts);
    }
}

fn first_focusable_by_tabindex(form: &HtmlFormElement) -> impl FnOnce() -> Option<HtmlElement> {
    let list = form.query_selector_all("[tabindex]").ok();
    move || {
        let list = list?;
        (0..list.length())
            .filter_map(|i| list.item(i)?.dyn_into::<HtmlElement>().ok())
            .find(|el| el.tab_index() != -1)
    }
}
