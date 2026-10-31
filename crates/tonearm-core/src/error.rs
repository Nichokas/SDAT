//! Typed error for the core's public boundary.
//!
//! Internals use `anyhow`; foreign bindings (`UniFFI`/Swift) need a closed
//! set of cases, so `CoreError` classifies whatever bubbled up.

/// Errors exposed by `tonearm-core` to its callers.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// No session tokens are available (signed out or expired refresh).
    #[error("not signed in")]
    NotSignedIn,
    /// Transport failure (DNS, TLS, timeout, connection reset).
    #[error("network error: {0}")]
    Network(String),
    /// The server answered with a non-success status.
    #[error("HTTP {status}: {message}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Server message or body excerpt.
        message: String,
    },
    /// The response could not be decoded.
    #[error("decode error: {0}")]
    Decode(String),
    /// Local filesystem failure (cache, country file).
    #[error("io error: {0}")]
    Io(String),
    /// Anything else.
    #[error("{0}")]
    Other(String),
}

/// Result alias for the core boundary.
pub type CoreResult<T> = Result<T, CoreError>;

impl From<reqwest::Error> for CoreError {
    fn from(e: reqwest::Error) -> Self {
        match e.status() {
            Some(s) => Self::Http { status: s.as_u16(), message: e.to_string() },
            None if e.is_decode() => Self::Decode(e.to_string()),
            None => Self::Network(e.to_string()),
        }
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Decode(e.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<anyhow::Error> for CoreError {
    fn from(e: anyhow::Error) -> Self {
        for cause in e.chain() {
            if let Some(r) = cause.downcast_ref::<reqwest::Error>() {
                return match r.status() {
                    Some(s) => Self::Http { status: s.as_u16(), message: format!("{e:#}") },
                    None if r.is_decode() => Self::Decode(format!("{e:#}")),
                    None => Self::Network(format!("{e:#}")),
                };
            }
            if cause.is::<serde_json::Error>() {
                return Self::Decode(format!("{e:#}"));
            }
            if cause.is::<std::io::Error>() {
                return Self::Io(format!("{e:#}"));
            }
        }
        Self::Other(format!("{e:#}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_anyhow_chains() {
        let e = anyhow::Error::from(serde_json::from_str::<u8>("x").unwrap_err()).context("page");
        assert!(matches!(CoreError::from(e), CoreError::Decode(_)));
        let e = anyhow::Error::from(std::io::Error::other("disk")).context("cache");
        assert!(matches!(CoreError::from(e), CoreError::Io(_)));
        assert!(matches!(CoreError::from(anyhow::anyhow!("boom")), CoreError::Other(m) if m == "boom"));
    }
}
