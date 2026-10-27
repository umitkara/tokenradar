//! Blocking client for the CLI: no async runtime, a named pipe opened like a file.
//!
//! Reads on a pipe cannot time out by themselves, so [`with_deadline`] runs an exchange on a
//! helper thread and gives up on it when time is up; the CLI exits right after, taking the
//! thread with it.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::proto::{MAX_MESSAGE_BYTES, Request, Response};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// No pipe: the app is not running.
    #[error("the TokenRadar app is not running")]
    NotRunning,
    #[error("the TokenRadar app did not answer in time")]
    Timeout,
    #[error("IPC failed: {0}")]
    Io(#[from] io::Error),
    #[error("the TokenRadar app sent an invalid reply")]
    BadReply,
}

/// A connection to the app.
pub struct Client {
    reader: BufReader<File>,
}

impl Client {
    /// Opens the pipe, waiting up to `timeout` while every instance is busy.
    pub fn connect(name: &str, timeout: Duration) -> Result<Self, ClientError> {
        let file = open_pipe(name, timeout)?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }

    /// Sends all `requests` in one write, then reads one response per request that expects
    /// one, in order.
    pub fn call(&mut self, requests: &[Request]) -> Result<Vec<Response>, ClientError> {
        let mut out = Vec::new();
        for r in requests {
            out.extend_from_slice(&r.encode());
        }
        self.reader.get_mut().write_all(&out)?;
        let expected = requests.iter().filter(|r| r.op.expects_reply()).count();
        let mut replies = Vec::with_capacity(expected);
        for _ in 0..expected {
            replies.push(self.recv()?);
        }
        Ok(replies)
    }

    fn recv(&mut self) -> Result<Response, ClientError> {
        let mut line = Vec::new();
        let limit = u64::try_from(MAX_MESSAGE_BYTES + 1).unwrap_or(u64::MAX);
        (&mut self.reader)
            .take(limit)
            .read_until(b'\n', &mut line)?;
        if line.last() != Some(&b'\n') {
            // Closed early or over the size limit.
            return Err(ClientError::BadReply);
        }
        Response::decode(&line).ok_or(ClientError::BadReply)
    }
}

/// Runs `f` on a helper thread; `Err(Timeout)` if it does not finish by `timeout`.
pub fn with_deadline<T, F>(timeout: Duration, f: F) -> Result<T, ClientError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ClientError> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("ipc-client".into())
        .spawn(move || {
            let _ = tx.send(f());
        })?;
    match rx.recv_timeout(timeout) {
        Ok(r) => r,
        Err(_) => Err(ClientError::Timeout),
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn open_pipe(name: &str, timeout: Duration) -> Result<File, ClientError> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY};
    use windows_sys::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION;
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;

    let deadline = Instant::now() + timeout;
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    loop {
        // SECURITY_IDENTIFICATION: a server squatting on the name cannot act as this user.
        match OpenOptions::new()
            .read(true)
            .write(true)
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(name)
        {
            Ok(f) => return Ok(f),
            Err(e)
                if e.raw_os_error() == Some(i32::try_from(ERROR_FILE_NOT_FOUND).unwrap_or(2)) =>
            {
                return Err(ClientError::NotRunning);
            }
            Err(e) if e.raw_os_error() == Some(i32::try_from(ERROR_PIPE_BUSY).unwrap_or(231)) => {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return Err(ClientError::Timeout);
                }
                let ms = u32::try_from(left.as_millis()).unwrap_or(u32::MAX).max(1);
                // SAFETY: `wide` is NUL-terminated and outlives the call.
                unsafe { WaitNamedPipeW(wide.as_ptr(), ms) };
            }
            Err(e) => return Err(e.into()),
        }
    }
}

#[cfg(not(windows))]
fn open_pipe(_name: &str, _timeout: Duration) -> Result<File, ClientError> {
    // Unix sockets arrive with Step 21.
    Err(ClientError::NotRunning)
}
