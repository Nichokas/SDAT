//! HTTP client for TIDAL (port of `pkg/tidalapi`).
//!
//! Middleware (headers, base
//! URL, default query params), auth strategies (refresh token and `OpenAPI`
//! client credentials), device linking and every endpoint Tonearm uses.
//!
//! All functions are blocking; UI code runs them on a worker thread.

use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

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
        crate::http::client_builder()
            .user_agent("Go-http-client/1.1")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build http client"))
}

#[derive(Default)]
struct CachedToken {
    token: String,
    /// Both clocks, expired once either passes: `Instant` stops while an iPhone sleeps
    /// (`CLOCK_UPTIME_RAW`), and the wall clock can be set back.
    expiry: Option<(Instant, SystemTime)>,
    /// Bumped by a reset, so a request started before it can't store its token.
    generation: u64,
}

impl CachedToken {
    fn valid(&self) -> Option<String> {
        match self.expiry {
            Some((mono, wall)) if !self.token.is_empty() && Instant::now() < mono && SystemTime::now() < wall => Some(self.token.clone()),
            _ => None,
        }
    }
}

static USER_TOKEN: Mutex<CachedToken> = Mutex::new(CachedToken { token: String::new(), expiry: None, generation: 0 });
static OPENAPI_TOKEN: Mutex<CachedToken> = Mutex::new(CachedToken { token: String::new(), expiry: None, generation: 0 });
enum CountryLookup {
    Found(String),
    RetryAfter(Instant),
}

impl CountryLookup {
    fn cached_code(&self) -> Option<&str> {
        match self {
            Self::Found(code) => Some(code),
            Self::RetryAfter(until) if Instant::now() < *until => Some("WW"),
            Self::RetryAfter(_) => None,
        }
    }
}

static COUNTRY: Mutex<Option<CountryLookup>> = Mutex::new(None);

/// Forget the cached user access token (after sign-in/out).
pub fn reset_user_token() {
    reset(&USER_TOKEN);
}

fn reset(cache: &Mutex<CachedToken>) {
    let mut t = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    *t = CachedToken { generation: t.generation + 1, ..CachedToken::default() };
}

/// [`crate::diag::net::send`] that forgets the cached access tokens on a 401, so the next
/// request gets new ones even when both expiry clocks were fooled (a sleep and a clock change).
pub(crate) fn send(rb: RequestBuilder) -> reqwest::Result<Response> {
    let resp = crate::diag::net::send(rb)?;
    if resp.status() == StatusCode::UNAUTHORIZED {
        reset(&USER_TOKEN);
        reset(&OPENAPI_TOKEN);
    }
    Ok(resp)
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
    expires_in: u64,
}

fn token_request(form: &[(&str, &str)]) -> reqwest::Result<Response> {
    token_request_with(http(), form)
}

fn token_request_with(client: &Client, form: &[(&str, &str)]) -> reqwest::Result<Response> {
    send(client.post(format!("{}/v1/oauth2/token", crate::config::get().auth_base_url())).form(form))
}

fn fetch_access_token(cache: &Mutex<CachedToken>, form: &[(&str, &str)]) -> Result<String> {
    // Release the lock before the (slow) token request.
    let (cached, generation) = {
        let c = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        (c.valid(), c.generation)
    };
    if let Some(t) = cached {
        return Ok(t);
    }
    let resp = token_request(form)?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "failed to get token: {}", resp.status());
    }
    let t: AccessToken = resp.json()?;
    let mut c = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if c.generation == generation {
        let ttl = Duration::from_secs(t.expires_in);
        *c = CachedToken { token: t.access_token.clone(), expiry: Some((Instant::now() + ttl, SystemTime::now() + ttl)), generation };
    }
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
    let resp = send(
        http()
            .get(format!("{}/v1/country/context?countryCode=WW&locale=en_US&deviceType=BROWSER", crate::config::get().api_base_url()))
            .header("x-tidal-token", client_id()),
    )?;
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
    let resp = send(
        http()
            .get(format!("{}/v1/sessions", crate::config::get().api_base_url()))
            .header("x-tidal-token", client_id())
            .header("Authorization", format!("Bearer {token}")),
    )?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
    }
    Ok(resp.json::<R>()?.country_code)
}

