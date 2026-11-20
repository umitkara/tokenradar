//! The history pages' queries (Step 14b): each call builds a [`QueryContext`] from the running
//! core (primary account, latest calibrations, the computer's zone) and runs off the async
//! workers. The view models come from `tokenradar_core::queries`; nothing is computed here.

use std::path::Path;

use tauri::AppHandle;
use tokenradar_core::calibration::Calibration;
use tokenradar_core::config::{AppConfig, DataConfig};
use tokenradar_core::domain::{AccountId, LimitRef, Timestamp};
use tokenradar_core::limits::LimitView;
use tokenradar_core::paths::Paths;
use tokenradar_core::queries::{self, PlanInput, ProjectFilter, QueryContext, SystemZone};
use tokenradar_core::store::{Store, StoreError};

use crate::core_bridge;

/// What one query needs from the running core.
pub struct Query {
    pub store: Store,
    pub account: AccountId,
    pub calibrations: Vec<Calibration>,
    /// Uuids of the running sessions.
    pub live: Vec<String>,
    pub limits: Vec<LimitView>,
    /// Limits the account does not have (ADR-041).
    pub absent: Vec<LimitRef>,
    subscription: Option<String>,
    tier: Option<String>,
    user_fee_usd: Option<f64>,
}

impl Query {
    pub fn plan(&self) -> PlanInput<'_> {
        PlanInput {
            subscription: self.subscription.as_deref(),
            tier: self.tier.as_deref(),
            user_fee_usd: self.user_fee_usd,
        }
    }
}

/// Error text when the core is not running or the account is unknown yet.
const NOT_READY: &str = "core not running";

/// Runs `f` with a context for now; an error string when the core is not ready or the query
/// failed (logged).
pub fn run<T>(
    app: &AppHandle,
    f: impl FnOnce(&Query, &QueryContext<'_>) -> Result<T, StoreError>,
) -> Result<T, String> {
    let q = prepare(app).ok_or_else(|| NOT_READY.to_owned())?;
    let zone = SystemZone;
    let ctx = QueryContext {
        account: q.account,
        now: Timestamp::now(),
        calibrations: &q.calibrations,
        zone: &zone,
    };
    f(&q, &ctx).map_err(|error| {
        tracing::warn!(%error, "history query failed");
        error.to_string()
    })
}

/// The config's temporary-folder patterns as `LIKE` patterns, when `filter` hides them (Step
/// 16b). Read on each call, so a change in the settings applies to the next list.
pub fn temp_patterns(filter: &ProjectFilter) -> Vec<String> {
    if !filter.hide_temp {
        return Vec::new();
    }
    let dirs = Paths::resolve()
        .ok()
        .and_then(|paths| AppConfig::load(paths.config_path()).ok())
        .map_or_else(|| DataConfig::default().temp_dirs, |c| c.data.temp_dirs);
    queries::temp_like_patterns(&dirs, |name| std::env::var(name).ok())
}

fn prepare(app: &AppHandle) -> Option<Query> {
    let store = core_bridge::store(app)?;
    let snapshot = core_bridge::panel_snapshot(app)?;
    let account = snapshot.account?;
    let calibrations = store
        .latest_calibrations(account.id)
        .map_err(|error| tracing::warn!(%error, "history: calibrations not readable"))
        .unwrap_or_default();
    let config_dir = store
        .accounts()
        .ok()?
        .into_iter()
        .find(|a| a.id == account.id)
        .map(|a| a.spec.config_dir);
    let user_fee_usd = config_dir.and_then(|dir| {
        let paths = Paths::resolve().ok()?;
        AppConfig::load(paths.config_path())
            .ok()?
            .account(Path::new(&dir))?
            .plan_fee_usd
    });
    Some(Query {
        store,
        account: account.id,
        calibrations,
        live: snapshot
            .live_sessions
            .iter()
            .filter(|s| s.status.is_some())
            .filter_map(|s| s.session_uuid.clone())
            .collect(),
        limits: snapshot.limits,
        absent: snapshot.absent,
        subscription: account.plan,
        tier: account.tier,
        user_fee_usd,
    })
}
