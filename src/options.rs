//! Options do [`Select`](crate::Select) e do [`SelectMultiple`](crate::SelectMultiple), em três
//! formatos (ADR-0012). Sem DOM: o módulo só normaliza a lista (rótulo, valor, desabilitada) e
//! guarda como **comparar** dois valores (a identidade da opção).
//!
//! | Formato | Entrada | Rótulo | Modelo |
//! |---|---|---|---|
//! | 1. primitivos | `Vec<P>` / `Signal<Vec<P>>` | `Display` de `P` | `P` |
//! | 2. `{ key, label }` | `Vec<Opt<K>>` / `Signal<Vec<Opt<K>>>` | `label` | `K` |
//! | 3. objetos | [`ObjectOptions::new`] + `key_field` + `label_field` | `label_field(&T)` | `T`; com [`key_as_value`](ObjectOptions::key_as_value), `K` |
//!
//! Os formatos 1 e 2 entram por `Into<Options<_>>` (`options=vec!["a", "b"]`); o 3 pelo builder
//! [`ObjectOptions`], que usa *type-state*: esquecer `key_field` ou `label_field` não compila.
//!
//! ```
//! use leptos_nebula::{ObjectOptions, Opt, Options};
//!
//! // 1. primitivos
//! let _: Options<&str> = vec!["Brasil", "Chile"].into();
//! // 2. { key, label }
//! let _: Options<u32> = vec![Opt::new(1, "Brasil"), Opt::new(2, "Chile").disabled()].into();
//! // 3. objetos (o modelo guarda o objeto) ou só a key
//! #[derive(Clone)]
//! struct Country { id: u32, name: String }
//! let items = vec![Country { id: 1, name: "Brasil".into() }];
//! let _: Options<Country> = ObjectOptions::new(items)
//!     .key_field(|c: &Country| c.id)
//!     .label_field(|c: &Country| c.name.clone())
//!     .into();
//! ```
//!
//! Falta o `label_field`: não compila.
//!
//! ```compile_fail,E0277
//! use leptos_nebula::{ObjectOptions, Options};
//! #[derive(Clone)]
//! struct Country { id: u32 }
//! let _: Options<Country> = ObjectOptions::new(vec![Country { id: 1 }])
//!     .key_field(|c: &Country| c.id)
//!     .into();
//! ```
//!
//! `key_as_value` sem `key_field`: não compila.
//!
//! ```compile_fail,E0599
//! use leptos_nebula::{ObjectOptions, Options};
//! #[derive(Clone)]
//! struct Country { name: String }
//! let _ = ObjectOptions::new(vec![Country { name: String::new() }])
//!     .label_field(|c: &Country| c.name.clone())
//!     .key_as_value();
//! ```

use std::fmt::Display;
use std::sync::Arc;

use leptos::prelude::*;

/// Uma opção já normalizada.
#[derive(Clone, Debug)]
pub(crate) struct Item<V> {
    pub label: String,
    pub value: V,
    pub disable: bool,
}

type Same<V> = Arc<dyn Fn(&V, &V) -> bool + Send + Sync>;
type DisableFn<T> = Arc<dyn Fn(&T) -> bool + Send + Sync>;

/// A lista de opções que o `Select` consome: rótulo, valor `V` e se está desabilitada, mais a
/// identidade (como saber se dois `V` são a mesma opção). Reativa: vem de `Vec` ou `Signal<Vec>`.
///
/// Não se constrói à mão: use `.into()` num `Vec<P>` (formato 1) ou `Vec<Opt<K>>` (formato 2), ou
/// [`ObjectOptions::new`] (formato 3).
pub struct Options<V: Send + Sync + 'static> {
    items: Signal<Vec<Item<V>>>,
    same: Same<V>,
}

impl<V: Send + Sync + 'static> Clone for Options<V> {
    fn clone(&self) -> Self {
        Self {
            items: self.items,
            same: Arc::clone(&self.same),
        }
    }
}

impl<V: Send + Sync + 'static> Options<V> {
    /// As opções normalizadas (reativo).
    pub(crate) fn items(&self) -> Signal<Vec<Item<V>>> {
        self.items
    }

    /// `true` se `a` e `b` são o mesmo valor (a identidade do formato).
    pub(crate) fn same(&self, a: &V, b: &V) -> bool {
        (self.same)(a, b)
    }
}

// — Formato 1 —

impl<P> From<Signal<Vec<P>>> for Options<P>
where
    P: Display + PartialEq + Clone + Send + Sync + 'static,
{
    fn from(list: Signal<Vec<P>>) -> Self {
        Options {
            items: Signal::derive(move || {
                list.with(|l| {
                    l.iter()
                        .map(|p| Item {
                            label: p.to_string(),
                            value: p.clone(),
                            disable: false,
                        })
                        .collect()
                })
            }),
            same: Arc::new(|a: &P, b: &P| a == b),
        }
    }
}

impl<P> From<Vec<P>> for Options<P>
where
    P: Display + PartialEq + Clone + Send + Sync + 'static,
{
    fn from(list: Vec<P>) -> Self {
        Self::from(Signal::<Vec<P>>::from(list))
    }
}

