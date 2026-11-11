//! Step 12r: the cache timer's countdown and the `git_status` item.

use super::*;
use crate::statusline::git_status::GitStatus;
use crate::statusline::line::{Format, IconSet, Item, Zone};

/// The sample in UTC: the cache goes cold at 22:07.
fn sample() -> Line {
    let mut line = Line::sample(now(), false);
    line.zone = Zone::Fixed(0);
    line
}

fn item(kind: SegmentKind, format: Option<Format>) -> Item {
    let mut item = Item::new(kind);
    if format.is_some() {
        item.format = format;
    }
    item
}

fn prefs(item: Item, icons: IconSet) -> StatuslinePrefs {
    StatuslinePrefs {
        items: vec![item],
        icons,
        ..StatuslinePrefs::default()
    }
    .normalized()
}

fn one(locale: Locale, line: &Line, kind: SegmentKind, format: Option<Format>) -> String {
    draw(
        locale,
        line,
        &prefs(item(kind, format), IconSet::Text),
        None,
    )
}

fn en(line: &Line, kind: SegmentKind, format: Format) -> String {
    one(Locale::En, line, kind, Some(format))
}

/// The paint of the span that holds `text`.
fn paint_of(line: &Line, item: Item, text: &str) -> Paint {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let style = Style {
        color: true,
        width: None,
    };
    spans(&l, line, &prefs(item, IconSet::Text), style)
        .remove(0)
        .into_iter()
        .find(|s| s.text.contains(text))
        .unwrap_or_else(|| panic!("no span with {text}"))
        .paint
}

fn expires_in(line: &mut Line, ms: i64) {
    let cache = line.facts.cache.as_mut().expect("the sample has a cache");
    cache.expires_at = Some(Timestamp::from_millis(NOW + ms));
}

const HIDDEN: &str = "no limit data yet";

fn git(line: &mut Line) -> &mut GitStatus {
    line.git.as_mut().expect("the sample has a git status")
}

#[test]
fn the_countdown_shows_whole_minutes_and_seconds_only_near_the_end() {
    let mut line = sample();
    let kind = SegmentKind::CacheTimer;
    let countdown = Format::Countdown;
    // The clock stays the default: lines written before keep their look.
    assert_eq!(
        one(Locale::En, &line, kind, None),
        "cache →22:07 1h",
        "the default"
    );
    expires_in(&mut line, 47 * 60_000 + 30_000);
    assert_eq!(en(&line, kind, countdown), "cache 47m");
    expires_in(&mut line, 62 * 60_000);
    assert_eq!(en(&line, kind, countdown), "cache 1h2m");
    expires_in(&mut line, 120_000);
    assert_eq!(en(&line, kind, countdown), "cache 2m");
    // Under two minutes the seconds count.
    expires_in(&mut line, 72_000);
    assert_eq!(en(&line, kind, countdown), "cache 1m12s");
    expires_in(&mut line, 60_400);
    assert_eq!(en(&line, kind, countdown), "cache 1m");
    expires_in(&mut line, 45_000);
    assert_eq!(en(&line, kind, countdown), "cache 45s");
    assert_eq!(
        paint_of(&line, item(kind, Some(countdown)), "45s"),
        Paint::Warn
    );
    expires_in(&mut line, 5 * 60_000);
    assert_eq!(
        paint_of(&line, item(kind, Some(countdown)), "5m"),
        Paint::Plain
    );
    // Cold looks the same in both formats.
    expires_in(&mut line, -1_000);
    assert_eq!(en(&line, kind, countdown), "cache cold");
    assert_eq!(one(Locale::Tr, &line, kind, Some(countdown)), "cache soğuk");
}

#[test]
fn the_countdown_speaks_turkish() {
    let mut line = sample();
    expires_in(&mut line, 72_000);
    let tr = one(
        Locale::Tr,
        &line,
        SegmentKind::CacheTimer,
        Some(Format::Countdown),
    );
    assert!(tr.starts_with("cache 1"), "{tr}");
    assert!(tr.ends_with("12sn"), "{tr}");
}

#[test]
fn the_git_status_counts_each_kind_of_change() {
    let mut line = sample();
    let kind = SegmentKind::GitStatus;
    assert_eq!(
        one(Locale::En, &line, kind, None),
        "+2 ~3 ?1 \u{21E1}1",
        "the sample, compact by default"
    );
    assert_eq!(paint_of(&line, Item::new(kind), "+2"), Paint::Ok);
    assert_eq!(paint_of(&line, Item::new(kind), "~3"), Paint::Warn);
    assert_eq!(paint_of(&line, Item::new(kind), "?1"), Paint::Dim);
    git(&mut line).conflicted = 2;
    git(&mut line).behind = Some(4);
    assert_eq!(
        en(&line, kind, Format::Compact),
        "+2 ~3 ?1 !2 \u{21E1}1 \u{21E3}4"
    );
    assert_eq!(paint_of(&line, Item::new(kind), "!2"), Paint::Crit);
    // Not scanned: unknown, never zero.
    git(&mut line).untracked = 0;
    git(&mut line).untracked_skipped = true;
    assert_eq!(
        en(&line, kind, Format::Compact),
        "+2 ~3 ?? !2 \u{21E1}1 \u{21E3}4"
    );
    // Clean and level with the upstream: a check mark.
    *git(&mut line) = GitStatus {
        ahead: Some(0),
        behind: Some(0),
        ..GitStatus::default()
    };
    assert_eq!(en(&line, kind, Format::Compact), "\u{2713}");
    assert_eq!(paint_of(&line, Item::new(kind), "\u{2713}"), Paint::Ok);
    // Without the app's answer the item is hidden, never guessed.
    line.git = None;
    assert_eq!(en(&line, kind, Format::Compact), HIDDEN);
}

