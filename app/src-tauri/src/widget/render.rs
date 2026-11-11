//! Draws the widget strip (design-language §4, "Widget"): shapes with `tiny-skia`, text through
//! a [`Text`] engine (DirectWrite in the app, a deterministic stand-in in tests; ADR-058).
//!
//! ```text
//! 7g ███░░░░░░░ %34 · Çar
//! 5s ████████░░ %72 · 1:48
//! ```
//!
//! Docked on a short taskbar (Step 18b) the rows become one line, without reset texts:
//!
//! ```text
//! 7g %34 ███░░ · 5s %72 ████░
//! ```
//!
//! Every length is a logical pixel times the scale. Bars are whole pixels tall and sit on the
//! pixel grid, so they stay sharp at 100 %. Colors are the panel's tokens, so a bar means the
//! same thing in the widget and in the panel.

use tiny_skia::{BlendMode, Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Transform};
use tokenradar_core::limits::Severity;

use super::model::{Row, WidgetVisual};

/// Logical size: 220×44 with two rows, 220×28 with one (design-language §4).
pub const WIDTH: f32 = 220.0;
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 7.0;
const ROW_H: f32 = 15.0;
const ONE_ROW_H: f32 = 14.0;
const RADIUS: f32 = 8.0;
const BAR_H: f32 = 4.0;
const GAP_LABEL: f32 = 6.0;
const GAP_VALUE: f32 = 8.0;
const GAP_SEP: f32 = 4.0;
/// Room left after a critical fill, so the state does not rest on color alone (as the tray).
const NOTCH: f32 = 2.0;
const DASH: f32 = 6.0;
const DASH_GAP: f32 = 3.0;
const MARK_W: f32 = 2.0;
const MARK_OVER: f32 = 3.0;
const DOT_R: f32 = 2.5;
const LABEL_PX: f32 = 11.0;
const VALUE_PX: f32 = 12.0;
/// The compact line's bars, and the room around its `·` separators.
const COMPACT_BAR: f32 = 40.0;
const COMPACT_GAP: f32 = 8.0;
/// The hover highlight leaves this much of the taskbar's height free above and below, with
/// the corner radius of the taskbar's own buttons.
const HOVER_INSET: f32 = 4.0;
const HOVER_RADIUS: f32 = 4.0;

/// The colors the strip is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    /// High contrast: the system's window and text colors, shapes only.
    HighContrast {
        ink: [u8; 3],
        back: [u8; 3],
    },
}

/// How the strip is drawn, besides its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub theme: Theme,
    /// Effective DPI / 96, in hundredths (`150` = 150 %), so the style stays `Eq`.
    pub scale_pct: u32,
    /// Background opacity in percent; text and bars stay opaque.
    pub opacity: u8,
    pub shape: Shape,
}

/// What the strip stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// On the desktop: a rounded plate in the theme's surface color.
    Floating,
    /// On the taskbar (Step 18b): as tall as the taskbar (`height`, physical pixels), no
    /// plate, so the taskbar shows through. `compact` is the one-line layout of a short
    /// taskbar; `hover` draws the highlight of the taskbar's own buttons.
    Docked {
        height: u32,
        compact: bool,
        hover: bool,
    },
}

impl Style {
    #[allow(clippy::cast_precision_loss)]
    fn scale(self) -> f32 {
        self.scale_pct.max(50) as f32 / 100.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Medium,
}

/// A text run's font: size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Font {
    pub px: f32,
    pub weight: Weight,
}

/// Measures and draws text. Implementations must draw premultiplied, anti-aliased glyphs
/// over what is already in the pixmap.
pub trait Text {
    /// The advance width of `text`, in physical pixels.
    fn width(&mut self, text: &str, font: Font) -> f32;
    /// Draws `text` starting at `x` with its baseline at `baseline`.
    fn draw(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        font: Font,
        x: f32,
        baseline: f32,
        color: Color,
    );
}

/// sRGB values of the design tokens in `tokens.css` (`tests::palette_matches_tokens`).
struct Palette {
    surface: [u8; 3],
    stroke: Color,
    text1: Color,
    text2: Color,
    text3: Color,
    track: Color,
    ok: Color,
    warn: Color,
    crit: Color,
    /// `--signal-est`, which is `--text-2`.
    est: Color,
    /// High contrast: the track is an outline, so filled and empty differ by shape.
    outline: bool,
    /// The docked hover highlight: the subtle fill of `WinUI` (`SubtleFillColorSecondary`).
    hover: Color,
}

