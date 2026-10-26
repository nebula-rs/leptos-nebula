//! `<Select>` e `<SelectMultiple>`: o mesmo controle (`control.rs`), com o modelo adaptado.

use std::sync::Arc;

use leptos::prelude::*;

use super::control::{Config, Model, generated_id, select_field};
use crate::field::use_validate_with;
use crate::{
    Color, FieldAfter, FieldAppend, FieldBefore, FieldDesign, FieldError, FieldHint, FieldLoading,
    FieldPrepend, LazyRules, Options, Rule, ValidationOptions,
};

/// `Select` de uma opção. ≈ `QSelect` sem `multiple`.
///
/// `value` é o `v-model`: `Some(valor)` com uma opção escolhida, `None` sem escolha. `options`
/// é a lista de [`SelectOption`] (já no formato `emit_value` + `map_options` do Quasar: o modelo
/// recebe o `value` da opção, não a opção). A lista abre num popup debaixo do campo, com a largura
/// dele, e fecha ao escolher.
///
/// O campo é um [`Field`](crate::Field): todas as props visuais (`label`, `hint`, `design`…) e de
/// validação (`rules`, `lazy_rules`, `error`, `error_message`, `no_error_icon`) valem, e dentro de
/// um [`Form`](crate::Form) o `Select` se registra sozinho. As `rules` recebem `&Option<T>`.
///
/// **Teclado** (o foco fica no campo): `Enter`, `Espaço`, `↓` e `↑` abrem; `↑`/`↓`/`Home`/`End`
/// andam pelas opções, pulando as desabilitadas; digitar letras vai à opção cujo label começa com
/// elas (type-ahead); `Enter`/`Espaço` escolhem; `Esc` fecha; `Tab` fecha e segue.
///
/// **Acessibilidade:** o campo é um `<input readonly role="combobox">` com `aria-expanded`,
/// `aria-controls` e `aria-activedescendant` (a opção ativa); a lista é um `role="listbox"` de
/// `role="option"` com `aria-selected` e `aria-disabled`. Dê um nome ao campo com `label` ou
/// `aria_label`.
///
/// Ainda não: `use_input` (o `use_filter` é a busca **embutida**, ADR-0013), `use_chips`, `new_value_mode`, rolagem virtual,
/// `options_dense`, `options_cover`, `menu_anchor`/`menu_self`/`menu_offset`, `behavior`
/// (diálogo no celular), `hide_selected`, `max_values`, os slots `option`/`selected`/`no_option`,
/// `option_value`/`option_label`/`option_disable` (use `SelectOption`), `name` (nenhum valor vai
/// no envio nativo do `<form>`).
#[component]
pub fn Select<T>(
    /// O valor escolhido (`v-model`).
    value: RwSignal<Option<T>>,
    /// As opções.
    #[prop(into)]
    options: Options<T>,
    /// `id` do campo (o `for` do label). Sem ele, o `Select` gera `lx-select-<n>`.
    #[prop(optional, into)]
    id: Option<String>,
    /// Nome do campo no `Form`, para diagnóstico.
    #[prop(optional, into)]
    name: Option<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    /// Sobrepõe o texto do campo (o padrão é o label da opção escolhida).
    #[prop(optional, into)]
    display_value: MaybeProp<String>,
    /// Botão que limpa a escolha (aparece com valor, fora de `disable`/`readonly`).
    #[prop(optional)]
    clearable: bool,
    /// Esconde a seta do campo.
    #[prop(optional)]
    hide_dropdown_icon: bool,
    /// Texto da lista sem opções.
    #[prop(optional, into)]
    no_options_label: Option<String>,
    /// Caixa de busca no topo do popup (busca AICI pelo rótulo: sem diferenciar maiúsculas nem
    /// acentos, "contém"). O foco entra na caixa ao abrir; `Enter` escolhe, `Esc`/`Tab` fecham e o
    /// foco volta ao campo. Sem ela, nada muda.
    #[prop(optional)]
    use_filter: bool,
    /// `aria-label` da caixa de busca (padrão "Filtrar opções").
    #[prop(optional, into)]
    filter_label: Option<String>,
    /// Texto quando a busca não acha nada (padrão "Nenhum resultado"; `aria-live="polite"`).
    #[prop(optional, into)]
    no_results_label: Option<String>,
    /// Foca o campo ao montar e o marca com `data-autofocus` (o `autofocus` do `Form`).
    #[prop(optional)]
    autofocus: bool,
    /// Nome acessível do campo e da lista, se não houver `label`.
    #[prop(optional, into)]
    aria_label: Option<String>,
    // — Field —
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] stack_label: bool,
    #[prop(optional, into)] hint: Option<String>,
    #[prop(optional)] hide_hint: bool,
    #[prop(optional, into)] prefix: Option<String>,
    #[prop(optional, into)] suffix: Option<String>,
    #[prop(optional)] design: FieldDesign,
    #[prop(optional)] dense: bool,
    #[prop(optional)] color: Color,
    #[prop(optional, into)] disable: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional)] bottom_slots: bool,
    #[prop(optional)] hide_bottom_space: bool,
    /// Spinner no fim da caixa.
    #[prop(optional, into)]
    loading: Signal<bool>,
    // — Validação —
    /// Regras (`use_validate`), sobre o `Option<T>` do modelo.
    #[prop(optional)]
    rules: Vec<Rule<Option<T>>>,
    #[prop(optional)] lazy_rules: LazyRules,
    #[prop(optional, into)] error: Signal<Option<bool>>,
    #[prop(optional, into)] error_message: Signal<String>,
    #[prop(optional)] no_error_icon: bool,
    // — Eventos —
    /// Depois da transição de entrada do popup.
    #[prop(optional, into)]
    on_popup_show: Option<Callback<()>>,
    /// Depois da transição de saída do popup.
    #[prop(optional, into)]
    on_popup_hide: Option<Callback<()>>,
    /// O botão `clearable` foi usado; recebe o valor que havia.
    #[prop(optional, into)]
    on_clear: Option<Callback<Option<T>>>,
    // — Slots do Field —
    #[prop(optional)] prepend: Option<FieldPrepend>,
    #[prop(optional)] append: Option<FieldAppend>,
    #[prop(optional)] before: Option<FieldBefore>,
    #[prop(optional)] after: Option<FieldAfter>,
    #[prop(optional)] hint_slot: Option<FieldHint>,
    #[prop(optional)] error_slot: Option<FieldError>,
    #[prop(optional)] loading_slot: Option<FieldLoading>,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
{
    let validation = use_validate_with(
        value.into(),
        ValidationOptions {
            rules,
            lazy_rules,
            disable,
            error,
            error_message,
            name,
        },
        {
            let options = options.clone();
            Arc::new(move |a: &Option<T>, b: &Option<T>| match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => options.same(a, b),
                _ => false,
            })
        },
    );
    let model = Model {
        selected: Signal::derive(move || value.get().into_iter().collect()),
        multiple: false,
        set: Callback::new(move |v: Vec<T>| value.set(v.into_iter().next())),
    };
    let on_clear = on_clear
        .map(|cb| Callback::new(move |previous: Vec<T>| cb.run(previous.into_iter().next())));
    select_field(Config {
        model,
        options,
        id: id.unwrap_or_else(generated_id),
        placeholder,
        display_value,
        aria_label,
        clearable,
        hide_dropdown_icon,
        no_options_label: no_options_label.unwrap_or_else(|| "Sem opções".to_string()),
        use_filter,
        filter_label: filter_label.unwrap_or_else(|| "Filtrar opções".to_string()),
        no_results_label: no_results_label.unwrap_or_else(|| "Nenhum resultado".to_string()),
        autofocus,
        validation,
        on_popup_show,
        on_popup_hide,
        on_clear,
        label,
        stack_label,
        hint,
        hide_hint,
        prefix,
        suffix,
        design,
        dense,
        color,
        disable,
        readonly,
        bottom_slots,
        hide_bottom_space,
        loading,
        no_error_icon,
        prepend,
        append,
        before,
        after,
        hint_slot,
        error_slot,
        loading_slot,
    })
}

