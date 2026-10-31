//! Secret storage through libsecret (Secret Service / XDG portal), same schema
//! (`dev.dergs.Tonearm`, attribute `key`) as the Go implementation, so existing
//! sign-ins keep working.

use std::collections::HashMap;

use base64::Engine;

use crate::i18n::{tr, trf};

const REFRESH_TOKEN_KEY: &str = "refresh_token";

#[derive(Debug, Clone)]
pub struct ServiceError {
    pub title: String,
    pub body: String,
    pub fatal: bool,
}

fn schema() -> libsecret::Schema {
    let mut attrs = HashMap::new();
    attrs.insert("key", libsecret::SchemaAttributeType::String);
    libsecret::Schema::new("dev.dergs.Tonearm", libsecret::SchemaFlags::NONE, attrs)
}

fn attrs(key: &str) -> HashMap<&str, &str> {
    let mut a = HashMap::new();
    a.insert("key", key);
    a
}

fn lookup(key: &str) -> Result<Option<String>, gtk::glib::Error> {
    libsecret::password_lookup_sync(Some(&schema()), attrs(key), gtk::gio::Cancellable::NONE).map(|o| o.map(|s| s.to_string()))
}

pub fn healthcheck() -> Option<ServiceError> {
    let err = match lookup("dummy_key") {
        Ok(_) => return None,
        Err(e) => e.to_string(),
    };
    if err.contains("name is not activatable") || err.contains("ServiceUnknown") {
        return Some(ServiceError {
            title: tr("Secret Service Unavailable"),
            body: tr("No secret service provider is available.\n\nTonearm will not be able to store any authentication-related data and you will not be able to sign in.\n\nPlease install a secret service provider such as GNOME Keyring or KDE Wallet."),
            fatal: false,
        });
    }
    if err.contains("user interaction failed") {
        return Some(ServiceError {
            title: tr("Secret Service Issue"),
            body: tr("Your secret service provider was found, but refused to interact with Tonearm.\n\nThis could because you cancelled a prompt or, if you are using a Flatpak, the provider not implementing the XDG Secret Portal service.\n\nTonearm will not be able to store any authentication-related data and you will not be able to sign in."),
            fatal: false,
        });
    }
    Some(ServiceError {
        title: tr("Secret Service Error"),
        body: trf("An unknown error occurred when checking for a secret service provider.\n\nSigning in may or may not work. Please see the raw error message for more details:\n\n%s", &[&err]),
        fatal: false,
    })
}

pub fn has_refresh_token() -> bool {
    match lookup(REFRESH_TOKEN_KEY) {
        Ok(Some(v)) => !v.is_empty(),
        Ok(None) => false,
        Err(e) => {
            log::error!("error checking refresh token in keyring: {e}");
            false
        }
    }
}

pub fn get_refresh_token() -> String {
    match lookup(REFRESH_TOKEN_KEY) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            log::error!("error reading refresh token from keyring: {e}");
            String::new()
        }
    }
}

/// Must be followed by `crate::state::notify_signed_in_changed()` on the main thread.
pub fn set_refresh_token(token: &str) -> anyhow::Result<()> {
    libsecret::password_store_sync(
        Some(&schema()),
        attrs(REFRESH_TOKEN_KEY),
        Some("default"),
        "Tonearm TIDAL Refresh Token",
        token,
        gtk::gio::Cancellable::NONE,
    )?;
    Ok(())
}

pub fn delete_refresh_token() -> anyhow::Result<()> {
    libsecret::password_clear_sync(Some(&schema()), attrs(REFRESH_TOKEN_KEY), gtk::gio::Cancellable::NONE).map_err(|e| {
        log::error!("error deleting refresh token from keyring: {e}");
        anyhow::anyhow!(e)
    })
}

/// Extracts the `uid` claim from the refresh token JWT.
pub fn user_id_from_token(token: &str) -> String {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return String::new();
    }
    let Ok(payload) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1].trim_end_matches('=')) else {
        return String::new();
    };
    #[derive(serde::Deserialize)]
    struct Claims {
        uid: i64,
    }
    match serde_json::from_slice::<Claims>(&payload) {
        Ok(c) => c.uid.to_string(),
        Err(_) => String::new(),
    }
}

pub fn user_id() -> String {
    if !has_refresh_token() {
        return String::new();
    }
    user_id_from_token(&get_refresh_token())
}

#[cfg(test)]
mod tests {
    #[test]
    fn jwt_uid() {
        // header.{"uid":12345}.sig
        let payload = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, br#"{"uid":12345}"#);
        assert_eq!(super::user_id_from_token(&format!("x.{payload}.y")), "12345");
        assert_eq!(super::user_id_from_token("garbage"), "");
    }
}
