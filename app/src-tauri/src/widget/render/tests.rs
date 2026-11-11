use std::path::PathBuf;

use tiny_skia::Pixmap;
use tokenradar_core::limits::Severity;

use super::*;
use crate::paint::oklch_to_srgb8;
use crate::widget::model::{Row, WidgetVisual};

/// Glyphs as boxes: deterministic on every machine, so the golden files cover the layout and
/// the shapes. DirectWrite itself is checked by `text::tests` on Windows.
pub(crate) struct BoxText;

const ADVANCE: f32 = 0.55;

impl Text for BoxText {
    fn width(&mut self, text: &str, font: Font) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let n = text.chars().count() as f32;
        n * ADVANCE * font.px
    }

    fn draw(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        font: Font,
        x: f32,
        baseline: f32,
        color: Color,
    ) {
        let adv = ADVANCE * font.px;
        let height = font.px
            * if font.weight == Weight::Medium {
                0.72
            } else {
                0.66
            };
        let mut pb = PathBuilder::new();
        for (i, c) in (0u16..).zip(text.chars()) {
            if c.is_whitespace() {
                continue;
            }
            let x = x + f32::from(i) * adv + adv * 0.15;
            if let Some(r) = Rect::from_xywh(x, baseline - height, adv * 0.7, height) {
                pb.push_rect(r);
            }
        }
        if let Some(path) = pb.finish() {
            fill(pixmap, &path, color);
        }
    }
}

fn row(label: &str, percent: u8, reset: &str) -> Row {
    Row {
        label: label.into(),
        value: format!("%{percent}"),
        reset: Some(reset.into()),
        fill: f32::from(percent) / 100.0,
        severity: Some(Severity::of(f64::from(percent))),
        estimate: false,
        stale: false,
        mark: None,
    }
}

fn visual(rows: Vec<Row>) -> WidgetVisual {
    WidgetVisual {
        rows,
        error: false,
        widest_value: "≈%100".into(),
        reset_samples: vec!["00:00".into(), "Çar".into()],
    }
}

/// The design's cases (single row, both rows, stale, estimate) plus critical with the time
/// mark, the error dot and no data.
fn fixtures() -> Vec<(&'static str, WidgetVisual)> {
    let weekly = row("7g", 34, "Çar");
    let five = row("5s", 72, "1:48");
    vec![
        ("single", visual(vec![weekly.clone()])),
        ("both", visual(vec![weekly.clone(), five.clone()])),
        (
            "stale",
            visual(vec![
                Row {
                    stale: true,
                    ..weekly.clone()
                },
                Row {
                    stale: true,
                    ..five.clone()
                },
            ]),
        ),
        (
            "estimate",
            visual(vec![
                weekly.clone(),
                Row {
                    estimate: true,
                    value: "≈%72".into(),
                    ..five.clone()
                },
            ]),
        ),
        (
            "crit-mark",
            visual(vec![
                Row {
                    mark: Some(0.6),
                    ..row("7g", 93, "Çar")
                },
                Row {
                    mark: Some(0.4),
                    ..row("5s", 80, "0:12")
                },
            ]),
        ),
        (
            "error",
            WidgetVisual {
                error: true,
                ..visual(vec![weekly])
            },
        ),
        (
            "no-data",
            visual(vec![Row {
                value: "–".into(),
                reset: None,
                fill: 0.0,
                severity: None,
                ..five
            }]),
        ),
    ]
}

