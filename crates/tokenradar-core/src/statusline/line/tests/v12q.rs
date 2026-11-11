//! Step 12q: items from the payload's newer fields (effort, prompt cache, spend limit, PR,
//! worktree, repository) and the new formats of older items.

use super::*;
use crate::statusline::line::prefs::with_separators;
use crate::statusline::line::{
    CacheState, Format, IconSet, Item, PullRequest, Repo, Review, Span, Zone,
};

const DOCUMENTED: &str = include_str!("../../../../../../testdata/statusline/documented.json");

/// The sample in UTC: the cache goes cold at 22:07.
fn sample() -> Line {
    let mut line = Line::sample(now(), false);
    line.zone = Zone::Fixed(0);
    line
}

fn alone(item: Item, icons: IconSet) -> StatuslinePrefs {
    StatuslinePrefs {
        items: vec![item],
        icons,
        ..StatuslinePrefs::default()
    }
    .normalized()
}

fn item(kind: SegmentKind, format: Option<Format>) -> Item {
    let mut item = Item::new(kind);
    if format.is_some() {
        item.format = format;
    }
    item
}

fn one(locale: Locale, line: &Line, kind: SegmentKind, format: Option<Format>) -> String {
    draw(
        locale,
        line,
        &alone(item(kind, format), IconSet::Text),
        None,
    )
}

fn en(line: &Line, kind: SegmentKind) -> String {
    one(Locale::En, line, kind, None)
}

fn spans_of(line: &Line, item: Item) -> Vec<Span> {
    let l = Localizer::new(Locale::En, &[Domain::Cli]);
    let style = Style {
        color: true,
        width: None,
    };
    spans(&l, line, &alone(item, IconSet::Text), style).remove(0)
}

/// The paint of the span that holds `text`.
fn paint_of(line: &Line, item: Item, text: &str) -> Paint {
    spans_of(line, item)
        .into_iter()
        .find(|s| s.text.contains(text))
        .unwrap_or_else(|| panic!("no span with {text}"))
        .paint
}

fn cache(line: &mut Line) -> &mut crate::statusline::line::PromptCache {
    line.facts.cache.as_mut().expect("the sample has a cache")
}

const HIDDEN: &str = "no limit data yet";

#[test]
fn effort_shows_the_level_its_flags_and_hides_without_one() {
    let mut line = sample();
    assert_eq!(en(&line, SegmentKind::Effort), "high");
    assert_eq!(one(Locale::Tr, &line, SegmentKind::Effort, None), "yüksek");
    line.facts.effort = Some("max".into());
    line.facts.fast_mode = true;
    line.facts.thinking = Some(false);
    let flags = Some(Format::Flags);
    assert_eq!(
        one(Locale::En, &line, SegmentKind::Effort, flags),
        "max fast no thinking"
    );
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::Effort, flags),
        "maks hızlı düşünme yok"
    );
    // The level format leaves the flags out.
    assert_eq!(en(&line, SegmentKind::Effort), "max");
    // Nerd: a rocket for fast mode; thinking stays in words.
    let nerd = draw(
        Locale::En,
        &line,
        &alone(item(SegmentKind::Effort, flags), IconSet::Nerd),
        None,
    );
    assert_eq!(nerd, "\u{F463} max \u{F427} no thinking");
    // Unknown thinking counts as on.
    line.facts.thinking = None;
    line.facts.fast_mode = false;
    assert_eq!(one(Locale::En, &line, SegmentKind::Effort, flags), "max");
    // A level this build does not know shows as it came.
    line.facts.effort = Some("turbo".into());
    assert_eq!(en(&line, SegmentKind::Effort), "turbo");
    // No effort field: the model takes none. Nothing, not "unknown".
    line.facts.effort = None;
    assert_eq!(en(&line, SegmentKind::Effort), HIDDEN);
}

