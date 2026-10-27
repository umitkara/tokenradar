// Token counts in these vectors are far below 2^52.
#![allow(clippy::cast_precision_loss)]

use super::*;
use crate::domain::TokenCounts;

fn usage(tokens: TokenCounts) -> Usage {
    Usage {
        tokens,
        ..Usage::default()
    }
}

fn cost(model: &str, usage: &Usage) -> i64 {
    Pricing::claude().cost(model, usage).cost.0
}

/// The record from `docs/data-sources.md` S3.
fn sample_tokens() -> TokenCounts {
    TokenCounts {
        input: 2,
        output: 1117,
        cache_read: 19_701,
        cache_w5m: 0,
        cache_w1h: 13_986,
        thinking: 511,
    }
}

#[test]
fn embedded_table_parses() {
    let pricing = Pricing::claude();
    assert_eq!(pricing.version(), "2026-10-27");
    assert_eq!(pricing.models[pricing.fallback].id, "claude-opus-5-5");
}

/// Every row of the official table, `$/MTok`:
/// (model, base input, 5m cache write, 1h cache write, cache hit, output).
#[test]
fn table_matches_the_published_prices() {
    #[rustfmt::skip]
    let published: &[(&str, f64, f64, f64, f64, f64)] = &[
        ("claude-fable-5-1",  10.0, 12.50, 20.0, 0.25, 50.0),
        ("claude-mythos-5-1", 10.0, 12.50, 20.0, 0.25, 50.0),
        ("claude-fable-5",    10.0, 12.50, 20.0, 1.00, 50.0),
        ("claude-mythos-5",   10.0, 12.50, 20.0, 1.00, 50.0),
        ("claude-opus-5-5",    4.0,  5.00,  8.0, 0.20, 20.0),
        ("claude-opus-5",      5.0,  6.25, 10.0, 0.50, 25.0),
        ("claude-opus-4-8",    5.0,  6.25, 10.0, 0.50, 25.0),
        ("claude-opus-4-7",    5.0,  6.25, 10.0, 0.50, 25.0),
        ("claude-opus-4-6",    5.0,  6.25, 10.0, 0.50, 25.0),
        ("claude-opus-4-5",    5.0,  6.25, 10.0, 0.50, 25.0),
        ("claude-opus-4-1",   15.0, 18.75, 30.0, 1.50, 75.0),
        ("claude-opus-4",     15.0, 18.75, 30.0, 1.50, 75.0),
        ("claude-sonnet-5-5",  2.0,  2.50,  4.0, 0.10, 10.0),
        ("claude-sonnet-5",    2.0,  2.50,  4.0, 0.20, 10.0),
        ("claude-sonnet-4-6",  3.0,  3.75,  6.0, 0.30, 15.0),
        ("claude-sonnet-4-5",  3.0,  3.75,  6.0, 0.30, 15.0),
        ("claude-sonnet-4",    3.0,  3.75,  6.0, 0.30, 15.0),
        ("claude-haiku-5-5",   0.10, 0.125, 0.20, 0.01, 0.50),
        ("claude-haiku-4-5",   1.0,  1.25,  2.0, 0.10,  5.0),
        ("claude-3-5-haiku",   0.80, 1.00,  1.60, 0.08, 4.0),
    ];
    // 100k tokens stay at or under Haiku 5.5's long-prompt threshold.
    for &row in published {
        check_row(row, 100_000);
    }
    // Haiku 5.5 above 100,000 prompt tokens.
    check_row(
        ("claude-haiku-5-5", 0.50, 0.625, 1.0, 0.05, 2.50),
        1_000_000,
    );
}

/// Prices `n` tokens of each category on its own and compares with `price × n`.
fn check_row((model, input, w5m, w1h, read, output): (&str, f64, f64, f64, f64, f64), n: u64) {
    let expected = |usd_per_mtok: f64| MicroUsd::from_f64_rounded(usd_per_mtok * n as f64).0;
    let d = TokenCounts::default();
    let cases = [
        ("input", TokenCounts { input: n, ..d }, input),
        ("5m write", TokenCounts { cache_w5m: n, ..d }, w5m),
        ("1h write", TokenCounts { cache_w1h: n, ..d }, w1h),
        ("cache hit", TokenCounts { cache_read: n, ..d }, read),
        // Output does not count towards the prompt size; pad the prompt for the long tier.
        (
            "output",
            TokenCounts {
                output: n,
                cache_read: n.min(100_001),
                ..d
            },
            output,
        ),
    ];
    for (what, tokens, price) in cases {
        let mut got = cost(model, &usage(tokens));
        if what == "output" {
            got -= cost(
                model,
                &usage(TokenCounts {
                    cache_read: tokens.cache_read,
                    ..d
                }),
            );
        }
        assert_eq!(got, expected(price), "{model} {what} ({n} tokens)");
    }
    assert!(Pricing::claude().cost(model, &usage(d)).model_known);
}

#[test]
fn record_with_cache_creation_breakdown() {
    // 4 × (2 + 2.0·13986 + 0.05·19701) + 20 × 1117 = 138176.2 µ$
    assert_eq!(cost("claude-opus-5-5", &usage(sample_tokens())), 138_176);
}

