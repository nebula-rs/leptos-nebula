//! O controle do `Autocomplete`: um `<input type="text" role="combobox" aria-autocomplete="list">`
//! dentro do [`Field`] e a lista de sugestões (`role="listbox"`) num popup ancorado à caixa.
//!
//! Reaproveita o que o `Select` já verificou: o popup (`components::popup`: `Overlay`, Popover
//! API, `Portal`, pilha de `Esc`), a navegação (`select::nav`), as classes `lx-menu*` e
//! `lx-select-option-*`, o botão `lx-input-clear` e o `Emitter` do `debounce` do `Input`. O foco
//! fica **sempre** no campo (a sugestão ativa é apontada por `aria-activedescendant`).
//!
//! Tudo que toca o DOM roda só em handlers e `Effect`s, nunca durante a renderização (SSR).

use std::sync::Arc;
use std::time::Duration;

use leptos::leptos_dom::helpers::set_timeout;
use leptos::{ev, html, portal::Portal, prelude::*};

use super::logic::{KeyAction, key_action, step, visible};
use crate::field::FieldProps;
use crate::input::emit::Emitter;
use crate::list_popup::{Hooks, ListPopup};
use crate::search::normalize;
use crate::util::next_id;
use crate::{
    Color, Field, FieldAfter, FieldAppend, FieldBefore, FieldDesign, FieldError, FieldHint,
    FieldLoading, FieldPrepend, FieldValidation, Icon,
};

/// `id` do controle quando o app não passa um (estável entre servidor e cliente).
pub(super) fn generated_id() -> String {
    next_id("lx-autocomplete")
}

/// Classe da sugestão: ativa (a do `aria-activedescendant`) × as demais, em shortcuts completos.
fn option_class(active: bool) -> &'static str {
    if active {
        "lx-select-option-active"
    } else {
        "lx-select-option-idle"
    }
}

/// Tudo que o `Autocomplete` repassa: o controle e as props do `Field`.
pub(super) struct Config {
    pub value: RwSignal<String>,
    pub options: Signal<Vec<String>>,
    pub filter: bool,
    pub debounce: Option<u32>,
    pub id: String,
    pub placeholder: Option<String>,
    pub aria_label: Option<String>,
    pub clearable: bool,
    pub autofocus: bool,
    pub validation: FieldValidation,
    pub on_clear: Option<Callback<String>>,
    pub label: Option<String>,
    pub stack_label: bool,
    pub hint: Option<String>,
    pub hide_hint: bool,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub design: FieldDesign,
    pub dense: bool,
    pub color: Color,
    pub disable: Signal<bool>,
    pub readonly: Signal<bool>,
    pub bottom_slots: bool,
    pub hide_bottom_space: bool,
    pub loading: Signal<bool>,
    pub no_error_icon: bool,
    pub name: Option<String>,
    pub prepend: Option<FieldPrepend>,
    pub append: Option<FieldAppend>,
    pub before: Option<FieldBefore>,
    pub after: Option<FieldAfter>,
    pub hint_slot: Option<FieldHint>,
    pub error_slot: Option<FieldError>,
    pub loading_slot: Option<FieldLoading>,
}

