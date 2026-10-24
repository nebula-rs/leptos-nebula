//! Máscara do `Input` — porta pura (sem DOM) do `use-mask.js` do Quasar (R-17).
//!
//! Especificação: `docs/ui/forms.md#mascara`. O [`MaskState`] reproduz o `useMask`: o texto
//! exibido (`inner`), o texto sem máscara, o preenchimento (`fill_mask`/`reverse_fill_mask`) e o
//! cálculo da posição do cursor. O controle nativo entra pelo trait [`MaskControl`] (valor,
//! seleção e foco, em **caracteres**, não em unidades UTF-16): o componente implementa o trait
//! sobre o `<input>`; os testes, sobre um `FakeInput` que imita o jsdom.
//!
//! O JS usa expressões regulares para desmascarar. Aqui há um mini motor de *backtracking*
//! ([`Piece`]) com exatamente a semântica gulosa das regexes do Quasar, e as classes de caractere
//! são predicados (`Fn(char) -> bool`) em vez de `pattern`/`negate` em texto. O `negate` de um
//! token é sempre o complemento do seu `pattern`.
//!
//! O `nextTick` do Vue vira [`MaskState::flush_caret`]: [`MaskState::update_mask_value`] grava o
//! valor na hora e deixa o reposicionamento do cursor pendente; quem chama o executa no ciclo
//! seguinte (no componente, uma microtarefa).

// A porta é completa (tokens personalizados, `set_text_type`…); o `Input` ainda não usa tudo.
#![cfg_attr(not(test), allow(dead_code))]

use std::rc::Rc;

/// Máscaras nomeadas (`mask="phone"`…).
fn named_mask(name: &str) -> Option<&'static str> {
    Some(match name {
        "date" => "####/##/##",
        "datetime" => "####/##/## ##:##",
        "time" => "##:##",
        "fulltime" => "##:##:##",
        "phone" => "(###) ### - ####",
        "card" => "#### #### #### ####",
        _ => return None,
    })
}

/// Tipos de `input_type` (do `InputEvent`) cujo novo valor é o anterior com **uma** edição
/// contígua do usuário; os demais (colar, soltar, autofill, IME, valor programático) podem trazer
/// qualquer texto, inclusive já mascarado.
const EDIT_INPUT_TYPES: [&str; 6] = [
    "insertText",
    "deleteContentBackward",
    "deleteContentForward",
    "deleteWordBackward",
    "deleteWordForward",
    "deleteByCut",
];

/// Preenchimento do que falta para completar a máscara (`fill-mask` do Quasar: `Boolean | String`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillMask {
    /// Sem preenchimento (padrão).
    #[default]
    Off,
    /// Preenche com `_`.
    Default,
    /// Preenche com o caractere dado.
    Char(char),
}

impl FillMask {
    fn is_on(self) -> bool {
        self != FillMask::Off
    }

    fn fill_char(self) -> char {
        match self {
            FillMask::Char(c) => c,
            _ => '_',
        }
    }
}

impl From<bool> for FillMask {
    fn from(on: bool) -> Self {
        if on { FillMask::Default } else { FillMask::Off }
    }
}

impl From<char> for FillMask {
    fn from(c: char) -> Self {
        FillMask::Char(c)
    }
}

impl From<&str> for FillMask {
    /// Como o Quasar: usa o **primeiro** caractere; texto vazio equivale a `true` (`_`).
    fn from(s: &str) -> Self {
        s.chars().next().map_or(FillMask::Default, FillMask::Char)
    }
}

type Transform = Rc<dyn Fn(&str) -> String>;

/// Definição de um token da máscara: `pattern` (identifica o tipo, para decidir o que é separador)
/// e o teste de um caractere; opcionalmente uma transformação (`A`/`a`/`X`/`x`).
#[derive(Clone)]
pub struct MaskToken {
    pattern: Rc<str>,
    test: Rc<dyn Fn(char) -> bool>,
    transform: Option<Transform>,
}

impl MaskToken {
    /// `pattern` só identifica o tipo (dois tokens com o mesmo `pattern` aceitam os mesmos
    /// caracteres); `test` decide se um caractere cabe no token.
    pub fn new(pattern: &str, test: impl Fn(char) -> bool + 'static) -> Self {
        Self {
            pattern: Rc::from(pattern),
            test: Rc::new(test),
            transform: None,
        }
    }

    /// Transformação aplicada ao caractere aceito (ex.: caixa alta).
    pub fn transform(mut self, f: impl Fn(&str) -> String + 'static) -> Self {
        self.transform = Some(Rc::new(f));
        self
    }
}

/// Tokens personalizados (`mask-tokens`): somam-se aos padrões e podem sobrescrevê-los.
#[derive(Clone, Default)]
pub struct MaskTokens(Vec<(char, MaskToken)>);

impl MaskTokens {
    /// Sem tokens extras.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define (ou troca) o token `key`.
    pub fn with(mut self, key: char, token: MaskToken) -> Self {
        self.0.retain(|(k, _)| *k != key);
        self.0.push((key, token));
        self
    }
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}
fn is_alpha(c: char) -> bool {
    c.is_ascii_alphabetic()
}
fn is_alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn default_tokens() -> Vec<(char, MaskToken)> {
    let upper = |s: &str| s.to_uppercase();
    let lower = |s: &str| s.to_lowercase();
    vec![
        ('#', MaskToken::new("[\\d]", is_digit)),
        ('S', MaskToken::new("[a-zA-Z]", is_alpha)),
        ('N', MaskToken::new("[0-9a-zA-Z]", is_alnum)),
        ('A', MaskToken::new("[a-zA-Z]", is_alpha).transform(upper)),
        ('a', MaskToken::new("[a-zA-Z]", is_alpha).transform(lower)),
        (
            'X',
            MaskToken::new("[0-9a-zA-Z]", is_alnum).transform(upper),
        ),
        (
            'x',
            MaskToken::new("[0-9a-zA-Z]", is_alnum).transform(lower),
        ),
    ]
}

