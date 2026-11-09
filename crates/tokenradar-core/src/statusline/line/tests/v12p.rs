//! Step 12p: icon sets (`text`, `unicode`, `nerd`) and the `capsule` and `solid` bar styles.

use super::*;
use crate::statusline::line::{BarStyle, CAPSULE, Format, IconSet, Item, Theme};

fn alone(item: Item, icons: IconSet) -> StatuslinePrefs {
    StatuslinePrefs {
        items: vec![item],
        icons,
        ..StatuslinePrefs::default()
    }
    .normalized()
}

fn one(line: &Line, item: Item, icons: IconSet) -> String {
    draw(Locale::En, line, &alone(item, icons), None)
}

fn ansi(prefs: &StatuslinePrefs, line: &Line, color: bool) -> String {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    render(&l, line, prefs, Style { color, width: None })
}

fn is_pua(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}

/// The value kinds, every one of which has a value in the sample.
fn values() -> impl Iterator<Item = SegmentKind> {
    SegmentKind::ALL
        .into_iter()
        .filter(|k| !k.is_layout() && *k != SegmentKind::DailyBudget)
}

#[test]
fn the_nerd_set_puts_a_symbol_before_every_value_it_has_one_for() {
    let line = with_five_hour();
    for kind in values() {
        let text = one(&line, Item::new(kind), IconSet::Nerd);
        match kind.icon(IconSet::Nerd) {
            Some(icon) => assert!(text.starts_with(&format!("{icon} ")), "{kind:?}: {text}"),
            // Their values speak for themselves, or carry the model's own name.
            None => assert!(
                matches!(
                    kind,
                    SegmentKind::Scoped
                        | SegmentKind::SessionShare
                        | SegmentKind::Cost
                        | SegmentKind::VimMode
                        | SegmentKind::Compact
                ),
                "{kind:?} has no icon"
            ),
        }
    }
    assert_eq!(
        one(&line, Item::new(SegmentKind::GitBranch), IconSet::Nerd),
        "\u{F418} main"
    );
    assert_eq!(
        one(&line, Item::new(SegmentKind::Model), IconSet::Nerd),
        "\u{F4BC} Opus 5.5"
    );
}

#[test]
fn the_unicode_set_is_small_and_the_rest_keep_their_words() {
    let line = with_five_hour();
    let u = |kind| one(&line, Item::new(kind), IconSet::Unicode);
    assert_eq!(u(SegmentKind::TokensIn), "\u{21E1} 76k");
    assert_eq!(u(SegmentKind::TokensOut), "\u{21E3} 1.2k");
    assert_eq!(u(SegmentKind::GitBranch), "\u{2387} main");
    assert_eq!(u(SegmentKind::ProjectDir), "\u{2302} alpha");
    assert_eq!(u(SegmentKind::Cwd), "\u{203A} src");
    assert_eq!(u(SegmentKind::SessionId), "# a813d09f");
    // No good symbol: the words, as in the text set.
    assert_eq!(u(SegmentKind::Context), "ctx 38%");
    assert_eq!(u(SegmentKind::CacheRatio), "cache 89%");
    // No label in words and no symbol: the value alone.
    assert_eq!(u(SegmentKind::Model), "Opus 5.5");
    // The text set is the line as it was.
    assert_eq!(
        one(&line, Item::new(SegmentKind::GitBranch), IconSet::Text),
        "git main"
    );
}

#[test]
fn the_users_label_wins_and_a_blank_one_hides_the_symbol_too() {
    let line = with_five_hour();
    let mut own = Item::new(SegmentKind::GitBranch);
    own.label = "br".into();
    assert_eq!(one(&line, own, IconSet::Nerd), "br main");
    let mut blank = Item::new(SegmentKind::GitBranch);
    blank.label = " ".into();
    assert_eq!(one(&line, blank.clone(), IconSet::Nerd), "main");
    assert_eq!(one(&line, blank, IconSet::Unicode), "main");
}

