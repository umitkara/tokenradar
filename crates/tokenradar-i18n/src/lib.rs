//! Localization for the Rust side of TokenRadar (tray tooltip, notifications, CLI).
//!
//! Translations live in `locales/{en,tr}/*.ftl` and are shared with the frontend. They are
//! embedded at compile time, so the CLI needs no files next to it. Each caller loads only the
//! [`Domain`]s it uses: the statusline runs on every Claude Code update and should not parse
//! the tray's or the notifier's messages.
//!
//! Custom Fluent functions (architecture §12): `NUM(n, decimals)`, `PCT(n)`, `DUR(ms, style)`
//! and `TR_SUFFIX(value, case)`. `TR_SUFFIX` takes a number or an already formatted string, so
//! the suffix matches what is shown: `{ TR_SUFFIX(PCT($pct), "poss") }` → `%72'si`. In English
//! it returns its input unchanged.

pub mod format;
pub mod suffix;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

pub use format::{DurStyle, HourCycle, Weekday};
pub use suffix::Case;

/// A supported UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    #[default]
    En,
    Tr,
}

impl Locale {
    /// BCP 47 language tag, matching the `locales/` directory name.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Tr => "tr",
        }
    }

    /// `tr`, `tr-TR`, `TR_tr`… are Turkish; everything else is English.
    pub fn from_tag(tag: &str) -> Self {
        if tag.get(..2).is_some_and(|p| p.eq_ignore_ascii_case("tr")) {
            Self::Tr
        } else {
            Self::En
        }
    }

    /// The operating system's UI language.
    pub fn detect() -> Self {
        sys_locale::get_locale().map_or(Self::En, |tag| Self::from_tag(&tag))
    }
}

/// A group of messages, one `.ftl` file per locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    /// The `tokenradar` console binary and the statusline.
    Cli,
    /// The tray icon's tooltip and menu.
    Tray,
    /// Desktop notifications.
    Notify,
}

impl Domain {
    pub const ALL: [Self; 3] = [Self::Cli, Self::Tray, Self::Notify];

    /// Embedded Fluent source.
    pub const fn source(self, locale: Locale) -> &'static str {
        match (self, locale) {
            (Self::Cli, Locale::En) => include_str!("../../../locales/en/cli.ftl"),
            (Self::Cli, Locale::Tr) => include_str!("../../../locales/tr/cli.ftl"),
            (Self::Tray, Locale::En) => include_str!("../../../locales/en/tray.ftl"),
            (Self::Tray, Locale::Tr) => include_str!("../../../locales/tr/tray.ftl"),
            (Self::Notify, Locale::En) => include_str!("../../../locales/en/notify.ftl"),
            (Self::Notify, Locale::Tr) => include_str!("../../../locales/tr/notify.ftl"),
        }
    }
}

/// A Fluent argument value.
#[derive(Debug, Clone, Copy)]
pub enum Arg<'a> {
    Num(f64),
    Str(&'a str),
}

impl From<f64> for Arg<'_> {
    fn from(n: f64) -> Self {
        Self::Num(n)
    }
}

impl From<i64> for Arg<'_> {
    #[allow(clippy::cast_precision_loss)]
    fn from(n: i64) -> Self {
        Self::Num(n as f64)
    }
}

impl<'a> From<&'a str> for Arg<'a> {
    fn from(s: &'a str) -> Self {
        Self::Str(s)
    }
}

/// Formats localized messages for one locale.
pub struct Localizer {
    locale: Locale,
    bundle: FluentBundle<FluentResource>,
}