/// Um elemento da máscara já interpretada.
#[derive(Clone)]
enum MaskDef {
    Lit(char),
    Tok(MaskToken),
}

impl MaskDef {
    fn test(&self, c: char) -> bool {
        match self {
            MaskDef::Lit(_) => false,
            MaskDef::Tok(t) => (t.test)(c),
        }
    }
}

fn apply_transform(t: &MaskToken, c: char, out: &mut Vec<char>) {
    match &t.transform {
        Some(f) => out.extend(f(&c.to_string()).chars()),
        None => out.push(c),
    }
}

// ---------------------------------------------------------------------------------------------
// Mini motor de regex (só o que o `use-mask.js` precisa)
// ---------------------------------------------------------------------------------------------

type Cls = Rc<dyn Fn(char) -> bool>;

/// Pedaço de uma "regex" de sequência. Todos os grupos são opcionais e gulosos (`(X+)?`,
/// `(X)?`), com *backtracking* sobre o restante, como o motor do JS.
#[derive(Clone)]
enum Piece {
    /// `(X+)?` (capturando ou não).
    Run { cls: Cls, capture: bool },
    /// `(X)?`.
    One { cls: Cls, capture: bool },
    /// `X*` (não captura).
    Star { cls: Cls },
    /// `X?` (não captura).
    Opt { cls: Cls },
    /// `$`.
    End,
}

fn go(p: &[Piece], s: &[char], pos: usize, caps: &mut Vec<String>) -> Option<usize> {
    let Some(piece) = p.first() else {
        return Some(pos);
    };
    let rest = &p[1..];
    let run_len = |cls: &Cls| s[pos..].iter().take_while(|c| cls(**c)).count();
    match piece {
        Piece::Run { cls, capture } => {
            for k in (1..=run_len(cls)).rev() {
                let mark = caps.len();
                if *capture {
                    caps.push(s[pos..pos + k].iter().collect());
                }
                if let Some(e) = go(rest, s, pos + k, caps) {
                    return Some(e);
                }
                caps.truncate(mark);
            }
            go(rest, s, pos, caps)
        }
        Piece::One { cls, capture } => {
            if pos < s.len() && cls(s[pos]) {
                let mark = caps.len();
                if *capture {
                    caps.push(s[pos].to_string());
                }
                if let Some(e) = go(rest, s, pos + 1, caps) {
                    return Some(e);
                }
                caps.truncate(mark);
            }
            go(rest, s, pos, caps)
        }
        Piece::Opt { cls } => {
            if pos < s.len()
                && cls(s[pos])
                && let Some(e) = go(rest, s, pos + 1, caps)
            {
                return Some(e);
            }
            go(rest, s, pos, caps)
        }
        Piece::Star { cls } => {
            for k in (0..=run_len(cls)).rev() {
                if let Some(e) = go(rest, s, pos + k, caps) {
                    return Some(e);
                }
            }
            None
        }
        Piece::End => (pos == s.len()).then_some(pos),
    }
}

/// `regex.exec(s)` ancorada no início: o tamanho consumido e os grupos capturados (os que não
/// participaram não aparecem: no JS viram `undefined`, que o `join('')` ignora).
fn exec(pieces: &[Piece], s: &[char]) -> Option<(usize, Vec<String>)> {
    let mut caps = Vec::new();
    go(pieces, s, 0, &mut caps).map(|end| (end, caps))
}

fn cls(f: impl Fn(char) -> bool + 'static) -> Cls {
    Rc::new(f)
}

/// `.` do JS: qualquer caractere menos terminador de linha.
fn any_char(c: char) -> bool {
    !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Desmascaramento: a regex "grossa" que tira os separadores e a sequência de extração por token.
struct Unmask {
    unmask: Vec<Piece>,
    extract: Vec<Vec<Piece>>,
    mask_len: usize,
}

// ---------------------------------------------------------------------------------------------
// Controle nativo
// ---------------------------------------------------------------------------------------------

/// Direção da seleção (`selectionDirection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// `none`.
    None,
    /// `forward`.
    Forward,
    /// `backward`.
    Backward,
}

/// O que a máscara precisa do `<input>`: valor, seleção e foco. Posições em **caracteres**.
pub trait MaskControl {
    /// `input.value`.
    fn value(&self) -> String;
    /// `input.value = v` (o navegador leva o cursor para o fim se o valor mudou).
    fn set_value(&mut self, v: &str);
    /// `(selectionStart, selectionEnd, selectionDirection)`.
    fn selection(&self) -> (usize, usize, Direction);
    /// `setSelectionRange(start, end, direction)`; limita ao tamanho do valor.
    fn set_selection(&mut self, start: usize, end: usize, dir: Direction);
    /// `document.activeElement === input`.
    fn is_focused(&self) -> bool;
}

fn char_len(ctl: &dyn MaskControl) -> usize {
    ctl.value().chars().count()
}

/// `setSelectionRange` com a conversão `unsigned long` do WebIDL: negativo vira "muito grande"
/// (e é limitado ao fim).
fn set_sel(ctl: &mut dyn MaskControl, start: isize, end: isize, dir: Direction) {
    let len = char_len(ctl);
    let conv = |v: isize| if v < 0 { len } else { (v as usize).min(len) };
    ctl.set_selection(conv(start), conv(end), dir);
}

/// Tecla relevante para a máscara (o componente traduz o `KeyboardEvent`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskKey {
    /// Seta esquerda (37).
    Left,
    /// Seta direita (39).
    Right,
    /// Backspace (8).
    Backspace,
    /// Delete (46).
    Delete,
    /// Qualquer outra.
    Other,
}

/// O que o `onMaskedKeydown` precisa do evento.
#[derive(Clone, Copy, Debug)]
pub struct MaskKeyEvent {
    /// Tecla.
    pub key: MaskKey,
    /// Shift pressionado.
    pub shift: bool,
    /// Alt pressionado (o navegador cuida).
    pub alt: bool,
    /// Alguém já chamou `preventDefault`.
    pub default_prevented: bool,
    /// Composição de IME em curso (`shouldIgnoreKey`).
    pub composing: bool,
}

