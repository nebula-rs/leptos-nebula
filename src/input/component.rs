//! `<Input>` e `InputHandle` (≈ `QInput` do Quasar): um `<input>`/`<textarea>` dentro de um
//! [`Field`].

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use leptos::{ev, html, prelude::*};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement, HtmlTextAreaElement};

use super::InputType;
use crate::field::FieldProps;
use crate::{
    Color, Field, FieldAfter, FieldAppend, FieldBefore, FieldDesign, FieldError, FieldHint,
    FieldPrepend, Icon, LazyRules, Rule, ValidationOptions, use_field_context, use_validate,
};

/// Os dois possíveis controles nativos do `Input` (só um deles é montado).
#[derive(Clone, Copy)]
struct Native {
    input: NodeRef<html::Input>,
    textarea: NodeRef<html::Textarea>,
}

impl Native {
    fn element(&self) -> Option<HtmlElement> {
        match self.input.get_untracked() {
            Some(el) => Some(el.unchecked_into::<HtmlElement>()),
            None => self
                .textarea
                .get_untracked()
                .map(|el| el.unchecked_into::<HtmlElement>()),
        }
    }

    fn focus(&self) {
        if let Some(el) = self.element() {
            let _ = el.focus();
        }
    }

    fn blur(&self) {
        if let Some(el) = self.element() {
            let _ = el.blur();
        }
    }

    fn select(&self) {
        if let Some(el) = self.input.get_untracked() {
            HtmlInputElement::select(&el);
        } else if let Some(el) = self.textarea.get_untracked() {
            HtmlTextAreaElement::select(&el);
        }
    }
}

/// Controle de um [`Input`] por código (≈ `focus()`, `blur()` e `select()` do `QInput`).
///
/// `Copy`. Crie com [`InputHandle::new`] e passe em `handle=`; o `<Input>` o conecta ao
/// controle nativo ao ser criado. Antes disso (ou depois de o `<Input>` ser descartado) os
/// métodos não fazem nada.
#[derive(Clone, Copy)]
pub struct InputHandle(StoredValue<Option<Native>, LocalStorage>);

impl Default for InputHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandle {
    /// Handle ainda sem `<Input>`.
    pub fn new() -> Self {
        Self(StoredValue::new_local(None))
    }

    fn native(&self) -> Option<Native> {
        self.0.try_get_value().flatten()
    }

    /// Foca o controle nativo.
    pub fn focus(&self) {
        if let Some(n) = self.native() {
            n.focus();
        }
    }

    /// Tira o foco do controle nativo.
    pub fn blur(&self) {
        if let Some(n) = self.native() {
            n.blur();
        }
    }

    /// Seleciona todo o texto do controle.
    pub fn select(&self) {
        if let Some(n) = self.native() {
            n.select();
        }
    }
}

/// Contador só para o navegador sem SSR (`csr`), onde não há `SharedContext`.
static CSR_IDS: AtomicUsize = AtomicUsize::new(0);

/// `id` gerado para o controle quando o app não passa `id`.
///
/// Seguro para hidratação: com SSR/SSG o número vem do `SharedContext` do Leptos
/// (`next_id`), uma sequência por requisição/página que o servidor e o cliente percorrem na
/// mesma ordem (a mesma que o Leptos usa para os `Resource`); um contador global não serviria,
/// pois o SSG renderiza várias páginas no mesmo processo. Sem `SharedContext` (`csr` puro) usa
/// um contador atômico, que não precisa casar com nada.
fn generated_id() -> String {
    let n = match Owner::current_shared_context() {
        Some(ctx) => ctx.next_id().into_inner(),
        None => CSR_IDS.fetch_add(1, Ordering::Relaxed),
    };
    format!("lx-input-{n}")
}

