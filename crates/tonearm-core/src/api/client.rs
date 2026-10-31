//! HTTP client for TIDAL (port of `pkg/tidalapi`).
//!
//! Middleware (headers, base
//! URL, default query params), auth strategies (refresh token and `OpenAPI`
//! client credentials), device linking and every endpoint Tonearm uses.
//!
//! All functions are blocking; UI code runs them on a worker thread.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::error::{bail, bail_http, Context, CoreError, CoreResult as Result};
use base64::Engine;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::{legacy, openapi};

pub const CLIENT_VERSION: &str = "2025.12.13";
pub const UNAUTHENTICATED_CLIENT_ID: &str = "txNoH4kkV41MfH25";

fn double_b64(parts: [&str; 2]) -> String {
    let e = &base64::engine::general_purpose::STANDARD;
    let mut first = Vec::new();
    for p in parts {
        first.extend(e.decode(p).unwrap_or_default());
    }
    String::from_utf8(e.decode(first).unwrap_or_default()).unwrap_or_default()
}

pub fn client_id() -> &'static str {
    static V: OnceLock<String> = OnceLock::new();
    V.get_or_init(|| double_b64(["WmxneVNuaGtiVzUw", "V2xkTE1HbDRWQT09"]))
}

pub fn client_secret() -> &'static str {
    static V: OnceLock<String> = OnceLock::new();
    V.get_or_init(|| double_b64(["TVU1dU9VRm1SRUZxZUhKblNrWktZa3RPVjB4bFFY", "bExSMVpIYlVsT2RWaFFVRXhJVmxoQmRuaEJaejA9"]))
}

fn http() -> &'static Client {
    static C: OnceLock<Client> = OnceLock::new();
    C.get_or_init(|| // Go's net/http sends this UA by default; TIDAL's CDN rejects requests without one.
        Client::builder().user_agent("Go-http-client/1.1").build().expect("failed to build http client"))
}

#[derive(Default)]
struct CachedToken {
    token: String,
    expiry: Option<Instant>,
}

impl CachedToken {
    fn valid(&self) -> Option<String> {
        match self.expiry {
            Some(e) if !self.token.is_empty() && Instant::now() < e => Some(self.token.clone()),
            _ => None,
        }
    }
}

static USER_TOKEN: Mutex<CachedToken> = Mutex::new(CachedToken { token: String::new(), expiry: None });
static OPENAPI_TOKEN: Mutex<CachedToken> = Mutex::new(CachedToken { token: String::new(), expiry: None });
static COUNTRY: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Forget the cached user access token (after sign-in/out).
pub fn reset_user_token() {
    *USER_TOKEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = CachedToken::default();
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
    expires_in: u64,
}

fn token_request(form: &[(&str, &str)]) -> Result<Response> {
    Ok(http()
        .post(format!("{}/v1/oauth2/token", crate::config::get().auth_base_url()))
        .form(form)
        .send()?)
}

fn fetch_access_token(cache: &Mutex<CachedToken>, form: &[(&str, &str)]) -> Result<String> {
    // Release the lock before the (slow) token request.
    let cached = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner).valid();
    if let Some(t) = cached {
        return Ok(t);
    }
    let resp = token_request(form)?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "failed to get token: {}", resp.status());
    }
    let t: AccessToken = resp.json()?;
    *cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = CachedToken { token: t.access_token.clone(), expiry: Some(Instant::now() + Duration::from_secs(t.expires_in)) };
    Ok(t.access_token)
}

/// secrets.AuthStrategy: user's refresh token.
fn user_access_token() -> Result<Option<String>> {
    if !crate::auth::tokens().has_refresh_token() {
        return Ok(None);
    }
    let refresh = crate::auth::tokens().get_refresh_token();
    fetch_access_token(
        &USER_TOKEN,
        &[("client_id", client_id()), ("client_secret", client_secret()), ("grant_type", "refresh_token"), ("refresh_token", &refresh)],
    )
    .map(Some)
}

