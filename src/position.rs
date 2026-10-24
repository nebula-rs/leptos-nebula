//! Motor de posição dos popups ancorados (`Menu`, `Tooltip`): porta do
//! `private.use-position-engine` do Quasar (o motor "fallback", em pixels), **puro** e sem DOM.
//!
//! Entram o retângulo da âncora, o tamanho natural do popup e o da janela, as origens
//! (`anchor` na âncora, `self_` no popup), o `offset`, `fit` e `cover`; sai o canto superior
//! esquerdo do popup (`position: fixed`), os tetos de tamanho e o que mudou. Quando o popup não
//! cabe no lado pedido ele **vira** para o lado com mais espaço e ganha um teto de tamanho que
//! cabe nesse espaço. Quem mede e escreve o estilo é `components/popup.rs`.

use std::{fmt, str::FromStr};

/// Linha de origem vertical.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Vertical {
    Top,
    Center,
    Bottom,
}

/// Linha de origem horizontal. `Start`/`End` seguem a direção do texto (esquerda/direita em
/// `ltr`, ao contrário em `rtl`); `Left`/`Right` são fixos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Horizontal {
    Left,
    Middle,
    Right,
    Start,
    End,
}

impl Horizontal {
    /// Troca `Start`/`End` por `Left`/`Right` conforme a direção.
    pub fn resolve(self, rtl: bool) -> Self {
        match (self, rtl) {
            (Self::Start, false) | (Self::End, true) => Self::Left,
            (Self::Start, true) | (Self::End, false) => Self::Right,
            (other, _) => other,
        }
    }
}

/// Um ponto de ancoragem, como o `'bottom start'` do Quasar: `Position::new(Vertical::Bottom,
/// Horizontal::Start)` ou `"bottom start".parse()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub vertical: Vertical,
    pub horizontal: Horizontal,
}

impl Position {
    pub const fn new(vertical: Vertical, horizontal: Horizontal) -> Self {
        Self {
            vertical,
            horizontal,
        }
    }
}

/// `Position` ilegível (o texto não é `<top|center|bottom> <left|middle|right|start|end>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsePositionError(String);

impl fmt::Display for ParsePositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "posição inválida `{}`: use `<top|center|bottom> <left|middle|right|start|end>`",
            self.0
        )
    }
}

impl std::error::Error for ParsePositionError {}

impl FromStr for Position {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParsePositionError(s.to_string());
        let mut parts = s.split_whitespace();
        let (Some(v), Some(h), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(err());
        };
        let vertical = match v {
            "top" => Vertical::Top,
            "center" => Vertical::Center,
            "bottom" => Vertical::Bottom,
            _ => return Err(err()),
        };
        let horizontal = match h {
            "left" => Horizontal::Left,
            "middle" => Horizontal::Middle,
            "right" => Horizontal::Right,
            "start" => Horizontal::Start,
            "end" => Horizontal::End,
            _ => return Err(err()),
        };
        Ok(Self::new(vertical, horizontal))
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = match self.vertical {
            Vertical::Top => "top",
            Vertical::Center => "center",
            Vertical::Bottom => "bottom",
        };
        let h = match self.horizontal {
            Horizontal::Left => "left",
            Horizontal::Middle => "middle",
            Horizontal::Right => "right",
            Horizontal::Start => "start",
            Horizontal::End => "end",
        };
        write!(f, "{v} {h}")
    }
}

/// Retângulo em pixels da janela (como o `getBoundingClientRect`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub top: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }

    pub fn right(&self) -> f64 {
        self.left + self.width
    }
}

/// Largura e altura em pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// Tudo que o motor precisa saber.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    /// Retângulo da âncora.
    pub anchor: Rect,
    /// Tamanho natural do popup (sem os tetos que o motor decide aqui).
    pub popup: Size,
    /// Área visível da janela (o `clientWidth`/`clientHeight` do `<html>`).
    pub viewport: Size,
    /// Ponto da âncora a que o popup se prende.
    pub anchor_origin: Position,
    /// Ponto do popup que encosta no da âncora.
    pub self_origin: Position,
    /// `[x, y]` em pixels: expande a caixa da âncora para fora; o popup se posiciona contra a
    /// caixa expandida. Ignorado com `cover`.
    pub offset: (f64, f64),
    /// O popup tem pelo menos a largura da âncora.
    pub fit: bool,
    /// O popup cobre a âncora: `self_origin` = `anchor_origin`, sem `offset`, pelo menos do
    /// tamanho da âncora.
    pub cover: bool,
    /// Direção do texto (resolve `Start`/`End`).
    pub rtl: bool,
    /// Teto de altura que o popup já tem (CSS), em pixels; combinado (`min`) com o espaço.
    pub bound_max_height: Option<f64>,
    /// Teto de largura que o popup já tem (CSS), em pixels; combinado (`min`) com o espaço.
    pub bound_max_width: Option<f64>,
}