#[test]
fn effort_colors_follow_the_level() {
    let mut line = sample();
    for (level, paint) in [
        ("low", Paint::Dim),
        ("medium", Paint::Plain),
        ("high", Paint::Ok),
        ("xhigh", Paint::Ok),
        ("max", Paint::Warn),
        ("turbo", Paint::Plain),
    ] {
        line.facts.effort = Some(level.into());
        let shown = en(&line, SegmentKind::Effort);
        assert_eq!(
            paint_of(&line, Item::new(SegmentKind::Effort), &shown),
            paint,
            "{level}"
        );
    }
}

#[test]
fn the_model_can_carry_the_effort() {
    let mut line = sample();
    let with = Some(Format::WithEffort);
    assert_eq!(
        one(Locale::En, &line, SegmentKind::Model, with),
        "Opus 5.5 · high"
    );
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::Model, with),
        "Opus 5.5 · yüksek"
    );
    line.facts.effort = None;
    assert_eq!(one(Locale::En, &line, SegmentKind::Model, with), "Opus 5.5");
    // The default stays the name alone.
    line.facts.effort = Some("high".into());
    assert_eq!(en(&line, SegmentKind::Model), "Opus 5.5");
}

#[test]
fn the_cache_timer_shows_the_local_time_it_goes_cold() {
    let mut line = sample();
    assert_eq!(en(&line, SegmentKind::CacheTimer), "cache →22:07 1h");
    assert_eq!(
        paint_of(&line, Item::new(SegmentKind::CacheTimer), "22:07"),
        Paint::Plain
    );
    // Three hours east: the local time moves, the TTL does not.
    line.zone = Zone::Fixed(180);
    assert_eq!(en(&line, SegmentKind::CacheTimer), "cache →01:07 1h");
    line.zone = Zone::Fixed(0);
    // The last minute: the warning color.
    cache(&mut line).expires_at = Some(Timestamp::from_millis(NOW + 60_000));
    assert_eq!(
        paint_of(&line, Item::new(SegmentKind::CacheTimer), "21:21"),
        Paint::Warn
    );
    cache(&mut line).expires_at = Some(Timestamp::from_millis(NOW + 61_000));
    assert_eq!(
        paint_of(&line, Item::new(SegmentKind::CacheTimer), "21:21"),
        Paint::Plain
    );
    // An unknown TTL shows as it came; without one, the time alone.
    cache(&mut line).ttl = Some("2h".into());
    assert_eq!(en(&line, SegmentKind::CacheTimer), "cache →21:21 2h");
    cache(&mut line).ttl = None;
    assert_eq!(en(&line, SegmentKind::CacheTimer), "cache →21:21");
}

#[test]
fn the_cache_is_cold_unless_warm_and_ahead_of_its_expiry() {
    let mut line = sample();
    let expect_cold = |line: &Line| {
        assert_eq!(line.cache_state(), Some(CacheState::Cold));
        assert_eq!(en(line, SegmentKind::CacheTimer), "cache cold");
        assert_eq!(
            paint_of(line, Item::new(SegmentKind::CacheTimer), "cold"),
            Paint::Dim
        );
    };
    // The last response had no cache tokens: not warm, no expiry, still observed.
    cache(&mut line).warm = false;
    expect_cold(&line);
    cache(&mut line).warm = true;
    cache(&mut line).expires_at = None;
    expect_cold(&line);
    // Past its expiry, whatever `warm` says (the line was drawn before it ran out).
    cache(&mut line).expires_at = Some(Timestamp::from_millis(NOW - 1_000));
    expect_cold(&line);
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::CacheTimer, None),
        "cache soğuk"
    );
    // Caching never seen this session: no item at all.
    cache(&mut line).observed = false;
    assert_eq!(line.cache_state(), None);
    assert_eq!(en(&line, SegmentKind::CacheTimer), HIDDEN);
    line.facts.cache = None;
    assert_eq!(en(&line, SegmentKind::CacheTimer), HIDDEN);
}

#[cfg(windows)]
#[test]
fn the_cache_timer_uses_the_system_zone_on_windows() {
    let mut line = Line::sample(now(), false);
    line.zone = Zone::System;
    let Some(CacheState::Warm { until, .. }) = line.cache_state() else {
        panic!("warm");
    };
    assert!(until.is_some(), "the local time is known on Windows");
}

