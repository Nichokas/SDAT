//! Credentials injected by the front end. The desktop app keeps the refresh
//! token in the Secret Service; the core only asks for it.

use std::sync::OnceLock;

/// Read access to the user's TIDAL credentials.
pub trait TokenStore: Send + Sync {
    /// Whether a refresh token is stored (signed in).
    fn has_refresh_token(&self) -> bool;
    /// The stored refresh token, or an empty string.
    fn get_refresh_token(&self) -> String;
    /// The signed-in user's id, or an empty string.
    fn user_id(&self) -> String;
}

/// No credentials: everything runs signed out.
#[derive(Debug, Clone, Copy, Default)]
pub struct SignedOut;
impl TokenStore for SignedOut {
    fn has_refresh_token(&self) -> bool {
        false
    }
    fn get_refresh_token(&self) -> String {
        String::new()
    }
    fn user_id(&self) -> String {
        String::new()
    }
}

static TOKENS: OnceLock<Box<dyn TokenStore>> = OnceLock::new();

/// Installs the front end's token store. Call once at start-up, before any
/// other core call; later calls are ignored (returns `false`).
pub fn install(t: Box<dyn TokenStore>) -> bool {
    TOKENS.set(t).is_ok()
}

/// The installed token store, or [`SignedOut`].
pub fn tokens() -> &'static dyn TokenStore {
    TOKENS.get_or_init(|| Box::new(SignedOut)).as_ref()
}
