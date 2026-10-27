//! API-equivalent cost weights used for weighted tokens and calibration (architecture §9.1).
//!
//! Prices are per model; the family only picks a stand-in for models the table does not know
//! yet (ADR-025).

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex, PoisonError};

use serde::Deserialize;

use crate::domain::{MicroUsd, ModelFamily, Speed, Usage};

const CLAUDE_TOML: &str = include_str!("claude.toml");

static CLAUDE: LazyLock<Pricing> =
    LazyLock::new(|| Pricing::from_toml(CLAUDE_TOML).expect("embedded claude.toml is valid"));

#[derive(Debug, thiserror::Error)]
pub enum PricingError {
    #[error("invalid pricing table: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported pricing schema {0}")]
    Schema(u32),
    #[error("fallback model {0:?} is not in the table")]
    MissingFallback(String),
    #[error("unknown model family {0:?}")]
    Family(String),
}

/// Prices of one model in `$/MTok` (= micro-dollars per token).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPrice {
    pub id: String,
    pub family: ModelFamily,
    pub input: f64,
    pub output: f64,
    pub cache_write_5m_mult: f64,
    pub cache_write_1h_mult: f64,
    pub cache_read_mult: f64,
    pub fast_mult: f64,
    pub long_context: Option<LongContext>,
}

/// Higher base prices for prompts above a size threshold.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct LongContext {
    /// Prompts with more tokens than this pay the long-context prices.
    pub threshold: u64,
    pub input: f64,
    pub output: f64,
}

/// Result of pricing one response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priced {
    pub cost: MicroUsd,
    /// False when the model was not in the table and a stand-in price was used.
    pub model_known: bool,
}

/// An embedded, versioned price table.
#[derive(Debug)]
pub struct Pricing {
    version: String,
    models: Vec<ModelPrice>,
    fallback: usize,
    web_search_micro_usd: f64,
    warned: Mutex<HashSet<String>>,
}

impl Pricing {
    /// The embedded Claude table.
    pub fn claude() -> &'static Self {
        &CLAUDE
    }

    pub fn from_toml(text: &str) -> Result<Self, PricingError> {
        let file: PricingFile = toml::from_str(text)?;
        if file.schema != 1 {
            return Err(PricingError::Schema(file.schema));
        }
        let d = &file.defaults;
        let models = file
            .model
            .into_iter()
            .map(|m| {
                Ok(ModelPrice {
                    family: m
                        .family
                        .parse()
                        .map_err(|_| PricingError::Family(m.family))?,
                    id: m.id,
                    input: m.input,
                    output: m.output,
                    cache_write_5m_mult: m.cache_write_5m_mult.unwrap_or(d.cache_write_5m_mult),
                    cache_write_1h_mult: m.cache_write_1h_mult.unwrap_or(d.cache_write_1h_mult),
                    cache_read_mult: m.cache_read_mult.unwrap_or(d.cache_read_mult),
                    fast_mult: m.fast_mult.unwrap_or(d.fast_mult),
                    long_context: m.long_context,
                })
            })
            .collect::<Result<Vec<_>, PricingError>>()?;
        let fallback = models
            .iter()
            .position(|m| m.id == file.fallback)
            .ok_or(PricingError::MissingFallback(file.fallback))?;
        Ok(Self {
            version: file.version,
            models,
            fallback,
            web_search_micro_usd: d.web_search_micro_usd,
            warned: Mutex::new(HashSet::new()),
        })
    }

    /// Table version, stored alongside derived values such as calibrations.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Exact entry for a model, if the table knows it.
    pub fn find(&self, model: &str) -> Option<&ModelPrice> {
        let name = strip_platform_prefix(model);
        self.models
            .iter()
            .filter(|m| matches_id(name, &m.id))
            .max_by_key(|m| m.id.len())
    }

    /// Price for a model: the exact entry, else the newest entry of its family, else the
    /// fallback. The second value is false for stand-ins.
    pub fn lookup(&self, model: &str) -> (&ModelPrice, bool) {
        if let Some(price) = self.find(model) {
            return (price, true);
        }
        let family = family_of(model);
        let stand_in = self
            .models
            .iter()
            .find(|m| m.family == family)
            .unwrap_or(&self.models[self.fallback]);
        self.warn_once(model, &stand_in.id);
        (stand_in, false)
    }

    /// API-equivalent cost of one response:
    ///
    /// ```text
    /// C = speed × [ p_in × (input + w5m·cache_w5m + w1h·cache_w1h + r·cache_read)
    ///             + p_out × output ] + web_search × price_per_search
    /// ```
    pub fn cost(&self, model: &str, usage: &Usage) -> Priced {
        let (price, model_known) = self.lookup(model);
        let t = &usage.tokens;
        let (p_in, p_out) = match price.long_context {
            Some(lc) if t.prompt_tokens() > lc.threshold => (lc.input, lc.output),
            _ => (price.input, price.output),
        };
        let speed = match usage.speed {
            Speed::Standard => 1.0,
            Speed::Fast => price.fast_mult,
        };
        let weighted_input = f(t.input)
            + price.cache_write_5m_mult * f(t.cache_w5m)
            + price.cache_write_1h_mult * f(t.cache_w1h)
            + price.cache_read_mult * f(t.cache_read);
        let tokens = speed * (p_in * weighted_input + p_out * f(t.output));
        let web = self.web_search_micro_usd * f64::from(usage.web_search);
        Priced {
            cost: MicroUsd::from_f64_rounded(tokens + web),
            model_known,
        }
    }

    fn warn_once(&self, model: &str, stand_in: &str) {
        let mut warned = self.warned.lock().unwrap_or_else(PoisonError::into_inner);
        if warned.insert(model.to_owned()) {
            tracing::warn!(
                model,
                stand_in,
                "model missing from the pricing table; using a stand-in price"
            );
        }
    }
}