impl MaskKeyEvent {
    /// Evento de `key` sem modificadores.
    pub fn new(key: MaskKey) -> Self {
        Self {
            key,
            shift: false,
            alt: false,
            default_prevented: false,
            composing: false,
        }
    }
}

/// Resultado de uma operação que pode mudar o modelo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaskOutput {
    /// Novo valor para o modelo (`value`), se ele precisa mudar.
    pub emit: Option<String>,
    /// A emissão veio de `emitValue(val, true)`: o `watch` do modelo deve ignorar o eco.
    pub stop_watcher: bool,
    /// O texto exibido voltou a coincidir com o modelo: uma emissão adiada (debounce) ficou velha.
    pub cancel_pending: bool,
}

/// Entradas da máscara (as props do Quasar + o modelo atual).
#[derive(Clone, Default)]
pub struct MaskProps {
    /// `mask`.
    pub mask: Option<String>,
    /// `fill-mask`.
    pub fill_mask: FillMask,
    /// `reverse-fill-mask`.
    pub reverse_fill_mask: bool,
    /// `unmasked-value`.
    pub unmasked_value: bool,
    /// `mask-tokens`.
    pub tokens: MaskTokens,
    /// O tipo do controle aceita máscara (`text`, `search`, `url`, `tel`, `password`,
    /// `textarea` ou `autogrow`).
    pub text_type: bool,
    /// O modelo (`modelValue`).
    pub model: String,
}

/// Reposicionamento do cursor deixado para o "próximo ciclo".
#[derive(Clone, Debug)]
struct Pending {
    masked: Vec<char>,
    pre_masked_len: usize,
    end: usize,
    end_reverse: usize,
    renders_empty: bool,
    data_before_caret: Option<usize>,
    masked_data_len: usize,
    data_changed: bool,
    input_type: Option<String>,
}

/// Estado da máscara de um `Input` (≈ `useMask`).
pub struct MaskState {
    /// Entradas atuais.
    props: MaskProps,
    inner: String,
    has_mask: bool,
    computed_mask: Vec<MaskDef>,
    /// `true` = posição de token (`MARKER`), `false` = literal.
    mask_marked: Vec<bool>,
    mask_replaced: Vec<char>,
    unmask: Option<Unmask>,
    /// Tamanho do `inner` antes de o `fill_with_mask` preenchê-lo.
    inner_data_len: usize,
    pasted_text_start: usize,
    selection_anchor: Option<usize>,
    pending: Vec<Pending>,
}

/// `Array.prototype.slice(start)` com índice negativo contando do fim.
fn slice_from<T>(v: &[T], start: isize) -> &[T] {
    let len = v.len() as isize;
    let s = if start < 0 {
        (len + start).max(0)
    } else {
        start.min(len)
    };
    &v[s as usize..]
}

fn get(v: &[bool], i: isize) -> Option<bool> {
    (i >= 0).then(|| v.get(i as usize).copied()).flatten()
}

impl MaskState {
    /// Cria o estado e calcula o valor exibido inicial a partir do modelo (`getInitialMaskedValue`).
    pub fn new(props: MaskProps) -> Self {
        let mut st = Self {
            props,
            inner: String::new(),
            has_mask: false,
            computed_mask: Vec::new(),
            mask_marked: Vec::new(),
            mask_replaced: Vec::new(),
            unmask: None,
            inner_data_len: 0,
            pasted_text_start: 0,
            selection_anchor: None,
            pending: Vec::new(),
        };
        st.update_mask_internals();
        if st.has_mask {
            let model = st.props.model.clone();
            let masked = st.mask_value(&st.unmask_value(&model), false);
            st.inner_data_len = masked.chars().count();
            st.inner = if st.props.fill_mask.is_on() {
                st.fill_with_mask(&masked)
            } else {
                masked
            };
        } else {
            st.inner_data_len = 0;
            st.inner = st.props.model.clone();
        }
        st
    }

    /// Texto exibido (`innerValue`).
    pub fn inner_value(&self) -> &str {
        &self.inner
    }

    /// `reverse-fill-mask` atual.
    pub fn reverse_fill_mask(&self) -> bool {
        self.props.reverse_fill_mask
    }

    /// Há máscara ativa (`hasMask`).
    pub fn has_mask(&self) -> bool {
        self.has_mask
    }

    /// Define o valor exibido quando **não** há máscara (o `watch` do modelo faz
    /// `innerValue = v`).
    pub fn set_inner_plain(&mut self, v: &str) {
        self.inner = v.to_string();
    }

    /// Informa o modelo atual (o `modelValue` que o JS lê das props a cada chamada).
    pub fn set_model(&mut self, model: &str) {
        self.props.model = model.to_string();
    }

    /// `watch(() => type + autogrow)`: só recalcula o interno (não remascara).
    pub fn set_text_type(&mut self, text_type: bool) {
        self.props.text_type = text_type;
        self.update_mask_internals();
    }

    /// `watch(() => mask)`.
    pub fn set_mask(
        &mut self,
        ctl: Option<&mut dyn MaskControl>,
        mask: Option<String>,
    ) -> MaskOutput {
        self.props.mask = mask;
        if self.props.mask.is_some() {
            let inner = self.inner.clone();
            self.update_mask_value(ctl, &inner, true, None)
        } else {
            // `has_mask` ainda descreve o estado ANTIGO: com máscara, o `inner` pode trazer
            // preenchimento que não carrega dado.
            let src = if self.has_mask {
                self.strip_fill_padding(&self.inner, self.inner_data_len)
            } else {
                self.inner.clone()
            };
            let val = self.unmask_value(&src);
            self.update_mask_internals();
            let mut out = MaskOutput::default();
            if self.props.model != val {
                out.emit = Some(val);
            }
            out
        }
    }

    /// `watch(() => fillMask + reverseFillMask)`.
    pub fn set_fill(
        &mut self,
        ctl: Option<&mut dyn MaskControl>,
        fill_mask: FillMask,
        reverse: bool,
    ) -> MaskOutput {
        self.props.fill_mask = fill_mask;
        self.props.reverse_fill_mask = reverse;
        if self.has_mask {
            let inner = self.inner.clone();
            self.update_mask_value(ctl, &inner, true, None)
        } else {
            MaskOutput::default()
        }
    }