fn themes() -> [(&'static str, Theme); 3] {
    [
        ("dark", Theme::Dark),
        ("light", Theme::Light),
        (
            "hc",
            Theme::HighContrast {
                ink: [255, 255, 255],
                back: [0, 0, 0],
            },
        ),
    ]
}

fn style(theme: Theme, scale_pct: u32) -> Style {
    Style {
        theme,
        scale_pct,
        opacity: 85,
    }
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/widget")
}

/// As the tray: antialiasing may differ by a few levels between SIMD paths.
const TOLERANCE: u8 = 24;

/// `testdata/widget/<theme>-<case>-<scale>.png`; `UPDATE_GOLDEN=1` rewrites them.
#[test]
fn widget_strips_match_golden_files() {
    let update = std::env::var_os("UPDATE_GOLDEN").is_some_and(|v| v == "1");
    let dir = golden_dir();
    if update {
        std::fs::create_dir_all(&dir).expect("golden dir");
    }
    let mut failures = Vec::new();
    for (theme_name, theme) in themes() {
        for (case, v) in fixtures() {
            for scale in [100, 150] {
                let name = format!("{theme_name}-{case}-{scale}.png");
                let path = dir.join(&name);
                let got = render(&v, style(theme, scale), &mut BoxText);
                if update {
                    std::fs::write(&path, got.encode_png().expect("encode")).expect("write");
                    continue;
                }
                let Ok(want) = Pixmap::load_png(&path) else {
                    failures.push(format!("{name}: missing (run with UPDATE_GOLDEN=1)"));
                    continue;
                };
                if let Some(diff) = difference(&got, &want) {
                    failures.push(format!("{name}: {diff}"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "golden mismatches:\n{}",
        failures.join("\n")
    );
}

/// Compares straight RGBA, since a PNG round trip re-premultiplies.
fn difference(got: &Pixmap, want: &Pixmap) -> Option<String> {
    if (got.width(), got.height()) != (want.width(), want.height()) {
        return Some(format!(
            "size {}x{} != {}x{}",
            got.width(),
            got.height(),
            want.width(),
            want.height()
        ));
    }
    let straight = |p: &Pixmap| -> Vec<u8> {
        p.pixels()
            .iter()
            .flat_map(|c| {
                let c = c.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect()
    };
    let (a, b) = (straight(got), straight(want));
    let worst = a
        .iter()
        .zip(&b)
        .enumerate()
        .max_by_key(|(_, (x, y))| x.abs_diff(**y))?;
    let delta = worst.1.0.abs_diff(*worst.1.1);
    (delta > TOLERANCE).then(|| {
        let px = worst.0 / 4;
        let w = got.width() as usize;
        format!("pixel ({}, {}) differs by {delta}", px % w, px / w)
    })
}

#[test]
fn widget_size_follows_rows_and_scale() {
    let s = |pct| style(Theme::Dark, pct);
    assert_eq!(size(2, s(100)), (220, 44));
    assert_eq!(size(1, s(100)), (220, 28));
    assert_eq!(size(2, s(150)), (330, 66));
    assert_eq!(size(1, s(125)), (275, 35));
}

fn rgb_at(p: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let c = p.pixel(x, y).expect("in bounds").demultiply();
    [c.red(), c.green(), c.blue(), c.alpha()]
}

/// The bar's row: the middle of a single-row strip.
fn bar_y(p: &Pixmap) -> u32 {
    p.height() / 2
}

#[test]
fn widget_fill_and_track_use_the_panel_tokens() {
    let p = render(
        &visual(vec![row("7g", 50, "Çar")]),
        style(Theme::Dark, 100),
        &mut BoxText,
    );
    let y = bar_y(&p);
    // Scan the bar row: the fill (ok) comes first, then the track.
    let colors: Vec<[u8; 4]> = (0..p.width()).map(|x| rgb_at(&p, x, y)).collect();
    let first_ok = colors.iter().position(|c| c[..3] == [58, 223, 215]);
    let first_track = colors.iter().position(|c| c[..3] == [51, 56, 60]);
    assert!(
        matches!((first_ok, first_track), (Some(a), Some(b)) if a < b),
        "{first_ok:?} {first_track:?}"
    );
}

#[test]
fn widget_background_opacity_spares_the_bars() {
    let mut st = style(Theme::Light, 100);
    st.opacity = 40;
    let p = render(&visual(vec![row("7g", 50, "Çar")]), st, &mut BoxText);
    // A background pixel well inside the corner radius, and a fill pixel.
    assert_eq!(rgb_at(&p, 4, 14)[3], 102);
    let y = bar_y(&p);
    let fill = (0..p.width())
        .map(|x| rgb_at(&p, x, y))
        .find(|c| c[..3] == [0, 117, 124]);
    assert_eq!(fill.map(|c| c[3]), Some(255));
}

#[test]
fn widget_critical_bars_leave_a_notch_after_the_fill() {
    let count_track = |percent| {
        let p = render(
            &visual(vec![row("7g", percent, "Çar")]),
            style(Theme::Dark, 100),
            &mut BoxText,
        );
        let y = bar_y(&p);
        (0..p.width())
            .filter(|&x| rgb_at(&p, x, y)[..3] == [51, 56, 60])
            .count()
    };
    // 89 % and 91 % fill almost the same; the critical one loses track pixels to the notch.
    assert!(count_track(91) + 2 <= count_track(89));
}

#[test]
fn widget_stale_bars_are_half_transparent() {
    let fresh = render(
        &visual(vec![row("7g", 50, "Çar")]),
        style(Theme::Dark, 100),
        &mut BoxText,
    );
    let stale = render(
        &visual(vec![Row {
            stale: true,
            ..row("7g", 50, "Çar")
        }]),
        style(Theme::Dark, 100),
        &mut BoxText,
    );
    let y = bar_y(&fresh);
    let x = (0..fresh.width())
        .find(|&x| rgb_at(&fresh, x, y)[..3] == [58, 223, 215])
        .expect("a fill pixel");
    assert_ne!(rgb_at(&fresh, x, y), rgb_at(&stale, x, y));
}

#[test]
fn widget_bgra_swaps_channels_without_demultiplying() {
    let mut p = Pixmap::new(1, 1).expect("pixmap");
    p.fill(Color::from_rgba8(200, 100, 50, 128));
    let src = p.data().to_vec();
    let mut out = [0u8; 4];
    to_bgra(&p, &mut out);
    assert_eq!(out, [src[2], src[1], src[0], src[3]]);
}

/// The hard-coded sRGB values match the OKLCH tokens of `tokens.css`.
#[test]
fn widget_palette_matches_tokens() {
    let dark = Palette::of(Theme::Dark);
    let light = Palette::of(Theme::Light);
    let cases: [((f32, f32, f32), Color); 14] = [
        ((0.96, 0.005, 250.0), dark.text1),
        ((0.78, 0.01, 250.0), dark.text2),
        ((0.68, 0.01, 250.0), dark.text3),
        ((0.82, 0.13, 190.0), dark.ok),
        ((0.84, 0.15, 80.0), dark.warn),
        ((0.72, 0.18, 25.0), dark.crit),
        ((0.22, 0.01, 250.0), light.text1),
        ((0.42, 0.01, 250.0), light.text2),
        ((0.50, 0.01, 250.0), light.text3),
        ((0.86, 0.01, 250.0), light.track),
        ((0.50, 0.11, 200.0), light.ok),
        ((0.53, 0.13, 65.0), light.warn),
        ((0.52, 0.19, 25.0), light.crit),
        ((0.42, 0.01, 250.0), light.est),
    ];
    for ((l, c, h), color) in cases {
        let got = color.to_color_u8();
        assert_eq!(
            [got.red(), got.green(), got.blue()],
            oklch_to_srgb8(l, c, h),
            "oklch({l} {c} {h})"
        );
    }
    assert_eq!(dark.surface, oklch_to_srgb8(0.20, 0.01, 250.0));
    assert_eq!(light.surface, oklch_to_srgb8(0.97, 0.005, 250.0));
}
