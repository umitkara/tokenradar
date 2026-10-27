//! When to call the usage API (architecture §10). Pure state: no thread, clock or network, so
//! every transition is testable with fake time.
//!
//! | Situation                                 | Interval |
//! |-------------------------------------------|----------|
//! | fresh statusline data (< 3 min)           | 15 min   |
//! | a live session is busy, no statusline     | 2 min    |
//! | live sessions, all idle                   | 5 min    |
//! | no live session                           | 15 min   |
//!
//! Extra triggers: start-up, the panel opening when the last success is over 60 s old, 30 s
//! after a `resets_at`, new credentials after a 401. Hard floor: 60 s between requests.
//! Regular intervals get ±10 % jitter. 429/5xx back off exponentially from 60 s to 30 min
//! (or as `Retry-After` says); network errors do the same in the offline state (ADR-028).

use serde::Serialize;
use specta::Type;

use crate::domain::Timestamp;

const SEC: i64 = 1000;
const MIN: i64 = 60 * SEC;

/// Never two requests closer than this.
pub const MIN_GAP_MS: i64 = MIN;
pub const BACKOFF_START_MS: i64 = MIN;
pub const BACKOFF_MAX_MS: i64 = 30 * MIN;
/// A `Retry-After` beyond this is treated as a misconfiguration.
pub const RETRY_AFTER_CAP_MS: i64 = 6 * 60 * MIN;
/// Statusline data younger than this makes the API a supplement only.
pub const STATUSLINE_FRESH_MS: i64 = 3 * MIN;
/// A reset is re-read this long after `resets_at`.
pub const RESET_GRACE_MS: i64 = 30 * SEC;
/// Opening the panel refreshes data older than this.
pub const PANEL_MAX_AGE_MS: i64 = MIN;
const JITTER: f64 = 0.10;

/// What the live sessions are doing, summarised for the interval table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    None,
    Idle,
    Busy,
}

/// Inputs from the rest of the core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PollInputs {
    /// Last statusline observation of this account.
    pub statusline_at: Option<Timestamp>,
    pub activity: Activity,
    /// Current `resets_at` of every limit of the account.
    pub resets: Vec<Timestamp>,
}

/// How a request ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Success,
    /// 401/403: the token expired. Wait for Claude Code to write a new one.
    Unauthorized,
    /// No credentials file or no token in it. Also waits for the file to change.
    NoToken,
    /// 429.
    RateLimited {
        retry_after_ms: Option<i64>,
    },
    /// 5xx.
    ServerError {
        retry_after_ms: Option<i64>,
    },
    /// Connect, DNS, TLS or timeout.
    Network,
    /// A 2xx body without usable limits, or an unexpected status.
    BadResponse,
}

/// The API client's state as the panel shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ApiState {
    /// No consent: the API is never called.
    Off,
    Active,
    /// 401 or no token; resumes when `.credentials.json` changes.
    TokenStale,
    Backoff {
        until: Timestamp,
    },
    Offline {
        retry_at: Timestamp,
    },
    /// The last response was unusable; retried with backoff.
    Error {
        retry_at: Timestamp,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    None,
    TokenStale,
    Backoff(Timestamp),
    Offline(Timestamp),
    Error(Timestamp),
}

/// Counters for `doctor`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
pub struct PollDiagnostics {
    pub requests: u64,
    pub errors: u64,
    /// A short category such as `rate_limited`; never response bodies or tokens.
    pub last_error: Option<String>,
    pub last_error_at: Option<Timestamp>,
    pub last_attempt: Option<Timestamp>,
    pub last_success: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct PollScheduler {
    consent: bool,
    block: Block,
    /// Current backoff step; 0 when the last request succeeded.
    backoff_ms: i64,
    /// A trigger asked for a request as soon as the floor allows.
    force: bool,
    /// Factor for the next regular interval, in `1 ± JITTER`.
    jitter: f64,
    rng: u64,
    diag: PollDiagnostics,
}

impl PollScheduler {
    /// `seed` drives the jitter; any value works, tests pass a fixed one.
    pub fn new(consent: bool, seed: u64) -> Self {
        let mut s = Self {
            consent,
            block: Block::None,
            backoff_ms: 0,
            force: true, // start-up
            jitter: 1.0,
            rng: seed | 1,
            diag: PollDiagnostics::default(),
        };
        s.jitter = s.next_jitter();
        s
    }

    pub fn diagnostics(&self) -> &PollDiagnostics {
        &self.diag
    }

    pub fn consent(&self) -> bool {
        self.consent
    }