/// Country used for requests: the user's preference, else [`automatic_country_code`].
pub fn country_code() -> String {
    crate::country::preferred().unwrap_or_else(automatic_country_code)
}

/// The account's session country, else the IP-based context (Go used only the
/// latter, which can be wrong). Successful lookups are cached until [`reset_country_code`].
pub fn automatic_country_code() -> String {
    let lock = || COUNTRY.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(c) = lock().as_ref().and_then(CountryLookup::cached_code) {
        return c.to_owned();
    }
    // The network lookup runs without holding COUNTRY, so `reset_country_code`
    // (called on the UI thread at login/logout) never waits on HTTP.
    let epoch = COUNTRY_GEN.load(std::sync::atomic::Ordering::SeqCst);
    let c = match session_country_code().or_else(|e| {
        log::debug!("session country unavailable: {e:#}");
        fetch_country_code()
    }) {
        Ok(c) => {
            log::info!("Discovered country code countryCode={c}");
            CountryLookup::Found(c)
        }
        Err(e) => {
            log::warn!("Failed to fetch country code, defaulting to WW: {e}");
            CountryLookup::RetryAfter(Instant::now() + Duration::from_secs(2))
        }
    };
    let mut g = lock();
    let code = c.cached_code().unwrap_or("WW").to_owned();
    if COUNTRY_GEN.load(std::sync::atomic::Ordering::SeqCst) != epoch {
        return code;
    }
    if let Some(CountryLookup::Found(code)) = g.as_ref() {
        return code.clone();
    }
    *g = Some(c);
    code
}

static COUNTRY_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Forgets the discovered country (e.g. after login/logout or a preference change).
pub fn reset_country_code() {
    COUNTRY_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
    decode_json(send(request(Method::GET, path, query)?)?, path)
}

fn decode_json<T: DeserializeOwned>(resp: Response, context: &str) -> Result<T> {
    if resp.status() != StatusCode::OK {
        return Err(read_http_error(resp));
    }
    let bytes = resp.bytes()?;
    openapi::from_slice_lenient(&bytes).with_context(|| format!("decoding {context}"))
}

fn read_http_error(resp: Response) -> CoreError {
    let status = resp.status();
    let error = CoreError::http(status.as_u16(), format!("unexpected status code: {}", status.as_u16()));
    if status == StatusCode::BAD_REQUEST
        && let Some(request) = discover_request_details(resp.url())
    {
        let mut body = Vec::new();
        let substatus = resp.take(4096).read_to_end(&mut body).ok().and_then(|_| numeric_error_code(&body));
        return error.context(match substatus {
            Some(code) => format!("{request} tidalErrorCode={code}"),
            None => request,
        });
    }
    error
}

// Use route templates and selected metadata because Discover sends these errors to Sentry.
fn discover_request_details(url: &reqwest::Url) -> Option<String> {
    let path: Vec<_> = url.path_segments()?.collect();
    let route = match path.as_slice() {
        ["v2", "albums", _, "relationships", "items"] => "/v2/albums/{id}/relationships/items",
        ["v2", "playlists", _, "relationships", "items"] => "/v2/playlists/{id}/relationships/items",
        ["v2", "tracks", _, "relationships", "radio"] => "/v2/tracks/{id}/relationships/radio",
        ["v2", "userCollectionTracks", _, "relationships", "items"] => "/v2/userCollectionTracks/{id}/relationships/items",
        ["v2", "artist", _] => "/v2/artist/{id}",
        ["v2", "artists", _] => "/v2/artists/{id}",
        ["v1", "users", _, "favorites", "ids"] => "/v1/users/{id}/favorites/ids",
        _ => return None,
    };
    let mut country = None;
    let mut paginated = false;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "countryCode" if value.len() == 2 && value.bytes().all(|b| b.is_ascii_uppercase()) => country = Some(value),
            "page[cursor]" => paginated = true,
            _ => {}
        }
    }
    Some(format!("GET {route} countryCode={} paginated={paginated}", country.as_deref().unwrap_or("unknown")))
}

