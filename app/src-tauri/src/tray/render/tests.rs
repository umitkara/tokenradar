use std::path::PathBuf;

use tiny_skia::{FilterQuality, Pixmap, PixmapPaint};

use super::*;
use crate::tray::visual::TrayState;

const SIZES: [u32; 4] = [16, 20, 24, 32];
const HC_INK: [u8; 3] = [255, 255, 255];

fn themes() -> [(&'static str, Theme); 3] {
    [
        ("dark", Theme::Dark),
        ("light", Theme::Light),
        ("hc", Theme::HighContrast { ink: HC_INK }),
    ]
}

#[allow(clippy::unnecessary_wraps)] // fills `Option<Ring>` fields
fn ring(percent: u8) -> Option<Ring> {
    Some(Ring {
        percent,
        severity: Severity::of(f64::from(percent)),
    })
}

/// The seven states of step 08 plus both partial-data cases.
fn fixtures() -> Vec<(&'static str, TrayVisual)> {
    let ok = TrayVisual {
        weekly: ring(34),
        five_hour: ring(42),
        ..TrayVisual::default()
    };
    vec![
        ("no-data", TrayVisual::default()),
        ("ok", ok),
        (
            "warn",
            TrayVisual {
                weekly: ring(60),
                five_hour: ring(80),
                ..ok
            },
        ),
        (
            "crit",
            TrayVisual {
                weekly: ring(91),
                five_hour: ring(93),
                ..ok
            },
        ),
        ("stale", TrayVisual { stale: true, ..ok }),
        ("error", TrayVisual { error: true, ..ok }),
        ("paused", TrayVisual { paused: true, ..ok }),
        ("five-hour-only", TrayVisual { weekly: None, ..ok }),
        (
            "weekly-only",
            TrayVisual {
                five_hour: None,
                ..ok
            },
        ),
    ]
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/tray")
}

/// Largest per-channel difference that still counts as equal. Antialiasing may differ by a
/// few levels between CPUs (SIMD paths) without being visible.
const TOLERANCE: u8 = 24;

/// `testdata/tray/<theme>-<state>-<px>.png`; `UPDATE_GOLDEN=1` rewrites them.
#[test]
fn tray_icons_match_golden_files() {
    let update = std::env::var_os("UPDATE_GOLDEN").is_some_and(|v| v == "1");
    let dir = golden_dir();
    let mut failures = Vec::new();
    for (theme_name, theme) in themes() {
        for (state, visual) in fixtures() {
            for px in SIZES {
                let name = format!("{theme_name}-{state}-{px}.png");
                let path = dir.join(&name);
                let got = render(visual, px, theme);
                if update {
                    let png = got.encode_png().expect("encode");
                    std::fs::write(&path, png).expect("write golden");
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
    let (a, b) = (to_rgba(got), to_rgba(want));
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
fn tray_fixtures_cover_every_state() {
    let states: Vec<TrayState> = fixtures().iter().map(|(_, v)| v.state()).collect();
    for s in [
        TrayState::NoData,
        TrayState::Ok,
        TrayState::Warn,
        TrayState::Crit,
        TrayState::Stale,
        TrayState::Error,
        TrayState::Paused,
    ] {
        assert!(states.contains(&s), "{s:?}");
    }
}

fn alpha_at(p: &Pixmap, x: u32, y: u32) -> u8 {
    p.pixel(x, y).expect("in bounds").alpha()
}

#[test]
fn tray_no_data_draws_tracks_and_a_centre_dot() {
    let p = render(TrayVisual::default(), 16, Theme::Light);
    // Outer track at 12 o'clock, inner track below it, and the centre dot.
    assert_eq!(alpha_at(&p, 8, 1), 255);
    assert_eq!(alpha_at(&p, 8, 4), 255);
    assert!(alpha_at(&p, 8, 8) > 0);
}

#[test]
fn tray_rings_fill_clockwise_from_twelve() {
    let half = TrayVisual {
        weekly: ring(50),
        five_hour: ring(50),
        ..TrayVisual::default()
    };
    let p = render(half, 32, Theme::Dark);
    let ok = p.pixel(28, 16).expect("in bounds").demultiply();
    let track = p.pixel(3, 16).expect("in bounds").demultiply();
    // Right half (3 o'clock) is filled with --signal-ok, left half (9 o'clock) is track.
    assert_eq!((ok.red(), ok.green(), ok.blue()), (58, 223, 215));
    assert_eq!((track.red(), track.green(), track.blue()), (68, 72, 77));
}

#[test]
fn tray_critical_rings_get_a_notch() {
    let crit = |percent| TrayVisual {
        five_hour: ring(percent),
        ..TrayVisual::default()
    };
    // At 89 % the track continues right after the arc; at 91 % a gap separates them.
    let warn = render(crit(89), 32, Theme::Light);
    let notched = render(crit(91), 32, Theme::Light);
    let opaque = |p: &Pixmap| p.pixels().iter().filter(|c| c.alpha() == 255).count();
    assert!(opaque(&notched) < opaque(&warn));
}

#[test]
fn tray_stale_icons_are_half_transparent() {
    let fresh = TrayVisual {
        weekly: ring(30),
        ..TrayVisual::default()
    };
    let stale = TrayVisual {
        stale: true,
        ..fresh
    };
    let (a, b) = (
        render(fresh, 16, Theme::Light),
        render(stale, 16, Theme::Light),
    );
    assert_eq!(alpha_at(&a, 8, 1), 255);
    assert_eq!(alpha_at(&b, 8, 1), 127);
}

#[test]
fn tray_corner_marks_stay_opaque_on_stale_icons() {
    let v = TrayVisual {
        weekly: ring(30),
        stale: true,
        error: true,
        paused: true,
        ..TrayVisual::default()
    };
    let p = render(v, 16, Theme::Dark);
    // Centre of the error dot and the right pause bar.
    assert_eq!(alpha_at(&p, 13, 2), 255);
    assert_eq!(alpha_at(&p, 15, 14), 255);
}

#[test]
fn tray_ring_widths_follow_the_design_at_16_px() {
    let g = Geometry::of(16);
    assert_eq!((g.outer_w, g.inner_w), (2.0, 3.0));
    assert_eq!(g.outer_r - g.outer_w - g.inner_r, 1.0, "gap");
    for px in SIZES {
        let g = Geometry::of(px);
        assert!(g.hole_r() >= 2.0, "{px}: hole {}", g.hole_r());
        for w in [g.outer_w, g.inner_w, g.inner_r] {
            assert_eq!(w.fract(), 0.0, "{px}: whole pixels");
        }
    }
}

#[test]
fn tray_rgba_is_straight_alpha() {
    let p = render(
        TrayVisual {
            stale: true,
            weekly: ring(70),
            ..TrayVisual::default()
        },
        16,
        Theme::Dark,
    );
    let rgba = to_rgba(&p);
    // (8, 1): fully covered outer ring at 50 % opacity keeps its full colour.
    let i = (16 + 8) * 4;
    assert_eq!(&rgba[i..i + 4], &[58, 223, 215, 127]);
}

/// The hard-coded sRGB values match the OKLCH tokens of design-language §2.
#[test]
fn tray_palette_matches_tokens() {
    let cases: [((f32, f32, f32), Color); 8] = [
        ((0.82, 0.13, 190.0), Palette::of(Theme::Dark).ok),
        ((0.84, 0.15, 80.0), Palette::of(Theme::Dark).warn),
        ((0.70, 0.19, 25.0), Palette::of(Theme::Dark).crit),
        ((0.58, 0.12, 195.0), Palette::of(Theme::Light).ok),
        ((0.66, 0.15, 70.0), Palette::of(Theme::Light).warn),
        ((0.56, 0.20, 25.0), Palette::of(Theme::Light).crit),
        ((0.40, 0.01, 250.0), Palette::of(Theme::Dark).track),
        ((0.85, 0.01, 250.0), Palette::of(Theme::Light).track),
    ];
    for ((l, c, h), color) in cases {
        let want = oklch_to_srgb8(l, c, h);
        let got = color.to_color_u8();
        assert_eq!(
            [got.red(), got.green(), got.blue()],
            want,
            "oklch({l} {c} {h})"
        );
    }
}

#[allow(clippy::many_single_char_names)] // the names of the OKLab formulas
fn oklch_to_srgb8(l: f32, c: f32, h: f32) -> [u8; 3] {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let l_ = (l + 0.396_337_8 * a + 0.215_803_76 * b).powi(3);
    let m_ = (l - 0.105_561_35 * a - 0.063_854_17 * b).powi(3);
    let s_ = (l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    let lin = [
        4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_,
        -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_,
    ];
    lin.map(|x| {
        let x = x.clamp(0.0, 1.0);
        let e = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (e * 255.0).round() as u8;
        v
    })
}

/// Visual review aid: `cargo test -p tokenradar-app tray_contact_sheet -- --ignored` writes
/// `target/tray-<theme>.png` with every state and size, 8× enlarged on a taskbar-like
/// background, plus an actual-size row.
#[test]
#[ignore = "writes review images"]
fn tray_contact_sheet() {
    const SCALE: f32 = 8.0;
    const CELL: u32 = 32 * 8 + 16;
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    for (name, theme) in themes() {
        let bg = match theme {
            Theme::Dark => Color::from_rgba8(32, 32, 32, 255),
            Theme::Light => Color::from_rgba8(238, 238, 238, 255),
            Theme::HighContrast { .. } => Color::BLACK,
        };
        let fx = fixtures();
        #[allow(clippy::cast_possible_truncation)]
        let cols = fx.len() as u32;
        let rows = u32::try_from(SIZES.len()).expect("four sizes");
        let mut sheet = Pixmap::new(cols * CELL, rows * CELL + 48).expect("sheet");
        sheet.fill(bg);
        let nearest = PixmapPaint {
            quality: FilterQuality::Nearest,
            ..PixmapPaint::default()
        };
        for (col, (_, visual)) in (0u32..).zip(&fx) {
            let mut x_small = 0;
            for (row, px) in (0u32..).zip(SIZES) {
                let icon = render(*visual, px, theme);
                #[allow(clippy::cast_precision_loss)]
                let t = Transform::from_scale(SCALE, SCALE)
                    .post_translate((col * CELL + 8) as f32, (row * CELL + 8) as f32);
                sheet.draw_pixmap(0, 0, icon.as_ref(), &nearest, t, None);
                #[allow(clippy::cast_possible_wrap)]
                sheet.draw_pixmap(
                    (col * CELL + 8 + x_small) as i32,
                    (rows * CELL + 8) as i32,
                    icon.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
                x_small += px + 6;
            }
        }
        let path = out_dir.join(format!("tray-{name}.png"));
        std::fs::write(&path, sheet.encode_png().expect("encode")).expect("write sheet");
        eprintln!("wrote {}", path.display());
    }
}

/// Step 08 budget: < 0.2 ms per icon. Run in release:
/// `cargo test -p tokenradar-app --release tray_render_speed -- --ignored --nocapture`.
#[test]
#[ignore = "measurement"]
fn tray_render_speed() {
    const N: u32 = 1000;
    for px in SIZES {
        let fx = fixtures();
        let started = std::time::Instant::now();
        let mut bytes = 0usize;
        for i in 0..N {
            let (_, visual) = &fx[i as usize % fx.len()];
            let icon = render(*visual, px, Theme::Dark);
            bytes += to_rgba(&icon).len();
        }
        let per = started.elapsed() / N;
        eprintln!("{px} px: {per:?} per icon (render + RGBA), {bytes} bytes total");
        assert!(per.as_micros() < 200, "{px} px: {per:?}");
    }
}
