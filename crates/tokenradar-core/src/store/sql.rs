//! `rusqlite` conversions for domain types.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

use crate::domain::{AccountId, ModelId, MsgKey, ProjectId, SessionId, Timestamp};

macro_rules! sql_i64_newtype {
    ($($ty:ty),+) => {$(
        impl ToSql for $ty {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.0))
            }
        }

        impl FromSql for $ty {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                i64::column_result(value).map(Self)
            }
        }
    )+};
}

sql_i64_newtype!(AccountId, SessionId, ProjectId, ModelId);

impl ToSql for Timestamp {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_millis()))
    }
}

impl FromSql for Timestamp {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(Self::from_millis)
    }
}

impl ToSql for MsgKey {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(&self.0[..]))
    }
}

impl FromSql for MsgKey {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let blob = value.as_blob()?;
        <[u8; 16]>::try_from(blob)
            .map(Self)
            .map_err(|_| FromSqlError::InvalidBlobSize {
                expected_size: 16,
                blob_size: blob.len(),
            })
    }
}

/// Token counts are `u64` in the domain and `INTEGER` (i64) in SQLite. Real counts are far
/// below `i64::MAX`; saturate rather than wrap if a corrupt line claims otherwise.
pub(crate) fn tokens_to_sql(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
pub(crate) fn tokens_from_sql(n: i64) -> u64 {
    u64::try_from(n).unwrap_or(0)
}
