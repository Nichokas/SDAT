//! Start-up injection (Keychain token store, `UserDefaults` config) and the
//! TIDAL device-linking login.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tonearm_core::{api, auth, config, state};

use crate::{FfiError, Quality, run};

/// Credentials kept by the app (Keychain). Read on every request.
#[uniffi::export(with_foreign)]
pub trait TokenStore: Send + Sync {
    /// TIDAL refresh token, `None` when signed out.
    fn refresh_token(&self) -> Option<String>;
    /// TIDAL user id, `None` when signed out.
    fn user_id(&self) -> Option<String>;
}

/// Settings kept by the app (`UserDefaults`).
#[uniffi::export(with_foreign)]
pub trait AppConfig: Send + Sync {
    /// Default streaming quality.
    fn streaming_quality(&self) -> Quality;
    /// Writable directory for small preference files (Application Support).
    fn config_dir(&self) -> Option<String>;
}

struct Tokens(Arc<dyn TokenStore>);
impl auth::TokenStore for Tokens {
    fn has_refresh_token(&self) -> bool {
        self.0.refresh_token().is_some_and(|t| !t.is_empty())
    }
    fn get_refresh_token(&self) -> String {
        self.0.refresh_token().unwrap_or_default()
    }
    fn user_id(&self) -> String {
        self.0.user_id().unwrap_or_default()
    }
}

struct Config(Arc<dyn AppConfig>);
impl config::Config for Config {
    fn audio_quality(&self) -> tonearm_core::model::TrackQuality {
        self.0.streaming_quality().into()
    }
    fn config_dir(&self) -> Option<std::path::PathBuf> {
        self.0.config_dir().map(Into::into)
    }
}

/// Installs the app's stores. Call once at launch, before any other call.
#[uniffi::export]
pub fn initialize(tokens: Arc<dyn TokenStore>, config: Arc<dyn AppConfig>) {
    auth::install(Box::new(Tokens(tokens)));
    config::install(Box::new(Config(config)));
}

/// Call after the token store changed (login or logout): drops the cached
/// access token and the favourites.
#[uniffi::export]
pub fn session_changed() {
    api::reset_user_token();
    for c in [&state::ALBUMS, &state::ARTISTS, &state::MIXES, &state::PLAYLISTS, &state::TRACKS] {
        c.bust();
    }
}

/// Tokens to store in the Keychain after a successful login.
#[derive(Debug, Clone, uniffi::Record)]
pub struct LoginTokens {
    pub refresh_token: String,
    pub user_id: String,
}

/// A pending device login: show `user_code`, open `verification_url`
/// (link.tidal.com) and `wait()` until the user approves it.
#[derive(Debug, uniffi::Object)]
pub struct DeviceLogin {
    device_code: String,
    interval: i64,
    user_code: String,
    verification_url: String,
    expires_in_secs: i64,
    cancelled: Arc<AtomicBool>,
}

#[uniffi::export]
impl DeviceLogin {
    pub fn user_code(&self) -> String {
        self.user_code.clone()
    }
    /// `https://link.tidal.com/XXXXX` with the code filled in.
    pub fn verification_url(&self) -> String {
        self.verification_url.clone()
    }
    pub fn expires_in_secs(&self) -> i64 {
        self.expires_in_secs
    }
    /// Polls until approved, expired (`Other`) or cancelled (`Cancelled`).
    pub async fn wait(&self) -> Result<LoginTokens, FfiError> {
        let (code, interval, c) = (self.device_code.clone(), self.interval, Arc::clone(&self.cancelled));
        let r = run(move || api::poll_device_linking_status(&code, interval, &|| c.load(Ordering::SeqCst))).await?;
        let user_id = if r.user_id != 0 { r.user_id } else { r.user.id };
        Ok(LoginTokens { refresh_token: r.refresh_token, user_id: user_id.to_string() })
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

/// Starts a TIDAL device login.
#[uniffi::export]
pub async fn start_device_login() -> Result<Arc<DeviceLogin>, FfiError> {
    let d = run(api::request_device_linking_challenge).await?;
    let verification_url = if d.verification_uri_complete.starts_with("http") { d.verification_uri_complete } else { format!("https://{}", d.verification_uri_complete) };
    Ok(Arc::new(DeviceLogin {
        device_code: d.device_code,
        interval: d.interval,
        user_code: d.user_code,
        verification_url,
        expires_in_secs: d.expires_in,
        cancelled: Arc::new(AtomicBool::new(false)),
    }))
}
