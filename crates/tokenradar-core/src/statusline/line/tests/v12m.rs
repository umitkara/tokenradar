//! Step 12m: new items (daily budget, burn rate, session name, cache ratio, output style), the
//! `remaining` and `tokens_left` formats, the reset as a clock time, bar widths and styles.

use super::*;
use crate::statusline::line::{BarStyle, Format, Item, Preset, ResetFormat, SampleState};

const SAMPLE: &str = include_str!("../../../../../../testdata/statusline/sample.json");

fn alone(item: Item) -> StatuslinePrefs {
    StatuslinePrefs {
        items: vec![item],
        ..StatuslinePrefs::default()
    }
    .normalized()
}

fn one(line: &Line, item: Item) -> String {
    draw(Locale::En, line, &alone(item), None)
}

fn ansi(line: &Line, item: Item) -> String {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let style = Style {
        color: true,
        width: None,
    };
    render(&l, line, &alone(item), style)
}

fn weekly(percent: f64, resets_in: i64) -> Line {
    let mut line = Line::sample(now(), false);
    line.seven_day = Some(Window {
        elapsed: elapsed(Some(resets_in), WEEK_MS),
        ..win(percent, resets_in)
    });
    line
}

#[test]
fn the_daily_budget_shares_the_room_left_over_the_days_to_the_reset() {
    let budget = || Item::new(SegmentKind::DailyBudget);
    // 59 points over four started days.
    assert_eq!(
        one(&weekly(41.0, 3 * DAY + 4 * 60 * MIN), budget()),
        "15%/day"
    );
    // Below ten points a decimal stays; less than a day left counts as one.
    assert_eq!(one(&weekly(81.2, 6 * DAY + MIN), budget()), "2.7%/day");
    assert_eq!(one(&weekly(41.0, 2 * 60 * MIN), budget()), "59%/day");
    // Nothing left: zero, critical.
    let spent = weekly(100.0, 2 * DAY);
    assert_eq!(one(&spent, budget()), "0%/day");
    assert!(ansi(&spent, budget()).contains("\x1b[31m"));
    // No weekly window, or no reset time: hidden.
    let mut none = weekly(41.0, DAY);
    none.seven_day = None;
    assert_eq!(one(&none, budget()), "no limit data yet");
    let mut unknown = weekly(41.0, DAY);
    unknown.seven_day.as_mut().expect("weekly").resets_in = None;
    assert_eq!(unknown.daily_budget(), None);
    // The source is the window's.
    let api = Line {
        seven_day: Some(Window {
            source: Source::Api,
            ..win(41.0, DAY)
        }),
        ..weekly(0.0, DAY)
    };
    assert_eq!(api.daily_budget().expect("budget").source, Source::Api);
}

#[test]
fn the_daily_budget_warns_ahead_of_the_time_line_and_dims_when_stale() {
    let budget = || Item::new(SegmentKind::DailyBudget);
    // Four days left of seven: 3/7 ≈ 43 % of the time gone. 60 % used is ahead by 17 points.
    let ahead = weekly(60.0, 4 * DAY);
    assert_eq!(
        ahead.daily_budget().expect("budget").severity,
        Severity::Warn
    );
    assert!(ansi(&ahead, budget()).contains("\x1b[33m"));
    // 45 % used: within ten points of the time line.
    let even = weekly(45.0, 4 * DAY);
    assert_eq!(even.daily_budget().expect("budget").severity, Severity::Ok);
    // Past the critical threshold the window's color wins.
    let crit = weekly(93.0, 4 * DAY);
    assert_eq!(
        crit.daily_budget().expect("budget").severity,
        Severity::Crit
    );
    // The first seventh of the week is quiet (ADR-045).
    let early = weekly(30.0, 7 * DAY - 60 * MIN);
    assert_eq!(early.daily_budget().expect("budget").severity, Severity::Ok);
    // An app value older than FRESH_MS: dim.
    let stale = Line::sample_in(now(), false, SampleState::Stale);
    assert!(ansi(&stale, budget()).contains("\x1b[2m"));
}

#[test]
fn the_burn_rate_is_cost_over_time_after_five_minutes() {
    let rate = || Item::new(SegmentKind::BurnRate);
    // $4.12 over 1,234,567 ms (20.6 minutes).
    assert_eq!(one(&with_five_hour(), rate()), "API $12.01/h");
    let mut short = with_five_hour();
    short.facts.duration_ms = Some(BURN_RATE_MIN_MS - 1);
    assert_eq!(short.burn_rate(), None);
    short.facts.duration_ms = None;
    assert_eq!(short.burn_rate(), None);
    let mut free = with_five_hour();
    free.cost = None;
    assert_eq!(free.burn_rate(), None);
    // Two measurements: no `≈`, the cost's source.
    let r = with_five_hour().burn_rate().expect("rate");
    assert_eq!(r.source, Source::Statusline);
}