    pub fn state(&self) -> ApiState {
        if !self.consent {
            return ApiState::Off;
        }
        match self.block {
            Block::None => ApiState::Active,
            Block::TokenStale => ApiState::TokenStale,
            Block::Backoff(until) => ApiState::Backoff { until },
            Block::Offline(retry_at) => ApiState::Offline { retry_at },
            Block::Error(retry_at) => ApiState::Error { retry_at },
        }
    }

    /// Regular interval for the current situation, before jitter.
    pub fn base_interval_ms(now: Timestamp, inputs: &PollInputs) -> i64 {
        let s1_fresh = inputs
            .statusline_at
            .is_some_and(|t| now.as_millis() - t.as_millis() < STATUSLINE_FRESH_MS);
        if s1_fresh {
            15 * MIN
        } else {
            match inputs.activity {
                Activity::Busy => 2 * MIN,
                Activity::Idle => 5 * MIN,
                Activity::None => 15 * MIN,
            }
        }
    }

    /// When the next request is due, or `None` when none may be sent (no consent, stale
    /// token). A time in the past means "now".
    pub fn due_at(&self, now: Timestamp, inputs: &PollInputs) -> Option<Timestamp> {
        if !self.consent {
            return None;
        }
        let last = self.diag.last_attempt;
        let floor = last.map(|t| t.saturating_add_millis(MIN_GAP_MS));
        let at = |t: Timestamp| Some(floor.map_or(t, |f| f.max(t)));
        match self.block {
            Block::TokenStale => return None,
            Block::Backoff(until) | Block::Offline(until) | Block::Error(until) => {
                return at(until);
            }
            Block::None => {}
        }
        let Some(last) = last else {
            return Some(now); // never asked yet
        };
        if self.force {
            return at(now);
        }
        // Truncation is fine: the interval is minutes, the error a fraction of a millisecond.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let interval = (Self::base_interval_ms(now, inputs) as f64 * self.jitter) as i64;
        let mut due = last.saturating_add_millis(interval);
        for reset in &inputs.resets {
            let t = reset.saturating_add_millis(RESET_GRACE_MS);
            if t > last {
                due = due.min(t);
            }
        }
        at(due)
    }

    /// A request is being sent now.
    pub fn on_attempt(&mut self, now: Timestamp) {
        self.diag.requests += 1;
        self.diag.last_attempt = Some(now);
        self.force = false;
        self.jitter = self.next_jitter();
    }

    pub fn on_outcome(&mut self, now: Timestamp, outcome: &Outcome) {
        let category = match outcome {
            Outcome::Success => {
                self.block = Block::None;
                self.backoff_ms = 0;
                self.diag.last_success = Some(now);
                return;
            }
            Outcome::Unauthorized => {
                self.block = Block::TokenStale;
                "unauthorized"
            }
            Outcome::NoToken => {
                self.block = Block::TokenStale;
                "no_token"
            }
            Outcome::RateLimited { retry_after_ms } | Outcome::ServerError { retry_after_ms } => {
                let wait = self.step_backoff(*retry_after_ms);
                self.block = Block::Backoff(now.saturating_add_millis(wait));
                if matches!(outcome, Outcome::RateLimited { .. }) {
                    "rate_limited"
                } else {
                    "server_error"
                }
            }
            Outcome::Network => {
                let wait = self.step_backoff(None);
                self.block = Block::Offline(now.saturating_add_millis(wait));
                "network"
            }
            Outcome::BadResponse => {
                let wait = self.step_backoff(None);
                self.block = Block::Error(now.saturating_add_millis(wait));
                "bad_response"
            }
        };
        self.diag.errors += 1;
        self.diag.last_error = Some(category.to_owned());
        self.diag.last_error_at = Some(now);
    }

    /// `.credentials.json` changed: a stale token may have been replaced.
    pub fn on_credentials_changed(&mut self) {
        if self.block == Block::TokenStale {
            self.block = Block::None;
            self.force = true;
        }
    }

    /// The panel opened. Refreshes data older than [`PANEL_MAX_AGE_MS`]; an offline client
    /// tries again at once. Backoff after 429/5xx is respected.
    pub fn on_panel_opened(&mut self, now: Timestamp) {
        let old = self
            .diag
            .last_success
            .is_none_or(|t| now.as_millis() - t.as_millis() > PANEL_MAX_AGE_MS);
        if !old {
            return;
        }
        match self.block {
            Block::None => self.force = true,
            Block::Offline(_) => {
                self.block = Block::Offline(now);
            }
            Block::TokenStale | Block::Backoff(_) | Block::Error(_) => {}
        }
    }