impl Palette {
    fn of(theme: Theme) -> Self {
        let rgb = |r, g, b| Color::from_rgba8(r, g, b, 255);
        match theme {
            Theme::Dark => Self {
                surface: [19, 22, 26],
                stroke: Color::from_rgba8(255, 255, 255, 23),
                text1: rgb(239, 242, 245),
                text2: rgb(179, 184, 190),
                text3: rgb(148, 153, 158),
                // --track is oklch(0.42 0.01 250 / 0.6): composed over the surface here, so the
                // background's opacity does not thin it out.
                track: rgb(51, 56, 60),
                ok: rgb(58, 223, 215),
                warn: rgb(253, 190, 69),
                crit: rgb(255, 111, 105),
                est: rgb(179, 184, 190),
                outline: false,
                hover: Color::from_rgba8(255, 255, 255, 15),
            },
            Theme::Light => Self {
                surface: [243, 245, 248],
                stroke: Color::from_rgba8(0, 0, 0, 20),
                text1: rgb(23, 27, 31),
                text2: rgb(73, 78, 82),
                text3: rgb(95, 100, 105),
                track: rgb(204, 210, 215),
                ok: rgb(0, 117, 124),
                warn: rgb(158, 88, 0),
                crit: rgb(190, 34, 42),
                est: rgb(73, 78, 82),
                outline: false,
                hover: Color::from_rgba8(0, 0, 0, 10),
            },
            Theme::HighContrast {
                ink: [r, g, b],
                back,
            } => {
                let ink = rgb(r, g, b);
                Self {
                    surface: back,
                    stroke: ink,
                    text1: ink,
                    text2: ink,
                    text3: ink,
                    track: ink,
                    ok: ink,
                    warn: ink,
                    crit: ink,
                    est: ink,
                    outline: true,
                    hover: Color::TRANSPARENT,
                }
            }
        }
    }

    fn severity(&self, s: Severity) -> Color {
        match s {
            Severity::Ok => self.ok,
            Severity::Warn => self.warn,
            Severity::Crit => self.crit,
        }
    }
}

/// The strip's size in physical pixels. Floating, it depends on the number of rows; docked,
/// it is as tall as the taskbar, and the compact line as wide as its texts.
pub fn size(visual: &WidgetVisual, style: Style, text: &mut dyn Text) -> (u32, u32) {
    let s = style.scale();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = |v: f32| (v * s).round().max(1.0) as u32;
    match style.shape {
        Shape::Floating => {
            let h = if visual.rows.len() > 1 {
                PAD_Y * 2.0 + ROW_H * 2.0
            } else {
                PAD_Y * 2.0 + ONE_ROW_H
            };
            (px(WIDTH), px(h))
        }
        Shape::Docked {
            height,
            compact: false,
            ..
        } => (px(WIDTH), height.max(1)),
        Shape::Docked {
            height,
            compact: true,
            ..
        } => {
            let cols = Columns::of(visual, s, text);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let w = cols.compact_width(visual.rows.len(), s).ceil().max(1.0) as u32;
            (w, height.max(1))
        }
    }
}

/// Column widths shared by the rows, so the bars line up and do not jump as values change.
struct Columns {
    label: f32,
    value: f32,
    sep: f32,
    reset: f32,
}

impl Columns {
    fn of(visual: &WidgetVisual, s: f32, text: &mut dyn Text) -> Self {
        let (label_font, value_font) = fonts(s);
        let widest = |items: &mut dyn Iterator<Item = &str>, font, text: &mut dyn Text| {
            items.map(|t| text.width(t, font)).fold(0.0_f32, f32::max)
        };
        Self {
            label: widest(
                &mut visual.rows.iter().map(|r| r.label.as_str()),
                label_font,
                text,
            ),
            value: widest(
                &mut visual
                    .rows
                    .iter()
                    .map(|r| r.value.as_str())
                    .chain(std::iter::once(visual.widest_value.as_str())),
                value_font,
                text,
            ),
            sep: text.width("·", label_font),
            reset: widest(
                &mut visual.reset_samples.iter().map(String::as_str),
                label_font,
                text,
            ),
        }
    }

    /// One compact item: label, value, bar.
    fn compact_item(&self, s: f32) -> f32 {
        self.label + GAP_LABEL * s + self.value + GAP_VALUE * s + COMPACT_BAR * s
    }

