//! The limit engine: fuses observations, records snapshots and windows, detects resets and
//! builds the panel's view model (architecture §7–§8).
//!
//! - **Fusion:** per `(account, kind, scope)` the newest observation leads: its source,
//!   window and time are shown. Within 30 s of each other the better source leads:
//!   statusline > API > estimate. The percentage combines every real reading of the window:
//!   the statusline rounds down and the API to the nearest point, so the highest lower bound
//!   wins (see [`super::fusion`], ADR-060).
//! - **Persistence:** every non-estimate observation goes to `limit_snapshots`, except that an
//!   unchanged reading (same percentage and window) from the same source is written at most
//!   every 5 minutes. That heartbeat keeps the time series alive without repeating each
//!   statusline call (ADR-028).
//! - **Windows:** a new `resets_at` opens a `limit_windows` row; `peak_percent` and
//!   `hit_limit` follow.
//! - **Reset:** once `resets_at` passes without a newer reading, the limit is assumed at 0 %
//!   (`Estimate`, reason `Reset`) and [`LimitEvent::Reset`] is raised.
//!
//! The engine is plain state driven by the caller's clock; the thread around it lives in
//! [`crate::runtime`].

use std::cell::Cell;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};

use super::fusion::Bound;
use super::presence::{self, Presence};
use super::rate::{self, Projection};
use super::share::{self, RootUsage};
use super::view::{
    AccountView, ApiStatusView, EstimateReason, FRESH_MS, Freshness, IndexState, LimitEvent,
    LimitView, PanelSnapshot, STALE_MS, SessionShareView, Severity, StatuslineState, THRESHOLDS,
};
use crate::domain::{
    AccountId, LimitKind, LimitObservation, LimitRef, MicroUsd, Scope, SessionId, Source, Timestamp,
};
use crate::live_sessions::{LiveSession, LiveStatus};
use crate::providers::claude::usage_api::{
    Activity, ApiState, PollDiagnostics, PollInputs, PollUpdate, UsageReport, UsageSchema,
};
use crate::store::{Store, StoreError, WINDOW_MATCH_TOLERANCE_MS};

const MIN: i64 = 60 * 1000;
pub const SESSION_WINDOW_MS: i64 = 5 * 60 * MIN;
pub const WEEK_MS: i64 = 7 * 24 * 60 * MIN;
/// Observations closer than this compete by source priority.
pub const FUSION_WINDOW_MS: i64 = 30 * 1000;
/// An unchanged reading is written again after this long.
pub const HEARTBEAT_MS: i64 = 5 * MIN;
/// Percentages closer than this are "unchanged".
pub(super) const PERCENT_EPSILON: f64 = 0.005;
/// Token rate window for the session list.
const TOKEN_RATE_MS: i64 = 5 * MIN;
const TOP_SESSIONS: usize = 3;

/// Length of a limit's window, if it has windows.
pub fn window_length_ms(kind: &LimitKind) -> Option<i64> {
    match kind {
        LimitKind::Session => Some(SESSION_WINDOW_MS),
        LimitKind::WeeklyAll | LimitKind::WeeklyScoped => Some(WEEK_MS),
        LimitKind::Spend | LimitKind::Other(_) => None,
    }
}

fn priority(source: Source) -> u8 {
    match source {
        Source::Statusline | Source::CodexLog => 3,
        Source::Api => 2,
        Source::Estimate => 1,
    }
}

pub(super) fn same_window(a: Option<Timestamp>, b: Option<Timestamp>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => (a.as_millis() - b.as_millis()).abs() <= WINDOW_MATCH_TOLERANCE_MS,
        (None, None) => true,
        _ => false,
    }
}

/// Whether `new` replaces `cur` as the current value.
fn supersedes(new: &LimitObservation, cur: &LimitObservation) -> bool {
    let dt = new.observed_at.as_millis() - cur.observed_at.as_millis();
    if dt.abs() <= FUSION_WINDOW_MS {
        let (pn, pc) = (priority(new.source), priority(cur.source));
        if pn == pc { dt >= 0 } else { pn > pc }
    } else {
        dt > 0
    }
}