#[test]
fn the_dirty_mark_shows_only_while_something_is_uncommitted() {
    let mut line = sample();
    let kind = SegmentKind::GitStatus;
    assert_eq!(en(&line, kind, Format::Dirty), "*");
    let nerd = draw(
        Locale::En,
        &line,
        &prefs(item(kind, Some(Format::Dirty)), IconSet::Nerd),
        None,
    );
    assert!(nerd.contains('\u{F444}'), "{nerd}");
    // Commits ahead alone are not a dirty tree.
    *git(&mut line) = GitStatus {
        ahead: Some(3),
        ..GitStatus::default()
    };
    assert_eq!(en(&line, kind, Format::Dirty), HIDDEN);
    git(&mut line).untracked = 1;
    assert_eq!(en(&line, kind, Format::Dirty), "*");
}

#[test]
fn ahead_and_behind_hide_without_an_upstream_or_a_distance() {
    let mut line = sample();
    let kind = SegmentKind::GitStatus;
    assert_eq!(en(&line, kind, Format::AheadBehind), "\u{21E1}1");
    git(&mut line).behind = Some(2);
    assert_eq!(en(&line, kind, Format::AheadBehind), "\u{21E1}1 \u{21E3}2");
    git(&mut line).ahead = Some(0);
    git(&mut line).behind = Some(0);
    assert_eq!(en(&line, kind, Format::AheadBehind), HIDDEN);
    git(&mut line).ahead = None;
    git(&mut line).behind = None;
    assert_eq!(en(&line, kind, Format::AheadBehind), HIDDEN);
}

#[test]
fn a_narrow_line_gives_up_the_git_status_before_the_limit() {
    let line = sample();
    let prefs = StatuslinePrefs {
        items: vec![
            Item::new(SegmentKind::SevenDay),
            Item::new(SegmentKind::GitStatus),
        ],
        ..StatuslinePrefs::default()
    }
    .normalized();
    let wide = draw(Locale::En, &line, &prefs, None);
    assert!(wide.ends_with("+2 ~3 ?1 \u{21E1}1"), "{wide}");
    let narrow = draw(Locale::En, &line, &prefs, Some(12));
    assert!(narrow.starts_with("7d"), "{narrow}");
    assert!(!narrow.contains("~3"), "{narrow}");
}

#[test]
fn the_git_marks_take_one_cell_even_in_east_asian_terminals() {
    use unicode_width::UnicodeWidthChar;
    for c in ['\u{21E1}', '\u{21E3}', '*', '\u{2713}', '+', '~', '?', '!'] {
        assert_eq!(c.width(), Some(1), "{c:?}");
        assert_eq!(c.width_cjk(), Some(1), "{c:?} is ambiguous");
    }
    let icon = SegmentKind::GitStatus.icon(IconSet::Nerd).expect("an icon");
    assert!(icon.chars().all(|c| ('\u{E000}'..='\u{F8FF}').contains(&c)));
    assert_eq!(SegmentKind::GitStatus.icon(IconSet::Unicode), None);
}

#[test]
fn the_untracked_switch_reads_back_and_belongs_to_git_status_alone() {
    assert_eq!(
        SegmentKind::parse("git_status"),
        Some(SegmentKind::GitStatus)
    );
    assert_eq!(
        SegmentKind::GitStatus.formats(),
        &[Format::Compact, Format::Dirty, Format::AheadBehind]
    );
    let toml = r#"
version = 2
[[items]]
kind = "git_status"
format = "ahead_behind"
untracked = false
[[items]]
kind = "repo"
untracked = false
"#;
    let prefs: StatuslinePrefs = toml::from_str(toml).expect("prefs");
    let prefs = prefs.normalized();
    assert_eq!(prefs.items[0].format, Some(Format::AheadBehind));
    assert!(!prefs.items[0].untracked);
    // Another kind has nothing to switch off.
    assert!(prefs.items[1].untracked);
    let mut doc = toml_edit::DocumentMut::new();
    let mut table = toml_edit::Table::new();
    prefs.write_toml(&mut table, false);
    doc["statusline"] = toml_edit::Item::Table(table);
    let text = doc.to_string();
    assert_eq!(text.matches("untracked = false").count(), 1, "{text}");
    assert!(text.contains("format = \"ahead_behind\""), "{text}");
    // A value of the wrong type falls back to the default (ADR-042).
    let lax: StatuslinePrefs =
        toml::from_str("version = 2\n[[items]]\nkind = \"git_status\"\nuntracked = \"no\"\n")
            .expect("prefs");
    assert_eq!(lax.items[0].kind, SegmentKind::GitStatus);
    assert!(lax.items[0].untracked);
}
