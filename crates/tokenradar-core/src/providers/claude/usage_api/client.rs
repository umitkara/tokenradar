//! HTTP transport for the usage API.
//!
//! The core runs on plain threads (ADR-027). The poller thread owns one current-thread Tokio
//! runtime and blocks on each request (ADR-028): no hidden runtime thread, and the same
//! `reqwest`/`rustls` stack the updater uses later. Runtime and client are created on the
//! first request, so without consent neither exists.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue, RETRY_AFTER};
use zeroize::Zeroizing;

use crate::secret::Secret;

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const BETA_HEADER: &str = "anthropic-beta";
const BETA_VALUE: &str = "oauth-2025-04-20";
const TIMEOUT: Duration = Duration::from_secs(10);
/// The real body is ~3 KB.
const MAX_BODY_BYTES: usize = 1 << 20;

/// A response, reduced to what the poller needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReply {
    pub status: u16,
    pub retry_after_ms: Option<i64>,
    pub body: Vec<u8>,
}

/// Why no response arrived. Categories only: `reqwest` messages are not passed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("the request timed out")]
    Timeout,
    #[error("could not connect")]
    Connect,
    #[error("the request failed")]
    Request,
    #[error("the response body was too large")]
    TooLarge,
    #[error("the token contains characters a header cannot carry")]
    BadToken,
    #[error("could not start the HTTP client")]
    Setup,
    #[error("the core is shutting down")]
    Cancelled,
}

/// Sends one usage request. A trait so tests can count calls without a network.
pub trait UsageTransport: Send {
    fn get_usage(&mut self, token: &Secret<String>) -> Result<HttpReply, TransportError>;
}

/// How often a request in flight checks for shutdown.
const CANCEL_POLL: Duration = Duration::from_millis(100);

/// The real transport.
#[derive(Default)]
pub struct HttpTransport {
    inner: Option<(tokio::runtime::Runtime, reqwest::Client)>,
    cancel: Option<Arc<AtomicBool>>,
}

impl HttpTransport {
    /// A transport whose request in flight is abandoned within ~100 ms once `cancel` is set,
    /// so shutdown does not wait for the 10 s timeout.
    pub fn with_cancel(cancel: Arc<AtomicBool>) -> Self {
        Self {
            inner: None,
            cancel: Some(cancel),
        }
    }

    fn ensure(&mut self) -> Result<&(tokio::runtime::Runtime, reqwest::Client), TransportError> {
        if self.inner.is_none() {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| TransportError::Setup)?;
            let client = {
                let _guard = rt.enter();
                reqwest::Client::builder()
                    .user_agent(concat!("TokenRadar/", env!("CARGO_PKG_VERSION")))
                    .timeout(TIMEOUT)
                    .build()
                    .map_err(|_| TransportError::Setup)?
            };
            self.inner = Some((rt, client));
        }
        self.inner.as_ref().ok_or(TransportError::Setup)
    }
}

impl UsageTransport for HttpTransport {
    fn get_usage(&mut self, token: &Secret<String>) -> Result<HttpReply, TransportError> {
        let cancel = self.cancel.clone();
        let (rt, client) = self.ensure()?;
        let mut auth = Zeroizing::new(String::with_capacity(token.expose().len() + 7));
        auth.push_str("Bearer ");
        auth.push_str(token.expose());
        let mut auth_value = HeaderValue::from_str(&auth).map_err(|_| TransportError::BadToken)?;
        // Hides the value from `Debug` and from HTTP/2 header compression tables. The bytes
        // inside `HeaderValue` cannot be zeroized; they are freed with the request.
        auth_value.set_sensitive(true);
        drop(auth);

        let request = client
            .get(USAGE_URL)
            .header(AUTHORIZATION, auth_value)
            .header(BETA_HEADER, BETA_VALUE)
            .header(CONTENT_TYPE, "application/json");
        rt.block_on(async move {
            let send = request.send();
            tokio::pin!(send);
            let response = loop {
                match tokio::time::timeout(CANCEL_POLL, &mut send).await {
                    Ok(result) => break result.map_err(|e| classify(&e))?,
                    Err(_) if cancel.as_ref().is_some_and(|c| c.load(Ordering::Relaxed)) => {
                        return Err(TransportError::Cancelled);
                    }
                    Err(_) => {}
                }
            };
            let status = response.status().as_u16();
            let retry_after_ms = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_retry_after);
            if response
                .content_length()
                .is_some_and(|n| n > MAX_BODY_BYTES as u64)
            {
                return Err(TransportError::TooLarge);
            }
            let body = response.bytes().await.map_err(|e| classify(&e))?;
            if body.len() > MAX_BODY_BYTES {
                return Err(TransportError::TooLarge);
            }
            Ok(HttpReply {
                status,
                retry_after_ms,
                body: body.to_vec(),
            })
        })
    }
}

fn classify(e: &reqwest::Error) -> TransportError {
    if e.is_timeout() {
        TransportError::Timeout
    } else if e.is_connect() {
        TransportError::Connect
    } else {
        TransportError::Request
    }
}

/// `Retry-After`: delay seconds or an HTTP date. Returns milliseconds from now.
pub(crate) fn parse_retry_after(v: &str) -> Option<i64> {
    let v = v.trim();
    if let Ok(secs) = v.parse::<i64>() {
        return (secs >= 0).then(|| secs.saturating_mul(1000));
    }
    let at = jiff::fmt::rfc2822::parse(v).ok()?.timestamp();
    Some((at.as_millisecond() - jiff::Timestamp::now().as_millisecond()).max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_accepts_seconds_and_dates() {
        assert_eq!(parse_retry_after("120"), Some(120_000));
        assert_eq!(parse_retry_after(" 0 "), Some(0));
        assert_eq!(parse_retry_after("-5"), None);
        assert_eq!(parse_retry_after("soon"), None);
        // A date in the past means "now".
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), Some(0));
    }
}
