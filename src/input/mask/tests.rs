//! Porta de `ui/src/components/input/use-mask.test.js` do Quasar. O `FakeInput` imita o jsdom:
//! `value =` leva a seleção para o fim quando o valor muda, `setSelectionRange` limita ao
//! tamanho, `focus()` não mexe na seleção. O `nextTick` do Vue é `ctx.tick()`.
//!
//! Não portados (ver as Notas da R-17): `useMaskProps`/forma do retorno (API do Vue), o `emit` de
//! `click`/`keydown` (o componente chama `on_keydown`), o modelo `null` (o `value` é `String`) e o
//! `maskTokens` com `pattern`/`negate` em texto (aqui são predicados).

use super::*;
use crate::input::InputType;

// --- infraestrutura -------------------------------------------------------------------------

struct FakeInput {
    value: String,
    start: usize,
    end: usize,
    dir: Direction,
    focused: bool,
}

impl FakeInput {
    fn new() -> Self {
        Self {
            value: String::new(),
            start: 0,
            end: 0,
            dir: Direction::None,
            focused: false,
        }
    }

    fn set_range(&mut self, start: usize, end: usize) {
        self.set_selection(start, end, Direction::None);
    }
}

impl MaskControl for FakeInput {
    fn value(&self) -> String {
        self.value.clone()
    }

    fn set_value(&mut self, v: &str) {
        if self.value != v {
            self.value = v.to_string();
            let n = v.chars().count();
            self.start = n;
            self.end = n;
            self.dir = Direction::None;
        }
    }

    fn selection(&self) -> (usize, usize, Direction) {
        (self.start, self.end, self.dir)
    }

    fn set_selection(&mut self, start: usize, end: usize, dir: Direction) {
        let n = self.value.chars().count();
        self.end = end.min(n);
        self.start = start.min(self.end);
        self.dir = dir;
    }

    fn is_focused(&self) -> bool {
        self.focused
    }
}

struct Ctx {
    st: MaskState,
    input: FakeInput,
    /// Chamadas de `emitValue` (o que `updateMaskValue` devolve).
    emit_value: Vec<String>,
    /// Chamadas de `emit('update:modelValue')` dos watchers.
    emit_model: Vec<String>,
}

fn create_mask(f: impl FnOnce(&mut MaskProps)) -> Ctx {
    let mut props = MaskProps {
        text_type: true,
        ..Default::default()
    };
    f(&mut props);
    let st = MaskState::new(props);
    let mut input = FakeInput::new();
    input.set_value(st.inner_value());
    Ctx {
        st,
        input,
        emit_value: Vec::new(),
        emit_model: Vec::new(),
    }
}

fn with_mask(model: &str, mask: &str) -> Ctx {
    create_mask(|p| {
        p.model = model.into();
        p.mask = Some(mask.into());
    })
}

impl Ctx {
    fn inner(&self) -> String {
        self.st.inner_value().to_string()
    }

    fn update(&mut self, raw: &str, flag: bool, input_type: Option<&str>) {
        let out = self
            .st
            .update_mask_value(Some(&mut self.input), raw, flag, input_type);
        if let Some(v) = out.emit {
            self.emit_value.push(v);
        }
    }

    fn tick(&mut self) {
        self.st.flush_caret(Some(&mut self.input));
    }

    fn emit_model_from(&mut self, out: MaskOutput) {
        if let Some(v) = out.emit {
            self.emit_model.push(v);
        }
    }

    fn set_mask(&mut self, mask: Option<&str>) {
        let out = self
            .st
            .set_mask(Some(&mut self.input), mask.map(str::to_string));
        self.emit_model_from(out);
        self.tick();
    }

    fn set_fill(&mut self, fill: FillMask, reverse: bool) {
        let out = self.st.set_fill(Some(&mut self.input), fill, reverse);
        self.emit_model_from(out);
        self.tick();
    }

    /// `syncModel`: fecha o laço do `v-model`.
    fn sync_model(&mut self) {
        if let Some(last) = self.emit_value.last().cloned() {
            self.st.set_model(&last);
        }
    }

    fn sel_start(&self) -> usize {
        self.input.start
    }

    fn type_char(&mut self, c: char) {
        self.edit(Edit::Type(c));
    }

    fn edit(&mut self, e: Edit) {
        match e {
            Edit::Type(c) => {
                let (start, end) = (self.input.start, self.input.end);
                let chars: Vec<char> = self.input.value.chars().collect();
                let mut val: String = chars[..start].iter().collect();
                val.push(c);
                val.extend(&chars[end..]);
                self.input.set_value(&val);
                self.input.set_range(start + 1, start + 1);
                self.update(&val, false, Some("insertText"));
            }
            Edit::Bksp => {
                let ev = MaskKeyEvent::new(MaskKey::Backspace);
                self.st.on_masked_keydown(&mut self.input, &ev);
                let end = self.input.end;
                let mut start = self.input.start;
                if start == end {
                    if start == 0 {
                        return;
                    }
                    start -= 1;
                }
                let chars: Vec<char> = self.input.value.chars().collect();
                let mut val: String = chars[..start].iter().collect();
                val.extend(&chars[end..]);
                self.input.set_value(&val);
                self.input.set_range(start, start);
                self.update(&val, false, Some("deleteContentBackward"));
            }
        }
        self.tick();
        self.tick();
    }

