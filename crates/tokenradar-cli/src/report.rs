//! `status`, `limits` and `sessions`: from the app when it runs, otherwise read-only from its
//! database and Claude Code's session files.

use std::path::PathBuf;
use std::time::Duration;

use tokenradar_core::domain::{LimitRef, Timestamp};
use tokenradar_core::ipc::{
    AbsentLimit, AccountInfo, Client, Hello, LimitInfo, Op, Origin, PROTOCOL_VERSION, Request,
    SessionInfo, StatusQuery, StatusReply, with_deadline,
};
use tokenradar_core::limits::history::{HISTORY_SCHEMA, HistoryReply, window_series};
use tokenradar_core::limits::{WEEK_MS, fusion, presence};
use tokenradar_core::live_sessions::running_sessions;
use tokenradar_core::statusline::default_config_dir;
use tokenradar_core::store::ReadOnlyStore;
use tokenradar_i18n::{Arg, Locale, Localizer};

use crate::context::Context;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    Status,
    Limits,
    Sessions,
}

pub fn run(ctx: &Context, report: Report, json: bool) -> Result<String, String> {
    let now = Timestamp::now();
    let status = from_app(ctx).unwrap_or_else(|| offline(ctx, now));
    if json {
        let value = match report {
            Report::Status => status_json(&status, ctx.l10n.locale()),
            Report::Limits => serde_json::to_string_pretty(&status.limits_reply()),
            Report::Sessions => serde_json::to_string_pretty(&status.sessions_reply()),
        };
        return value.map_err(|e| e.to_string());
    }
    Ok(human(&ctx.l10n, &status, report, now))
}

/// `status --json`: the app's reply plus the CLI's resolved `locale` (`en`, `tr`), so a
/// script or the Claude Code mod speaks the user's language. The field lives only here, not
/// in the IPC reply.
fn status_json(status: &StatusReply, locale: Locale) -> serde_json::Result<String> {
    let mut value = serde_json::to_value(status)?;
    if let Some(map) = value.as_object_mut() {
        map.insert("locale".to_owned(), locale.tag().into());
    }
    serde_json::to_string_pretty(&value)
}

/// The app's full status; `None` when it does not run or does not answer.
pub fn from_app(ctx: &Context) -> Option<StatusReply> {
    from_app_with(ctx, StatusQuery::default())
}

/// [`from_app`] with a query of the caller's, e.g. doctor's working folder (Step 12r).
pub fn from_app_with(ctx: &Context, query: StatusQuery) -> Option<StatusReply> {
    let name = ctx.pipe_name.as_deref()?;
    let mut client = Client::connect(name, CONNECT_TIMEOUT).ok()?;
    let requests = [
        Request::new(
            1,
            Op::Hello(Hello {
                client: format!("tokenradar-cli/{}", env!("CARGO_PKG_VERSION")),
                version: PROTOCOL_VERSION,
            }),
        ),
        Request::new(2, Op::StatusGet(query)),
    ];
    let replies = with_deadline(EXCHANGE_TIMEOUT, move || client.call(&requests)).ok()?;
    replies.into_iter().find(|r| r.id == 2)?.into_data().ok()
}

/// `history`: each limit's readings in its current window, read-only from the database even
/// while the app runs (WAL lets a reader in beside the writer), so the IPC stays as it is
/// (ADR-046). No database or no account is an empty `series`, not an error.
pub fn history(ctx: &Context) -> Result<String, String> {
    let now = Timestamp::now();
    let store = match ctx.paths.as_ref() {
        Some(p) => ReadOnlyStore::open(p.db_path()).map_err(|e| e.to_string())?,
        None => None,
    };
    let mut series = Vec::new();
    if let Some(store) = &store
        && let Some(account) = store
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .next()
    {
        let absent = store
            .kv_get(&presence::kv_key(account.id))
            .ok()
            .flatten()
            .map(|v| presence::decode(&v))
            .unwrap_or_default();
        let latest = store
            .latest_snapshots(account.id)
            .map_err(|e| e.to_string())?;
        // The longest window is a week: one read covers every series.
        let snapshots = store
            .snapshots_between(
                account.id,
                now.saturating_sub_millis(WEEK_MS),
                now.saturating_add_millis(1),
            )
            .map_err(|e| e.to_string())?;
        series = window_series(&latest, &snapshots, &absent, now);
    }
    let reply = HistoryReply {
        schema: HISTORY_SCHEMA,
        generated_at: now,
        series,
    };
    serde_json::to_string(&reply).map_err(|e| e.to_string())
}

