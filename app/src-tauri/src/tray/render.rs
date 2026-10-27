//! Draws the tray icon: the dual ring (design-language §4, "Tray ikonu"), without a
//! `WebView`.
//!
//! Outer ring = weekly limit, inner ring = 5-hour window. Both start at 12 o'clock and fill
//! clockwise. Ring widths are whole pixels and the centre sits on a pixel corner, so ring edges
//! land on the pixel grid and stay sharp at 16 px.

use std::f32::consts::{FRAC_PI_2, TAU};

use tiny_skia::{BlendMode, Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Transform};
use tokenradar_core::limits::Severity;

use super::visual::{Ring, TrayVisual};

/// The colours the icon is drawn with, picked by the taskbar's theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Dark taskbar (`SystemUsesLightTheme = 0`, the Windows default).
    Dark,
    Light,
    /// High-contrast mode: one ink colour (the system's window text), shapes only.
    HighContrast {
        ink: [u8; 3],
    },
}

struct Palette {
    track: Color,
    /// High contrast: the track is a thin line instead of a band, so filled and empty
    /// differ by shape, not by shade.
    thin_track: bool,
    ok: Color,
    warn: Color,
    crit: Color,
    /// The "no data" centre dot.
    dot: Color,
    /// The pause mark.
    mark: Color,
}