/// Campo de texto: um [`Field`] com um `<input>` (ou `<textarea>`) dentro. ≈ `QInput`.
///
/// `value` é o `v-model` (um `RwSignal<String>`, como em `Tabs`). Todas as props visuais do
/// `Field` (`label`, `hint`, `design`…) e de validação (`rules`, `lazy_rules`, `error`,
/// `error_message`, `no_error_icon`) são repassadas; o `Input` chama `use_validate` e registra-se
/// no `Form` mais próximo.
///
/// **Id do controle:** o `<label for>` precisa do `id` do controle. Passe `id` para escolher
/// um; sem `id`, o `Input` gera `lx-input-<n>`, estável entre servidor e cliente (ver
/// `docs/ui/forms.md#input`). O `name` é só o atributo `name` do controle.
///
/// `Date`, `Time` e `DatetimeLocal` mantêm o label sempre flutuando (o navegador sempre mostra o
/// controle). Com `clearable`, um botão no fim limpa o valor, devolve o foco ao controle, chama
/// `on_clear` (com o valor anterior) e limpa a validação.
///
/// Ainda não: `mask` (R-17), `debounce`, `autogrow`, `counter`, `loading`, `shadow_text` (R-18),
/// `type="file"`, `input_class`/`input_style`, `model_modifiers`, `display_value`, `on_paste`,
/// `bg_color`, `rounded`, `square`, `dark`, `reactive_rules`, os slots `label`/`counter`/
/// `loading`/`control`. `value` é sempre `String` (sem conversão numérica).
#[component]
pub fn Input(
    /// O valor (`v-model`).
    value: RwSignal<String>,
    /// Tipo do controle; padrão `Text`.
    #[prop(optional)]
    type_: InputType,
    /// `id` do controle (o `for` do label). Sem ele, o `Input` gera um.
    #[prop(optional, into)]
    id: Option<String>,
    /// Atributo `name` do controle (e nome do campo no `Form`, para diagnóstico).
    #[prop(optional, into)]
    name: Option<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    #[prop(optional)] maxlength: Option<u32>,
    /// Marca o controle com `data-autofocus` (o `autofocus` do `Form` o encontra) e o foca ao
    /// montar.
    #[prop(optional)]
    autofocus: bool,
    /// Botão que limpa o valor (aparece com valor e fora de `disable`/`readonly`).
    #[prop(optional)]
    clearable: bool,
    /// Ícone do botão de limpar: a classe **literal** `i-lucide-<nome>` (como em [`Icon`]).
    #[prop(optional)]
    clear_icon: Option<&'static str>,
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
    // — Validação —
    /// Regras (`use_validate`).
    #[prop(optional)]
    rules: Vec<Rule<String>>,
    #[prop(optional)] lazy_rules: LazyRules,
    #[prop(optional, into)] error: Signal<Option<bool>>,
    #[prop(optional, into)] error_message: Signal<String>,
    #[prop(optional)] no_error_icon: bool,
    // — Eventos —
    /// O controle recebeu o foco.
    #[prop(optional, into)]
    on_focus: Option<Callback<ev::FocusEvent>>,
    /// O controle perdeu o foco.
    #[prop(optional, into)]
    on_blur: Option<Callback<ev::FocusEvent>>,
    /// Evento `change` nativo (ao confirmar/sair), com o valor.
    #[prop(optional, into)]
    on_change: Option<Callback<String>>,
    #[prop(optional, into)] on_keydown: Option<Callback<ev::KeyboardEvent>>,
    /// O botão `clearable` foi usado; recebe o valor que havia.
    #[prop(optional, into)]
    on_clear: Option<Callback<String>>,
    /// Controle por código (`focus`/`blur`/`select`).
    #[prop(optional)]
    handle: Option<InputHandle>,
    // — Slots do Field —
    #[prop(optional)] prepend: Option<FieldPrepend>,
    #[prop(optional)] append: Option<FieldAppend>,
    #[prop(optional)] before: Option<FieldBefore>,
    #[prop(optional)] after: Option<FieldAfter>,
    #[prop(optional)] hint_slot: Option<FieldHint>,
    #[prop(optional)] error_slot: Option<FieldError>,
) -> impl IntoView {
    let id = id.unwrap_or_else(generated_id);
    let native = Native {
        input: NodeRef::new(),
        textarea: NodeRef::new(),
    };
    if let Some(handle) = handle {
        handle.0.set_value(Some(native));
        on_cleanup(move || {
            handle.0.try_set_value(None);
        });
    }

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

    let has_value = Signal::derive(move || !value.with(String::is_empty));

    // Só no navegador (Effects não rodam no servidor): foco inicial.
    Effect::new(move |_| {
        let mounted = native.input.get().is_some() || native.textarea.get().is_some();
        if autofocus && mounted {
            native.focus();
        }
    });

    let show_clear =
        Signal::derive(move || clearable && has_value.get() && !disable.get() && !readonly.get());
    let clear = move || {
        let previous = value.get_untracked();
        native.focus();
        value.set(String::new());
        if let Some(cb) = on_clear {
            cb.run(previous);
        }
        // Como o Quasar: depois do próximo ciclo, quando a validação disparada pela mudança
        // do valor já rodou, limpa o erro.
        set_timeout(move || validation.reset_validation(), Duration::ZERO);
    };
    let clear_icon = clear_icon.unwrap_or("i-lucide-x");
    let append = (clearable || append.is_some()).then(|| FieldAppend {
        children: {
            let user = append;
            Arc::new(move || {
                view! {
                    {move || {
                        show_clear
                            .get()
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        class="lx-input-clear"
                                        aria-label="Limpar"
                                        on:click=move |ev: ev::MouseEvent| {
                                            ev.prevent_default();
                                            clear();
                                        }
                                    >
                                        <Icon name=clear_icon />
                                    </button>
                                }
                            })
                    }}
                    {user.as_ref().map(|slot| (slot.children)())}
                }
                .into_any()
            })
        },
    });

    let control = NativeControlProps {
        id: id.clone(),
        name,
        type_,
        placeholder,
        maxlength,
        autofocus,
        value,
        disable,
        readonly,
        native,
        on_focus,
        on_blur,
        on_change,
        on_keydown,
    };

    // O `view!` não repassa um `Option` de slot; monta-se as props do `Field` e atribui-se
    // os campos (todos `pub`).
    let mut props = FieldProps::builder()
        .children(ToChildren::to_children(move || NativeControl(control)))
        .build();
    props.label = label;
    props.stack_label = stack_label || type_.always_floats();
    props.hint = hint;
    props.hide_hint = hide_hint;
    props.prefix = prefix;
    props.suffix = suffix;
    props.design = design;
    props.dense = dense;
    props.color = color;
    props.disable = disable;
    props.readonly = readonly;
    props.for_ = Some(id);
    props.has_value = has_value;
    props.bottom_slots = bottom_slots;
    props.hide_bottom_space = hide_bottom_space;
    props.validation = Some(validation);
    props.no_error_icon = no_error_icon;
    props.prepend = prepend;
    props.append = append;
    props.before = before;
    props.after = after;
    props.hint_slot = hint_slot;
    props.error_slot = error_slot;
    Field(props)
}