/// auth.OpenAPIClientAuthStrategy: client credentials for openapi.tidal.com.
fn openapi_access_token() -> Result<String> {
    fetch_access_token(&OPENAPI_TOKEN, &[("client_id", client_id()), ("client_secret", client_secret()), ("grant_type", "client_credentials")])
}

pub fn fetch_country_code() -> Result<String> {
    #[derive(Deserialize)]
    struct R {
        #[serde(rename = "countryCode")]
        country_code: String,
    }
    let resp = http()
        .get(format!("{}/v1/country/context?countryCode=WW&locale=en_US&deviceType=BROWSER", crate::config::get().api_base_url()))
        .header("x-tidal-token", client_id())
        .send()?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
    }
    Ok(resp.json::<R>()?.country_code)
}

/// The account's own country from its session (authenticated, read-only).
fn session_country_code() -> Result<String> {
    #[derive(Deserialize)]
    struct R {
        #[serde(rename = "countryCode")]
        country_code: String,
    }
    let Some(token) = user_access_token()? else { return Err(CoreError::NotSignedIn) };
    let resp = http()
        .get(format!("{}/v1/sessions", crate::config::get().api_base_url()))
        .header("x-tidal-token", client_id())
        .header("Authorization", format!("Bearer {token}"))
        .send()?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
    }
    Ok(resp.json::<R>()?.country_code)
}

/// Country used for requests: the user's preference, else the account's session
/// country, else the IP-based context (Go used only the latter, which can be wrong).
pub fn country_code() -> String {
    if let Some(c) = crate::country::preferred() {
        return c;
    }
    let mut g = COUNTRY.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    g.get_or_insert_with(|| {
        let r = session_country_code().or_else(|e| {
            log::debug!("session country unavailable: {e:#}");
            fetch_country_code()
        });
        match r {
            Ok(c) => {
                log::info!("Discovered country code countryCode={c}");
                c
            }
            Err(e) => {
                log::error!("Failed to fetch country code, defaulting to WW: {e}");
                "WW".into()
            }
        }
    })
    .clone()
}

/// Forgets the discovered country (e.g. after login/logout or a preference change).
pub fn reset_country_code() {
    *COUNTRY.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

fn utc_offset() -> String {
    let secs = chrono::Local::now().offset().local_minus_utc();
    let sign = if secs < 0 { '-' } else { '+' };
    let s = secs.abs();
    format!("{}{:02}:{:02}", sign, s / 3600, (s % 3600) / 60)
}

fn locale() -> String {
    let l = crate::i18n::user_locale();
    if l.is_empty() {
        "en_US".into()
    } else {
        l
    }
}

/// Builds a request with the same middleware as Go's `MiddlewareRoundTripper`.
fn request(method: Method, path: &str, query: &[(&str, String)]) -> Result<RequestBuilder> {
    let url = if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        format!("{}{}", crate::config::get().api_base_url(), path)
    };
    let mut url = reqwest::Url::parse(&url).with_context(|| format!("invalid url {url}"))?;
    {
        let existing: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(k, _)| !matches!(k.as_ref(), "deviceType" | "platform" | "countryCode" | "timeOffset" | "locale"))
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let mut q = url.query_pairs_mut();
        q.clear();
        for (k, v) in &existing {
            q.append_pair(k, v);
        }
        for (k, v) in query {
            q.append_pair(k, v);
        }
        q.append_pair("deviceType", "BROWSER");
        q.append_pair("platform", "WEB");
        q.append_pair("countryCode", &country_code());
        q.append_pair("timeOffset", &utc_offset());
        q.append_pair("locale", &locale())
    };
    let mut auth = user_access_token()?;
    if auth.is_none() && url.as_str().contains(&crate::config::get().openapi_base_url()) {
        auth = Some(openapi_access_token()?);
    }
    let mut rb = http()
        .request(method, url)
        .header("x-tidal-client-version", CLIENT_VERSION)
        .header("User-Agent", format!("Tonearm/{CLIENT_VERSION} Mozilla/5.0 (Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0"));
    match auth {
        Some(t) => rb = rb.header("x-tidal-token", client_id()).header("Authorization", format!("Bearer {t}")),
        None => rb = rb.header("x-tidal-token", UNAUTHENTICATED_CLIENT_ID),
    }
    Ok(rb)
}

