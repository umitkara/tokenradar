//! Step 12k: separators as items, `space` and `flex`, regions and the display width (ADR-051).

use super::*;
use crate::statusline::line::{Format, Item, Preset, Theme};

use SegmentKind as K;

/// A version 2 list, exactly as given.
fn v2(items: Vec<Item>) -> StatuslinePrefs {
    StatuslinePrefs {
        items,
        ..StatuslinePrefs::default()
    }
    .normalized()
}

fn sep() -> Item {
    Item::new(K::Separator)
}

fn space(width: u8) -> Item {
    let mut item = Item::new(K::Space);
    item.width = width;
    item
}

fn read(text: &str) -> StatuslinePrefs {
    toml::from_str(text).expect("toml")
}

fn stored(prefs: &StatuslinePrefs) -> String {
    let mut doc = toml_edit::DocumentMut::new();
    let mut table = toml_edit::Table::new();
    prefs.write_toml(&mut table, true);
    doc["statusline"] = toml_edit::Item::Table(table);
    doc.to_string()
}

fn reread(text: &str) -> StatuslinePrefs {
    #[derive(serde::Deserialize)]
    struct File {
        statusline: StatuslinePrefs,
    }
    toml::from_str::<File>(text).expect("toml").statusline
}

fn plain(line: &Line, prefs: &StatuslinePrefs, width: Option<usize>) -> String {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    render(
        &l,
        line,
        prefs,
        Style {
            color: false,
            width,
        },
    )
}

fn kinds(prefs: &StatuslinePrefs) -> Vec<SegmentKind> {
    prefs.items.iter().map(|i| i.kind).collect()
}

#[test]
fn tables_without_a_version_draw_the_same_after_the_migration() {
    // 12c: only the old keys.
    let old = read("segments = [\"seven_day\", \"scoped\", \"context\", \"cost\"]\npace = false");
    assert_eq!(
        draw(Locale::En, &weekly_only(), &old, None),
        "7d ████░░░░ 41% · 3d4h │ Fable 22% │ ctx 38% │ API $4.12"
    );
    assert_eq!(
        kinds(&old),
        vec![
            K::SevenDay,
            K::Separator,
            K::Scoped,
            K::Separator,
            K::Context,
            K::Separator,
            K::Cost
        ]
    );
    // 12i: items, a separator of the user's, a text, a line break, an own glyph.
    let old = read(
        r#"separator = "·"
           [[items]]
           kind = "custom_text"
           text = "★"
           [[items]]
           kind = "context"
           [[items]]
           kind = "separator"
           text = "›"
           [[items]]
           kind = "cost"
           [[items]]
           kind = "model"
           [[items]]
           kind = "line_break"
           [[items]]
           kind = "git_branch"
           [[items]]
           kind = "cwd""#,
    );
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &old, None),
        "★ ctx 38% › API $4.12 · Opus 5.5\ngit main · src"
    );
    // The double 12c layout: no separator across the line break.
    let old = read("segments = [\"seven_day\", \"cost\"]\nlayout = \"double\"\npace = false");
    assert_eq!(kinds(&old), vec![K::SevenDay, K::LineBreak, K::Cost]);
    // An empty or blank glyph: the gap 12i drew, as a space.
    let old = read("separator = \"\"\nsegments = [\"context\", \"cost\"]");
    assert_eq!(kinds(&old), vec![K::Context, K::Space, K::Cost]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &old, None),
        "ctx 38% API $4.12"
    );
    let old = read("separator = \" \"\nsegments = [\"context\", \"cost\"]");
    assert_eq!(old.items[1].width, 3);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &old, None),
        "ctx 38%   API $4.12"
    );
}