// — Formato 2 —

/// Formato 2: uma opção `{ key, label }`. O modelo do `Select` guarda a **key**.
#[derive(Clone, Debug, PartialEq)]
pub struct Opt<K> {
    /// A identidade da opção: o que vai para o modelo.
    pub key: K,
    /// Texto da opção (e o que o campo exibe quando ela está escolhida).
    pub label: String,
    /// Opção desabilitada: aparece, mas não é escolhida nem recebe a navegação.
    pub disable: bool,
}

impl<K> Opt<K> {
    /// Opção habilitada.
    pub fn new(key: K, label: impl Into<String>) -> Self {
        Self {
            key,
            label: label.into(),
            disable: false,
        }
    }

    /// Esta mesma opção, desabilitada.
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.disable = true;
        self
    }
}

impl<K> From<Signal<Vec<Opt<K>>>> for Options<K>
where
    K: PartialEq + Clone + Send + Sync + 'static,
{
    fn from(list: Signal<Vec<Opt<K>>>) -> Self {
        Options {
            items: Signal::derive(move || {
                list.with(|l| {
                    l.iter()
                        .map(|o| Item {
                            label: o.label.clone(),
                            value: o.key.clone(),
                            disable: o.disable,
                        })
                        .collect()
                })
            }),
            same: Arc::new(|a: &K, b: &K| a == b),
        }
    }
}

impl<K> From<Vec<Opt<K>>> for Options<K>
where
    K: PartialEq + Clone + Send + Sync + 'static,
{
    fn from(list: Vec<Opt<K>>) -> Self {
        Self::from(Signal::<Vec<Opt<K>>>::from(list))
    }
}

// — Formato 3 —

/// Estado do builder: `key_field` ainda não foi dado.
pub struct NoKey;
/// Estado do builder: `label_field` ainda não foi dado.
pub struct NoLabel;
/// Estado do builder: `key_field` dado (guarda a closure).
pub struct HasKey<T, K>(Arc<dyn Fn(&T) -> K + Send + Sync>);
/// Estado do builder: `label_field` dado (guarda a closure).
pub struct HasLabel<T>(Arc<dyn Fn(&T) -> String + Send + Sync>);

/// Formato 3: o builder de [`ObjectOptions::new`]. `K` é a key; `T` **não** precisa de `PartialEq`
/// (a identidade é a key). Os parâmetros `KS` e `LS` são o *type-state*: só com `key_field` **e**
/// `label_field` o builder vira [`Options<T>`] (`.into()`); `key_as_value` pede `key_field`.
pub struct ObjectOptions<T: Send + Sync + 'static, KS, LS> {
    items: Signal<Vec<T>>,
    key: KS,
    label: LS,
    disable: Option<DisableFn<T>>,
}

impl<T: Send + Sync + 'static> ObjectOptions<T, NoKey, NoLabel> {
    /// Formato 3: começa o builder a partir de uma lista de objetos (`Vec<T>` ou
    /// `Signal<Vec<T>>`). Faltam `key_field` e `label_field` (os dois obrigatórios).
    pub fn new(items: impl Into<Signal<Vec<T>>>) -> Self {
        ObjectOptions {
            items: items.into(),
            key: NoKey,
            label: NoLabel,
            disable: None,
        }
    }
}

impl<T: Send + Sync + 'static, KS, LS> ObjectOptions<T, KS, LS> {
    /// Como obter a key (a identidade) de um objeto.
    pub fn key_field<K>(
        self,
        f: impl Fn(&T) -> K + Send + Sync + 'static,
    ) -> ObjectOptions<T, HasKey<T, K>, LS> {
        ObjectOptions {
            items: self.items,
            key: HasKey(Arc::new(f)),
            label: self.label,
            disable: self.disable,
        }
    }

    /// Como obter o rótulo de um objeto.
    pub fn label_field(
        self,
        f: impl Fn(&T) -> String + Send + Sync + 'static,
    ) -> ObjectOptions<T, KS, HasLabel<T>> {
        ObjectOptions {
            items: self.items,
            key: self.key,
            label: HasLabel(Arc::new(f)),
            disable: self.disable,
        }
    }

    /// Opcional: quais objetos aparecem desabilitados.
    #[must_use]
    pub fn disable_field(mut self, f: impl Fn(&T) -> bool + Send + Sync + 'static) -> Self {
        self.disable = Some(Arc::new(f));
        self
    }
}

impl<T, K> ObjectOptions<T, HasKey<T, K>, HasLabel<T>>
where
    T: Send + Sync + 'static,
    K: PartialEq + Clone + Send + Sync + 'static,
{
    /// O modelo guarda a **key** em vez do objeto (≈ `emit-value`): `Options<K>`. Muda o tipo do
    /// valor do `Select`. Só existe depois de `key_field` (e `label_field`).
    pub fn key_as_value(self) -> Options<K> {
        self.normalize(|_, k| k.clone(), Arc::new(|a: &K, b: &K| a == b))
    }

    fn normalize<V>(
        self,
        value: impl Fn(&T, K) -> V + Send + Sync + 'static,
        same: Same<V>,
    ) -> Options<V>
    where
        V: Send + Sync + 'static,
    {
        let ObjectOptions {
            items,
            key,
            label,
            disable,
        } = self;
        let (key, label) = (key.0, label.0);
        Options {
            items: Signal::derive(move || {
                items.with(|l| {
                    l.iter()
                        .map(|t| Item {
                            label: label(t),
                            value: value(t, key(t)),
                            disable: disable.as_ref().is_some_and(|d| d(t)),
                        })
                        .collect()
                })
            }),
            same,
        }
    }
}

