//! Wire format (architecture §11): one JSON object per line.
//!
//! ```text
//! → {"v":1,"id":7,"op":"status.get","data":{"session_uuid":"…"}}
//! ← {"v":1,"id":7,"ok":true,"data":{…}}
//! ← {"v":1,"id":8,"ok":false,"error":"unsupported"}
//! ```
//!
//! `statusline.push` gets no response. An unknown `op` or a newer `v` is answered with an error
//! and the connection stays open, so old apps and new CLIs (or the reverse) degrade gracefully.
//! A line that is not a request at all closes the connection.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::view::{LimitsReply, SessionsReply, StatusReply};
use crate::statusline::StatuslinePush;

/// Current protocol version.
pub const PROTOCOL_VERSION: u8 = 1;

/// Largest accepted line, request or response.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

pub const ERR_UNSUPPORTED: &str = "unsupported";
pub const ERR_VERSION: &str = "unsupported_version";
pub const ERR_BAD_DATA: &str = "bad_data";

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub v: u8,
    pub id: u32,
    pub op: Op,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Hello(Hello),
    /// Boxed: a payload is much larger than every other request.
    StatuslinePush(Box<StatuslinePush>),
    StatusGet(StatusQuery),
    LimitsGet,
    SessionsLive,
}

impl Op {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Hello(_) => "hello",
            Self::StatuslinePush(_) => "statusline.push",
            Self::StatusGet(_) => "status.get",
            Self::LimitsGet => "limits.get",
            Self::SessionsLive => "sessions.live",
        }
    }

    /// Whether the server answers.
    pub const fn expects_reply(&self) -> bool {
        !matches!(self, Self::StatuslinePush(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// e.g. `tokenradar-cli/0.1.0`.
    pub client: String,
    /// Highest protocol version the client speaks.
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloReply {
    pub server: String,
    pub protocol: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusQuery {
    /// The statusline's session, to get its share of the window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_uuid: Option<String>,
}

/// The envelope as it travels.
#[derive(Serialize, Deserialize)]
struct WireRequest {
    v: u8,
    id: u32,
    op: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Serialize, Deserialize)]
struct WireResponse {
    v: u8,
    id: u32,
    ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Why a line was not accepted as a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Not a request envelope; the connection is closed.
    Malformed,
    /// A well-formed envelope the server answers with an error.
    Refused { id: u32, error: &'static str },
}

impl Request {
    pub fn new(id: u32, op: Op) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            op,
        }
    }

    /// One line, newline included.
    pub fn encode(&self) -> Vec<u8> {
        let data = match &self.op {
            Op::Hello(h) => serde_json::to_value(h).ok(),
            Op::StatuslinePush(p) => serde_json::to_value(p).ok(),
            Op::StatusGet(q) => serde_json::to_value(q).ok(),
            Op::LimitsGet | Op::SessionsLive => None,
        };
        line(&WireRequest {
            v: self.v,
            id: self.id,
            op: self.op.name().to_owned(),
            data,
        })
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let wire: WireRequest =
            serde_json::from_slice(bytes).map_err(|_| DecodeError::Malformed)?;
        let refuse = |error| DecodeError::Refused { id: wire.id, error };
        if wire.v > PROTOCOL_VERSION || wire.v == 0 {
            return Err(refuse(ERR_VERSION));
        }
        let data = wire.data.unwrap_or(Value::Null);
        let id = wire.id;
        let op = match wire.op.as_str() {
            "hello" => Op::Hello(parse(data, id)?),
            "statusline.push" => Op::StatuslinePush(Box::new(parse(data, id)?)),
            "status.get" => Op::StatusGet(if data.is_null() {
                StatusQuery::default()
            } else {
                parse(data, id)?
            }),
            "limits.get" => Op::LimitsGet,
            "sessions.live" => Op::SessionsLive,
            _ => return Err(refuse(ERR_UNSUPPORTED)),
        };
        Ok(Self {
            v: wire.v,
            id: wire.id,
            op,
        })
    }
}

/// A reply: `data` on success, an error code otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub v: u8,
    pub id: u32,
    pub result: Result<Value, String>,
}

impl Response {
    pub fn ok(id: u32, data: &impl Serialize) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            result: serde_json::to_value(data).map_err(|_| ERR_BAD_DATA.to_owned()),
        }
    }

    pub fn error(id: u32, error: &str) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            result: Err(error.to_owned()),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let (ok, data, error) = match &self.result {
            Ok(d) => (true, Some(d.clone()), None),
            Err(e) => (false, None, Some(e.clone())),
        };
        line(&WireResponse {
            v: self.v,
            id: self.id,
            ok,
            data,
            error,
        })
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let wire: WireResponse = serde_json::from_slice(bytes).ok()?;
        let result = if wire.ok {
            Ok(wire.data.unwrap_or(Value::Null))
        } else {
            Err(wire.error.unwrap_or_default())
        };
        Some(Self {
            v: wire.v,
            id: wire.id,
            result,
        })
    }

    /// The data as `T`, or the error code.
    pub fn into_data<T: serde::de::DeserializeOwned>(self) -> Result<T, String> {
        let data = self.result?;
        serde_json::from_value(data).map_err(|_| ERR_BAD_DATA.to_owned())
    }
}