#[test]
fn the_migration_runs_once() {
    let old = read("segments = [\"seven_day\", \"context\", \"cost\"]");
    let text = stored(&old);
    assert!(text.contains("version = 2"), "{text}");
    let again = reread(&text);
    assert_eq!(again, old);
    assert_eq!(reread(&stored(&again)), old, "no separators pile up");
    // A version 2 list is taken as it is: two values side by side stay so.
    let v2 = read("version = 2\n[[items]]\nkind = \"context\"\n[[items]]\nkind = \"cost\"");
    assert_eq!(kinds(&v2), vec![K::Context, K::Cost]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &v2, None),
        "ctx 38% API $4.12"
    );
    // The presets are version 2 lists with the separators in them.
    let p = Preset::Default.prefs();
    assert_eq!(reread(&stored(&p)), p);
    assert!(p.items.iter().any(|i| i.kind == K::Separator));
}

#[test]
fn a_missing_or_hidden_value_leaves_no_double_separator() {
    // No 5-hour window on this account: the line does not start with a separator.
    let p = Preset::Default.prefs();
    assert_eq!(p.items[1].kind, K::Separator);
    let out = draw(Locale::En, &weekly_only(), &p, None);
    assert!(out.starts_with("7d "), "{out}");
    // A hidden value between two separators: one is drawn.
    let mut cost = Item::new(K::Cost);
    cost.enabled = false;
    let p = v2(vec![
        Item::new(K::Context),
        sep(),
        cost,
        sep(),
        Item::new(K::Model),
        sep(),
    ]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &p, None),
        "ctx 38% │ Opus 5.5"
    );
    // Separators in a row and at the ends: one, between the values only.
    let p = v2(vec![
        sep(),
        Item::new(K::Context),
        Item::new(K::Separator).with_text("›"),
        sep(),
        Item::new(K::Model),
    ]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &p, None),
        "ctx 38% › Opus 5.5"
    );
}

#[test]
fn spaces_carry_their_own_columns() {
    let p = v2(vec![Item::new(K::Context), space(3), Item::new(K::Cost)]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &p, None),
        "ctx 38%   API $4.12"
    );
    // Between a separator and a value; a separator nothing follows goes, its space stays.
    let p = v2(vec![
        Item::new(K::Context),
        sep(),
        space(2),
        Item::new(K::Cost),
        sep(),
        space(1),
    ]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &p, None),
        "ctx 38% │  API $4.12 "
    );
    // The width is 1–8 on a space and 1 on everything else.
    let mut wide = space(20);
    let mut odd = Item::new(K::Cost);
    odd.width = 5;
    let p = v2(vec![space(0), Item::new(K::Context), wide.clone(), odd]);
    let widths: Vec<u8> = p.items.iter().map(|i| i.width).collect();
    assert_eq!(widths, vec![1, 1, MAX_SPACE, 1]);
    wide.width = 4;
    let text = stored(&v2(vec![wide, Item::new(K::Cost)]));
    assert_eq!(text.matches("width").count(), 1, "{text}");
    assert!(text.contains("width = 4"), "{text}");
}

#[test]
fn one_flex_leans_the_rest_on_the_right_edge() {
    let p = v2(vec![
        Item::new(K::Context),
        Item::new(K::Flex),
        Item::new(K::Cost),
    ]);
    let out = plain(&with_five_hour(), &p, cols(40));
    assert_eq!(display_width(&out), 40, "{out:?}");
    assert!(
        out.starts_with("ctx 38% ") && out.ends_with(" API $4.12"),
        "{out:?}"
    );
    // Unknown width: one space.
    assert_eq!(plain(&with_five_hour(), &p, None), "ctx 38% API $4.12");
    // The margin keeps columns free past the four Claude Code keeps.
    let margin = StatuslinePrefs { margin: 3, ..p };
    assert_eq!(
        display_width(&plain(&with_five_hour(), &margin, cols(40))),
        37
    );
}

