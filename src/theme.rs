//! Store de tema: `data-theme`/`data-mode` no `<html>`, persistidos no `localStorage`.
//!
//! Modelo do `Dark.set(true | false | 'auto')` do Quasar. Nada toca o DOM durante a
//! renderização (SSR): leitura do armazenamento e `matchMedia` acontecem num `Effect`.

use leptos::prelude::*;

const STORAGE_THEME: &str = "lx.theme";
const STORAGE_MODE: &str = "lx.mode";

/// Tema visual. Só os que têm arquivo de tokens são aceitos ([`Theme::AVAILABLE`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Theme {
    #[default]
    Scandi,
    /// Ainda sem tokens (plano futuro).
    Material,
    /// Ainda sem tokens (plano futuro).
    Glass,
}

impl Theme {
    /// Temas com arquivo de tokens.
    pub const AVAILABLE: [Theme; 1] = [Theme::Scandi];

    /// Valor de `data-theme`.
    pub fn id(self) -> &'static str {
        match self {
            Theme::Scandi => "scandi",
            Theme::Material => "material",
            Theme::Glass => "glass",
        }
    }

    pub fn is_available(self) -> bool {
        Self::AVAILABLE.contains(&self)
    }

    /// Lê um id salvo; ids de temas ainda sem tokens são recusados.
    pub fn parse(id: &str) -> Option<Self> {
        Self::AVAILABLE.into_iter().find(|t| t.id() == id)
    }
}

/// Modo de cor. `Auto` segue `prefers-color-scheme`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    Light,
    Dark,
    #[default]
    Auto,
}

impl Mode {
    pub fn id(self) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
            Mode::Auto => "auto",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "light" => Some(Mode::Light),
            "dark" => Some(Mode::Dark),
            "auto" => Some(Mode::Auto),
            _ => None,
        }
    }

    /// Valor de `data-mode` (`light`/`dark`), resolvendo `Auto` pela preferência do sistema.
    pub fn resolve(self, system_dark: bool) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
            Mode::Auto if system_dark => "dark",
            Mode::Auto => "light",
        }
    }

    /// Próximo modo no ciclo claro → escuro → auto.
    pub fn next(self) -> Self {
        match self {
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Auto,
            Mode::Auto => Mode::Light,
        }
    }
}

/// Estado de tema compartilhado por contexto.
#[derive(Clone, Copy)]
pub struct ThemeStore {
    pub mode: RwSignal<Mode>,
    pub theme: RwSignal<Theme>,
    /// `prefers-color-scheme: dark` no momento (só muda no navegador).
    system_dark: RwSignal<bool>,
}

impl ThemeStore {
    /// `data-mode` resolvido (`light`/`dark`).
    pub fn resolved_mode(&self) -> &'static str {
        self.mode.get().resolve(self.system_dark.get())
    }
}

/// Cria a store, a põe no contexto e mantém `<html>` e `localStorage` sincronizados.
/// Chame uma vez na raiz do app.
pub fn provide_theme() -> ThemeStore {
    let store = ThemeStore {
        mode: RwSignal::new(Mode::Auto),
        theme: RwSignal::new(Theme::Scandi),
        system_dark: RwSignal::new(false),
    };
    provide_context(store);
    let root = crate::root::use_root();

    // Só roda no navegador: Effects não executam durante a renderização no servidor.
    Effect::new(move |prev: Option<()>| {
        if prev.is_none() {
            load(store);
        }
        let theme = store.theme.get();
        let mode = store.mode.get();
        let resolved = mode.resolve(store.system_dark.get());
        if let Some(target) = crate::root::theme_target(root) {
            let _ = target.set_attribute("data-theme", theme.id());
            let _ = target.set_attribute("data-mode", resolved);
        }
        storage_set(STORAGE_THEME, theme.id());
        storage_set(STORAGE_MODE, mode.id());
    });
    store
}

/// A store de tema do contexto. Panica se [`provide_theme`] não foi chamado (invariante do app).
pub fn use_theme() -> ThemeStore {
    expect_context::<ThemeStore>()
}

/// Lê o salvo e assina `prefers-color-scheme` (reage a mudanças do sistema).
fn load(store: ThemeStore) {
    if let Some(theme) = storage_get(STORAGE_THEME).and_then(|s| Theme::parse(&s)) {
        store.theme.set(theme);
    }
    if let Some(mode) = storage_get(STORAGE_MODE).and_then(|s| Mode::parse(&s)) {
        store.mode.set(mode);
    }
    let Some(mql) = web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
    else {
        return;
    };
    store.system_dark.set(mql.matches());
    let on_change = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MediaQueryListEvent)>::new(
        move |e: web_sys::MediaQueryListEvent| store.system_dark.set(e.matches()),
    );
    let _ = mql.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
    // A store vive o tempo todo do app; o listener também.
    on_change.forget();
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn storage_get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok()?
}

fn storage_set(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
}

use wasm_bindgen::JsCast;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_roundtrip() {
        for m in [Mode::Light, Mode::Dark, Mode::Auto] {
            assert_eq!(Mode::parse(m.id()), Some(m));
        }
        assert_eq!(Mode::parse("x"), None);
    }

    #[test]
    fn auto_follows_system() {
        assert_eq!(Mode::Auto.resolve(true), "dark");
        assert_eq!(Mode::Auto.resolve(false), "light");
        assert_eq!(Mode::Light.resolve(true), "light");
        assert_eq!(Mode::Dark.resolve(false), "dark");
    }

    #[test]
    fn cycle_covers_all_modes() {
        assert_eq!(Mode::Light.next().next().next(), Mode::Light);
    }

    #[test]
    fn only_available_themes_parse() {
        assert_eq!(Theme::parse("scandi"), Some(Theme::Scandi));
        assert_eq!(Theme::parse("material"), None);
        assert_eq!(Theme::parse("glass"), None);
        assert!(!Theme::Glass.is_available());
    }
}