fn get_json<T: DeserializeOwned>(path: &str, query: &[(&str, String)]) -> Result<T> {
    let resp = request(Method::GET, path, query)?.send()?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
    }
    let bytes = resp.bytes()?;
    openapi::from_slice_lenient(&bytes).with_context(|| format!("decoding {path}"))
}

fn include_q(include: &[&str]) -> Vec<(&'static str, String)> {
    if include.is_empty() {
        vec![]
    } else {
        vec![("include", include.join(","))]
    }
}

fn openapi(path: &str) -> String {
    format!("{}{}", crate::config::get().openapi_base_url(), path)
}

// ---------------- OpenAPI v2 ----------------

pub fn album(id: &str, include: &[&str]) -> Result<openapi::Album> {
    get_json(&openapi(&format!("/v2/albums/{id}")), &include_q(include))
}
pub fn artist(id: &str, include: &[&str]) -> Result<openapi::Artist> {
    get_json(&openapi(&format!("/v2/artists/{id}")), &include_q(include))
}
pub fn playlist(id: &str, include: &[&str]) -> Result<openapi::Playlist> {
    get_json(&openapi(&format!("/v2/playlists/{id}")), &include_q(include))
}
pub fn track(id: &str, include: &[&str]) -> Result<openapi::Track> {
    get_json(&openapi(&format!("/v2/tracks/{id}")), &include_q(include))
}

/// GET of a paginated relationship (`.../relationships/<x>`).
pub fn relationship(path: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    let mut q = Vec::new();
    if !cursor.is_empty() {
        q.push(("page[cursor]", cursor.to_string()));
    }
    q.extend(include_q(include));
    get_json(&openapi(path), &q)
}

pub fn album_items(id: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    relationship(&format!("/v2/albums/{id}/relationships/items"), cursor, include)
}
pub fn playlist_items(id: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    relationship(&format!("/v2/playlists/{id}/relationships/items"), cursor, include)
}
pub fn track_radio(id: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    relationship(&format!("/v2/tracks/{id}/relationships/radio"), cursor, include)
}
pub fn user_collection_items(kind: &str, user_id: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    // kind: Tracks | Albums | Artists | Playlists
    relationship(&format!("/v2/userCollection{kind}/{user_id}/relationships/items"), cursor, include)
}

/// Like [`user_collection_items`] with an explicit `sort` (e.g. `-addedAt`).
pub fn user_collection_items_sorted(kind: &str, user_id: &str, cursor: &str, include: &[&str], sort: &str) -> Result<openapi::Relationships> {
    let mut q = vec![("sort", sort.to_string())];
    if !cursor.is_empty() {
        q.push(("page[cursor]", cursor.to_string()));
    }
    q.extend(include_q(include));
    get_json(&openapi(&format!("/v2/userCollection{kind}/{user_id}/relationships/items")), &q)
}

fn query_escape(q: &str) -> String {
    // Go's url.QueryEscape (spaces become '+')
    urlencoding::encode(q).replace("%20", "+")
}

pub fn search(query: &str, include: &[&str]) -> Result<openapi::SearchResultSingle> {
    let mut q = vec![("filter[query]", query.to_string())];
    q.push(("include", include.join(",")));
    // The endpoint returns `data` as an array (Go: Response[[]SearchResultData]); use its first entry.
    let r: openapi::SearchResult = get_json(&openapi("/v2/searchResults"), &q)?;
    Ok(openapi::first_search_result(r))
}
pub fn search_relationship(query: &str, kind: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    relationship(&format!("/v2/searchResults/{}/relationships/{kind}", query_escape(query)), cursor, include)
}