/// O formato 3 completo vira `Options<T>`: o modelo guarda o objeto, a identidade é a key.
impl<T, K> From<ObjectOptions<T, HasKey<T, K>, HasLabel<T>>> for Options<T>
where
    T: Clone + Send + Sync + 'static,
    K: PartialEq + Clone + Send + Sync + 'static,
{
    fn from(o: ObjectOptions<T, HasKey<T, K>, HasLabel<T>>) -> Self {
        let key = Arc::clone(&o.key.0);
        o.normalize(
            |t, _| t.clone(),
            Arc::new(move |a: &T, b: &T| key(a) == key(b)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels<V: Send + Sync + Clone>(o: &Options<V>) -> Vec<String> {
        o.items()
            .get_untracked()
            .into_iter()
            .map(|i| i.label)
            .collect()
    }
    fn values<V: Send + Sync + Clone>(o: &Options<V>) -> Vec<V> {
        o.items()
            .get_untracked()
            .into_iter()
            .map(|i| i.value)
            .collect()
    }
    fn flags<V: Send + Sync + Clone>(o: &Options<V>) -> Vec<bool> {
        o.items()
            .get_untracked()
            .into_iter()
            .map(|i| i.disable)
            .collect()
    }

    // Sem `PartialEq`, de propósito: a identidade do formato 3 é a key.
    #[derive(Clone, Debug)]
    struct User {
        id: u32,
        name: String,
        banned: bool,
    }

    fn users() -> Vec<User> {
        vec![
            User {
                id: 1,
                name: "Ana".into(),
                banned: false,
            },
            User {
                id: 2,
                name: "Bia".into(),
                banned: true,
            },
        ]
    }

    fn user_builder() -> ObjectOptions<User, HasKey<User, u32>, HasLabel<User>> {
        ObjectOptions::new(users())
            .key_field(|u: &User| u.id)
            .label_field(|u: &User| u.name.clone())
    }

    #[test]
    fn format_1_label_is_display_and_identity_is_eq() {
        let o: Options<i32> = vec![10, 20].into();
        assert_eq!(labels(&o), ["10", "20"]);
        assert_eq!(values(&o), [10, 20]);
        assert_eq!(flags(&o), [false, false]);
        assert!(o.same(&10, &10));
        assert!(!o.same(&10, &20));
        let s: Options<&str> = vec!["a", "b"].into();
        assert_eq!(labels(&s), ["a", "b"]);
    }

    #[test]
    fn format_2_model_is_the_key() {
        let o: Options<u32> = vec![Opt::new(7, "Sete"), Opt::new(8, "Oito")].into();
        assert_eq!(labels(&o), ["Sete", "Oito"]);
        assert_eq!(values(&o), [7, 8]);
        assert!(o.same(&7, &7) && !o.same(&7, &8));
    }

    #[test]
    fn disabled_option() {
        let o: Options<u32> = vec![Opt::new(1, "a"), Opt::new(2, "b").disabled()].into();
        assert_eq!(flags(&o), [false, true]);
        assert!(Opt::new(1, "x").disabled().disable);
    }

    #[test]
    fn format_3_keeps_the_object_and_compares_by_key() {
        let o: Options<User> = user_builder().into();
        assert_eq!(labels(&o), ["Ana", "Bia"]);
        let vs = values(&o);
        assert_eq!(vs[0].id, 1);
        // Outro objeto com a mesma key (e outros campos) é a mesma opção.
        let same_key = User {
            id: 1,
            name: "Outra".into(),
            banned: false,
        };
        assert!(o.same(&vs[0], &same_key));
        assert!(!o.same(&vs[0], &vs[1]));
        assert_eq!(flags(&o), [false, false]);
    }

    #[test]
    fn format_3_disable_field() {
        let o: Options<User> = user_builder().disable_field(|u| u.banned).into();
        assert_eq!(flags(&o), [false, true]);
    }

    #[test]
    fn key_as_value_returns_the_key() {
        let o = user_builder().disable_field(|u| u.banned).key_as_value();
        assert_eq!(values(&o), [1, 2]);
        assert_eq!(labels(&o), ["Ana", "Bia"]);
        assert_eq!(flags(&o), [false, true]);
        assert!(o.same(&1, &1) && !o.same(&1, &2));
    }

    #[test]
    fn options_follow_the_signal() {
        let list = RwSignal::new(vec![1, 2]);
        let sig: Signal<Vec<i32>> = list.into();
        let o: Options<i32> = sig.into();
        assert_eq!(values(&o), [1, 2]);
        list.set(vec![3]);
        assert_eq!(values(&o), [3]);
    }
}
