//! Port of `internal/features/scrobbling`: `ListenBrainz` and Last.fm scrobblers
//! driven by a listening clock (scrobble after half the track or 4 minutes).
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, bail};
use md5::{Digest, Md5};
use serde::Serialize;

use crate::model::Track;
use crate::player::{self, PlaybackStatus};
use crate::settings;

fn http() -> &'static reqwest::blocking::Client {
    static C: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    C.get_or_init(|| reqwest::blocking::Client::builder().user_agent("Go-http-client/1.1").timeout(Duration::from_secs(30)).build().expect("http client"))
}

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Scrobble data captured on the main thread so it can be sent from a worker.
#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub url: String,
    pub duration: Duration,
}

impl TrackMeta {
    pub fn from_track(t: &Track) -> Self {
        TrackMeta {
            title: t.title().to_string(),
            artist: t.artist_names().join(", "),
            album: t.album.as_ref().map(|a| a.title.clone()).unwrap_or_default(),
            url: t.url(),
            duration: t.duration(),
        }
    }
}

/// Whether the clock reached the scrobble threshold (Go: half the duration or 4 minutes).
pub fn should_scrobble(listened_secs: u64, duration: Duration) -> bool {
    listened_secs >= duration.as_secs() / 2 || listened_secs >= 240
}

// ---------------- ListenBrainz ----------------

#[derive(Serialize)]
struct LbAdditionalInfo {
    media_player: &'static str,
    music_service: &'static str,
    origin_url: String,
    submission_client: &'static str,
    duration_ms: i64,
    isrc: String,
}
#[derive(Serialize)]
struct LbMetadata {
    additional_info: LbAdditionalInfo,
    artist_name: String,
    track_name: String,
}
#[derive(Serialize)]
struct LbPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    listened_at: Option<i64>,
    track_metadata: LbMetadata,
}
#[derive(Serialize)]
struct LbRequest {
    listen_type: &'static str,
    payload: Vec<LbPayload>,
}

fn listenbrainz_request(t: &TrackMeta, listened_at: Option<i64>) -> String {
    serde_json::to_string(&LbRequest {
        listen_type: if listened_at.is_some() { "single" } else { "playing_now" },
        payload: vec![LbPayload {
            listened_at,
            track_metadata: LbMetadata {
                additional_info: LbAdditionalInfo {
                    media_player: "Tonearm",
                    music_service: "tidal.com",
                    origin_url: t.url.clone(),
                    submission_client: "Tonearm",
                    duration_ms: t.duration.as_millis() as i64,
                    isrc: String::new(),
                },
                artist_name: t.artist.clone(),
                track_name: t.title.clone(),
            },
        }],
    })
    .unwrap_or_default()
}

fn listenbrainz_configured() -> bool {
    settings::enable_listenbrainz() && !settings::listenbrainz_token().is_empty()
}

fn listenbrainz_send(url: &str, token: &str, body: String) {
    let r = http()
        .post(format!("{url}/1/submit-listens"))
        .header("Authorization", format!("Token {token}"))
        .header("Content-Type", "application/json")
        .body(body)
        .send();
    match r {
        Err(e) => log::error!("failed to send request to listenbrainz: {e}"),
        Ok(r) if !r.status().is_success() => {
            let s = r.status();
            log::error!("listenbrainz returned non-200 status code status={s} body={}", r.text().unwrap_or_default());
        }
        _ => {}
    }
}

// ---------------- Last.fm ----------------

const LASTFM_API_KEY: &str = "9691195478432f52cee57cc2a1b8b066";
const LASTFM_SECRET: &str = "3544f7702f11faea922c586be7b4df63";
const LASTFM_API: &str = "https://ws.audioscrobbler.com/2.0/";
const LASTFM_ERR_UNAUTHORIZED_TOKEN: i64 = 14;

pub fn lastfm_signature(params: &BTreeMap<String, String>) -> String {
    let mut s = String::new();
    for (k, v) in params {
        if k == "format" || k == "callback" {
            continue;
        }
        s.push_str(k);
        s.push_str(v);
    }
    s.push_str(LASTFM_SECRET);
    hex(&Md5::digest(s.as_bytes()))
}

/// Lower-case hex encoding without per-byte allocations.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[derive(Debug)]
pub struct LastFmError {
    pub code: i64,
    pub message: String,
}
impl std::fmt::Display for LastFmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Last.fm error {}: {}", self.code, self.message)
    }
}
impl std::error::Error for LastFmError {}