/// Query for `GET /v2/trackManifests/{id}`.
pub fn manifest_query(formats: &[&str], manifest_type: &str) -> Vec<(&'static str, String)> {
    vec![
        ("adaptive", "false".into()),
        ("formats", formats.join(",")),
        ("manifestType", manifest_type.into()),
        ("uriScheme", "HTTPS".into()),
        ("usage", "PLAYBACK".into()),
    ]
}

/// MPEG-DASH manifest (desktop, `GStreamer`).
pub fn track_manifest(id: &str, formats: &[&str]) -> Result<openapi::TrackManifest> {
    get_json(&openapi(&format!("/v2/trackManifests/{id}")), &manifest_query(formats, openapi::MANIFEST_TYPE_MPEG_DASH))
}

/// HLS manifest: `attributes.uri` is a master playlist with unencrypted fMP4
/// segments, playable by `AVPlayer` for AAC, FLAC and `FLAC_HIRES`.
pub fn track_manifest_hls(id: &str, formats: &[&str]) -> Result<openapi::TrackManifest> {
    get_json(&openapi(&format!("/v2/trackManifests/{id}")), &manifest_query(formats, openapi::MANIFEST_TYPE_HLS))
}

// ---------------- v1 ----------------

pub fn page_v1(path: &str) -> Result<legacy::Page> {
    get_json(&format!("/v1/pages/{path}"), &[])
}

/// Returns (list, etag) or None when not modified.
pub fn favourite_ids(user_id: &str, if_none_match: &str) -> Result<Option<(legacy::FavouritesIdLists, String)>> {
    let mut rb = request(Method::GET, &format!("/v1/users/{user_id}/favorites/ids"), &[])?;
    if !if_none_match.is_empty() {
        rb = rb.header("If-None-Match", if_none_match);
    }
    let resp = rb.send()?;
    if resp.status() == StatusCode::NOT_MODIFIED {
        return Ok(None);
    }
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
    }
    let etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    Ok(Some((openapi::from_slice_lenient(&resp.bytes()?)?, etag)))
}

/// kind: tracks | albums | playlists | artists
/// Drops cached lists that a favourites change affects.
fn invalidate_favourites(kind: &str) {
    if kind == "tracks" {
        crate::cache::invalidate(crate::cache::FAVOURITE_TRACKS);
    }
}

pub fn favourite_add(user_id: &str, kind: &str, id: &str) -> Result<()> {
    invalidate_favourites(kind);
    let field = match kind {
        "tracks" => "trackIds",
        "albums" => "albumIds",
        "playlists" => "uuids",
        _ => "artistIds",
    };
    let resp = request(Method::POST, &format!("/v1/users/{user_id}/favorites/{kind}"), &[])?
        .form(&[(field, id), ("onArtifactNotFound", "FAIL")])
        .send()?;
    if resp.status() != StatusCode::OK {
        let t = resp.text().unwrap_or_default();
        bail!("error while adding {} to favourites: {t}", kind.trim_end_matches('s'));
    }
    Ok(())
}

pub fn favourite_remove(user_id: &str, kind: &str, id: &str) -> Result<()> {
    invalidate_favourites(kind);
    let resp = request(Method::DELETE, &format!("/v1/users/{user_id}/favorites/{kind}/{id}"), &[])?.send()?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "error while removing from favourites: {}", resp.status());
    }
    Ok(())
}

/// One of the signed-in user's own (editable) playlists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnPlaylist {
    pub id: String,
    pub name: String,
    pub number_of_items: i64,
}