#[test]
fn misses_show_their_count_and_the_last_cause() {
    let mut line = sample();
    assert_eq!(
        en(&line, SegmentKind::CacheMisses),
        "2 misses · tools changed"
    );
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::CacheMisses, None),
        "2 kaçırma · araçlar değişti"
    );
    assert_eq!(
        one(
            Locale::En,
            &line,
            SegmentKind::CacheMisses,
            Some(Format::Count)
        ),
        "2 misses"
    );
    cache(&mut line).misses = Some(1);
    cache(&mut line).causes = vec![
        "ttl_expired_5m".into(),
        "likely_server_side".into(),
        "x".into(),
    ];
    assert_eq!(
        en(&line, SegmentKind::CacheMisses),
        "1 miss · 5m cache expired +2"
    );
    // A cause this build does not know: its name.
    cache(&mut line).causes = vec!["model_changed".into()];
    assert_eq!(
        en(&line, SegmentKind::CacheMisses),
        "1 miss · model_changed"
    );
    // No cause identified: the count alone.
    cache(&mut line).causes = Vec::new();
    assert_eq!(en(&line, SegmentKind::CacheMisses), "1 miss");
    // No misses: hidden, the count format too.
    cache(&mut line).misses = Some(0);
    assert_eq!(en(&line, SegmentKind::CacheMisses), HIDDEN);
    assert_eq!(
        one(
            Locale::En,
            &line,
            SegmentKind::CacheMisses,
            Some(Format::Count)
        ),
        HIDDEN
    );
}

#[test]
fn the_recache_cost_says_when_it_applies_and_hides_after_a_compaction() {
    let mut line = sample();
    assert_eq!(en(&line, SegmentKind::RecacheCost), "if cold 45k");
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::RecacheCost, None),
        "soğursa 45k"
    );
    // Not an estimate: no `≈`.
    assert!(!en(&line, SegmentKind::RecacheCost).contains('≈'));
    // Cold already: it is the next request's cost, so it stays.
    cache(&mut line).warm = false;
    assert_eq!(en(&line, SegmentKind::RecacheCost), "if cold 45k");
    cache(&mut line).recache = None;
    assert_eq!(en(&line, SegmentKind::RecacheCost), HIDDEN);
}

#[test]
fn the_session_cache_ratio_is_a_format_of_the_cache_ratio() {
    let mut line = sample();
    assert_eq!(en(&line, SegmentKind::CacheRatio), "cache 89%");
    let session = Some(Format::Session);
    assert_eq!(
        one(Locale::En, &line, SegmentKind::CacheRatio, session),
        "cache 91% (session)"
    );
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::CacheRatio, session),
        "cache %91 (oturum)"
    );
    cache(&mut line).hit_ratio = None;
    assert_eq!(
        one(Locale::En, &line, SegmentKind::CacheRatio, session),
        HIDDEN
    );
}

#[test]
fn the_spend_limit_shares_the_limits_formats_resets_and_colors() {
    let mut line = sample();
    let pct = Some(Format::Percent);
    assert_eq!(
        one(Locale::En, &line, SegmentKind::SpendLimit, pct),
        "$ 42% · 12d"
    );
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::SpendLimit, pct),
        "$ %42 · 12g"
    );
    assert_eq!(
        one(
            Locale::En,
            &line,
            SegmentKind::SpendLimit,
            Some(Format::Amount)
        ),
        "$21/$50 · 12d"
    );
    assert_eq!(
        one(
            Locale::En,
            &line,
            SegmentKind::SpendLimit,
            Some(Format::Remaining)
        ),
        "$ 58% left · 12d"
    );
    let bar = en(&line, SegmentKind::SpendLimit);
    assert!(bar.starts_with("$ ████░░░░ 42%"), "{bar}");
    // No time mark: the period's length is not always known.
    assert!(!bar.contains('⏱'), "{bar}");
    // The reset as a local time, as the windows do.
    let mut clock = item(SegmentKind::SpendLimit, pct);
    clock.reset_format = ResetFormat::Clock;
    let text = draw(Locale::En, &line, &alone(clock, IconSet::Text), None);
    assert_eq!(text, "$ 42% · Wed 21:20");
    // The thresholds of every limit.
    let spend = line.spend_limit.as_mut().expect("spend");
    spend.window.percent = 93.0;
    assert_eq!(
        paint_of(&line, item(SegmentKind::SpendLimit, pct), "93%"),
        Paint::Crit
    );
    // Past the limit.
    line.spend_limit.as_mut().expect("spend").window.percent = 104.0;
    assert_eq!(
        one(
            Locale::En,
            &line,
            SegmentKind::SpendLimit,
            Some(Format::Remaining)
        ),
        "$ 0% left · 12d"
    );
}