/// O `<input>`/`<textarea>` nativo; lê o `FieldContext` do `Field` ao redor.
#[component]
fn NativeControl(
    id: String,
    name: Option<String>,
    type_: InputType,
    placeholder: Option<String>,
    maxlength: Option<u32>,
    autofocus: bool,
    value: RwSignal<String>,
    disable: Signal<bool>,
    readonly: Signal<bool>,
    native: Native,
    on_focus: Option<Callback<ev::FocusEvent>>,
    on_blur: Option<Callback<ev::FocusEvent>>,
    on_change: Option<Callback<String>>,
    on_keydown: Option<Callback<ev::KeyboardEvent>>,
) -> impl IntoView {
    let ctx = use_field_context().expect("o controle do Input vai dentro de um <Field>");
    let class = if type_.is_textarea() {
        format!("{} lx-input-textarea", ctx.control_class)
    } else {
        ctx.control_class.to_string()
    };
    let has_error = ctx.has_error;
    let described_by = {
        let (error_id, hint_id) = (ctx.error_id.clone(), ctx.hint_id.clone());
        move || {
            if has_error.get() {
                error_id.clone()
            } else {
                hint_id.clone()
            }
        }
    };
    let aria_invalid = move || has_error.get().then_some("true");
    let data_autofocus = autofocus.then_some("");
    let tabindex = autofocus.then_some("0");
    // Só escreve se mudou: um `type="number"` com texto incompleto (`1e`) tem `value` vazio, e
    // regravar o mesmo valor apagaria o que a pessoa digitou.
    let set_value = move |new: String| {
        if value.with_untracked(|v| *v != new) {
            value.set(new);
        }
    };

    if type_.is_textarea() {
        view! {
            <textarea
                node_ref=native.textarea
                id=id
                name=name
                class=class
                rows="6"
                placeholder=placeholder
                maxlength=maxlength
                tabindex=tabindex
                data-autofocus=data_autofocus
                disabled=move || disable.get()
                readonly=move || readonly.get()
                aria-invalid=aria_invalid
                aria-describedby=described_by
                prop:value=move || value.get()
                on:input=move |ev| set_value(event_target_value(&ev))
                on:change=move |ev| {
                    if let Some(cb) = on_change {
                        cb.run(event_target_value(&ev));
                    }
                }
                on:focus=move |ev| {
                    if let Some(cb) = on_focus {
                        cb.run(ev);
                    }
                }
                on:blur=move |ev| {
                    if let Some(cb) = on_blur {
                        cb.run(ev);
                    }
                }
                on:keydown=move |ev| {
                    if let Some(cb) = on_keydown {
                        cb.run(ev);
                    }
                }
            >
                {value.get_untracked()}
            </textarea>
        }
        .into_any()
    } else {
        view! {
            <input
                node_ref=native.input
                id=id
                name=name
                type=type_.attr()
                class=class
                placeholder=placeholder
                maxlength=maxlength
                tabindex=tabindex
                data-autofocus=data_autofocus
                disabled=move || disable.get()
                readonly=move || readonly.get()
                aria-invalid=aria_invalid
                aria-describedby=described_by
                value=value.get_untracked()
                prop:value=move || value.get()
                on:input=move |ev| set_value(event_target_value(&ev))
                on:change=move |ev| {
                    if let Some(cb) = on_change {
                        cb.run(event_target_value(&ev));
                    }
                }
                on:focus=move |ev| {
                    if let Some(cb) = on_focus {
                        cb.run(ev);
                    }
                }
                on:blur=move |ev| {
                    if let Some(cb) = on_blur {
                        cb.run(ev);
                    }
                }
                on:keydown=move |ev| {
                    if let Some(cb) = on_keydown {
                        cb.run(ev);
                    }
                }
            />
        }
        .into_any()
    }
}