/// Monta o `Field` com o controle dentro.
pub(super) fn autocomplete_field(cfg: Config) -> impl IntoView {
    let Config {
        value,
        options,
        filter,
        debounce,
        id,
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
    } = cfg;

    // O texto exibido acompanha cada tecla; `value` só é escrito depois do `debounce` (como o
    // `Input`). O filtro usa o texto exibido, não `value`: a lista reage na hora.
    let inner = RwSignal::new(value.get_untracked());
    let emitter = Emitter::new(value, debounce);
    // O descarte do campo entrega o que estava adiado.
    on_cleanup(move || emitter.flush());
    // O modelo mudou por fora (ou pelo eco do próprio campo): o texto exibido o acompanha.
    Effect::new(move |prev: Option<()>| {
        let v = value.get();
        if prev.is_some() && inner.get_untracked() != v {
            emitter.cancel();
            inner.set(v);
        }
    });

    // Os textos normalizados uma vez por mudança da lista; a agulha, uma vez por mudança do texto.
    let normalized = Memo::new(move |_| {
        if filter {
            options.with(|o| o.iter().map(|s| normalize(s)).collect::<Vec<_>>())
        } else {
            Vec::new()
        }
    });
    let visible = Memo::new(move |_| {
        let text = if filter { inner.get() } else { String::new() };
        options.with(|o| normalized.with(|n| visible(o, n, &text, filter)))
    });

    let input = NodeRef::<html::Input>::new();
    let open = RwSignal::new(false);
    let active = RwSignal::new(None::<usize>);
    // A lista só aparece aberta e com sugestões.
    let shown = Memo::new(move |_| open.get() && visible.with(|v| !v.is_empty()));
    let ctl = Ctl {
        p: ListPopup::new(
            "Autocomplete",
            &id,
            open,
            shown.into(),
            active,
            Hooks::default(),
        ),
        open,
        active,
        shown,
        input,
        inner,
        visible,
        emitter,
        value,
        disable,
        readonly,
    };

    Effect::new(move |_| {
        if ctl.p.o.marker.get().is_some() {
            // a âncora é a caixa do campo; clicar nela não alterna a lista (posiciona o cursor)
            ctl.p.mount();
        }
    });
    Effect::new(move |_| ctl.p.sync());
    // Campo desabilitado ou somente leitura não fica aberto.
    Effect::new(move |_| {
        if (disable.get() || readonly.get()) && open.get_untracked() {
            open.set(false);
        }
    });
    // A lista mudou: os índices apontam para outros textos, então a ativa se perde.
    Effect::new(move |prev: Option<()>| {
        visible.track();
        if prev.is_some() {
            active.set(None);
        }
    });
    // A sugestão ativa anda: mantém-na à vista dentro da lista.
    Effect::new(move |_| {
        let a = active.get();
        if shown.get_untracked() {
            ctl.p.scroll_to(a);
        }
    });
    on_cleanup(move || ctl.p.cleanup());

    // Botão de limpar (como o `clearable` do `Input`; o spinner toma o lugar dele).
    let has_value = Signal::derive(move || !inner.with(String::is_empty));
    let busy = Signal::derive(move || loading.get() || validation.loading().get());
    let show_clear = Signal::derive(move || {
        clearable && has_value.get() && !disable.get() && !readonly.get() && !busy.get()
    });
    let clear = move || {
        let previous = value.get_untracked();
        if let Some(el) = input.get_untracked() {
            let _ = el.focus();
        }
        // Limpar é imediato: descarta o que o `debounce` segurava.
        emitter.cancel();
        open.set(false);
        inner.set(String::new());
        value.set(String::new());
        if let Some(cb) = on_clear {
            cb.run(previous);
        }
        // Depois do próximo ciclo, quando a validação disparada pela mudança já rodou, limpa o erro.
        set_timeout(move || validation.reset_validation(), Duration::ZERO);
    };
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
                                        <Icon name="i-lucide-x" />
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

    let shared = Shared {
        ctl,
        id: id.clone(),
        name,
        placeholder,
        list_label: aria_label.clone().or_else(|| label.clone()),
        aria_label,
        autofocus,
    };
    // O `view!` não repassa um `Option` de slot: monta-se as props do `Field` e atribui-se os
    // campos (todos `pub`).
    let mut props = FieldProps::builder()
        .children(ToChildren::to_children(move || control_view(shared)))
        .build();
    props.label = label;
    props.stack_label = stack_label;
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
    props.loading = loading;
    props.loading_slot = loading_slot;
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

// ── Ligação ao DOM ─────────────────────────────────────────────────────────

/// Estado e ações do `Autocomplete`. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
struct Ctl {
    /// A máquina do popup (âncora, transição, `Esc`, clique fora, rolagem).
    p: ListPopup,
    /// O usuário quer a lista (digitou, ou usou `↓`/`↑`); só aparece com sugestões (`shown`).
    open: RwSignal<bool>,
    active: RwSignal<Option<usize>>,
    shown: Memo<bool>,
    input: NodeRef<html::Input>,
    inner: RwSignal<String>,
    visible: Memo<Vec<String>>,
    emitter: Emitter,
    value: RwSignal<String>,
    disable: Signal<bool>,
    readonly: Signal<bool>,
}

/// O que o controle precisa além do `Ctl`.
struct Shared {
    ctl: Ctl,
    id: String,
    name: Option<String>,
    placeholder: Option<String>,
    aria_label: Option<String>,
    /// Nome acessível da lista: `aria_label`, ou o `label` do campo.
    list_label: Option<String>,
    autofocus: bool,
}

impl Ctl {
    fn editable(&self) -> bool {
        !self.disable.get_untracked() && !self.readonly.get_untracked()
    }

    /// Preenche o texto com a sugestão `i` e fecha a lista; o foco continua no campo.
    fn pick(&self, i: usize) {
        let Some(text) = self.visible.with_untracked(|v| v.get(i).cloned()) else {
            return;
        };
        // a escolha é imediata: descarta o que o `debounce` segurava
        self.emitter.cancel();
        self.inner.set(text.clone());
        self.value.set(text);
        self.open.set(false);
        self.active.set(None);
    }

    fn on_input(&self, text: String) {
        self.inner.set(text.clone());
        self.emitter.emit(text, false);
        // digitar recomeça a escolha; a lista abre se houver sugestão
        self.active.set(None);
        if self.editable() {
            self.open.set(true);
        }
    }

    fn on_keydown(&self, e: &ev::KeyboardEvent) {
        let key = e.key();
        // Enter envia o formulário: o que o `debounce` segurava precisa estar no `value` antes.
        if key == "Enter" {
            self.emitter.flush();
        }
        // `Enter` que confirma uma composição (IME) não escolhe nada
        if e.is_composing() || !self.editable() || e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        let len = self.visible.with_untracked(Vec::len);
        let action = key_action(
            &key,
            self.shown.get_untracked(),
            self.active.get_untracked(),
            len,
        );
        match action {
            KeyAction::Nothing => {}
            KeyAction::Open(nav) => {
                e.prevent_default();
                self.active.set(step(len, None, nav));
                self.open.set(true);
            }
            KeyAction::Move(nav) => {
                e.prevent_default();
                self.active.set(step(len, self.active.get_untracked(), nav));
            }
            KeyAction::Pick(i) => {
                e.prevent_default();
                self.pick(i);
            }
            // sem `preventDefault`: o `Enter` ainda envia o `Form`, o `Tab` ainda passa o foco
            KeyAction::Close => self.open.set(false),
        }
    }
}

/// O `<input role="combobox">`, o marcador da âncora e o popup (`Portal`).
fn control_view(sh: Shared) -> impl IntoView {
    let Shared {
        ctl,
        id,
        name,
        placeholder,
        aria_label,
        list_label,
        autofocus,
    } = sh;
    let ctx =
        crate::use_field_context().expect("o controle do Autocomplete vai dentro de um <Field>");
    let class = ctx.control_class.to_string();
    let (open, active, shown, disable, readonly) =
        (ctl.open, ctl.active, ctl.shown, ctl.disable, ctl.readonly);
    let (inner, visible, emitter) = (ctl.inner, ctl.visible, ctl.emitter);
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
    let list_id = format!("{id}_list");
    let active_id = {
        let id = id.clone();
        move || {
            if !shown.get() {
                return None;
            }
            let i = active.get()?;
            visible
                .with(|v| i < v.len())
                .then(|| format!("{id}_opt_{i}"))
        }
    };
    let data_autofocus = autofocus.then_some("");
    let mount_focus = ctl.input;
    if autofocus {
        Effect::new(move |_| {
            if let Some(el) = mount_focus.get() {
                let _ = el.focus();
            }
        });
    }

    let popup = {
        let (id, list_id, list_label) = (id.clone(), list_id.clone(), list_label);
        move || {
            let class = ctl.p.popup_class();
            let items = {
                let id = id.clone();
                move || {
                    visible
                        .get()
                        .into_iter()
                        .enumerate()
                        .map(|(i, text)| {
                            view! {
                                <li
                                    id=format!("{id}_opt_{i}")
                                    role="option"
                                    class=move || option_class(active.get() == Some(i))
                                    aria-selected=move || (active.get() == Some(i)).to_string()
                                    on:click=move |_| ctl.pick(i)
                                    on:mousemove=move |_| ctl.p.hover(i)
                                >
                                    <span class="lx-select-option-label">{text}</span>
                                </li>
                            }
                        })
                        .collect_view()
                }
            };
            // `mousedown` com `preventDefault`: clicar na lista não tira o foco do campo (o
            // `aria-activedescendant` vive nele)
            view! {
                <div
                    node_ref=ctl.p.o.el
                    popover="manual"
                    class=class
                    on:mousedown=move |e: ev::MouseEvent| e.prevent_default()
                >
                    <ul
                        id=list_id.clone()
                        role="listbox"
                        class="lx-select-list"
                        aria-label=list_label.clone()
                    >
                        {items}
                    </ul>
                </div>
            }
        }
    };

    view! {
        <input
            node_ref=ctl.input
            id=id
            name=name
            type="text"
            class=class
            role="combobox"
            autocomplete="off"
            placeholder=placeholder
            data-autofocus=data_autofocus
            disabled=move || disable.get()
            readonly=move || readonly.get()
            aria-autocomplete="list"
            aria-haspopup="listbox"
            aria-expanded=move || shown.get().to_string()
            aria-controls=list_id
            aria-activedescendant=active_id
            aria-label=aria_label.clone()
            aria-invalid=move || has_error.get().then_some("true")
            aria-describedby=described_by
            value=inner.get_untracked()
            prop:value=move || inner.get()
            on:input=move |e| ctl.on_input(event_target_value(&e))
            on:keydown=move |e| ctl.on_keydown(&e)
            // o popup nunca recebe o foco: sair do campo fecha a lista (e entrega o texto adiado
            // antes de o `Field` validar no blur)
            on:focusout=move |_| emitter.flush()
            on:blur=move |_| {
                emitter.flush();
                open.set(false);
            }
        />
        <span hidden=true node_ref=ctl.p.o.marker></span>
        {match crate::root::portal_mount() {
            Some(m) => view! { <Portal mount=m>{popup()}</Portal> }.into_any(),
            None => view! { <Portal>{popup()}</Portal> }.into_any(),
        }}
    }
}