    #[allow(clippy::cast_precision_loss)]
    fn compact_width(&self, rows: usize, s: f32) -> f32 {
        let n = rows.max(1) as f32;
        PAD_X * 2.0 * s + n * self.compact_item(s) + (n - 1.0) * (COMPACT_GAP * 2.0 * s + self.sep)
    }
}

fn fonts(s: f32) -> (Font, Font) {
    (
        Font {
            px: LABEL_PX * s,
            weight: Weight::Regular,
        },
        Font {
            px: VALUE_PX * s,
            weight: Weight::Medium,
        },
    )
}

/// Renders `visual` as premultiplied RGBA.
pub fn render(visual: &WidgetVisual, style: Style, text: &mut dyn Text) -> Pixmap {
    let (w, h) = size(visual, style, text);
    let mut pixmap = Pixmap::new(w, h).expect("non-zero widget size");
    let pal = Palette::of(style.theme);
    let s = style.scale();
    #[allow(clippy::cast_precision_loss)]
    let (wf, hf) = (w as f32, h as f32);

    let ground = ground(&mut pixmap, &pal, style, wf, hf, s);
    let cols = Columns::of(visual, s, text);
    let compact = matches!(style.shape, Shape::Docked { compact: true, .. });
    let painted = Painted {
        pal: &pal,
        cols: &cols,
        s,
        clear_halo: !ground.plate,
    };

    if compact {
        let mut x = PAD_X * s;
        for (i, row) in visual.rows.iter().enumerate() {
            if i > 0 {
                let (label_font, _) = fonts(s);
                let baseline = (hf / 2.0 + VALUE_PX * s * 0.36).round();
                text.draw(
                    &mut pixmap,
                    "·",
                    label_font,
                    x + COMPACT_GAP * s,
                    baseline,
                    pal.text2,
                );
                x += COMPACT_GAP * 2.0 * s + cols.sep;
            }
            draw_compact(&mut pixmap, row, &painted, x, hf / 2.0, text);
            x += cols.compact_item(s);
        }
    } else {
        let rows = visual.rows.len();
        for (i, row) in visual.rows.iter().enumerate() {
            // Rows are ROW_H apart, around the middle: for the floating plate this is
            // PAD_Y + ROW_H * (i + 0.5), for the taskbar the middle of its height.
            #[allow(clippy::cast_precision_loss)]
            let center = hf / 2.0 + (i as f32 - (rows as f32 - 1.0) / 2.0) * ROW_H * s;
            draw_row(&mut pixmap, row, &painted, center, wf, text);
        }
    }

    if visual.error {
        let r = DOT_R * s;
        if let Some(dot) = PathBuilder::from_circle(wf - r - 3.0 * s, ground.top + r + 3.0 * s, r) {
            fill(&mut pixmap, &dot, pal.crit);
        }
    }
    pixmap
}

/// What a row is drawn with.
struct Painted<'a> {
    pal: &'a Palette,
    cols: &'a Columns,
    s: f32,
    /// Without a plate, the time mark's halo cuts the bar open instead of painting the
    /// surface color: the taskbar shows through, whatever its color.
    clear_halo: bool,
}

/// The ground's outcome: whether there is a plate, and where its top edge is.
struct Ground {
    plate: bool,
    top: f32,
}

#[allow(clippy::many_single_char_names)] // sizes and channels
fn ground(pixmap: &mut Pixmap, pal: &Palette, style: Style, w: f32, h: f32, s: f32) -> Ground {
    let (top, height, hover) = match style.shape {
        Shape::Floating => (0.0, h, false),
        Shape::Docked { hover, .. } => {
            let inset = (HOVER_INSET * s).round().min(h / 4.0);
            (inset, h - 2.0 * inset, hover)
        }
    };
    // High contrast keeps its opaque plate on the taskbar too: the system colors only promise
    // contrast against each other, not against the taskbar.
    let plate = matches!(style.shape, Shape::Floating) || pal.outline;
    if !plate {
        // Almost invisible but not zero: a layered window lets fully clear pixels click
        // through, and the docked strip must still take hover, clicks and drags.
        pixmap.fill(Color::from_rgba8(0, 0, 0, 1));
        if hover && let Some(shape) = rounded(0.0, top, w, height, HOVER_RADIUS * s) {
            fill(pixmap, &shape, pal.hover);
        }
        return Ground { plate, top };
    }
    let radius = match style.shape {
        Shape::Floating => RADIUS,
        Shape::Docked { .. } => HOVER_RADIUS,
    } * s;
    let alpha = if pal.outline {
        255
    } else {
        u8::try_from(u32::from(style.opacity.min(100)) * 255 / 100).unwrap_or(255)
    };
    let [r, g, b] = pal.surface;
    if let Some(shape) = rounded(0.0, top, w, height, radius) {
        fill(pixmap, &shape, Color::from_rgba8(r, g, b, alpha));
    }
    // A hairline edge keeps the strip apart from a background of its own color.
    let line = s.round().max(1.0);
    let half = line / 2.0;
    if let Some(edge) = rounded(half, top + half, w - line, height - line, radius - half) {
        let mut paint = Paint::default();
        paint.set_color(pal.stroke);
        paint.anti_alias = true;
        let stroke = tiny_skia::Stroke {
            width: line,
            ..tiny_skia::Stroke::default()
        };
        pixmap.stroke_path(&edge, &paint, &stroke, Transform::identity(), None);
    }
    Ground { plate, top }
}

