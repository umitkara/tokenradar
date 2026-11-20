//! Export of sessions, windows and daily totals as CSV or JSON (Step 17a, ADR-070).
//!
//! - **Metadata and usage only:** names, projects, times, models, tokens, cost and shares.
//!   Project paths are included unless the caller hides them.
//! - **CSV for Excel:** UTF-8 with a byte-order mark, `;` and a decimal comma in Turkish
//!   (where `,` is the decimal separator), `,` and a decimal point in English. Headers are in
//!   the chosen language; times are local `YYYY-MM-DD HH:MM`. A text cell that starts like a
//!   formula gets a leading `'`, so a session title cannot run as one in a spreadsheet.
//! - **JSON for programs:** English keys, ISO 8601 UTC times, plain numbers.

use std::collections::{HashMap, HashSet};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use specta::Type;
use tokenradar_i18n::{Domain, Locale, Localizer};

use crate::domain::{LimitKind, MicroUsd, Timestamp};
use crate::queries::{
    self, QueryContext, SessionFilter, SessionSort, TimeRange, WindowFilter, WindowRow,
};
use crate::store::{Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Json,
}

impl ExportFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

/// What to export: the list a page shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportRequest {
    Sessions { filter: SessionFilter },
    Windows { range: Option<TimeRange> },
    Daily { range: TimeRange },
}

impl ExportRequest {
    /// A file name stem: `tokenradar-sessions`.
    pub const fn stem(&self) -> &'static str {
        match self {
            Self::Sessions { .. } => "tokenradar-sessions",
            Self::Windows { .. } => "tokenradar-windows",
            Self::Daily { .. } => "tokenradar-daily",
        }
    }
}

/// `tokenradar-sessions-2026-11-21.csv`, dated in local time.
pub fn file_name(
    request: &ExportRequest,
    format: ExportFormat,
    now: Timestamp,
    zone: &dyn queries::LocalZone,
) -> String {
    let ms = now.as_millis() + zone.offset_ms(now);
    let date = jiff::Timestamp::from_millisecond(ms)
        .map(|t| t.to_zoned(jiff::tz::TimeZone::UTC).date().to_string())
        .unwrap_or_default();
    format!("{}-{date}.{}", request.stem(), format.extension())
}

/// Rows a session export reads per page.
const PAGE: u32 = queries::MAX_PAGE;

/// The file's bytes.
pub fn export(
    store: &Store,
    ctx: &QueryContext<'_>,
    request: &ExportRequest,
    format: ExportFormat,
    hide_paths: bool,
    locale: Locale,
) -> Result<Vec<u8>, StoreError> {
    let table = match request {
        ExportRequest::Sessions { filter } => sessions(store, ctx, filter, hide_paths)?,
        ExportRequest::Windows { range } => windows(store, ctx, *range)?,
        ExportRequest::Daily { range } => daily(store, ctx, *range)?,
    };
    Ok(match format {
        ExportFormat::Csv => table.csv(ctx, locale),
        ExportFormat::Json => table.json(ctx),
    })
}

/// A typed cell: the CSV and JSON writers format it each in their own way.
#[derive(Debug, Clone, PartialEq)]
enum Cell {
    Text(String),
    Int(i64),
    Num(f64, usize),
    Usd(MicroUsd),
    Time(Timestamp),
    /// Local calendar day, given as its local midnight.
    Day(Timestamp),
    Bool(bool),
    Empty,
}

struct Table {
    /// `(json key, header message id)` per column.
    columns: Vec<(&'static str, &'static str)>,
    rows: Vec<Vec<Cell>>,
}

impl Table {
    fn csv(&self, ctx: &QueryContext<'_>, locale: Locale) -> Vec<u8> {
        let l10n = Localizer::new(locale, &[Domain::Export]);
        let sep = match locale {
            Locale::Tr => ';',
            Locale::En => ',',
        };
        let mut out = String::from('\u{feff}');
        let headers: Vec<String> = self.columns.iter().map(|(_, id)| l10n.msg(id)).collect();
        push_line(&mut out, headers.iter().map(String::as_str), sep);
        for row in &self.rows {
            let cells: Vec<String> = row
                .iter()
                .map(|c| csv_cell(c, ctx, locale, &l10n))
                .collect();
            push_line(&mut out, cells.iter().map(String::as_str), sep);
        }
        out.into_bytes()
    }