#[test]
fn two_flexes_centre_the_middle_region() {
    let p = v2(vec![
        Item::new(K::Context),
        Item::new(K::Flex),
        Item::new(K::Model),
        Item::new(K::Flex),
        Item::new(K::Cost),
    ]);
    let out = plain(&with_five_hour(), &p, cols(60));
    assert_eq!(display_width(&out), 60, "{out:?}");
    // "Opus 5.5" is 8 wide: it starts at (60 - 8) / 2 = 26.
    assert_eq!(out.find("Opus"), Some(26), "{out:?}");
    assert!(out.ends_with(" API $4.12"), "{out:?}");
    // Little room: the middle keeps a column off its neighbours.
    let tight = v2(vec![
        Item::new(K::Duration).with_format(Format::Hms),
        Item::new(K::Flex),
        Item::new(K::Model),
        Item::new(K::Flex),
        Item::new(K::Version),
    ]);
    let out = plain(&with_five_hour(), &tight, cols(30));
    assert_eq!(out, "time 0:20:34 Opus 5.5 v2.1.310", "{out:?}");
    // Only the middle: a centred line, its leading spaces kept by a reset.
    let alone = v2(vec![
        Item::new(K::Flex),
        Item::new(K::Cost),
        Item::new(K::Flex),
    ]);
    let out = plain(&with_five_hour(), &alone, cols(29));
    assert_eq!(out, "\x1b[0m          API $4.12          ");
}

#[test]
fn more_flexes_share_the_free_columns() {
    let p = v2(vec![
        Item::new(K::Context),
        Item::new(K::Flex),
        Item::new(K::Cost),
        Item::new(K::Flex),
        Item::new(K::Model),
        Item::new(K::Flex),
        Item::new(K::Version),
    ]);
    // 7 + 9 + 8 + 8 = 32 wide; 10 free: 3, 3 and the remaining 4.
    let out = plain(&with_five_hour(), &p, cols(42));
    assert_eq!(out, "ctx 38%   API $4.12   Opus 5.5    v2.1.310");
    // Two flexes in a row: an empty middle region.
    let p = v2(vec![
        Item::new(K::Context),
        Item::new(K::Flex),
        Item::new(K::Flex),
        Item::new(K::Cost),
    ]);
    let out = plain(&with_five_hour(), &p, cols(30));
    assert_eq!(display_width(&out), 30, "{out:?}");
    assert!(
        out.starts_with("ctx 38% ") && out.ends_with(" API $4.12"),
        "{out:?}"
    );
}

#[test]
fn a_narrow_line_keeps_every_flex_and_drops_values_first() {
    let p = v2(vec![
        Item::new(K::SevenDay).with_format(Format::Percent),
        Item::new(K::Flex),
        Item::new(K::Model),
        Item::new(K::Flex),
        Item::new(K::Cost),
    ]);
    let line = weekly_only();
    for width in [12, 20, 30, 40, 50] {
        let out = plain(&line, &p, cols(width));
        assert!(display_width(&out) <= width.max(8), "{width}: {out:?}");
        assert!(out.contains("7d 41%"), "{width}: {out:?}");
    }
    // Just wide enough: every flex is one column.
    let full = "7d 41% · 3d4h Opus 5.5 API $4.12";
    assert_eq!(plain(&line, &p, cols(display_width(full))), full);
    // One less: the last session value goes, the flexes stay.
    let out = plain(&line, &p, cols(display_width(full) - 1));
    assert_eq!(out.trim_end(), "7d 41% · 3d4h Opus 5.5", "{out:?}");
}

#[test]
fn lines_of_only_layout_items_are_left_out() {
    let p = v2(vec![
        Item::new(K::Context),
        Item::new(K::LineBreak),
        Item::new(K::Flex),
        space(2),
        Item::new(K::Flex),
    ]);
    assert_eq!(draw(Locale::En, &with_five_hour(), &p, cols(40)), "ctx 38%");
}

#[test]
fn wide_glyphs_count_their_cells() {
    assert_eq!(display_width("⏱▲≈"), 3, "observed one cell each");
    assert_eq!(display_width("日本"), 4);
    let p = v2(vec![
        Item::new(K::CustomText).with_text("日本日本日本"),
        Item::new(K::Flex),
        Item::new(K::Cost),
    ]);
    let out = plain(&with_five_hour(), &p, cols(30));
    assert_eq!(display_width(&out), 30, "{out:?}");
    // Too narrow for the text: the text goes before the line passes the width.
    let out = plain(&with_five_hour(), &p, cols(20));
    assert!(display_width(&strip(&out)) <= 20, "{out:?}");
}