#[test]
fn the_amount_falls_back_to_the_percentage_without_its_dollars() {
    let mut line = sample();
    line.spend_limit.as_mut().expect("spend").used_usd = None;
    assert_eq!(
        one(
            Locale::En,
            &line,
            SegmentKind::SpendLimit,
            Some(Format::Amount)
        ),
        "$ 42% · 12d"
    );
    line.spend_limit = None;
    assert_eq!(en(&line, SegmentKind::SpendLimit), HIDDEN);
}

#[test]
fn the_spend_limit_comes_from_the_payload_and_starts_over_at_its_reset() {
    let p = StatuslinePayload::parse(DOCUMENTED.as_bytes()).expect("parse");
    // 2025-02-01 10:00 UTC: before the documented reset (2025-03-01).
    let before = Timestamp::from_millis(1_738_404_000_000);
    let line = Line::from_sources(Some(&p), None, before);
    let spend = line.spend_limit.expect("spend");
    assert_eq!(spend.window.percent, 62.8);
    assert_eq!(spend.window.source, Source::Statusline);
    assert_eq!(
        (spend.used_usd, spend.limit_usd),
        (Some(314.12), Some(500.0))
    );
    // After it: an estimated 0 %, and the old dollars are gone.
    let after = Timestamp::from_millis(1_740_787_200_000 + 1_000);
    let spend = Line::from_sources(Some(&p), None, after)
        .spend_limit
        .expect("spend");
    assert!(spend.window.estimated());
    assert_eq!(spend.window.percent, 0.0);
    assert_eq!(spend.used_usd, None);
    let mut line = Line::from_sources(Some(&p), None, after);
    line.zone = Zone::Fixed(0);
    assert_eq!(
        one(
            Locale::En,
            &line,
            SegmentKind::SpendLimit,
            Some(Format::Amount)
        ),
        "≈$ 0%"
    );
}

#[test]
fn the_pr_shows_its_number_and_review_state() {
    let mut line = sample();
    assert_eq!(en(&line, SegmentKind::Pr), "#123 ✓");
    assert_eq!(
        one(Locale::En, &line, SegmentKind::Pr, Some(Format::Full)),
        "#123 approved"
    );
    assert_eq!(
        one(Locale::Tr, &line, SegmentKind::Pr, Some(Format::Full)),
        "#123 onaylandı"
    );
    assert_eq!(paint_of(&line, Item::new(SegmentKind::Pr), "✓"), Paint::Ok);
    let set = |line: &mut Line, review: Option<Review>, mr: bool| {
        line.facts.pr = Some(PullRequest {
            number: 45,
            review,
            mr,
        });
    };
    set(&mut line, Some(Review::ChangesRequested), false);
    assert_eq!(en(&line, SegmentKind::Pr), "#45 ⨯");
    assert_eq!(
        paint_of(&line, Item::new(SegmentKind::Pr), "⨯"),
        Paint::Crit
    );
    set(&mut line, Some(Review::Pending), false);
    assert_eq!(en(&line, SegmentKind::Pr), "#45 ∙");
    assert_eq!(
        paint_of(&line, Item::new(SegmentKind::Pr), "∙"),
        Paint::Warn
    );
    // A GitLab merge request, draft: `!` and the word, faint.
    set(&mut line, Some(Review::Draft), true);
    assert_eq!(en(&line, SegmentKind::Pr), "!45 draft");
    assert_eq!(one(Locale::Tr, &line, SegmentKind::Pr, None), "!45 taslak");
    assert_eq!(
        paint_of(&line, Item::new(SegmentKind::Pr), "draft"),
        Paint::Dim
    );
    // No state: the number alone. One this build does not know: as it came.
    set(&mut line, None, false);
    assert_eq!(en(&line, SegmentKind::Pr), "#45");
    set(&mut line, Some(Review::Other("queued".into())), false);
    assert_eq!(en(&line, SegmentKind::Pr), "#45 queued");
    line.facts.pr = None;
    assert_eq!(en(&line, SegmentKind::Pr), HIDDEN);
}