    /// `removeRange`: corte e apagamentos de palavra.
    fn remove_range(&mut self, from: usize, to: usize, input_type: &str) {
        let chars: Vec<char> = self.input.value.chars().collect();
        let mut val: String = chars[..from].iter().collect();
        val.extend(&chars[to..]);
        self.input.set_value(&val);
        self.input.set_range(from, from);
        self.update(&val, false, Some(input_type));
        self.sync_model();
    }
}

#[derive(Clone, Copy)]
enum Edit {
    Type(char),
    Bksp,
}

// --- has_mask / tipos -----------------------------------------------------------------------

#[test]
fn starts_empty_without_mask() {
    let ctx = create_mask(|_| {});
    assert!(!ctx.st.has_mask());
    assert_eq!(ctx.inner(), "");
}

#[test]
fn masks_a_field_by_type() {
    use InputType as T;
    // (tipo, autogrow, esperado) — a linha "an autogrowing field" é o `number` com autogrow
    let cases = [
        (T::Text, false, true),
        (T::Textarea, false, true),
        (T::Search, false, true),
        (T::Url, false, true),
        (T::Tel, false, true),
        (T::Password, false, true),
        (T::Number, false, false),
        (T::Number, true, true),
    ];
    for (t, autogrow, expected) in cases {
        let ctx = create_mask(|p| {
            p.mask = Some("###".into());
            p.text_type = type_allows_mask(t, autogrow);
        });
        assert_eq!(ctx.st.has_mask(), expected, "{t:?} autogrow={autogrow}");
    }
}

#[test]
fn other_types_do_not_allow_a_mask() {
    for t in [
        InputType::Email,
        InputType::Date,
        InputType::Time,
        InputType::DatetimeLocal,
    ] {
        assert!(!type_allows_mask(t, false), "{t:?}");
    }
}

#[test]
fn stays_out_of_the_way_with_no_mask_or_an_empty_one() {
    for mask in [None, Some("")] {
        let ctx = create_mask(|p| {
            p.model = "abc".into();
            p.mask = mask.map(str::to_string);
        });
        assert!(!ctx.st.has_mask());
        assert_eq!(ctx.inner(), "abc");
    }
}

#[test]
fn masks_the_initial_model_value() {
    assert_eq!(with_mask("12345", "###-##").inner(), "123-45");
}

#[test]
fn keeps_only_the_accepted_chars() {
    let cases = [
        ("digits", "###", "12a3", "123"),
        ("letters", "SSS", "ab1c", "abc"),
        ("alphanumerics", "NNN", "a1-b", "a1b"),
        ("letters, uppercased", "AAA", "a1bc", "ABC"),
        ("letters, lowercased", "aaa", "A1BC", "abc"),
        ("alphanumerics, uppercased", "XXX", "a1-c", "A1C"),
        ("alphanumerics, lowercased", "xxx", "A1-C", "a1c"),
    ];
    for (name, mask, model, expected) in cases {
        assert_eq!(with_mask(model, mask).inner(), expected, "{name}");
    }
}

#[test]
fn rejects_non_ascii_chars() {
    let cases = [
        ("digits", "###", "٣٤٥"),
        ("letters", "SSS", "éàü"),
        ("alphanumerics", "NNN", "é３ü"),
    ];
    for (name, mask, model) in cases {
        assert_eq!(with_mask(model, mask).inner(), "", "{name}");
    }
}

#[test]
fn resolves_the_named_masks() {
    let cases = [
        ("date", "2020/01/02", "20200102"),
        ("datetime", "2020/01/02 03:04", "202001020304"),
        ("time", "03:04", "0304"),
        ("fulltime", "03:04:05", "030405"),
        ("phone", "(123) 456 - 7890", "1234567890"),
        ("card", "1234 5678 9012 3456", "1234567890123456"),
    ];
    for (named, expected, model) in cases {
        assert_eq!(with_mask(model, named).inner(), expected, "{named}");
    }
}

#[test]
fn expands_the_named_masks() {
    let cases = [
        ("date", "20260824", "2026/08/24"),
        ("datetime", "202608241530", "2026/08/24 15:30"),
        ("time", "1530", "15:30"),
        ("fulltime", "153045", "15:30:45"),
        ("phone", "1234567890", "(123) 456 - 7890"),
        ("card", "1234567812345678", "1234 5678 1234 5678"),
    ];
    for (mask, model, expected) in cases {
        assert_eq!(with_mask(model, mask).inner(), expected, "{mask}");
    }
}

#[test]
fn treats_an_unknown_token_as_a_literal() {
    assert_eq!(with_mask("ab5", "CCC").inner(), "CCC");
}

fn token_c(pattern: &str, test: fn(char) -> bool) -> MaskToken {
    MaskToken::new(pattern, test)
}

#[test]
fn accepts_custom_tokens() {
    let ctx = create_mask(|p| {
        p.model = "ab5".into();
        p.mask = Some("CCC".into());
        p.tokens = MaskTokens::new().with(
            'C',
            token_c(
                "[0-4a-eA-E]",
                |c| matches!(c, '0'..='4' | 'a'..='e' | 'A'..='E'),
            )
            .transform(|v| v.to_uppercase()),
        );
    });
    // "5" não cabe no padrão personalizado: a máscara para ali
    assert_eq!(ctx.inner(), "AB");
}

