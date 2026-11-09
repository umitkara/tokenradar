use tokenradar_i18n::{Domain, Locale, Localizer};

use super::*;
use crate::ipc::{AbsentLimit, SessionInfo};

const MIN: i64 = 60_000;
const DAY: i64 = 24 * 60 * MIN;
const NOW: i64 = 1_794_000_000_000;

const NO_FIVE_HOUR: &str = include_str!("../../../../../testdata/statusline/no-five-hour.json");

fn now() -> Timestamp {
    Timestamp::from_millis(NOW)
}

fn win(percent: f64, resets_in: i64) -> Window {
    Window {
        percent,
        resets_in: Some(resets_in),
        eta: None,
        elapsed: None,
        source: Source::Statusline,
        observed_at: now(),
    }
}

/// The approved default line of an account without a 5-hour window.
fn weekly_only() -> Line {
    let mut line = Line::sample(now(), false);
    line.seven_day = Some(win(41.0, 3 * DAY + 4 * 60 * MIN));
    line
}

fn with_five_hour() -> Line {
    Line::sample(now(), true)
}

fn prefs(segments: &[SegmentKind], layout: Layout, bar: bool) -> StatuslinePrefs {
    StatuslinePrefs::legacy(segments, layout, bar, false)
}

/// The default segments without the time mark: the 12c line, which most tests pin.
fn classic() -> StatuslinePrefs {
    prefs(&DEFAULT_SEGMENTS, Layout::Single, true)
}

/// A terminal on which the line may take `n` columns: Claude Code keeps four more free.
#[allow(clippy::unnecessary_wraps)] // the width `draw` takes
fn cols(n: usize) -> Option<usize> {
    Some(n + CLAUDE_CODE_RESERVED)
}

fn draw(locale: Locale, line: &Line, prefs: &StatuslinePrefs, width: Option<usize>) -> String {
    let l = Localizer::new(locale, &[Domain::Cli]);
    // Color on for the bar, then the escapes stripped to compare text.
    strip(&render(&l, line, prefs, Style { color: true, width }))
}

fn strip(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn the_default_line_without_a_five_hour_window() {
    let p = classic();
    assert_eq!(
        draw(Locale::En, &weekly_only(), &p, None),
        "7d ████░░░░ 41% · 3d4h │ Fable 22% │ ctx 38% │ ≈2.1% this session │ API $4.12"
    );
    assert_eq!(
        draw(Locale::Tr, &weekly_only(), &p, None),
        "7g ████░░░░ %41 · 3g4sa │ Fable %22 │ ctx %38 │ ≈%2,1 bu oturum │ API $4,12"
    );
}

#[test]
fn an_account_with_the_window_leads_with_it() {
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &classic(), None),
        "5h ██████░░ 72% · 1h48m ▲38m │ 7d ████░░░░ 41% · 3d4h │ Fable 22% │ ctx 38% │ \
         ≈2.1% this session │ API $4.12"
    );
}

#[test]
fn segments_and_order_follow_the_prefs() {
    use SegmentKind::{Context, Cost, Model, SevenDay};
    let p = prefs(&[Model, Cost, SevenDay, Context], Layout::Single, false);
    assert_eq!(
        draw(Locale::En, &weekly_only(), &p, None),
        "Opus 5.5 │ API $4.12 │ 7d 41% · 3d4h │ ctx 38%"
    );
}

#[test]
fn the_double_layout_puts_limits_first_and_the_session_second() {
    let mut segments = DEFAULT_SEGMENTS.to_vec();
    segments.push(SegmentKind::Model);
    let p = prefs(&segments, Layout::Double, true);
    assert_eq!(
        draw(Locale::En, &weekly_only(), &p, None),
        "7d ████░░░░ 41% · 3d4h │ Fable 22%\nctx 38% │ ≈2.1% this session │ API $4.12 │ Opus 5.5"
    );
    // A line with nothing on it is left out.
    let mut line = weekly_only();
    line.context = None;
    line.session_share = None;
    line.cost = None;
    line.model = None;
    assert_eq!(
        draw(Locale::En, &line, &p, None),
        "7d ████░░░░ 41% · 3d4h │ Fable 22%"
    );
}