#[test]
fn honest_marks_stay_in_every_set() {
    let line = with_five_hour();
    for icons in IconSet::ALL {
        // The share is an estimate of this session; the cost is priced as the API would.
        assert_eq!(
            one(&line, Item::new(SegmentKind::SessionShare), icons),
            "≈2.1% this session"
        );
        assert_eq!(one(&line, Item::new(SegmentKind::Cost), icons), "API $4.12");
        assert!(one(&line, Item::new(SegmentKind::BurnRate), icons).contains("API $12.01/h"));
    }
    // The flame goes before the mark, never instead of it.
    assert_eq!(
        one(&line, Item::new(SegmentKind::BurnRate), IconSet::Nerd),
        "\u{F490} API $12.01/h"
    );
    // An estimate keeps its `≈` and its dim drawing with a symbol for a label.
    let mut estimated = weekly_only();
    estimated.seven_day.as_mut().expect("weekly").source = Source::Estimate;
    let mut item = Item::new(SegmentKind::SevenDay).with_format(Format::Percent);
    item.reset = false;
    let p = alone(item, IconSet::Nerd);
    assert_eq!(draw(Locale::En, &estimated, &p, None), "≈\u{F455} 41%");
    assert!(ansi(&p, &estimated, true).starts_with("\x1b[2m"));
}

#[test]
fn only_the_nerd_set_changes_the_elapsed_and_compact_marks() {
    let line = with_five_hour();
    let mut week = Item::new(SegmentKind::SevenDay).with_format(Format::Percent);
    week.reset = false;
    assert_eq!(one(&line, week.clone(), IconSet::Text), "7d 41% ⏱54.8%");
    assert_eq!(one(&line, week.clone(), IconSet::Unicode), "7d 41% ⏱54.8%");
    assert_eq!(
        one(&line, week, IconSet::Nerd),
        "\u{F455} 41% \u{F4E3}54.8%"
    );
    let sample = Line::sample(now(), false);
    let compact = |icons| one(&sample, Item::new(SegmentKind::Compact), icons);
    assert_eq!(compact(IconSet::Unicode), "⟲3 · 300k");
    assert_eq!(compact(IconSet::Nerd), "\u{F48C}3 · 300k");
    let mut unknown = Line::sample(now(), false);
    unknown.compacts = None;
    assert_eq!(
        one(&unknown, Item::new(SegmentKind::Compact), IconSet::Nerd),
        "\u{F48C} 300k"
    );
}

#[test]
fn unicode_symbols_take_one_cell_in_every_terminal_and_nerd_ones_are_in_the_pua() {
    use unicode_width::UnicodeWidthChar;
    for kind in SegmentKind::ALL {
        if let Some(icon) = kind.icon(IconSet::Unicode) {
            let mut chars = icon.chars();
            let c = chars.next().expect("one char");
            assert_eq!(chars.next(), None, "{kind:?}");
            assert_eq!(c.width(), Some(1), "{c:?}");
            // Not East Asian "Ambiguous": one cell in a CJK terminal too.
            assert_eq!(c.width_cjk(), Some(1), "{c:?} is ambiguous");
        }
        if let Some(icon) = kind.icon(IconSet::Nerd) {
            assert_eq!(icon.chars().count(), 1, "{kind:?}");
            assert!(icon.chars().all(is_pua), "{kind:?}: {icon:?}");
            // `unicode-width` without `cjk` counts them as one cell, as a Mono font draws them.
            assert_eq!(display_width(icon), 1, "{kind:?}");
        }
    }
    for mark in [IconSet::Nerd.elapsed_mark(), IconSet::Nerd.compact_mark()] {
        assert!(mark.chars().all(is_pua), "{mark:?}");
    }
    for c in CAPSULE.iter().flatten() {
        assert!(is_pua(*c), "{c:?}");
    }
    // The solid style's mark is ASCII.
    assert_eq!(BarStyle::Solid.glyphs().2, '|');
}