    /// `watch(() => unmaskedValue)`.
    pub fn set_unmasked_value(
        &mut self,
        ctl: Option<&mut dyn MaskControl>,
        unmasked: bool,
    ) -> MaskOutput {
        self.props.unmasked_value = unmasked;
        if self.has_mask {
            let inner = self.inner.clone();
            self.update_mask_value(ctl, &inner, false, None)
        } else {
            MaskOutput::default()
        }
    }

    /// `watch(() => maskTokens, { deep: true })`.
    pub fn set_tokens(
        &mut self,
        ctl: Option<&mut dyn MaskControl>,
        tokens: MaskTokens,
    ) -> MaskOutput {
        self.props.tokens = tokens;
        if self.has_mask {
            let inner = self.inner.clone();
            self.update_mask_value(ctl, &inner, true, None)
        } else {
            MaskOutput::default()
        }
    }

    fn token_map(&self) -> Vec<(char, MaskToken)> {
        let mut map = default_tokens();
        for (k, t) in &self.props.tokens.0 {
            map.retain(|(mk, _)| mk != k);
            map.push((*k, t.clone()));
        }
        map
    }

    fn update_mask_internals(&mut self) {
        let mask_prop = self.props.mask.clone();
        self.has_mask = mask_prop.as_ref().is_some_and(|m| !m.is_empty()) && self.props.text_type;

        if !self.has_mask {
            self.unmask = None;
            self.mask_marked.clear();
            self.mask_replaced.clear();
            return;
        }

        let mask_prop = mask_prop.unwrap_or_default();
        let local: Vec<char> = named_mask(&mask_prop)
            .unwrap_or(&mask_prop)
            .chars()
            .collect();
        let reverse = self.props.reverse_fill_mask;
        let fill_char = self.props.fill_mask.fill_char();
        let map = self.token_map();
        let lookup = |c: char| map.iter().find(|(k, _)| *k == c).map(|(_, t)| t.clone());

        // Specials do JS: a regex da máscara os trata como literais (e a barra invertida só
        // escapa o que não é especial).
        const SPECIAL: &str = ".*+?^${}()|[]";
        const NOT_ESCAPABLE: &str = ".*+?^${}()|[\\";

        let mut mask: Vec<MaskDef> = Vec::new();
        let mut unmask_chars: Vec<char> = Vec::new();
        let mut extract: Vec<(MaskToken, bool)> = Vec::new();
        let mut first_match = reverse;
        let mut unmask_char: Option<char> = None;
        let mut last_token: Option<MaskToken> = None;

        let mut i = 0;
        while i < local.len() {
            let c = local[i];
            let literal =
                if c == '\\' && i + 1 < local.len() && !NOT_ESCAPABLE.contains(local[i + 1]) {
                    i += 2;
                    Some(local[i - 1])
                } else if SPECIAL.contains(c) {
                    i += 1;
                    Some(c)
                } else if let Some(t) = lookup(c) {
                    i += 1;
                    mask.push(MaskDef::Tok(t.clone()));
                    last_token = Some(t.clone());
                    if first_match {
                        extract.push((t.clone(), true));
                        first_match = false;
                    }
                    extract.push((t, false));
                    None
                } else {
                    i += 1;
                    Some(c)
                };
            if let Some(l) = literal {
                unmask_char = Some(l);
                mask.push(MaskDef::Lit(l));
                unmask_chars.push(l);
            }
        }

        // Tipos de token distintos (por `pattern`) e o que conta como separador: um caractere que
        // nenhum token da máscara aceita.
        let mut kinds: Vec<MaskToken> = Vec::new();
        for d in &mask {
            if let MaskDef::Tok(t) = d
                && !kinds.iter().any(|k| k.pattern == t.pattern)
            {
                kinds.push(t.clone());
            }
        }
        let separator: Cls = match (kinds.len(), &last_token) {
            (1, Some(t)) => {
                let test = t.test.clone();
                cls(move |c| !test(c))
            }
            _ => {
                let tests: Vec<Cls> = kinds.iter().map(|k| k.test.clone()).collect();
                cls(move |c| !tests.iter().any(|t| t(c)))
            }
        };
        let negate_last: Cls = match &last_token {
            Some(t) => {
                let test = t.test.clone();
                cls(move |c| !test(c))
            }
            None => cls(any_char),
        };
        let fill_cls: Cls = cls(move |c| c == fill_char);
        let sep_opt = |s: &Cls| Piece::Run {
            cls: s.clone(),
            capture: false,
        };

        // unmask: `^ ([^c]+)?c? ... ([^u]+)? [u]* $` (sem literais: `^(.+)?$`)
        let mut unmask_seq: Vec<Piece> = Vec::new();
        for lit in &unmask_chars {
            let l = *lit;
            unmask_seq.push(Piece::Run {
                cls: cls(move |c| c != l),
                capture: true,
            });
            unmask_seq.push(Piece::Opt {
                cls: cls(move |c| c == l),
            });
        }
        match unmask_char {
            None => unmask_seq.push(Piece::Run {
                cls: cls(any_char),
                capture: true,
            }),
            Some(u) => {
                unmask_seq.push(Piece::Run {
                    cls: cls(move |c| c != u),
                    capture: true,
                });
                unmask_seq.push(Piece::Star {
                    cls: cls(move |c| c == u),
                });
            }
        }
        unmask_seq.push(Piece::End);

        let extract_last = extract.len().saturating_sub(1);
        let extract_seqs: Vec<Vec<Piece>> = extract
            .iter()
            .enumerate()
            .map(|(index, (tok, overflow))| {
                let pat = tok.test.clone();
                let mut re: Vec<Piece> = Vec::new();
                if *overflow {
                    re.push(sep_opt(&separator));
                    re.push(Piece::Run {
                        cls: pat.clone(),
                        capture: true,
                    });
                    re.push(sep_opt(&separator));
                    re.push(Piece::Run {
                        cls: pat,
                        capture: true,
                    });
                } else {
                    re.push(sep_opt(&separator));
                    re.push(Piece::One {
                        cls: pat,
                        capture: true,
                    });
                }
                if index == 0 && reverse {
                    let mut seq = vec![Piece::Star {
                        cls: fill_cls.clone(),
                    }];
                    seq.extend(re);
                    seq
                } else if index == extract_last {
                    re.push(Piece::Run {
                        cls: negate_last.clone(),
                        capture: true,
                    });
                    re.push(if reverse {
                        Piece::End
                    } else {
                        Piece::Star {
                            cls: fill_cls.clone(),
                        }
                    });
                    re
                } else {
                    re
                }
            })
            .collect();

        self.mask_marked = mask.iter().map(|d| matches!(d, MaskDef::Tok(_))).collect();
        self.mask_replaced = mask
            .iter()
            .map(|d| match d {
                MaskDef::Lit(c) => *c,
                MaskDef::Tok(_) => fill_char,
            })
            .collect();
        self.unmask = Some(Unmask {
            unmask: unmask_seq,
            extract: extract_seqs,
            mask_len: mask.len(),
        });
        self.computed_mask = mask;
    }