#[test]
fn escapes_a_token_with_a_backslash() {
    assert_eq!(with_mask("12", r"\###").inner(), "#12");
}

#[test]
fn fills_the_remainder() {
    let cases = [
        ("nothing", FillMask::Off, "12"),
        ("the default char", FillMask::Default, "12_-__"),
        ("the supplied char", FillMask::Char('0'), "120-00"),
    ];
    for (name, fill, expected) in cases {
        let ctx = create_mask(|p| {
            p.model = "12".into();
            p.mask = Some("###-##".into());
            p.fill_mask = fill;
        });
        assert_eq!(ctx.inner(), expected, "{name}");
    }
}

#[test]
fn fill_mask_from_bool_and_str() {
    assert_eq!(FillMask::from(true), FillMask::Default);
    assert_eq!(FillMask::from(false), FillMask::Off);
    assert_eq!(FillMask::from("0"), FillMask::Char('0'));
    assert_eq!(FillMask::from("ab"), FillMask::Char('a'));
    assert_eq!(FillMask::from(""), FillMask::Default);
}

#[test]
fn consumes_the_value_from_the_end_when_reverse_filling() {
    let ctx = create_mask(|p| {
        p.model = "123".into();
        p.mask = Some("##:##".into());
        p.reverse_fill_mask = true;
    });
    assert_eq!(ctx.inner(), "1:23");
}

#[test]
fn re_masks_when_the_mask_changes() {
    let mut ctx = with_mask("12345", "###-##");
    ctx.set_mask(Some("##/###"));
    assert_eq!(ctx.inner(), "12/345");
}

#[test]
fn re_anchors_the_caret_after_its_slots_when_the_mask_changes_7777() {
    let mut ctx = with_mask("", "###.#.###");
    ctx.input.focused = true;
    ctx.update("18", false, Some("insertText"));
    ctx.tick();
    assert_eq!(ctx.input.value, "18");
    ctx.input.set_range(2, 2);

    // uma máscara calculada que vira com o valor recém-digitado
    ctx.set_mask(Some("#.###.###"));

    assert_eq!(ctx.input.value, "1.8");
    // continua depois dos 2 dados, para o próximo caractere continuar o valor
    assert_eq!(ctx.sel_start(), 3);
}

#[test]
fn rides_a_length_switching_mask_up_and_down_its_threshold() {
    // o padrão "Multiple masks" (#7920): a máscara curta tem um token a mais no fim
    const SHORT: &str = "(##) ####-#####";
    const LONG: &str = "(##) #####-####";
    let mut ctx = with_mask("", SHORT);
    ctx.input.focused = true;

    fn apply_edit(ctx: &mut Ctx, val: &str, input_type: &str) {
        let n = val.chars().count();
        ctx.input.set_value(val);
        ctx.input.set_range(n, n);
        ctx.update(val, false, Some(input_type));
    }

    for digit in "1123456789".chars() {
        let v = format!("{}{digit}", ctx.input.value);
        apply_edit(&mut ctx, &v, "insertText");
        ctx.tick();
    }
    assert_eq!(ctx.input.value, "(11) 2345-6789");

    // o 11º dígito cai no token extra, depois a máscara troca e o valor se reorganiza
    let v = format!("{}0", ctx.input.value);
    apply_edit(&mut ctx, &v, "insertText");
    ctx.tick();
    assert_eq!(ctx.input.value, "(11) 2345-67890");

    ctx.set_mask(Some(LONG));
    assert_eq!(ctx.input.value, "(11) 23456-7890");
    assert_eq!(ctx.sel_start(), ctx.input.value.chars().count());

    // apagar esse dígito volta para a máscara curta
    apply_edit(&mut ctx, "(11) 23456-789", "deleteContentBackward");
    ctx.tick();
    ctx.set_mask(Some(SHORT));
    assert_eq!(ctx.input.value, "(11) 2345-6789");
    assert_eq!(ctx.sel_start(), ctx.input.value.chars().count());
}

#[test]
fn reverse_fills_masks_that_mix_token_types() {
    let cases = [
        ("AA-##", "ab12", "AB-12"),
        ("AA-##", "AB-12", "AB-12"),
        ("AA-##", "12", "12"),
        ("A#A#", "a5", "A5"),
        ("A#A#", "a5b6", "A5B6"),
        ("SS##", "ab12", "ab12"),
        ("##-AA", "12ab", "12-AB"),
    ];
    for (mask, model, expected) in cases {
        let ctx = create_mask(|p| {
            p.model = model.into();
            p.mask = Some(mask.into());
            p.reverse_fill_mask = true;
        });
        assert_eq!(ctx.inner(), expected, "{mask} <- {model}");
    }
}