fn numeric_error_code(body: &[u8]) -> Option<u32> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    value["subStatus"].as_u64().and_then(|code| u32::try_from(code).ok())
        .or_else(|| value["errors"][0]["code"].as_str()?.parse().ok())
}

/// The signed-in user's access token; never the app-only one.
pub(crate) fn user_bearer() -> Result<String> {
    user_access_token()?.ok_or(CoreError::NotSignedIn)
}

/// The user access token if one is cached and valid; never a network call.
pub(crate) fn cached_user_token() -> Option<String> {
    USER_TOKEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner).valid()
}

/// JSON:API call to openapi.tidal.com (`path` starts with `/v2/`) with the
/// user's token. `Value::Null` for an empty answer.
pub(crate) fn openapi_call(method: Method, path: &str, body: Option<&serde_json::Value>) -> Result<serde_json::Value> {
    openapi_path(path)?;
    // One token for the whole request: the account cannot change under it.
    openapi_call_with(&user_bearer()?, method, path, body)
}

/// [`openapi_call`] with a token the caller took (and checked) itself.
pub(crate) fn openapi_call_with(token: &str, method: Method, path: &str, body: Option<&serde_json::Value>) -> Result<serde_json::Value> {
    openapi_path(path)?;
    let label = method.to_string();
    let sep = if path.contains('?') { '&' } else { '?' };
    let rb = authed(method, &format!("{path}{sep}countryCode={}", country_code()), token);
    jsonapi(rb, &label, path, body)
}

/// [`openapi_call`] with a token taken earlier, for work that outlives the
/// credentials (detaching after a sign-out).
pub(crate) fn openapi_call_as(token: &str, method: Method, path: &str, body: Option<&serde_json::Value>) -> Result<serde_json::Value> {
    openapi_path(path)?;
    let label = method.to_string();
    jsonapi(authed(method, path, token), &label, path, body)
}

fn authed(method: Method, path: &str, token: &str) -> RequestBuilder {
    http().request(method, openapi(path)).bearer_auth(token).header("x-tidal-token", client_id()).header("x-tidal-client-version", CLIENT_VERSION)
}

/// The user token goes to the `OpenAPI` host only.
fn openapi_path(path: &str) -> Result<()> {
    if !path.starts_with("/v2/") {
        bail!("not an OpenAPI path: {path}");
    }
    Ok(())
}

fn jsonapi(rb: RequestBuilder, label: &str, path: &str, body: Option<&serde_json::Value>) -> Result<serde_json::Value> {
    let mut rb = rb.header("Accept", "application/vnd.api+json");
    if let Some(b) = body {
        rb = rb.header("Content-Type", "application/vnd.api+json").body(b.to_string());
    }
    let resp = send(rb)?;
    let status = resp.status();
    let bytes = resp.bytes()?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes);
        bail_http!(status.as_u16(), "{label} {path}: {status}: {}", detail.chars().take(300).collect::<String>());
    }
    if bytes.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_slice(&bytes).with_context(|| format!("decoding {path}"))
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
    get_json(&openapi(path), &relationship_q(cursor, include))
}