/// Turns a `GET /v2/playlists?filter[owners.id]=me` page into playlists plus the next cursor.
pub fn parse_own_playlists(bytes: &[u8]) -> Result<(Vec<OwnPlaylist>, Option<String>)> {
    let r: openapi::Response<Vec<openapi::PlaylistData>> = openapi::from_slice_lenient(bytes).context("decoding playlists")?;
    let list = r
        .data
        .into_iter()
        .filter(|p| p.type_ == "playlists" && !p.id.is_empty())
        .map(|p| OwnPlaylist { id: p.id, name: p.attributes.name, number_of_items: p.attributes.number_of_items })
        .collect();
    let next = r.links.meta.and_then(|m| m.next_cursor).filter(|c| !c.is_empty());
    Ok((list, next))
}

/// Lists the playlists owned by the signed-in user (read-only).
pub fn own_playlists() -> Result<Vec<OwnPlaylist>> {
    const MAX_PAGES: usize = 20;
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut q = vec![("filter[owners.id]", "me".to_owned())];
        if let Some(c) = cursor.take() {
            q.push(("page[cursor]", c));
        }
        let resp = request(Method::GET, &openapi("/v2/playlists"), &q)?.send()?;
        if resp.status() != StatusCode::OK {
            bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
        }
        let (page, next) = parse_own_playlists(&resp.bytes()?)?;
        out.extend(page);
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    Ok(out)
}

/// JSON:API body for adding tracks to a playlist.
pub fn add_items_body(track_ids: &[&str]) -> serde_json::Value {
    serde_json::json!({ "data": track_ids.iter().map(|id| serde_json::json!({ "type": "tracks", "id": id })).collect::<Vec<_>>() })
}

/// Appends a track to one of the user's playlists.
pub fn playlist_add_track(playlist_id: &str, track_id: &str) -> Result<()> {
    let resp = request(Method::POST, &openapi(&format!("/v2/playlists/{playlist_id}/relationships/items")), &[])?
        .header("Content-Type", "application/vnd.api+json")
        .body(add_items_body(&[track_id]).to_string())
        .send()?;
    crate::cache::invalidate(&crate::cache::playlist_key(playlist_id));
    if !resp.status().is_success() {
        bail_http!(resp.status().as_u16(), "error while adding track to playlist: {}", resp.status().as_u16());
    }
    Ok(())
}

/// `meta.itemId` of the first entry of `track_id` in a page of playlist items.
pub fn playlist_item_id(items: &[openapi::Relationship], track_id: &str) -> Option<String> {
    items
        .iter()
        .filter(|r| r.id == track_id)
        .find_map(|r| r.meta.as_ref()?.get("itemId")?.as_str().map(str::to_string))
}

/// Removes the first occurrence of a track from one of the user's playlists.
pub fn playlist_remove_track(playlist_id: &str, track_id: &str) -> Result<()> {
    let mut cursor = String::new();
    let item_id = loop {
        let page = playlist_items(playlist_id, &cursor, &[])?;
        if let Some(id) = playlist_item_id(&page.data, track_id) {
            break id;
        }
        match page.links.meta.and_then(|m| m.next_cursor) {
            Some(c) if !c.is_empty() => cursor = c,
            _ => bail!("track not found in playlist"),
        }
    };
    let body = serde_json::json!({ "data": [{ "type": "tracks", "id": track_id, "meta": { "itemId": item_id } }] });
    let resp = request(Method::DELETE, &openapi(&format!("/v2/playlists/{playlist_id}/relationships/items")), &[])?
        .header("Content-Type", "application/vnd.api+json")
        .body(body.to_string())
        .send()?;
    crate::cache::invalidate(&crate::cache::playlist_key(playlist_id));
    if !resp.status().is_success() {
        bail_http!(resp.status().as_u16(), "error while removing track from playlist: {}", resp.status().as_u16());
    }
    Ok(())
}

#[cfg(test)]
mod playlist_tests {
    use super::*;