/// Map key: `(kind, scope)` in a stable display order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TrackKey(u8, String, Option<String>);

impl TrackKey {
    fn of(kind: &LimitKind, scope: Option<&Scope>) -> Self {
        let order = match kind {
            LimitKind::Session => 0,
            LimitKind::WeeklyAll => 1,
            LimitKind::WeeklyScoped => 2,
            LimitKind::Spend => 3,
            LimitKind::Other(_) => 4,
        };
        Self(
            order,
            kind.as_str().to_owned(),
            scope.map(ToString::to_string),
        )
    }
}

#[derive(Debug, Clone)]
struct Written {
    percent: f64,
    resets_at: Option<Timestamp>,
    at: Timestamp,
}

#[derive(Debug, Clone)]
struct Track {
    /// The leading reading, with the fused percentage of its window.
    fused: LimitObservation,
    /// The leading reading's own percentage.
    lead_percent: f64,
    /// The window's running lower bound; `None` for an estimate.
    bound: Option<Bound>,
    /// Time of the newest real reading folded into `bound`.
    last_at: Timestamp,
    reason: Option<EstimateReason>,
    /// Fused values of the current window from the last 30 minutes, oldest first.
    history: Vec<(Timestamp, f64)>,
    written: HashMap<Source, Written>,
    will_hit: bool,
}

impl Track {
    fn new(obs: LimitObservation) -> Self {
        let bound = Bound::start(obs.source, obs.percent, obs.observed_at);
        Self {
            lead_percent: obs.percent,
            last_at: obs.observed_at,
            fused: LimitObservation {
                percent: bound.map_or(obs.percent, |b| b.value()),
                ..obs
            },
            bound,
            reason: None,
            history: Vec::new(),
            written: HashMap::new(),
            will_hit: false,
        }
    }

    /// Takes a real reading of the running window, newer or older: it moves the bound and
    /// may lead. Readings of another window, and estimates, change nothing here; the caller
    /// decides whether they replace the track.
    fn fold(&mut self, obs: &LimitObservation) {
        let Some(bound) = self.bound.as_mut() else {
            return;
        };
        if obs.source == Source::Estimate || !same_window(obs.resets_at, self.fused.resets_at) {
            return;
        }
        let newest = obs.observed_at >= self.last_at;
        bound.fold(obs.source, obs.percent, obs.observed_at, newest);
        let value = bound.value();
        if supersedes(obs, &self.fused) {
            self.fused = obs.clone();
            self.lead_percent = obs.percent;
        }
        self.fused.percent = value;
        if newest {
            self.last_at = obs.observed_at;
        }
    }

    /// Another source that lifted the value above what the leading reading says alone.
    fn combined_with(&self) -> Option<Source> {
        let bound = self.bound?;
        let alone =
            Bound::start(self.fused.source, self.lead_percent, self.fused.observed_at)?.value();
        (bound.source() != self.fused.source && self.fused.percent > alone + PERCENT_EPSILON)
            .then_some(bound.source())
    }

    fn is_real(&self) -> bool {
        self.fused.source != Source::Estimate
    }

    /// Start of the current window: `resets_at − length`; after a reset, the reset itself.
    fn window_start(&self) -> Option<Timestamp> {
        if self.reason == Some(EstimateReason::Reset) {
            return Some(self.fused.observed_at);
        }
        let length = window_length_ms(&self.fused.kind)?;
        self.fused
            .resets_at
            .map(|r| r.saturating_sub_millis(length))
    }