    /// `computedUnmask`.
    fn computed_unmask(&self, val: &str) -> String {
        let Some(u) = &self.unmask else {
            return val.to_string();
        };
        let mut chars: Vec<char> = val.chars().collect();
        if !self.props.reverse_fill_mask {
            chars.truncate(u.mask_len + 1);
        }
        let val: Vec<char> = match exec(&u.unmask, &chars) {
            Some((_, caps)) => caps.concat().chars().collect(),
            None => val.chars().collect(),
        };
        let mut extract_match = String::new();
        let mut any_matched = false;
        let mut rest: &[char] = &val;
        for seq in &u.extract {
            let Some((len, caps)) = exec(seq, rest) else {
                break;
            };
            rest = &rest[len..];
            any_matched = true;
            for c in caps {
                extract_match.push_str(&c);
            }
        }
        // `extractMatch.length !== 0` no JS: todo matcher tem ao menos um grupo (mesmo
        // `undefined`), então basta que um tenha casado.
        if any_matched {
            return extract_match;
        }
        val.into_iter().collect()
    }

    fn unmask_value(&self, val: &str) -> String {
        if self.unmask.is_none() {
            val.to_string()
        } else {
            self.computed_unmask(val)
        }
    }

    // --- valores -----------------------------------------------------------------------------

    fn mask_value(&self, val: &str, flag: bool) -> String {
        if val.is_empty() {
            return String::new();
        }
        let v: Vec<char> = val.chars().collect();
        let out = if self.props.reverse_fill_mask {
            self.mask_value_reverse(&v, flag)
        } else {
            self.mask_value_forward(&v, flag)
        };
        out.into_iter().collect()
    }

    fn mask_value_forward(&self, val: &[char], flag: bool) -> Vec<char> {
        let mut val_index = 0;
        let mut output = Vec::new();
        for def in &self.computed_mask {
            let val_char = val.get(val_index).copied();
            match def {
                MaskDef::Lit(l) => {
                    output.push(*l);
                    if flag && val_char == Some(*l) {
                        val_index += 1;
                    }
                }
                MaskDef::Tok(t) => match val_char {
                    Some(c) if (t.test)(c) => {
                        apply_transform(t, c, &mut output);
                        val_index += 1;
                    }
                    _ => return output,
                },
            }
        }
        output
    }

    fn mask_value_reverse(&self, val: &[char], flag: bool) -> Vec<char> {
        let first_token_index = self.mask_marked.iter().position(|m| *m);
        let mut val_index = val.len() as isize - 1;
        let mut output: Vec<char> = Vec::new();
        // Literais encontrados indo para a esquerda ficam retidos até um dado cair à esquerda
        // deles: emiti-los de imediato deixaria o separador de um slot vazio pendurado na frente.
        let mut pending: Vec<char> = Vec::new();
        let at = |i: isize| (i >= 0).then(|| val.get(i as usize).copied()).flatten();

        let mut mask_index = self.computed_mask.len() as isize - 1;
        while mask_index >= 0 && val_index != -1 {
            let def = &self.computed_mask[mask_index as usize];
            let mut val_char = at(val_index);
            match def {
                MaskDef::Lit(l) => {
                    pending.insert(0, *l);
                    if flag && val_char == Some(*l) {
                        val_index -= 1;
                    }
                }
                MaskDef::Tok(t) if val_char.is_some_and(|c| (t.test)(c)) => {
                    let mut head = std::mem::take(&mut pending);
                    head.extend(output);
                    output = head;
                    loop {
                        let mut piece = Vec::new();
                        apply_transform(t, val_char.unwrap_or_default(), &mut piece);
                        piece.extend(output);
                        output = piece;
                        val_index -= 1;
                        val_char = at(val_index);
                        let again = first_token_index == Some(mask_index as usize)
                            && val_char.is_some_and(|c| (t.test)(c));
                        if !again {
                            break;
                        }
                    }
                }
                MaskDef::Tok(_) => return output,
            }
            mask_index -= 1;
        }
        output
    }