#[test]
fn record_without_breakdown_counts_writes_as_5m() {
    let (w5m, w1h) = TokenCounts::split_cache_creation(13_986, None);
    let tokens = TokenCounts {
        cache_w5m: w5m,
        cache_w1h: w1h,
        ..sample_tokens()
    };
    // 4 × (2 + 1.25·13986 + 0.05·19701) + 20 × 1117 = 96218.2 µ$
    assert_eq!(cost("claude-opus-5-5", &usage(tokens)), 96_218);
}

#[test]
fn fast_mode_doubles_every_token_category() {
    let fast = Usage {
        speed: Speed::Fast,
        ..usage(sample_tokens())
    };
    // 2 × 138176.2
    assert_eq!(cost("claude-opus-5-5", &fast), 276_352);
}

#[test]
fn thinking_tokens_are_not_billed_twice() {
    let a = sample_tokens();
    let b = TokenCounts { thinking: 0, ..a };
    assert_eq!(
        cost("claude-opus-5-5", &usage(a)),
        cost("claude-opus-5-5", &usage(b))
    );
}

#[test]
fn haiku_5_5_long_prompts_pay_long_context_prices() {
    let at_threshold = TokenCounts {
        input: 50_000,
        cache_read: 50_000,
        output: 1000,
        ..Default::default()
    };
    // 0.10 × (50000 + 0.1·50000) + 0.50 × 1000
    assert_eq!(cost("claude-haiku-5-5", &usage(at_threshold)), 6_000);

    let over = TokenCounts {
        cache_read: 60_000,
        ..at_threshold
    };
    // 0.50 × (50000 + 0.1·60000) + 2.50 × 1000
    assert_eq!(cost("claude-haiku-5-5", &usage(over)), 30_500);
}

#[test]
fn web_search_is_billed_per_search_and_fetch_is_free() {
    let u = Usage {
        web_search: 3,
        web_fetch: 7,
        ..Usage::default()
    };
    assert_eq!(cost("claude-sonnet-5", &u), 30_000);
}

#[test]
fn model_names_match_with_platform_and_date_suffixes() {
    let p = Pricing::claude();
    let id_of = |name: &str| p.find(name).map(|m| m.id.as_str());
    assert_eq!(id_of("claude-haiku-4-5-20251001"), Some("claude-haiku-4-5"));
    assert_eq!(id_of("claude-3-5-haiku-20241022"), Some("claude-3-5-haiku"));
    assert_eq!(id_of("claude-opus-4-1-20250805"), Some("claude-opus-4-1"));
    assert_eq!(id_of("claude-opus-4-20250514"), Some("claude-opus-4"));
    assert_eq!(
        id_of("us.anthropic.claude-opus-4-8-v1:0"),
        Some("claude-opus-4-8")
    );
    assert_eq!(id_of("claude-opus-4-5@20251101"), Some("claude-opus-4-5"));
    assert_eq!(id_of("claude-opus-4-6[1m]"), Some("claude-opus-4-6"));
    // A plain prefix is not a match.
    assert_eq!(id_of("claude-opus-5-7"), None);
    assert_eq!(id_of("claude-opus-55"), None);
}

#[test]
fn unknown_models_use_a_flagged_stand_in() {
    let tokens = TokenCounts {
        input: 1000,
        output: 100,
        ..Default::default()
    };
    let p = Pricing::claude();

    let future_opus = p.cost("claude-opus-6", &usage(tokens));
    assert!(!future_opus.model_known);
    assert_eq!(
        future_opus.cost,
        p.cost("claude-opus-5-5", &usage(tokens)).cost
    );

    let future_haiku = p.cost("claude-haiku-6", &usage(tokens));
    assert_eq!(
        future_haiku.cost,
        p.cost("claude-haiku-5-5", &usage(tokens)).cost
    );

    let foreign = p.cost("gpt-5-codex", &usage(tokens));
    assert!(!foreign.model_known);
    assert_eq!(foreign.cost, p.cost("claude-opus-5-5", &usage(tokens)).cost);
}

#[test]
fn families_are_detected_by_pattern() {
    assert_eq!(family_of("claude-opus-5-5"), ModelFamily::Opus);
    assert_eq!(family_of("claude-sonnet-9"), ModelFamily::Sonnet);
    assert_eq!(family_of("claude-3-5-haiku-20241022"), ModelFamily::Haiku);
    assert_eq!(family_of("claude-fable-6"), ModelFamily::Fable);
    assert_eq!(family_of("claude-mythos-preview"), ModelFamily::Mythos);
    assert_eq!(family_of("<synthetic>"), ModelFamily::Synthetic);
    assert_eq!(family_of("gpt-5-codex"), ModelFamily::Unknown);
}

#[test]
fn synthetic_records_cost_nothing() {
    let priced = Pricing::claude().cost("<synthetic>", &usage(sample_tokens()));
    assert_eq!(
        priced,
        Priced {
            cost: MicroUsd::ZERO,
            model_known: true
        }
    );
}

#[test]
fn malformed_tables_are_rejected() {
    assert!(matches!(
        Pricing::from_toml("schema = 2"),
        Err(PricingError::Toml(_))
    ));
    let bad_fallback =
        CLAUDE_TOML.replace(r#"fallback = "claude-opus-5-5""#, r#"fallback = "nope""#);
    assert!(matches!(
        Pricing::from_toml(&bad_fallback),
        Err(PricingError::MissingFallback(_))
    ));
}
