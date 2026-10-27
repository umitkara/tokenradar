//! `<config>/.credentials.json`: plan metadata and, with consent, the OAuth access token
//! (data-sources.md S5, ADR-006).
//!
//! - [`read_plan`] reads `subscriptionType` and `rateLimitTier` only. Its struct has no token
//!   fields: serde skips them without building a string.
//! - [`read_access_token`] reads `accessToken` only, straight into a [`Secret`]. Callers must
//!   check consent first and drop the secret as soon as the request is sent.
//! - `refreshToken` has no field anywhere, so it is never deserialized. TokenRadar never
//!   refreshes: refresh tokens rotate, and refreshing would log Claude Code out.
//!
//! The file's raw bytes contain the tokens whatever struct parses them, so the read buffer is
//! [`Zeroizing`] and wiped as soon as parsing ends. The file is never written.

use std::io::Read as _;
use std::path::Path;

use serde::Deserialize;
use zeroize::Zeroizing;

use crate::secret::Secret;

/// File name inside a config directory.
pub const CREDENTIALS_FILE: &str = ".credentials.json";

/// The real file is well under 4 KB.
const MAX_FILE_BYTES: u64 = 64 << 10;

/// Plan metadata. No secrets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanInfo {
    /// e.g. `max`, `pro`.
    pub subscription_type: Option<String>,
    /// e.g. `default_claude_max_20x`.
    pub rate_limit_tier: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
    #[error("no credentials file")]
    Missing,
    #[error("could not read the credentials file: {0}")]
    Io(#[source] std::io::Error),
    #[error("the credentials file is larger than expected")]
    TooLarge,
    /// The message comes from serde and never quotes values.
    #[error("the credentials file has an unexpected format: {0}")]
    Format(String),
    #[error("the credentials file has no OAuth access token")]
    NoToken,
}

#[derive(Deserialize)]
struct PlanFile {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<PlanFields>,
}

/// Only metadata fields: `accessToken` and `refreshToken` are skipped as unknown.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanFields {
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
}

#[derive(Deserialize)]
struct TokenFile {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<TokenFields>,
}

/// Only the access token. There is deliberately no `refresh_token` field.
#[derive(Deserialize)]
struct TokenFields {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}

/// Plan metadata of a config directory. Needs no consent: the tokens are never deserialized.
pub fn read_plan(config_dir: &Path) -> Result<PlanInfo, CredentialsError> {
    let bytes = read_file(config_dir)?;
    let file: PlanFile = parse(&bytes)?;
    let fields = file.oauth.unwrap_or(PlanFields {
        subscription_type: None,
        rate_limit_tier: None,
    });
    Ok(PlanInfo {
        subscription_type: fields.subscription_type.filter(|s| !s.is_empty()),
        rate_limit_tier: fields.rate_limit_tier.filter(|s| !s.is_empty()),
    })
}

/// The OAuth access token. **Only call with the user's consent, right before a request.**
pub fn read_access_token(config_dir: &Path) -> Result<Secret<String>, CredentialsError> {
    let bytes = read_file(config_dir)?;
    let file: TokenFile = parse(&bytes)?;
    file.oauth
        .and_then(|o| o.access_token)
        .filter(|t| !t.is_empty())
        .map(Secret::new)
        .ok_or(CredentialsError::NoToken)
}

fn read_file(config_dir: &Path) -> Result<Zeroizing<Vec<u8>>, CredentialsError> {
    let path = config_dir.join(CREDENTIALS_FILE);
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(CredentialsError::Missing);
        }
        Err(e) => return Err(CredentialsError::Io(e)),
    };
    let mut bytes = Zeroizing::new(Vec::with_capacity(4096));
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(CredentialsError::Io)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(CredentialsError::TooLarge);
    }
    Ok(bytes)
}

fn parse<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, CredentialsError> {
    // Not `e.to_string()`: "invalid type" messages quote the offending value, which could be
    // a token. Category and position are enough to diagnose.
    serde_json::from_slice(bytes).map_err(|e| {
        CredentialsError::Format(format!(
            "{:?} error at line {} column {}",
            e.classify(),
            e.line(),
            e.column()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCESS: &str = "sk-ant-oat01-FAKE-access-token-for-tests";
    const REFRESH: &str = "sk-ant-ort01-FAKE-refresh-token-for-tests";

    fn write(dir: &Path, body: &str) {
        std::fs::write(dir.join(CREDENTIALS_FILE), body).expect("write");
    }

    fn sample() -> String {
        format!(
            r#"{{"claudeAiOauth":{{"accessToken":"{ACCESS}","refreshToken":"{REFRESH}",
            "expiresAt":1793050000000,"refreshTokenExpiresAt":1800000000000,
            "scopes":["user:inference","user:profile"],"subscriptionType":"max",
            "subscriptionTypeSource":"x","rateLimitTier":"default_claude_max_20x"}}}}"#
        )
    }

    #[test]
    fn plan_metadata_is_read_without_tokens() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), &sample());
        let plan = read_plan(dir.path()).expect("plan");
        assert_eq!(plan.subscription_type.as_deref(), Some("max"));
        assert_eq!(
            plan.rate_limit_tier.as_deref(),
            Some("default_claude_max_20x")
        );
        let rendered = format!("{plan:?}");
        assert!(!rendered.contains("sk-ant"), "{rendered}");
    }

    #[test]
    fn access_token_is_read_into_a_secret() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), &sample());
        let token = read_access_token(dir.path()).expect("token");
        assert_eq!(token.expose(), ACCESS);
        assert_eq!(format!("{token:?}"), "***");
    }

    /// The refresh token has no field: whatever it holds, even a value of the wrong type,
    /// never reaches a string.
    #[test]
    fn refresh_token_is_never_deserialized() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":{"not":"a string"},"subscriptionType":"pro"}}"#,
        );
        assert_eq!(read_access_token(dir.path()).expect("token").expose(), "a");
        assert_eq!(
            read_plan(dir.path())
                .expect("plan")
                .subscription_type
                .as_deref(),
            Some("pro")
        );
    }

    #[test]
    fn missing_and_malformed_files_are_errors_without_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(matches!(
            read_plan(dir.path()),
            Err(CredentialsError::Missing)
        ));
        write(
            dir.path(),
            &format!(r#"{{"claudeAiOauth":{{"accessToken":"{ACCESS}" "#),
        );
        let err = read_access_token(dir.path()).expect_err("truncated");
        assert!(!err.to_string().contains("sk-ant"), "{err}");
        // A token where a number belongs: serde's message would quote it.
        write(dir.path(), &format!(r#"{{"claudeAiOauth":"{ACCESS}"}}"#));
        let err = read_access_token(dir.path()).expect_err("wrong shape");
        assert!(!err.to_string().contains("sk-ant"), "{err}");
        write(
            dir.path(),
            r#"{"claudeAiOauth":{"subscriptionType":"max"}}"#,
        );
        assert!(matches!(
            read_access_token(dir.path()),
            Err(CredentialsError::NoToken)
        ));
        write(dir.path(), r"{}");
        assert_eq!(read_plan(dir.path()).expect("plan"), PlanInfo::default());
    }
}