    /// Desmascara o resultado de UMA edição contígua comparando com o valor exibido anterior.
    fn unmask_edit_value(
        &self,
        prev: &str,
        prev_data_len: usize,
        val: &str,
        input_type: Option<&str>,
    ) -> String {
        let prev: Vec<char> = prev.chars().collect();
        let val: Vec<char> = val.chars().collect();
        let (prev_len, val_len) = (prev.len(), val.len());
        let min_len = prev_len.min(val_len);

        let mut start = 0;
        while start < min_len && prev[start] == val[start] {
            start += 1;
        }
        let mut end = 0;
        while end < min_len - start && prev[prev_len - 1 - end] == val[val_len - 1 - end] {
            end += 1;
        }

        let data_at = self.data_flags(&prev, prev_data_len);
        let mut before: Vec<char> = (0..start)
            .filter(|i| data_at[*i])
            .map(|i| prev[i])
            .collect();
        let mut after: Vec<char> = (prev_len - end..prev_len)
            .filter(|i| data_at[*i])
            .map(|i| prev[i])
            .collect();

        let raw_inserted = &val[start..val_len - end];
        let mut inserted: Vec<char> = Vec::new();
        if !raw_inserted.is_empty() {
            if self.props.reverse_fill_mask {
                // Conteúdo alinhado à direita não tem slot fixo: fica o que algum token aceita.
                for c in raw_inserted {
                    if self.computed_mask.iter().any(|d| d.test(*c)) {
                        inserted.push(*c);
                    }
                }
            } else {
                // Os dados preenchem os slots em ordem: a inserção continua no slot seguinte ao
                // dado que a precede.
                let token_defs: Vec<&MaskDef> = self
                    .computed_mask
                    .iter()
                    .filter(|d| matches!(d, MaskDef::Tok(_)))
                    .collect();
                let mut slot = before.len();
                for c in raw_inserted {
                    if token_defs.get(slot).is_some_and(|d| d.test(*c)) {
                        inserted.push(*c);
                        slot += 1;
                    }
                }
            }
        } else if !self.props.reverse_fill_mask && start + end < prev_len {
            // Remoção pura. Se o trecho removido só tinha literais, remascarar os restauraria e a
            // edição viraria um no-op com o cursor à deriva; teclados virtuais (iOS) não passam
            // pelo keydown que pré-seleciona os literais (#17639): remove-se o dado vizinho.
            let deleted_data = (start..prev_len - end).any(|i| data_at[i]);
            if !deleted_data {
                if input_type == Some("deleteContentForward") {
                    if !after.is_empty() {
                        after.remove(0);
                    }
                } else {
                    before.pop();
                }
            }
        }

        before.into_iter().chain(inserted).chain(after).collect()
    }

    /// `true` nas posições de `s` que guardam dado (não literal, aceito pelo token e fora do
    /// preenchimento). O preenchimento só se reconhece pela posição (`data_len`): um caractere de
    /// preenchimento pode passar no próprio token (`fill_mask='0'` com `#`) (#18523).
    fn data_flags(&self, s: &[char], data_len: usize) -> Vec<bool> {
        let str_len = s.len();
        let reverse = self.props.reverse_fill_mask;
        let marked: Vec<bool> = if reverse {
            self.padded_mask_marked(str_len)
        } else {
            self.mask_marked.clone()
        };
        let def_offset = if reverse {
            self.computed_mask.len() as isize - str_len as isize
        } else {
            0
        };
        let fill_from = if reverse { 0 } else { data_len.min(str_len) } as isize;
        let fill_to = if reverse {
            str_len as isize - data_len as isize
        } else {
            str_len as isize
        };
        (0..str_len)
            .map(|i| {
                let ii = i as isize;
                if marked.get(i) != Some(&true) || (ii >= fill_from && ii < fill_to) {
                    return false;
                }
                let di = def_offset + ii;
                match (di >= 0)
                    .then(|| self.computed_mask.get(di as usize))
                    .flatten()
                {
                    None | Some(MaskDef::Lit(_)) => true,
                    Some(d) => d.test(s[i]),
                }
            })
            .collect()
    }

    fn count_data_chars(&self, s: &[char], position: usize, data_len: usize) -> usize {
        self.data_flags(s, data_len)
            .iter()
            .take(position)
            .filter(|b| **b)
            .count()
    }

    fn padded_mask_marked(&self, size: usize) -> Vec<bool> {
        let mm = &self.mask_marked;
        if size < mm.len() {
            // `slice(-size)`; com `size == 0` o JS devolve tudo (`slice(-0)`).
            return if size == 0 {
                mm.clone()
            } else {
                mm[mm.len() - size..].to_vec()
            };
        }
        let Some(pad_pos) = mm.iter().position(|m| *m) else {
            return mm.clone();
        };
        let mut out = mm[..pad_pos].to_vec();
        out.extend(std::iter::repeat_n(true, size - mm.len()));
        out.extend_from_slice(&mm[pad_pos..]);
        out
    }

    fn fill_with_mask(&self, val: &str) -> String {
        let v: Vec<char> = val.chars().collect();
        let rep = &self.mask_replaced;
        if rep.len() as isize - v.len() as isize <= 0 {
            return val.to_string();
        }
        if self.props.reverse_fill_mask && !v.is_empty() {
            rep[..rep.len() - v.len()].iter().chain(v.iter()).collect()
        } else {
            v.iter().chain(rep[v.len()..].iter()).collect()
        }
    }

    /// Desfaz o `fill_with_mask`: guarda só os `data_len` caracteres que o valor tinha antes.
    fn strip_fill_padding(&self, s: &str, data_len: usize) -> String {
        let v: Vec<char> = s.chars().collect();
        if data_len >= v.len() {
            return s.to_string();
        }
        if self.props.reverse_fill_mask {
            v[v.len() - data_len..].iter().collect()
        } else {
            v[..data_len].iter().collect()
        }
    }

    // --- atualização -------------------------------------------------------------------------

