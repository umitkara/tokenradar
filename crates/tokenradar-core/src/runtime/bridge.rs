//! The statusline bridge inside the runtime: the IPC handler and the spool import.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use super::{CoreStatus, LimitsMsg, lock};
use crate::domain::{AccountId, LimitObservation, Timestamp};
use crate::ipc::proto::Handler;
use crate::ipc::{CompactCount, LimitsReply, SessionInfo, SessionsReply, StatusQuery, StatusReply};
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
    /// For the asked session's compactions (Step 12n): one small indexed read per call.
    pub store: Store,
}

impl CoreIpc {
    fn snapshot(&self, session: Option<&str>) -> StatusReply {
        lock(&self.status).limits.as_ref().map_or_else(
            || StatusReply::empty(Timestamp::now()),
            |s| StatusReply::from_snapshot(s, session),
        )
    }
}

/// Adds the session's compactions to `reply`. A session the view does not list gets an entry
/// of its own; a session the database does not know stays without a count.
pub(super) fn add_compacts(reply: &mut StatusReply, uuid: &str, store: &Store) {
    let counts = match store.compact_counts(uuid) {
        Ok(Some(c)) => CompactCount::from(c),
        Ok(None) => return,
        Err(error) => {
            tracing::debug!(%error, "could not read compactions");
            return;
        }
    };
    match &mut reply.session {
        Some(s) => s.compacts = Some(counts),
        None => reply.session = Some(SessionInfo::compacts_only(uuid, counts)),
    }
}

impl Handler for CoreIpc {
    fn push(&self, push: StatuslinePush) {
        let readings = observations(&push, &self.accounts);
        if !readings.is_empty() {
            let _ = self.limits_tx.send(LimitsMsg::Statusline(readings));
        }
    }

    fn status(&self, query: &StatusQuery) -> StatusReply {
        let mut reply = self.snapshot(query.session_uuid.as_deref());
        if let Some(uuid) = query.session_uuid.as_deref() {
            add_compacts(&mut reply, uuid, &self.store);
        }
        reply
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

/// Feeds every spooled payload with a reading not yet stored to `ingest`, oldest first, as
/// `(all readings, new readings)`, and deletes each file once it is done. A store error leaves
/// the file for the next round.
pub(super) fn import_spool(
    dir: &Path,
    store: &Store,
    accounts: &[(AccountId, PathBuf)],
    mut ingest: impl FnMut(&[LimitObservation], &[LimitObservation]),
) -> SpoolImport {
    let mut report = SpoolImport::default();
    for claimed in spool::claim(dir) {
        report.malformed += claimed.malformed;
        let mut complete = true;
        for push in &claimed.pushes {
            let readings = observations(push, accounts);
            let mut new = Vec::new();
            for obs in &readings {
                match store.snapshot_exists(obs) {
                    Ok(true) => report.duplicates += 1,
                    Ok(false) => {
                        new.push(obs.clone());
                        report.ingested += 1;
                    }
                    Err(error) => {
                        tracing::warn!(%error, "could not check a spooled reading");
                        complete = false;
                    }
                }
            }
            // A payload seen before (every reading stored) must not count twice for presence.
            if !new.is_empty() {
                ingest(&readings, &new);
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

#[cfg(test)]
mod compact_tests {
    use super::*;
    use crate::domain::{
        AccountSpec, CompactEvent, CompactTrigger, Provider, SessionKind, SessionSpec,
    };
    use crate::store::IndexBatch;

    #[test]
    fn the_asked_session_gets_its_compactions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path().join("db.sqlite")).expect("store");
        let w = store.writer();
        let account = w
            .upsert_account(AccountSpec {
                provider: Provider::Claude,
                label: "Claude".into(),
                config_dir: "C:/c".into(),
                plan: None,
                tier: None,
                environment: "native".into(),
            })
            .expect("account");
        for uuid in ["s1", "s2"] {
            w.upsert_session(SessionSpec::new(account, uuid, SessionKind::Main))
                .expect("session");
        }
        w.commit_index_batch(IndexBatch {
            compacts: vec![CompactEvent {
                uuid: "c-1".into(),
                session_uuid: "s1".into(),
                at: Timestamp::from_millis(1),
                trigger: CompactTrigger::Manual,
                pre_tokens: None,
            }],
            ..IndexBatch::default()
        })
        .expect("batch");

        // Not in the view: an entry of its own, nothing measured.
        let mut reply = StatusReply::empty(Timestamp::from_millis(5));
        add_compacts(&mut reply, "s1", &store);
        let s = reply.session.expect("session");
        assert_eq!(s.session_uuid.as_deref(), Some("s1"));
        assert_eq!(s.compacts.map(|c| (c.manual, c.total())), Some((1, 1)));
        assert_eq!(s.share_percent, None);

        // In the view: the count joins what the view has.
        let mut reply = StatusReply::empty(Timestamp::from_millis(5));
        let mut listed = SessionInfo::compacts_only("s2", CompactCount::default());
        listed.share_percent = Some(4.0);
        listed.compacts = None;
        reply.session = Some(listed);
        add_compacts(&mut reply, "s2", &store);
        let s = reply.session.expect("session");
        assert_eq!(
            (s.share_percent, s.compacts),
            (Some(4.0), Some(CompactCount::default()))
        );

        // Unknown to the database: no count at all.
        let mut reply = StatusReply::empty(Timestamp::from_millis(5));
        add_compacts(&mut reply, "new", &store);
        assert!(reply.session.is_none());
    }
}