/// Token counts are far below 2^52, where `f64` stops being exact.
#[allow(clippy::cast_precision_loss)]
fn f(tokens: u64) -> f64 {
    tokens as f64
}

/// Family of a model name by pattern, for names the table may not know yet.
pub fn family_of(model: &str) -> ModelFamily {
    if let Some(price) = Pricing::claude().find(model) {
        return price.family;
    }
    let lower = model.to_ascii_lowercase();
    [
        ("fable", ModelFamily::Fable),
        ("mythos", ModelFamily::Mythos),
        ("opus", ModelFamily::Opus),
        ("sonnet", ModelFamily::Sonnet),
        ("haiku", ModelFamily::Haiku),
    ]
    .into_iter()
    .find_map(|(needle, family)| lower.contains(needle).then_some(family))
    .unwrap_or(ModelFamily::Unknown)
}

/// `us.anthropic.claude-opus-4-8-v1:0` → `claude-opus-4-8-v1:0`.
fn strip_platform_prefix(model: &str) -> &str {
    match model.find("claude-") {
        Some(i) if i > 0 && model[..i].ends_with('.') => &model[i..],
        _ => model,
    }
}

/// `name` is `id` itself or `id` with a date, version, `@` or `[` suffix. A plain prefix is
/// not enough: `claude-opus-5` must not match `claude-opus-5-5`.
fn matches_id(name: &str, id: &str) -> bool {
    let Some(rest) = name.strip_prefix(id) else {
        return false;
    };
    if rest.is_empty() || rest.starts_with(['@', '[', ':']) {
        return true;
    }
    let Some(rest) = rest.strip_prefix('-') else {
        return false;
    };
    let date = rest.len() >= 8
        && rest.as_bytes()[..8].iter().all(u8::is_ascii_digit)
        && (rest.len() == 8 || !rest.as_bytes()[8].is_ascii_digit());
    let version = rest
        .strip_prefix('v')
        .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()));
    date || version
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PricingFile {
    schema: u32,
    version: String,
    fallback: String,
    defaults: Defaults,
    model: Vec<ModelEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Defaults {
    cache_write_5m_mult: f64,
    cache_write_1h_mult: f64,
    cache_read_mult: f64,
    fast_mult: f64,
    web_search_micro_usd: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelEntry {
    id: String,
    family: String,
    input: f64,
    output: f64,
    cache_write_5m_mult: Option<f64>,
    cache_write_1h_mult: Option<f64>,
    cache_read_mult: Option<f64>,
    fast_mult: Option<f64>,
    long_context: Option<LongContext>,
}

#[cfg(test)]
mod tests;