    /// Adds the fused value at the time of `obs` to the slope's history. Only the newest
    /// reading counts: an older one already went into the bound, and the value it would
    /// stand for at its time is unknown.
    fn remember(&mut self, obs: &LimitObservation, now: Timestamp) {
        if obs.source == Source::Estimate
            || !same_window(obs.resets_at, self.fused.resets_at)
            || obs.observed_at < self.last_at
        {
            return;
        }
        let at = obs.observed_at;
        let pos = self.history.partition_point(|(t, _)| *t <= at);
        self.history.insert(pos, (at, self.fused.percent));
        let cutoff = now.as_millis() - rate::RATE_LOOKBACK_MS;
        self.history.retain(|(t, _)| t.as_millis() >= cutoff);
    }
}

/// Static facts about an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMeta {
    pub id: AccountId,
    pub label: String,
    pub plan: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug)]
struct AccountState {
    meta: AccountMeta,
    tracks: BTreeMap<TrackKey, Track>,
    api_state: ApiState,
    api_diag: PollDiagnostics,
    extra_usage: Option<bool>,
    statusline_last: Option<Timestamp>,
    /// Which limits the account has; absent ones have no track (ADR-041).
    presence: Presence,
}

pub struct LimitEngine {
    store: Store,
    /// The account the panel shows: the first one configured.
    primary: Option<AccountId>,
    accounts: BTreeMap<AccountId, AccountState>,
    live: Vec<LiveSession>,
    index: IndexState,
    /// Errors are counted, never fatal.
    store_errors: u64,
}

impl LimitEngine {
    /// Restores the last known values from the store. Resets that passed while the app was
    /// closed are applied silently: no event for something that happened long ago.
    pub fn new(
        store: Store,
        accounts: Vec<AccountMeta>,
        api_consent: bool,
        now: Timestamp,
    ) -> Result<Self, StoreError> {
        let primary = accounts.first().map(|a| a.id);
        let mut map = BTreeMap::new();
        for meta in accounts {
            let id = meta.id;
            let mut state = AccountState {
                meta,
                tracks: BTreeMap::new(),
                api_state: if api_consent {
                    ApiState::Active
                } else {
                    ApiState::Off
                },
                api_diag: PollDiagnostics::default(),
                extra_usage: None,
                statusline_last: None,
                presence: Presence::with_absent(
                    store
                        .kv_get(&presence::kv_key(id))?
                        .map(|v| presence::decode(&v))
                        .unwrap_or_default(),
                ),
            };
            // The fused value needs every reading of the current window; the longest is a
            // week, so one read covers them all.
            let recent = store.snapshots_between(
                id,
                now.saturating_sub_millis(WEEK_MS),
                now.saturating_add_millis(1),
            )?;
            for obs in store.latest_snapshots(id)? {
                if obs.source == Source::Statusline {
                    state.statusline_last = state.statusline_last.max(Some(obs.observed_at));
                }
                if state.presence.is_absent(&LimitRef::of(&obs)) {
                    continue;
                }
                let key = TrackKey::of(&obs.kind, obs.scope.as_ref());
                let track = restore_track(&obs, &recent, &key, now);
                state.tracks.insert(key, track);
            }
            map.insert(id, state);
        }
        let mut engine = Self {
            store,
            primary,
            accounts: map,
            live: Vec::new(),
            index: IndexState::default(),
            store_errors: 0,
        };
        engine.apply_resets(now, false);
        Ok(engine)
    }

    pub fn store_errors(&self) -> u64 {
        self.store_errors
    }

