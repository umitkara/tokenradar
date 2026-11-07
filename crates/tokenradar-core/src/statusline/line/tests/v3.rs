//! Step 12i: items, formats, labels, colors, themes and the stored table.

use super::*;
use crate::statusline::line::{AnsiColor, ColorMode, Format, Item, Preset, SampleState, Theme};

fn with(items: Vec<Item>) -> StatuslinePrefs {
    StatuslinePrefs {
        items,
        ..StatuslinePrefs::default()
    }
    .normalized()
}

fn one(item: Item) -> String {
    draw(Locale::En, &with_five_hour(), &with(vec![item]), None)
}

fn ansi(line: &Line, prefs: &StatuslinePrefs) -> String {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    render(
        &l,
        line,
        prefs,
        Style {
            color: true,
            width: None,
        },
    )
}

#[test]
fn every_format_draws_its_own_form() {
    use SegmentKind as K;
    let cases: [(Item, &str); 20] = [
        (
            Item::new(K::Context).with_format(Format::Bar),
            "ctx ████░░░░ 38%",
        ),
        (
            Item::new(K::Context).with_format(Format::Fraction),
            "ctx 76k/200k",
        ),
        (
            Item::new(K::Cost).with_format(Format::FourDecimals),
            "API $4.1200",
        ),
        (Item::new(K::Model), "Opus 5.5"),
        (
            Item::new(K::Model).with_format(Format::Full),
            "claude-opus-5-5",
        ),
        (Item::new(K::TokensIn), "in 76k"),
        (Item::new(K::TokensOut), "out 1.2k"),
        (
            Item::new(K::TokensIn).with_format(Format::Full),
            "in 76,000",
        ),
        (Item::new(K::Duration), "time 20m"),
        (
            Item::new(K::Duration).with_format(Format::Hms),
            "time 0:20:34",
        ),
        (Item::new(K::ApiDuration), "api 5m"),
        (Item::new(K::LinesChanged), "+156 -23"),
        (Item::new(K::LinesChanged).with_format(Format::Net), "+133"),
        (Item::new(K::SessionId), "a813d09f"),
        (Item::new(K::Cwd), "src"),
        (
            Item::new(K::Cwd).with_format(Format::Home),
            "~/projects/alpha/src",
        ),
        (Item::new(K::ProjectDir), "alpha"),
        (Item::new(K::Version), "v2.1.310"),
        (Item::new(K::GitBranch), "git main"),
        (Item::new(K::VimMode), "NORMAL"),
    ];
    for (item, expected) in cases {
        let kind = item.kind;
        assert_eq!(one(item), expected, "{kind:?}");
    }
    // Turkish numbers and words.
    let tr = |item: Item| draw(Locale::Tr, &with_five_hour(), &with(vec![item]), None);
    assert_eq!(tr(Item::new(K::TokensOut)), "çıktı 1,2k");
    assert_eq!(tr(Item::new(K::Duration)), "süre 20dk");
    // The 7d window without the bar.
    assert_eq!(
        one(Item::new(K::SevenDay).with_format(Format::Percent)),
        "7d 41% ⏱54.8% · 3d4h"
    );
}

#[test]
fn missing_values_hide_their_items() {
    let mut line = with_five_hour();
    line.facts = Facts::default();
    let p = with(vec![
        Item::new(SegmentKind::SevenDay).with_format(Format::Percent),
        Item::new(SegmentKind::GitBranch),
        Item::new(SegmentKind::TokensIn),
        Item::new(SegmentKind::Context).with_format(Format::Fraction),
    ]);
    // Without token counts the fraction falls back to the percentage.
    assert_eq!(
        draw(Locale::En, &line, &p, None),
        "7d 41% ⏱54.8% · 3d4h │ ctx 38%"
    );
}

#[test]
fn labels_replace_the_default_and_spaces_mean_none() {
    let mut seven = Item::new(SegmentKind::SevenDay).with_format(Format::Percent);
    seven.label = "week".into();
    seven.pace = false;
    seven.reset = false;
    assert_eq!(one(seven.clone()), "week 41%");
    seven.label = " ".into();
    assert_eq!(one(seven), "41%");
    let mut cost = Item::new(SegmentKind::Cost);
    cost.label = "≈cost".into();
    // The `API` mark stays (ADR-042).
    assert_eq!(one(cost), "≈cost API $4.12");
    let mut fable = Item::new(SegmentKind::Scoped);
    fable.label = "models".into();
    assert_eq!(one(fable), "models Fable 22%");
}