#[test]
fn powerline_regions_close_their_chains_and_lean_right() {
    let p = StatuslinePrefs {
        theme: Theme::Powerline,
        ..v2(vec![
            Item::new(K::SevenDay).with_format(Format::Percent),
            Item::new(K::Flex),
            Item::new(K::Model),
            Item::new(K::Cost),
        ])
    };
    let line = with_five_hour();
    let out = ansi_at(&line, &p, cols(80));
    // The left chain ends with its joint, then plain spaces, then the right chain starts with
    // a left-pointing joint in the first block's color and joins with one.
    assert!(out.contains("\x1b[36m\u{E0B0}\x1b[0m "), "{out:?}");
    assert!(
        out.contains(" \x1b[34m\u{E0B2}\x1b[0m\x1b[97;44m "),
        "{out:?}"
    );
    assert!(out.contains("\x1b[32;44m\u{E0B2}\x1b[0m"), "{out:?}");
    assert!(out.ends_with("\x1b[30;42m \x1b[0m"), "{out:?}");
    assert_eq!(display_width(&strip(&out)), 80, "{out:?}");
    // Round joints point left too.
    let round = StatuslinePrefs {
        theme: Theme::PowerlineRound,
        ..p.clone()
    };
    assert!(ansi_at(&line, &round, cols(80)).contains("\u{E0B6}"));
    // A space ends a chain and has no block.
    let spaced = StatuslinePrefs {
        theme: Theme::Powerline,
        ..v2(vec![Item::new(K::Model), space(2), Item::new(K::Cost)])
    };
    let out = ansi_at(&line, &spaced, None);
    assert!(
        out.contains("\x1b[34m\u{E0B0}\x1b[0m  \x1b[30;42m"),
        "{out:?}"
    );
    // Without color: the plain theme, aligned the same way.
    let out = plain(&line, &p, cols(80));
    assert!(
        out.starts_with("7d 41%") && out.ends_with("Opus 5.5 API $4.12"),
        "{out:?}"
    );
    assert_eq!(display_width(&out), 80);
}

fn ansi_at(line: &Line, prefs: &StatuslinePrefs, width: Option<usize>) -> String {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    render(&l, line, prefs, Style { color: true, width })
}

#[test]
fn flex_and_space_spans_know_their_item() {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let p = v2(vec![
        Item::new(K::Context),
        space(2),
        Item::new(K::Cost),
        Item::new(K::Flex),
        Item::new(K::Model),
    ]);
    let lines = spans(
        &l,
        &with_five_hour(),
        &p,
        Style {
            color: true,
            width: cols(60),
        },
    );
    let tags: Vec<Option<u16>> = lines[0].iter().map(|s| s.item).collect();
    assert_eq!(tags, vec![Some(0), Some(1), Some(2), Some(3), Some(4)]);
}

#[test]
fn the_split_preset_puts_the_project_on_the_right() {
    let p = Preset::Split.prefs();
    let out = plain(&with_five_hour(), &p, cols(120));
    assert_eq!(display_width(&out), 120, "{out:?}");
    assert!(out.starts_with("5h 72% ⏱"), "{out:?}");
    assert!(out.ends_with(" Opus 5.5 │ src │ git main"), "{out:?}");
    assert_eq!(reread(&stored(&p)), p);
}

#[test]
fn export_and_import_keep_the_layout() {
    let mut p = Preset::Split.prefs();
    p.margin = 2;
    p.items.insert(1, space(3));
    let back = StatuslinePrefs::import_toml(&p.export_toml()).expect("import");
    assert_eq!(back.prefs, p);
    assert_eq!(back.ignored, Vec::<String>::new());
    // The margin is 0–8.
    assert_eq!(read("version = 2\nmargin = 99").margin, MAX_MARGIN);
    assert_eq!(read("version = 2\nmargin = -1").margin, 0);
}
