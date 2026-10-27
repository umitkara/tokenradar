//! The app's IPC server: a named pipe on a small single-threaded runtime of its own (ADR-029).
//!
//! - Every pipe instance carries a DACL for the current user only and rejects remote clients.
//! - The first instance is created with `FILE_FLAG_FIRST_PIPE_INSTANCE`, so the server fails
//!   instead of sharing a name another process created first.
//! - Clients are served concurrently, at most [`MAX_CLIENTS`] at once; more are dropped.
//! - A message must arrive within [`READ_TIMEOUT`] and fit in [`MAX_MESSAGE_BYTES`]. A broken
//!   message closes its connection only.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions};
use tokio::sync::{Semaphore, oneshot};

use super::proto::{self, DecodeError, Handler, MAX_MESSAGE_BYTES, Request, Response};
use super::security::{SecurityAttributes, current_user_sid};

pub const MAX_CLIENTS: usize = 16;
pub const READ_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Full pipe path, `\\.\pipe\…`.
    pub name: String,
    /// Reported in `hello`, e.g. `tokenradar-app/0.1.0`.
    pub server_id: String,
    pub read_timeout: Duration,
    pub max_clients: usize,
}

impl ServerConfig {
    pub fn new(name: String, server_id: String) -> Self {
        Self {
            name,
            server_id,
            read_timeout: READ_TIMEOUT,
            max_clients: MAX_CLIENTS,
        }
    }
}

pub type StopSender = oneshot::Sender<()>;
pub type StopReceiver = oneshot::Receiver<()>;

/// The channel [`run`] stops on; dropping the sender stops it too. Callers need no tokio.
pub fn stop_channel() -> (StopSender, StopReceiver) {
    oneshot::channel()
}

/// Serves until `stop` fires or its sender is dropped. Blocks the calling thread. `ready`
/// learns whether the pipe could be created.
pub fn run(
    config: &ServerConfig,
    handler: Arc<dyn Handler>,
    stop: oneshot::Receiver<()>,
    ready: impl FnOnce(io::Result<()>),
) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return ready(Err(e)),
    };
    rt.block_on(serve(config, handler, stop, ready));
}

async fn serve(
    config: &ServerConfig,
    handler: Arc<dyn Handler>,
    mut stop: oneshot::Receiver<()>,
    ready: impl FnOnce(io::Result<()>),
) {
    let attrs = match current_user_sid().and_then(|sid| SecurityAttributes::only_user(&sid)) {
        Ok(a) => a,
        Err(e) => return ready(Err(e)),
    };
    let mut server = match create(&config.name, &attrs, true) {
        Ok(s) => s,
        Err(e) => return ready(Err(e)),
    };
    ready(Ok(()));
    let slots = Arc::new(Semaphore::new(config.max_clients));
    loop {
        tokio::select! {
            _ = &mut stop => break,
            res = server.connect() => {
                // Hand this instance to the client and listen on a fresh one.
                let next = match create(&config.name, &attrs, false) {
                    Ok(s) => s,
                    Err(error) => {
                        tracing::warn!(%error, "could not create an IPC pipe instance");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                let conn = std::mem::replace(&mut server, next);
                if let Err(error) = res {
                    tracing::debug!(%error, "IPC client left before it was served");
                    continue;
                }
                let Ok(permit) = slots.clone().try_acquire_owned() else {
                    tracing::debug!("too many IPC clients; dropping one");
                    continue;
                };
                let handler = handler.clone();
                let config = config.clone();
                tokio::spawn(async move {
                    serve_client(conn, handler.as_ref(), &config).await;
                    drop(permit);
                });
            }
        }
    }
}

#[allow(unsafe_code)]
fn create(name: &str, attrs: &SecurityAttributes, first: bool) -> io::Result<NamedPipeServer> {
    let mut opts = ServerOptions::new();
    opts.first_pipe_instance(first)
        .reject_remote_clients(true)
        .pipe_mode(PipeMode::Byte);
    // SAFETY: `attrs` points to a valid SECURITY_ATTRIBUTES that outlives the call.
    unsafe { opts.create_with_security_attributes_raw(name, attrs.as_ptr()) }
}

async fn serve_client(conn: NamedPipeServer, handler: &dyn Handler, config: &ServerConfig) {
    let (read, mut write) = tokio::io::split(conn);
    let mut reader = BufReader::new(read);
    let limit = u64::try_from(MAX_MESSAGE_BYTES + 1).unwrap_or(u64::MAX);
    loop {
        let mut line = Vec::new();
        let mut limited = (&mut reader).take(limit);
        let read = limited.read_until(b'\n', &mut line);
        match tokio::time::timeout(config.read_timeout, read).await {
            Ok(Ok(0)) => return,
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                tracing::debug!(%error, "IPC read failed");
                return;
            }
            Err(_) => {
                tracing::debug!("IPC client idle too long");
                return;
            }
        }
        if line.len() > MAX_MESSAGE_BYTES {
            tracing::debug!("IPC message too large");
            return;
        }
        let reply = match Request::decode(&line) {
            Ok(req) => proto::handle(handler, req, &config.server_id),
            Err(DecodeError::Refused { id, error }) => Some(Response::error(id, error)),
            Err(DecodeError::Malformed) => {
                tracing::debug!("malformed IPC message");
                return;
            }
        };
        if let Some(reply) = reply
            && write.write_all(&reply.encode()).await.is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
