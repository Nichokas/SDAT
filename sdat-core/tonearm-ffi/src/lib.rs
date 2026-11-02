//! Swift-facing API of `tonearm-core` (`UniFFI` proc-macros).
//!
//! Every network call is `async` for Swift and runs the blocking core on a
//! worker thread (`blocking::unblock`), so no async runtime is needed.
//! Only what the Torneam screens use is exported.

#![allow(clippy::needless_pass_by_value, reason = "UniFFI passes owned values across the boundary")]
#![allow(clippy::must_use_candidate, reason = "exported API")]
#![allow(clippy::missing_errors_doc, reason = "every error is FfiError, documented on the type")]

mod auth;
mod convert;
mod library;
mod live;
mod playback;
mod scrobbling;

pub use auth::*;
pub use convert::*;
pub use library::*;
pub use live::*;
pub use playback::*;
pub use scrobbling::*;

use tonearm_core::CoreError;

uniffi::setup_scaffolding!();

/// Error returned by every fallible exported function.
/// No field is named `message`: it would clash with `Throwable.message` in the Kotlin bindings.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    /// No TIDAL session: show the login screen.
    #[error("not logged in")]
    NotSignedIn,
    #[error("network: {detail}")]
    Network { detail: String },
    #[error("HTTP {status}: {detail}")]
    Http { status: u16, detail: String },
    #[error("decode: {detail}")]
    Decode { detail: String },
    #[error("Last.fm error {code}: {detail}")]
    LastFm { code: i64, detail: String },
    #[error("cancelled")]
    Cancelled,
    #[error("{detail}")]
    Other { detail: String },
}

impl From<CoreError> for FfiError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::NotSignedIn => Self::NotSignedIn,
            CoreError::Network(detail) => Self::Network { detail },
            CoreError::Http { status, message: detail } => Self::Http { status, detail },
            CoreError::Decode(detail) => Self::Decode { detail },
            CoreError::LastFm { code, message: detail } => Self::LastFm { code, detail },
            CoreError::Cancelled(_) => Self::Cancelled,
            CoreError::Io(detail) | CoreError::Other(detail) => Self::Other { detail },
        }
    }
}

/// Runs blocking core work on a worker thread.
pub(crate) async fn run<T: Send + 'static>(f: impl FnOnce() -> tonearm_core::CoreResult<T> + Send + 'static) -> Result<T, FfiError> {
    blocking::unblock(f).await.map_err(Into::into)
}

/// Forces the locale of the core's few translated strings (e.g. `es_ES`).
/// Call before anything else; later calls are ignored.
#[uniffi::export]
pub fn set_locale(locale: String) {
    tonearm_core::i18n::set_locale(&locale);
}