#[test]
fn a_narrow_terminal_drops_session_segments_then_detail_then_limits() {
    let p = classic();
    let line = with_five_hour();
    let full = draw(Locale::En, &line, &p, None);
    let width = display_width(&full);
    assert_eq!(draw(Locale::En, &line, &p, cols(width)), full);
    // Session segments go first, from the end.
    assert_eq!(
        draw(Locale::En, &line, &p, cols(width - 1)),
        "5h ██████░░ 72% · 1h48m ▲38m │ 7d ████░░░░ 41% · 3d4h │ Fable 22% │ ctx 38% │ \
         ≈2.1% this session"
    );
    assert_eq!(
        draw(Locale::En, &line, &p, cols(70)),
        "5h ██████░░ 72% · 1h48m ▲38m │ 7d ████░░░░ 41% · 3d4h │ Fable 22%"
    );
    // Then the bar, the projection and the countdowns.
    assert_eq!(
        draw(Locale::En, &line, &p, cols(50)),
        "5h 72% · 1h48m ▲38m │ 7d 41% · 3d4h │ Fable 22%"
    );
    assert_eq!(
        draw(Locale::En, &line, &p, cols(30)),
        "5h 72% │ 7d 41% │ Fable 22%"
    );
    // Then limits from the end; the first always stays.
    assert_eq!(draw(Locale::Tr, &line, &p, cols(16)), "5s %72 │ 7g %41");
    assert_eq!(draw(Locale::Tr, &line, &p, cols(3)), "5s %72");
}

#[test]
fn without_color_the_bar_goes_and_no_escape_is_written() {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let out = render(
        &l,
        &weekly_only(),
        &classic(),
        Style {
            color: false,
            width: None,
        },
    );
    assert!(!out.contains('\x1b') && !out.contains('█'), "{out:?}");
    assert!(out.starts_with("7d 41% · 3d4h │ Fable 22%"), "{out:?}");
}

#[test]
fn colors_follow_the_thresholds() {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let mut line = with_five_hour();
    line.five_hour.as_mut().expect("5h").percent = 93.0;
    line.context.as_mut().expect("ctx").value = 80.0;
    let out = render(
        &l,
        &line,
        &classic(),
        Style {
            color: true,
            width: None,
        },
    );
    assert!(out.starts_with("\x1b[31m5h \x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[31m████████\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[31m93%\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[36m41%\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[2m░░░░\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[33m▲38m\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[33mctx 80%\x1b[0m"), "{out:?}");
    // The separator is an item: dim, the spaces around it plain (ADR-051).
    assert!(out.contains(" \x1b[2m│\x1b[0m "), "{out:?}");
    // A full bar has no empty part, and no empty escape pair is written for it.
    assert!(!out.contains("\x1b[2m\x1b[0m"), "{out:?}");
}

#[test]
fn the_bar_fills_in_eighths_and_any_use_shows() {
    assert_eq!(render::bar_cells(0.0, 8), 0);
    assert_eq!(render::bar_cells(0.1, 8), 1);
    assert_eq!(render::bar_cells(12.5, 8), 1);
    assert_eq!(render::bar_cells(41.0, 8), 4);
    assert_eq!(render::bar_cells(99.9, 8), 8);
    assert_eq!(render::bar_cells(140.0, 8), 8);
}

#[test]
fn estimates_carry_the_mark_and_stale_app_values_are_dim() {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let mut line = with_five_hour();
    line.five_hour = Some(Window {
        percent: 0.0,
        resets_in: None,
        eta: None,
        elapsed: None,
        source: Source::Estimate,
        observed_at: now(),
    });
    // An API reading eleven minutes old.
    line.seven_day.as_mut().expect("7d").observed_at = Timestamp::from_millis(NOW - 11 * MIN);
    line.seven_day.as_mut().expect("7d").source = Source::Api;
    let p = prefs(
        &[SegmentKind::FiveHour, SegmentKind::SevenDay],
        Layout::Single,
        true,
    );
    let out = render(
        &l,
        &line,
        &p,
        Style {
            color: true,
            width: None,
        },
    );
    assert_eq!(strip(&out), "≈5h ░░░░░░░░ 0% │ 7d ████░░░░ 41% · 3d4h");
    assert!(out.starts_with("\x1b[2m≈5h \x1b[0m"), "{out:?}");
    assert!(
        out.contains("\x1b[2m7d \x1b[0m\x1b[2m████\x1b[0m"),
        "{out:?}"
    );
    assert!(out.contains("\x1b[2m3d4h\x1b[0m"), "{out:?}");
    // Nine minutes is still fresh.
    line.seven_day.as_mut().expect("7d").observed_at = Timestamp::from_millis(NOW - 9 * MIN);
    let out = render(
        &l,
        &line,
        &p,
        Style {
            color: true,
            width: None,
        },
    );
    assert!(out.contains("\x1b[36m7d \x1b[0m"), "{out:?}");
}