    pub fn set_consent(&mut self, granted: bool) {
        if granted && !self.consent {
            self.force = true;
        }
        self.consent = granted;
    }

    fn step_backoff(&mut self, retry_after_ms: Option<i64>) -> i64 {
        self.backoff_ms = if self.backoff_ms == 0 {
            BACKOFF_START_MS
        } else {
            (self.backoff_ms * 2).min(BACKOFF_MAX_MS)
        };
        retry_after_ms.map_or(self.backoff_ms, |r| r.clamp(MIN_GAP_MS, RETRY_AFTER_CAP_MS))
    }

    /// xorshift64*: jitter needs no quality, only spread.
    fn next_jitter(&mut self) -> f64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        let x = self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
        #[allow(clippy::cast_precision_loss)]
        let unit = x as f64 / (1u64 << 53) as f64;
        1.0 - JITTER + 2.0 * JITTER * unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_793_000_000_000;

    fn t(ms: i64) -> Timestamp {
        Timestamp::from_millis(T0 + ms)
    }

    fn idle() -> PollInputs {
        PollInputs {
            activity: Activity::Idle,
            ..PollInputs::default()
        }
    }

    /// Sends a request whenever it is due and returns the attempt times (relative to T0).
    fn run(s: &mut PollScheduler, inputs: &PollInputs, until: i64, outcome: &Outcome) -> Vec<i64> {
        let mut out = Vec::new();
        let mut now = t(0);
        while now < t(until) {
            match s.due_at(now, inputs) {
                Some(due) if due <= now => {
                    s.on_attempt(now);
                    s.on_outcome(now, outcome);
                    out.push(now.as_millis() - T0);
                }
                Some(due) => now = due.min(t(until)),
                None => break,
            }
        }
        out
    }

    #[test]
    fn without_consent_nothing_is_ever_due() {
        let mut s = PollScheduler::new(false, 1);
        assert_eq!(s.due_at(t(0), &idle()), None);
        s.on_panel_opened(t(0));
        s.on_credentials_changed();
        assert_eq!(s.due_at(t(0), &idle()), None);
        assert_eq!(s.state(), ApiState::Off);
    }

    #[test]
    fn interval_table() {
        let now = t(0);
        let base = |statusline_at, activity| {
            PollScheduler::base_interval_ms(
                now,
                &PollInputs {
                    statusline_at,
                    activity,
                    resets: vec![],
                },
            ) / MIN
        };
        assert_eq!(base(Some(t(-2 * MIN)), Activity::Busy), 15);
        assert_eq!(base(Some(t(-4 * MIN)), Activity::Busy), 2);
        assert_eq!(base(None, Activity::Busy), 2);
        assert_eq!(base(None, Activity::Idle), 5);
        assert_eq!(base(None, Activity::None), 15);
    }

    #[test]
    fn polls_at_once_on_start_then_follows_the_interval_with_jitter() {
        let mut s = PollScheduler::new(true, 42);
        let times = run(&mut s, &idle(), 60 * MIN, &Outcome::Success);
        assert_eq!(times[0], 0);
        for pair in times.windows(2) {
            let gap = pair[1] - pair[0];
            assert!((270 * SEC..=330 * SEC).contains(&gap), "gap {gap}");
        }
        // 60 min / 5 min ± 10 %
        assert!((11..=14).contains(&times.len()), "{times:?}");
        // Jitter actually varies.
        let gaps: std::collections::HashSet<_> = times.windows(2).map(|p| p[1] - p[0]).collect();
        assert!(gaps.len() > 1);
    }

    #[test]
    fn triggers_never_break_the_sixty_second_floor() {
        let mut s = PollScheduler::new(true, 7);
        s.on_attempt(t(0));
        s.on_outcome(t(0), &Outcome::Success);
        s.on_panel_opened(t(10 * SEC)); // data 10 s old: no refresh
        assert!(s.due_at(t(10 * SEC), &idle()).expect("due") >= t(270 * SEC));
        s.on_panel_opened(t(61 * SEC + 1)); // older than 60 s
        assert_eq!(s.due_at(t(61 * SEC + 1), &idle()), Some(t(61 * SEC + 1)));
        // A reset 5 s after the last request asks for a re-read 30 s after the reset, but no
        // sooner than 60 s after that request.
        let mut s = PollScheduler::new(true, 7);
        s.on_attempt(t(0));
        s.on_outcome(t(0), &Outcome::Success);
        let inputs = PollInputs {
            resets: vec![t(5 * SEC)],
            ..idle()
        };
        assert_eq!(s.due_at(t(1), &inputs), Some(t(MIN)));
        let inputs = PollInputs {
            resets: vec![t(2 * MIN)],
            ..idle()
        };
        assert_eq!(s.due_at(t(1), &inputs), Some(t(2 * MIN + RESET_GRACE_MS)));
    }