#[test]
fn the_nerd_set_draws_review_states_as_octicons() {
    let line = sample();
    let nerd = draw(
        Locale::En,
        &line,
        &alone(Item::new(SegmentKind::Pr), IconSet::Nerd),
        None,
    );
    assert_eq!(nerd, "\u{F407} #123 \u{F42E}");
}

#[test]
fn worktree_and_repo_show_what_the_payload_names() {
    let mut line = sample();
    assert_eq!(en(&line, SegmentKind::Worktree), "wt feature-x");
    assert_eq!(en(&line, SegmentKind::Repo), "ada/alpha");
    assert_eq!(
        one(Locale::En, &line, SegmentKind::Repo, Some(Format::Name)),
        "alpha"
    );
    assert_eq!(
        one(Locale::En, &line, SegmentKind::Repo, Some(Format::Host)),
        "github.com/ada/alpha"
    );
    line.facts.repo = Some(Repo {
        host: None,
        owner: None,
        name: "alpha".into(),
    });
    assert_eq!(
        one(Locale::En, &line, SegmentKind::Repo, Some(Format::Host)),
        "alpha"
    );
    // The main working tree, and no `origin`: nothing.
    line.facts.worktree = None;
    line.facts.repo = None;
    assert_eq!(en(&line, SegmentKind::Worktree), HIDDEN);
    assert_eq!(en(&line, SegmentKind::Repo), HIDDEN);
}

#[test]
fn folders_can_count_the_added_ones() {
    let mut line = sample();
    let with = Some(Format::WithAdded);
    assert_eq!(
        one(Locale::En, &line, SegmentKind::ProjectDir, with),
        "alpha +2"
    );
    assert_eq!(one(Locale::En, &line, SegmentKind::Cwd, with), "src +2");
    line.facts.added_dirs = 0;
    assert_eq!(
        one(Locale::En, &line, SegmentKind::ProjectDir, with),
        "alpha"
    );
}

#[test]
fn the_documented_payload_fills_every_new_fact() {
    let p = StatuslinePayload::parse(DOCUMENTED.as_bytes()).expect("parse");
    let line = Line::from_sources(Some(&p), None, Timestamp::from_millis(1_738_428_000_000));
    let f = &line.facts;
    assert_eq!(f.effort.as_deref(), Some("max"));
    assert!(f.fast_mode);
    assert_eq!(f.thinking, Some(false));
    let cache = f.cache.as_ref().expect("cache");
    assert!(cache.observed && cache.warm);
    assert_eq!(cache.misses, Some(2));
    assert_eq!(cache.hit_ratio, Some(91.0));
    assert_eq!(cache.recache, Some(45_000));
    assert_eq!(line.miss_cause(), Some(("tools_changed", 1)));
    // 20 minutes before its expiry.
    let Some(CacheState::Warm {
        expires_in, ending, ..
    }) = line.cache_state()
    else {
        panic!("warm");
    };
    assert_eq!((expires_in, ending), (1_200_000, false));
    assert_eq!(
        f.pr,
        Some(PullRequest {
            number: 45,
            review: Some(Review::Draft),
            mr: true,
        })
    );
    // `worktree.name` wins over `workspace.git_worktree`.
    assert_eq!(f.worktree.as_deref(), Some("my-feature"));
    assert_eq!(
        f.repo
            .as_ref()
            .map(|r| (r.owner.as_deref(), r.name.as_str())),
        Some((Some("group/subgroup"), "alpha"))
    );
    assert_eq!(f.added_dirs, 2);
}