fn lastfm_call(method: &str, extra: &[(&str, String)], post: bool, sign: bool) -> Result<serde_json::Value> {
    let mut p: BTreeMap<String, String> = BTreeMap::new();
    p.insert("method".into(), method.into());
    p.insert("api_key".into(), LASTFM_API_KEY.into());
    for (k, v) in extra {
        p.insert((*k).into(), v.clone());
    }
    if sign {
        let sig = lastfm_signature(&p);
        p.insert("api_sig".into(), sig);
    }
    p.insert("format".into(), "json".into());
    let resp = if post { http().post(LASTFM_API).form(&p).send()? } else { {
        let qs: Vec<String> = p.iter().map(|(k, v)| format!("{k}={}", urlencoding::encode(v))).collect();
        http().get(format!("{LASTFM_API}?{}", qs.join("&"))).send()?
    } };
    let v: serde_json::Value = resp.json()?;
    if let Some(code) = v.get("error").and_then(serde_json::Value::as_i64) {
        return Err(LastFmError { code, message: v.get("message").and_then(|m| m.as_str()).unwrap_or_default().to_string() }.into());
    }
    Ok(v)
}

fn session_key() -> String {
    settings::lastfm_token()
}

pub fn lastfm_auth_url(token: &str) -> String {
    format!("https://www.last.fm/api/auth/?api_key={LASTFM_API_KEY}&token={token}")
}

pub fn lastfm_get_token() -> Result<String> {
    let v = lastfm_call("auth.getToken", &[], false, true)?;
    v.get("token").and_then(|t| t.as_str()).map(String::from).ok_or_else(|| anyhow!("missing token"))
}

/// Exchanges an authorised token for a session key. `Ok(None)` while still unauthorised.
pub fn lastfm_get_session(token: &str) -> Result<Option<String>> {
    match lastfm_call("auth.getSession", &[("token", token.to_string())], false, true) {
        Ok(v) => Ok(v.pointer("/session/key").and_then(|k| k.as_str()).map(String::from)),
        Err(e) => match e.downcast_ref::<LastFmError>() {
            Some(le) if le.code == LASTFM_ERR_UNAUTHORIZED_TOKEN => Ok(None),
            _ => Err(e),
        },
    }
}

/// Name of the logged in user (`user.getInfo` with the session key).
pub fn lastfm_self_name(sk: &str) -> Result<String> {
    if sk.is_empty() {
        bail!("not logged in");
    }
    let v = lastfm_call("user.getInfo", &[("sk", sk.to_string())], false, true)?;
    v.pointer("/user/name").and_then(|n| n.as_str()).map(String::from).ok_or_else(|| anyhow!("missing user"))
}

fn lastfm_track_params(t: &TrackMeta, sk: &str) -> Vec<(&'static str, String)> {
    let mut v = vec![("artist", t.artist.clone()), ("track", t.title.clone()), ("sk", sk.to_string())];
    if !t.album.is_empty() {
        v.push(("album", t.album.clone()));
    }
    if t.duration.as_secs() > 0 {
        v.push(("duration", t.duration.as_secs().to_string()));
    }
    v
}

/// Worker-thread check mirroring Go's `IsConfigured` (enabled, session key, valid user).
fn lastfm_configured(enabled: bool, sk: &str) -> bool {
    if !enabled || sk.is_empty() {
        return false;
    }
    match lastfm_self_name(sk) {
        Ok(_) => true,
        Err(e) => {
            log::error!("error while fetching user error={e:#}");
            false
        }
    }
}

pub fn lastfm_unconfigure() {
    settings::set_lastfm_token("");
}

// ---------------- dispatch ----------------

#[derive(Clone)]
struct Config {
    lb: bool,
    lb_url: String,
    lb_token: String,
    lfm_enabled: bool,
    lfm_sk: String,
}

fn config() -> Config {
    Config {
        lb: listenbrainz_configured(),
        lb_url: settings::listenbrainz_url(),
        lb_token: settings::listenbrainz_token(),
        lfm_enabled: settings::enable_lastfm(),
        lfm_sk: session_key(),
    }
}

fn now_playing(meta: TrackMeta) {
    let c = config();
    std::thread::spawn(move || {
        if c.lb {
            listenbrainz_send(&c.lb_url, &c.lb_token, listenbrainz_request(&meta, None));
        }
        if lastfm_configured(c.lfm_enabled, &c.lfm_sk)
            && let Err(e) = lastfm_call("track.updateNowPlaying", &lastfm_track_params(&meta, &c.lfm_sk), true, true)
        {
            log::error!("error while updating now playing error={e:#}");
        }
    });
}