/// One item of the compact line, starting at `x`: `7g %34 ███░░`.
fn draw_compact(
    pixmap: &mut Pixmap,
    row: &Row,
    p: &Painted<'_>,
    x: f32,
    center: f32,
    text: &mut dyn Text,
) {
    let s = p.s;
    let (label_font, value_font) = fonts(s);
    let baseline = (center + VALUE_PX * s * 0.36).round();
    text.draw(pixmap, &row.label, label_font, x, baseline, p.pal.text2);
    let x_value_end = x + p.cols.label + GAP_LABEL * s + p.cols.value;
    let vw = text.width(&row.value, value_font);
    let value_color = if row.stale { p.pal.text3 } else { p.pal.text1 };
    text.draw(
        pixmap,
        &row.value,
        value_font,
        x_value_end - vw,
        baseline,
        value_color,
    );
    let bar_x0 = (x_value_end + GAP_VALUE * s).round();
    let bar_x1 = (bar_x0 + COMPACT_BAR * s).round();
    layered_bar(pixmap, row, p, bar_x0, bar_x1, center);
}

fn draw_row(
    pixmap: &mut Pixmap,
    row: &Row,
    p: &Painted<'_>,
    center: f32,
    w: f32,
    text: &mut dyn Text,
) {
    let (pal, cols, s) = (p.pal, p.cols, p.s);
    let (label_font, value_font) = fonts(s);
    // Digits are about 0.7 em tall: this baseline centers them on the bar.
    let baseline = (center + VALUE_PX * s * 0.36).round();

    let x_label = PAD_X * s;
    text.draw(pixmap, &row.label, label_font, x_label, baseline, pal.text2);

    let reset_col = (GAP_SEP * 2.0) * s + cols.sep + cols.reset;
    let x_value_end = w - PAD_X * s - reset_col;
    let bar_x0 = (x_label + cols.label + GAP_LABEL * s).round();
    let bar_x1 = (x_value_end - cols.value - GAP_VALUE * s).round();

    let value_color = if row.stale { pal.text3 } else { pal.text1 };
    let vw = text.width(&row.value, value_font);
    text.draw(
        pixmap,
        &row.value,
        value_font,
        x_value_end - vw,
        baseline,
        value_color,
    );
    if let Some(reset) = &row.reset {
        let x_sep = x_value_end + GAP_SEP * s;
        text.draw(pixmap, "·", label_font, x_sep, baseline, pal.text2);
        let x_reset = x_sep + cols.sep + GAP_SEP * s;
        text.draw(pixmap, reset, label_font, x_reset, baseline, pal.text2);
    }
    layered_bar(pixmap, row, p, bar_x0, bar_x1, center);
}