#[test]
fn renders_as_a_fixed_point() {
    let cases = [
        ("####/##/##", false, "20260824"),
        ("(###) ###-####", false, "1234567890"),
        ("##:##", false, "1234"),
        ("#.##", true, "123"),
        ("AA-##", false, "ab12"),
        ("AA-##", true, "ab12"),
        ("A#A#", true, "a5b6"),
        ("##-AA", false, "12ab"),
    ];
    for (mask, reverse, model) in cases {
        let mut ctx = create_mask(|p| {
            p.model = model.into();
            p.mask = Some(mask.into());
            p.reverse_fill_mask = reverse;
        });
        let rendered = ctx.inner();
        // o que o controle exibe tem de desmascarar de volta ao que o produziu
        ctx.update(&rendered, false, None);
        assert_eq!(ctx.inner(), rendered, "{mask} reverse={reverse}");
    }
}

#[test]
fn gives_the_unmasked_value_back_when_the_mask_goes_away() {
    let mut ctx = with_mask("12:34", "##:##");
    ctx.set_mask(None);
    assert!(!ctx.st.has_mask());
    assert_eq!(ctx.emit_model, ["1234"]);
}

#[test]
fn drops_the_fill_padding_when_the_mask_goes_away_18523() {
    let mut ctx = create_mask(|p| {
        p.model = "12".into();
        p.mask = Some("###-##".into());
        p.fill_mask = FillMask::Char('0');
    });
    assert_eq!(ctx.inner(), "120-00");
    ctx.set_mask(None);
    // o modelo já guarda o único dado que havia
    assert!(ctx.emit_model.is_empty());
}

#[test]
fn stays_quiet_when_the_model_already_holds_the_unmasked_value() {
    let mut ctx = with_mask("1234", "##:##");
    ctx.set_mask(None);
    assert!(ctx.emit_model.is_empty());
}

#[test]
fn reacts_to_a_change_of_the_fill_mask() {
    let mut ctx = with_mask("12", "###-##");
    ctx.set_fill(FillMask::Default, false);
    assert_eq!(ctx.inner(), "12_-__");
}

#[test]
fn reacts_to_a_change_of_the_reverse_fill_mask() {
    let mut ctx = with_mask("12", "###-##");
    ctx.set_fill(FillMask::Off, true);
    assert_eq!(ctx.inner(), "12");
}

#[test]
fn reacts_to_a_change_of_the_custom_tokens() {
    let alnum = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    let mut ctx = create_mask(|p| {
        p.model = "ab5".into();
        p.mask = Some("CCC".into());
        p.tokens = MaskTokens::new().with('C', token_c("[a-z0-9]", alnum));
    });
    assert_eq!(ctx.inner(), "ab5");

    let out = ctx.st.set_tokens(
        Some(&mut ctx.input),
        MaskTokens::new().with('C', token_c("[a-z]", |c| c.is_ascii_lowercase())),
    );
    ctx.emit_model_from(out);
    ctx.tick();
    assert_eq!(ctx.inner(), "ab");
}

#[test]
fn reacts_to_a_change_of_the_field_type() {
    let mut ctx = with_mask("123", "###");
    assert!(ctx.st.has_mask());
    ctx.st.set_text_type(false);
    assert!(!ctx.st.has_mask());
}

#[test]
fn reacts_to_a_change_of_unmasked_value() {
    let mut ctx = with_mask("", "###-##");
    ctx.update("12345", false, None);
    assert_eq!(ctx.emit_value, ["123-45"]);
    let out = ctx.st.set_unmasked_value(Some(&mut ctx.input), true);
    assert_eq!(out.emit.as_deref(), Some("12345"));
}

// --- digitação ------------------------------------------------------------------------------

#[test]
fn masks_what_gets_typed_in_and_reports_it_upwards() {
    let mut ctx = with_mask("", "###-##");
    ctx.update("12345", false, None);
    assert_eq!(ctx.inner(), "123-45");
    assert_eq!(ctx.input.value, "123-45");
    assert_eq!(ctx.emit_value, ["123-45"]);
}

#[test]
fn keeps_a_typed_char_that_matches_an_upcoming_mask_literal_8354() {
    let mut ctx = with_mask("", "+1 123 ### ## ##");
    // os dígitos literais do prefixo não podem engolir o "1" digitado
    ctx.update("1", false, None);
    assert_eq!(ctx.inner(), "+1 123 1");
    ctx.update("+1 123 1234567", false, None);
    assert_eq!(ctx.inner(), "+1 123 123 45 67");
}

#[test]
fn keeps_a_typed_digit_that_equals_a_digit_literal_15624_18051() {
    let mut ctx = with_mask("", "11##");
    // o "1" digitado no campo vazio cai no primeiro slot, não vira um dos literais "11"
    ctx.update("1", false, Some("insertText"));
    assert_eq!(ctx.inner(), "111");
    // o próximo "1" é anexado pelo navegador ao que está exibido
    let v = format!("{}1", ctx.input.value);
    ctx.update(&v, false, Some("insertText"));
    assert_eq!(ctx.inner(), "1111");
}

#[test]
fn keeps_a_typed_zero_that_equals_a_zero_literal_18051() {
    let mut ctx = with_mask("", "04## ### ###");
    ctx.update("0", false, Some("insertText"));
    assert_eq!(ctx.inner(), "040");
    let v = format!("{}0", ctx.input.value);
    ctx.update(&v, false, Some("insertText"));
    assert_eq!(ctx.inner(), "0400 ");
}

