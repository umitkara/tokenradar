//! Local IPC between the app (server) and the `tokenradar` CLI (client), architecture §11.
//!
//! Transport (ADR-029): a Windows named pipe, `\\.\pipe\tokenradar-<hash of the user's SID>`.
//! The server uses tokio's named pipes on its own small runtime; the client opens the pipe as
//! a file and needs no runtime, which keeps the CLI small and fast.

pub mod client;
pub mod proto;
#[cfg(windows)]
pub mod security;
#[cfg(windows)]
pub mod server;
pub mod view;

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

pub use client::{Client, ClientError, with_deadline};
pub use proto::{
    Handler, Hello, HelloReply, MAX_MESSAGE_BYTES, Op, PROTOCOL_VERSION, Request, Response,
    StatusQuery,
};
pub use view::{
    AccountInfo, JSON_SCHEMA, LimitInfo, LimitsReply, Origin, SessionInfo, SessionsReply,
    StatusReply,
};

/// The pipe of the current user. Each user has their own; the name reveals nothing about them.
pub fn pipe_name() -> std::io::Result<String> {
    #[cfg(windows)]
    {
        Ok(pipe_name_for(&security::current_user_sid()?))
    }
    #[cfg(not(windows))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "IPC is Windows-only until Step 21",
        ))
    }
}

/// `\\.\pipe\tokenradar-` + the first 12 hex digits of SHA-256 of `sid`.
pub fn pipe_name_for(sid: &str) -> String {
    let digest = Sha256::digest(sid.as_bytes());
    let hex = digest[..6].iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    format!(r"\\.\pipe\tokenradar-{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_names_are_stable_per_user_and_hide_the_sid() {
        let a = pipe_name_for("S-1-5-21-1-2-3-1001");
        assert_eq!(a, pipe_name_for("S-1-5-21-1-2-3-1001"));
        assert_ne!(a, pipe_name_for("S-1-5-21-1-2-3-1002"));
        assert!(a.starts_with(r"\\.\pipe\tokenradar-"));
        assert_eq!(a.len(), r"\\.\pipe\tokenradar-".len() + 12);
        assert!(!a.contains("1001"));
    }
}