/// Resultado: o estilo `position: fixed` do popup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub top: f64,
    pub left: f64,
    /// Teto de altura, quando o espaço não bastava (ou o teto do CSS é menor).
    pub max_height: Option<f64>,
    /// Teto de largura, quando o espaço não bastava (ou o teto do CSS é menor).
    pub max_width: Option<f64>,
    /// Largura mínima de `fit`/`cover` (a da âncora, limitada pelo teto do CSS).
    pub min_width: Option<f64>,
    /// Altura mínima de `cover`.
    pub min_height: Option<f64>,
    /// Origens finais (com `Start`/`End` resolvidos e a virada aplicada).
    pub anchor_origin: Position,
    pub self_origin: Position,
    /// O eixo vertical virou para o outro lado.
    pub flipped_vertical: bool,
    /// O eixo horizontal virou para o outro lado.
    pub flipped_horizontal: bool,
}

// ── Um eixo ────────────────────────────────────────────────────────────────

/// Uma linha do eixo: a do começo (topo/esquerda), a do centro, a do fim (base/direita).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    Lo,
    Mid,
    Hi,
}

/// A caixa da âncora, já expandida pelo `offset`, ao longo de um eixo.
#[derive(Clone, Copy)]
struct Axis {
    lo: f64,
    hi: f64,
    mid: f64,
}

impl Axis {
    fn line(&self, l: Line) -> f64 {
        match l {
            Line::Lo => self.lo,
            Line::Mid => self.mid,
            Line::Hi => self.hi,
        }
    }
}

struct AxisOut {
    anchor: Line,
    this: Line,
    cap: Option<f64>,
    flipped: bool,
    /// Posição do começo do popup.
    start: f64,
}

/// Decide um eixo (`QMenu`/`applyBoundary`): se o popup não cabe no lado pedido, vira para o lado
/// com mais espaço (o pedido, em empate) e ganha um teto de tamanho igual a esse espaço. Um eixo
/// em que o popup é centralizado não vira, só é empurrado para dentro da janela quando a âncora
/// também é centralizada.
fn resolve_axis(
    a: Axis,
    mut av: Line,
    mut sv: Line,
    size: f64,
    viewport: f64,
    bound: Option<f64>,
) -> AxisOut {
    let (av0, sv0) = (av, sv);
    let mut cap = None;
    let mut eff = size;

    if sv != Line::Mid {
        let start = a.line(av) - if sv == Line::Hi { size } else { 0.0 };
        if start < 0.0 || start + size > viewport {
            // as duas colocações do eixo: começo do popup na linha "abaixo" ou fim do popup na
            // linha "acima"
            let (below, above) = match (av, av == sv) {
                (Line::Mid, _) => (Line::Mid, Line::Mid),
                (_, true) => (Line::Lo, Line::Hi),
                (_, false) => (Line::Hi, Line::Lo),
            };
            let space_below = viewport - a.line(below).max(0.0);
            let space_above = a.line(above).min(viewport);
            let take_below = if sv == Line::Lo {
                space_below >= space_above
            } else {
                space_below > space_above
            };
            let space = if take_below {
                av = below;
                sv = Line::Lo;
                space_below
            } else {
                av = above;
                sv = Line::Hi;
                space_above
            };
            let c = bound.map_or(space, |b| space.min(b));
            cap = Some(c);
            eff = size.min(c);
        }
    }

    let flipped = (av, sv) != (av0, sv0);
    let line = a.line(av);
    let start = match sv {
        Line::Lo => line,
        Line::Hi => line - eff,
        Line::Mid => {
            let s = line - eff / 2.0;
            if av == Line::Mid {
                // âncora e popup centralizados: não sai da janela
                s.min(viewport - eff).max(0.0)
            } else {
                s
            }
        }
    };

    AxisOut {
        anchor: av,
        this: sv,
        cap,
        flipped,
        start,
    }
}