    fn json(&self, ctx: &QueryContext<'_>) -> Vec<u8> {
        let rows: Vec<Value> = self
            .rows
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                for ((key, _), cell) in self.columns.iter().zip(row) {
                    obj.insert((*key).to_owned(), json_cell(cell, ctx));
                }
                Value::Object(obj)
            })
            .collect();
        let mut bytes = serde_json::to_vec_pretty(&rows).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }
}

fn push_line<'a>(out: &mut String, cells: impl Iterator<Item = &'a str>, sep: char) {
    for (i, cell) in cells.enumerate() {
        if i > 0 {
            out.push(sep);
        }
        if cell.contains([sep, '"', '\n', '\r']) {
            out.push('"');
            out.push_str(&cell.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(cell);
        }
    }
    out.push_str("\r\n");
}

fn local(ctx: &QueryContext<'_>, t: Timestamp) -> Option<jiff::civil::DateTime> {
    let ms = t.as_millis() + ctx.zone.offset_ms(t);
    jiff::Timestamp::from_millisecond(ms)
        .ok()
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC).datetime())
}

fn number(locale: Locale, n: f64, decimals: usize) -> String {
    let s = format!("{n:.decimals$}");
    match locale {
        // No thousands separator: Excel reads `1234,5` in Turkish and `1234.5` in English.
        Locale::Tr => s.replace('.', ","),
        Locale::En => s,
    }
}

fn csv_cell(cell: &Cell, ctx: &QueryContext<'_>, locale: Locale, l10n: &Localizer) -> String {
    match cell {
        Cell::Text(s) => {
            if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
                format!("'{s}")
            } else {
                s.clone()
            }
        }
        Cell::Int(n) => n.to_string(),
        Cell::Num(n, d) => number(locale, *n, *d),
        Cell::Usd(m) => number(locale, usd(*m), 2),
        Cell::Time(t) => {
            local(ctx, *t).map_or_else(String::new, |d| d.strftime("%Y-%m-%d %H:%M").to_string())
        }
        Cell::Day(t) => {
            local(ctx, *t).map_or_else(String::new, |d| d.strftime("%Y-%m-%d").to_string())
        }
        Cell::Bool(b) => l10n.msg(if *b { "export-yes" } else { "export-no" }),
        Cell::Empty => String::new(),
    }
}

#[allow(clippy::cast_precision_loss)]
fn usd(m: MicroUsd) -> f64 {
    m.0 as f64 / 1_000_000.0
}

fn json_cell(cell: &Cell, ctx: &QueryContext<'_>) -> Value {
    match cell {
        Cell::Text(s) => json!(s),
        Cell::Int(n) => json!(n),
        Cell::Num(n, d) => {
            let p = 10f64.powi(i32::try_from(*d).unwrap_or(2));
            json!((n * p).round() / p)
        }
        Cell::Usd(m) => json!((usd(*m) * 1e6).round() / 1e6),
        Cell::Time(t) => jiff::Timestamp::from_millisecond(t.as_millis())
            .map_or(Value::Null, |ts| json!(ts.to_string())),
        // A day stays a local calendar date.
        Cell::Day(t) => local(ctx, *t).map_or(Value::Null, |d| json!(d.date().to_string())),
        Cell::Bool(b) => json!(b),
        Cell::Empty => Value::Null,
    }
}

fn opt_num(n: Option<f64>, d: usize) -> Cell {
    n.map_or(Cell::Empty, |v| Cell::Num(v, d))
}

fn opt_time(t: Option<Timestamp>) -> Cell {
    t.map_or(Cell::Empty, Cell::Time)
}

