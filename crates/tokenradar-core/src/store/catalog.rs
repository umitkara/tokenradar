//! Accounts, projects, sessions and models.

use rusqlite::{Connection, OptionalExtension, params};

use super::Result;
use crate::domain::{
    Account, AccountId, AccountSpec, ModelFamily, ModelId, ProjectId, ProjectSpec, Session,
    SessionId, SessionSpec, SessionTitle, Timestamp, TitleSource,
};

/// Inserts or updates the account identified by `(provider, config_dir)`.
pub(crate) fn upsert_account(
    conn: &Connection,
    spec: &AccountSpec,
    now: Timestamp,
) -> Result<AccountId> {
    let id = conn
        .prepare_cached(
            "INSERT INTO accounts (provider, label, config_dir, plan, tier, environment, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (provider, config_dir) DO UPDATE SET
               label = excluded.label,
               plan = COALESCE(excluded.plan, plan),
               tier = COALESCE(excluded.tier, tier),
               environment = excluded.environment
             RETURNING id",
        )?
        .query_row(
            params![
                spec.provider.as_str(),
                spec.label,
                spec.config_dir,
                spec.plan,
                spec.tier,
                spec.environment,
                now
            ],
            |r| r.get(0),
        )?;
    Ok(id)
}

pub(crate) fn account(conn: &Connection, id: AccountId) -> Result<Option<Account>> {
    let row = conn
        .prepare_cached(
            "SELECT provider, label, config_dir, plan, tier, environment, created_at
             FROM accounts WHERE id = ?1",
        )?
        .query_row([id], |r| {
            Ok((
                parse_col(r, 0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })
        .optional()?;
    Ok(row.map(
        |(provider, label, config_dir, plan, tier, environment, created_at)| Account {
            id,
            spec: AccountSpec {
                provider,
                label,
                config_dir,
                plan,
                tier,
                environment,
            },
            created_at,
        },
    ))
}

pub(crate) fn upsert_project(conn: &Connection, spec: &ProjectSpec) -> Result<ProjectId> {
    let id = conn
        .prepare_cached(
            "INSERT INTO projects (account_id, path_norm, display_name) VALUES (?1, ?2, ?3)
             ON CONFLICT (account_id, path_norm) DO UPDATE SET display_name = excluded.display_name
             RETURNING id",
        )?
        .query_row(
            params![spec.account_id, spec.path_norm, spec.display_name],
            |r| r.get(0),
        )?;
    Ok(id)
}

/// Inserts or merges the session identified by `(account_id, uuid)`: `None` fields keep the
/// stored value, `first_ts` only moves earlier and `last_ts` only later. A title replaces the
/// stored one only when its source ranks the same or higher ([`TitleSource::rank`]).
pub(crate) fn upsert_session(conn: &Connection, spec: &SessionSpec) -> Result<SessionId> {
    let (title, title_source) = match &spec.title {
        Some(t) => (Some(t.text.as_str()), Some(t.source.as_str())),
        None => (None, None),
    };
    let id = conn
        .prepare_cached(
            "INSERT INTO sessions (account_id, uuid, kind, parent_id, root_id, project_id, title,
                                   title_source, git_branch, entrypoint, cc_version, first_ts,
                                   last_ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT (account_id, uuid) DO UPDATE SET
               kind       = excluded.kind,
               parent_id  = COALESCE(excluded.parent_id, parent_id),
               root_id    = COALESCE(excluded.root_id, root_id),
               project_id = COALESCE(excluded.project_id, project_id),
               title      = CASE WHEN excluded.title IS NOT NULL AND (title IS NULL OR
                               (CASE excluded.title_source WHEN 'custom' THEN 3 WHEN 'agent' THEN 2 ELSE 1 END)
                            >= (CASE title_source WHEN 'custom' THEN 3 WHEN 'agent' THEN 2 ELSE 1 END))
                             THEN excluded.title ELSE title END,
               title_source = CASE WHEN excluded.title IS NOT NULL AND (title IS NULL OR
                               (CASE excluded.title_source WHEN 'custom' THEN 3 WHEN 'agent' THEN 2 ELSE 1 END)
                            >= (CASE title_source WHEN 'custom' THEN 3 WHEN 'agent' THEN 2 ELSE 1 END))
                             THEN excluded.title_source ELSE title_source END,
               git_branch = COALESCE(excluded.git_branch, git_branch),
               entrypoint = COALESCE(excluded.entrypoint, entrypoint),
               cc_version = COALESCE(excluded.cc_version, cc_version),
               first_ts   = MIN(COALESCE(excluded.first_ts, first_ts), COALESCE(first_ts, excluded.first_ts)),
               last_ts    = MAX(COALESCE(excluded.last_ts, last_ts), COALESCE(last_ts, excluded.last_ts))
             RETURNING id",
        )?
        .query_row(
            params![
                spec.account_id,
                spec.uuid,
                spec.kind.as_str(),
                spec.parent_id,
                spec.root_id,
                spec.project_id,
                title,
                title_source,
                spec.git_branch,
                spec.entrypoint,
                spec.cc_version,
                spec.first_ts,
                spec.last_ts
            ],
            |r| r.get(0),
        )?;
    Ok(id)
}

pub(crate) fn session_id_by_uuid(
    conn: &Connection,
    account: AccountId,
    uuid: &str,
) -> Result<Option<SessionId>> {
    Ok(conn
        .prepare_cached("SELECT id FROM sessions WHERE account_id = ?1 AND uuid = ?2")?
        .query_row(params![account, uuid], |r| r.get(0))
        .optional()?)
}

pub(crate) fn session(conn: &Connection, id: SessionId) -> Result<Option<Session>> {
    conn.prepare_cached(
        "SELECT account_id, uuid, kind, parent_id, root_id, project_id, title, git_branch,
                entrypoint, cc_version, first_ts, last_ts, title_source
         FROM sessions WHERE id = ?1",
    )?
    .query_row([id], |r| {
        let title: Option<String> = r.get(6)?;
        let source: Option<TitleSource> = match r.get::<_, Option<String>>(12)? {
            Some(_) => Some(parse_col(r, 12)?),
            None => None,
        };
        let spec = SessionSpec {
            account_id: r.get(0)?,
            uuid: r.get(1)?,
            kind: parse_col(r, 2)?,
            parent_id: r.get(3)?,
            root_id: r.get(4)?,
            project_id: r.get(5)?,
            // A title without a source predates `title_source`; treat it as the weakest.
            title: title.map(|text| SessionTitle {
                text,
                source: source.unwrap_or(TitleSource::Ai),
            }),
            git_branch: r.get(7)?,
            entrypoint: r.get(8)?,
            cc_version: r.get(9)?,
            first_ts: r.get(10)?,
            last_ts: r.get(11)?,
        };
        Ok(Session { id, spec })
    })
    .optional()
    .map_err(Into::into)
}

/// Returns the id for a model name, inserting it on first sight. A stored `unknown` family is
/// replaced once the pricing table learns the model; a known family is never rewritten.
pub(crate) fn upsert_model(conn: &Connection, name: &str, family: ModelFamily) -> Result<ModelId> {
    let id = conn
        .prepare_cached(
            "INSERT INTO models (name, family) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET
               family = CASE WHEN family = 'unknown' THEN excluded.family ELSE family END
             RETURNING id",
        )?
        .query_row(params![name, family.as_str()], |r| r.get(0))?;
    Ok(id)
}

/// Reads a `TEXT` column into a `FromStr` domain enum.
pub(crate) fn parse_col<T>(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let text: String = row.get(idx)?;
    text.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
    })
}