    /// Takes one observation from any source. Returns threshold crossings.
    pub fn ingest(&mut self, obs: &LimitObservation, now: Timestamp) -> Vec<LimitEvent> {
        let mut events = Vec::new();
        let Some(state) = self.accounts.get_mut(&obs.account) else {
            tracing::debug!(account = %obs.account, "observation for an unknown account");
            return events;
        };
        if obs.source == Source::Statusline {
            state.statusline_last = state.statusline_last.max(Some(obs.observed_at));
        }
        if obs.source != Source::Estimate && state.presence.observed(&LimitRef::of(obs)) {
            tracing::info!(limit = %LimitRef::of(obs).encode(), "limit is present again");
            store_absent(
                &self.store,
                obs.account,
                &state.presence,
                &mut self.store_errors,
            );
        }
        let key = TrackKey::of(&obs.kind, obs.scope.as_ref());

        // Persist first: the time series wants every real reading, winner or not.
        let mut written = None;
        if obs.source != Source::Estimate {
            let last = state
                .tracks
                .get(&key)
                .and_then(|t| t.written.get(&obs.source));
            let due = last.is_none_or(|w| {
                (w.percent - obs.percent).abs() > PERCENT_EPSILON
                    || !same_window(w.resets_at, obs.resets_at)
                    || (obs.observed_at.as_millis() - w.at.as_millis()).abs() >= HEARTBEAT_MS
            });
            if due {
                match self
                    .store
                    .writer()
                    .record_snapshot(obs.clone(), window_length_ms(&obs.kind))
                {
                    Ok(_) => {
                        written = Some(Written {
                            percent: obs.percent,
                            resets_at: obs.resets_at,
                            at: obs.observed_at,
                        });
                    }
                    Err(error) => {
                        self.store_errors += 1;
                        tracing::warn!(%error, "could not store a limit snapshot");
                    }
                }
            }
        }

        let account = obs.account;
        let track = match state.tracks.entry(key) {
            Entry::Vacant(slot) => {
                let track = slot.insert(Track::new(obs.clone()));
                track.remember(obs, now);
                if let Some(w) = written {
                    track.written.insert(obs.source, w);
                }
                return events;
            }
            Entry::Occupied(slot) => slot.into_mut(),
        };
        if let Some(w) = written {
            track.written.insert(obs.source, w);
        }
        let prev = track.fused.clone();
        if track.bound.is_some()
            && obs.source != Source::Estimate
            && same_window(obs.resets_at, prev.resets_at)
        {
            // A reading of the running window moves the bound, leading or not.
            track.fold(obs);
        } else if supersedes(obs, &prev) {
            // A new window, the first reading after an estimate, or an estimate.
            let comparable = same_window(prev.resets_at, obs.resets_at);
            let mut next = Track::new(obs.clone());
            next.written = std::mem::take(&mut track.written);
            if comparable {
                next.history = std::mem::take(&mut track.history);
                next.will_hit = track.will_hit;
            }
            *track = next;
        } else {
            // An older reading of another window: nothing to show or learn.
            return events;
        }
        track.remember(obs, now);
        let percent = track.fused.percent;
        if same_window(prev.resets_at, track.fused.resets_at) {
            for th in THRESHOLDS {
                let th_f = f64::from(th);
                if prev.percent < th_f && percent >= th_f {
                    events.push(LimitEvent::ThresholdCrossed {
                        account,
                        kind: obs.kind.clone(),
                        scope: obs.scope.clone(),
                        threshold: th,
                        percent,
                        resets_at: track.fused.resets_at,
                    });
                }
            }
        }
        events
    }

    /// Applies a poller result: state, observations, breakdown, extra usage.
    pub fn apply_poll(&mut self, update: PollUpdate, now: Timestamp) -> Vec<LimitEvent> {
        let mut events = Vec::new();
        let Some(state) = self.accounts.get_mut(&update.account) else {
            return events;
        };
        state.api_state = update.state;
        state.api_diag = update.diagnostics;
        let Some(UsageReport {
            observations,
            schema,
            breakdown,
            extra,
        }) = update.report
        else {
            return events;
        };
        if let Some(enabled) = extra.enabled() {
            state.extra_usage = Some(enabled);
        }
        if let Some(b) = breakdown
            && let Err(error) = self.store.writer().insert_breakdown(
                update.account,
                b.as_of,
                b.window_started_at,
                b.rows,
            )
        {
            self.store_errors += 1;
            tracing::warn!(%error, "could not store the surface breakdown");
        }
        for obs in &observations {
            events.extend(self.ingest(obs, now));
        }
        // A response is a full listing: what it lacks, the account does not have. The flat
        // fields only ever describe the expected limits; an empty response proves nothing.
        let listed: Vec<LimitRef> = observations.iter().map(LimitRef::of).collect();
        let expected = presence::EXPECTED.map(|k| LimitRef::new(k, None));
        let candidates: Vec<LimitRef> = match schema {
            UsageSchema::Empty => Vec::new(),
            UsageSchema::FlatFields => expected.to_vec(),
            UsageSchema::LimitsArray => {
                self.accounts
                    .get(&update.account)
                    .map_or_else(Vec::new, |state| {
                        let mut all = expected.to_vec();
                        for t in state.tracks.values() {
                            let r = LimitRef::of(&t.fused);
                            if !all.contains(&r) {
                                all.push(r);
                            }
                        }
                        all
                    })
            }
        };
        if let Some(state) = self.accounts.get_mut(&update.account) {
            let became = state.presence.listing(&listed, candidates);
            self.mark_absent(update.account, &became);
        }
        events
    }

