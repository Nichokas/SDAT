//! TIDAL's account playback state over JSON:API (`installations`,
//! `userPlaybackStates`, `playQueues`), parsed into the types below.
//!
//! The spec (v1.10.254) marks these endpoints `INTERNAL`. Observed with
//! SDAT's sign-in on 2026-11-15, beyond what the spec says:
//! - `clientProvidedInstallationId` must be a UUID. `POST /installations` with
//!   a known one returns the existing record (201) and keeps its old name.
//! - An attachment (`availablePlayers`) lapses about 90 s after the last
//!   attach. When the active player's lapses, `activePlayer` reads `null`
//!   and the status turns `PAUSED`; the queue and its position stay.
//! - `position` is an ISO 8601 duration with a fraction (`PT1M23.456S`);
//!   writing a bare number is rejected. Writing `current` without a
//!   position clears it.
//! - Event streams (`pulsar.tidal.com`) are server-sent events: a
//!   `SUBSCRIBE_SUCCESS` line, then `data: CHANGED` per change and
//!   `:keepalive` comments. The state's stream fires on status, player and
//!   queue selection changes; queue edits fire only on the queue's stream.
//! - `future` comes in pages of 20; `include` takes at most 10 paths.
//! - Setting `shuffle` reshuffles the queue on the server, so SDAT always
//!   writes the order it plays with shuffle off.

use reqwest::Method;
use serde_json::{json, Value};

use super::{Device, Repeat};
use crate::api::{self, openapi};
use crate::error::{CoreError, CoreResult as Result};
use crate::model::{track_from_openapi, Track};

/// Items in the atomic queue write, then per append.
const FIRST_CHUNK: usize = 50;
const CHUNK: usize = 20;
/// What a track needs for display, below `current.` or `future.`.
const TRACK_INCLUDES: &str = "artists,albums.coverArt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Status {
    #[default]
    Idle,
    Playing,
    Paused,
}

/// `userPlaybackStates/me`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct State {
    pub status: Status,
    pub active: Option<String>,
    pub players: Vec<Device>,
    pub queue_id: Option<String>,
    pub stream: Option<String>,
}

/// One occurrence in a queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub track_id: String,
    pub item_id: String,
}

/// The selected queue without its future.
#[derive(Debug, Clone, Default)]
pub(super) struct Head {
    pub revision: String,
    pub repeat: Repeat,
    pub current: Option<Entry>,
    pub position_ms: Option<i64>,
    /// Server time of the position write, Unix ms.
    pub position_at_ms: Option<i64>,
    pub updated_by: Option<String>,
    pub track: Option<Track>,
    pub stream: Option<String>,
}

fn call(method: Method, path: &str, body: Option<&Value>) -> Result<Value> {
    api::openapi_call(method, path, body)
}

fn str_at(v: &Value, ptr: &str) -> Option<String> {
    v.pointer(ptr).and_then(Value::as_str).map(str::to_string)
}

fn revision(v: &Value) -> String {
    str_at(v, "/meta/revision").or_else(|| str_at(v, "/data/attributes/revision")).unwrap_or_default()
}

fn included<'a>(v: &'a Value, type_: &'a str) -> impl Iterator<Item = &'a Value> {
    v.get("included").and_then(Value::as_array).into_iter().flatten().filter(move |o| o.get("type").and_then(Value::as_str) == Some(type_))
}

fn stream_href(v: &Value) -> Option<String> {
    included(v, "changeEventStreams").find_map(|o| str_at(o, "/attributes/eventStreamLink/href"))
}

fn installation(v: &Value) -> Option<Device> {
    Some(Device { id: str_at(v, "/id")?, name: str_at(v, "/attributes/name").unwrap_or_default() })
}

fn entry(v: &Value) -> Option<Entry> {
    Some(Entry { track_id: str_at(v, "/id")?, item_id: str_at(v, "/meta/itemId")? })
}

pub(super) fn parse_state(v: &Value) -> State {
    let d = &v["data"];
    let status = match d.pointer("/attributes/playbackStatus").and_then(Value::as_str) {
        Some("PLAYING") => Status::Playing,
        Some("PAUSED") => Status::Paused,
        _ => Status::Idle,
    };
    let names: Vec<Device> = included(v, "installations").filter_map(installation).collect();
    let players = d
        .pointer("/relationships/availablePlayers/data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| str_at(p, "/id"))
        .map(|id| names.iter().find(|n| n.id == id).cloned().unwrap_or(Device { id, name: String::new() }))
        .collect();
    State {
        status,
        active: str_at(d, "/relationships/activePlayer/data/id"),
        players,
        queue_id: str_at(d, "/relationships/playQueue/data/id"),
        stream: stream_href(v),
    }
}

fn parse_repeat(s: Option<&str>) -> Repeat {
    match s {
        Some("ONE") => Repeat::One,
        Some("BATCH") => Repeat::All,
        _ => Repeat::Off,
    }
}

fn repeat_str(r: Repeat) -> &'static str {
    match r {
        Repeat::Off => "NONE",
        Repeat::One => "ONE",
        Repeat::All => "BATCH",
    }
}

