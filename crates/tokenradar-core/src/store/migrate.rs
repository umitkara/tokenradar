//! Embedded, ordered migrations tracked by `PRAGMA user_version`.

use rusqlite::Connection;

use super::{Result, StoreError};

/// `(version, sql)`. Append only; a released migration is never edited.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("migrations/0001_init.sql"))];

pub(crate) const LATEST: i64 = MIGRATIONS[MIGRATIONS.len() - 1].0;

/// Applies every migration newer than the database's `user_version`, each in its own
/// transaction together with the version bump.
pub(crate) fn run(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > LATEST {
        return Err(StoreError::SchemaTooNew {
            found: current,
            supported: LATEST,
        });
    }
    for &(version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
        tracing::info!(version, "applied database migration");
    }
    Ok(())
}