#[test]
fn texts_and_separators_sit_between_values_with_spaces() {
    let p = with(vec![
        Item::new(SegmentKind::CustomText).with_text("★"),
        Item::new(SegmentKind::Context),
        Item::new(SegmentKind::Separator).with_text("›"),
        Item::new(SegmentKind::Cost),
        Item::new(SegmentKind::Model),
    ]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &p, None),
        "★ ctx 38% › API $4.12 │ Opus 5.5"
    );
    // An empty separator: values are joined by a space.
    let mut bare = p.clone();
    bare.separator = String::new();
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &bare, None),
        "★ ctx 38% › API $4.12 Opus 5.5"
    );
    // A narrow terminal drops the value together with the separator before it.
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &p, Some(20)),
        "★ ctx 38%"
    );
}

#[test]
fn line_breaks_start_new_lines_and_empty_lines_are_left_out() {
    let p = with(vec![
        Item::new(SegmentKind::Context),
        Item::new(SegmentKind::LineBreak),
        Item::new(SegmentKind::GitBranch),
        Item::new(SegmentKind::LineBreak),
        Item::new(SegmentKind::CustomText).with_text("only text"),
        Item::new(SegmentKind::LineBreak),
        Item::new(SegmentKind::Cost),
    ]);
    assert_eq!(
        draw(Locale::En, &with_five_hour(), &p, None),
        "ctx 38%\ngit main\nAPI $4.12"
    );
}

#[test]
fn a_user_color_applies_below_the_warning_threshold_only() {
    let p = with(vec![
        Item::new(SegmentKind::FiveHour)
            .with_format(Format::Percent)
            .with_color(AnsiColor::Blue),
        Item::new(SegmentKind::SevenDay)
            .with_format(Format::Percent)
            .with_color(AnsiColor::Blue),
        Item::new(SegmentKind::Model).with_color(AnsiColor::BrightMagenta),
    ]);
    let mut line = with_five_hour();
    line.five_hour.as_mut().expect("5h").percent = 93.0;
    let out = ansi(&line, &p);
    // 93 %: the critical color wins.
    assert!(out.contains("\x1b[31m93%\x1b[0m"), "{out:?}");
    // 41 %: the user's blue instead of the automatic cyan.
    assert!(out.contains("\x1b[34m7d \x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[34m41%\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[95mOpus 5.5\x1b[0m"), "{out:?}");
    // A stale value stays dim.
    line.seven_day.as_mut().expect("7d").observed_at = Timestamp::from_millis(NOW - 11 * MIN);
    line.seven_day.as_mut().expect("7d").source = Source::Api;
    let out = ansi(&line, &p);
    assert!(out.contains("\x1b[2m41%\x1b[0m"), "{out:?}");
}

#[test]
fn powerline_draws_blocks_and_falls_back_without_color() {
    let p = StatuslinePrefs {
        theme: Theme::Powerline,
        ..with(vec![
            Item::new(SegmentKind::SevenDay).with_format(Format::Percent),
            Item::new(SegmentKind::Cost),
            Item::new(SegmentKind::Separator),
            Item::new(SegmentKind::Model),
        ])
    };
    let line = with_five_hour();
    let out = ansi(&line, &p);
    // Cyan block (black text), then green, then blue; the separator item is skipped.
    assert!(
        out.starts_with("\x1b[30;46m \x1b[0m\x1b[30;46m7d \x1b[0m"),
        "{out:?}"
    );
    assert!(out.contains("\x1b[36;42m\u{E0B0}\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[32;44m\u{E0B0}\x1b[0m"), "{out:?}");
    assert!(out.ends_with("\x1b[34m\u{E0B0}\x1b[0m"), "{out:?}");
    assert!(!strip(&out).contains('│'), "{out:?}");
    // Past the critical threshold the block turns red.
    let mut hot = line.clone();
    hot.seven_day.as_mut().expect("7d").percent = 95.0;
    assert!(ansi(&hot, &p).contains("\x1b[97;41m7d \x1b[0m"));
    // Without color: the plain theme.
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let plain = render(
        &l,
        &line,
        &p,
        Style {
            color: false,
            width: None,
        },
    );
    assert_eq!(plain, "7d 41% ⏱54.8% · 3d4h │ API $4.12 │ Opus 5.5");
    // Two blocks of one color are joined by the thin glyph.
    let same = StatuslinePrefs {
        theme: Theme::PowerlineRound,
        ..with(vec![
            Item::new(SegmentKind::Model),
            Item::new(SegmentKind::Cwd),
        ])
    };
    let out = ansi(&line, &same);
    assert!(
        out.contains("\u{E0B5}") && out.ends_with("\x1b[34m\u{E0B4}\x1b[0m"),
        "{out:?}"
    );
}

