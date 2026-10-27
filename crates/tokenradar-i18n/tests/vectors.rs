//! Runs the shared `testdata/i18n-vectors.json` against the Rust formatters (architecture §12).
//! The TypeScript side runs the same file (Step 09).

use serde_json::Value;
use tokenradar_i18n::format::{self, DurStyle, HourCycle, Weekday};
use tokenradar_i18n::{Case, Locale, suffix};

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/i18n-vectors.json"
    );
    let text = std::fs::read_to_string(path).expect("i18n-vectors.json is readable");
    serde_json::from_str(&text).expect("i18n-vectors.json is valid JSON")
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    let items = v[key]
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` is a list"));
    assert!(!items.is_empty(), "`{key}` has vectors");
    items
}

fn locale(v: &Value) -> Locale {
    Locale::from_tag(v["locale"].as_str().expect("locale"))
}

fn out(v: &Value) -> &str {
    v["out"].as_str().expect("out")
}

fn f64_of(v: &Value, key: &str) -> f64 {
    v[key].as_f64().unwrap_or_else(|| panic!("{key} in {v}"))
}

fn u8_of(v: &Value, key: &str) -> u8 {
    v[key]
        .as_u64()
        .and_then(|n| u8::try_from(n).ok())
        .unwrap_or_else(|| panic!("{key} in {v}"))
}

#[test]
fn schema_is_known() {
    assert_eq!(vectors()["schema"], 1);
}

#[test]
fn num_vectors() {
    let v = vectors();
    for case in list(&v, "num") {
        let decimals = usize::from(u8_of(case, "decimals"));
        let got = format::num(locale(case), f64_of(case, "n"), decimals);
        assert_eq!(got, out(case), "{case}");
    }
}

#[test]
fn pct_vectors() {
    let v = vectors();
    for case in list(&v, "pct") {
        assert_eq!(
            format::pct(locale(case), f64_of(case, "n")),
            out(case),
            "{case}"
        );
    }
}

#[test]
fn dur_vectors() {
    let v = vectors();
    for case in list(&v, "dur") {
        let ms = case["ms"].as_i64().expect("ms");
        let style = DurStyle::parse(case["style"].as_str().expect("style"));
        assert_eq!(format::dur(locale(case), ms, style), out(case), "{case}");
    }
}

#[test]
fn clock_vectors() {
    let v = vectors();
    for case in list(&v, "clock") {
        let cycle = if case["hour12"].as_bool().expect("hour12") {
            HourCycle::H12
        } else {
            HourCycle::H23
        };
        let weekday = Weekday::from_sunday0(u16::from(u8_of(case, "weekday")));
        let got = format::weekday_clock(
            locale(case),
            weekday,
            u8_of(case, "hour"),
            u8_of(case, "minute"),
            cycle,
        );
        assert_eq!(got, out(case), "{case}");
    }
}

/// Numbers go through `NUM` first, exactly like the `TR_SUFFIX` Fluent function does.
#[test]
fn tr_suffix_vectors() {
    let v = vectors();
    for case in list(&v, "tr_suffix") {
        let text = match &case["input"] {
            Value::String(s) => s.clone(),
            n => format::num(Locale::Tr, n.as_f64().expect("number input"), 0),
        };
        let kind = Case::parse(case["case"].as_str().expect("case")).expect("known case");
        assert_eq!(suffix::tr_suffix(&text, kind), out(case), "{case}");
    }
}