    /// `updateMaskValue(rawVal, updateMaskInternalsFlag, inputType)`: remascara `raw`, grava o
    /// texto no controle **na hora** (sem "piscar") e deixa o cursor para [`Self::flush_caret`].
    /// Devolve o que mudou no modelo.
    pub fn update_mask_value(
        &mut self,
        mut ctl: Option<&mut dyn MaskControl>,
        raw: &str,
        update_internals: bool,
        input_type: Option<&str>,
    ) -> MaskOutput {
        let end = ctl.as_deref().map_or(0, |c| c.selection().1);
        let end_reverse = ctl
            .as_deref()
            .map_or(0, |c| char_len(c).saturating_sub(end));
        let is_edit = input_type.is_some_and(|t| EDIT_INPUT_TYPES.contains(&t));
        let unmasked = if !update_internals && is_edit {
            let prev = self.inner.clone();
            self.unmask_edit_value(&prev, self.inner_data_len, raw, input_type)
        } else {
            self.unmask_value(raw)
        };

        // Uma reconstrução dos internos (mask/fill mudaram) pode deslocar o layout, então o cursor
        // não pode manter o deslocamento bruto: guarda-se quantos dados havia antes dele no layout
        // ANTIGO (#7777).
        let mut data_before_caret = None;
        if update_internals
            && !self.mask_marked.is_empty()
            && let Some(c) = ctl.as_deref()
        {
            let v: Vec<char> = c.value().chars().collect();
            data_before_caret = Some(self.count_data_chars(&v, end, self.inner_data_len));
        }

        if update_internals {
            self.update_mask_internals();
        }

        let pre_masked = self.mask_value(&unmasked, update_internals);
        let masked = if self.props.fill_mask.is_on() {
            self.fill_with_mask(&pre_masked)
        } else {
            pre_masked.clone()
        };
        let masked_data_len = pre_masked.chars().count();
        let changed = self.inner != masked;
        // O cursor tem de seguir o DADO: `changed` só diz que a string mudou, e um caractere de
        // preenchimento que também é dado válido deixa o render idêntico enquanto o dado cresce
        // (#18523).
        let data_changed = changed || masked_data_len != self.inner_data_len;
        self.inner_data_len = masked_data_len;
        let renders_empty = self.props.fill_mask.is_on() && masked_data_len == 0;

        if let Some(c) = ctl.as_deref_mut()
            && c.value() != masked
        {
            c.set_value(&masked);
        }
        if changed {
            self.inner = masked.clone();
        }

        if ctl.as_deref().is_some_and(|c| c.is_focused()) {
            self.pending.push(Pending {
                masked: masked.chars().collect(),
                pre_masked_len: masked_data_len,
                end,
                end_reverse,
                renders_empty,
                data_before_caret,
                masked_data_len,
                data_changed,
                input_type: input_type.map(str::to_string),
            });
        }

        // Desmascara ANTES do preenchimento: o preenchimento não carrega dado (#18523).
        let val = if self.props.unmasked_value {
            self.unmask_value(&pre_masked)
        } else {
            masked
        };

        if self.props.model != val {
            MaskOutput {
                emit: Some(val),
                stop_watcher: true,
                cancel_pending: false,
            }
        } else {
            MaskOutput {
                cancel_pending: true,
                ..Default::default()
            }
        }
    }

    /// O "próximo ciclo" (`nextTick`) de [`Self::update_mask_value`]: posiciona o cursor.
    pub fn flush_caret(&mut self, ctl: Option<&mut dyn MaskControl>) {
        let pending = std::mem::take(&mut self.pending);
        let Some(c) = ctl else { return };
        for p in pending {
            self.place_caret(c, &p);
        }
    }

    fn place_caret(&self, inp: &mut dyn MaskControl, p: &Pending) {
        let reverse = self.props.reverse_fill_mask;
        let masked_len = p.masked.len() as isize;
        let pre_len = p.pre_masked_len as isize;
        let (end, end_reverse) = (p.end as isize, p.end_reverse as isize);
        let fwd = Direction::Forward;

        if p.renders_empty {
            let cursor = if reverse {
                self.mask_replaced.len() as isize
            } else {
                0
            };
            set_sel(inp, cursor, cursor, fwd);
            return;
        }

        if let Some(before) = p.data_before_caret {
            // Reancora o cursor depois do mesmo número de dados que tinha antes da reconstrução.
            let mut cursor: usize = 0;
            let mut found = 0;
            while cursor < p.masked.len() && found < before {
                found = self.count_data_chars(&p.masked, cursor + 1, p.masked_data_len);
                cursor += 1;
            }
            set_sel(inp, cursor as isize, cursor as isize, fwd);
            return;
        }

        let input_type = p.input_type.as_deref();
        if input_type == Some("insertFromPaste") && !reverse {
            let max_end = inp.selection().1 as isize;
            let mut cursor = end - 1;
            let mut i = self.pasted_text_start as isize;
            while i <= cursor && i < max_end {
                if get(&self.mask_marked, i) != Some(true) {
                    cursor += 1;
                }
                i += 1;
            }
            self.move_right(inp, cursor);
            return;
        }

        if matches!(
            input_type,
            Some("deleteContentBackward" | "deleteContentForward")
        ) {
            let cursor = if reverse {
                if end == 0 {
                    if masked_len > pre_len { 1 } else { 0 }
                } else {
                    let sub = if p.renders_empty {
                        0
                    } else {
                        pre_len.min(end_reverse) + 1
                    };
                    (masked_len - sub).max(0) + 1
                }
            } else {
                end
            };
            set_sel(inp, cursor, cursor, fwd);
            return;
        }

        let first_marker = self
            .mask_marked
            .iter()
            .position(|m| *m)
            .map_or(-1, |i| i as isize);
        if reverse {
            if p.data_changed {
                let sub = if p.renders_empty {
                    0
                } else {
                    pre_len.min(end_reverse + 1)
                };
                let cursor = (masked_len - sub).max(0);
                if cursor == 1 && end == 1 {
                    set_sel(inp, cursor, cursor, fwd);
                } else {
                    self.move_right_reverse(inp, cursor);
                }
            } else {
                let cursor = masked_len - end_reverse;
                set_sel(inp, cursor, cursor, Direction::Backward);
            }
        } else if p.data_changed {
            let cursor = 0.max(first_marker).max(pre_len.min(end) - 1);
            self.move_right(inp, cursor);
        } else {
            self.move_right(inp, end - 1);
        }
    }

    // --- cursor ------------------------------------------------------------------------------

    fn move_left(&self, inp: &mut dyn MaskControl, mut cursor: isize) {
        let mm = &self.mask_marked;
        let no_mark_before = !slice_from(mm, cursor - 1).contains(&true);
        let mut i = (cursor - 1).max(0);
        while i >= 0 {
            if get(mm, i) == Some(true) {
                cursor = i;
                if no_mark_before {
                    cursor += 1;
                }
                break;
            }
            i -= 1;
        }
        if i < 0 && get(mm, cursor) == Some(false) {
            return self.move_right(inp, 0);
        }
        if cursor >= 0 {
            set_sel(inp, cursor, cursor, Direction::Backward);
        }
    }