    /// One statusline payload's readings, before they are ingested one by one: a payload that
    /// lacks a window counts toward that window's absence (see [`presence`]).
    pub fn note_statusline(&mut self, readings: &[LimitObservation]) {
        let Some(account) = readings.first().map(|o| o.account) else {
            return;
        };
        let Some(state) = self.accounts.get_mut(&account) else {
            return;
        };
        let covered = presence::EXPECTED.map(|k| LimitRef::new(k, None));
        let became = state.presence.statusline(readings, &covered);
        self.mark_absent(account, &became);
    }

    /// Drops the tracks of limits that became absent and stores the decision.
    fn mark_absent(&mut self, account: AccountId, became: &[LimitRef]) {
        if became.is_empty() {
            return;
        }
        let Some(state) = self.accounts.get_mut(&account) else {
            return;
        };
        for r in became {
            tracing::info!(limit = %r.encode(), "limit is absent on this account");
            state
                .tracks
                .remove(&TrackKey::of(&r.kind, r.scope.as_ref()));
        }
        store_absent(
            &self.store,
            account,
            &state.presence,
            &mut self.store_errors,
        );
    }

    pub fn set_live_sessions(&mut self, live: Vec<LiveSession>) {
        self.live = live;
    }

    pub fn set_index_state(&mut self, index: IndexState) {
        self.index = index;
    }

    pub fn set_plan(&mut self, account: AccountId, plan: Option<String>, tier: Option<String>) {
        if let Some(state) = self.accounts.get_mut(&account) {
            state.meta.plan = plan;
            state.meta.tier = tier;
        }
    }

    /// Detects passed resets. Call on every wake-up.
    pub fn tick(&mut self, now: Timestamp) -> Vec<LimitEvent> {
        self.apply_resets(now, true)
    }

    fn apply_resets(&mut self, now: Timestamp, emit: bool) -> Vec<LimitEvent> {
        let mut events = Vec::new();
        for (account, state) in &mut self.accounts {
            for track in state.tracks.values_mut() {
                let Some(reset_at) = track.fused.resets_at else {
                    continue;
                };
                if !track.is_real() || now < reset_at {
                    continue;
                }
                let previous = track.fused.percent;
                // A weekly window repeats on the same weekday and time; a 5-hour window only
                // starts with the next use, so its reset time is unknown.
                let next = match track.fused.kind {
                    LimitKind::WeeklyAll | LimitKind::WeeklyScoped => {
                        let mut next = reset_at.saturating_add_millis(WEEK_MS);
                        while next <= now {
                            next = next.saturating_add_millis(WEEK_MS);
                        }
                        Some(next)
                    }
                    _ => None,
                };
                track.fused = LimitObservation {
                    percent: 0.0,
                    resets_at: next,
                    severity: None,
                    source: Source::Estimate,
                    session_uuid: None,
                    observed_at: reset_at,
                    ..track.fused.clone()
                };
                track.reason = Some(EstimateReason::Reset);
                track.history.clear();
                track.will_hit = false;
                if emit {
                    events.push(LimitEvent::Reset {
                        account: *account,
                        kind: track.fused.kind.clone(),
                        scope: track.fused.scope.clone(),
                        previous_percent: previous,
                        reset_at,
                    });
                }
            }
        }
        events
    }