fn bar_item(style: BarStyle) -> Item {
    let mut item = Item::new(SegmentKind::SevenDay);
    item.bar_style = style;
    item.reset = false;
    item
}

#[test]
fn the_capsule_keeps_its_frame_and_marks_the_cell_the_time_falls_in() {
    let line = with_five_hour();
    // 41 % of 8 cells: 4 full; 54.8 % of the time: the fifth cell, drawn bright.
    let text = one(&line, bar_item(BarStyle::Capsule), IconSet::Text);
    assert_eq!(
        text,
        "7d \u{EE03}\u{EE04}\u{EE04}\u{EE04}\u{EE01}\u{EE01}\u{EE01}\u{EE02} 41% ⏱54.8%"
    );
    // No extra cell: the bar is as wide as it says.
    assert_eq!(
        display_width(&text),
        display_width("7d ") + 8 + display_width(" 41% ⏱54.8%")
    );
    let colored = ansi(
        &alone(bar_item(BarStyle::Capsule), IconSet::Text),
        &line,
        true,
    );
    // Full cells in the window's color, the marked one plain, the rest dim.
    assert!(
        colored.contains("\x1b[36m\u{EE03}\u{EE04}\u{EE04}\u{EE04}\x1b[0m"),
        "{colored:?}"
    );
    assert!(
        colored.contains("\x1b[0m\u{EE01}\x1b[2m\u{EE01}\u{EE01}\u{EE02}\x1b[0m"),
        "{colored:?}"
    );
    // Full and empty at the ends, both ways round.
    let mut full = with_five_hour();
    full.seven_day.as_mut().expect("weekly").percent = 100.0;
    let mut quiet = bar_item(BarStyle::Capsule);
    quiet.pace = false;
    assert!(one(&full, quiet.clone(), IconSet::Text).starts_with("7d \u{EE03}\u{EE04}"));
    assert!(one(&full, quiet.clone(), IconSet::Text).contains("\u{EE04}\u{EE05} 100%"));
    let mut none = with_five_hour();
    none.seven_day.as_mut().expect("weekly").percent = 0.0;
    assert!(one(&none, quiet, IconSet::Text).starts_with("7d \u{EE00}\u{EE01}"));
}

#[test]
fn the_solid_bar_fills_with_background_and_marks_with_a_bar_inside_a_cell() {
    let line = with_five_hour();
    let p = alone(bar_item(BarStyle::Solid), IconSet::Text);
    assert_eq!(draw(Locale::En, &line, &p, None), "7d     |    41% ⏱54.8%");
    let colored = ansi(&p, &line, true);
    // Four cyan cells, the mark on the first grey one in black, three more grey ones.
    assert!(colored.contains("\x1b[36;46m    \x1b[0m"), "{colored:?}");
    assert!(
        colored.contains("\x1b[30;100m|\x1b[0m\x1b[100m   \x1b[0m"),
        "{colored:?}"
    );
    // The user's color fills below the warning threshold; the thresholds still win.
    let green = alone(
        bar_item(BarStyle::Solid).with_color(AnsiColor::Green),
        IconSet::Text,
    );
    assert!(ansi(&green, &line, true).contains("\x1b[36;42m    \x1b[0m"));
    let warn = Line::sample_in(now(), true, SampleState::Warn);
    let warn_text = ansi(&green, &warn, true);
    assert!(warn_text.contains(";43m"), "{warn_text:?}");
    assert!(!warn_text.contains(";42m"), "{warn_text:?}");
}

