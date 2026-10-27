//! Shared helpers for store tests.

use rusqlite::Connection;

use super::{catalog, configure_writer, migrate};
use crate::domain::{
    AccountId, AccountSpec, MicroUsd, ModelFamily, ModelId, MsgKey, Provider, SessionId,
    SessionKind, SessionSpec, Timestamp, TokenCounts, Usage, UsageEvent,
};

pub(crate) fn memory_db() -> Connection {
    let mut conn = Connection::open_in_memory().expect("open in-memory db");
    configure_writer(&conn).expect("pragmas");
    migrate::run(&mut conn).expect("migrate");
    conn
}

/// One account with two sessions and two models.
pub(crate) struct Fixture {
    pub account: AccountId,
    pub sessions: [SessionId; 2],
    pub models: [ModelId; 2],
}

pub(crate) fn account_spec(config_dir: &str) -> AccountSpec {
    AccountSpec {
        provider: Provider::Claude,
        label: "Personal".into(),
        config_dir: config_dir.into(),
        plan: None,
        tier: None,
        environment: "native".into(),
    }
}

pub(crate) fn seed(conn: &Connection) -> Fixture {
    let account = catalog::upsert_account(
        conn,
        &account_spec("C:/Users/test/.claude"),
        Timestamp::from_millis(0),
    )
    .expect("account");
    let session = |uuid: &str| {
        catalog::upsert_session(conn, &SessionSpec::new(account, uuid, SessionKind::Main))
            .expect("session")
    };
    let model = |name: &str, family| catalog::upsert_model(conn, name, family).expect("model");
    Fixture {
        account,
        sessions: [session("s-0"), session("s-1")],
        models: [
            model("claude-opus-5-5", ModelFamily::Opus),
            model("claude-sonnet-5-5", ModelFamily::Sonnet),
        ],
    }
}

impl Fixture {
    /// An event whose token counts are derived from `output` so that every column differs.
    pub(crate) fn event(
        &self,
        key: u8,
        output: u64,
        ts_ms: i64,
        session: usize,
        model: usize,
    ) -> UsageEvent {
        UsageEvent {
            account_id: self.account,
            session_id: self.sessions[session],
            model_id: self.models[model],
            ts: Timestamp::from_millis(ts_ms),
            msg_key: MsgKey::from_ids(&format!("msg_{key}"), &format!("req_{key}")),
            usage: Usage {
                tokens: TokenCounts {
                    input: output % 7 + 1,
                    output,
                    cache_read: output * 3,
                    cache_w5m: output * 2,
                    cache_w1h: output / 2,
                    thinking: output / 3,
                },
                ..Usage::default()
            },
            cost: MicroUsd(i64::try_from(output).expect("small") * 11 + 5),
        }
    }
}