    /// The next instant something changes without new input: a reset or a freshness step.
    pub fn next_wake(&self, now: Timestamp) -> Option<Timestamp> {
        let mut wake: Option<Timestamp> = None;
        let mut consider = |t: Timestamp| {
            if t > now {
                wake = Some(wake.map_or(t, |w| w.min(t)));
            }
        };
        for state in self.accounts.values() {
            for track in state.tracks.values() {
                if track.is_real()
                    && let Some(r) = track.fused.resets_at
                {
                    consider(r);
                }
                consider(track.fused.observed_at.saturating_add_millis(FRESH_MS));
                consider(track.fused.observed_at.saturating_add_millis(STALE_MS));
            }
        }
        wake
    }

    /// What the account's poller should know.
    pub fn poll_inputs(&self, account: AccountId) -> PollInputs {
        let Some(state) = self.accounts.get(&account) else {
            return PollInputs::default();
        };
        let mine = self.live.iter().filter(|s| s.account == account);
        let activity = mine.fold(Activity::None, |acc, s| match (acc, s.status) {
            (Activity::Busy, _) | (_, LiveStatus::Busy) => Activity::Busy,
            _ => Activity::Idle,
        });
        let mut resets: Vec<Timestamp> = state
            .tracks
            .values()
            .filter(|t| t.is_real())
            .filter_map(|t| t.fused.resets_at)
            .collect();
        resets.sort_unstable();
        resets.dedup();
        PollInputs {
            statusline_at: state.statusline_last,
            activity,
            resets,
        }
    }

    /// Builds the panel snapshot of the primary account. Also returns projection changes.
    pub fn snapshot(&mut self, now: Timestamp) -> (PanelSnapshot, Vec<LimitEvent>) {
        let mut events = Vec::new();
        let empty_api = ApiStatusView {
            state: ApiState::Off,
            diagnostics: PollDiagnostics::default(),
        };
        let Some(account) = self.primary else {
            return (
                PanelSnapshot {
                    account: None,
                    limits: Vec::new(),
                    absent: Vec::new(),
                    live_sessions: Vec::new(),
                    top_sessions: Vec::new(),
                    index_state: self.index.clone(),
                    api: empty_api,
                    statusline: StatuslineState::default(),
                    generated_at: now,
                },
                events,
            );
        };
        let queries = Queries {
            store: &self.store,
            account,
            errors: Cell::new(0),
        };
        let state = self
            .accounts
            .get_mut(&account)
            .expect("primary account has a state");
        let (limits, session_window) = limit_views(&queries, state, now, &mut events);
        // Sessions: shares of the current 5-hour window. Without a session limit the last
        // five hours stand in, for cost and tokens only.
        let (p_now, start) =
            session_window.unwrap_or((None, now.saturating_sub_millis(SESSION_WINDOW_MS)));
        let (live_rows, top_rows) = session_rows(&queries, &self.live, p_now, start, now);

        let snapshot = PanelSnapshot {
            account: Some(AccountView {
                id: account,
                label: state.meta.label.clone(),
                plan: state.meta.plan.clone(),
                tier: state.meta.tier.clone(),
                extra_usage_enabled: state.extra_usage,
            }),
            limits,
            absent: state.presence.absent(),
            live_sessions: live_rows,
            top_sessions: top_rows,
            index_state: self.index.clone(),
            api: ApiStatusView {
                state: state.api_state,
                diagnostics: state.api_diag.clone(),
            },
            statusline: StatuslineState {
                last_seen: state.statusline_last,
            },
            generated_at: now,
        };
        self.store_errors += queries.errors.get();
        (snapshot, events)
    }
}