#[test]
fn session_name_output_style_and_cache_ratio_come_from_the_payload() {
    let p = crate::statusline::payload::StatuslinePayload::parse(SAMPLE.as_bytes()).expect("parse");
    let line = Line::from_sources(Some(&p), None, now());
    assert_eq!(line.facts.session_name.as_deref(), Some("synthetic sample"));
    assert_eq!(line.facts.output_style.as_deref(), Some("default"));
    // 67,500 read of 8,500 + 5,000 + 67,500.
    assert_eq!(one(&line, Item::new(SegmentKind::CacheRatio)), "cache 83%");
    assert_eq!(line.facts.context_used, Some(81_000));
    assert_eq!(
        one(&line, Item::new(SegmentKind::SessionName)),
        "synthetic sample"
    );
    assert_eq!(one(&line, Item::new(SegmentKind::OutputStyle)), "default");
    // Nothing read from the cache, or no counts at all.
    let zero = br#"{"context_window":{"current_usage":{"input_tokens":0}}}"#;
    let p = crate::statusline::payload::StatuslinePayload::parse(zero).expect("parse");
    let line = Line::from_sources(Some(&p), None, now());
    assert_eq!(line.facts.cache_ratio, None);
    assert_eq!(line.facts.context_used, Some(0));
    let line = Line::from_sources(None, None, now());
    assert_eq!(line.facts.cache_ratio, None);
    assert_eq!(
        one(&line, Item::new(SegmentKind::CacheRatio)),
        "no limit data yet"
    );
}

#[test]
fn remaining_and_tokens_left_show_what_is_left() {
    let line = with_five_hour();
    assert_eq!(
        one(
            &line,
            Item::new(SegmentKind::SevenDay).with_format(Format::Remaining)
        ),
        "7d 59% left ⏱ 54.8% · 3d4h"
    );
    assert_eq!(
        one(
            &line,
            Item::new(SegmentKind::Context).with_format(Format::Remaining)
        ),
        "ctx 62% left"
    );
    assert_eq!(
        one(
            &line,
            Item::new(SegmentKind::Context).with_format(Format::TokensLeft)
        ),
        "ctx 124k left"
    );
    assert_eq!(
        one(
            &line,
            Item::new(SegmentKind::Scoped).with_format(Format::Remaining)
        ),
        "Fable 78% left"
    );
    // Without the last request's counts, from the used percentage: 62 % of 200k.
    let mut rough = with_five_hour();
    rough.facts.context_used = None;
    assert_eq!(rough.tokens_left(), Some(124_000));
    // Over-full never goes below zero.
    let mut full = with_five_hour();
    full.facts.context_used = Some(250_000);
    assert_eq!(full.tokens_left(), Some(0));
    // The colors follow what is used: 93 % used is critical, shown as 7 % left.
    let crit = Line::sample_in(now(), true, SampleState::Crit);
    let out = ansi(
        &crit,
        Item::new(SegmentKind::SevenDay).with_format(Format::Remaining),
    );
    assert!(out.contains("\x1b[31m7% left"), "{out:?}");
    // In Turkish.
    let p = alone(Item::new(SegmentKind::Context).with_format(Format::Remaining));
    assert_eq!(draw(Locale::Tr, &line, &p, None), "ctx %62 kaldı");
}

#[test]
fn the_reset_can_be_a_local_clock_time() {
    let mut line = with_five_hour();
    line.zone = Zone::Fixed(180);
    let clock = |kind| {
        let mut item = Item::new(kind);
        item.reset_format = ResetFormat::Clock;
        item.pace = false;
        item.format = Some(Format::Percent);
        item
    };
    assert_eq!(
        one(&line, clock(SegmentKind::SevenDay)),
        "7d 41% · Tue 04:20"
    );
    let p = alone(clock(SegmentKind::FiveHour));
    assert_eq!(
        draw(Locale::Tr, &line, &p, None),
        "5s %72 · Cmt 02:08 ▲≈38dk"
    );
    // Narrow: the clock goes with the countdowns, before the limit itself.
    let p = alone(clock(SegmentKind::SevenDay));
    assert_eq!(draw(Locale::En, &line, &p, cols(10)), "7d 41%");
}

#[test]
fn the_system_zone_gives_a_clock_on_windows_and_a_countdown_elsewhere() {
    let line = with_five_hour();
    let got = line.reset_clock(DAY);
    if cfg!(windows) {
        let c = got.expect("a local time");
        assert!(c.weekday < 7 && c.hour < 24 && c.minute < 60);
    } else {
        assert_eq!(got, None);
        let mut item = Item::new(SegmentKind::SevenDay).with_format(Format::Percent);
        item.reset_format = ResetFormat::Clock;
        item.pace = false;
        assert_eq!(one(&line, item), "7d 41% · 3d4h");
    }
}

