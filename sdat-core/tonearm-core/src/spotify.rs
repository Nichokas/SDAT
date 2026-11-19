//! Reads a public Spotify playlist without an API key or a Spotify login.
//!
//! The playlist's embed page (`open.spotify.com/embed/playlist/<id>`) carries an anonymous
//! access token. Spotify's Web API rejects that token, but the web player's own endpoints
//! accept it: `spclient` lists every track of the playlist and gives each track's ISRC.
//! These endpoints are undocumented, so every decoder here tolerates missing fields.
//!
//! All functions are blocking.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::error::{bail, CoreError, CoreResult as Result};

const EMBED: &str = "https://open.spotify.com/embed/playlist";
const SPCLIENT: &str = "https://spclient.wg.spotify.com";
const WORKERS: usize = 6;
const ATTEMPTS: usize = 4;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);

/// A track of a Spotify playlist.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpotifyTrack {
    pub title: String,
    pub artists: Vec<String>,
    pub duration: Duration,
    /// Empty when Spotify has none.
    pub isrc: String,
}

/// A Spotify playlist with its tracks in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotifyPlaylist {
    pub name: String,
    pub tracks: Vec<SpotifyTrack>,
    /// Entries that are not catalogue tracks (local files, podcast episodes) or that Spotify no longer has.
    pub skipped: usize,
}

/// The playlist ID in a share link (`https://open.spotify.com/playlist/<id>?si=...`,
/// with or without an `intl-xx` prefix), a `spotify:playlist:<id>` URI or a bare ID.
pub fn playlist_id(link: &str) -> Option<String> {
    let link = link.trim();
    let id = if let Some(rest) = link.strip_prefix("spotify:playlist:") {
        rest
    } else if let Some((page, rest)) = link.split_once("/playlist/") {
        let page = page.strip_prefix("https://").or_else(|| page.strip_prefix("http://")).unwrap_or(page);
        if page.split('/').next() != Some("open.spotify.com") {
            return None;
        }
        rest.split(['?', '/', '#']).next()?
    } else {
        link
    };
    (id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| id.to_owned())
}

/// Reads the playlist `id`. `progress(done, total)` follows the track lookups;
/// `cancelled` is polled between them.
pub(crate) fn fetch_playlist(id: &str, progress: &(dyn Fn(usize, usize) + Sync), cancelled: &(dyn Fn() -> bool + Sync)) -> Result<SpotifyPlaylist> {
    let session = Session { playlist_id: id.to_owned(), token: Mutex::new(fetch_token(id)?), cancelled };
    let resp = session.get(&format!("{SPCLIENT}/playlist/v2/playlist/{id}"))?;
    if resp.status() != StatusCode::OK {
        return Err(unexpected(resp.status()));
    }
    let (name, entries) = parse_playlist(&resp.bytes().map_err(unreachable)?)?;
    let ids: Vec<&str> = entries.iter().filter_map(|uri| uri.strip_prefix("spotify:track:")).collect();

    let next = AtomicUsize::new(0);
    // Counted and reported under one lock, so the reports arrive in order.
    let done = Mutex::new(0);
    let stopped = AtomicBool::new(false);
    let found: Mutex<Vec<Option<SpotifyTrack>>> = Mutex::new(vec![None; ids.len()]);
    let failure: Mutex<Option<CoreError>> = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(ids.len()) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= ids.len() || lock(&failure).is_some() {
                    return;
                }
                if cancelled() {
                    stopped.store(true, Ordering::SeqCst);
                    return;
                }
                match session.track(ids[i]) {
                    Ok(track) => lock(&found)[i] = track,
                    Err(e) => {
                        lock(&failure).get_or_insert(e);
                        return;
                    }
                }
                let mut done = lock(&done);
                *done += 1;
                progress(*done, ids.len());
            });
        }
    });
    if stopped.load(Ordering::SeqCst) || cancelled() {
        return Err(CoreError::Cancelled("Spotify import cancelled".into()));
    }
    if let Some(e) = lock(&failure).take() {
        return Err(e);
    }
    let tracks: Vec<SpotifyTrack> = found.into_inner().unwrap_or_else(PoisonError::into_inner).into_iter().flatten().collect();
    Ok(SpotifyPlaylist { name, skipped: entries.len() - tracks.len(), tracks })
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn http() -> &'static Client {
    static C: OnceLock<Client> = OnceLock::new();
    C.get_or_init(|| // The embed page serves its data only to a browser user agent.
        crate::http::client_builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build http client"))
}

const NOT_FOUND: &str = "Spotify playlist not found. Make sure it is public.";