fn vertical_line(v: Vertical) -> Line {
    match v {
        Vertical::Top => Line::Lo,
        Vertical::Center => Line::Mid,
        Vertical::Bottom => Line::Hi,
    }
}

fn horizontal_line(h: Horizontal) -> Line {
    match h {
        Horizontal::Middle => Line::Mid,
        // `Start`/`End` já foram resolvidos antes de chegar aqui
        Horizontal::Left | Horizontal::Start => Line::Lo,
        Horizontal::Right | Horizontal::End => Line::Hi,
    }
}

fn to_vertical(l: Line) -> Vertical {
    match l {
        Line::Lo => Vertical::Top,
        Line::Mid => Vertical::Center,
        Line::Hi => Vertical::Bottom,
    }
}

fn to_horizontal(l: Line) -> Horizontal {
    match l {
        Line::Lo => Horizontal::Left,
        Line::Mid => Horizontal::Middle,
        Line::Hi => Horizontal::Right,
    }
}

/// Calcula a colocação. Ver o módulo.
pub fn compute(input: &Input) -> Placement {
    let Input {
        anchor: rect,
        viewport,
        cover,
        fit,
        ..
    } = *input;
    let anchor_origin = Position::new(
        input.anchor_origin.vertical,
        input.anchor_origin.horizontal.resolve(input.rtl),
    );
    let self_origin = if cover {
        anchor_origin
    } else {
        Position::new(
            input.self_origin.vertical,
            input.self_origin.horizontal.resolve(input.rtl),
        )
    };
    let (ox, oy) = if cover { (0.0, 0.0) } else { input.offset };

    // `fit`/`cover`: pelo menos o tamanho da âncora (limitado pelo teto que o popup já tem)
    let min_width = (fit || cover).then(|| {
        input
            .bound_max_width
            .map_or(rect.width, |b| rect.width.min(b))
    });
    let min_height = cover.then(|| {
        input
            .bound_max_height
            .map_or(rect.height, |b| rect.height.min(b))
    });
    let width = min_width.map_or(input.popup.width, |m| input.popup.width.max(m));
    let height = min_height.map_or(input.popup.height, |m| input.popup.height.max(m));

    let ay = Axis {
        lo: rect.top - oy,
        hi: rect.bottom() + oy,
        mid: rect.top + rect.height / 2.0,
    };
    let ax = Axis {
        lo: rect.left - ox,
        hi: rect.right() + ox,
        mid: rect.left + rect.width / 2.0,
    };

    let v = resolve_axis(
        ay,
        vertical_line(anchor_origin.vertical),
        vertical_line(self_origin.vertical),
        height,
        viewport.height,
        input.bound_max_height,
    );
    let h = resolve_axis(
        ax,
        horizontal_line(anchor_origin.horizontal),
        horizontal_line(self_origin.horizontal),
        width,
        viewport.width,
        input.bound_max_width,
    );

    Placement {
        top: v.start,
        left: h.start,
        max_height: v.cap,
        max_width: h.cap,
        min_width,
        min_height,
        anchor_origin: Position::new(to_vertical(v.anchor), to_horizontal(h.anchor)),
        self_origin: Position::new(to_vertical(v.this), to_horizontal(h.this)),
        flipped_vertical: v.flipped,
        flipped_horizontal: h.flipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Horizontal as H;
    use Vertical as V;

    /// Janela 1000×800; âncora 100×40 em (left, top); popup 200×100.
    fn base(left: f64, top: f64) -> Input {
        Input {
            anchor: Rect {
                top,
                left,
                width: 100.0,
                height: 40.0,
            },
            popup: Size {
                width: 200.0,
                height: 100.0,
            },
            viewport: Size {
                width: 1000.0,
                height: 800.0,
            },
            anchor_origin: Position::new(V::Bottom, H::Left),
            self_origin: Position::new(V::Top, H::Left),
            offset: (0.0, 0.0),
            fit: false,
            cover: false,
            rtl: false,
            bound_max_height: None,
            bound_max_width: None,
        }
    }

    fn at(input: Input, anchor: (V, H), this: (V, H)) -> Placement {
        compute(&Input {
            anchor_origin: Position::new(anchor.0, anchor.1),
            self_origin: Position::new(this.0, this.1),
            ..input
        })
    }

    #[test]
    fn parses_and_prints_positions() {
        let p: Position = "bottom start".parse().unwrap();
        assert_eq!(p, Position::new(V::Bottom, H::Start));
        assert_eq!(p.to_string(), "bottom start");
        assert_eq!(
            "center middle".parse::<Position>().unwrap(),
            Position::new(V::Center, H::Middle)
        );
        assert!("middle center".parse::<Position>().is_err());
        assert!("bottom".parse::<Position>().is_err());
        assert!("bottom left extra".parse::<Position>().is_err());
        assert!("".parse::<Position>().is_err());
    }

    #[test]
    fn start_and_end_follow_the_direction() {
        assert_eq!(H::Start.resolve(false), H::Left);
        assert_eq!(H::Start.resolve(true), H::Right);
        assert_eq!(H::End.resolve(false), H::Right);
        assert_eq!(H::End.resolve(true), H::Left);
        assert_eq!(H::Middle.resolve(true), H::Middle);
        assert_eq!(H::Left.resolve(true), H::Left);
    }

    #[test]
    fn below_the_anchor_aligned_to_its_left_edge() {
        // âncora em (300, 200): borda de baixo = 240
        let p = compute(&base(300.0, 200.0));
        assert_eq!((p.top, p.left), (240.0, 300.0));
        assert_eq!((p.max_height, p.max_width), (None, None));
        assert!(!p.flipped_vertical && !p.flipped_horizontal);
    }

    #[test]
    fn rtl_start_aligns_to_the_right_edge() {
        let input = Input {
            rtl: true,
            ..base(300.0, 200.0)
        };
        // bottom start / top start em rtl = right / right: o fim do popup na borda direita (400)
        let p = at(input, (V::Bottom, H::Start), (V::Top, H::Start));
        assert_eq!(p.left, 400.0 - 200.0);
        assert_eq!(p.self_origin.horizontal, H::Right);
    }

    #[test]
    fn every_side() {
        let input = base(300.0, 200.0); // âncora: top 200, bottom 240, left 300, right 400
        // acima: o fim do popup na borda de cima
        let p = at(input, (V::Top, H::Left), (V::Bottom, H::Left));
        assert_eq!((p.top, p.left), (200.0 - 100.0, 300.0));
        // à direita: começo do popup na borda direita, topos alinhados
        let p = at(input, (V::Top, H::Right), (V::Top, H::Left));
        assert_eq!((p.top, p.left), (200.0, 400.0));
        // à esquerda: fim do popup na borda esquerda
        let p = at(input, (V::Top, H::Left), (V::Top, H::Right));
        assert_eq!((p.top, p.left), (200.0, 300.0 - 200.0));
        // centralizado embaixo (tooltip): meio do popup no meio da âncora
        let p = at(input, (V::Bottom, H::Middle), (V::Top, H::Middle));
        assert_eq!((p.top, p.left), (240.0, 350.0 - 100.0));
        // centro e meio: popup centrado sobre a âncora
        let p = at(input, (V::Center, H::Middle), (V::Center, H::Middle));
        assert_eq!((p.top, p.left), (220.0 - 50.0, 350.0 - 100.0));
        // canto de baixo à direita da âncora, popup para fora
        let p = at(input, (V::Bottom, H::Right), (V::Top, H::Left));
        assert_eq!((p.top, p.left), (240.0, 400.0));
    }

    #[test]
    fn offset_expands_the_anchor_box() {
        let input = Input {
            offset: (10.0, 14.0),
            ..base(300.0, 200.0)
        };
        // abaixo: 240 + 14; esquerda: 300 - 10
        let p = compute(&input);
        assert_eq!((p.top, p.left), (254.0, 290.0));
        // acima: o fim do popup em 200 - 14
        let p = at(input, (V::Top, H::Left), (V::Bottom, H::Left));
        assert_eq!(p.top, 186.0 - 100.0);
        // à direita: 400 + 10
        let p = at(input, (V::Top, H::Right), (V::Top, H::Left));
        assert_eq!(p.left, 410.0);
    }

    #[test]
    fn flips_up_when_there_is_more_room_above() {
        // âncora no fundo: bottom = 780; abaixo sobram 20, acima 760
        let p = compute(&base(300.0, 740.0));
        assert!(p.flipped_vertical);
        assert_eq!(p.top, 740.0 - 100.0);
        assert_eq!(p.self_origin.vertical, V::Bottom);
        assert_eq!(p.anchor_origin.vertical, V::Top);
        // o popup (100) cabe acima (760): o teto é o espaço, não o tamanho
        assert_eq!(p.max_height, Some(740.0));
    }

    #[test]
    fn flips_down_when_requested_above_but_there_is_no_room() {
        // âncora no topo (top = 30): pedir acima (precisa de 100) não cabe
        let input = base(300.0, 30.0);
        let p = at(input, (V::Top, H::Left), (V::Bottom, H::Left));
        assert!(p.flipped_vertical);
        assert_eq!(p.self_origin.vertical, V::Top);
        assert_eq!(p.top, 70.0); // borda de baixo da âncora
        assert_eq!(p.max_height, Some(800.0 - 70.0));
    }

    #[test]
    fn keeps_the_requested_side_when_it_has_at_least_as_much_room() {
        // popup maior que a janela inteira: nenhum lado cabe; o pedido (abaixo) tem mais espaço
        // que o outro e fica, com teto igual ao espaço
        let input = Input {
            popup: Size {
                width: 200.0,
                height: 700.0,
            },
            ..base(300.0, 100.0)
        };
        let p = compute(&input);
        assert_eq!(p.top, 140.0);
        assert_eq!(p.max_height, Some(800.0 - 140.0));
        assert!(!p.flipped_vertical, "o lado pedido tinha mais espaço");
        assert_eq!(p.self_origin.vertical, V::Top);
    }

    #[test]
    fn caps_to_the_css_bound_when_it_is_smaller() {
        let input = Input {
            bound_max_height: Some(300.0),
            popup: Size {
                width: 200.0,
                height: 300.0,
            },
            ..base(300.0, 600.0)
        };
        // abaixo sobram 160; acima 600 → vira, teto = min(600, 300)
        let p = compute(&input);
        assert!(p.flipped_vertical);
        assert_eq!(p.max_height, Some(300.0));
        assert_eq!(p.top, 600.0 - 300.0);
    }

    #[test]
    fn the_effective_height_is_the_cap_when_the_cap_is_smaller_than_the_popup() {
        let input = Input {
            popup: Size {
                width: 200.0,
                height: 500.0,
            },
            ..base(300.0, 650.0)
        };
        // abaixo: 110 de espaço; acima: 650 → vira; altura efetiva = min(500, 650) = 500
        let p = compute(&input);
        assert!(p.flipped_vertical);
        assert_eq!(p.top, 650.0 - 500.0);
        assert_eq!(p.max_height, Some(650.0));
    }

    #[test]
    fn flips_left_when_there_is_no_room_on_the_right() {
        // âncora perto da borda direita: left 900, right 1000
        let input = base(900.0, 200.0);
        // pedido: popup à direita da âncora
        let p = at(input, (V::Top, H::Right), (V::Top, H::Left));
        assert!(p.flipped_horizontal);
        assert_eq!(p.self_origin.horizontal, H::Right);
        assert_eq!(p.anchor_origin.horizontal, H::Left);
        assert_eq!(p.left, 900.0 - 200.0);
        assert_eq!(p.max_width, Some(900.0));
        // o padrão (alinhado à esquerda da âncora, 900 + 200 > 1000) também vira: fim do popup
        // na borda direita da âncora
        let p = compute(&input);
        assert!(p.flipped_horizontal);
        assert_eq!(p.self_origin.horizontal, H::Right);
        assert_eq!(p.anchor_origin.horizontal, H::Right);
        assert_eq!(p.left, 1000.0 - 200.0);
    }

    #[test]
    fn flips_right_when_there_is_no_room_on_the_left() {
        let input = base(20.0, 200.0);
        // pedido: popup à esquerda da âncora (fim do popup na borda esquerda)
        let p = at(input, (V::Top, H::Left), (V::Top, H::Right));
        assert!(p.flipped_horizontal);
        assert_eq!(p.self_origin.horizontal, H::Left);
        assert_eq!(p.left, 120.0);
    }

    #[test]
    fn centered_popup_on_a_centered_anchor_is_clamped_to_the_window() {
        // âncora quase na borda esquerda: meio em 60; popup de 200 centralizado sairia em -40
        let input = base(10.0, 200.0);
        let p = at(input, (V::Bottom, H::Middle), (V::Top, H::Middle));
        assert_eq!(p.left, 0.0);
        assert!(!p.flipped_horizontal);
        // e na borda direita
        let input = base(890.0, 200.0);
        let p = at(input, (V::Bottom, H::Middle), (V::Top, H::Middle));
        assert_eq!(p.left, 1000.0 - 200.0);
        // vertical, mesmo critério
        let input = base(300.0, 10.0);
        let p = at(input, (V::Center, H::Right), (V::Center, H::Left));
        assert_eq!(p.top, 0.0);
    }

    #[test]
    fn fit_makes_the_popup_at_least_as_wide_as_the_anchor() {
        let input = Input {
            fit: true,
            popup: Size {
                width: 60.0,
                height: 100.0,
            },
            ..base(300.0, 200.0)
        };
        let p = compute(&input);
        assert_eq!(p.min_width, Some(100.0));
        assert_eq!(p.min_height, None);
        // popup mais largo que a âncora: o mínimo continua sendo a âncora
        let wide = Input {
            popup: Size {
                width: 300.0,
                height: 100.0,
            },
            ..input
        };
        assert_eq!(compute(&wide).min_width, Some(100.0));
        // o mínimo conta no cálculo de virada: com a âncora larga o popup passa da borda direita
        let big = Input {
            anchor: Rect {
                top: 200.0,
                left: 300.0,
                width: 600.0,
                height: 40.0,
            },
            popup: Size {
                width: 100.0,
                height: 100.0,
            },
            ..input
        };
        let p = at(big, (V::Bottom, H::Left), (V::Top, H::Left));
        assert_eq!(p.min_width, Some(600.0));
        assert!(!p.flipped_horizontal, "300 + 600 = 900 cabe em 1000");
        // `fit` não passa do teto do CSS
        let bounded = Input {
            bound_max_width: Some(250.0),
            ..big
        };
        assert_eq!(compute(&bounded).min_width, Some(250.0));
    }

    #[test]
    fn cover_sits_over_the_anchor_with_its_size_and_no_offset() {
        let input = Input {
            cover: true,
            offset: (30.0, 30.0),
            anchor_origin: Position::new(V::Center, H::Middle),
            // `self_` é ignorado com cover
            self_origin: Position::new(V::Bottom, H::Right),
            popup: Size {
                width: 40.0,
                height: 10.0,
            },
            ..base(300.0, 200.0)
        };
        let p = compute(&input);
        assert_eq!((p.min_width, p.min_height), (Some(100.0), Some(40.0)));
        // 100×40, centrado no centro da âncora (350, 220)
        assert_eq!((p.left, p.top), (300.0, 200.0));
        assert_eq!(p.self_origin, p.anchor_origin);
    }

    #[test]
    fn cover_with_a_top_left_origin_aligns_the_corners() {
        let input = Input {
            cover: true,
            anchor_origin: Position::new(V::Top, H::Left),
            offset: (50.0, 50.0),
            ..base(300.0, 200.0)
        };
        let p = compute(&input);
        assert_eq!((p.top, p.left), (200.0, 300.0));
    }

    #[test]
    fn a_popup_that_fits_gets_no_caps() {
        let p = compute(&base(300.0, 200.0));
        assert_eq!((p.max_height, p.max_width), (None, None));
        assert_eq!((p.min_width, p.min_height), (None, None));
    }

    #[test]
    fn horizontal_cap_combines_with_the_css_bound() {
        let input = Input {
            bound_max_width: Some(150.0),
            ..base(900.0, 200.0)
        };
        let p = at(input, (V::Top, H::Right), (V::Top, H::Left));
        assert!(p.flipped_horizontal);
        assert_eq!(p.max_width, Some(150.0));
    }
}