impl std::fmt::Debug for Localizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Localizer")
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl Localizer {
    /// Loads `domains` for `locale`. The embedded files are checked by tests, so a parse
    /// error here is a build defect; the faulty entries are skipped rather than panicking.
    pub fn new(locale: Locale, domains: &[Domain]) -> Self {
        let lang: LanguageIdentifier = locale.tag().parse().unwrap_or_default();
        let mut bundle = FluentBundle::new(vec![lang]);
        // Unicode isolation marks would show up as garbage in terminals and tooltips.
        bundle.set_use_isolating(false);
        register_functions(&mut bundle, locale);
        for domain in domains {
            let resource = FluentResource::try_new(domain.source(locale).to_owned())
                .unwrap_or_else(|(partial, _)| partial);
            // Duplicate ids across domains keep the first definition.
            let _ = bundle.add_resource(resource);
        }
        Self { locale, bundle }
    }

    pub const fn locale(&self) -> Locale {
        self.locale
    }

    /// The message `key` with `args`. A missing key returns the key itself so the gap is
    /// visible instead of silent.
    pub fn fmt(&self, key: &str, args: &[(&str, Arg<'_>)]) -> String {
        let Some(pattern) = self.bundle.get_message(key).and_then(|m| m.value()) else {
            return key.to_owned();
        };
        let mut fargs = FluentArgs::with_capacity(args.len());
        for (name, value) in args {
            match *value {
                Arg::Num(n) => fargs.set(*name, FluentValue::from(n)),
                Arg::Str(s) => fargs.set(*name, FluentValue::from(s)),
            }
        }
        let mut errors = Vec::new();
        let out = self
            .bundle
            .format_pattern(pattern, Some(&fargs), &mut errors);
        debug_assert!(errors.is_empty(), "{key}: {errors:?}");
        out.into_owned()
    }

    /// [`Self::fmt`] without arguments.
    pub fn msg(&self, key: &str) -> String {
        self.fmt(key, &[])
    }
}

fn register_functions(bundle: &mut FluentBundle<FluentResource>, locale: Locale) {
    let _ = bundle.add_function("NUM", move |pos, _| {
        let n = number(pos.first());
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let decimals = number(pos.get(1)).clamp(0.0, 6.0) as usize;
        FluentValue::from(format::num(locale, n, decimals))
    });
    let _ = bundle.add_function("PCT", move |pos, _| {
        FluentValue::from(format::pct(locale, number(pos.first())))
    });
    let _ = bundle.add_function("DUR", move |pos, _| {
        #[allow(clippy::cast_possible_truncation)]
        let ms = number(pos.first()) as i64;
        let style = match pos.get(1) {
            Some(FluentValue::String(s)) => DurStyle::parse(s),
            _ => DurStyle::Short,
        };
        FluentValue::from(format::dur(locale, ms, style))
    });
    let _ = bundle.add_function("TR_SUFFIX", move |pos, _| {
        let text = match pos.first() {
            Some(FluentValue::String(s)) => s.to_string(),
            Some(FluentValue::Number(n)) => {
                format::num(locale, n.value, shortest_decimals(n.value))
            }
            _ => String::new(),
        };
        let case = match pos.get(1) {
            Some(FluentValue::String(s)) => Case::parse(s),
            _ => None,
        };
        match (locale, case) {
            (Locale::Tr, Some(case)) => FluentValue::from(suffix::tr_suffix(&text, case)),
            _ => FluentValue::from(text),
        }
    });
}

/// Fraction digits needed to print `n` without trailing zeros (at most 6).
fn shortest_decimals(n: f64) -> usize {
    (0..6)
        .find(|&d| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let scale = 10f64.powi(d as i32);
            ((n * scale).round() - n * scale).abs() < 1e-9
        })
        .unwrap_or(6)
}

fn number(v: Option<&FluentValue<'_>>) -> f64 {
    match v {
        Some(FluentValue::Number(n)) => n.value,
        Some(FluentValue::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[test]
    fn locale_tags_match_locale_directories() {
        assert_eq!(Locale::En.tag(), "en");
        assert_eq!(Locale::Tr.tag(), "tr");
    }

    #[test]
    fn locale_tags_are_matched_by_language() {
        assert_eq!(Locale::from_tag("tr-TR"), Locale::Tr);
        assert_eq!(Locale::from_tag("TR"), Locale::Tr);
        assert_eq!(Locale::from_tag("en-US"), Locale::En);
        assert_eq!(Locale::from_tag("de"), Locale::En);
        assert_eq!(Locale::from_tag(""), Locale::En);
    }

    #[test]
    fn localizer_keeps_its_locale() {
        assert_eq!(Localizer::new(Locale::Tr, &[]).locale(), Locale::Tr);
    }

    #[test]
    fn missing_keys_are_visible() {
        let l10n = Localizer::new(Locale::En, &Domain::ALL);
        assert_eq!(l10n.msg("no-such-key"), "no-such-key");
    }

    #[test]
    fn custom_functions_reach_the_messages() {
        let tr = Localizer::new(Locale::Tr, &Domain::ALL);
        let en = Localizer::new(Locale::En, &Domain::ALL);
        let args = [("pct", Arg::Num(9.0))];
        assert_eq!(tr.fmt("sl-session-share", &args), "≈%9 bu oturum");
        assert_eq!(en.fmt("sl-session-share", &args), "≈9% this session");
        let args = [("ms", Arg::Num(108.0 * 60_000.0))];
        assert_eq!(tr.fmt("sl-resets-in", &args), "1sa48dk");
    }

    #[test]
    fn tr_suffix_follows_the_formatted_value() {
        let tr = Localizer::new(Locale::Tr, &Domain::ALL);
        let en = Localizer::new(Locale::En, &Domain::ALL);
        let until = [("until", Arg::Str("10:40"))];
        assert_eq!(
            tr.fmt("tray-muted", &until),
            "Bildirimler 10:40'a kadar susturuldu"
        );
        assert_eq!(
            en.fmt("tray-muted", &until),
            "Notifications muted until 10:40"
        );
        let weekly = [("value", Arg::Str("%34")), ("at", Arg::Str("Çar 10:59"))];
        assert_eq!(
            tr.fmt("tray-weekly-resets", &weekly),
            "Haftalık: %34 · Çar 10:59'da yenilenir"
        );
    }

    #[test]
    fn tr_suffix_formats_bare_numbers() {
        let mut bundle = FluentBundle::new(vec![LanguageIdentifier::default()]);
        bundle.set_use_isolating(false);
        register_functions(&mut bundle, Locale::Tr);
        let src = "a = { TR_SUFFIX(1000000, \"abl\") }\nb = { TR_SUFFIX(23.5, \"loc\") }\nc = { TR_SUFFIX(5, \"nope\") }\n";
        let _ = bundle.add_resource(FluentResource::try_new(src.to_owned()).expect("valid"));
        let fmt = |id: &str| {
            let msg = bundle
                .get_message(id)
                .and_then(|m| m.value())
                .expect("message");
            bundle
                .format_pattern(msg, None, &mut Vec::new())
                .into_owned()
        };
        assert_eq!(fmt("a"), "1.000.000'dan");
        assert_eq!(fmt("b"), "23,5'te");
        assert_eq!(fmt("c"), "5");
    }

    /// Every embedded file parses, and both locales define the same messages with the same
    /// variables (architecture §12, parity).
    #[test]
    fn locales_are_in_parity() {
        for domain in Domain::ALL {
            let en = messages(domain.source(Locale::En));
            let tr = messages(domain.source(Locale::Tr));
            assert_eq!(
                en.keys().collect::<Vec<_>>(),
                tr.keys().collect::<Vec<_>>(),
                "{domain:?}: message ids differ"
            );
            for (id, vars) in &en {
                assert_eq!(Some(vars), tr.get(id), "{domain:?}/{id}: variables differ");
            }
            for locale in [Locale::En, Locale::Tr] {
                FluentResource::try_new(domain.source(locale).to_owned())
                    .unwrap_or_else(|(_, e)| panic!("{domain:?}/{locale:?}: {e:?}"));
            }
        }
    }

    /// Message id → variable names, from the source text.
    fn messages(src: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut current = None;
        for line in src.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if !line.starts_with(' ')
                && let Some((id, _)) = line.split_once('=')
            {
                let id = id.trim().to_owned();
                out.entry(id.clone()).or_default();
                current = Some(id);
            }
            if let Some(id) = &current {
                let vars = out.entry(id.clone()).or_default();
                let mut rest = line;
                while let Some(i) = rest.find('$') {
                    rest = &rest[i + 1..];
                    let end = rest
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                        .unwrap_or(rest.len());
                    vars.insert(rest[..end].to_owned());
                }
            }
        }
        out
    }
}
