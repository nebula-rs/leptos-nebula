//! O controle do `Select`: um `<input readonly role="combobox">` dentro do [`Field`] e a lista
//! (`role="listbox"`) num popup ancorado à caixa do campo.
//!
//! Reaproveita a base dos popups do `Menu` (`components::popup`: Popover API, `Portal`, motor de
//! posição, pilha de `Esc`), mas não o `Menu`: o `Menu` põe o foco **dentro** do popup (papel
//! `menu`), e o `combobox` precisa do foco no campo, com a opção ativa apontada por
//! `aria-activedescendant`. Ver `docs/ui/forms.md#select`.
//!
//! Tudo que toca o DOM roda só em handlers e `Effect`s, nunca durante a renderização (SSR).

use std::sync::Arc;
use std::time::Duration;

use leptos::leptos_dom::helpers::set_timeout;
use leptos::{ev, html, portal::Portal, prelude::*};
use wasm_bindgen::JsCast;

use super::nav::{
    NavKey, TypeAhead, find_prefix, initial_active, join_labels, move_active, nav_key, toggle,
    typeahead_char,
};
use crate::components::popup::{
    Listener, ListenerOpts, Overlay, Params, Phase, document, esc_is_top, esc_push, esc_remove,
    focus_quietly,
};
use crate::field::FieldProps;
use crate::position::{Horizontal, Position, Vertical};
use crate::util::{cls, next_id};
use crate::{
    Color, Field, FieldAfter, FieldAppend, FieldBefore, FieldDesign, FieldError, FieldHint,
    FieldLoading, FieldPrepend, FieldValidation, Icon, Options,
};

const BOTTOM_START: Position = Position::new(Vertical::Bottom, Horizontal::Start);
const TOP_START: Position = Position::new(Vertical::Top, Horizontal::Start);

/// `id` do controle quando o app não passa um (estável entre servidor e cliente).
pub(super) fn generated_id() -> String {
    next_id("lx-select")
}

/// Classe da opção: ativa (a do `aria-activedescendant`) × selecionada × desabilitada, em
/// shortcuts completos e exclusivos.
fn option_class(disabled: bool, selected: bool, active: bool) -> &'static str {
    match (disabled, selected, active) {
        (true, _, _) => "lx-select-option-disabled",
        (false, true, true) => "lx-select-option-selected-active",
        (false, true, false) => "lx-select-option-selected",
        (false, false, true) => "lx-select-option-active",
        (false, false, false) => "lx-select-option-idle",
    }
}

/// Seta do campo: gira com o popup aberto.
fn arrow_class(open: bool) -> &'static str {
    if open {
        "lx-select-arrow-open"
    } else {
        "lx-select-arrow-closed"
    }
}

/// Como o `Select` lê e escreve a seleção (a única diferença entre `Select` e `SelectMultiple`).
pub(super) struct Model<T: Send + Sync + 'static> {
    pub selected: Signal<Vec<T>>,
    pub multiple: bool,
    /// Grava a nova seleção (uma lista com 0 ou 1 item no `Select`).
    pub set: Callback<Vec<T>>,
}

impl<T: Send + Sync> Clone for Model<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Send + Sync> Copy for Model<T> {}

/// Tudo que o `Select` e o `SelectMultiple` repassam: o controle e as props do `Field`.
pub(super) struct Config<T: Send + Sync + 'static> {
    pub model: Model<T>,
    pub options: Options<T>,
    pub id: String,
    pub placeholder: Option<String>,
    pub display_value: MaybeProp<String>,
    pub aria_label: Option<String>,
    pub clearable: bool,
    pub hide_dropdown_icon: bool,
    pub no_options_label: String,
    pub autofocus: bool,
    pub validation: FieldValidation,
    pub on_popup_show: Option<Callback<()>>,
    pub on_popup_hide: Option<Callback<()>>,
    pub on_clear: Option<Callback<Vec<T>>>,
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
    pub prepend: Option<FieldPrepend>,
    pub append: Option<FieldAppend>,
    pub before: Option<FieldBefore>,
    pub after: Option<FieldAfter>,
    pub hint_slot: Option<FieldHint>,
    pub error_slot: Option<FieldError>,
    pub loading_slot: Option<FieldLoading>,
}