#[test]
fn re_lays_out_the_data_when_typing_into_the_middle() {
    let mut ctx = with_mask("12345", "###-##");
    // "9" inserido logo depois de "123" do "123-45" exibido
    ctx.update("1239-45", false, Some("insertText"));
    assert_eq!(ctx.inner(), "123-94");
}

#[test]
fn clears_out_when_deleting_the_only_typed_char_behind_literals() {
    let mut ctx = with_mask("", "11##");
    ctx.update("1", false, Some("insertText"));
    assert_eq!(ctx.input.value, "111");
    ctx.update("11", false, Some("deleteContentBackward"));
    assert_eq!(ctx.inner(), "");
}

#[test]
fn drops_a_typed_char_its_token_slot_rejects_keeping_the_rest() {
    let mut ctx = with_mask("ab12", "AA##");
    // dígito digitado na frente, onde se espera uma letra
    ctx.update("5AB12", false, Some("insertText"));
    assert_eq!(ctx.inner(), "AB12");
}

#[test]
fn types_over_the_fill_chars_not_into_them() {
    let mut ctx = create_mask(|p| {
        p.model = "12".into();
        p.mask = Some("###-##".into());
        p.fill_mask = FillMask::Default;
    });
    // "3" digitado no cursor, que está sobre o primeiro caractere de preenchimento
    ctx.update("123_-__", false, Some("insertText"));
    assert_eq!(ctx.inner(), "123-__");
}

#[test]
fn shifts_the_digits_while_reverse_filling_with_zero_18523() {
    let mut ctx = create_mask(|p| {
        p.mask = Some("#.##".into());
        p.fill_mask = FillMask::Char('0');
        p.reverse_fill_mask = true;
    });
    let mut rendered = vec![ctx.inner()];
    // digitar anexa no cursor, que o preenchimento reverso mantém no fim
    for ch in ['1', '0', '0', '5'] {
        let v = format!("{}{ch}", ctx.inner());
        let n = v.chars().count();
        ctx.input.set_value(&v);
        ctx.input.set_range(n, n);
        let v = ctx.input.value.clone();
        ctx.update(&v, false, Some("insertText"));
        let inner = ctx.inner();
        ctx.input.set_value(&inner);
        rendered.push(inner);
    }
    assert_eq!(rendered, ["0.00", "0.01", "0.10", "1.00", "10.05"]);
}

#[test]
fn backspacing_a_zero_fill_char_drops_a_data_char_18523() {
    let mut ctx = create_mask(|p| {
        p.model = "12".into();
        p.mask = Some("###-##".into());
        p.fill_mask = FillMask::Char('0');
    });
    assert_eq!(ctx.inner(), "120-00");
    // o último caractere de preenchimento é apagado: remascarar o restauraria e a edição seria um
    // no-op; sai o dado vizinho
    let v: String = {
        let mut s = ctx.inner();
        s.pop();
        s
    };
    ctx.input.set_value(&v);
    let n = v.chars().count();
    ctx.input.set_range(n, n);
    ctx.update(&v, false, Some("deleteContentBackward"));
    assert_eq!(ctx.inner(), "100-00");
}

#[test]
fn types_a_leading_zero_before_the_next_digit_not_after_18523() {
    let mut ctx = create_mask(|p| {
        p.mask = Some("##:##".into());
        p.fill_mask = FillMask::Char('0');
        p.unmasked_value = true;
    });
    ctx.input.focused = true;
    ctx.type_char('0');
    ctx.type_char('1');
    // o render não mostra a diferença; o modelo precisa mostrar
    assert_eq!(ctx.input.value, "01:00");
    assert_eq!(ctx.emit_value.last().map(String::as_str), Some("01"));
}

#[test]
fn advances_the_caret_when_a_typed_char_leaves_the_render_alone_18523() {
    let mut ctx = create_mask(|p| {
        p.mask = Some("###-##".into());
        p.fill_mask = FillMask::Char('0');
        p.unmasked_value = true;
    });
    ctx.input.focused = true;
    // todos estes renderizam "000-00": só o cursor pode andar; se parar, o próximo caractere cai
    // na frente do anterior
    for expected in ["0", "00", "000", "0000", "00000"] {
        ctx.type_char('0');
        assert_eq!(ctx.input.value, "000-00");
        assert_eq!(ctx.emit_value.last().map(String::as_str), Some(expected));
    }
}

/// `traceFillCharParity`: repete a sequência em duas máscaras que só diferem no caractere de
/// preenchimento. Só o preenchimento pode diferir; o modelo e o cursor, não.
fn trace_fill_char_parity(mask: &str, reverse: bool, ops: &[Edit], fill: FillMask) -> Vec<String> {
    let mut ctx = create_mask(|p| {
        p.mask = Some(mask.into());
        p.reverse_fill_mask = reverse;
        p.fill_mask = fill;
        p.unmasked_value = true;
    });
    let mut trace = Vec::new();
    for op in ops {
        ctx.input.focused = true;
        ctx.edit(*op);
        let model = ctx.emit_value.last().cloned().unwrap_or_default();
        let name = match op {
            Edit::Type(c) => format!("type{c}"),
            Edit::Bksp => "bksp".to_string(),
        };
        trace.push(format!(
            "{name} {}caret={}",
            if reverse {
                String::new()
            } else {
                format!("model={model} ")
            },
            ctx.sel_start()
        ));
    }
    trace
}