#[test]
fn spans_know_their_item() {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let p = with(vec![
        Item::new(SegmentKind::Context),
        Item::new(SegmentKind::Cost),
    ]);
    let lines = spans(
        &l,
        &with_five_hour(),
        &p,
        Style {
            color: true,
            width: None,
        },
    );
    let tags: Vec<Option<u16>> = lines[0].iter().map(|s| s.item).collect();
    assert_eq!(tags, vec![Some(0), None, Some(1)]);
}

#[test]
fn control_characters_never_reach_the_terminal() {
    let mut text = Item::new(SegmentKind::CustomText).with_text("a\x1b[2Jb\u{9b}c\n");
    text.label = "x\x07".into();
    let mut ctx = Item::new(SegmentKind::Context);
    ctx.label = "\x1b]0;t\x07ctx".into();
    let p = with(vec![text, ctx]);
    assert_eq!(p.items[0].text, "a[2Jbc");
    assert_eq!(p.items[0].label, "", "custom text has no label");
    let out = ansi(&with_five_hour(), &p);
    assert_eq!(out.matches('\x1b').count(), out.matches("\x1b[").count());
    assert!(strip(&out).contains("a[2Jbc ]0;tctx 38%"), "{out:?}");
}

#[test]
fn normalizing_keeps_each_value_once_and_never_leaves_nothing() {
    let p = with(vec![
        Item::new(SegmentKind::Cost),
        Item::new(SegmentKind::Separator),
        Item::new(SegmentKind::Cost),
        Item::new(SegmentKind::Separator).with_text(""),
    ]);
    let kinds: Vec<SegmentKind> = p.items.iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        vec![
            SegmentKind::Cost,
            SegmentKind::Separator,
            SegmentKind::Separator
        ]
    );
    assert_eq!(p.items[2].text, "│", "an empty glyph is the default");
    // Only layout items, or every value off: the default line.
    assert_eq!(
        with(vec![Item::new(SegmentKind::CustomText).with_text("hi")]),
        StatuslinePrefs::default()
    );
    let mut off = Item::new(SegmentKind::Cost);
    off.enabled = false;
    assert_eq!(with(vec![off]), StatuslinePrefs::default());
    // A format the kind does not take is its default.
    let odd = with(vec![Item::new(SegmentKind::Cost).with_format(Format::Bar)]);
    assert_eq!(odd.items[0].format, Some(Format::TwoDecimals));
}