    #[test]
    fn finds_playlist_item_id() {
        let r: openapi::Relationships = serde_json::from_str(
            r#"{"data":[{"id":"1","type":"tracks","meta":{"itemId":"x1"}},{"id":"2","type":"tracks","meta":{"itemId":"x2"}}],"links":{}}"#,
        )
        .expect("json");
        assert_eq!(playlist_item_id(&r.data, "2").as_deref(), Some("x2"));
        assert_eq!(playlist_item_id(&r.data, "3"), None);
    }

    #[test]
    fn parses_owned_playlists_and_cursor() {
        let json = br#"{"data":[
            {"id":"aaaa-1","type":"playlists","attributes":{"name":"Road trip","numberOfItems":12,"accessType":"PUBLIC","description":null}},
            {"id":"bbbb-2","type":"playlists","attributes":{"name":"Focus","numberOfItems":0}}],
            "links":{"self":"/playlists?filter[owners.id]=me","next":"/playlists?page[cursor]=xyz","meta":{"nextCursor":"xyz"}}}"#;
        let (list, next) = parse_own_playlists(json).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], OwnPlaylist { id: "aaaa-1".into(), name: "Road trip".into(), number_of_items: 12 });
        assert_eq!(list[1].number_of_items, 0);
        assert_eq!(next.as_deref(), Some("xyz"));
    }

    #[test]
    fn last_page_has_no_cursor() {
        let (list, next) = parse_own_playlists(br#"{"data":[],"links":{"self":"/playlists"}}"#).unwrap();
        assert!(list.is_empty());
        assert!(next.is_none());
    }

    #[test]
    fn add_body_is_jsonapi_track_list() {
        assert_eq!(add_items_body(&["123"]).to_string(), r#"{"data":[{"id":"123","type":"tracks"}]}"#);
    }
}

// ---------------- v2 (legacy) ----------------

pub fn artist_page(id: &str) -> Result<legacy::ArtistPage> {
    get_json(&format!("/v2/artist/{id}"), &[])
}

pub fn home_feed_static() -> Result<legacy::Page2> {
    get_json("/v2/home/feed/static", &[])
}

pub fn feed_activities(user_id: &str) -> Result<Vec<legacy::Activity>> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct R {
        activities: Option<Vec<legacy::Activity>>,
    }
    let r: R = get_json("/v2/feed/activities", &[("userId", user_id.into())])?;
    Ok(r.activities.unwrap_or_default())
}

pub fn feed_seen(user_id: &str) -> Result<()> {
    let resp = request(Method::PUT, "/v2/feed/activities/seen", &[("userId", user_id.into())])?.send()?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
    }
    Ok(())
}

pub fn favourite_mix_add(id: &str) -> Result<()> {
    let resp = request(Method::PUT, "/v2/favorites/mixes/add", &[])?.form(&[("mixIds", id), ("onArtifactNotFound", "FAIL")]).send()?;
    if resp.status() != StatusCode::OK {
        bail!("error while adding mix to favourites: {}", resp.text().unwrap_or_default());
    }
    Ok(())
}

pub fn favourite_mix_remove(id: &str) -> Result<()> {
    let resp = request(Method::PUT, "/v2/favorites/mixes/remove", &[("mixIds", id.into())])?.send()?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "error while removing mix from favourites: {}", resp.status());
    }
    Ok(())
}

pub fn favourite_mix_ids(cursor: &str, if_none_match: &str) -> Result<(legacy::FavouritesIds, String)> {
    let mut q = vec![("limit", "50".to_string())];
    if !cursor.is_empty() {
        q.push(("cursor", cursor.into()));
    }
    let mut rb = request(Method::GET, "/v2/favorites/mixes/ids", &q)?;
    if !if_none_match.is_empty() {
        rb = rb.header("If-None-Match", if_none_match);
    }
    let resp = rb.send()?;
    if resp.status() != StatusCode::OK {
        bail!("error while retrieving favourite mix IDs: {}", resp.text().unwrap_or_default());
    }
    let etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    Ok((openapi::from_slice_lenient(&resp.bytes()?)?, etag))
}

