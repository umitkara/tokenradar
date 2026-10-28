//! `tokenradar statusline`: Claude Code's statusline command.
//!
//! Claude Code cancels a running statusline command when the next update arrives, so the
//! reading is pushed to the app first and the line is formatted after. Without the app the
//! reading goes to the spool and the line comes from stdin alone. Nothing here may fail
//! loudly: a broken statusline is worse than a sparse one.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

use tokenradar_core::config::{AppConfig, ChainPosition};
use tokenradar_core::domain::Timestamp;
use tokenradar_core::ipc::{
    Client, Hello, MAX_MESSAGE_BYTES, Op, PROTOCOL_VERSION, Request, StatusQuery, StatusReply,
    with_deadline,
};
use tokenradar_core::statusline::shell::Shell;
use tokenradar_core::statusline::{
    StatuslinePayload, StatuslinePush, default_config_dir, path_in_dir, spool,
};
use tokenradar_i18n::Localizer;

use crate::render::{self, Line, Style};

/// Claude Code writes stdin at once; anything slower is not a statusline call.
const STDIN_TIMEOUT: Duration = Duration::from_millis(200);
/// Opening the pipe while every instance is busy.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(50);
/// The whole exchange with the app.
const EXCHANGE_TIMEOUT: Duration = Duration::from_millis(200);
/// The previous statusline command in chain mode.
pub const CHAIN_TIMEOUT: Duration = Duration::from_millis(300);

/// What `respond` needs from the outside world; tests point it at temporary places.
pub struct Deps<'a> {
    pub l10n: &'a Localizer,
    pub pipe_name: Option<String>,
    pub spool_dir: Option<PathBuf>,
    /// The previous command and where its output goes, in chain mode.
    pub chain: Option<(String, ChainPosition)>,
    /// How long the chained command may take; [`CHAIN_TIMEOUT`] outside tests.
    pub chain_timeout: Duration,
    pub style: Style,
}

/// The lines to print for `input` (Claude Code's stdin).
pub fn respond(input: Option<&[u8]>, deps: &Deps<'_>) -> String {
    let observed_at = Timestamp::now();
    let payload = input.and_then(|b| StatuslinePayload::parse(b).ok());
    // The previous command runs while we talk to the app.
    let chained = deps
        .chain
        .as_ref()
        .and_then(|(cmd, pos)| Chained::spawn(cmd, input.unwrap_or_default()).map(|c| (c, *pos)));
    let reply = payload.clone().and_then(|payload| {
        deliver(
            StatuslinePush {
                observed_at,
                payload,
            },
            deps,
        )
    });
    let line = Line::from_sources(payload.as_ref(), reply.as_ref(), Timestamp::now());
    let ours = render::render(deps.l10n, &line, deps.style);
    let Some((chained, position)) = chained else {
        return ours;
    };
    match chained.finish(deps.chain_timeout) {
        Some(theirs) if !theirs.is_empty() => match position {
            ChainPosition::Before => format!("{theirs}\n{ours}"),
            ChainPosition::After => format!("{ours}\n{theirs}"),
        },
        _ => ours,
    }
}

/// Pushes the reading and asks for the enrichment. Without the app, spools it.
fn deliver(push: StatuslinePush, deps: &Deps<'_>) -> Option<StatusReply> {
    let client = deps
        .pipe_name
        .as_deref()
        .and_then(|name| Client::connect(name, CONNECT_TIMEOUT).ok());
    let Some(mut client) = client else {
        // Not running, or no pipe at all: keep the reading for later.
        if let Some(dir) = &deps.spool_dir {
            let _ = spool::append(dir, &push);
        }
        return None;
    };
    let session_uuid = push.payload.session_id.clone();
    let requests = [
        Request::new(
            1,
            Op::Hello(Hello {
                client: format!("tokenradar-cli/{}", env!("CARGO_PKG_VERSION")),
                version: PROTOCOL_VERSION,
            }),
        ),
        Request::new(2, Op::StatuslinePush(Box::new(push))),
        Request::new(3, Op::StatusGet(StatusQuery { session_uuid })),
    ];
    // The push is written first and in one piece; a slow answer only costs the enrichment.
    let replies = with_deadline(EXCHANGE_TIMEOUT, move || client.call(&requests)).ok()?;
    replies
        .into_iter()
        .find(|r| r.id == 3)?
        .into_data::<StatusReply>()
        .ok()
}

/// The previous statusline command, running with the same stdin.
struct Chained {
    child: std::process::Child,
    output: mpsc::Receiver<Vec<u8>>,
}

impl Chained {
    fn spawn(command: &str, input: &[u8]) -> Option<Self> {
        let mut child = Shell::detect()
            .command(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut stdin = child.stdin.take()?;
        let input = input.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
        let mut stdout = child.stdout.take()?;
        let (tx, output) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = (&mut stdout)
                .take(u64::try_from(MAX_MESSAGE_BYTES).unwrap_or(u64::MAX))
                .read_to_end(&mut buf);
            let _ = tx.send(buf);
        });
        Some(Self { child, output })
    }

    /// Its output without trailing newlines, or `None` if it took longer than `timeout`
    /// (then it is killed).
    fn finish(mut self, timeout: Duration) -> Option<String> {
        if let Ok(out) = self.output.recv_timeout(timeout) {
            let _ = self.child.wait();
            let text = String::from_utf8_lossy(&out);
            Some(text.trim_end_matches(['\r', '\n']).to_owned())
        } else {
            let _ = self.child.kill();
            let _ = self.child.wait();
            None
        }
    }
}

/// Reads stdin up to [`MAX_MESSAGE_BYTES`], waiting at most [`STDIN_TIMEOUT`].
pub fn read_stdin() -> Option<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let limit = u64::try_from(MAX_MESSAGE_BYTES + 1).unwrap_or(u64::MAX);
        let ok = std::io::stdin().take(limit).read_to_end(&mut buf).is_ok();
        let _ = tx.send(ok.then_some(buf));
    });
    rx.recv_timeout(STDIN_TIMEOUT)
        .ok()
        .flatten()
        .filter(|b| b.len() <= MAX_MESSAGE_BYTES)
}

/// The previous command to chain: from the account whose directory holds the transcript,
/// else Claude Code's default directory.
pub fn chain_from_config(
    config: &AppConfig,
    input: Option<&[u8]>,
) -> Option<(String, ChainPosition)> {
    let transcript = input
        .and_then(|b| StatuslinePayload::parse(b).ok())
        .and_then(|p| p.transcript_path);
    let account = transcript
        .as_deref()
        .and_then(|t| {
            config
                .accounts
                .iter()
                .find(|a| path_in_dir(t, &a.config_dir))
        })
        .or_else(|| default_config_dir().and_then(|d| config.account(&d)))?;
    let s = &account.statusline;
    s.previous_command
        .clone()
        .filter(|c| s.chain && !c.trim().is_empty())
        .map(|c| (c, s.chain_position))
}

#[cfg(test)]
mod tests;