fn stored(prefs: &StatuslinePrefs, legacy: bool) -> String {
    let mut doc = toml_edit::DocumentMut::new();
    let mut table = toml_edit::Table::new();
    prefs.write_toml(&mut table, legacy);
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

#[test]
fn the_stored_table_round_trips_and_older_builds_get_the_12c_keys() {
    for preset in Preset::ALL {
        let p = preset.prefs();
        assert_eq!(reread(&stored(&p, true)), p, "{preset:?}");
        assert_eq!(reread(&stored(&p, false)), p, "{preset:?}");
    }
    let mut p = Preset::Detailed.prefs();
    p.color = ColorMode::Always;
    p.items[1].label = "wk".into();
    p.items[1].pace = false;
    let text = stored(&p, true);
    assert!(
        text.contains("segments = [\"five_hour\", \"seven_day\""),
        "{text}"
    );
    assert!(text.contains("layout = \"double\""), "{text}");
    assert!(text.contains("[[statusline.items]]"), "{text}");
    // Only what differs from an item's defaults is written.
    assert!(!text.contains("enabled"), "{text}");
    assert!(text.contains("label = \"wk\"\npace = false"), "{text}");
    assert_eq!(reread(&text), p);
    // An older build reading only the 12c keys draws the same kinds.
    let legacy: StatuslinePrefs = toml::from_str(
        "segments = [\"five_hour\", \"seven_day\", \"scoped\", \"context\", \"session_share\", \
         \"cost\", \"model\"]\nlayout = \"double\"",
    )
    .expect("toml");
    assert_eq!(legacy.items.len(), 8);
}

#[test]
fn wrong_types_in_items_fall_back_instead_of_failing() {
    let p: StatuslinePrefs = toml::from_str(
        r#"theme = "neon"
           color = 5
           [[items]]
           kind = "cost"
           enabled = "yes"
           color = "chartreuse"
           [[items]]
           kind = "rainbow"
           [[items]]
           kind = "seven_day"
           format = "pie"
           pace = 1"#,
    )
    .expect("toml");
    assert_eq!(p.theme, Theme::Plain);
    assert_eq!(p.color, ColorMode::Auto);
    let kinds: Vec<SegmentKind> = p.items.iter().map(|i| i.kind).collect();
    assert_eq!(kinds, vec![SegmentKind::Cost, SegmentKind::SevenDay]);
    assert!(p.items[0].enabled && p.items[0].color.is_none());
    assert_eq!(p.items[1].format, Some(Format::Bar));
    assert!(p.items[1].pace);
}

#[test]
fn export_and_import_round_trip_and_list_what_was_skipped() {
    let mut p = Preset::Powerline.prefs();
    p.items[3].color = Some(AnsiColor::BrightYellow);
    let text = p.export_toml();
    assert!(text.starts_with("[statusline]\n"), "{text}");
    assert!(!text.contains("segments"), "{text}");
    let back = StatuslinePrefs::import_toml(&text).expect("import");
    assert_eq!(back.prefs, p);
    assert_eq!(back.ignored, Vec::<String>::new());
    // Without the header, with unknown keys and kinds.
    let back = StatuslinePrefs::import_toml(
        "theme = \"powerline\"\nwidth = 3\n[[items]]\nkind = \"cost\"\n[[items]]\nkind = \"x\"\n",
    )
    .expect("import");
    assert_eq!(back.prefs.theme, Theme::Powerline);
    assert_eq!(
        back.ignored,
        vec!["width".to_owned(), "items.kind = x".to_owned()]
    );
    // Only a syntax error fails.
    assert!(StatuslinePrefs::import_toml("theme = ").is_err());
}

#[test]
fn the_preview_sample_shows_each_state() {
    let p = with(vec![
        Item::new(SegmentKind::SevenDay).with_format(Format::Percent),
    ]);
    let draw_in = |state| ansi(&Line::sample_in(now(), false, state), &p);
    assert!(draw_in(SampleState::Normal).contains("\x1b[36m41%"));
    assert!(draw_in(SampleState::Warn).contains("\x1b[33m78%"));
    assert!(draw_in(SampleState::Crit).contains("\x1b[31m93%"));
    assert!(draw_in(SampleState::Stale).contains("\x1b[2m41%"));
}

#[test]
fn color_modes_resolve_against_no_color() {
    assert!(ColorMode::Auto.resolve(false));
    assert!(!ColorMode::Auto.resolve(true));
    assert!(ColorMode::Always.resolve(true));
    assert!(!ColorMode::Never.resolve(false));
}

#[test]
fn payload_metadata_reaches_the_facts() {
    let payload = StatuslinePayload::parse(
        br#"{"session_id":"s-1","version":"2.1.310","model":{"id":"claude-x"},
            "workspace":{"current_dir":"C:\\Work\\Alpha\\src","project_dir":"C:\\Work\\Alpha"},
            "cost":{"total_duration_ms":61000,"total_api_duration_ms":-5,"total_lines_added":3},
            "context_window":{"total_input_tokens":15500,"total_output_tokens":1200,
                              "context_window_size":200000},
            "vim":{"mode":"INSERT"},"agent":{"name":"rev\u001b[31m"},
            "worktree":{"branch":"wt-x"}}"#,
    )
    .expect("payload");
    let f = Line::from_sources(Some(&payload), None, now()).facts;
    assert_eq!(f.model_id.as_deref(), Some("claude-x"));
    assert_eq!(f.cwd.as_deref(), Some("C:\\Work\\Alpha\\src"));
    assert_eq!(f.project_dir.as_deref(), Some("C:\\Work\\Alpha"));
    assert_eq!(f.duration_ms, Some(61_000));
    assert_eq!(f.api_duration_ms, None, "a negative count is not one");
    assert_eq!(f.lines, Some((3, 0)));
    assert_eq!((f.tokens_in, f.tokens_out), (Some(15_500), Some(1_200)));
    assert_eq!(f.context_size, Some(200_000));
    assert_eq!(f.vim_mode.as_deref(), Some("INSERT"));
    assert_eq!(f.agent_name.as_deref(), Some("rev[31m"));
    assert_eq!(f.git_branch.as_deref(), Some("wt-x"));
    // None of it is forwarded in the push or the spool.
    let json = serde_json::to_string(&payload).expect("json");
    for field in [
        "workspace",
        "vim",
        "agent",
        "worktree",
        "total_duration_ms",
        "total_input_tokens",
    ] {
        assert!(!json.contains(field), "{field} in {json}");
    }
}