fn ms_of_iso(s: &str) -> Option<i64> {
    openapi::parse_iso8601_duration(s).map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

pub(super) fn iso_of_ms(ms: i64) -> String {
    let ms = ms.max(0);
    format!("PT{}.{:03}S", ms / 1000, ms % 1000)
}

fn unix_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|t| t.timestamp_millis())
}

/// Tracks of `rels` in order, from the response's included objects.
fn tracks(v: &Value, rels: &Value) -> Vec<Track> {
    let Ok(included) = openapi::from_value_lenient::<openapi::IncludedObjects>(v.get("included").cloned().unwrap_or(Value::Null)) else {
        return Vec::new();
    };
    let rels: Vec<openapi::Relationship> = openapi::from_value_lenient(rels.clone()).unwrap_or_default();
    included.tracks(&rels).iter().map(track_from_openapi).collect()
}

pub(super) fn parse_head(v: &Value) -> Head {
    let d = &v["data"];
    let cur = d.pointer("/relationships/current/data").filter(|c| !c.is_null());
    let meta = cur.and_then(|c| c.get("meta"));
    Head {
        revision: revision(v),
        repeat: parse_repeat(d.pointer("/attributes/repeat").and_then(Value::as_str)),
        current: cur.and_then(entry),
        position_ms: meta.and_then(|m| m.get("position")).and_then(Value::as_str).and_then(ms_of_iso),
        position_at_ms: meta.and_then(|m| m.get("positionUpdatedAt")).and_then(Value::as_str).and_then(unix_ms),
        updated_by: meta.and_then(|m| str_at(m, "/updatedByInstallationId")),
        track: cur.and_then(|c| tracks(v, &Value::Array(vec![c.clone()])).into_iter().next()),
        stream: stream_href(v),
    }
}

/// Registers this install (idempotent) and returns its installation id.
pub(super) fn register(key: &str, name: &str) -> Result<String> {
    let v = call(
        Method::POST,
        "/v2/installations",
        Some(&json!({"data": {"type": "installations", "attributes": {"clientProvidedInstallationId": key, "name": name}}})),
    )?;
    str_at(&v, "/data/id").ok_or_else(|| CoreError::Decode("installation without id".into()))
}

fn players_body(id: &str) -> Value {
    json!({"data": [{"type": "installations", "id": id}]})
}

pub(super) fn attach(id: &str) -> Result<()> {
    call(Method::POST, "/v2/userPlaybackStates/me/relationships/availablePlayers", Some(&players_body(id))).map(drop)
}

/// `token`: taken while still signed in, as a sign-out removes the credentials right away.
pub(super) fn detach(id: &str, token: Option<&str>) -> Result<()> {
    let (path, body) = ("/v2/userPlaybackStates/me/relationships/availablePlayers", players_body(id));
    match token {
        Some(t) => api::openapi_call_as(t, Method::DELETE, path, Some(&body)),
        None => call(Method::DELETE, path, Some(&body)),
    }
    .map(drop)
}

pub(super) fn state() -> Result<State> {
    call(Method::GET, "/v2/userPlaybackStates/me?include=activePlayer,availablePlayers,changeEventStream&includeLinkage=playQueue", None).map(|v| parse_state(&v))
}

pub(super) fn head(queue: &str) -> Result<Head> {
    let inc = TRACK_INCLUDES.split(',').map(|i| format!("current.{i}")).collect::<Vec<_>>().join(",");
    call(Method::GET, &format!("/v2/playQueues/{queue}?include=changeEventStream,{inc}"), None).map(|v| parse_head(&v))
}

/// [`head`] for showing or playing the current track: the included
/// metadata can be missing while the queue still names the track.
pub(super) fn head_with_track(queue: &str) -> Result<Head> {
    let mut h = head(queue)?;
    if h.track.is_none()
        && let Some(c) = &h.current
    {
        h.track = crate::service::get_track(&c.track_id).ok();
    }
    Ok(h)
}

pub(super) fn installation_name(id: &str) -> Result<String> {
    call(Method::GET, &format!("/v2/installations/{id}"), None).map(|v| str_at(&v, "/data/attributes/name").unwrap_or_default())
}

