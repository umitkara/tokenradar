//! OAuth usage API (data-sources.md S2): parser, transport, schedule and the poller thread.
//!
//! Consent gates everything: without it the token is never read and no request is sent.
//! With it, the token is read from `.credentials.json` right before each request, used for
//! that request only and zeroized (ADR-006). It is never refreshed.

mod client;
mod parse;
mod schedule;

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

pub use client::{HttpReply, HttpTransport, TransportError, USAGE_URL, UsageTransport};
pub use parse::{
    ExtraUsage, SurfaceBreakdown, UsageParseError, UsageReport, UsageSchema, parse_usage,
};
pub use schedule::{
    Activity, ApiState, MIN_GAP_MS, Outcome, PollDiagnostics, PollInputs, PollScheduler,
};

use super::credentials::{self, CredentialsError};
use crate::clock::Clock;
use crate::domain::AccountId;

/// Upper bound on one sleep, so a wrong clock jump cannot park the thread for days.
const MAX_SLEEP: Duration = Duration::from_mins(15);

/// Commands for a running poller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollerMsg {
    Inputs(PollInputs),
    CredentialsChanged,
    PanelOpened,
    Consent(bool),
    Stop,
}

/// What the poller reports after every request or state change.
#[derive(Debug, Clone, PartialEq)]
pub struct PollUpdate {
    pub account: AccountId,
    pub state: ApiState,
    pub diagnostics: PollDiagnostics,
    /// Present after a successful request.
    pub report: Option<UsageReport>,
}

/// Polls one account's usage.
pub struct UsageApiPoller<T: UsageTransport, C: Clock> {
    account: AccountId,
    config_dir: PathBuf,
    scheduler: PollScheduler,
    transport: T,
    clock: C,
    inputs: PollInputs,
}

impl<T: UsageTransport, C: Clock> UsageApiPoller<T, C> {
    pub fn new(
        account: AccountId,
        config_dir: PathBuf,
        consent: bool,
        transport: T,
        clock: C,
    ) -> Self {
        let seed = clock.now().as_millis().unsigned_abs() ^ (account.0.unsigned_abs() << 32);
        Self {
            account,
            config_dir,
            scheduler: PollScheduler::new(consent, seed),
            transport,
            clock,
            inputs: PollInputs::default(),
        }
    }

    pub fn update(&self, report: Option<UsageReport>) -> PollUpdate {
        PollUpdate {
            account: self.account,
            state: self.scheduler.state(),
            diagnostics: self.scheduler.diagnostics().clone(),
            report,
        }
    }

    /// Sends one request now, whatever the schedule says, and records the outcome. Returns
    /// the parsed report on success. Never called without consent.
    pub fn request(&mut self) -> Option<UsageReport> {
        debug_assert!(self.scheduler.consent(), "request without consent");
        if !self.scheduler.consent() {
            return None;
        }
        let now = self.clock.now();
        self.scheduler.on_attempt(now);
        let (outcome, report) = self.fetch();
        let done = self.clock.now();
        self.scheduler.on_outcome(done, &outcome);
        match &outcome {
            Outcome::Success => tracing::debug!(account = %self.account, "usage api: ok"),
            other => {
                tracing::info!(account = %self.account, outcome = ?other, "usage api: failed");
            }
        }
        report
    }

    fn fetch(&mut self) -> (Outcome, Option<UsageReport>) {
        let reply = {
            let token = match credentials::read_access_token(&self.config_dir) {
                Ok(token) => token,
                Err(CredentialsError::Missing | CredentialsError::NoToken) => {
                    return (Outcome::NoToken, None);
                }
                Err(error) => {
                    tracing::warn!(account = %self.account, %error, "usage api: credentials unreadable");
                    return (Outcome::NoToken, None);
                }
            };
            self.transport.get_usage(&token)
            // `token` is dropped and zeroized here, before the response is looked at.
        };
        let reply = match reply {
            Ok(reply) => reply,
            Err(TransportError::BadToken) => return (Outcome::Unauthorized, None),
            // Shutting down: the outcome is never looked at.
            Err(TransportError::Cancelled) => return (Outcome::Network, None),
            Err(error) => {
                tracing::debug!(account = %self.account, %error, "usage api: transport error");
                return (Outcome::Network, None);
            }
        };
        match reply.status {
            200..=299 => match parse_usage(&reply.body, self.account, self.clock.now()) {
                Ok(report) if report.schema != UsageSchema::Empty => {
                    (Outcome::Success, Some(report))
                }
                Ok(_) | Err(_) => (Outcome::BadResponse, None),
            },
            401 | 403 => (Outcome::Unauthorized, None),
            429 => (
                Outcome::RateLimited {
                    retry_after_ms: reply.retry_after_ms,
                },
                None,
            ),
            500..=599 => (
                Outcome::ServerError {
                    retry_after_ms: reply.retry_after_ms,
                },
                None,
            ),
            _ => (Outcome::BadResponse, None),
        }
    }

    /// Applies a command. Returns `false` for [`PollerMsg::Stop`], and whether the visible
    /// state may have changed.
    fn handle(&mut self, msg: PollerMsg) -> Option<bool> {
        let now = self.clock.now();
        let before = self.scheduler.state();
        match msg {
            PollerMsg::Stop => return None,
            PollerMsg::Inputs(inputs) => self.inputs = inputs,
            PollerMsg::CredentialsChanged => self.scheduler.on_credentials_changed(),
            PollerMsg::PanelOpened => self.scheduler.on_panel_opened(now),
            PollerMsg::Consent(granted) => self.scheduler.set_consent(granted),
        }
        Some(self.scheduler.state() != before)
    }

    /// The thread body: sleeps until the next request is due or a command arrives.
    pub fn run(mut self, rx: &mpsc::Receiver<PollerMsg>, mut emit: impl FnMut(PollUpdate)) {
        emit(self.update(None));
        loop {
            let now = self.clock.now();
            let msg = match self.scheduler.due_at(now, &self.inputs) {
                Some(due) if due <= now => {
                    let report = self.request();
                    emit(self.update(report));
                    continue;
                }
                Some(due) => {
                    let wait = Duration::from_millis(
                        u64::try_from(due.as_millis() - now.as_millis()).unwrap_or(0),
                    )
                    .min(MAX_SLEEP);
                    match rx.recv_timeout(wait) {
                        Ok(msg) => msg,
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                // Nothing may be sent (no consent, stale token): sleep until told otherwise.
                None => match rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => return,
                },
            };
            match self.handle(msg) {
                None => return,
                Some(true) => emit(self.update(None)),
                Some(false) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests;