    fn move_right(&self, inp: &mut dyn MaskControl, mut cursor: isize) {
        let mm = &self.mask_marked;
        let limit = char_len(inp) as isize;
        let mut i = limit.min(cursor + 1);
        while i <= limit {
            if get(mm, i) == Some(true) {
                cursor = i;
                break;
            } else if get(mm, i - 1) == Some(true) {
                cursor = i;
            }
            i += 1;
        }
        if i > limit && get(mm, cursor - 1) == Some(false) {
            return self.move_left(inp, limit);
        }
        set_sel(inp, cursor, cursor, Direction::Forward);
    }

    fn move_left_reverse(&self, inp: &mut dyn MaskControl, mut cursor: isize) {
        let mm = self.padded_mask_marked(char_len(inp));
        let mut i = (cursor - 1).max(0);
        while i >= 0 {
            if get(&mm, i - 1) == Some(true) {
                cursor = i;
                break;
            } else if get(&mm, i) == Some(true) {
                cursor = i;
                if i == 0 {
                    break;
                }
            }
            i -= 1;
        }
        if i < 0 && get(&mm, cursor) == Some(false) {
            return self.move_right_reverse(inp, 0);
        }
        if cursor >= 0 {
            set_sel(inp, cursor, cursor, Direction::Backward);
        }
    }

    fn move_right_reverse(&self, inp: &mut dyn MaskControl, mut cursor: isize) {
        let limit = char_len(inp) as isize;
        let mm = self.padded_mask_marked(limit as usize);
        let upto = ((cursor + 1).max(0) as usize).min(mm.len());
        let no_mark_before = !mm[..upto].contains(&true);
        let mut i = limit.min(cursor + 1);
        while i <= limit {
            if get(&mm, i - 1) == Some(true) {
                cursor = i;
                if cursor > 0 && no_mark_before {
                    cursor -= 1;
                }
                break;
            }
            i += 1;
        }
        if i > limit && get(&mm, cursor - 1) == Some(false) {
            return self.move_left_reverse(inp, limit);
        }
        set_sel(inp, cursor, cursor, Direction::Forward);
    }

    /// `onMaskedClick` (sem o `emit('click')`, do componente): zera a âncora da seleção.
    pub fn on_masked_click(&mut self) {
        self.selection_anchor = None;
    }

    /// `onMaskedKeydown` (sem o `emit('keydown')`): devolve `true` se o evento deve receber
    /// `preventDefault`.
    pub fn on_masked_keydown(&mut self, inp: &mut dyn MaskControl, e: &MaskKeyEvent) -> bool {
        if e.default_prevented || e.alt || e.composing {
            return false;
        }
        let (start, end, dir) = inp.selection();
        let (start, end) = (start as isize, end as isize);
        if !e.shift {
            self.selection_anchor = None;
        }
        let reverse = self.props.reverse_fill_mask;
        match e.key {
            MaskKey::Left | MaskKey::Right => {
                if e.shift && self.selection_anchor.is_none() {
                    self.selection_anchor = Some(if dir == Direction::Forward {
                        start
                    } else {
                        end
                    } as usize);
                }
                let anchor = self.selection_anchor.map(|a| a as isize);
                let from = if anchor == Some(start) { end } else { start };
                match (e.key, reverse) {
                    (MaskKey::Right, false) => self.move_right(inp, from),
                    (MaskKey::Left, false) => self.move_left(inp, from),
                    (MaskKey::Right, true) => self.move_right_reverse(inp, from),
                    _ => self.move_left_reverse(inp, from),
                }
                if e.shift {
                    let cursor = inp.selection().0;
                    let a = anchor.unwrap_or(0) as usize;
                    inp.set_selection(a.min(cursor), a.max(cursor), Direction::Forward);
                }
                true
            }
            MaskKey::Backspace if !reverse && start == end => {
                self.move_left(inp, start);
                let new_start = inp.selection().0 as isize;
                set_sel(inp, new_start, end, Direction::Backward);
                false
            }
            MaskKey::Delete if reverse && start == end => {
                self.move_right_reverse(inp, end);
                let new_end = inp.selection().1 as isize;
                set_sel(inp, start, new_end, Direction::Forward);
                false
            }
            _ => false,
        }
    }

    /// `moveCursorForPaste`: ancora a colagem no primeiro slot livre.
    pub fn move_cursor_for_paste(&mut self, inp: &mut dyn MaskControl, start: usize, end: usize) {
        let pre_masked = self.mask_value(&self.unmask_value(&inp.value()), false);
        let first_marker = self.mask_marked.iter().position(|m| *m).unwrap_or(0);
        let start = first_marker.max(pre_masked.chars().count().min(start));
        self.pasted_text_start = start;
        inp.set_selection(start, end, Direction::Forward);
    }
}

/// O tipo de controle aceita máscara (`getIsTypeText`): texto/senha/busca/url/tel/textarea, ou
/// `autogrow` (que vira textarea).
pub fn type_allows_mask(type_: super::InputType, autogrow: bool) -> bool {
    use super::InputType as T;
    autogrow
        || matches!(
            type_,
            T::Text | T::Password | T::Search | T::Url | T::Tel | T::Textarea
        )
}

// ---------------------------------------------------------------------------------------------
// Posições UTF-16 <-> caracteres (o DOM conta em unidades UTF-16)
// ---------------------------------------------------------------------------------------------

/// Posição UTF-16 -> índice de caractere (arredonda para dentro de um par substituto).
pub fn utf16_to_char(s: &str, offset: usize) -> usize {
    let mut units = 0;
    for (i, c) in s.chars().enumerate() {
        if units >= offset {
            return i;
        }
        units += c.len_utf16();
    }
    s.chars().count()
}

/// Índice de caractere -> posição UTF-16.
pub fn char_to_utf16(s: &str, index: usize) -> usize {
    s.chars().take(index).map(char::len_utf16).sum()
}

#[cfg(test)]
mod tests;
