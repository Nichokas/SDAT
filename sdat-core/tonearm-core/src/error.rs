//! Typed error returned by every fallible public function of the core.
//!
//! `Display` keeps the exact messages the desktop app logged before the
//! switch from `anyhow`, so its logs and toasts read the same.

/// Errors exposed by `tonearm-core` to its callers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// No session tokens are available (signed out or expired refresh).
    #[error("not logged in")]
    NotSignedIn,
    /// Transport failure (DNS, TLS, timeout, connection reset).
    #[error("{0}")]
    Network(String),
    /// The server answered with a non-success status.
    #[error("{message}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Human readable message (kept identical to the old `anyhow` text).
        message: String,
    },
    /// The response could not be decoded.
    #[error("{0}")]
    Decode(String),
    /// Local filesystem failure (cache, country file).
    #[error("{0}")]
    Io(String),
    /// Last.fm API error (`error` field of its JSON answer).
    #[error("Last.fm error {code}: {message}")]
    LastFm {
        /// Last.fm error code.
        code: i64,
        /// Last.fm error message.
        message: String,
    },
    /// The user or the caller cancelled the operation.
    #[error("{0}")]
    Cancelled(String),
    /// Anything else.
    #[error("{0}")]
    Other(String),
}

/// Result alias for the core boundary.
pub type CoreResult<T> = Result<T, CoreError>;

impl CoreError {
    /// HTTP error with the given status and message.
    pub fn http(status: impl Into<u16>, message: impl Into<String>) -> Self {
        Self::Http { status: status.into(), message: message.into() }
    }

    /// Prefixes the message with `ctx` (like `anyhow`'s alternate format).
    #[must_use]
    pub fn context(self, ctx: impl std::fmt::Display) -> Self {
        let p = |m: String| format!("{ctx}: {m}");
        match self {
            Self::Network(m) => Self::Network(p(m)),
            Self::Http { status, message } => Self::Http { status, message: p(message) },
            Self::Decode(m) => Self::Decode(p(m)),
            Self::Io(m) => Self::Io(p(m)),
            Self::Cancelled(m) => Self::Cancelled(p(m)),
            Self::Other(m) => Self::Other(p(m)),
            e @ (Self::NotSignedIn | Self::LastFm { .. }) => e,
        }
    }
}

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

impl From<reqwest::header::InvalidHeaderValue> for CoreError {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<url::ParseError> for CoreError {
    fn from(e: url::ParseError) -> Self {
        Self::Other(e.to_string())
    }
}

/// `anyhow::Context` look-alike for core results.
pub(crate) trait Context<T> {
    fn context(self, ctx: impl std::fmt::Display) -> CoreResult<T>;
    fn with_context<C: std::fmt::Display>(self, f: impl FnOnce() -> C) -> CoreResult<T>;
}

impl<T, E: Into<CoreError>> Context<T> for Result<T, E> {
    fn context(self, ctx: impl std::fmt::Display) -> CoreResult<T> {
        self.map_err(|e| e.into().context(ctx))
    }
    fn with_context<C: std::fmt::Display>(self, f: impl FnOnce() -> C) -> CoreResult<T> {
        self.map_err(|e| e.into().context(f()))
    }
}

/// `anyhow::bail!` look-alike: returns `CoreError::Other`.
macro_rules! bail {
    ($($t:tt)*) => { return Err($crate::error::CoreError::Other(format!($($t)*))) };
}
pub(crate) use bail;

/// Returns `CoreError::Http` with the given status and formatted message.
macro_rules! bail_http {
    ($status:expr, $($t:tt)*) => { return Err($crate::error::CoreError::http($status, format!($($t)*))) };
}
pub(crate) use bail_http;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_keeps_the_kind() {
        let e: CoreResult<u8> = serde_json::from_str::<u8>("x").context("page");
        assert!(matches!(e, Err(CoreError::Decode(m)) if m.starts_with("page: ")));
        let e: CoreResult<()> = Err(CoreError::http(404u16, "unexpected status code: 404")).context("album");
        assert_eq!(e.unwrap_err(), CoreError::http(404u16, "album: unexpected status code: 404"));
    }

    #[test]
    fn display_matches_old_messages() {
        assert_eq!(CoreError::NotSignedIn.to_string(), "not logged in");
        assert_eq!(CoreError::LastFm { code: 14, message: "x".into() }.to_string(), "Last.fm error 14: x");
    }
}