/// What the app last stored, as it would show it now, plus the running sessions.
fn offline(ctx: &Context, now: Timestamp) -> StatusReply {
    let mut status = StatusReply::empty(now);
    status.origin = Origin::Database;
    let store = ctx
        .paths
        .as_ref()
        .and_then(|p| ReadOnlyStore::open(p.db_path()).ok().flatten());
    if let Some(store) = &store
        && let Some(account) = store.accounts().ok().and_then(|a| a.into_iter().next())
    {
        status.account = Some(AccountInfo {
            label: account.spec.label.clone(),
            plan: account.spec.plan.clone(),
            tier: account.spec.tier.clone(),
        });
        // The app's decision about limits this account does not have (ADR-041).
        let absent = store
            .kv_get(&presence::kv_key(account.id))
            .ok()
            .flatten()
            .map(|v| presence::decode(&v))
            .unwrap_or_default();
        // Each source's newest reading, combined the way the app does (ADR-060).
        status.limits = fusion::fuse_latest(
            &store
                .latest_snapshots_by_source(account.id)
                .unwrap_or_default(),
        )
        .iter()
        .filter(|o| !absent.contains(&LimitRef::of(o)))
        .map(|o| LimitInfo::from_stored(o, now))
        .collect();
        status.absent = absent.iter().map(AbsentLimit::from_ref).collect();
    }
    let mut dirs: Vec<PathBuf> = ctx
        .config
        .accounts
        .iter()
        .map(|a| a.config_dir.clone())
        .collect();
    if dirs.is_empty() {
        dirs.extend(default_config_dir());
    }
    for dir in dirs {
        for f in running_sessions(&dir) {
            status.live_sessions.push(SessionInfo::from_live_file(
                &f.session_uuid,
                f.name.as_deref(),
                &f.cwd,
                f.status,
            ));
        }
    }
    status
}

fn human(l: &Localizer, s: &StatusReply, report: Report, now: Timestamp) -> String {
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    let has_data = s.origin == Origin::App || s.account.is_some();
    if report == Report::Status {
        line(l.msg(match (s.origin, has_data) {
            (Origin::App, _) => "data-from-app",
            (Origin::Database, true) => "data-from-db",
            (Origin::Database, false) => "data-none",
        }));
        if let Some(a) = &s.account {
            let plan = [a.plan.as_deref(), a.tier.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
            line(if plan.is_empty() {
                a.label.clone()
            } else {
                format!("{} · {plan}", a.label)
            });
        }
        line(String::new());
    }
    if matches!(report, Report::Status | Report::Limits) {
        if s.limits.is_empty() {
            line(l.msg("no-limits"));
        } else {
            line(limits_table(l, &s.limits, now));
        }
        for a in &s.absent {
            let name = limit_name(l, &a.kind, a.scope.as_deref());
            line(l.fmt("limit-absent", &[("name", Arg::Str(&name))]));
        }
    }
    if report == Report::Status {
        line(String::new());
    }
    if matches!(report, Report::Status | Report::Sessions) {
        if s.live_sessions.is_empty() {
            line(l.msg("no-live-sessions"));
        } else {
            line(sessions_table(l, &s.live_sessions));
        }
    }
    if report == Report::Status && s.origin == Origin::App {
        line(String::new());
        line(match s.statusline_last_seen {
            Some(at) => l.fmt(
                "statusline-last-seen",
                &[("ms", ms(now.as_millis() - at.as_millis()))],
            ),
            None => l.msg("statusline-never-seen"),
        });
    }
    if s.limits.iter().any(|x| x.estimated) && report != Report::Sessions {
        line(l.msg("estimate-note"));
    }
    out.truncate(out.trim_end_matches('\n').len());
    out
}

#[allow(clippy::cast_precision_loss)] // durations are far below 2^52 ms
fn ms(ms: i64) -> Arg<'static> {
    Arg::Num(ms.max(0) as f64)
}

const NONE: &str = "—";

pub(crate) fn limit_name(l: &Localizer, kind: &str, scope: Option<&str>) -> String {
    match kind {
        "session" => l.msg("limit-session"),
        "weekly_all" => l.msg("limit-weekly-all"),
        "weekly_scoped" => l.fmt(
            "limit-weekly-scoped",
            &[("scope", Arg::Str(scope.unwrap_or(NONE)))],
        ),
        other => l.fmt("limit-other", &[("kind", Arg::Str(other))]),
    }
}

