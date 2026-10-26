//! Localization for the Rust side of TokenRadar (tray tooltip, notifications, CLI).
//!
//! Translations live in `locales/{en,tr}/*.ftl` and are shared with the frontend.
//! Fluent loading and the custom functions (`NUM`, `PCT`, `DUR`, `TR_SUFFIX`) arrive in Step 08.

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
}

/// Formats localized messages for one locale.
#[derive(Debug, Clone)]
pub struct Localizer {
    locale: Locale,
}

impl Localizer {
    pub const fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub const fn locale(&self) -> Locale {
        self.locale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_tags_match_locale_directories() {
        assert_eq!(Locale::En.tag(), "en");
        assert_eq!(Locale::Tr.tag(), "tr");
    }

    #[test]
    fn localizer_keeps_its_locale() {
        assert_eq!(Localizer::new(Locale::Tr).locale(), Locale::Tr);
    }
}