// ---------------- device linking ----------------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DeviceLinkingChallenge {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: i64,
    pub interval: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TokenUser {
    pub id: i64,
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(rename = "clientName")]
    pub client_name: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
    pub user: TokenUser,
    pub user_id: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TokenError {
    status: i64,
    error: String,
    sub_status: i64,
    error_description: String,
}

const SCOPE: &str = "r_usr w_usr w_sub";

pub fn request_device_linking_challenge() -> Result<DeviceLinkingChallenge> {
    let resp = http()
        .post(format!("{}/v1/oauth2/device_authorization", crate::config::get().auth_base_url()))
        .form(&[("client_id", client_id()), ("scope", SCOPE)])
        .send()?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "failed to get token: {}", resp.status());
    }
    Ok(resp.json()?)
}

/// Polls until the user approved the device. `cancelled` is checked between polls.
pub fn poll_device_linking_status(device_code: &str, interval: i64, cancelled: &dyn Fn() -> bool) -> Result<TokenResponse> {
    let form = [
        ("client_id", client_id()),
        ("client_secret", client_secret()),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("scope", SCOPE),
    ];
    loop {
        if cancelled() {
            return Err(CoreError::Cancelled("linking canceled".into()));
        }
        let resp = token_request(&form)?;
        if resp.status() != StatusCode::OK {
            let e: TokenError = resp.json()?;
            match e.error.as_str() {
                "authorization_pending" => {
                    // sleep in small steps so cancellation is responsive
                    let until = Instant::now() + Duration::from_secs(interval.max(1) as u64);
                    while Instant::now() < until {
                        if cancelled() {
                            return Err(CoreError::Cancelled("linking canceled".into()));
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                    continue;
                }
                "expired_token" => bail!("linking session expired"),
                _ => return Err(CoreError::Other(e.error_description)),
            }
        }
        return Ok(resp.json()?);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn client_credentials_decode() {
        assert_eq!(super::client_id().len(), 16);
        assert!(!super::client_secret().is_empty());
        assert_eq!(super::query_escape("a b/c"), "a+b%2Fc");
    }
}

#[cfg(test)]
mod net_tests {
    #[test]
    #[ignore = "network"]
    fn country_code_live() {
        let r = super::http()
            .get(format!("{}/v1/country/context?countryCode=WW&locale=en_US&deviceType=BROWSER", crate::config::get().api_base_url()))
            .header("x-tidal-token", super::client_id())
            .send()
            .unwrap();
        eprintln!("status {} url {}", r.status(), r.url());
        eprintln!("{}", r.text().unwrap());
    }
}


#[cfg(test)]
mod search_live {
    #[test]
    #[ignore = "network"]
    fn search_live() {
        let r = super::search("franz ferdinand", &["topHits", "topHits.coverArt", "topHits.profileArt", "topHits.albums.coverArt", "topHits.albums.artists", "topHits.artists"]).unwrap();
        let hits = &r.data.relationships.top_hits.data;
        let (ar, al, pl, tr) = (r.included.artists(hits).len(), r.included.albums(hits).len(), r.included.playlists(hits).len(), r.included.tracks(hits).len());
        let mut c = std::collections::BTreeMap::new();
        for o in &r.included.0 { *c.entry(o.type_.clone()).or_insert(0) += 1; }
        let mut h = std::collections::BTreeMap::new();
        for o in hits { *h.entry(o.type_.clone()).or_insert(0) += 1; }
        eprintln!("included {c:?} hits {h:?}");
        eprintln!("artists {ar} albums {al} playlists {pl} tracks {tr}");
        assert!(ar > 0 && al > 0 && tr > 0);
        // "View All" pages use the search result id, like Go.
        for kind in ["tracks", "albums", "artists", "playlists"] {
            let rel = super::search_relationship(&r.data.id, kind, "", &[]).unwrap();
            eprintln!("view all {kind}: {} items, next={}", rel.data.len(), rel.links.next.is_some());
        }
    }
}