    #[test]
    fn rate_limits_back_off_exponentially_up_to_thirty_minutes() {
        let mut s = PollScheduler::new(true, 3);
        let times = run(
            &mut s,
            &idle(),
            3 * 60 * MIN,
            &Outcome::RateLimited {
                retry_after_ms: None,
            },
        );
        let gaps: Vec<i64> = times.windows(2).map(|p| (p[1] - p[0]) / MIN).collect();
        assert_eq!(&gaps[..6], &[1, 2, 4, 8, 16, 30]);
        assert!(gaps[6..].iter().all(|&g| g == 30));
        assert!(matches!(s.state(), ApiState::Backoff { .. }));
        assert_eq!(s.diagnostics().last_error.as_deref(), Some("rate_limited"));
    }

    #[test]
    fn retry_after_is_obeyed_and_success_resets_the_backoff() {
        let mut s = PollScheduler::new(true, 3);
        s.on_attempt(t(0));
        s.on_outcome(
            t(0),
            &Outcome::ServerError {
                retry_after_ms: Some(7 * MIN),
            },
        );
        assert_eq!(s.due_at(t(1), &idle()), Some(t(7 * MIN)));
        // Panel opening does not override a server's backoff.
        s.on_panel_opened(t(2 * MIN));
        assert_eq!(s.due_at(t(2 * MIN), &idle()), Some(t(7 * MIN)));
        // Too short a Retry-After still honours the floor.
        s.on_attempt(t(7 * MIN));
        s.on_outcome(
            t(7 * MIN),
            &Outcome::RateLimited {
                retry_after_ms: Some(1),
            },
        );
        assert_eq!(s.due_at(t(7 * MIN), &idle()), Some(t(8 * MIN)));
        s.on_attempt(t(8 * MIN));
        s.on_outcome(t(8 * MIN), &Outcome::Success);
        assert_eq!(s.state(), ApiState::Active);
        s.on_attempt(t(20 * MIN));
        s.on_outcome(t(20 * MIN), &Outcome::Network);
        // The backoff starts over at 60 s.
        assert_eq!(s.due_at(t(20 * MIN), &idle()), Some(t(21 * MIN)));
    }

    #[test]
    fn unauthorized_waits_for_new_credentials() {
        let mut s = PollScheduler::new(true, 9);
        s.on_attempt(t(0));
        s.on_outcome(t(0), &Outcome::Unauthorized);
        assert_eq!(s.state(), ApiState::TokenStale);
        assert_eq!(s.due_at(t(24 * 60 * MIN), &idle()), None);
        s.on_panel_opened(t(MIN * 5));
        assert_eq!(s.due_at(t(MIN * 5), &idle()), None);
        s.on_credentials_changed();
        assert_eq!(s.state(), ApiState::Active);
        assert_eq!(
            s.due_at(t(30 * SEC), &idle()),
            Some(t(MIN)),
            "floor still applies"
        );
        assert_eq!(s.due_at(t(5 * MIN), &idle()), Some(t(5 * MIN)));
        s.on_attempt(t(5 * MIN));
        s.on_outcome(t(5 * MIN), &Outcome::Success);
        assert_eq!(s.diagnostics().errors, 1);
        assert_eq!(s.diagnostics().requests, 2);
    }

    #[test]
    fn offline_retries_with_backoff_and_on_panel_open() {
        let mut s = PollScheduler::new(true, 5);
        s.on_attempt(t(0));
        s.on_outcome(t(0), &Outcome::Network);
        s.on_attempt(t(MIN));
        s.on_outcome(t(MIN), &Outcome::Network);
        assert_eq!(
            s.state(),
            ApiState::Offline {
                retry_at: t(3 * MIN)
            }
        );
        s.on_panel_opened(t(MIN + 70 * SEC));
        assert_eq!(
            s.due_at(t(MIN + 70 * SEC), &idle()),
            Some(t(MIN + 70 * SEC))
        );
    }

    #[test]
    fn granting_consent_polls_at_once() {
        let mut s = PollScheduler::new(false, 5);
        assert_eq!(s.due_at(t(0), &idle()), None);
        s.set_consent(true);
        assert_eq!(s.due_at(t(0), &idle()), Some(t(0)));
        s.set_consent(false);
        assert_eq!(s.due_at(t(0), &idle()), None);
    }
}
