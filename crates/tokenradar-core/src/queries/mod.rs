//! Read models of the main window (Step 14b): windows, sessions, projects, the time series,
//! the heat map and the overview.
//!
//! - **Precomputed.** Attribution is stored per window (Step 14a, ADR-064); these queries read
//!   it and add up `usage_5m`. Only [`get_window`] and [`get_overview`] may compute one
//!   attribution, through [`crate::attribution::ensure`].
//! - **Keyset pagination.** Lists take an opaque cursor (the last row's sort key and id) and
//!   never use `OFFSET`, so a page stays stable while new rows arrive.
//! - **Honest numbers.** Every percentage carries a [`Basis`]: how it was computed, whether it
//!   is shown with `≈` and whether it is dimmed. Estimated windows report `hit_limit: None`.
//! - **Local time.** Day, week-day and hour boundaries come from a [`LocalZone`]; storage and
//!   cursors stay in UTC.

mod lttb;
mod overview;
mod plans;
mod projects;
mod series;
mod sessions;
mod temp;
mod tree;
mod windows;
mod zone;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::attribution::Method;
use crate::calibration::Calibration;
use crate::domain::{AccountId, LimitWindow, Timestamp};
use crate::limits::Confidence;
use crate::store::StoreError;

pub use lttb::lttb;
pub use overview::{
    ModelSlice, Overview, OverviewOther, PlanFeeSource, PlanInput, PlanValue, TokenBreakdown,
    TopProject, TopSession, WeekSpan, get_overview,
};
pub use plans::{PlanFee, plan_fee};
pub use projects::{
    ProjectDetail, ProjectFilter, ProjectNode, ProjectPage, ProjectRow, ProjectSort, ProjectTree,
    ProjectWeek, get_project, list_projects, project_tree,
};
pub use series::{
    HeatCell, HeatMetric, Heatmap, SeriesPoint, SessionMarker, TimeSeries, WindowBand, get_heatmap,
    get_timeseries,
};
pub use sessions::{
    CacheStats, CwdRow, ImpactRow, SessionDetail, SessionFilter, SessionPage, SessionRow,
    SessionSort, SubagentRow, TimelineBucket, get_session, get_session_impact, list_sessions,
    session_dirs,
};
pub use temp::temp_like_patterns;
pub use windows::{
    OtherView, ShareRow, WindowDetail, WindowFilter, WindowPage, WindowRow, get_window,
    list_windows,
};
pub use zone::{FixedZone, LocalZone, SystemZone, local_day_start, local_month_start};

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

/// Rows a list page returns when the caller asks for none or too many.
pub const DEFAULT_PAGE: u32 = 50;
pub const MAX_PAGE: u32 = 200;
/// Points a time series is thinned to at most.
pub const MAX_SERIES_POINTS: u32 = 1_500;

const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 7 * DAY_MS;

/// What every query needs besides its arguments.
#[derive(Clone, Copy)]
pub struct QueryContext<'a> {
    pub account: AccountId,
    pub now: Timestamp,
    /// The account's stored calibrations (their versions decide staleness).
    pub calibrations: &'a [Calibration],
    pub zone: &'a dyn LocalZone,
}

impl QueryContext<'_> {
    fn calibration_for(&self, w: &LimitWindow) -> Option<&Calibration> {
        self.calibrations
            .iter()
            .find(|c| c.kind == w.kind && c.scope == w.scope)
    }
}

/// `from <= t < to`, UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TimeRange {
    pub from: Timestamp,
    pub to: Timestamp,
}

impl TimeRange {
    pub const ALL: Self = Self {
        from: Timestamp::from_millis(0),
        to: Timestamp::from_millis(i64::MAX),
    };

    pub fn new(from: Timestamp, to: Timestamp) -> Self {
        Self { from, to }
    }

    /// The `days` before `now`.
    pub fn last_days(now: Timestamp, days: i64) -> Self {
        Self::new(now.saturating_sub_millis(days * DAY_MS), now)
    }
}

/// How a percentage was obtained and how to show it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct Basis {
    pub method: Method,
    /// Calibration confidence for this window's usage; `None` without a calibration.
    pub confidence: Option<f64>,
    /// Show with `≈`: not split with a usable calibration.
    pub approx: bool,
    /// Show dimmed: low confidence, or no measurement behind it.
    pub dim: bool,
    /// Computed with an older calibration; a background pass will redo it.
    pub stale: bool,
}

impl Basis {
    fn new(method: Method, confidence: Option<f64>, stale: bool) -> Self {
        let low =
            confidence.is_none_or(|c| crate::calibration::estimator::level(c) == Confidence::Low);
        Self {
            method,
            confidence,
            approx: method != Method::Calibrated,
            dim: match method {
                Method::Calibrated => low,
                Method::Proportional | Method::Estimated | Method::CostOnly => true,
            },
            stale,
        }
    }