#[test]
fn nothing_to_show_says_so() {
    let p = classic();
    assert_eq!(
        draw(Locale::En, &Line::empty(now()), &p, None),
        "no limit data yet"
    );
    assert_eq!(
        draw(Locale::Tr, &Line::empty(now()), &p, None),
        "henüz limit verisi yok"
    );
    // Session data without limits is still worth a line (an API-key user).
    let mut line = Line::empty(now());
    line.context = Some(Reading::now(5.0, Source::Statusline, now()));
    assert_eq!(draw(Locale::En, &line, &p, None), "ctx 5%");
}

fn app_limit(kind: &str, scope: Option<&str>, percent: f64, source: &str) -> LimitInfo {
    LimitInfo {
        kind: kind.into(),
        scope: scope.map(Into::into),
        percent,
        resets_at: Some(Timestamp::from_millis(NOW + 3 * 3_600_000)),
        source: source.into(),
        estimated: source == "estimate",
        observed_at: Timestamp::from_millis(NOW - MIN),
        freshness: "fresh".into(),
        eta_100_at: Some(Timestamp::from_millis(NOW + 30 * MIN)),
        will_hit_before_reset: true,
        projection_confidence: Some("medium".into()),
    }
}

fn session(share: Option<f64>) -> SessionInfo {
    SessionInfo {
        session_uuid: Some("s".into()),
        name: None,
        title: None,
        project: None,
        status: None,
        share_percent: share,
        cost_usd: 0.0,
        tokens_per_min: 0.0,
        compacts: None,
    }
}

#[test]
fn the_payload_wins_and_the_app_adds_scoped_projection_and_share() {
    let payload = StatuslinePayload::parse(
        br#"{"session_id":"s","model":{"id":"claude-opus-5-5","display_name":"Opus 5.5"},
            "cost":{"total_cost_usd":4.12},"context_window":{"used_percentage":38},
            "rate_limits":{"five_hour":{"used_percentage":50,"resets_at":1794007200}}}"#,
    )
    .expect("payload");
    let mut reply = StatusReply::empty(now());
    reply.limits = vec![
        app_limit("session", None, 40.0, "api"),
        app_limit("weekly_all", None, 20.0, "api"),
        app_limit("weekly_scoped", Some("model:Fable"), 22.0, "api"),
        app_limit("weekly_scoped", Some("model:Opus"), 9.0, "estimate"),
    ];
    reply.session = Some(session(Some(12.5)));
    let line = Line::from_sources(Some(&payload), Some(&reply), now());

    let five = line.five_hour.as_ref().expect("5h");
    assert!((five.percent - 50.0).abs() < 1e-9, "the payload's reading");
    assert_eq!(five.source, Source::Statusline);
    assert_eq!(five.observed_at, now());
    assert_eq!(five.resets_in, Some(2 * 3_600_000));
    assert_eq!(five.eta, Some(30 * MIN), "the app's projection");
    let seven = line.seven_day.as_ref().expect("7d from the app");
    assert!((seven.percent - 20.0).abs() < 1e-9);
    assert_eq!(seven.source, Source::Api);
    assert_eq!(seven.observed_at, Timestamp::from_millis(NOW - MIN));
    // Only the scoped measurement; the estimate is not shown.
    assert_eq!(line.scoped.len(), 1);
    assert_eq!(line.scoped[0].label, "Fable");
    assert_eq!(
        line.session_share,
        Some(Reading {
            value: 12.5,
            source: Source::Estimate,
            observed_at: now()
        })
    );
    assert_eq!(line.context.as_ref().map(|r| r.value), Some(38.0));
    assert_eq!(
        line.cost.as_ref().map(|r| r.source),
        Some(Source::Statusline)
    );
    assert_eq!(
        line.model.as_ref().map(|r| r.value.as_str()),
        Some("Opus 5.5")
    );
}