/// Future pages of a queue, up to `cap` entries; with tracks when `with_tracks`.
fn future(queue: &str, cap: usize, with_tracks: bool) -> Result<(Vec<Entry>, Vec<Track>)> {
    let mut path = format!("/v2/playQueues/{queue}/relationships/future");
    if with_tracks {
        let inc = TRACK_INCLUDES.split(',').map(|i| format!("future.{i}")).collect::<Vec<_>>().join(",");
        path = format!("{path}?include={inc}");
    }
    let (mut entries, mut out) = (Vec::new(), Vec::new());
    loop {
        let v = call(Method::GET, &path, None)?;
        let data = v.get("data").cloned().unwrap_or(Value::Null);
        entries.extend(data.as_array().into_iter().flatten().filter_map(entry));
        if with_tracks {
            out.extend(tracks(&v, &data));
        }
        match str_at(&v, "/links/next") {
            Some(next) if entries.len() < cap => path = format!("/v2{next}"),
            _ => break,
        }
    }
    entries.truncate(cap);
    out.truncate(cap);
    Ok((entries, out))
}

/// Item ids of the future, after a write.
pub(super) fn future_entries(queue: &str, cap: usize) -> Result<Vec<Entry>> {
    future(queue, cap, false).map(|f| f.0)
}

/// Future tracks, for a resume.
pub(super) fn future_tracks(queue: &str, cap: usize) -> Result<(Vec<Entry>, Vec<Track>)> {
    future(queue, cap, true)
}

pub(super) fn create_queue() -> Result<String> {
    let v = call(Method::POST, "/v2/playQueues", Some(&json!({"data": {"type": "playQueues"}})))?;
    let id = str_at(&v, "/data/id").ok_or_else(|| CoreError::Decode("queue without id".into()))?;
    call(Method::PATCH, "/v2/userPlaybackStates/me/relationships/playQueue", Some(&json!({"data": {"type": "playQueues", "id": id}})))?;
    Ok(id)
}

/// Makes `id` the active player; returns who TIDAL says is active now.
pub(super) fn set_active(id: &str) -> Result<Option<String>> {
    let v = call(Method::PATCH, "/v2/userPlaybackStates/me/relationships/activePlayer", Some(&json!({"data": {"type": "installations", "id": id}})))?;
    Ok(str_at(&v, "/data/id"))
}

pub(super) fn set_status(playing: bool) -> Result<()> {
    let s = if playing { "PLAYING" } else { "PAUSED" };
    call(Method::PATCH, "/v2/userPlaybackStates/me", Some(&json!({"data": {"type": "userPlaybackStates", "id": "me", "attributes": {"playbackStatus": s}}})))
        .map(drop)
}

fn track_ids(ids: &[String]) -> Value {
    Value::Array(ids.iter().map(|id| json!({"type": "tracks", "id": id})).collect())
}

/// Replaces the queue with `tracks` (the first becomes current); returns the revision.
pub(super) fn rewrite(queue: &str, tracks: &[String], repeat: Repeat) -> Result<String> {
    let (first, rest) = tracks.split_at(tracks.len().min(FIRST_CHUNK));
    let body = json!({"data": {"type": "playQueues", "id": queue,
        "attributes": {"shuffle": "OFF", "shuffled": false, "repeat": repeat_str(repeat)},
        "relationships": {
            "future": {"data": track_ids(first), "meta": {"mode": "REPLACE_ALL"}},
            "current": {"meta": {"mode": "FIRST_FUTURE_ITEM"}}}}});
    let mut rev = revision(&call(Method::PATCH, &format!("/v2/playQueues/{queue}"), Some(&body))?);
    for chunk in rest.chunks(CHUNK) {
        let body = json!({"data": track_ids(chunk), "meta": {"mode": "ADD_TO_BACK"}});
        rev = revision(&call(Method::POST, &format!("/v2/playQueues/{queue}/relationships/future"), Some(&body))?);
    }
    Ok(rev)
}

pub(super) fn set_repeat(queue: &str, repeat: Repeat) -> Result<String> {
    let body = json!({"data": {"type": "playQueues", "id": queue, "attributes": {"repeat": repeat_str(repeat)}}});
    call(Method::PATCH, &format!("/v2/playQueues/{queue}"), Some(&body)).map(|v| revision(&v))
}

