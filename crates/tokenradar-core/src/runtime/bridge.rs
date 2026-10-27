//! The statusline bridge inside the runtime: the IPC handler and the spool import.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use super::{CoreStatus, LimitsMsg, lock};
use crate::domain::{AccountId, LimitObservation, Timestamp};
use crate::ipc::proto::Handler;
use crate::ipc::{LimitsReply, SessionsReply, StatusQuery, StatusReply};
use crate::statusline::{StatuslinePush, account_for, spool};
use crate::store::Store;

/// Where the IPC server listens (see [`crate::ipc::pipe_name`]).
#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub pipe_name: String,
    /// Reported to clients in `hello`, e.g. `tokenradar-app/0.1.0`.
    pub server_id: String,
}

/// Answers the CLI from the runtime's state.
pub(super) struct CoreIpc {
    pub limits_tx: mpsc::Sender<LimitsMsg>,
    pub status: Arc<Mutex<CoreStatus>>,
    pub accounts: Vec<(AccountId, PathBuf)>,
}

impl CoreIpc {
    fn snapshot(&self, session: Option<&str>) -> StatusReply {
        lock(&self.status).limits.as_ref().map_or_else(
            || StatusReply::empty(Timestamp::now()),
            |s| StatusReply::from_snapshot(s, session),
        )
    }
}

impl Handler for CoreIpc {
    fn push(&self, push: StatuslinePush) {
        for obs in observations(&push, &self.accounts) {
            let _ = self.limits_tx.send(LimitsMsg::Observe(obs));
        }
    }

    fn status(&self, query: &StatusQuery) -> StatusReply {
        self.snapshot(query.session_uuid.as_deref())
    }

    fn limits(&self) -> LimitsReply {
        self.snapshot(None).limits_reply()
    }

    fn sessions(&self) -> SessionsReply {
        self.snapshot(None).sessions_reply()
    }
}

/// The readings of a push, attributed by its transcript path.
pub(super) fn observations(
    push: &StatuslinePush,
    accounts: &[(AccountId, PathBuf)],
) -> Vec<LimitObservation> {
    let account = account_for(
        push.payload.transcript_path.as_deref(),
        accounts.iter().map(|(id, dir)| (*id, dir.as_path())),
    );
    if let Some(account) = account {
        push.payload.observations(account, push.observed_at)
    } else {
        tracing::debug!("statusline reading for an unknown config directory");
        Vec::new()
    }
}

/// What one spool import did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpoolImport {
    pub ingested: usize,
    pub duplicates: usize,
    pub malformed: usize,
    pub files: usize,
}

/// Feeds every spooled reading not yet stored to `ingest`, oldest first, and deletes each file
/// once it is done. A store error leaves the file for the next round.
pub(super) fn import_spool(
    dir: &Path,
    store: &Store,
    accounts: &[(AccountId, PathBuf)],
    mut ingest: impl FnMut(&LimitObservation),
) -> SpoolImport {
    let mut report = SpoolImport::default();
    for claimed in spool::claim(dir) {
        report.malformed += claimed.malformed;
        let mut complete = true;
        for push in &claimed.pushes {
            for obs in observations(push, accounts) {
                match store.snapshot_exists(&obs) {
                    Ok(true) => report.duplicates += 1,
                    Ok(false) => {
                        ingest(&obs);
                        report.ingested += 1;
                    }
                    Err(error) => {
                        tracing::warn!(%error, "could not check a spooled reading");
                        complete = false;
                    }
                }
            }
        }
        if complete {
            report.files += 1;
            if let Err(error) = claimed.finish() {
                tracing::warn!(%error, "could not delete an imported spool file");
            }
        }
    }
    if report != SpoolImport::default() {
        tracing::info!(?report, "imported spooled statusline readings");
    }
    report
}
