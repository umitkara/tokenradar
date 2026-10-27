//! A small key-value table for app state that does not deserve its own table.

use rusqlite::{Connection, OptionalExtension, params};

use super::Result;

pub(crate) fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.prepare_cached("SELECT value FROM kv WHERE key = ?1")?
        .query_row([key], |r| r.get(0))
        .optional()
        .map_err(Into::into)
}

pub(crate) fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.prepare_cached(
        "INSERT INTO kv (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )?
    .execute(params![key, value])?;
    Ok(())
}

pub(crate) fn delete(conn: &Connection, key: &str) -> Result<()> {
    conn.prepare_cached("DELETE FROM kv WHERE key = ?1")?
        .execute([key])?;
    Ok(())
}