#[test]
fn without_the_app_only_payload_segments_show() {
    let payload = StatuslinePayload::parse(NO_FIVE_HOUR.as_bytes()).expect("payload");
    let line = Line::from_sources(Some(&payload), None, now());
    assert!(line.five_hour.is_none() && line.scoped.is_empty() && line.session_share.is_none());
    assert_eq!(
        draw(Locale::En, &line, &classic(), None),
        "7d ███░░░░░ 37.4% · 3d10h │ ctx 18% │ API $1.07"
    );

    // A passed reset is an estimated 0 %.
    let later = Timestamp::from_millis(1_794_297_599_000 + 1);
    let line = Line::from_sources(Some(&payload), None, later);
    let seven = line.seven_day.expect("7d");
    assert!(seven.estimated() && seven.percent == 0.0 && seven.resets_in.is_none());
}

#[test]
fn a_missing_window_never_falls_back_to_an_estimate_or_an_absent_window() {
    let payload = StatuslinePayload::parse(NO_FIVE_HOUR.as_bytes()).expect("payload");

    // The app's estimated 0 % after a reset is not shown.
    let mut reply = StatusReply::empty(now());
    reply.limits = vec![app_limit("session", None, 0.0, "estimate")];
    let line = Line::from_sources(Some(&payload), Some(&reply), now());
    assert_eq!(line.five_hour, None);

    // The account has no 5-hour window: no segment, whatever the app still holds.
    reply.limits = vec![app_limit("session", None, 12.0, "statusline")];
    reply.absent = vec![AbsentLimit {
        kind: "session".into(),
        scope: None,
    }];
    let line = Line::from_sources(Some(&payload), Some(&reply), now());
    assert_eq!(line.five_hour, None);

    // A measurement from the app (another session's reading) still fills the gap.
    reply.absent.clear();
    let line = Line::from_sources(Some(&payload), Some(&reply), now());
    assert_eq!(line.five_hour.map(|w| w.percent), Some(12.0));

    // The payload's own reading wins over "absent": the window came back.
    reply.absent = vec![AbsentLimit {
        kind: "session".into(),
        scope: None,
    }];
    let back = StatuslinePayload::parse(
        br#"{"rate_limits":{"five_hour":{"used_percentage":4,"resets_at":1794010000}}}"#,
    )
    .expect("payload");
    let line = Line::from_sources(Some(&back), Some(&reply), now());
    assert_eq!(line.five_hour.map(|w| w.percent), Some(4.0));
}

#[test]
fn the_display_name_is_read_but_never_forwarded() {
    let payload = StatuslinePayload::parse(NO_FIVE_HOUR.as_bytes()).expect("payload");
    let model = payload.model.as_ref().expect("model");
    assert_eq!(model.display_name.as_deref(), Some("Opus 5.5"));
    let json = serde_json::to_string(&payload).expect("json");
    assert!(
        json.contains("claude-opus-5-5") && !json.contains("Opus 5.5"),
        "{json}"
    );
}

#[test]
fn prefs_read_leniently() {
    use SegmentKind::{Context, Cost, SevenDay};
    let read = |toml: &str| -> StatuslinePrefs { toml::from_str(toml).expect("toml") };
    assert_eq!(read(""), StatuslinePrefs::default());
    let p = read(
        r#"segments = ["cost", "rainbow", "seven_day", "cost", "context"]
           layout = "double"
           bar = false"#,
    );
    assert_eq!(
        p,
        StatuslinePrefs::legacy(&[Cost, SevenDay, Context], Layout::Double, false, true)
    );
    let no_pace = read("pace = false");
    assert!(
        no_pace
            .items
            .iter()
            .filter(|i| i.kind.is_limit() && i.kind != SegmentKind::Scoped)
            .all(|i| !i.pace)
    );
    // Unknown layout, an empty list or only unknown names: the defaults.
    let p = read(
        r#"segments = ["rainbow"]
                    layout = "triple""#,
    );
    assert_eq!(p, StatuslinePrefs::default());
    assert_eq!(read("segments = []"), StatuslinePrefs::default());
}