#[test]
fn the_linked_worktree_stands_in_outside_a_worktree_session() {
    let mut p = StatuslinePayload::parse(DOCUMENTED.as_bytes()).expect("parse");
    p.worktree = None;
    let line = Line::from_sources(Some(&p), None, now());
    assert_eq!(line.facts.worktree.as_deref(), Some("feature-xyz"));
}

#[test]
fn without_the_new_fields_nothing_new_shows() {
    // The 12m sample payload has an expiry but none of the other cache fields.
    let p = StatuslinePayload::parse(NO_FIVE_HOUR.as_bytes()).expect("parse");
    let line = Line::from_sources(Some(&p), None, now());
    for kind in [
        SegmentKind::Effort,
        SegmentKind::CacheTimer,
        SegmentKind::CacheMisses,
        SegmentKind::RecacheCost,
        SegmentKind::SpendLimit,
        SegmentKind::Pr,
        SegmentKind::Worktree,
    ] {
        assert_eq!(en(&line, kind), HIDDEN, "{kind:?}");
    }
}

#[test]
fn a_narrow_terminal_drops_the_session_items_before_the_spend_limit() {
    let line = sample();
    let mut spend = item(SegmentKind::SpendLimit, Some(Format::Percent));
    spend.reset = false;
    let prefs = StatuslinePrefs {
        items: with_separators(
            vec![
                spend,
                Item::new(SegmentKind::Effort),
                Item::new(SegmentKind::CacheTimer),
            ],
            "│",
        ),
        ..StatuslinePrefs::default()
    }
    .normalized();
    let full = draw(Locale::En, &line, &prefs, None);
    assert_eq!(full, "$ 42% │ high │ cache →22:07 1h");
    let n = full.chars().count();
    assert_eq!(draw(Locale::En, &line, &prefs, cols(n - 1)), "$ 42% │ high");
    assert_eq!(draw(Locale::En, &line, &prefs, cols(6)), "$ 42%");
}

#[test]
fn new_marks_take_one_cell_in_every_terminal_and_nerd_ones_are_in_the_pua() {
    use unicode_width::UnicodeWidthChar;
    for c in ['✓', '⨯', '∙', '→'] {
        assert_eq!(c.width(), Some(1), "{c:?}");
    }
    // `→` is "Ambiguous" like the `·` and `≈` the line already uses; the review marks are not.
    for c in ['✓', '⨯', '∙'] {
        assert_eq!(c.width_cjk(), Some(1), "{c:?} is ambiguous");
    }
    for kind in [
        SegmentKind::Effort,
        SegmentKind::CacheTimer,
        SegmentKind::CacheMisses,
        SegmentKind::RecacheCost,
        SegmentKind::Pr,
        SegmentKind::Worktree,
        SegmentKind::Repo,
    ] {
        let icon = kind.icon(IconSet::Nerd).expect("an icon");
        assert!(
            icon.chars().all(|c| ('\u{E000}'..='\u{F8FF}').contains(&c)),
            "{kind:?}"
        );
        assert_eq!(kind.icon(IconSet::Unicode), None, "{kind:?}");
    }
}