#[test]
fn powerline_blocks_draw_the_capsule_but_not_the_solid_bar_and_no_color_draws_neither() {
    let line = with_five_hour();
    let in_blocks = |style| StatuslinePrefs {
        theme: Theme::Powerline,
        ..alone(bar_item(style), IconSet::Nerd)
    };
    let capsule = draw(Locale::En, &line, &in_blocks(BarStyle::Capsule), None);
    assert!(capsule.contains("\u{EE03}\u{EE04}"), "{capsule}");
    // Every cell in the block's text color (black on cyan), the mark in its muted grey.
    let colored = ansi(&in_blocks(BarStyle::Capsule), &line, true);
    assert!(colored.contains("\x1b[30;46m\u{EE03}"), "{colored:?}");
    assert!(colored.contains("\x1b[2;90;46m\u{EE01}"), "{colored:?}");
    let solid = draw(Locale::En, &line, &in_blocks(BarStyle::Solid), None);
    assert!(!solid.contains('|'), "{solid}");
    assert!(solid.contains("\u{F455} 41%"), "{solid}");
    for style in [BarStyle::Capsule, BarStyle::Solid] {
        let plain = strip(&ansi(&alone(bar_item(style), IconSet::Text), &line, false));
        assert_eq!(plain, "7d 41% ⏱54.8%");
    }
}

#[test]
fn the_context_bar_takes_the_new_styles_too() {
    let line = with_five_hour();
    let mut ctx = Item::new(SegmentKind::Context).with_format(Format::Bar);
    ctx.bar_style = BarStyle::Capsule;
    // 38 % of 8: 4 full (any use shows), no time mark.
    assert_eq!(
        one(&line, ctx, IconSet::Nerd),
        "\u{F51E} \u{EE03}\u{EE04}\u{EE04}\u{EE04}\u{EE01}\u{EE01}\u{EE01}\u{EE02} 38%"
    );
}

#[test]
fn the_style_samples_carry_what_the_picker_draws() {
    let text = |style| {
        bar_sample(style)
            .iter()
            .map(|s| s.text.as_str())
            .collect::<String>()
    };
    assert_eq!(text(BarStyle::Blocks), "██░░░");
    assert_eq!(
        text(BarStyle::Capsule),
        "\u{EE03}\u{EE04}\u{EE01}\u{EE01}\u{EE02}"
    );
    let solid = bar_sample(BarStyle::Solid);
    assert_eq!(text(BarStyle::Solid), "     ");
    assert_eq!(solid.first().and_then(|s| s.bg), Some(AnsiColor::Cyan));
    assert_eq!(solid.last().and_then(|s| s.bg), Some(AnsiColor::Gray));
}

#[test]
fn the_icon_set_reads_leniently_and_round_trips() {
    let read = |text: &str| StatuslinePrefs::import_toml(text).expect("toml").prefs;
    assert_eq!(read("icons = \"nerd\"").icons, IconSet::Nerd);
    assert_eq!(read("icons = \"unicode\"").icons, IconSet::Unicode);
    // Unknown or of the wrong type: the words.
    assert_eq!(read("icons = \"emoji\"").icons, IconSet::Text);
    assert_eq!(read("icons = 3").icons, IconSet::Text);
    assert_eq!(read("").icons, IconSet::Text);
    assert!(read("icons = \"nerd\"").items.len() > 1);
    // Known to the import, so not listed as ignored.
    let imported = StatuslinePrefs::import_toml("icons = \"nerd\"").expect("toml");
    assert!(imported.ignored.is_empty(), "{:?}", imported.ignored);
    // Written and read back; the new bar styles too.
    let mut p = alone(bar_item(BarStyle::Capsule), IconSet::Nerd);
    p.items.push(Item {
        bar_style: BarStyle::Solid,
        ..Item::new(SegmentKind::Context).with_format(Format::Bar)
    });
    let text = p.export_toml();
    assert!(text.contains("icons = \"nerd\""), "{text}");
    assert!(text.contains("bar_style = \"capsule\""), "{text}");
    assert!(text.contains("bar_style = \"solid\""), "{text}");
    assert_eq!(read(&text), p.normalized());
}