/// The bar on a layer of its own, so a stale bar fades as a whole and the mark's halo only
/// cuts the bar.
fn layered_bar(pixmap: &mut Pixmap, row: &Row, p: &Painted<'_>, x0: f32, x1: f32, center: f32) {
    if x1 <= x0 {
        return;
    }
    let mut layer = Pixmap::new(pixmap.width(), pixmap.height()).expect("same size");
    bar(&mut layer, row, p, x0, x1, center);
    if row.stale {
        // Premultiplied pixels: halving every channel is exactly 50 % opacity (as the tray).
        for b in layer.data_mut() {
            *b /= 2;
        }
    }
    pixmap.draw_pixmap(
        0,
        0,
        layer.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

#[allow(clippy::many_single_char_names)] // geometry: x, y, h, r, s
fn bar(pixmap: &mut Pixmap, row: &Row, p: &Painted<'_>, x0: f32, x1: f32, center: f32) {
    let (pal, s) = (p.pal, p.s);
    let h = (BAR_H * s).round().max(2.0);
    let y = (center - h / 2.0).round();
    let len = x1 - x0;
    let r = h / 2.0;
    let fill_to = x0 + len * row.fill;
    let notched = row.severity == Some(Severity::Crit) && !pal.outline;
    let track_from = if notched {
        (fill_to + NOTCH * s).min(x1)
    } else {
        x0
    };

    if pal.outline {
        let line = s.round().max(1.0);
        let half = line / 2.0;
        if let Some(outline) = rounded(x0 + half, y + half, len - line, h - line, r - half) {
            let mut paint = Paint::default();
            paint.set_color(pal.track);
            paint.anti_alias = true;
            let stroke = tiny_skia::Stroke {
                width: line,
                ..tiny_skia::Stroke::default()
            };
            pixmap.stroke_path(&outline, &paint, &stroke, Transform::identity(), None);
        }
    } else if track_from < x1
        && let Some(track) = rounded(track_from, y, x1 - track_from, h, r)
    {
        fill(pixmap, &track, pal.track);
    }

    if let Some(severity) = row.severity
        && row.fill > 0.0
    {
        let full = rounded(x0, y, fill_to - x0, h, r);
        if row.estimate {
            // Dashes clipped to the rounded fill, as the panel's estimate bar.
            let mut mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height()).expect("size");
            if let Some(full) = &full {
                mask.fill_path(full, FillRule::Winding, true, Transform::identity());
            }
            let (dash, gap) = ((DASH * s).round(), (DASH_GAP * s).round());
            let mut x = x0;
            let mut pb = PathBuilder::new();
            while x < fill_to {
                if let Some(rect) = Rect::from_xywh(x, y, dash.min(fill_to - x), h) {
                    pb.push_rect(rect);
                }
                x += dash + gap;
            }
            if let Some(dashes) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(pal.est);
                paint.anti_alias = true;
                pixmap.fill_path(
                    &dashes,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    Some(&mask),
                );
            }
        } else if let Some(full) = full {
            fill(pixmap, &full, pal.severity(severity));
        }
    }

    if let Some(mark) = row.mark {
        let mw = (MARK_W * s).round().max(1.0);
        let x = (x0 + len * mark - mw / 2.0).round().clamp(x0, x1 - mw);
        let over = (MARK_OVER * s).round();
        let (top, height) = (y - over, h + 2.0 * over);
        // A halo in the background color keeps the mark apart from the fill under it.
        let halo = s.round().max(1.0);
        if let Some(rect) =
            Rect::from_xywh(x - halo, top - halo, mw + 2.0 * halo, height + 2.0 * halo)
        {
            let area = PathBuilder::from_rect(rect);
            if p.clear_halo {
                let paint = Paint {
                    blend_mode: BlendMode::Clear,
                    ..Paint::default()
                };
                pixmap.fill_path(
                    &area,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            } else {
                let [r, g, b] = pal.surface;
                fill(pixmap, &area, Color::from_rgba8(r, g, b, 255));
            }
        }
        if let Some(rect) = Rect::from_xywh(x, top, mw, height) {
            fill(pixmap, &PathBuilder::from_rect(rect), pal.text1);
        }
    }
}

/// A rectangle with round corners; `None` when it has no area.
#[allow(clippy::many_single_char_names)] // the textbook names of a rounded rectangle
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    // The cubic approximation of a quarter circle.
    let k = r * 0.552_284_8;
    let (x1, y1) = (x + w, y + h);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x1 - r, y);
    pb.cubic_to(x1 - r + k, y, x1, y + r - k, x1, y + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x + r, y1);
    pb.cubic_to(x + r - k, y1, x, y1 - r + k, x, y1 - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

fn fill(pixmap: &mut Pixmap, path: &Path, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    pixmap.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
}

/// Premultiplied BGRA, as `UpdateLayeredWindow` expects. `tiny-skia` is premultiplied RGBA
/// already, so only the channel order changes (no demultiply, unlike the tray's `to_rgba`).
pub fn to_bgra(pixmap: &Pixmap, out: &mut [u8]) {
    let (src, _) = pixmap.data().as_chunks::<4>();
    let (dst, _) = out.as_chunks_mut::<4>();
    for (s, d) in src.iter().zip(dst) {
        *d = [s[2], s[1], s[0], s[3]];
    }
}

#[cfg(test)]
mod tests;
