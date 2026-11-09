//! Step 12n: the `compact` item, its `total` and `split` formats and how it narrows.

use super::*;
use crate::ipc::CompactCount;
use crate::statusline::auto_compact::AutoCompact;
use crate::statusline::line::{Format, Item};

fn item(format: Format) -> Item {
    let mut item = Item::new(SegmentKind::Compact);
    item.format = Some(format);
    item
}

fn alone(item: Item) -> StatuslinePrefs {
    StatuslinePrefs {
        items: vec![item],
        ..StatuslinePrefs::default()
    }
    .normalized()
}

fn one(locale: Locale, line: &Line, format: Format) -> String {
    draw(locale, line, &alone(item(format)), None)
}

#[allow(clippy::unnecessary_wraps)] // what `Line::compacts` holds
fn count(auto: u32, manual: u32, other: u32) -> Option<CompactCount> {
    Some(CompactCount {
        auto,
        manual,
        other,
    })
}

#[test]
fn the_sample_shows_the_count_and_the_window() {
    let line = Line::sample(now(), false);
    assert_eq!(one(Locale::En, &line, Format::Total), "⟲ 3 · 300k");
    assert_eq!(
        one(Locale::En, &line, Format::Split),
        "⟲ 2 auto 1 manual · 300k"
    );
    assert_eq!(
        one(Locale::Tr, &line, Format::Split),
        "⟲ 2 oto 1 elle · 300k"
    );
    // The default format is the total.
    let default = draw(
        Locale::En,
        &line,
        &alone(Item::new(SegmentKind::Compact)),
        None,
    );
    assert_eq!(default, "⟲ 3 · 300k");
}

#[test]
fn zero_shows_unknown_hides_and_the_setting_has_three_states() {
    let mut line = Line::sample(now(), false);
    line.compacts = count(0, 0, 0);
    line.auto_compact = None;
    // Zero is worth showing; no setting, no window part.
    assert_eq!(one(Locale::En, &line, Format::Total), "⟲ 0");
    line.auto_compact = Some(AutoCompact::Off);
    assert_eq!(one(Locale::En, &line, Format::Total), "⟲ 0 · auto off");
    assert_eq!(one(Locale::Tr, &line, Format::Total), "⟲ 0 · oto kapalı");
    // Without the app the count is unknown: only the setting.
    line.compacts = None;
    assert_eq!(one(Locale::En, &line, Format::Total), "⟲ auto off");
    line.auto_compact = Some(AutoCompact::Window(1_000_000));
    assert_eq!(one(Locale::En, &line, Format::Split), "⟲ 1M");
    // Neither: the item is not drawn.
    line.auto_compact = None;
    assert!(!one(Locale::En, &line, Format::Total).contains('⟲'));
}

#[test]
fn an_unknown_trigger_counts_in_the_total_and_shows_apart_when_split() {
    let mut line = Line::sample(now(), false);
    line.compacts = count(2, 1, 1);
    line.auto_compact = None;
    assert_eq!(one(Locale::En, &line, Format::Total), "⟲ 4");
    assert_eq!(
        one(Locale::En, &line, Format::Split),
        "⟲ 2 auto 1 manual 1 other"
    );
    assert_eq!(
        one(Locale::Tr, &line, Format::Split),
        "⟲ 2 oto 1 elle 1 diğer"
    );
}

#[test]
fn a_narrow_terminal_drops_the_window_part_then_the_item() {
    let line = Line::sample(now(), false);
    let mut weekly = Item::new(SegmentKind::SevenDay);
    weekly.format = Some(Format::Percent);
    weekly.reset = false;
    weekly.pace = false;
    let prefs = StatuslinePrefs {
        items: vec![weekly, Item::new(SegmentKind::Compact)],
        ..StatuslinePrefs::default()
    }
    .normalized();
    let full = draw(Locale::En, &line, &prefs, None);
    assert!(full.ends_with("⟲ 3 · 300k"), "{full}");
    let width = full.chars().count();
    let narrower = draw(Locale::En, &line, &prefs, cols(width - 1));
    assert!(narrower.ends_with("⟲ 3"), "{narrower}");
    assert!(!narrower.contains("300k"), "{narrower}");
    let narrowest = draw(Locale::En, &line, &prefs, cols(width - 8));
    assert!(!narrowest.contains('⟲'), "{narrowest}");
    assert!(narrowest.contains("41%"), "{narrowest}");
}

#[test]
fn the_kind_and_its_formats_read_back() {
    assert_eq!(SegmentKind::parse("compact"), Some(SegmentKind::Compact));
    assert_eq!(SegmentKind::Compact.group(), Group::Session);
    assert_eq!(
        SegmentKind::Compact.formats(),
        &[Format::Total, Format::Split]
    );
    assert_eq!(Format::parse("split"), Some(Format::Split));
    let prefs: StatuslinePrefs =
        toml::from_str("version = 2\n[[items]]\nkind = \"compact\"\nformat = \"split\"\n")
            .expect("prefs");
    assert_eq!(prefs.items[0].kind, SegmentKind::Compact);
    assert_eq!(prefs.items[0].format, Some(Format::Split));
    // A format the kind does not take falls back to its default.
    let prefs: StatuslinePrefs =
        toml::from_str("version = 2\n[[items]]\nkind = \"compact\"\nformat = \"bar\"\n")
            .expect("prefs");
    assert_eq!(prefs.normalized().items[0].format, Some(Format::Total));
}
