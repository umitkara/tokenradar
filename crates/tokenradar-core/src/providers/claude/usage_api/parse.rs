//! Tolerant parser for `GET /api/oauth/usage` (data-sources.md S2).
//!
//! The endpoint is undocumented and grows code-named fields, so the body is read as a
//! `serde_json::Value` and only known fields are picked out. A malformed entry is skipped,
//! never the whole response. The response holds no conversation content.
//!
//! Schema priority: the `limits[]` array first. Only if it is missing or yields nothing usable
//! do the flat fields (`five_hour`, `seven_day`, `seven_day_<model>`) count.

use serde_json::{Map, Value};

use crate::domain::{AccountId, LimitKind, LimitObservation, Scope, Source, Timestamp};

/// Everything TokenRadar uses from one response.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageReport {
    pub observations: Vec<LimitObservation>,
    pub schema: UsageSchema,
    pub breakdown: Option<SurfaceBreakdown>,
    pub extra: ExtraUsage,
}

/// Which part of the response the observations came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSchema {
    LimitsArray,
    FlatFields,
    /// Neither held a usable limit.
    Empty,
}

/// `seven_day_breakdown`: each surface's share of the weekly window.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceBreakdown {
    pub as_of: Timestamp,
    pub window_started_at: Option<Timestamp>,
    /// `(key, percent)`, e.g. `("claude_code", Some(99.0))`.
    pub rows: Vec<(String, Option<f64>)>,
}

/// Whether paid usage beyond the plan is switched on. The UI shows only that.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExtraUsage {
    /// `extra_usage.is_enabled`
    pub extra_usage: Option<bool>,
    /// `spend.enabled`
    pub spend: Option<bool>,
}

impl ExtraUsage {
    /// `None` when the response said nothing about it.
    pub fn enabled(self) -> Option<bool> {
        match (self.extra_usage, self.spend) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(false) || b.unwrap_or(false)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UsageParseError {
    /// Only the category: the body itself is never echoed.
    #[error("the usage response is not a JSON object")]
    NotAnObject,
}

/// `seven_day_<suffix>` flat fields that are per-model weekly limits, and the display name the
/// `limits[]` schema uses for that model. Other suffixes (`oauth_apps`, `cowork`, code names)
/// are skipped.
const FLAT_MODEL_FIELDS: &[(&str, &str)] = &[
    ("seven_day_opus", "Opus"),
    ("seven_day_sonnet", "Sonnet"),
    ("seven_day_haiku", "Haiku"),
    ("seven_day_fable", "Fable"),
    ("seven_day_mythos", "Mythos"),
];

pub fn parse_usage(
    body: &[u8],
    account: AccountId,
    observed_at: Timestamp,
) -> Result<UsageReport, UsageParseError> {
    let root: Map<String, Value> = match serde_json::from_slice(body) {
        Ok(Value::Object(map)) => map,
        _ => return Err(UsageParseError::NotAnObject),
    };
    let obs = |kind, scope, percent, resets_at, severity| LimitObservation {
        account,
        kind,
        scope,
        percent,
        resets_at,
        severity,
        source: Source::Api,
        session_uuid: None,
        observed_at,
    };

    let mut observations: Vec<LimitObservation> = root
        .get("limits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let item = item.as_object()?;
                    let kind = LimitKind::from(item.get("kind")?.as_str()?);
                    let percent = item.get("percent").and_then(Value::as_f64)?;
                    Some(obs(
                        kind,
                        item.get("scope").and_then(scope),
                        percent,
                        item.get("resets_at").and_then(instant),
                        item.get("severity")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    let schema = if observations.is_empty() {
        let flat = [
            ("five_hour", LimitKind::Session, None),
            ("seven_day", LimitKind::WeeklyAll, None),
        ]
        .into_iter()
        .chain(FLAT_MODEL_FIELDS.iter().map(|&(field, model)| {
            (
                field,
                LimitKind::WeeklyScoped,
                Some(Scope::Model(model.to_owned())),
            )
        }));
        for (field, kind, scope) in flat {
            let Some(window) = root.get(field).and_then(Value::as_object) else {
                continue; // null or absent
            };
            if let Some(percent) = window.get("utilization").and_then(Value::as_f64) {
                observations.push(obs(
                    kind,
                    scope,
                    percent,
                    window.get("resets_at").and_then(instant),
                    None,
                ));
            }
        }
        if observations.is_empty() {
            UsageSchema::Empty
        } else {
            UsageSchema::FlatFields
        }
    } else {
        UsageSchema::LimitsArray
    };

    Ok(UsageReport {
        observations,
        schema,
        breakdown: root.get("seven_day_breakdown").and_then(breakdown),
        extra: ExtraUsage {
            extra_usage: root
                .get("extra_usage")
                .and_then(|v| v.get("is_enabled"))
                .and_then(Value::as_bool),
            spend: root
                .get("spend")
                .and_then(|v| v.get("enabled"))
                .and_then(Value::as_bool),
        },
    })
}

/// ISO 8601 string, or epoch seconds should a version switch to the statusline's format.
fn instant(v: &Value) -> Option<Timestamp> {
    match v {
        Value::String(s) => Timestamp::parse_iso(s).ok(),
        Value::Number(n) => n
            .as_f64()
            .filter(|s| s.is_finite())
            // Saturating `as` is the intent for absurd epoch values.
            .map(|s| {
                #[allow(clippy::cast_possible_truncation)]
                Timestamp::from_millis((s * 1000.0).round() as i64)
            }),
        _ => None,
    }
}

/// `{"model": {"id": null, "display_name": "Fable"}, "surface": null}`
fn scope(v: &Value) -> Option<Scope> {
    let name = |v: &Value| -> Option<String> {
        match v {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            Value::Object(o) => ["display_name", "id", "key"]
                .iter()
                .find_map(|k| o.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty()))
                .map(str::to_owned),
            _ => None,
        }
    };
    v.get("model")
        .and_then(name)
        .map(Scope::Model)
        .or_else(|| v.get("surface").and_then(name).map(Scope::Surface))
}

fn breakdown(v: &Value) -> Option<SurfaceBreakdown> {
    let rows = v
        .get("rows")?
        .as_array()?
        .iter()
        .filter_map(|row| {
            let key = row.get("key")?.as_str()?.to_owned();
            Some((key, row.get("percent").and_then(Value::as_f64)))
        })
        .collect::<Vec<_>>();
    Some(SurfaceBreakdown {
        as_of: v.get("as_of").and_then(instant)?,
        window_started_at: v.get("window_started_at").and_then(instant),
        rows,
    })
}
