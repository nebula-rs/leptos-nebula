//! `<Autocomplete>`: o componente público; monta a validação e repassa ao controle (`control.rs`).

use leptos::prelude::*;

use super::control::{Config, autocomplete_field, generated_id};
use crate::{
    Color, FieldAfter, FieldAppend, FieldBefore, FieldDesign, FieldError, FieldHint, FieldLoading,
    FieldPrepend, LazyRules, Rule, ValidationOptions, use_validate,
};

/// Campo de **texto livre** com uma lista de **sugestões**. ≈ `QSelect` com `use-input` +
/// `fill-input` + `hide-selected` + `new-value-mode`, mas como componente próprio
/// (ADR-0012).
///
/// `value` (`RwSignal<String>`) é o texto: qualquer texto vale, com ou sem sugestão. `options`
/// (`Signal<Vec<String>>`) são as **sugestões**, sempre textos; para ter objetos como fonte, o app
/// deriva os textos (`Signal::derive`). Escolher uma sugestão **preenche o texto**: nenhum evento
/// entrega objeto.
///
/// **Filtro:** por padrão (`filter=true`) a lista mostra as sugestões que **contêm** o texto
/// digitado, sem diferenciar maiúsculas **nem acentos** ([`search`](crate::search): `"sao"` acha
/// `"São Paulo"`); texto vazio mostra todas. Com `filter=false` as `options` aparecem como vieram
/// (o app já filtrou, por exemplo numa server function a partir de `value`; combine com
/// `debounce` e `loading`). A lista só abre com sugestões a mostrar.
///
/// **Teclado** (o foco fica sempre no campo): digitar abre a lista; `↓`/`↑` abrem (com a lista
/// fechada) e andam pelas sugestões, sem dar a volta; `Enter` com uma sugestão ativa preenche o
/// texto e fecha (com `preventDefault`); `Enter` **sem** sugestão ativa segue o comportamento
/// nativo (envia o `Form`); `Esc` fecha a lista (com `preventDefault` só se havia lista aberta, de
/// modo que um `Dialog` em volta não fecha junto); `Tab` e sair do campo fecham sem preencher;
/// clicar numa sugestão preenche e mantém o foco no campo. `Home`/`End` movem o cursor do texto.
///
/// **Acessibilidade:** o campo é um `<input role="combobox" aria-autocomplete="list">` com
/// `aria-expanded`, `aria-controls` e `aria-activedescendant` (a sugestão ativa, só com a lista
/// aberta); a lista é um `role="listbox"` de `role="option"`.
///
/// O campo é um [`Field`](crate::Field): todas as props visuais e de validação (`rules` sobre
/// `&String`, `lazy_rules`, `error`, `error_message`, `no_error_icon`) valem, e dentro de um
/// [`Form`](crate::Form) o `Autocomplete` se registra sozinho. `debounce` (ms) e `clearable`
/// funcionam como no [`Input`](crate::Input): o texto exibido acompanha cada tecla (e o filtro
/// também), mas `value` só é escrito depois da pausa; blur, `Enter`, escolher uma sugestão e o
/// descarte do campo o entregam na hora.
///
/// Ainda não: máscara, `counter`, `maxlength`, `on_popup_show`/`on_popup_hide`, `no_options_label`
/// (sem sugestões a lista não abre), `new_value_mode`, destaque do trecho achado, slot `option`,
/// `InputHandle`.
#[component]
pub fn Autocomplete(
    /// O texto (`v-model`).
    value: RwSignal<String>,
    /// As sugestões, sempre textos.
    #[prop(into)]
    options: Signal<Vec<String>>,
    /// Filtra as `options` pelo texto digitado (busca AICI); com `false` elas saem como vieram.
    #[prop(default = true)]
    filter: bool,
    /// Pausa em ms antes de escrever em `value` (como no `Input`). O filtro não espera por ela.
    #[prop(optional)]
    debounce: Option<u32>,
    /// `id` do campo (o `for` do label). Sem ele, o `Autocomplete` gera `lx-autocomplete-<n>`.
    #[prop(optional, into)]
    id: Option<String>,
    /// Atributo `name` do `<input>` (e nome do campo no `Form`, para diagnóstico).
    #[prop(optional, into)]
    name: Option<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    /// Botão que limpa o texto (aparece com texto e fora de `disable`/`readonly`).
    #[prop(optional)]
    clearable: bool,
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
    /// Spinner no fim da caixa (no lugar do botão `clearable`): sugestões a caminho.
    #[prop(optional, into)]
    loading: Signal<bool>,
    // — Validação —
    /// Regras (`use_validate`), sobre o texto.
    #[prop(optional)]
    rules: Vec<Rule<String>>,
    #[prop(optional)] lazy_rules: LazyRules,
    #[prop(optional, into)] error: Signal<Option<bool>>,
    #[prop(optional, into)] error_message: Signal<String>,
    #[prop(optional)] no_error_icon: bool,
    // — Eventos —
    /// O botão `clearable` foi usado; recebe o texto que havia.
    #[prop(optional, into)]
    on_clear: Option<Callback<String>>,
    // — Slots do Field —
    #[prop(optional)] prepend: Option<FieldPrepend>,
    #[prop(optional)] append: Option<FieldAppend>,
    #[prop(optional)] before: Option<FieldBefore>,
    #[prop(optional)] after: Option<FieldAfter>,
    #[prop(optional)] hint_slot: Option<FieldHint>,
    #[prop(optional)] error_slot: Option<FieldError>,
    #[prop(optional)] loading_slot: Option<FieldLoading>,
) -> impl IntoView {
    let validation = use_validate(
        value.into(),
        ValidationOptions {
            rules,
            lazy_rules,
            disable,
            error,
            error_message,
            name: name.clone(),
        },
    );
    autocomplete_field(Config {
        value,
        options,
        filter,
        debounce,
        id: id.unwrap_or_else(generated_id),
        placeholder,
        aria_label,
        clearable,
        autofocus,
        validation,
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
        name,
        prepend,
        append,
        before,
        after,
        hint_slot,
        error_slot,
        loading_slot,
    })
}