#[test]
fn bars_take_their_width_and_style() {
    let line = with_five_hour();
    let bar = |width: u8, style: BarStyle| {
        let mut item = Item::new(SegmentKind::SevenDay);
        item.bar_width = width;
        item.bar_style = style;
        item.reset = false;
        item
    };
    assert_eq!(one(&line, bar(4, BarStyle::Blocks)), "7d ██│░░ 41% ⏱ 54.8%");
    assert_eq!(
        one(&line, bar(12, BarStyle::Thin)),
        "7d ▬▬▬▬▬▭▭|▭▭▭▭▭ 41% ⏱ 54.8%"
    );
    assert_eq!(
        one(&line, bar(8, BarStyle::Dots)),
        "7d ⣿⣿⣿⣿|⣀⣀⣀⣀ 41% ⏱ 54.8%"
    );
    let mut ctx = Item::new(SegmentKind::Context).with_format(Format::Bar);
    ctx.bar_width = 12;
    ctx.bar_style = BarStyle::Dots;
    assert_eq!(one(&line, ctx), "ctx ⣿⣿⣿⣿⣿⣀⣀⣀⣀⣀⣀⣀ 38%");
}

#[test]
fn a_narrow_line_shortens_a_wide_bar_before_dropping_it() {
    let line = weekly_only();
    let mut item = Item::new(SegmentKind::SevenDay);
    item.bar_width = 12;
    item.reset = false;
    item.pace = false;
    let p = alone(item);
    let wide = draw(Locale::En, &line, &p, None);
    assert_eq!(wide, "7d █████░░░░░░░ 41%");
    let w = display_width(&wide);
    assert_eq!(draw(Locale::En, &line, &p, cols(w)), wide);
    assert_eq!(draw(Locale::En, &line, &p, cols(w - 1)), "7d ████░░░░ 41%");
    assert_eq!(draw(Locale::En, &line, &p, cols(w - 5)), "7d 41%");
}

#[test]
fn the_new_bar_glyphs_take_one_cell_in_every_terminal() {
    use unicode_width::UnicodeWidthChar;
    for style in [BarStyle::Thin, BarStyle::Dots] {
        let (full, empty, mark) = style.glyphs();
        for c in [full, empty, mark] {
            assert_eq!(c.width(), Some(1), "{c:?}");
            // Not East Asian "Ambiguous": one cell in a CJK terminal too.
            assert_eq!(c.width_cjk(), Some(1), "{c:?} is ambiguous");
        }
    }
}

#[test]
fn new_fields_read_leniently_and_round_trip() {
    let read = |items: &str| {
        let text = format!("version = 2\n{items}");
        StatuslinePrefs::import_toml(&text).expect("toml").prefs
    };
    let p = read(
        r#"[[items]]
kind = "seven_day"
format = "remaining"
reset_format = "clock"
bar_width = 12
bar_style = "dots"

[[items]]
kind = "five_hour"
reset_format = "sundial"
bar_width = 7
bar_style = "rainbow"

[[items]]
kind = "context"
reset_format = "clock"
bar_width = 4

[[items]]
kind = "daily_budget"
bar_width = 12
"#,
    );
    let [seven, five, ctx, budget] = &p.items[..] else {
        panic!("four items: {:?}", p.items);
    };
    assert_eq!(seven.format, Some(Format::Remaining));
    assert_eq!(seven.reset_format, ResetFormat::Clock);
    assert_eq!((seven.bar_width, seven.bar_style), (12, BarStyle::Dots));
    assert_eq!(five.reset_format, ResetFormat::Countdown);
    assert_eq!((five.bar_width, five.bar_style), (8, BarStyle::Blocks));
    // A reset format only on windows; a bar width only where a bar can be drawn.
    assert_eq!(ctx.reset_format, ResetFormat::Countdown);
    assert_eq!(ctx.bar_width, 4);
    assert_eq!(budget.bar_width, 8);
    // Written only where it differs from the default, and read back the same.
    let text = p.export_toml();
    assert!(text.contains("bar_style = \"dots\""));
    assert!(!text.contains("bar_width = 8"));
    let again = StatuslinePrefs::import_toml(&text).expect("toml").prefs;
    assert_eq!(again, p);
}

#[test]
fn only_the_detailed_preset_gets_a_new_item_and_samples_fill_them() {
    use SegmentKind as K;
    let new = [
        K::DailyBudget,
        K::BurnRate,
        K::SessionName,
        K::CacheRatio,
        K::OutputStyle,
    ];
    for preset in Preset::ALL {
        let kinds: Vec<K> = preset.prefs().items.iter().map(|i| i.kind).collect();
        let want: Vec<K> = if preset == Preset::Detailed {
            vec![K::DailyBudget]
        } else {
            Vec::new()
        };
        let got: Vec<K> = new.into_iter().filter(|k| kinds.contains(k)).collect();
        assert_eq!(got, want, "{preset:?}");
    }
    for state in [
        SampleState::Normal,
        SampleState::Warn,
        SampleState::Crit,
        SampleState::Stale,
    ] {
        let line = Line::sample_in(now(), false, state);
        for kind in new {
            assert_ne!(
                one(&line, Item::new(kind)),
                "no limit data yet",
                "{kind:?} {state:?}"
            );
        }
    }
    // The sample's budget follows its state: ok, ahead of time, critical.
    let sev = |state| {
        Line::sample_in(now(), false, state)
            .daily_budget()
            .expect("budget")
            .severity
    };
    assert_eq!(sev(SampleState::Normal), Severity::Ok);
    assert_eq!(sev(SampleState::Warn), Severity::Warn);
    assert_eq!(sev(SampleState::Crit), Severity::Crit);
}
