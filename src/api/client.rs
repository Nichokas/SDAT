//! HTTP client for TIDAL (port of `pkg/tidalapi`): middleware (headers, base
//! URL, default query params), auth strategies (refresh token and OpenAPI
//! client credentials), device linking and every endpoint Tonearm uses.
//!
//! All functions are blocking; UI code runs them through [`crate::api::run`].
#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::{legacy, openapi};
use crate::settings;

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
static COUNTRY: OnceLock<String> = OnceLock::new();

/// Forget the cached user access token (after sign-in/out).
pub fn reset_user_token() {
    *USER_TOKEN.lock().unwrap() = CachedToken::default();
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
    expires_in: u64,
}

fn token_request(form: &[(&str, &str)]) -> Result<Response> {
    Ok(http()
        .post(format!("{}/v1/oauth2/token", settings::auth_base_url()))
        .form(form)
        .send()?)
}

fn fetch_access_token(cache: &Mutex<CachedToken>, form: &[(&str, &str)]) -> Result<String> {
    if let Some(t) = cache.lock().unwrap().valid() {
        return Ok(t);
    }
    let resp = token_request(form)?;
    if resp.status() != StatusCode::OK {
        bail!("failed to get token: {}", resp.status());
    }
    let t: AccessToken = resp.json()?;
    *cache.lock().unwrap() = CachedToken { token: t.access_token.clone(), expiry: Some(Instant::now() + Duration::from_secs(t.expires_in)) };
    Ok(t.access_token)
}

/// secrets.AuthStrategy: user's refresh token.
fn user_access_token() -> Result<Option<String>> {
    if !crate::secrets::has_refresh_token() {
        return Ok(None);
    }
    let refresh = crate::secrets::get_refresh_token();
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
        .get(format!("{}/v1/country/context?countryCode=WW&locale=en_US&deviceType=BROWSER", settings::api_base_url()))
        .header("x-tidal-token", client_id())
        .send()?;
    if resp.status() != StatusCode::OK {
        bail!("unexpected status code: {}", resp.status().as_u16());
    }
    Ok(resp.json::<R>()?.country_code)
}

pub fn country_code() -> &'static str {
    COUNTRY.get_or_init(|| match fetch_country_code() {
        Ok(c) => {
            log::info!("Discovered country code countryCode={c}");
            c
        }
        Err(e) => {
            log::error!("Failed to fetch country code, defaulting to WW: {e}");
            "WW".into()
        }
    })
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

/// Builds a request with the same middleware as Go's MiddlewareRoundTripper.
fn request(method: Method, path: &str, query: &[(&str, String)]) -> Result<RequestBuilder> {
    let url = if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        format!("{}{}", settings::api_base_url(), path)
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
        for (k, v) in existing.iter() {
            q.append_pair(k, v);
        }
        for (k, v) in query {
            q.append_pair(k, v);
        }
        q.append_pair("deviceType", "BROWSER");
        q.append_pair("platform", "WEB");
        q.append_pair("countryCode", country_code());
        q.append_pair("timeOffset", &utc_offset());
        q.append_pair("locale", &locale());
    }
    let mut auth = user_access_token()?;
    if auth.is_none() && url.as_str().contains(&settings::openapi_base_url()) {
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
        bail!("unexpected status code: {}", resp.status().as_u16());
    }
    let bytes = resp.bytes()?;
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
    format!("{}{}", settings::openapi_base_url(), path)
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

fn query_escape(q: &str) -> String {
    // Go's url.QueryEscape (spaces become '+')
    urlencoding::encode(q).replace("%20", "+")
}

pub fn search(query: &str, include: &[&str]) -> Result<openapi::SearchResultSingle> {
    let mut q = vec![("filter[query]", query.to_string())];
    q.push(("include", include.join(",")));
    get_json(&openapi("/v2/searchResults"), &q)
}
pub fn search_relationship(query: &str, kind: &str, cursor: &str, include: &[&str]) -> Result<openapi::Relationships> {
    relationship(&format!("/v2/searchResults/{}/relationships/{kind}", query_escape(query)), cursor, include)
}

pub fn track_manifest(id: &str, formats: &[&str]) -> Result<openapi::TrackManifest> {
    get_json(
        &openapi(&format!("/v2/trackManifests/{id}")),
        &[
            ("adaptive", "false".into()),
            ("formats", formats.join(",")),
            ("manifestType", openapi::MANIFEST_TYPE_MPEG_DASH.into()),
            ("uriScheme", "HTTPS".into()),
            ("usage", "PLAYBACK".into()),
        ],
    )
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
        bail!("unexpected status code: {}", resp.status().as_u16());
    }
    let etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    Ok(Some((resp.json()?, etag)))
}

/// kind: tracks | albums | playlists | artists
pub fn favourite_add(user_id: &str, kind: &str, id: &str) -> Result<()> {
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
    let resp = request(Method::DELETE, &format!("/v1/users/{user_id}/favorites/{kind}/{id}"), &[])?.send()?;
    if resp.status() != StatusCode::OK {
        bail!("error while removing from favourites: {}", resp.status());
    }
    Ok(())
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
        bail!("unexpected status code: {}", resp.status().as_u16());
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
        bail!("error while removing mix from favourites: {}", resp.status());
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
    Ok((resp.json()?, etag))
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
        .post(format!("{}/v1/oauth2/device_authorization", settings::auth_base_url()))
        .form(&[("client_id", client_id()), ("scope", SCOPE)])
        .send()?;
    if resp.status() != StatusCode::OK {
        bail!("failed to get token: {}", resp.status());
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
            bail!("linking canceled");
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
                            bail!("linking canceled");
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                    continue;
                }
                "expired_token" => bail!("linking session expired"),
                _ => return Err(anyhow!(e.error_description)),
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
            .get(format!("{}/v1/country/context?countryCode=WW&locale=en_US&deviceType=BROWSER", crate::settings::api_base_url()))
            .header("x-tidal-token", super::client_id())
            .send()
            .unwrap();
        eprintln!("status {} url {}", r.status(), r.url());
        eprintln!("{}", r.text().unwrap());
    }
}