fn scrobble(meta: TrackMeta, listened_at: i64) {
    let c = config();
    std::thread::spawn(move || {
        if c.lb {
            listenbrainz_send(&c.lb_url, &c.lb_token, listenbrainz_request(&meta, Some(listened_at)));
        }
        if lastfm_configured(c.lfm_enabled, &c.lfm_sk) {
            let mut p = lastfm_track_params(&meta, &c.lfm_sk);
            p.push(("timestamp", now_unix().to_string()));
            if let Err(e) = lastfm_call("track.scrobble", &p, true, true) {
                log::error!("error while scrobbling error={e:#}");
            }
        }
    });
}

/// Listening clock for one track.
pub struct Clock {
    meta: TrackMeta,
    started_at: i64,
    counter: Arc<Mutex<u64>>,
    scrobbled: bool,
    source: Option<glib::SourceId>,
}

thread_local! {
    static CLOCK: RefCell<Option<Clock>> = const { RefCell::new(None) };
}

fn clock_start() {
    CLOCK.with(|c| {
        let mut c = c.borrow_mut();
        let Some(clock) = c.as_mut() else { return };
        if clock.source.is_some() || clock.scrobbled {
            return;
        }
        let id = glib::timeout_add_seconds_local(1, || {
            let done = CLOCK.with(|c| {
                let mut c = c.borrow_mut();
                let Some(clock) = c.as_mut() else { return true };
                let mut n = clock.counter.lock().unwrap();
                *n += 1;
                if should_scrobble(*n, clock.meta.duration) {
                    clock.scrobbled = true;
                    clock.source = None;
                    scrobble(clock.meta.clone(), clock.started_at);
                    return true;
                }
                false
            });
            if done { glib::ControlFlow::Break } else { glib::ControlFlow::Continue }
        });
        clock.source = Some(id);
    });
}

fn clock_stop() {
    CLOCK.with(|c| {
        if let Some(clock) = c.borrow_mut().as_mut()
            && let Some(s) = clock.source.take()
        {
            s.remove();
        }
    });
}

pub fn init() {
    player::track_changed().on(|ev| {
        let Some(ev) = ev else { return false };
        let meta = TrackMeta::from_track(&ev.track);
        now_playing(meta.clone());
        clock_stop();
        CLOCK.with(|c| {
            *c.borrow_mut() = Some(Clock { meta, started_at: now_unix(), counter: Arc::new(Mutex::new(0)), scrobbled: false, source: None });
        });
        if player::playback_state_changed().get().status == PlaybackStatus::Playing {
            clock_start();
        }
        false
    });
    player::playback_state_changed().on(|ps| {
        if ps.status == PlaybackStatus::Playing {
            clock_start();
        } else {
            clock_stop();
        }
        false
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> TrackMeta {
        TrackMeta { title: "T".into(), artist: "A, B".into(), album: "Al".into(), url: "https://tidal.com/track/1".into(), duration: Duration::from_secs(200) }
    }

    #[test]
    fn threshold() {
        assert!(!should_scrobble(99, Duration::from_secs(200)));
        assert!(should_scrobble(100, Duration::from_secs(200)));
        assert!(should_scrobble(240, Duration::from_hours(1)));
        assert!(!should_scrobble(239, Duration::from_hours(1)));
    }

    #[test]
    fn listenbrainz_payload() {
        let v: serde_json::Value = serde_json::from_str(&listenbrainz_request(&meta(), None)).unwrap();
        assert_eq!(v["listen_type"], "playing_now");
        assert!(v["payload"][0].get("listened_at").is_none());
        assert_eq!(v["payload"][0]["track_metadata"]["artist_name"], "A, B");
        assert_eq!(v["payload"][0]["track_metadata"]["additional_info"]["duration_ms"], 200_000);
        let v: serde_json::Value = serde_json::from_str(&listenbrainz_request(&meta(), Some(5))).unwrap();
        assert_eq!(v["listen_type"], "single");
        assert_eq!(v["payload"][0]["listened_at"], 5);
    }

    #[test]
    fn lastfm_sig() {
        let mut p = BTreeMap::new();
        p.insert("method".to_string(), "auth.getToken".to_string());
        p.insert("api_key".to_string(), LASTFM_API_KEY.to_string());
        p.insert("format".to_string(), "json".to_string());
        let expected = hex(&Md5::digest(format!("api_key{LASTFM_API_KEY}methodauth.getToken{LASTFM_SECRET}").as_bytes()));
        assert_eq!(lastfm_signature(&p), expected);
    }
}