fn sessions(
    store: &Store,
    ctx: &QueryContext<'_>,
    filter: &SessionFilter,
    hide_paths: bool,
) -> Result<Table, StoreError> {
    let paths: HashMap<i64, String> = if hide_paths {
        HashMap::new()
    } else {
        store.read(|c| {
            Ok(
                c.prepare_cached("SELECT id, COALESCE(display_path, path_norm) FROM projects")?
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<rusqlite::Result<_>>()?,
            )
        })?
    };
    let mut columns = vec![
        ("uuid", "export-col-uuid"),
        ("name", "export-col-name"),
        ("project", "export-col-project"),
    ];
    if !hide_paths {
        columns.push(("project_path", "export-col-project-path"));
    }
    columns.extend([
        ("started", "export-col-started"),
        ("last_active", "export-col-last-active"),
        ("models", "export-col-models"),
        ("cost_usd", "export-col-cost"),
        ("input_tokens", "export-col-input"),
        ("output_tokens", "export-col-output"),
        ("cache_read_tokens", "export-col-cache-read"),
        ("cache_write_tokens", "export-col-cache-write"),
        ("weekly_percent_estimate", "export-col-weekly-share"),
        ("five_hour_percent", "export-col-session-share"),
        ("subagents", "export-col-subagents"),
    ]);
    let mut rows = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = queries::list_sessions(
            store,
            ctx,
            filter,
            SessionSort::Started,
            cursor.as_deref(),
            Some(PAGE),
            &[],
        )?;
        for s in page.items {
            let mut row = vec![
                Cell::Text(s.uuid),
                Cell::Text(s.name.text),
                s.project
                    .as_ref()
                    .map_or(Cell::Empty, |p| Cell::Text(p.name.clone())),
            ];
            if !hide_paths {
                row.push(
                    s.project
                        .as_ref()
                        .and_then(|p| paths.get(&p.id.0))
                        .map_or(Cell::Empty, |p| Cell::Text(p.clone())),
                );
            }
            row.extend([
                opt_time(s.first_ts),
                opt_time(s.last_ts),
                Cell::Text(s.models.join(" ")),
                Cell::Usd(s.cost),
                Cell::Int(s.tokens.input),
                Cell::Int(s.tokens.output),
                Cell::Int(s.tokens.cache_read),
                Cell::Int(s.tokens.cache_write),
                opt_num(s.weekly_percent, 2),
                opt_num(s.session_percent, 2),
                Cell::Int(i64::from(s.subagents)),
            ]);
            rows.push(row);
        }
        match page.next {
            Some(n) => cursor = Some(n),
            None => break,
        }
    }
    Ok(Table { columns, rows })
}

fn windows(
    store: &Store,
    ctx: &QueryContext<'_>,
    range: Option<TimeRange>,
) -> Result<Table, StoreError> {
    let columns = vec![
        ("limit", "export-col-limit"),
        ("scope", "export-col-scope"),
        ("starts_at", "export-col-starts"),
        ("resets_at", "export-col-resets"),
        ("peak_percent", "export-col-peak"),
        ("hit_limit", "export-col-hit"),
        ("estimated", "export-col-estimated"),
        ("sessions", "export-col-sessions"),
        ("cost_usd", "export-col-cost"),
        ("other_percent", "export-col-other"),
        ("method", "export-col-method"),
    ];
    let filter = WindowFilter { range, limit: None };
    let mut rows = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = queries::list_windows(store, ctx, &filter, cursor.as_deref(), Some(PAGE))?;
        for w in page.items {
            rows.push(window_cells(w));
        }
        match page.next {
            Some(n) => cursor = Some(n),
            None => break,
        }
    }
    Ok(Table { columns, rows })
}

fn window_cells(w: WindowRow) -> Vec<Cell> {
    let limit = match w.kind {
        LimitKind::Session => "five_hour".to_owned(),
        other => other.as_str().to_owned(),
    };
    vec![
        Cell::Text(limit),
        w.scope.map_or(Cell::Empty, |s| Cell::Text(s.to_string())),
        Cell::Time(w.starts_at),
        Cell::Time(w.resets_at),
        opt_num(w.peak_percent, 1),
        w.hit_limit.map_or(Cell::Empty, Cell::Bool),
        Cell::Bool(w.estimated),
        Cell::Int(i64::from(w.sessions)),
        Cell::Usd(w.cost),
        opt_num(w.other_percent, 2),
        w.basis
            .map_or(Cell::Empty, |b| Cell::Text(b.method.as_str().to_owned())),
    ]
}