fn source_name(l: &Localizer, source: &str) -> String {
    match source {
        "statusline" => l.msg("source-statusline"),
        "api" => l.msg("source-api"),
        "estimate" => l.msg("source-estimate"),
        "codex_log" => l.msg("source-codex-log"),
        other => other.to_owned(),
    }
}

fn pct(l: &Localizer, value: f64, estimated: bool) -> String {
    let p = tokenradar_i18n::format::pct(l.locale(), value);
    if estimated { format!("≈{p}") } else { p }
}

fn limits_table(l: &Localizer, limits: &[LimitInfo], now: Timestamp) -> String {
    let head = [
        "col-limit",
        "col-used",
        "col-resets",
        "col-source",
        "col-projection",
    ]
    .map(|k| l.msg(k));
    let rows: Vec<Vec<String>> = limits
        .iter()
        .map(|x| {
            let resets = x.resets_at.map_or_else(
                || NONE.to_owned(),
                |r| {
                    tokenradar_i18n::format::dur(
                        l.locale(),
                        r.as_millis() - now.as_millis(),
                        tokenradar_i18n::DurStyle::Short,
                    )
                },
            );
            let projection = match (x.will_hit_before_reset, x.eta_100_at) {
                (true, Some(eta)) => l.fmt(
                    "projection-full-in",
                    &[("ms", ms(eta.as_millis() - now.as_millis()))],
                ),
                _ if x.projection_confidence.is_some() => l.msg("projection-reset-first"),
                _ => NONE.to_owned(),
            };
            vec![
                limit_name(l, &x.kind, x.scope.as_deref()),
                pct(l, x.percent, x.estimated),
                resets,
                source_name(l, &x.source),
                projection,
            ]
        })
        .collect();
    table(&head, &rows, &[false, true, true, false, false])
}

fn sessions_table(l: &Localizer, sessions: &[SessionInfo]) -> String {
    let head = [
        "col-session",
        "col-project",
        "col-status",
        "col-share",
        "col-cost",
    ]
    .map(|k| l.msg(k));
    let rows: Vec<Vec<String>> = sessions
        .iter()
        .map(|s| {
            let name = s
                .name
                .clone()
                .or_else(|| s.title.clone())
                .or_else(|| s.session_uuid.as_ref().map(|u| u.chars().take(8).collect()))
                .unwrap_or_else(|| NONE.to_owned());
            let status = match s.status.as_deref() {
                Some("busy") => l.msg("status-busy"),
                Some("idle") => l.msg("status-idle"),
                _ => l.msg("status-unknown"),
            };
            vec![
                name,
                s.project.clone().unwrap_or_else(|| NONE.to_owned()),
                status,
                s.share_percent
                    .map_or_else(|| NONE.to_owned(), |p| pct(l, p, true)),
                format!(
                    "${}",
                    tokenradar_i18n::format::num(l.locale(), s.cost_usd, 2)
                ),
            ]
        })
        .collect();
    table(&head, &rows, &[false, false, false, true, true])
}