fn fetch_token(playlist_id: &str) -> Result<String> {
    let resp = crate::diag::net::send(http().get(format!("{EMBED}/{playlist_id}"))).map_err(unreachable)?;
    match resp.status() {
        StatusCode::OK => parse_embed_token(&resp.text().map_err(unreachable)?),
        StatusCode::NOT_FOUND => bail!("{NOT_FOUND}"),
        s => Err(unexpected(s)),
    }
}

/// The anonymous access token inside the embed page's `__NEXT_DATA__` JSON.
/// The page of a private or deleted playlist is an HTTP 200 that says 404 inside.
fn parse_embed_token(html: &str) -> Result<String> {
    let data = || {
        let (_, rest) = html.split_once(r#"id="__NEXT_DATA__""#)?;
        let json = rest.split_once('>')?.1.split_once("</script>")?.0;
        serde_json::from_str::<serde_json::Value>(json).ok()
    };
    let Some(page) = data().and_then(|mut v| v.pointer_mut("/props/pageProps").map(serde_json::Value::take)) else {
        bail!("Spotify answered in a way SDAT does not understand.");
    };
    if page["status"] == 404 {
        bail!("{NOT_FOUND}");
    }
    match page.pointer("/state/settings/session/accessToken").and_then(serde_json::Value::as_str) {
        Some(token) if !token.is_empty() => Ok(token.to_owned()),
        _ => bail!("Spotify answered in a way SDAT does not understand."),
    }
}

/// Spotify failures are `Other`, with a sentence the apps show as it is: the apps word
/// every other kind of error as a TIDAL failure.
fn unexpected(status: StatusCode) -> CoreError {
    CoreError::Other(format!("Spotify did not answer as expected (HTTP {}). Try again later.", status.as_u16()))
}

#[expect(clippy::needless_pass_by_value, reason = "the signature `map_err` calls")]
fn unreachable(e: reqwest::Error) -> CoreError {
    log::warn!("Spotify request failed: {e}");
    CoreError::Other("Can't reach Spotify. Check your connection and try again.".into())
}

/// Name and entry URIs of a `spclient` playlist answer.
fn parse_playlist(bytes: &[u8]) -> Result<(String, Vec<String>)> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Playlist {
        attributes: Attributes,
        contents: Contents,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Attributes {
        name: String,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Contents {
        items: Vec<Item>,
        truncated: bool,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Item {
        uri: String,
    }
    let p: Playlist = serde_json::from_slice(bytes).map_err(|e| CoreError::Other(format!("Spotify answered in a way SDAT does not understand: {e}")))?;
    if p.contents.truncated {
        bail!("Spotify returned only part of the playlist.");
    }
    Ok((p.attributes.name, p.contents.items.into_iter().map(|i| i.uri).collect()))
}

/// A `spclient` track metadata answer.
fn parse_track(bytes: &[u8]) -> Result<SpotifyTrack> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Track {
        name: String,
        artist: Vec<Named>,
        duration: u64,
        external_id: Vec<ExternalId>,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Named {
        name: String,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct ExternalId {
        r#type: String,
        id: String,
    }
    let t: Track = serde_json::from_slice(bytes).map_err(|e| CoreError::Other(format!("Spotify answered in a way SDAT does not understand: {e}")))?;
    Ok(SpotifyTrack {
        title: t.name,
        artists: t.artist.into_iter().map(|a| a.name).collect(),
        duration: Duration::from_millis(t.duration),
        isrc: t.external_id.into_iter().find(|e| e.r#type.eq_ignore_ascii_case("isrc")).map_or_default(|e| e.id.to_ascii_uppercase()),
    })
}

/// `spclient` names a track by the hex form of its base-62 ID.
fn hex_id(base62: &str) -> Option<String> {
    let mut n: u128 = 0;
    for b in base62.bytes() {
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'z' => b - b'a' + 10,
            b'A'..=b'Z' => b - b'A' + 36,
            _ => return None,
        };
        n = n.checked_mul(62)?.checked_add(u128::from(digit))?;
    }
    Some(format!("{n:032x}"))
}

struct Session<'a> {
    cancelled: &'a (dyn Fn() -> bool + Sync),
    playlist_id: String,
    token: Mutex<String>,
}

impl Session<'_> {
    /// GET with the anonymous token; waits out rate limits and replaces an expired token.
    fn get(&self, url: &str) -> Result<Response> {
        let mut attempt = 0;
        loop {
            if (self.cancelled)() {
                return Err(CoreError::Cancelled("Spotify import cancelled".into()));
            }
            attempt += 1;
            let token = lock(&self.token).clone();
            let resp = crate::diag::net::send(http().get(url).bearer_auth(&token).header("Accept", "application/json")).map_err(unreachable)?;
            match resp.status() {
                StatusCode::UNAUTHORIZED if attempt < ATTEMPTS => {
                    let mut current = lock(&self.token);
                    if *current == token {
                        *current = fetch_token(&self.playlist_id)?;
                    }
                }
                StatusCode::TOO_MANY_REQUESTS if attempt < ATTEMPTS => {
                    let wait = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()?.parse().ok()).map_or(Duration::from_secs(2), Duration::from_secs);
                    if wait > MAX_RETRY_AFTER {
                        bail!("Spotify is limiting requests from this network. Try again later.");
                    }
                    // In short steps, so that a cancel does not wait for Spotify.
                    for _ in 0..wait.as_secs() * 4 {
                        if (self.cancelled)() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
                _ => return Ok(resp),
            }
        }
    }

    /// `None` for a track Spotify no longer has.
    fn track(&self, id: &str) -> Result<Option<SpotifyTrack>> {
        let Some(hex) = hex_id(id) else { return Ok(None) };
        let resp = self.get(&format!("{SPCLIENT}/metadata/4/track/{hex}"))?;
        match resp.status() {
            StatusCode::OK => parse_track(&resp.bytes().map_err(unreachable)?).map(Some),
            StatusCode::NOT_FOUND => Ok(None),
            s => Err(unexpected(s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_playlist_id_from_links_uris_and_bare_ids() {
        let id = Some("37i9dQZF1DXcBWIGoYBM5M".to_owned());
        assert_eq!(playlist_id("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=abc123"), id);
        assert_eq!(playlist_id(" https://open.spotify.com/intl-es/playlist/37i9dQZF1DXcBWIGoYBM5M/ "), id);
        assert_eq!(playlist_id("open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"), id);
        assert_eq!(playlist_id("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"), id);
        assert_eq!(playlist_id("37i9dQZF1DXcBWIGoYBM5M"), id);
    }

    #[test]
    fn rejects_links_that_are_not_spotify_playlists() {
        assert_eq!(playlist_id("https://open.spotify.com/album/37i9dQZF1DXcBWIGoYBM5M"), None);
        assert_eq!(playlist_id("https://tidal.com/playlist/37i9dQZF1DXcBWIGoYBM5M"), None);
        assert_eq!(playlist_id("https://example.com/open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"), None);
        assert_eq!(playlist_id("https://open.spotify.com/playlist/short"), None);
        assert_eq!(playlist_id(""), None);
    }

    #[test]
    fn finds_the_token_in_the_embed_page() {
        let html = r#"<html><script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"state":{"settings":{"session":{"accessToken":"tok","isAnonymous":true}}}}}}</script></html>"#;
        assert_eq!(parse_embed_token(html).as_deref(), Ok("tok"));
        parse_embed_token("<html></html>").expect_err("no data");
    }

    #[test]
    fn a_private_playlist_is_reported_as_not_found() {
        let html = r#"<script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"status":404,"title":"Page not found"}}}</script>"#;
        assert_eq!(parse_embed_token(html), Err(CoreError::Other(NOT_FOUND.into())));
    }

    #[test]
    fn reads_playlist_entries_in_order() {
        let json = br#"{"length":3,"attributes":{"name":"Mix"},"contents":{"truncated":false,"items":[
            {"uri":"spotify:track:4rzfv0JLZfVhOhbSQ8o5jZ"},{"uri":"spotify:local:a:b:c:1"},{"uri":"spotify:track:5o3jMYOSbaVz3tkgwhELSV"}]}}"#;
        let (name, entries) = parse_playlist(json).expect("playlist");
        assert_eq!(name, "Mix");
        assert_eq!(entries, ["spotify:track:4rzfv0JLZfVhOhbSQ8o5jZ", "spotify:local:a:b:c:1", "spotify:track:5o3jMYOSbaVz3tkgwhELSV"]);
    }

    #[test]
    fn refuses_a_truncated_playlist() {
        parse_playlist(br#"{"contents":{"truncated":true,"items":[]}}"#).expect_err("truncated");
    }

    #[test]
    fn reads_track_metadata() {
        let json = br#"{"name":"BbY WOW","artist":[{"name":"KAROL G"},{"name":"Judeline"}],"duration":225834,"external_id":[{"type":"isrc","id":"usug12607940"}]}"#;
        assert_eq!(
            parse_track(json).expect("track"),
            SpotifyTrack { title: "BbY WOW".into(), artists: vec!["KAROL G".into(), "Judeline".into()], duration: Duration::from_millis(225_834), isrc: "USUG12607940".into() }
        );
        assert_eq!(parse_track(br#"{"name":"x"}"#).expect("track").isrc, "");
    }

    #[test]
    fn converts_base62_ids_to_hex() {
        assert_eq!(hex_id("3h5T5JypYU7huFiVYhv1dr").as_deref(), Some("6ba14a2a16c246be974cb5e5a2ad5e1d"));
        assert_eq!(hex_id("not/base62"), None);
    }
}