/// A limit's track as the store left it: `newest` leads, every stored reading of its window
/// (oldest first in `recent`) goes into the bound and the slope, and the last write per
/// source restarts the heartbeat.
fn restore_track(
    newest: &LimitObservation,
    recent: &[LimitObservation],
    key: &TrackKey,
    now: Timestamp,
) -> Track {
    let mine: Vec<&LimitObservation> = recent
        .iter()
        .filter(|r| {
            TrackKey::of(&r.kind, r.scope.as_ref()) == *key
                && r.source != Source::Estimate
                && same_window(r.resets_at, newest.resets_at)
        })
        .collect();
    let mut track = Track::new((*mine.first().unwrap_or(&newest)).clone());
    for r in mine.iter().copied().chain(std::iter::once(newest)) {
        track.fold(r);
        track.remember(r, now);
        track.written.insert(
            r.source,
            Written {
                percent: r.percent,
                resets_at: r.resets_at,
                at: r.observed_at,
            },
        );
    }
    track
}

/// Keeps an account's absent limits in `kv` (see [`presence::kv_key`]).
fn store_absent(store: &Store, account: AccountId, presence: &Presence, errors: &mut u64) {
    let absent = presence.absent();
    let key = presence::kv_key(account);
    let result = if absent.is_empty() {
        store.writer().kv_delete(key)
    } else {
        store.writer().kv_set(key, presence::encode(&absent))
    };
    if let Err(error) = result {
        *errors += 1;
        tracing::warn!(%error, "could not store the absent limits");
    }
}

/// Store reads for one snapshot. A failed query counts as an error and reads as "no data".
struct Queries<'a> {
    store: &'a Store,
    account: AccountId,
    errors: Cell<u64>,
}

impl Queries<'_> {
    fn failed(&self, error: &StoreError) {
        self.errors.set(self.errors.get() + 1);
        tracing::warn!(%error, "limit engine query failed");
    }

    fn cost(&self, from: Timestamp, to: Timestamp) -> Option<i64> {
        self.store
            .cost_between(self.account, from, to)
            .map(|c| c.0)
            .map_err(|e| self.failed(&e))
            .ok()
    }

    fn roots(&self, from: Timestamp, to: Timestamp) -> Vec<RootUsage> {
        self.store
            .root_usage_between(self.account, from, to)
            .unwrap_or_else(|e| {
                self.failed(&e);
                Vec::new()
            })
    }
}

/// One view per tracked limit, plus the 5-hour window's `(measured percent, start)`.
fn limit_views(
    q: &Queries<'_>,
    state: &mut AccountState,
    now: Timestamp,
    events: &mut Vec<LimitEvent>,
) -> (Vec<LimitView>, Option<(Option<f64>, Timestamp)>) {
    let mut limits = Vec::with_capacity(state.tracks.len());
    let mut session_window = None;
    for track in state.tracks.values_mut() {
        let obs = &track.fused;
        let projection: Option<Projection> = if track.is_real() {
            // Cost scaling only makes sense for limits all local usage counts toward.
            let costs = match (&obs.kind, &obs.scope, track.window_start()) {
                (LimitKind::Session | LimitKind::WeeklyAll, None, Some(start)) => q
                    .cost(start, now)
                    .zip(q.cost(now.saturating_sub_millis(rate::FALLBACK_LOOKBACK_MS), now)),
                _ => None,
            };
            rate::projection(now, obs.percent, obs.resets_at, &track.history, costs)
        } else {
            None
        };
        let will_hit = projection.is_some_and(|p| p.will_hit_before_reset);
        if will_hit != track.will_hit {
            track.will_hit = will_hit;
            events.push(LimitEvent::ProjectionChanged {
                account: q.account,
                kind: obs.kind.clone(),
                scope: obs.scope.clone(),
                will_hit_before_reset: will_hit,
                eta_100_at: projection.and_then(|p| p.eta_100_at),
                resets_at: obs.resets_at,
            });
        }
        if obs.kind == LimitKind::Session && obs.scope.is_none() {
            let start = track
                .window_start()
                .unwrap_or_else(|| now.saturating_sub_millis(SESSION_WINDOW_MS));
            session_window = Some((track.is_real().then_some(obs.percent), start));
        }
        limits.push(LimitView {
            kind: obs.kind.clone(),
            scope: obs.scope.clone(),
            percent: obs.percent,
            resets_at: obs.resets_at,
            window_start: track.window_start(),
            severity: Severity::of(obs.percent),
            provider_severity: obs.severity.clone(),
            source: obs.source,
            combined_with: track.combined_with(),
            estimate_reason: track.reason,
            observed_at: obs.observed_at,
            freshness: Freshness::of(now.as_millis() - obs.observed_at.as_millis()),
            projection,
        });
    }
    (limits, session_window)
}