/// Columns padded to their widest cell; `right` aligns numbers.
fn table<S: AsRef<str>>(head: &[S], rows: &[Vec<String>], right: &[bool]) -> String {
    let mut widths: Vec<usize> = head.iter().map(|h| h.as_ref().chars().count()).collect();
    for row in rows {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.chars().count());
        }
    }
    let mut out = String::new();
    let mut put = |cells: &mut dyn Iterator<Item = &str>| {
        let mut line = String::new();
        for (i, cell) in cells.enumerate() {
            if i > 0 {
                line.push_str("  ");
            }
            let pad = widths[i].saturating_sub(cell.chars().count());
            if right.get(i).copied().unwrap_or(false) {
                line.push_str(&" ".repeat(pad));
                line.push_str(cell);
            } else {
                line.push_str(cell);
                line.push_str(&" ".repeat(pad));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    };
    put(&mut head.iter().map(AsRef::as_ref));
    for row in rows {
        put(&mut row.iter().map(String::as_str));
    }
    out.pop();
    out
}

#[cfg(test)]
mod tests {
    use tokenradar_i18n::{Domain, Locale};

    use super::*;

    fn reply(now: Timestamp) -> StatusReply {
        let mut s = StatusReply::empty(now);
        s.account = Some(AccountInfo {
            label: "Claude".into(),
            plan: Some("max".into()),
            tier: None,
        });
        s.limits = vec![
            LimitInfo {
                kind: "session".into(),
                scope: None,
                percent: 72.0,
                resets_at: Some(Timestamp::from_millis(now.as_millis() + 108 * 60_000)),
                source: "statusline".into(),
                estimated: false,
                observed_at: now,
                freshness: "fresh".into(),
                eta_100_at: Some(Timestamp::from_millis(now.as_millis() + 38 * 60_000)),
                will_hit_before_reset: true,
                projection_confidence: Some("medium".into()),
            },
            LimitInfo {
                kind: "weekly_all".into(),
                scope: None,
                percent: 34.5,
                resets_at: None,
                source: "estimate".into(),
                estimated: true,
                observed_at: now,
                freshness: "stale".into(),
                eta_100_at: None,
                will_hit_before_reset: false,
                projection_confidence: None,
            },
        ];
        s.live_sessions = vec![SessionInfo {
            session_uuid: Some("0a1f0000-0000-4000-8000-00000000a001".into()),
            name: None,
            title: Some("Fix the parser".into()),
            project: Some("Alpha".into()),
            status: Some("busy".into()),
            share_percent: Some(9.0),
            cost_usd: 1.5,
            tokens_per_min: 0.0,
            compacts: None,
        }];
        s.statusline_last_seen = Some(Timestamp::from_millis(now.as_millis() - 120_000));
        s
    }

    #[test]
    fn status_in_turkish() {
        let now = Timestamp::from_millis(1_793_059_200_000);
        let l = Localizer::new(Locale::Tr, &[Domain::Cli]);
        let out = human(&l, &reply(now), Report::Status, now);
        let expected = "\
TokenRadar uygulamasından canlı veri.
Claude · max

Limit      Kullanım  Sıfırlanma  Kaynak      Bu hızla
5 saatlik       %72  1 sa 48 dk  statusline  38 dk içinde dolar
Haftalık     ≈%34,5           —  tahmin      —

Oturum          Proje  Durum   Pencere payı  Maliyet
Fix the parser  Alpha  meşgul           ≈%9    $1,50

Statusline köprüsü en son 2 dk önce görüldü.
≈ işareti ölçümü değil tahmini gösterir.";
        assert_eq!(out, expected, "\n{out}");
    }

    #[test]
    fn limits_in_english() {
        let now = Timestamp::from_millis(1_793_059_200_000);
        let l = Localizer::new(Locale::En, &[Domain::Cli]);
        let out = human(&l, &reply(now), Report::Limits, now);
        let expected = "\
Limit     Used  Resets in  Source      At this rate
5-hour     72%     1h 48m  statusline  full in 38m
Weekly  ≈34.5%          —  estimate    —
≈ marks estimates, not measurements.";
        assert_eq!(out, expected, "\n{out}");
    }

    #[test]
    fn an_absent_window_is_named_not_shown() {
        let now = Timestamp::from_millis(1_793_059_200_000);
        let mut s = reply(now);
        s.limits.remove(0);
        s.absent = vec![AbsentLimit {
            kind: "session".into(),
            scope: None,
        }];
        let tr = human(
            &Localizer::new(Locale::Tr, &[Domain::Cli]),
            &s,
            Report::Limits,
            now,
        );
        assert!(
            !tr.contains("%72"),
            "
{tr}"
        );
        assert!(
            tr.contains("5 saatlik: bu hesapta yok."),
            "
{tr}"
        );
        let en = human(
            &Localizer::new(Locale::En, &[Domain::Cli]),
            &s,
            Report::Limits,
            now,
        );
        assert!(
            en.contains("5-hour: not on this account."),
            "
{en}"
        );
    }

    #[test]
    fn status_json_carries_the_locale() {
        let now = Timestamp::from_millis(1_793_059_200_000);
        let text = status_json(&reply(now), Locale::Tr).expect("json");
        let v: serde_json::Value = serde_json::from_str(&text).expect("parse");
        assert_eq!(v["locale"], "tr");
        assert_eq!(v["schema"], 1);
        assert_eq!(v["limits"][0]["kind"], "session");
    }

    #[test]
    fn json_has_a_schema_version() {
        let now = Timestamp::from_millis(1_793_059_200_000);
        let v = serde_json::to_value(reply(now).limits_reply()).expect("json");
        assert_eq!(v["schema"], 1);
        assert_eq!(v["limits"][0]["kind"], "session");
    }
}