#[test]
fn reads_the_same_whether_or_not_the_fill_char_is_data_like() {
    use Edit::{Bksp, Type};
    let cases: Vec<(&str, bool, Vec<Edit>)> = vec![
        (
            "###-##",
            false,
            vec![Type('0'), Type('6'), Type('0'), Type('3')],
        ),
        (
            "###-##",
            false,
            vec![Type('0'), Type('0'), Type('0'), Type('0')],
        ),
        (
            "###-##",
            false,
            vec![Type('1'), Type('2'), Bksp, Type('0'), Bksp],
        ),
        (
            "(###) ###-####",
            false,
            vec![Type('0'), Type('9'), Type('0'), Type('1')],
        ),
        (
            "##:##",
            false,
            vec![Type('0'), Type('1'), Type('0'), Type('2')],
        ),
        ("####", false, vec![Type('0'), Type('0'), Type('1')]),
        (
            "#.##",
            true,
            vec![Type('1'), Type('0'), Type('0'), Type('5')],
        ),
        ("#.##", true, vec![Type('0'), Type('0'), Type('5')]),
    ];
    for (mask, reverse, ops) in cases {
        // "_" nunca satisfaz "#"; "0" sempre
        assert_eq!(
            trace_fill_char_parity(mask, reverse, &ops, FillMask::Char('0')),
            trace_fill_char_parity(mask, reverse, &ops, FillMask::Default),
            "{mask} reverse={reverse}"
        );
    }
}

#[test]
fn re_anchors_the_caret_past_zero_fill_chars_on_a_mask_change_18523() {
    let mut ctx = create_mask(|p| {
        p.model = "18".into();
        p.mask = Some("###.#.###".into());
        p.fill_mask = FillMask::Char('0');
    });
    ctx.input.focused = true;
    let inner = ctx.inner();
    ctx.input.set_value(&inner);
    assert_eq!(ctx.input.value, "180.0.000");

    // o cursor está onde a região de preenchimento o estaciona, depois dos dados
    ctx.input.set_range(9, 9);
    ctx.set_mask(Some("#.###.###"));

    assert_eq!(ctx.input.value, "1.800.000");
    // de volta depois dos 2 dados, não encalhado no fim do preenchimento
    assert_eq!(ctx.sel_start(), 3);
}

#[test]
fn accepts_a_typed_digit_literal_look_alike_when_reverse_filling() {
    let mut ctx = create_mask(|p| {
        p.model = "123".into();
        p.mask = Some("##:##".into());
        p.reverse_fill_mask = true;
    });
    ctx.update("1:234", false, Some("insertText"));
    assert_eq!(ctx.inner(), "12:34");
}

#[test]
fn backspacing_a_literal_deletes_the_data_char_before_it_17639() {
    let mut ctx = with_mask("1234", "card");
    assert_eq!(ctx.inner(), "1234 ");
    // backspace de teclado virtual (sem keydown): o navegador tirou só o espaço literal
    ctx.update("1234", false, Some("deleteContentBackward"));
    assert_eq!(ctx.inner(), "123");
}

#[test]
fn forward_deleting_a_literal_deletes_the_data_char_after_it() {
    let mut ctx = with_mask("12345", "###-##");
    assert_eq!(ctx.inner(), "123-45");
    // cursor depois de "123"; DELETE tirou só o literal "-"
    ctx.update("12345", false, Some("deleteContentForward"));
    assert_eq!(ctx.inner(), "123-5");
}

#[test]
fn re_lays_out_the_value_after_cuts_and_word_deletes() {
    let cases: [(&str, bool, usize, usize, &str, &str); 6] = [
        (
            "a cut of the leading data",
            false,
            0,
            3,
            "deleteByCut",
            "45",
        ),
        (
            "a cut spanning the literal",
            false,
            1,
            4,
            "deleteByCut",
            "145-",
        ),
        (
            "a word delete of the tail",
            false,
            4,
            6,
            "deleteWordBackward",
            "123-",
        ),
        (
            "a word delete of everything",
            false,
            0,
            6,
            "deleteWordBackward",
            "",
        ),
        (
            "a forward word delete",
            false,
            0,
            4,
            "deleteWordForward",
            "45",
        ),
        ("a cut while filling", true, 0, 3, "deleteByCut", "45_-__"),
    ];
    for (name, fill, from, to, input_type, expected) in cases {
        let mut ctx = create_mask(|p| {
            p.model = "12345".into();
            p.mask = Some("###-##".into());
            if fill {
                p.fill_mask = FillMask::Default;
            }
        });
        ctx.remove_range(from, to, input_type);
        assert_eq!(ctx.inner(), expected, "{name}");
    }
}

#[test]
fn drops_a_right_aligned_separator_nothing_filled() {
    let mut ctx = create_mask(|p| {
        p.mask = Some("AA-##".into());
        p.reverse_fill_mask = true;
    });
    ctx.input.focused = true;
    // os dígitos chegam aos slots "#", os "A" ficam vazios: o "-" não tem o que separar
    for ch in ['0', '5', '7'] {
        ctx.type_char(ch);
        ctx.sync_model();
    }
    assert_eq!(ctx.inner(), "57");
}