/// sRGB values of the design tokens (design-language §2); `tests::palette_matches_tokens`
/// recomputes them from OKLCH.
impl Palette {
    fn of(theme: Theme) -> Self {
        let rgb = |r, g, b| Color::from_rgba8(r, g, b, 255);
        match theme {
            Theme::Dark => Self {
                // --track is oklch(0.40 0.01 250 / 0.5) on Mica; on the dark taskbar half
                // alpha all but vanishes, which also hides the critical notch. Opaque here.
                track: rgb(68, 72, 77),
                thin_track: false,
                ok: rgb(58, 223, 215),
                warn: rgb(253, 190, 69),
                crit: rgb(255, 100, 95),
                dot: rgb(169, 175, 180),
                mark: rgb(239, 242, 245),
            },
            Theme::Light => Self {
                track: rgb(201, 206, 212),
                thin_track: false,
                ok: rgb(0, 143, 144),
                warn: rgb(203, 127, 0),
                crit: rgb(208, 43, 49),
                dot: rgb(81, 86, 91),
                mark: rgb(23, 27, 31),
            },
            Theme::HighContrast { ink: [r, g, b] } => {
                let ink = rgb(r, g, b);
                Self {
                    track: ink,
                    thin_track: true,
                    ok: ink,
                    warn: ink,
                    crit: ink,
                    dot: ink,
                    mark: ink,
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

/// Pixel sizes for one icon size. 16 px: outer 2, gap 1, inner 3 (design-language §4); larger
/// sizes scale and round to whole pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    center: f32,
    outer_r: f32,
    outer_w: f32,
    inner_r: f32,
    inner_w: f32,
    /// Width of the critical notch, in pixels along the ring.
    notch: f32,
    /// Diameter of the error dot.
    dot_d: f32,
    /// Transparent margin around the corner marks.
    halo: f32,
    /// One bar of the pause mark.
    bar_w: f32,
    bar_h: f32,
}

impl Geometry {
    fn of(px: u32) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let size = px as f32;
        let s = size / 16.0;
        let outer_w = (2.0 * s).round().max(2.0);
        let gap = s.round().max(1.0);
        let inner_w = (3.0 * s).round().max(3.0);
        let center = size / 2.0;
        Self {
            center,
            outer_r: center,
            outer_w,
            inner_r: center - outer_w - gap,
            inner_w,
            notch: (1.5 * s).max(1.5),
            dot_d: (5.0 * s).round().max(4.0),
            halo: s.round().max(1.0),
            bar_w: s.round().max(1.0),
            bar_h: (5.0 * s).round().max(4.0),
        }
    }

    fn hole_r(&self) -> f32 {
        self.inner_r - self.inner_w
    }
}

/// Renders `visual` as a `px`×`px` icon. Returns premultiplied pixels; see [`to_rgba`].
pub fn render(visual: TrayVisual, px: u32, theme: Theme) -> Pixmap {
    let px = px.max(8);
    let mut pixmap = Pixmap::new(px, px).expect("non-zero icon size");
    let g = Geometry::of(px);
    let pal = Palette::of(theme);

    ring(
        &mut pixmap,
        &pal,
        g.center,
        g.outer_r,
        g.outer_w,
        g.notch,
        visual.weekly,
    );
    ring(
        &mut pixmap,
        &pal,
        g.center,
        g.inner_r,
        g.inner_w,
        g.notch,
        visual.five_hour,
    );

    if visual.weekly.is_none() && visual.five_hour.is_none() {
        let r = (g.hole_r() * 0.5).max(1.0);
        if let Some(path) = PathBuilder::from_circle(g.center, g.center, r) {
            fill(&mut pixmap, &path, pal.dot);
        }
    }

    if visual.stale {
        // Premultiplied pixels: halving every channel is exactly 50 % opacity.
        for b in pixmap.data_mut() {
            *b /= 2;
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let size = px as f32;
    if visual.error {
        let r = g.dot_d / 2.0;
        let (cx, cy) = (size - r, r);
        if let Some(halo) = PathBuilder::from_circle(cx, cy, r + g.halo) {
            clear(&mut pixmap, &halo);
        }
        if let Some(dot) = PathBuilder::from_circle(cx, cy, r) {
            fill(&mut pixmap, &dot, pal.crit);
        }
    }
    if visual.paused {
        // Two bars with a bar-wide gap, flush with the bottom-right corner.
        let w = g.bar_w * 3.0;
        let (x, y) = (size - w, size - g.bar_h);
        if let Some(halo) = Rect::from_xywh(x - g.halo, y - g.halo, w + g.halo, g.bar_h + g.halo) {
            clear(&mut pixmap, &PathBuilder::from_rect(halo));
        }
        for bx in [x, x + 2.0 * g.bar_w] {
            if let Some(bar) = Rect::from_xywh(bx, y, g.bar_w, g.bar_h) {
                fill(&mut pixmap, &PathBuilder::from_rect(bar), pal.mark);
            }
        }
    }
    pixmap
}

/// Straight (non-premultiplied) RGBA, as `tauri::image::Image` expects.
pub fn to_rgba(pixmap: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.data().len());
    for p in pixmap.pixels() {
        let c = p.demultiply();
        out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

/// One ring: its track, then the filled arc. A critical ring gets a transparent notch right
/// after the arc's tip, so the state does not depend on colour alone (design-language §2).
fn ring(
    pixmap: &mut Pixmap,
    pal: &Palette,
    center: f32,
    r_out: f32,
    width: f32,
    notch_px: f32,
    value: Option<Ring>,
) {
    let r_in = r_out - width;
    let mid = r_out - width / 2.0;
    let fill_to = value.map_or(0.0, |v| f32::from(v.percent) / 100.0);
    let notched = value.is_some_and(|v| v.severity == Severity::Crit);
    let notch = notch_px / (TAU * mid);

    // Where the track starts: under the whole ring, or after the notch.
    let (track_from, fill_to) = if !notched {
        (0.0, fill_to)
    } else if fill_to + notch <= 1.0 {
        (fill_to + notch, fill_to)
    } else {
        // Full or nearly full: the notch sits at 12 o'clock and the arc ends before it.
        (1.0, 1.0 - notch)
    };

    if track_from < 1.0 {
        let track = if pal.thin_track {
            // One whole pixel inside the band, so the line stays crisp.
            let edge = r_out - (width / 2.0).floor();
            sector(center, edge, edge - 1.0, track_from, 1.0)
        } else {
            sector(center, r_out, r_in, track_from, 1.0)
        };
        if let Some(track) = track {
            fill(pixmap, &track, pal.track);
        }
    }
    if let Some(v) = value
        && fill_to > 0.0
        && let Some(arc) = sector(center, r_out, r_in, 0.0, fill_to)
    {
        fill(pixmap, &arc, pal.severity(v.severity));
    }
}

/// The ring segment between radii `r_in..r_out` from turn `from` to turn `to` (0 = 12
/// o'clock, clockwise). A whole turn is an annulus.
fn sector(center: f32, r_out: f32, r_in: f32, from: f32, to: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    if to - from >= 1.0 - f32::EPSILON {
        pb.push_circle(center, center, r_out);
        pb.push_circle(center, center, r_in);
        return pb.finish();
    }
    let (a0, a1) = (from * TAU, to * TAU);
    let (x, y) = point(center, r_out, a0);
    pb.move_to(x, y);
    arc(&mut pb, center, r_out, a0, a1);
    let (x, y) = point(center, r_in, a1);
    pb.line_to(x, y);
    arc(&mut pb, center, r_in, a1, a0);
    pb.close();
    pb.finish()
}

/// A point at angle `a` (radians from 12 o'clock, clockwise in screen coordinates).
fn point(center: f32, r: f32, a: f32) -> (f32, f32) {
    (center + r * a.sin(), center - r * a.cos())
}

/// Appends a circular arc from angle `from` to `to` as cubic Béziers of at most 90° each.
#[allow(clippy::many_single_char_names)] // the textbook names of the arc formula
fn arc(pb: &mut PathBuilder, center: f32, r: f32, from: f32, to: f32) {
    let sweep = to - from;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (sweep.abs() / FRAC_PI_2).ceil().max(1.0) as u32;
    #[allow(clippy::cast_precision_loss)]
    let step = sweep / n as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
    for i in 0..n {
        #[allow(clippy::cast_precision_loss)]
        let a = from + step * i as f32;
        let b = a + step;
        let (x0, y0) = point(center, r, a);
        let (x3, y3) = point(center, r, b);
        // The tangent of (sin a, -cos a) is (cos a, sin a).
        pb.cubic_to(
            x0 + k * a.cos(),
            y0 + k * a.sin(),
            x3 - k * b.cos(),
            y3 - k * b.sin(),
            x3,
            y3,
        );
    }
}

fn fill(pixmap: &mut Pixmap, path: &Path, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    pixmap.fill_path(path, &paint, FillRule::EvenOdd, Transform::identity(), None);
}

fn clear(pixmap: &mut Pixmap, path: &Path) {
    let paint = Paint {
        blend_mode: BlendMode::Clear,
        anti_alias: true,
        ..Paint::default()
    };
    pixmap.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
}

#[cfg(test)]
mod tests;