fn relationship_q(cursor: &str, include: &[&str]) -> Vec<(&'static str, String)> {
    let mut q = Vec::new();
    if !cursor.is_empty() {
        q.push(("page[cursor]", cursor.to_string()));
    }
    q.extend(include_q(include));
    q
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

fn search_json<T: DeserializeOwned>(path: &str, query: &[(&str, String)], operation: &str) -> Result<T> {
    let resp = send(request(Method::GET, &openapi(path), query)?)?;
    if resp.status() == StatusCode::BAD_REQUEST {
        log::error!("TIDAL search {operation} failed: HTTP 400");
    }
    decode_json(resp, operation)
}

pub fn search(query: &str, include: &[&str]) -> Result<openapi::SearchResultSingle> {
    let mut q = vec![("filter[query]", query.to_string())];
    q.extend(include_q(include));
    // The endpoint returns `data` as an array (Go: Response[[]SearchResultData]); use its first entry.
    let r: openapi::SearchResult = search_json("/v2/searchResults", &q, "lookup")?;
    Ok(openapi::first_search_result(r))
}

/// Fetches a page using the opaque resource ID returned by [`search`], not the search text.
pub(crate) fn search_relationship(result_id: &str, kind: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    let path = format!("/v2/searchResults/{}/relationships/{kind}", urlencoding::encode(result_id));
    search_json(&path, &relationship_q(cursor, include), kind)
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

/// Logged-in user's profile picture: an image id or a full URL. Tries the
/// social profile (`/v2/profiles/{id}`) first, then legacy `GET /v1/users/{id}`.
pub fn user_picture(user_id: &str) -> Result<Option<String>> {
    if let Some(p) = get_json::<serde_json::Value>(&format!("/v2/profiles/{user_id}"), &[]).ok().as_ref().and_then(user_picture_from) {
        return Ok(Some(p));
    }
    let v: serde_json::Value = get_json(&format!("/v1/users/{user_id}"), &[])?;
    Ok(user_picture_from(&v))
}

/// Finds a picture id/URL under `picture`/`profilePicture`/`image` (string or object with `url`/`id`).
pub fn user_picture_from(v: &serde_json::Value) -> Option<String> {
    for key in ["picture", "profilePicture", "image", "imageId"] {
        match v.get(key) {
            Some(serde_json::Value::String(s)) if !s.is_empty() => return Some(s.clone()),
            Some(o @ serde_json::Value::Object(_)) => {
                for k in ["url", "id", "uuid"] {
                    if let Some(s) = o.get(k).and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
                        return Some(s.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    v.get("data").and_then(user_picture_from)
}

/// Returns (list, etag) or None when not modified.
pub fn favourite_ids(user_id: &str, if_none_match: &str) -> Result<Option<(legacy::FavouritesIdLists, String)>> {
    let mut rb = request(Method::GET, &format!("/v1/users/{user_id}/favorites/ids"), &[])?;
    if !if_none_match.is_empty() {
        rb = rb.header("If-None-Match", if_none_match);
    }
    let resp = send(rb)?;
    if resp.status() == StatusCode::NOT_MODIFIED {
        return Ok(None);
    }
    if resp.status() != StatusCode::OK {
        return Err(read_http_error(resp));
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
    let resp = send(
        request(Method::POST, &format!("/v1/users/{user_id}/favorites/{kind}"), &[])?
            .form(&[(field, id), ("onArtifactNotFound", "FAIL")]),
    )?;
    if resp.status() != StatusCode::OK {
        let t = resp.text().unwrap_or_default();
        bail!("error while adding {} to favourites: {t}", kind.trim_end_matches('s'));
    }
    Ok(())
}

pub fn favourite_remove(user_id: &str, kind: &str, id: &str) -> Result<()> {
    invalidate_favourites(kind);
    let resp = send(request(Method::DELETE, &format!("/v1/users/{user_id}/favorites/{kind}/{id}"), &[])?)?;
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
    const MAX_PAGES: usize = 1000;
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..MAX_PAGES {
        let mut q = vec![("filter[owners.id]", "me".to_owned())];
        if let Some(c) = cursor.take() {
            q.push(("page[cursor]", c));
        }
        let resp = send(request(Method::GET, &openapi("/v2/playlists"), &q)?)?;
        if resp.status() != StatusCode::OK {
            bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
        }
        let (page, next) = parse_own_playlists(&resp.bytes()?)?;
        out.extend(page);
        match next {
            Some(c) if seen.insert(c.clone()) => cursor = Some(c),
            Some(_) => bail!("playlist pagination returned a repeated cursor"),
            None => return Ok(out),
        }
    }
    bail!("owned playlist pagination exceeded {MAX_PAGES} pages")
}

/// JSON:API body for adding tracks to a playlist.
pub fn add_items_body(track_ids: &[&str]) -> serde_json::Value {
    serde_json::json!({ "data": track_ids.iter().map(|id| serde_json::json!({ "type": "tracks", "id": id })).collect::<Vec<_>>() })
}

/// Appends a track to one of the user's playlists.
pub fn playlist_add_track(playlist_id: &str, track_id: &str) -> Result<()> {
    let resp = send(
        request(Method::POST, &openapi(&format!("/v2/playlists/{playlist_id}/relationships/items")), &[])?
            .header("Content-Type", "application/vnd.api+json")
            .body(add_items_body(&[track_id]).to_string()),
    )?;
    crate::cache::invalidate(&crate::cache::playlist_key(playlist_id));
    if !resp.status().is_success() {
        bail_http!(resp.status().as_u16(), "error while adding track to playlist: {}", resp.status().as_u16());
    }
    Ok(())
}

/// Removes a track occurrence identified by the playlist relationship's `meta.itemId`.
pub fn playlist_remove_item(playlist_id: &str, track_id: &str, item_id: &str) -> Result<()> {
    if item_id.is_empty() {
        bail!("missing playlist item ID");
    }
    let body = serde_json::json!({ "data": [{ "type": "tracks", "id": track_id, "meta": { "itemId": item_id } }] });
    let resp = send(
        request(Method::DELETE, &openapi(&format!("/v2/playlists/{playlist_id}/relationships/items")), &[])?
            .header("Content-Type", "application/vnd.api+json")
            .body(body.to_string()),
    )?;
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
    fn reads_user_picture() {
        assert_eq!(user_picture_from(&serde_json::json!({"id":1,"picture":"ab-cd"})).as_deref(), Some("ab-cd"));
        assert_eq!(user_picture_from(&serde_json::json!({"id":1,"picture":null})), None);
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

/// Items of a home row's "view all" page (`items` either at the top level or in its first row).
pub fn view_all_items(path: &str, offset: usize, limit: usize) -> Result<Vec<legacy::Item2>> {
    let path = if path.starts_with("http") || path.starts_with('/') { path.to_string() } else { format!("/v2/{path}") };
    let v: serde_json::Value = get_json(&path, &[("offset", offset.to_string()), ("limit", limit.to_string())])?;
    let items = v.get("items").cloned().unwrap_or_default();
    let list = match items.as_array().and_then(|a| a.first()) {
        Some(first) if first.get("data").is_none() && first.get("items").is_some() => first["items"].clone(),
        _ => items,
    };
    Ok(serde_json::from_value(list).unwrap_or_default())
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
    let resp = send(request(Method::PUT, "/v2/feed/activities/seen", &[("userId", user_id.into())])?)?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "unexpected status code: {}", resp.status().as_u16());
    }
    Ok(())
}

pub fn favourite_mix_add(id: &str) -> Result<()> {
    let resp = send(request(Method::PUT, "/v2/favorites/mixes/add", &[])?.form(&[("mixIds", id), ("onArtifactNotFound", "FAIL")]))?;
    if resp.status() != StatusCode::OK {
        bail!("error while adding mix to favourites: {}", resp.text().unwrap_or_default());
    }
    Ok(())
}

pub fn favourite_mix_remove(id: &str) -> Result<()> {
    let resp = send(request(Method::PUT, "/v2/favorites/mixes/remove", &[("mixIds", id.into())])?)?;
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
    let resp = send(rb)?;
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
    #[serde(default = "default_device_poll_interval")]
    pub interval: i64,
}

const fn default_device_poll_interval() -> i64 {
    5
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
    let resp = send(
        http()
            .post(format!("{}/v1/oauth2/device_authorization", crate::config::get().auth_base_url()))
            .form(&[("client_id", client_id()), ("scope", SCOPE)]),
    )?;
    if resp.status() != StatusCode::OK {
        bail_http!(resp.status().as_u16(), "failed to get token: {}", resp.status());
    }
    Ok(resp.json()?)
}

/// Polls until the user approves the code, it expires (`expires_in` seconds) or `cancelled` returns true.
pub fn poll_device_linking_status(device_code: &str, interval: i64, expires_in: i64, cancelled: &dyn Fn() -> bool) -> Result<TokenResponse> {
    let deadline = Instant::now() + Duration::from_secs(expires_in.max(1) as u64);
    // The user is in the browser meanwhile, so the app is in the background: the OS may kill an idle
    // keep-alive socket or hold back data for a while. Every poll opens a fresh connection with a
    // bounded timeout, and failed sends are retried until the code itself expires.
    let client = crate::http::client_builder()
        .user_agent("Go-http-client/1.1")
        .pool_max_idle_per_host(0)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()?;
    let form = [
        ("client_id", client_id()),
        ("client_secret", client_secret()),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("scope", SCOPE),
    ];
    // Sleeps in small steps so cancellation is responsive.
    let mut interval = Duration::from_secs(interval.max(1) as u64);
    let pause = |interval| {
        let now = Instant::now();
        let until = now + deadline.saturating_duration_since(now).min(interval);
        while Instant::now() < until {
            if cancelled() {
                return Err(CoreError::Cancelled("linking canceled".into()));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        Ok(())
    };
    loop {
        if cancelled() {
            return Err(CoreError::Cancelled("linking canceled".into()));
        }
        if Instant::now() >= deadline {
            bail!("linking session expired");
        }
        let resp = match token_request_with(&client, &form) {
            Ok(r) => r,
            Err(e) if !(e.is_status() || e.is_redirect() || e.is_builder()) => {
                log::warn!("device login poll failed, retrying: {e}");
                pause(interval)?;
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        if resp.status() != StatusCode::OK {
            let e: TokenError = resp.json()?;
            match e.error.as_str() {
                "authorization_pending" => {
                    pause(interval)?;
                    continue;
                }
                "slow_down" => {
                    interval = interval.saturating_add(Duration::from_secs(5));
                    pause(interval)?;
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
    fn discover_error_details_include_only_safe_request_metadata() {
        let url = reqwest::Url::parse(
            "https://openapi.tidal.com/v2/userCollectionTracks/private-account/relationships/items?countryCode=ES&page%5Bcursor%5D=opaque&token=secret",
        )
        .unwrap();
        assert_eq!(
            super::discover_request_details(&url).as_deref(),
            Some("GET /v2/userCollectionTracks/{id}/relationships/items countryCode=ES paginated=true"),
        );
        for route in ["artist", "artists"] {
            let url = url.join(&format!("/v2/{route}/private-id?countryCode=ES")).unwrap();
            assert_eq!(
                super::discover_request_details(&url).unwrap(),
                format!("GET /v2/{route}/{{id}} countryCode=ES paginated=false"),
            );
        }
        assert_eq!(super::numeric_error_code(br#"{"subStatus":1001,"userMessage":"private"}"#), Some(1001));
        assert_eq!(super::numeric_error_code(br#"{"errors":[{"code":"123","detail":"private"}]}"#), Some(123));
        assert_eq!(super::numeric_error_code(br#"{"errors":[{"code":"private"}]}"#), None);
    }

    #[test]
    fn client_credentials_decode() {
        assert_eq!(super::client_id().len(), 16);
        assert!(!super::client_secret().is_empty());
    }

    #[test]
    fn unauthorized_response_forgets_the_cached_token() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().expect("accept");
            let _ = sock.read(&mut [0u8; 1024]);
            sock.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").expect("write");
        });
        let _sending = crate::diag::net::SENDING.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let far = std::time::Duration::from_secs(3600);
        *super::USER_TOKEN.lock().expect("lock") = super::CachedToken {
            token: "stale".into(),
            expiry: Some((std::time::Instant::now() + far, std::time::SystemTime::now() + far)),
            generation: 0,
        };
        assert_eq!(super::cached_user_token().as_deref(), Some("stale"));
        let resp = super::send(super::http().get(format!("http://{addr}/"))).expect("send");
        assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert_eq!(super::cached_user_token(), None, "a 401 drops the token so the next request refreshes");
    }
}

#[cfg(test)]
mod net_tests {
    #[test]
    #[ignore = "network"]
    fn country_code_live() {
        let r = crate::diag::net::send(
            super::http()
                .get(format!("{}/v1/country/context?countryCode=WW&locale=en_US&deviceType=BROWSER", crate::config::get().api_base_url()))
                .header("x-tidal-token", super::client_id()),
        )
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