#[test]
fn backspacing_a_literal_with_no_data_before_it_stays_put() {
    let mut ctx = with_mask("1", "+1 ###");
    assert_eq!(ctx.inner(), "+1 1");
    // cursor depois do "+"; o backspace tirou só o literal "+"
    ctx.update("1 1", false, Some("deleteContentBackward"));
    assert_eq!(ctx.inner(), "+1 1");
}

#[test]
fn still_strips_the_literals_out_of_a_pasted_masked_value() {
    let mut ctx = create_mask(|p| {
        p.mask = Some("+1 123 ### ## ##".into());
        p.unmasked_value = true;
    });
    ctx.update("+1 123 456 78 90", false, Some("insertFromPaste"));
    assert_eq!(ctx.inner(), "+1 123 456 78 90");
    assert_eq!(ctx.emit_value, ["4567890"]);
}

#[test]
fn reports_the_unmasked_value_when_asked_to() {
    let mut ctx = create_mask(|p| {
        p.mask = Some("###-##".into());
        p.unmasked_value = true;
    });
    ctx.update("12345", false, None);
    assert_eq!(ctx.input.value, "123-45");
    assert_eq!(ctx.emit_value, ["12345"]);
}

#[test]
fn leaves_the_fill_chars_out_of_the_unmasked_value_18523() {
    let mut ctx = create_mask(|p| {
        p.mask = Some("###-##".into());
        p.fill_mask = FillMask::Char('0');
        p.unmasked_value = true;
    });
    // "1" digitado no cursor, que está sobre o primeiro caractere de preenchimento
    let raw: String = std::iter::once('1')
        .chain(ctx.inner().chars().skip(1))
        .collect();
    ctx.update(&raw, false, Some("insertText"));
    assert_eq!(ctx.input.value, "100-00");
    // os cinco "0" são preenchimento, não dígitos digitados
    assert_eq!(ctx.emit_value, ["1"]);
}

#[test]
fn stays_quiet_when_the_value_did_not_really_change() {
    let mut ctx = with_mask("123-45", "###-##");
    ctx.update("12345", false, None);
    assert_eq!(ctx.inner(), "123-45");
    assert!(ctx.emit_value.is_empty());
}

#[test]
fn does_not_report_an_empty_value_for_an_empty_model() {
    // o `null` do JS não existe aqui (`value` é `String`); o equivalente é o modelo vazio
    let mut ctx = with_mask("", "###-##");
    ctx.update("", false, None);
    assert!(ctx.emit_value.is_empty());
}

#[test]
fn rebuilds_its_internals_when_told_to() {
    let mut ctx = with_mask("12345", "###-##");
    // a máscara é trocada sem passar pelo watcher
    ctx.st.props.mask = Some("##/###".into());
    ctx.update("12345", true, None);
    assert_eq!(ctx.inner(), "12/345");
    assert_eq!(ctx.emit_value.last().map(String::as_str), Some("12/345"));
}

// --- teclado e cursor -----------------------------------------------------------------------

#[test]
fn click_clears_the_selection_anchor() {
    let mut ctx = with_mask("", "###-##");
    ctx.st.selection_anchor = Some(3);
    ctx.st.on_masked_click();
    assert_eq!(ctx.st.selection_anchor, None);
}

#[test]
fn leaves_the_cursor_alone_when_alt_modifier_or_prevented() {
    let cases = [
        (
            "ALT is held",
            MaskKeyEvent {
                alt: true,
                ..MaskKeyEvent::new(MaskKey::Right)
            },
        ),
        ("the key is a modifier", MaskKeyEvent::new(MaskKey::Other)),
        (
            "the event was prevented",
            MaskKeyEvent {
                default_prevented: true,
                ..MaskKeyEvent::new(MaskKey::Right)
            },
        ),
        (
            "an IME composition is running",
            MaskKeyEvent {
                composing: true,
                ..MaskKeyEvent::new(MaskKey::Right)
            },
        ),
    ];
    for (name, ev) in cases {
        let mut ctx = with_mask("12345", "###-##");
        ctx.input.set_range(1, 1);
        let prevented = ctx.st.on_masked_keydown(&mut ctx.input, &ev);
        assert!(!prevented, "{name}");
        assert_eq!(ctx.sel_start(), 1, "{name}");
    }
}

#[test]
fn skips_over_the_mask_literals_when_moving_right() {
    let mut ctx = with_mask("12345", "###-##");
    ctx.input.set_range(3, 3);
    let prevented = ctx
        .st
        .on_masked_keydown(&mut ctx.input, &MaskKeyEvent::new(MaskKey::Right));
    assert!(prevented);
    // pousa depois do separador "-"
    assert_eq!(ctx.sel_start(), 4);
}

#[test]
fn skips_over_the_mask_literals_when_moving_left() {
    let mut ctx = with_mask("12345", "###-##");
    ctx.input.set_range(4, 4);
    ctx.st
        .on_masked_keydown(&mut ctx.input, &MaskKeyEvent::new(MaskKey::Left));
    // salta o separador "-", para o último slot de dígito antes dele
    assert_eq!(ctx.sel_start(), 2);
}