#[test]
fn elapsed_time_comes_from_the_reset_and_the_window_length() {
    assert_eq!(elapsed(Some(WEEK_MS / 4), WEEK_MS), Some(75.0));
    assert_eq!(elapsed(Some(WEEK_MS), WEEK_MS), Some(0.0));
    // Unknown, passed or longer than the window: nothing to show.
    assert_eq!(elapsed(None, WEEK_MS), None);
    assert_eq!(elapsed(Some(0), WEEK_MS), None);
    assert_eq!(elapsed(Some(WEEK_MS + 1), WEEK_MS), None);
}

#[test]
fn the_time_mark_sits_between_cells_and_the_text_follows_the_value() {
    assert_eq!(render::mark_at(0.0, 8), 0);
    assert_eq!(render::mark_at(43.0, 8), 3);
    assert_eq!(render::mark_at(44.0, 8), 4);
    assert_eq!(render::mark_at(100.0, 8), 8);

    let p = StatuslinePrefs::legacy(&[SegmentKind::SevenDay], Layout::Single, true, true);
    let mut line = weekly_only();
    // 41 % used, 43 % of the week gone: the mark stands in the empty part.
    line.seven_day = Some(Window {
        elapsed: Some(43.0),
        ..win(41.0, 3 * DAY + 4 * 60 * MIN)
    });
    assert_eq!(
        draw(Locale::En, &line, &p, None),
        "7d ███│█░░░░ 41% ⏱ 43% · 3d4h"
    );
    assert_eq!(
        draw(Locale::Tr, &line, &p, None),
        "7g ███│█░░░░ %41 ⏱ %43 · 3g4sa"
    );
    // Ahead of time: the mark stands inside the filled part.
    line.seven_day = Some(Window {
        elapsed: Some(20.0),
        ..win(72.0, 3 * DAY)
    });
    assert_eq!(
        draw(Locale::En, &line, &p, None),
        "7d ██│████░░ 72% ⏱ 20% · 3d"
    );
    // Off in the prefs: neither the mark nor the text.
    let off = StatuslinePrefs::legacy(&[SegmentKind::SevenDay], Layout::Single, true, false);
    assert_eq!(draw(Locale::En, &line, &off, None), "7d ██████░░ 72% · 3d");
}

#[test]
fn without_color_the_elapsed_text_stays_and_narrow_drops_it_after_the_bar() {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let p = StatuslinePrefs::legacy(&[SegmentKind::SevenDay], Layout::Single, true, true);
    let mut line = weekly_only();
    line.seven_day = Some(Window {
        elapsed: Some(43.0),
        ..win(41.0, 3 * DAY + 4 * 60 * MIN)
    });
    let plain = render(
        &l,
        &line,
        &p,
        Style {
            color: false,
            width: None,
        },
    );
    assert_eq!(plain, "7d 41% ⏱ 43% · 3d4h");
    // Narrow: the bar goes first, then the elapsed time, then the countdown.
    assert_eq!(draw(Locale::En, &line, &p, cols(19)), "7d 41% ⏱ 43% · 3d4h");
    assert_eq!(draw(Locale::En, &line, &p, cols(18)), "7d 41% · 3d4h");
    assert_eq!(draw(Locale::En, &line, &p, cols(8)), "7d 41%");
}

#[test]
fn the_payload_window_gets_its_elapsed_time() {
    let payload: StatuslinePayload = serde_json::from_str(NO_FIVE_HOUR).expect("fixture");
    let line = Line::from_sources(Some(&payload), None, now());
    let w = line.seven_day.expect("7d");
    let expected = elapsed(w.resets_in, WEEK_MS);
    assert!(expected.is_some());
    assert_eq!(w.elapsed, expected);
}

mod layout;
mod v12m;
mod v12n;
mod v12p;
mod v3;