/// `Select` de várias opções. ≈ `QSelect` com `multiple`.
///
/// Como o [`Select`], mas `value` é um `Vec<T>` (na ordem em que as opções foram escolhidas) e a
/// lista **não fecha ao escolher**: cada escolha alterna a opção (`aria-multiselectable`). O campo
/// mostra os labels escolhidos separados por vírgula, na ordem das opções. As `rules` recebem
/// `&Vec<T>`; `clearable` limpa para `[]`.
///
/// Ainda não: `use_chips`, `max_values`, remover a última com `Backspace`.
#[component]
pub fn SelectMultiple<T>(
    /// Os valores escolhidos (`v-model`).
    value: RwSignal<Vec<T>>,
    /// As opções.
    #[prop(into)]
    options: Options<T>,
    /// `id` do campo (o `for` do label). Sem ele, gera `lx-select-<n>`.
    #[prop(optional, into)]
    id: Option<String>,
    /// Nome do campo no `Form`, para diagnóstico.
    #[prop(optional, into)]
    name: Option<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    /// Sobrepõe o texto do campo (o padrão são os labels escolhidos).
    #[prop(optional, into)]
    display_value: MaybeProp<String>,
    /// Botão que limpa a seleção.
    #[prop(optional)]
    clearable: bool,
    /// Esconde a seta do campo.
    #[prop(optional)]
    hide_dropdown_icon: bool,
    /// Texto da lista sem opções.
    #[prop(optional, into)]
    no_options_label: Option<String>,
    /// Caixa de busca no topo do popup (busca AICI pelo rótulo: sem diferenciar maiúsculas nem
    /// acentos, "contém"). O foco entra na caixa ao abrir; `Enter` escolhe, `Esc`/`Tab` fecham e o
    /// foco volta ao campo. Sem ela, nada muda.
    #[prop(optional)]
    use_filter: bool,
    /// `aria-label` da caixa de busca (padrão "Filtrar opções").
    #[prop(optional, into)]
    filter_label: Option<String>,
    /// Texto quando a busca não acha nada (padrão "Nenhum resultado"; `aria-live="polite"`).
    #[prop(optional, into)]
    no_results_label: Option<String>,
    /// Foca o campo ao montar e o marca com `data-autofocus`.
    #[prop(optional)]
    autofocus: bool,
    /// Nome acessível do campo e da lista, se não houver `label`.
    #[prop(optional, into)]
    aria_label: Option<String>,
    // — Field —
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] stack_label: bool,
    #[prop(optional, into)] hint: Option<String>,
    #[prop(optional)] hide_hint: bool,
    #[prop(optional, into)] prefix: Option<String>,
    #[prop(optional, into)] suffix: Option<String>,
    #[prop(optional)] design: FieldDesign,
    #[prop(optional)] dense: bool,
    #[prop(optional)] color: Color,
    #[prop(optional, into)] disable: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional)] bottom_slots: bool,
    #[prop(optional)] hide_bottom_space: bool,
    /// Spinner no fim da caixa.
    #[prop(optional, into)]
    loading: Signal<bool>,
    // — Validação —
    /// Regras (`use_validate`), sobre o `Vec<T>` do modelo.
    #[prop(optional)]
    rules: Vec<Rule<Vec<T>>>,
    #[prop(optional)] lazy_rules: LazyRules,
    #[prop(optional, into)] error: Signal<Option<bool>>,
    #[prop(optional, into)] error_message: Signal<String>,
    #[prop(optional)] no_error_icon: bool,
    // — Eventos —
    /// Depois da transição de entrada do popup.
    #[prop(optional, into)]
    on_popup_show: Option<Callback<()>>,
    /// Depois da transição de saída do popup.
    #[prop(optional, into)]
    on_popup_hide: Option<Callback<()>>,
    /// O botão `clearable` foi usado; recebe a seleção que havia.
    #[prop(optional, into)]
    on_clear: Option<Callback<Vec<T>>>,
    // — Slots do Field —
    #[prop(optional)] prepend: Option<FieldPrepend>,
    #[prop(optional)] append: Option<FieldAppend>,
    #[prop(optional)] before: Option<FieldBefore>,
    #[prop(optional)] after: Option<FieldAfter>,
    #[prop(optional)] hint_slot: Option<FieldHint>,
    #[prop(optional)] error_slot: Option<FieldError>,
    #[prop(optional)] loading_slot: Option<FieldLoading>,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
{
    let validation = use_validate_with(
        value.into(),
        ValidationOptions {
            rules,
            lazy_rules,
            disable,
            error,
            error_message,
            name,
        },
        {
            let options = options.clone();
            Arc::new(move |a: &Vec<T>, b: &Vec<T>| {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| options.same(a, b))
            })
        },
    );
    let model = Model {
        selected: value.into(),
        multiple: true,
        set: Callback::new(move |v: Vec<T>| value.set(v)),
    };
    select_field(Config {
        model,
        options,
        id: id.unwrap_or_else(generated_id),
        placeholder,
        display_value,
        aria_label,
        clearable,
        hide_dropdown_icon,
        no_options_label: no_options_label.unwrap_or_else(|| "Sem opções".to_string()),
        use_filter,
        filter_label: filter_label.unwrap_or_else(|| "Filtrar opções".to_string()),
        no_results_label: no_results_label.unwrap_or_else(|| "Nenhum resultado".to_string()),
        autofocus,
        validation,
        on_popup_show,
        on_popup_hide,
        on_clear,
        label,
        stack_label,
        hint,
        hide_hint,
        prefix,
        suffix,
        design,
        dense,
        color,
        disable,
        readonly,
        bottom_slots,
        hide_bottom_space,
        loading,
        no_error_icon,
        prepend,
        append,
        before,
        after,
        hint_slot,
        error_slot,
        loading_slot,
    })
}