/// Selects `e` as current at `position_ms`; returns the revision.
pub(super) fn set_current(queue: &str, e: &Entry, position_ms: i64, me: &str) -> Result<String> {
    let body = json!({"data": {"type": "tracks", "id": e.track_id,
        "meta": {"itemId": e.item_id, "position": iso_of_ms(position_ms), "updatedByInstallationId": me}}});
    call(Method::PATCH, &format!("/v2/playQueues/{queue}/relationships/current"), Some(&body)).map(|v| revision(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE: &str = r#"{"data":{"id":"Hn4","type":"userPlaybackStates","attributes":{"playbackStatus":"PLAYING"},
        "relationships":{"availablePlayers":{"data":[{"id":"a","type":"installations"},{"id":"b","type":"installations"}]},
        "changeEventStream":{"data":{"id":"s","type":"changeEventStreams"}},
        "activePlayer":{"data":{"id":"b","type":"installations"}},
        "playQueue":{"data":{"id":"q1","type":"playQueues"}}}},
        "included":[{"id":"a","type":"installations","attributes":{"clientProvidedInstallationId":"u1","name":"Fedora desktop"}},
        {"id":"b","type":"installations","attributes":{"clientProvidedInstallationId":"u2","name":"iPhone"}},
        {"id":"s","type":"changeEventStreams","attributes":{"eventStreamLink":{"href":"https://pulsar.tidal.com/userPlaybackStates/Hn4"}}}]}"#;

    #[test]
    fn parses_the_state() {
        let s = parse_state(&serde_json::from_str(STATE).expect("json"));
        assert_eq!(s.status, Status::Playing);
        assert_eq!(s.active.as_deref(), Some("b"));
        assert_eq!(s.players, vec![Device { id: "a".into(), name: "Fedora desktop".into() }, Device { id: "b".into(), name: "iPhone".into() }]);
        assert_eq!(s.queue_id.as_deref(), Some("q1"));
        assert_eq!(s.stream.as_deref(), Some("https://pulsar.tidal.com/userPlaybackStates/Hn4"));
    }

    #[test]
    fn parses_an_idle_state() {
        let s = parse_state(&serde_json::from_str(r#"{"data":{"id":"x","type":"userPlaybackStates","attributes":{"playbackStatus":"IDLE"},
            "relationships":{"availablePlayers":{"data":[]},"activePlayer":{"data":null},"playQueue":{"data":null}}}}"#).expect("json"));
        assert_eq!(s, State::default());
    }

    const HEAD: &str = r#"{"data":{"id":"q1","type":"playQueues","attributes":{"revision":"r1","shuffle":"OFF","shuffled":false,"repeat":"BATCH"},
        "relationships":{"current":{"data":{"id":"40475829","type":"tracks","meta":{"itemId":"7706807","batchId":"b",
            "position":"PT1M23.456S","positionRevision":1,"positionUpdatedAt":"2026-11-15T12:38:59.242935Z","updatedByInstallationId":"b"}}},
        "changeEventStream":{"data":{"id":"s","type":"changeEventStreams"}}}},
        "included":[{"id":"40475829","type":"tracks","attributes":{"title":"Song","duration":"PT3M15S"},
            "relationships":{"albums":{"data":[{"id":"9","type":"albums"}]},"artists":{"data":[{"id":"7","type":"artists"}]}}},
        {"id":"9","type":"albums","attributes":{"title":"Album"}},{"id":"7","type":"artists","attributes":{"name":"Artist"}},
        {"id":"s","type":"changeEventStreams","attributes":{"eventStreamLink":{"href":"https://pulsar.tidal.com/playQueues/q1"}}}]}"#;

    #[test]
    fn parses_the_head() {
        let h = parse_head(&serde_json::from_str(HEAD).expect("json"));
        assert_eq!(h.revision, "r1");
        assert_eq!(h.repeat, Repeat::All);
        assert_eq!(h.current, Some(Entry { track_id: "40475829".into(), item_id: "7706807".into() }));
        assert_eq!(h.position_ms, Some(83_456));
        assert_eq!(h.position_at_ms, Some(1_794_746_339_242));
        assert_eq!(h.updated_by.as_deref(), Some("b"));
        let t = h.track.expect("track");
        assert_eq!((t.title(), t.duration().as_secs()), ("Song", 195));
        assert_eq!(t.album.map(|a| a.title).as_deref(), Some("Album"));
        assert_eq!(t.artists.first().map(|a| a.name.as_str()), Some("Artist"));
        assert_eq!(h.stream.as_deref(), Some("https://pulsar.tidal.com/playQueues/q1"));
    }

    #[test]
    fn a_cleared_position_is_none() {
        let h = parse_head(&serde_json::from_str(r#"{"data":{"id":"q","type":"playQueues","attributes":{"revision":"r","repeat":"NONE"},
            "relationships":{"current":{"data":{"id":"1","type":"tracks","meta":{"itemId":"5"}}}}}}"#).expect("json"));
        assert_eq!((h.position_ms, h.position_at_ms, h.updated_by), (None, None, None));
        assert!(h.track.is_none());
        let empty = parse_head(&serde_json::from_str(r#"{"data":{"id":"q","type":"playQueues","attributes":{"revision":"r"},"relationships":{"current":{"data":null}}}}"#).expect("json"));
        assert!(empty.current.is_none());
    }

    #[test]
    fn positions_round_trip() {
        assert_eq!(iso_of_ms(83_456), "PT83.456S");
        assert_eq!(iso_of_ms(-5), "PT0.000S");
        assert_eq!(ms_of_iso("PT1M23.456S"), Some(83_456));
        assert_eq!(ms_of_iso(&iso_of_ms(7)), Some(7));
    }
}