/// Monta o `Field` com o controle dentro.
pub(super) fn select_field<T>(cfg: Config<T>) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
{
    let Config {
        model,
        options,
        id,
        placeholder,
        display_value,
        aria_label,
        clearable,
        hide_dropdown_icon,
        no_options_label,
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
    } = cfg;
    let selected = model.selected;

    // As opções e a seleção viram vetores simples: o resto do controle não depende de `T`.
    let items = options.items();
    let labels =
        Memo::new(move |_| items.with(|o| o.iter().map(|x| x.label.clone()).collect::<Vec<_>>()));
    let disabled_flags =
        Memo::new(move |_| items.with(|o| o.iter().map(|x| x.disable).collect::<Vec<_>>()));
    let selected_flags = Memo::new({
        let options = options.clone();
        move |_| {
            items.with(|o| {
                selected.with(|s| {
                    o.iter()
                        .map(|x| s.iter().any(|y| options.same(y, &x.value)))
                        .collect::<Vec<bool>>()
                })
            })
        }
    });
    // Escolher uma opção: o `Select` troca o valor; o `SelectMultiple` alterna.
    let choose = Callback::new({
        let options = options.clone();
        move |i: usize| {
            let Some(opt) = items.with_untracked(|o| o.get(i).cloned()) else {
                return;
            };
            if opt.disable {
                return;
            }
            if model.multiple {
                let mut current = selected.get_untracked();
                toggle(&mut current, opt.value, |a, b| options.same(a, b));
                model.set.run(current);
            } else {
                model.set.run(vec![opt.value]);
            }
        }
    });

    // O texto do campo: `display_value`, ou os labels escolhidos na ordem das opções.
    let text = Signal::derive(move || {
        if let Some(t) = display_value.get() {
            return t;
        }
        let picked: Vec<String> = labels.with(|l| {
            selected_flags.with(|f| {
                l.iter()
                    .zip(f)
                    .filter(|(_, on)| **on)
                    .map(|(l, _)| l.clone())
                    .collect()
            })
        });
        join_labels(&picked)
    });
    let has_value =
        Signal::derive(move || !text.with(String::is_empty) || selected.with(|s| !s.is_empty()));

    let input = NodeRef::<html::Input>::new();
    let open = RwSignal::new(false);
    let active = RwSignal::new(None::<usize>);
    let ctl = Ctl {
        o: Overlay::new(),
        open,
        active,
        input,
        labels,
        disabled_flags,
        selected_flags,
        choose,
        multiple: model.multiple,
        disable,
        readonly,
        params: StoredValue::new(Params {
            anchor: BOTTOM_START,
            self_: TOP_START,
            offset: (0.0, 4.0),
            fit: true,
            cover: false,
            max_height: None,
            max_width: None,
        }),
        typeahead: StoredValue::new(TypeAhead::default()),
        dismiss: StoredValue::new_local(Vec::new()),
        anchor_listeners: StoredValue::new_local(Vec::new()),
        mounted: StoredValue::new(false),
        id: StoredValue::new(id.clone()),
        on_show: on_popup_show,
        on_hide: on_popup_hide,
    };

    Effect::new(move |_| {
        if ctl.o.marker.get().is_some() {
            ctl.mount();
        }
    });
    Effect::new(move |_| ctl.sync());
    // Campo desabilitado ou somente leitura não fica aberto.
    Effect::new(move |_| {
        if (disable.get() || readonly.get()) && open.get_untracked() {
            open.set(false);
        }
    });
    // A opção ativa anda: mantém-na à vista dentro da lista.
    Effect::new(move |_| {
        let a = active.get();
        if open.get_untracked() {
            ctl.scroll_to(a);
        }
    });
    on_cleanup(move || ctl.cleanup());

    // Botão de limpar (como o `clearable` do `Input`) + seta do campo + o `append` do app.
    let show_clear =
        Signal::derive(move || clearable && has_value.get() && !disable.get() && !readonly.get());
    let clear = move || {
        let previous = selected.get_untracked();
        if let Some(el) = input.get_untracked() {
            let _ = el.focus();
        }
        open.set(false);
        model.set.run(Vec::new());
        if let Some(cb) = on_clear {
            cb.run(previous);
        }
        // Depois do próximo ciclo, quando a validação disparada pela mudança já rodou, limpa o erro.
        set_timeout(move || validation.reset_validation(), Duration::ZERO);
    };
    let append = Some(FieldAppend {
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
                    {(!hide_dropdown_icon)
                        .then(|| {
                            view! {
                                <div class=move || arrow_class(open.get()) aria-hidden="true">
                                    <Icon name="i-lucide-chevron-down" />
                                </div>
                            }
                        })}
                    {user.as_ref().map(|slot| (slot.children)())}
                }
                .into_any()
            })
        },
    });

    let shared = Shared {
        ctl,
        id: id.clone(),
        placeholder,
        aria_label,
        text,
        autofocus,
        no_options_label,
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

/// Estado e ações do `Select`. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
struct Ctl {
    o: Overlay,
    open: RwSignal<bool>,
    active: RwSignal<Option<usize>>,
    input: NodeRef<html::Input>,
    labels: Memo<Vec<String>>,
    disabled_flags: Memo<Vec<bool>>,
    selected_flags: Memo<Vec<bool>>,
    choose: Callback<usize>,
    multiple: bool,
    disable: Signal<bool>,
    readonly: Signal<bool>,
    params: StoredValue<Params>,
    typeahead: StoredValue<TypeAhead>,
    dismiss: StoredValue<Vec<Listener>, LocalStorage>,
    anchor_listeners: StoredValue<Vec<Listener>, LocalStorage>,
    mounted: StoredValue<bool>,
    id: StoredValue<String>,
    on_show: Option<Callback<()>>,
    on_hide: Option<Callback<()>>,
}

/// O que o controle precisa além do `Ctl`.
struct Shared {
    ctl: Ctl,
    id: String,
    placeholder: Option<String>,
    aria_label: Option<String>,
    text: Signal<String>,
    autofocus: bool,
    no_options_label: String,
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

impl Ctl {
    fn option_id(&self, i: usize) -> String {
        format!("{}_opt_{i}", self.id.get_value())
    }

    fn editable(&self) -> bool {
        !self.disable.get_untracked() && !self.readonly.get_untracked()
    }

    /// A âncora é a caixa do campo (`.lx-field-box`): o popup nasce embaixo dela, com a largura
    /// dela. Um clique em qualquer ponto da caixa (menos num botão) abre e fecha.
    fn mount(&self) {
        if self.mounted.get_value() {
            return;
        }
        let Some(marker) = self.o.marker.get_untracked() else {
            return;
        };
        let Ok(Some(anchor)) = marker.closest(".lx-field-box") else {
            return;
        };
        self.mounted.set_value(true);
        self.o.set_anchor(anchor.clone());
        let t: &web_sys::EventTarget = anchor.as_ref();
        let this = *self;
        // Sem isto, tocar na seta (que não é focável) tiraria o foco do campo (o `blur` fecharia
        // o popup e o `click` o reabriria).
        let down = Listener::on(t, "mousedown", move |e| {
            if !in_control(&e) {
                e.prevent_default();
            }
        });
        let click = Listener::on(t, "click", move |e| {
            if !in_button(&e) && this.editable() {
                this.toggle();
            }
        });
        self.anchor_listeners.set_value(vec![down, click]);
    }

    fn sync(&self) {
        // `get` (e não `get_untracked`): roda de novo quando o `Portal` montar o popup
        if self.o.el.get().is_none() {
            return;
        }
        match (self.open.get(), self.o.phase.get_untracked()) {
            (true, Phase::Closed) => self.do_open(),
            (true, Phase::Leaving) => {
                self.o.revive();
                self.arm_dismiss();
            }
            (false, Phase::Open) => self.leave(),
            _ => {}
        }
    }

    fn do_open(&self) {
        let on_show = self.on_show;
        let shown = self.o.open(&self.params.get_value(), move || {
            if let Some(cb) = on_show {
                cb.run(());
            }
        });
        if !shown {
            leptos::logging::warn!("Select: sem âncora ou sem suporte à Popover API");
            self.open.set(false);
            return;
        }
        self.arm_dismiss();
        // o popup acabou de ficar visível: a opção ativa já pode ser rolada para a vista
        self.scroll_to(self.active.get_untracked());
    }

    fn leave(&self) {
        self.disarm_dismiss();
        self.active.set(None);
        self.typeahead.update_value(TypeAhead::reset);
        let on_hide = self.on_hide;
        self.o.leave(move || {
            if let Some(cb) = on_hide {
                cb.run(());
            }
        });
    }

    /// Abre com a opção `initial` ativa (ou a da seleção / a primeira habilitada).
    fn show(&self, initial: Option<usize>, from_end: bool) {
        if !self.editable() {
            return;
        }
        let a = initial.or_else(|| {
            self.disabled_flags.with_untracked(|d| {
                self.selected_flags
                    .with_untracked(|s| initial_active(d, s, from_end))
            })
        });
        self.active.set(a);
        // clicar na seta (ou no recuo) não focou o campo: o foco precisa estar nele
        if let Some(el) = self.input.get_untracked() {
            focus_quietly(&el);
        }
        self.open.set(true);
    }

    fn toggle(&self) {
        if self.open.get_untracked() {
            self.open.set(false);
        } else {
            self.show(None, false);
        }
    }

    fn scroll_to(&self, i: Option<usize>) {
        let Some(i) = i else { return };
        let Some(el) = document().and_then(|d| d.get_element_by_id(&self.option_id(i))) else {
            return;
        };
        let opts = web_sys::ScrollIntoViewOptions::new();
        opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
        el.scroll_into_view_with_scroll_into_view_options(&opts);
    }

    /// Escolhe a opção `i`. No `Select` fecha; no `SelectMultiple` a lista continua aberta.
    fn pick(&self, i: usize) {
        if self
            .disabled_flags
            .with_untracked(|d| d.get(i).copied().unwrap_or(true))
        {
            return;
        }
        self.typeahead.update_value(TypeAhead::reset);
        self.choose.run(i);
        if !self.multiple {
            self.open.set(false);
        }
    }

    fn hover(&self, i: usize) {
        let off = self
            .disabled_flags
            .with_untracked(|d| d.get(i).copied().unwrap_or(true));
        if !off && self.active.get_untracked() != Some(i) {
            self.active.set(Some(i));
        }
    }

    fn arm_dismiss(&self) {
        self.disarm_dismiss();
        let Some(doc) = document() else { return };
        let t: &web_sys::EventTarget = doc.as_ref();
        let this = *self;
        let id = self.o.id;
        esc_push(id);
        let esc = Listener::on(t, "keydown", move |e| {
            let Ok(e) = e.dyn_into::<web_sys::KeyboardEvent>() else {
                return;
            };
            if e.key() == "Escape" && !e.default_prevented() && esc_is_top(id) {
                // dentro de um `Dialog`, o mesmo `Esc` não pode fechar o diálogo também
                e.prevent_default();
                this.open.set(false);
            }
        });
        let outside = Listener::new(
            t,
            "pointerdown",
            ListenerOpts {
                capture: true,
                passive: true,
            },
            move |e| this.on_outside(&e),
        );
        self.dismiss.set_value(vec![esc, outside]);
    }

    fn disarm_dismiss(&self) {
        self.dismiss.set_value(Vec::new());
        esc_remove(self.o.id);
    }

    /// `pointerdown` fora do popup e do campo fecha (o clique dentro do campo alterna sozinho).
    fn on_outside(&self, e: &web_sys::Event) {
        let Some(node) = e.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()) else {
            return;
        };
        let in_popup = self.o.popup().is_some_and(|p| p.contains(Some(&node)));
        let in_anchor = self.o.anchor().is_some_and(|a| a.contains(Some(&node)));
        if !in_popup && !in_anchor && self.o.phase.get_untracked() == Phase::Open {
            self.open.set(false);
        }
    }

    fn on_keydown(&self, e: &ev::KeyboardEvent) {
        if !self.editable() || e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        let key = e.key();
        let is_open = self.open.get_untracked();
        let searching = self.typeahead.with_value(|t| t.is_active(now_ms()));
        if let Some(nav) = nav_key(&key) {
            e.prevent_default();
            if is_open {
                self.move_to(nav);
            } else if matches!(nav, NavKey::Next | NavKey::Prev) {
                self.show(None, nav == NavKey::Prev);
            }
        } else if key == "Enter" || (key == " " && !searching) {
            // Enter não envia o formulário: abre ou escolhe
            e.prevent_default();
            if !is_open {
                self.show(None, false);
            } else if let Some(i) = self.active.get_untracked() {
                self.pick(i);
            } else {
                self.open.set(false);
            }
        } else if key == "Tab" {
            // o foco segue para o próximo controle; o popup só fecha
            if is_open {
                self.open.set(false);
            }
        } else if let Some(c) = typeahead_char(&key, searching) {
            e.prevent_default();
            self.type_ahead(c, is_open);
        }
    }

    fn move_to(&self, nav: NavKey) {
        let next = self
            .disabled_flags
            .with_untracked(|d| move_active(d, self.active.get_untracked(), nav));
        if next.is_some() {
            self.active.set(next);
        }
    }

    fn type_ahead(&self, c: char, is_open: bool) {
        let now = now_ms();
        let buffer = self
            .typeahead
            .try_update_value(|t| t.push(c, now).to_string())
            .unwrap_or_default();
        // a busca parte da opção ativa; fechado, da selecionada
        let from = if is_open {
            self.active.get_untracked()
        } else {
            self.selected_flags
                .with_untracked(|s| s.iter().position(|on| *on))
        };
        let found = self.labels.with_untracked(|l| {
            self.disabled_flags
                .with_untracked(|d| find_prefix(l, d, &buffer, from))
        });
        let Some(i) = found else { return };
        if is_open {
            self.active.set(Some(i));
        } else {
            self.show(Some(i), false);
        }
    }

    fn cleanup(&self) {
        self.disarm_dismiss();
        self.anchor_listeners.set_value(Vec::new());
        self.o.cleanup();
    }
}

/// O alvo do evento é o `<input>` do controle (o foco normal vale).
fn in_control(e: &web_sys::Event) -> bool {
    closest_of(e, "input, button")
}

/// O alvo do evento está num botão (o de limpar).
fn in_button(e: &web_sys::Event) -> bool {
    closest_of(e, "button")
}

fn closest_of(e: &web_sys::Event, selector: &str) -> bool {
    e.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|el| el.closest(selector).ok().flatten().is_some())
}

