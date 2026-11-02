//! Last.fm and `ListenBrainz` scrobbling. The app owns the listening clock
//! and calls [`should_scrobble`]; credentials live in its Keychain.

use tonearm_core::scrobbling as s;

use crate::{FfiError, run};

const LISTENBRAINZ_URL: &str = "https://api.listenbrainz.org";

#[derive(Debug, Clone, uniffi::Record)]
pub struct ScrobbleTrack {
    pub title: String,
    /// Artists joined with ", ".
    pub artist: String,
    pub album: String,
    pub url: String,
    pub duration_secs: u32,
}

impl From<ScrobbleTrack> for s::TrackMeta {
    fn from(t: ScrobbleTrack) -> Self {
        s::TrackMeta { title: t.title, artist: t.artist, album: t.album, url: t.url, duration: std::time::Duration::from_secs(u64::from(t.duration_secs)) }
    }
}

/// Whether `listened_secs` of real playback reached the threshold (half the
/// track or 4 minutes; tracks of 30 s or less never scrobble).
#[uniffi::export]
pub fn should_scrobble(listened_secs: u64, duration_secs: u32) -> bool {
    s::should_scrobble(listened_secs, std::time::Duration::from_secs(u64::from(duration_secs)))
}

/// Last.fm web authorisation: open `url`, then [`lastfm_finish_auth`] with `token`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct LastFmAuth {
    pub token: String,
    pub url: String,
}

#[uniffi::export]
pub async fn lastfm_begin_auth() -> Result<LastFmAuth, FfiError> {
    let token = run(s::lastfm_get_token).await?;
    Ok(LastFmAuth { url: s::lastfm_auth_url(&token), token })
}

/// The session key once the user approved, `None` while still pending.
#[uniffi::export]
pub async fn lastfm_finish_auth(token: String) -> Result<Option<String>, FfiError> {
    run(move || s::lastfm_get_session(&token)).await
}

#[uniffi::export]
pub async fn lastfm_username(session_key: String) -> Result<String, FfiError> {
    run(move || s::lastfm_self_name(&session_key)).await
}

#[uniffi::export]
pub async fn lastfm_now_playing(session_key: String, track: ScrobbleTrack) -> Result<(), FfiError> {
    run(move || s::lastfm_call("track.updateNowPlaying", &s::lastfm_track_params(&track.into(), &session_key), true, true).map(drop)).await
}

/// `started_at`: Unix seconds when the track started playing.
#[uniffi::export]
pub async fn lastfm_scrobble(session_key: String, track: ScrobbleTrack, started_at: i64) -> Result<(), FfiError> {
    run(move || {
        let mut p = s::lastfm_track_params(&track.into(), &session_key);
        p.push(("timestamp", started_at.to_string()));
        s::lastfm_call("track.scrobble", &p, true, true).map(drop)
    })
    .await
}

/// Fire-and-forget (errors are logged by the core).
#[uniffi::export]
pub async fn listenbrainz_now_playing(user_token: String, track: ScrobbleTrack) {
    let body = s::listenbrainz_request(&track.into(), None);
    blocking::unblock(move || s::listenbrainz_send(LISTENBRAINZ_URL, &user_token, &body)).await;
}

/// Fire-and-forget (errors are logged by the core).
#[uniffi::export]
pub async fn listenbrainz_scrobble(user_token: String, track: ScrobbleTrack, listened_at: i64) {
    let body = s::listenbrainz_request(&track.into(), Some(listened_at));
    blocking::unblock(move || s::listenbrainz_send(LISTENBRAINZ_URL, &user_token, &body)).await;
}