fn parse<T: serde::de::DeserializeOwned>(data: Value, id: u32) -> Result<T, DecodeError> {
    serde_json::from_value(data).map_err(|_| DecodeError::Refused {
        id,
        error: ERR_BAD_DATA,
    })
}

fn line(v: &impl Serialize) -> Vec<u8> {
    let mut out = serde_json::to_vec(v).unwrap_or_default();
    out.push(b'\n');
    out
}

/// What the server can answer; implemented by the core runtime and by test doubles.
pub trait Handler: Send + Sync + 'static {
    fn push(&self, push: StatuslinePush);
    fn status(&self, query: &StatusQuery) -> StatusReply;
    fn limits(&self) -> LimitsReply;
    fn sessions(&self) -> SessionsReply;
}

/// The reply to `req`, or `None` for a push.
pub fn handle(handler: &dyn Handler, req: Request, server: &str) -> Option<Response> {
    let id = req.id;
    Some(match req.op {
        Op::Hello(_) => Response::ok(
            id,
            &HelloReply {
                server: server.to_owned(),
                protocol: PROTOCOL_VERSION,
            },
        ),
        Op::StatuslinePush(p) => {
            handler.push(*p);
            return None;
        }
        Op::StatusGet(q) => Response::ok(id, &handler.status(&q)),
        Op::LimitsGet => Response::ok(id, &handler.limits()),
        Op::SessionsLive => Response::ok(id, &handler.sessions()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Timestamp;
    use crate::statusline::StatuslinePayload;

    fn push() -> StatuslinePush {
        StatuslinePush {
            observed_at: Timestamp::from_millis(1_793_059_200_000),
            payload: StatuslinePayload::parse(
                br#"{"session_id":"s","rate_limits":{"five_hour":{"used_percentage":72,"resets_at":1793070000}}}"#,
            )
            .expect("payload"),
        }
    }

    #[test]
    fn requests_round_trip() {
        let ops = [
            Op::Hello(Hello {
                client: "tokenradar-cli/0.0.1".into(),
                version: PROTOCOL_VERSION,
            }),
            Op::StatuslinePush(Box::new(push())),
            Op::StatusGet(StatusQuery {
                session_uuid: Some("s".into()),
            }),
            Op::StatusGet(StatusQuery::default()),
            Op::LimitsGet,
            Op::SessionsLive,
        ];
        for (id, op) in ops.into_iter().enumerate() {
            let req = Request::new(u32::try_from(id).expect("small"), op);
            let bytes = req.encode();
            assert_eq!(bytes.last(), Some(&b'\n'));
            assert!(!bytes[..bytes.len() - 1].contains(&b'\n'), "one line");
            assert_eq!(Request::decode(&bytes), Ok(req));
        }
    }

    #[test]
    fn the_wire_shape_matches_the_architecture() {
        let bytes = Request::new(7, Op::LimitsGet).encode();
        assert_eq!(bytes, b"{\"v\":1,\"id\":7,\"op\":\"limits.get\"}\n");
        let r = Response::error(8, ERR_UNSUPPORTED).encode();
        assert_eq!(
            r,
            b"{\"v\":1,\"id\":8,\"ok\":false,\"error\":\"unsupported\"}\n"
        );
        let r = Response::ok(
            9,
            &HelloReply {
                server: "x".into(),
                protocol: 1,
            },
        );
        assert_eq!(Response::decode(&r.encode()), Some(r));
    }

    #[test]
    fn newer_versions_and_unknown_ops_are_refused_not_dropped() {
        assert_eq!(
            Request::decode(br#"{"v":2,"id":3,"op":"limits.get"}"#),
            Err(DecodeError::Refused {
                id: 3,
                error: ERR_VERSION
            })
        );
        assert_eq!(
            Request::decode(br#"{"v":1,"id":4,"op":"windows.list","data":{"x":1}}"#),
            Err(DecodeError::Refused {
                id: 4,
                error: ERR_UNSUPPORTED
            })
        );
        assert_eq!(
            Request::decode(br#"{"v":1,"id":5,"op":"hello","data":{"client":1}}"#),
            Err(DecodeError::Refused {
                id: 5,
                error: ERR_BAD_DATA
            })
        );
    }

    #[test]
    fn garbage_is_malformed() {
        for bad in [
            &b"not json"[..],
            b"[]",
            b"{\"id\":1}",
            b"{\"v\":1,\"op\":\"hello\"}",
        ] {
            assert_eq!(Request::decode(bad), Err(DecodeError::Malformed));
        }
    }
}