/// O `<input readonly role="combobox">`, o marcador da âncora e o popup (`Portal`).
///
/// Um `<input>` (e não um `<div tabindex>`): o `<label for>` do `Field` o rotula, o `Form` o foca
/// (o `Field` procura um `input`), o `placeholder` é nativo e o foco/estilo do campo são os do
/// `Input`. `readonly` é só para o teclado do celular e a edição; o `aria-readonly` do ARIA diz o
/// `readonly` do campo de fato.
fn control_view(sh: Shared) -> impl IntoView {
    let Shared {
        ctl,
        id,
        placeholder,
        aria_label,
        text,
        autofocus,
        no_options_label,
    } = sh;
    let ctx = crate::use_field_context().expect("o controle do Select vai dentro de um <Field>");
    let class = format!("{} lx-select-input", ctx.control_class);
    let (open, active, disable, readonly) = (ctl.open, ctl.active, ctl.disable, ctl.readonly);
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
            if !open.get() {
                return None;
            }
            active.get().map(|i| format!("{id}_opt_{i}"))
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

    let multiple = ctl.multiple;
    let (labels, disabled_flags, selected_flags) =
        (ctl.labels, ctl.disabled_flags, ctl.selected_flags);
    let phase = ctl.o.phase;
    let popup = {
        let (id, list_id, list_label) = (id.clone(), list_id.clone(), aria_label.clone());
        move || {
            let class = move || {
                cls([
                    "lx-menu",
                    if phase.get() == Phase::Open {
                        "lx-menu-open"
                    } else {
                        "lx-menu-enter"
                    },
                ])
            };
            let empty = move || labels.with(Vec::is_empty);
            let options = {
                let id = id.clone();
                move || {
                    labels
                        .get()
                        .into_iter()
                        .enumerate()
                        .map(|(i, label)| {
                            let off = move || disabled_flags.with(|d| d.get(i).copied().unwrap_or(false));
                            let on = move || selected_flags.with(|s| s.get(i).copied().unwrap_or(false));
                            view! {
                                <li
                                    id=format!("{id}_opt_{i}")
                                    role="option"
                                    class=move || option_class(off(), on(), active.get() == Some(i))
                                    aria-selected=move || on().to_string()
                                    aria-disabled=move || off().then_some("true")
                                    on:click=move |_| ctl.pick(i)
                                    on:mousemove=move |_| ctl.hover(i)
                                >
                                    <span class="lx-select-option-label">{label}</span>
                                    {move || {
                                        on()
                                            .then(|| {
                                                view! {
                                                    <span aria-hidden="true" class="lx-select-option-check">
                                                        <Icon name="i-lucide-check" />
                                                    </span>
                                                }
                                            })
                                    }}
                                </li>
                            }
                        })
                        .collect_view()
                }
            };
            let no_options = no_options_label.clone();
            // `mousedown` com `preventDefault`: clicar na lista não tira o foco do campo (o
            // `aria-activedescendant` vive nele)
            view! {
                <div
                    node_ref=ctl.o.el
                    popover="manual"
                    class=class
                    on:mousedown=move |e: ev::MouseEvent| e.prevent_default()
                >
                    <ul
                        id=list_id.clone()
                        role=move || (!empty()).then_some("listbox")
                        class="lx-select-list"
                        aria-multiselectable=multiple.then_some("true")
                        aria-label=list_label.clone()
                    >
                        {options}
                    </ul>
                    {move || {
                        empty()
                            .then(|| {
                                view! { <div class="lx-select-empty">{no_options.clone()}</div> }
                            })
                    }}
                </div>
            }
        }
    };

    view! {
        <input
            node_ref=ctl.input
            id=id
            type="text"
            class=class
            role="combobox"
            readonly=true
            autocomplete="off"
            placeholder=placeholder
            data-autofocus=data_autofocus
            disabled=move || disable.get()
            aria-haspopup="listbox"
            aria-expanded=move || open.get().to_string()
            aria-controls=list_id
            aria-activedescendant=active_id
            aria-label=aria_label.clone()
            aria-readonly=move || readonly.get().to_string()
            aria-invalid=move || has_error.get().then_some("true")
            aria-describedby=described_by
            value=text.get_untracked()
            prop:value=move || text.get()
            on:keydown=move |e| ctl.on_keydown(&e)
            // o popup nunca recebe o foco: sair do campo (programaticamente, ou pela janela) fecha
            on:blur=move |_| open.set(false)
        />
        <span hidden=true node_ref=ctl.o.marker></span>
        <Portal>{popup()}</Portal>
    }
}