#[test]
fn extends_the_selection_while_shift_is_held() {
    let mut ctx = with_mask("12345", "###-##");
    ctx.input.set_range(1, 1);
    let ev = MaskKeyEvent {
        shift: true,
        ..MaskKeyEvent::new(MaskKey::Right)
    };
    ctx.st.on_masked_keydown(&mut ctx.input, &ev);
    assert_eq!(ctx.input.start, 1);
    assert_eq!(ctx.input.end, 2);
}

#[test]
fn selects_up_to_the_character_a_backspace_is_about_to_remove() {
    let mut ctx = with_mask("12345", "###-##");
    ctx.input.set_range(4, 4);
    ctx.st
        .on_masked_keydown(&mut ctx.input, &MaskKeyEvent::new(MaskKey::Backspace));
    // a seleção atravessa o separador, então sai um caractere de verdade
    assert_eq!(ctx.input.start, 2);
    assert_eq!(ctx.input.end, 4);
}

#[test]
fn selects_the_character_a_delete_is_about_to_remove_when_reversed() {
    let mut ctx = create_mask(|p| {
        p.model = "12345".into();
        p.mask = Some("###-##".into());
        p.reverse_fill_mask = true;
    });
    ctx.input.set_range(1, 1);
    ctx.st
        .on_masked_keydown(&mut ctx.input, &MaskKeyEvent::new(MaskKey::Delete));
    assert_eq!(ctx.input.start, 1);
    assert!(ctx.input.end > 1);
}

#[test]
fn anchors_a_paste_on_the_first_free_slot() {
    let mut ctx = with_mask("12345", "###-##");
    ctx.st.move_cursor_for_paste(&mut ctx.input, 0, 6);
    assert_eq!((ctx.input.start, ctx.input.end), (0, 6));
    ctx.st.move_cursor_for_paste(&mut ctx.input, 10, 10);
    // nunca passa do que já foi mascarado
    assert_eq!(ctx.input.start, 6);
}

// --- varredura de combinações ---------------------------------------------------------------

#[test]
fn holds_its_ground_across_the_prop_combinations() {
    fn custom() -> MaskTokens {
        MaskTokens::new()
            .with('Z', MaskToken::new("[A-Z]", |c| c.is_ascii_uppercase()))
            .with('d', MaskToken::new("[0-9]", |c| c.is_ascii_digit()))
    }
    let sets: Vec<(Option<MaskTokens>, &str, Vec<&str>)> = vec![
        (
            None,
            "1234567890abXY",
            vec![
                "date", "datetime", "time", "fulltime", "phone", "card", "###-##", "#.##", "AA-##",
                "A#A#", r"\###", "NNN/NN",
            ],
        ),
        (Some(custom()), "ABCD1234", vec!["ZZ-dd", "Zd.dd", "dddd"]),
    ];
    let fills = [
        FillMask::Off,
        FillMask::Default,
        FillMask::Char('0'),
        FillMask::Char('A'),
    ];
    let mut failures: Vec<String> = Vec::new();

    for (tokens, pool, masks) in &sets {
        let pool: Vec<char> = pool.chars().collect();
        for mask in masks {
            for fill in fills {
                for reverse in [false, true] {
                    for unmasked in [false, true] {
                        let build = |model: &str| {
                            create_mask(|p| {
                                p.mask = Some((*mask).into());
                                p.fill_mask = fill;
                                p.reverse_fill_mask = reverse;
                                p.unmasked_value = unmasked;
                                p.tokens = tokens.clone().unwrap_or_default();
                                p.model = model.into();
                            })
                        };
                        let mut ctx = build("");
                        ctx.input.focused = true;
                        for i in 0..6 {
                            ctx.type_char(pool[(i * 3) % pool.len()]);
                            ctx.sync_model();
                        }
                        let n = ctx.input.value.chars().count().min(2);
                        ctx.remove_range(0, n, "deleteByCut");
                        for i in 0..3 {
                            ctx.type_char(pool[(i * 5) % pool.len()]);
                            ctx.sync_model();
                        }

                        let rendered = ctx.inner();
                        let model = if unmasked {
                            ctx.st.props.model.clone()
                        } else {
                            rendered.clone()
                        };
                        let label = format!(
                            "mask={mask} fill={fill:?} reverse={reverse} unmasked={unmasked}"
                        );

                        ctx.update(&rendered, false, None);
                        ctx.tick();
                        if ctx.inner() != rendered {
                            failures.push(format!(
                                "{label}: displaying {rendered} unmasked to {}",
                                ctx.inner()
                            ));
                            continue;
                        }

                        let echoed = build(&model);
                        if echoed.inner() != rendered {
                            failures.push(format!(
                                "{label}: model {model} re-rendered as {}, not {rendered}",
                                echoed.inner()
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

// --- posições UTF-16 ------------------------------------------------------------------------

#[test]
fn utf16_offsets_round_trip() {
    let s = "a😀b";
    assert_eq!(char_to_utf16(s, 0), 0);
    assert_eq!(char_to_utf16(s, 2), 3);
    assert_eq!(char_to_utf16(s, 3), 4);
    assert_eq!(utf16_to_char(s, 0), 0);
    assert_eq!(utf16_to_char(s, 1), 1);
    assert_eq!(utf16_to_char(s, 3), 2);
    assert_eq!(utf16_to_char(s, 4), 3);
    assert_eq!(utf16_to_char(s, 99), 3);
}