/// Live sessions and the window's top sessions, with shares, cost, token rate and labels.
fn session_rows(
    q: &Queries<'_>,
    live: &[LiveSession],
    p_now: Option<f64>,
    start: Timestamp,
    now: Timestamp,
) -> (Vec<SessionShareView>, Vec<SessionShareView>) {
    let account = q.account;
    let end = now.saturating_add_millis(1);
    let window_usage = q.roots(start, end);
    let recent_usage = q.roots(now.saturating_sub_millis(TOKEN_RATE_MS), end);
    let shares: HashMap<SessionId, Option<f64>> =
        share::shares(p_now, &window_usage).into_iter().collect();
    let usage_of = |id: SessionId| window_usage.iter().find(|u| u.root == id).copied();
    #[allow(clippy::cast_precision_loss)]
    let rate_of = |id: SessionId| {
        recent_usage.iter().find(|u| u.root == id).map_or(0.0, |u| {
            u.tokens as f64 / (TOKEN_RATE_MS as f64 / MIN as f64)
        })
    };
    let row = |id: Option<SessionId>| {
        let usage = id.and_then(usage_of);
        SessionShareView {
            session_id: id,
            session_uuid: None,
            name: None,
            title: None,
            project: None,
            status: None,
            share_percent: id.and_then(|i| shares.get(&i).copied().flatten()),
            cost: usage.map_or(MicroUsd::ZERO, |u: RootUsage| u.cost),
            tokens_per_min: id.map_or(0.0, rate_of),
        }
    };

    let mut live_rows: Vec<SessionShareView> = live
        .iter()
        .filter(|s| s.account == account)
        .map(|s| SessionShareView {
            session_uuid: Some(s.session_uuid.clone()),
            name: s.name.clone(),
            status: Some(s.status),
            ..row(s.session_id)
        })
        .collect();
    live_rows.sort_by(|a, b| {
        b.share_percent
            .unwrap_or(-1.0)
            .total_cmp(&a.share_percent.unwrap_or(-1.0))
            .then(b.cost.cmp(&a.cost))
    });
    let mut top_rows: Vec<SessionShareView> = window_usage
        .iter()
        .take(TOP_SESSIONS)
        .map(|u| {
            let live = live
                .iter()
                .find(|s| s.account == account && s.session_id == Some(u.root));
            SessionShareView {
                session_uuid: live.map(|s| s.session_uuid.clone()),
                name: live.and_then(|s| s.name.clone()),
                status: live.map(|s| s.status),
                ..row(Some(u.root))
            }
        })
        .collect();
    let ids: Vec<SessionId> = live_rows
        .iter()
        .chain(&top_rows)
        .filter_map(|r| r.session_id)
        .collect();
    match q.store.session_labels(&ids) {
        Ok(labels) => {
            for r in live_rows.iter_mut().chain(top_rows.iter_mut()) {
                if let Some(l) = labels.iter().find(|l| Some(l.id) == r.session_id) {
                    r.title.clone_from(&l.title);
                    r.project.clone_from(&l.project);
                }
            }
        }
        Err(error) => q.failed(&error),
    }
    (live_rows, top_rows)
}

#[cfg(test)]
mod tests;