#[test]
fn the_new_kinds_and_formats_read_back_and_normalize() {
    for (name, kind, group) in [
        ("effort", SegmentKind::Effort, Group::Session),
        ("cache_timer", SegmentKind::CacheTimer, Group::Session),
        ("cache_misses", SegmentKind::CacheMisses, Group::Session),
        ("recache_cost", SegmentKind::RecacheCost, Group::Session),
        ("spend_limit", SegmentKind::SpendLimit, Group::Limit),
        ("pr", SegmentKind::Pr, Group::Project),
        ("worktree", SegmentKind::Worktree, Group::Project),
        ("repo", SegmentKind::Repo, Group::Project),
    ] {
        assert_eq!(SegmentKind::parse(name), Some(kind));
        assert_eq!(kind.group(), group, "{name}");
    }
    assert!(SegmentKind::SpendLimit.is_limit() && SegmentKind::SpendLimit.has_bar());
    assert!(SegmentKind::SpendLimit.has_reset() && !SegmentKind::SpendLimit.is_window());
    // The cache timer's countdown comes in Step 12r: one look for now.
    assert_eq!(SegmentKind::CacheTimer.formats(), &[] as &[Format]);
    assert_eq!(SegmentKind::Effort.default_format(), Some(Format::Level));
    assert_eq!(
        SegmentKind::CacheMisses.default_format(),
        Some(Format::Cause)
    );
    assert_eq!(SegmentKind::Repo.default_format(), Some(Format::Full));
    // The older kinds keep their defaults: lines written before stay as they were.
    assert_eq!(SegmentKind::Model.default_format(), Some(Format::Short));
    assert_eq!(SegmentKind::CacheRatio.default_format(), Some(Format::Last));
    let toml = r#"
version = 2
[[items]]
kind = "spend_limit"
format = "amount"
reset_format = "clock"
pace = false
[[items]]
kind = "model"
format = "with_effort"
[[items]]
kind = "repo"
format = "with_effort"
[[items]]
kind = "cache_ratio"
"#;
    let prefs: StatuslinePrefs = toml::from_str(toml).expect("prefs");
    let spend = &prefs.items[0];
    assert_eq!(spend.format, Some(Format::Amount));
    // The spend limit keeps its reset format; it has no time mark to turn off.
    assert_eq!(spend.reset_format, ResetFormat::Clock);
    assert!(spend.pace);
    assert_eq!(prefs.items[1].format, Some(Format::WithEffort));
    // A format the kind does not take: its default.
    assert_eq!(prefs.items[2].format, Some(Format::Full));
    // Written back: only what differs from the defaults.
    let mut doc = toml_edit::DocumentMut::new();
    let mut table = toml_edit::Table::new();
    prefs.write_toml(&mut table, false);
    doc["statusline"] = toml_edit::Item::Table(table);
    let text = doc.to_string();
    assert!(text.contains("format = \"amount\""), "{text}");
    assert!(text.contains("reset_format = \"clock\""), "{text}");
    assert!(!text.contains("format = \"last\""), "{text}");
    assert!(!text.contains("format = \"full\""), "{text}");
}

#[test]
fn the_detailed_preset_takes_effort_and_the_cache_timer_and_the_rest_stay() {
    use crate::statusline::line::Preset;
    let detailed = Preset::Detailed.prefs();
    assert!(detailed.shows(SegmentKind::Effort));
    assert!(detailed.shows(SegmentKind::CacheTimer));
    for preset in [
        Preset::Default,
        Preset::Minimal,
        Preset::TwoLine,
        Preset::Powerline,
        Preset::Split,
    ] {
        let prefs = preset.prefs();
        for kind in [SegmentKind::Effort, SegmentKind::CacheTimer] {
            assert!(!prefs.shows(kind), "{preset:?} {kind:?}");
        }
        // The model keeps its name alone.
        assert!(
            prefs
                .items
                .iter()
                .all(|i| i.format != Some(Format::WithEffort)),
            "{preset:?}"
        );
    }
}

#[test]
fn the_sample_fills_every_new_item() {
    let line = sample();
    for kind in [
        SegmentKind::Effort,
        SegmentKind::CacheTimer,
        SegmentKind::CacheMisses,
        SegmentKind::RecacheCost,
        SegmentKind::SpendLimit,
        SegmentKind::Pr,
        SegmentKind::Worktree,
        SegmentKind::Repo,
    ] {
        for state in [
            SampleState::Normal,
            SampleState::Warn,
            SampleState::Crit,
            SampleState::Stale,
        ] {
            let mut line = Line::sample_in(now(), false, state);
            line.zone = Zone::Fixed(0);
            assert_ne!(en(&line, kind), HIDDEN, "{kind:?} {state:?}");
        }
        assert_ne!(en(&line, kind), HIDDEN, "{kind:?}");
    }
}