#[derive(Default)]
struct DaySums {
    roots: HashSet<i64>,
    cost: i64,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
}

fn daily(store: &Store, ctx: &QueryContext<'_>, range: TimeRange) -> Result<Table, StoreError> {
    const DAY_MS: i64 = 86_400_000;
    let columns = vec![
        ("date", "export-col-date"),
        ("sessions", "export-col-sessions"),
        ("cost_usd", "export-col-cost"),
        ("input_tokens", "export-col-input"),
        ("output_tokens", "export-col-output"),
        ("cache_read_tokens", "export-col-cache-read"),
        ("cache_write_tokens", "export-col-cache-write"),
    ];
    let to = range.to.min(ctx.now);
    let from = store
        .earliest_bucket(ctx.account)?
        .map_or(to, |e| e.max(range.from));
    // Local day number → sums, and the day's local midnight.
    let mut days: HashMap<i64, (Timestamp, DaySums)> = HashMap::new();
    for (a, b, offset) in queries::local_spans(ctx.zone, from.as_millis(), to.as_millis()) {
        let col = if b - a > 60 * DAY_MS {
            "+u.bucket_ts"
        } else {
            "u.bucket_ts"
        };
        let sql = format!(
            "SELECT u.bucket_ts, COALESCE(s.root_id, s.id), u.cost_micro_usd, u.input, u.output,
                    u.cache_read, u.cache_w5m + u.cache_w1h
             FROM usage_5m u JOIN sessions s ON s.id = u.session_id
             WHERE u.account_id = ?1 AND {col} >= ?2 AND {col} < ?3"
        );
        store.read(|c| {
            let mut stmt = c.prepare_cached(&sql)?;
            let mut q = stmt.query(params![ctx.account.0, a, b])?;
            while let Some(r) = q.next()? {
                let t: i64 = r.get(0)?;
                let day = (t + offset).div_euclid(DAY_MS);
                let e = days.entry(day).or_insert_with(|| {
                    (
                        queries::local_day_start(ctx.zone, Timestamp::from_millis(t)),
                        DaySums::default(),
                    )
                });
                e.1.roots.insert(r.get(1)?);
                e.1.cost += r.get::<_, i64>(2)?;
                e.1.input += r.get::<_, i64>(3)?;
                e.1.output += r.get::<_, i64>(4)?;
                e.1.cache_read += r.get::<_, i64>(5)?;
                e.1.cache_write += r.get::<_, i64>(6)?;
            }
            Ok(())
        })?;
    }
    let mut keys: Vec<i64> = days.keys().copied().collect();
    keys.sort_unstable();
    let rows = keys
        .into_iter()
        .filter_map(|k| days.remove(&k))
        .map(|(midnight, d)| {
            vec![
                Cell::Day(midnight),
                Cell::Int(i64::try_from(d.roots.len()).unwrap_or(i64::MAX)),
                Cell::Usd(MicroUsd(d.cost)),
                Cell::Int(d.input),
                Cell::Int(d.output),
                Cell::Int(d.cache_read),
                Cell::Int(d.cache_write),
            ]
        })
        .collect();
    Ok(Table { columns, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_lines_quote_separators_and_quotes() {
        let mut out = String::new();
        push_line(&mut out, ["a;b", "say \"hi\"", "plain"].into_iter(), ';');
        assert_eq!(out, "\"a;b\";\"say \"\"hi\"\"\";plain\r\n");
        let mut out = String::new();
        push_line(&mut out, ["1,5", "x"].into_iter(), ';');
        assert_eq!(out, "1,5;x\r\n", "a comma is no separator in Turkish files");
    }

    #[test]
    fn numbers_follow_the_locale() {
        assert_eq!(number(Locale::Tr, 1234.5, 2), "1234,50");
        assert_eq!(number(Locale::En, 1234.5, 2), "1234.50");
    }
}
