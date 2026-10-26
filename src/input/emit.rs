//! Emissão do valor do `Input` para o `value`, com `debounce` (≈ `emitValue`/`emitTimer`/
//! `emitValueFn` do `QInput.js`).
//!
//! [`EmitState`] é a lógica pura (testada no host): uma emissão **pendente** que o próximo
//! `emit` substitui, que `flush` entrega e que `cancel` descarta. [`Emitter`] a liga ao sinal
//! `value` e ao `set_timeout` (só em handlers e `Effect`s, nunca durante a renderização).

use std::time::Duration;

use leptos::leptos_dom::helpers::{TimeoutHandle, set_timeout_with_handle};
use leptos::prelude::*;

/// Uma emissão adiada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pending {
    /// O valor a escrever em `value`.
    pub val: String,
    /// A escrita deve ser marcada como "eco do próprio `Input`" (o `stopWatcher` da máscara).
    pub stop_watcher: bool,
}

/// Estado puro da emissão pendente.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct EmitState {
    pending: Option<Pending>,
}

impl EmitState {
    /// Há uma emissão adiada.
    #[cfg(test)]
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Agenda uma emissão, substituindo a pendente (cada tecla reinicia a espera).
    pub fn schedule(&mut self, val: String, stop_watcher: bool) {
        self.pending = Some(Pending { val, stop_watcher });
    }

    /// Entrega a emissão pendente (a pausa acabou, ou o campo perdeu o foco).
    pub fn take(&mut self) -> Option<Pending> {
        self.pending.take()
    }

    /// Descarta a emissão pendente (o modelo mudou por fora, o texto exibido voltou a coincidir
    /// com o modelo, ou o campo foi limpo).
    pub fn cancel(&mut self) {
        self.pending = None;
    }
}

/// `emitValueFn` só escreve se o modelo é outro (`props.modelValue !== val`).
pub(super) fn should_write(model: &str, val: &str) -> bool {
    model != val
}

/// Liga o [`EmitState`] ao sinal `value` e ao relógio. `Copy`: só guarda handles.
#[derive(Clone, Copy)]
pub(crate) struct Emitter {
    value: RwSignal<String>,
    /// Pausa em ms; `None` = sem `debounce` (escreve na hora).
    debounce: Option<u32>,
    state: StoredValue<EmitState>,
    timer: StoredValue<Option<TimeoutHandle>, LocalStorage>,
    /// Último valor que o próprio `Input` escreveu com `stop_watcher` (≈ `emitCachedValue`
    /// + `stopValueWatcher`): o `Effect` do modelo ignora esse eco.
    emitted: StoredValue<Option<String>>,
}

impl Emitter {
    pub fn new(value: RwSignal<String>, debounce: Option<u32>) -> Self {
        Self {
            value,
            debounce,
            state: StoredValue::new(EmitState::default()),
            timer: StoredValue::new_local(None),
            emitted: StoredValue::new(None),
        }
    }

    /// O eco registrado pela última escrita com `stop_watcher`, se ainda não foi consumido.
    pub fn take_echo(&self) -> Option<String> {
        self.emitted.try_update_value(Option::take).flatten()
    }

    /// Esquece o eco registrado.
    pub fn clear_echo(&self) {
        self.emitted.try_set_value(None);
    }

    /// Escreve `val` em `value` (agora, ou depois da pausa do `debounce`).
    pub fn emit(&self, val: String, stop_watcher: bool) {
        let Some(ms) = self.debounce else {
            self.write(&val, stop_watcher);
            return;
        };
        self.clear_timer();
        self.state.update_value(|s| s.schedule(val, stop_watcher));
        let this = *self;
        let handle = set_timeout_with_handle(
            move || {
                this.timer.try_set_value(None);
                this.flush();
            },
            Duration::from_millis(u64::from(ms)),
        )
        .ok();
        self.timer.try_set_value(handle);
    }

    /// Entrega já a emissão pendente (blur, `change`, Enter, descarte do campo).
    pub fn flush(&self) {
        self.clear_timer();
        if let Some(p) = self.state.try_update_value(EmitState::take).flatten() {
            self.write(&p.val, p.stop_watcher);
        }
    }

    /// Descarta a emissão pendente e o relógio.
    pub fn cancel(&self) {
        self.clear_timer();
        self.state.try_update_value(EmitState::cancel);
    }

    fn clear_timer(&self) {
        if let Some(h) = self.timer.try_update_value(Option::take).flatten() {
            h.clear();
        }
    }

    fn write(&self, val: &str, stop_watcher: bool) {
        let Some(model) = self.value.try_get_untracked() else {
            return;
        };
        if should_write(&model, val) {
            if stop_watcher {
                self.emitted.try_set_value(Some(val.to_string()));
            }
            self.value.try_set(val.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_emission_replaces_the_pending_one() {
        let mut s = EmitState::default();
        assert!(!s.has_pending());
        s.schedule("a".into(), false);
        s.schedule("ab".into(), true);
        assert!(s.has_pending());
        assert_eq!(
            s.take(),
            Some(Pending {
                val: "ab".into(),
                stop_watcher: true
            })
        );
        assert!(!s.has_pending());
        assert_eq!(s.take(), None);
    }

    #[test]
    fn cancel_drops_the_pending_emission() {
        let mut s = EmitState::default();
        s.schedule("a".into(), false);
        s.cancel();
        assert_eq!(s.take(), None);
    }

    #[test]
    fn writes_only_when_the_model_differs() {
        assert!(should_write("a", "b"));
        assert!(!should_write("a", "a"));
        assert!(should_write("", "x"));
        assert!(should_write("x", ""));
    }
}