    /// The weakest of several (a session's impact across windows).
    fn weakest(items: impl IntoIterator<Item = Self>) -> Option<Self> {
        items.into_iter().reduce(|a, b| {
            let rank = |m: Method| match m {
                Method::Calibrated => 0,
                Method::Proportional => 1,
                Method::Estimated => 2,
                Method::CostOnly => 3,
            };
            let method = if rank(b.method) > rank(a.method) {
                b.method
            } else {
                a.method
            };
            let confidence = match (a.confidence, b.confidence) {
                (Some(x), Some(y)) => Some(x.min(y)),
                _ => None,
            };
            Self {
                method,
                confidence,
                approx: a.approx || b.approx,
                dim: a.dim || b.dim,
                stale: a.stale || b.stale,
            }
        })
    }
}

/// Input, output, cache-read and cache-write tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
pub struct TokenSums {
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
}

impl TokenSums {
    pub fn total(&self) -> i64 {
        self.input + self.output + self.cache_read + self.cache_write
    }
}

/// A session's display name: the title, otherwise project and short uuid. Never content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct SessionName {
    pub text: String,
    /// No title was known; `text` is `project · uuid`.
    pub fallback: bool,
}

pub(crate) fn session_name(title: Option<&str>, project: Option<&str>, uuid: &str) -> SessionName {
    if let Some(t) = title.map(str::trim).filter(|t| !t.is_empty()) {
        return SessionName {
            text: t.to_owned(),
            fallback: false,
        };
    }
    let short: String = uuid.chars().take(8).collect();
    SessionName {
        text: match project {
            Some(p) => format!("{p} · {short}"),
            None => short,
        },
        fallback: true,
    }
}

/// A project as rows refer to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ProjectRef {
    pub id: crate::domain::ProjectId,
    pub name: String,
}

/// The cursor of a keyset page: the last row's sort key and id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cursor {
    pub key: f64,
    pub id: i64,
}

impl Cursor {
    pub(crate) fn encode(self) -> String {
        format!("{}:{}", self.key, self.id)
    }

    /// `None` for a missing or unreadable cursor: the first page.
    pub(crate) fn decode(s: Option<&str>) -> Option<Self> {
        let (key, id) = s?.rsplit_once(':')?;
        Some(Self {
            key: key.parse().ok()?,
            id: id.parse().ok()?,
        })
    }

    /// A cursor before every row of a descending list.
    pub(crate) const START: Self = Self {
        key: f64::INFINITY,
        id: i64::MAX,
    };
}

pub(crate) fn page_size(limit: Option<u32>) -> u32 {
    limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE)
}

/// `%text%` for `LIKE … ESCAPE '\'`.
pub(crate) fn like_pattern(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('%');
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

/// SQL clamps for open ranges: SQLite integers are 64-bit, `i64::MAX` is fine.
/// Ranges wider than this read `usage_5m` in primary-key order instead of through
/// `ix_5m_acct_bucket`: the index visits rows in time order and looks each one up in the key,
/// which beyond about 90 days of a busy year costs more than reading the whole table
/// (synthetic year: 62 ms against 16 ms, `docs/perf/2026-11-20-queries.md`).
pub(crate) const INDEX_SPAN_MS: i64 = 60 * DAY_MS;

/// The bucket column of `usage_5m u` for a `[from, to)` filter. The unary `+` keeps SQLite off
/// the time index for wide ranges; queries of one session tree always use [`TREE_BUCKET`].
pub(crate) fn bucket_col(from: i64, to: i64) -> &'static str {
    if to.saturating_sub(from) > INDEX_SPAN_MS {
        TREE_BUCKET
    } else {
        "u.bucket_ts"
    }
}

/// Ids of a project's sessions with their subagents (a subagent has no project of its own),
/// the project id being parameter `?{param}`. Both halves use an index.
pub(crate) fn project_sessions(param: usize) -> String {
    format!(
        "SELECT id FROM sessions WHERE project_id = ?{param}
         UNION ALL SELECT id FROM sessions
         WHERE root_id IN (SELECT id FROM sessions WHERE project_id = ?{param})"
    )
}

/// For queries that name sessions: the `(account_id, session_id)` key prefix is the way in.
pub(crate) const TREE_BUCKET: &str = "+u.bucket_ts";

pub(crate) fn bounds(range: Option<TimeRange>) -> (i64, i64) {
    let r = range.unwrap_or(TimeRange::ALL);
    (r.from.as_millis(), r.to.as_millis())
}
