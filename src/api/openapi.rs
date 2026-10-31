//! JSON:API models of the TIDAL `OpenAPI` (openapi.tidal.com/v2).
#![allow(dead_code)]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::time::Duration;

/// Removes `null` object members recursively so `#[serde(default)]` fills them in,
/// matching Go's encoding/json where null leaves the zero value.
pub fn strip_nulls(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.retain(|_, x| !x.is_null());
            m.values_mut().for_each(strip_nulls);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_nulls),
        _ => {}
    }
}

pub fn from_value_lenient<T: DeserializeOwned>(mut v: Value) -> serde_json::Result<T> {
    strip_nulls(&mut v);
    serde_json::from_value(v)
}

pub fn from_slice_lenient<T: DeserializeOwned>(b: &[u8]) -> serde_json::Result<T> {
    from_value_lenient(serde_json::from_slice(b)?)
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Response<D> {
    pub data: D,
    pub included: IncludedObjects,
    pub links: Links,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Links {
    pub meta: Option<LinksMeta>,
    pub next: Option<String>,
    #[serde(rename = "self")]
    pub self_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LinksMeta {
    pub next_cursor: Option<String>,
}

impl<D> Response<D> {
    pub fn next_cursor(&self) -> Option<String> {
        self.links.meta.as_ref().and_then(|m| m.next_cursor.clone())
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Relationship {
    pub id: String,
    pub meta: Option<serde_json::Map<String, Value>>,
    #[serde(rename = "type")]
    pub type_: String,
}

pub type Relationships = Response<Vec<Relationship>>;

#[derive(Debug, Clone)]
pub struct IncludedObject {
    pub id: String,
    pub type_: String,
    pub raw: Value,
}

/// Included objects, de-duplicated by (type, id) with their JSON deep-merged,
/// mirroring the Go implementation (jsons.Merge).
#[derive(Debug, Clone, Default)]
pub struct IncludedObjects(pub Vec<IncludedObject>);

fn merge_json(a: &mut Value, b: Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                match a.get_mut(&k) {
                    Some(existing) => merge_json(existing, v),
                    None => {
                        a.insert(k, v);
                    }
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => a.extend(b),
        (a, b) => {
            if !b.is_null() {
                *a = b;
            }
        }
    }
}

impl<'de> Deserialize<'de> for IncludedObjects {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw: Option<Vec<Value>> = Option::deserialize(d)?;
        let mut out: Vec<IncludedObject> = Vec::new();
        for v in raw.unwrap_or_default() {
            let id = v.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
            let type_ = v.get("type").and_then(Value::as_str).unwrap_or_default().to_string();
            if let Some(existing) = out.iter_mut().find(|o| o.id == id && o.type_ == type_) {
                merge_json(&mut existing.raw, v);
            } else {
                out.push(IncludedObject { id, type_, raw: v });
            }
        }
        Ok(IncludedObjects(out))
    }
}

pub const OBJECT_TYPE_ALBUMS: &str = "albums";
pub const OBJECT_TYPE_ARTISTS: &str = "artists";
pub const OBJECT_TYPE_ARTWORKS: &str = "artworks";
pub const OBJECT_TYPE_LYRICS: &str = "lyrics";
pub const OBJECT_TYPE_PLAYLISTS: &str = "playlists";
pub const OBJECT_TYPE_TRACKS: &str = "tracks";

impl IncludedObjects {
    #[allow(clippy::wrong_self_convention)]
    pub fn from_relationships(&self, rels: &[Relationship], t: &str) -> Vec<&IncludedObject> {
        let mut res = Vec::new();
        for rel in rels {
            for obj in &self.0 {
                if obj.id == rel.id && (obj.type_ == t || t.is_empty()) {
                    res.push(obj);
                }
            }
        }
        res
    }

    fn plain<T: DeserializeOwned>(&self, rels: &[Relationship], t: &str) -> Vec<T> {
        self.from_relationships(rels, t)
            .into_iter()
            .filter_map(|o| from_value_lenient(o.raw.clone()).ok())
            .collect()
    }

    fn wrap<T: DeserializeOwned>(&self, rels: &[Relationship], t: &str) -> Vec<Response<T>> {
        self.plain::<T>(rels, t)
            .into_iter()
            .map(|data| Response { data, included: self.clone(), links: Links::default() })
            .collect()
    }

    pub fn plain_artworks(&self, rels: &[Relationship]) -> Artworks {
        Artworks(self.plain(rels, OBJECT_TYPE_ARTWORKS))
    }
    pub fn albums(&self, rels: &[Relationship]) -> Vec<Album> {
        self.wrap(rels, OBJECT_TYPE_ALBUMS)
    }
    pub fn artists(&self, rels: &[Relationship]) -> Vec<Artist> {
        self.wrap(rels, OBJECT_TYPE_ARTISTS)
    }
    pub fn playlists(&self, rels: &[Relationship]) -> Vec<Playlist> {
        self.wrap(rels, OBJECT_TYPE_PLAYLISTS)
    }
    pub fn tracks(&self, rels: &[Relationship]) -> Vec<Track> {
        self.wrap(rels, OBJECT_TYPE_TRACKS)
    }
    pub fn plain_lyrics(&self, rels: &[Relationship]) -> Vec<LyricsData> {
        self.plain(rels, OBJECT_TYPE_LYRICS)
    }
}

// ---------- helpers for time / duration ----------

/// Parses an ISO8601 duration such as `PT3M25S` or `P1DT2H`.
pub fn parse_iso8601_duration(s: &str) -> Option<Duration> {
    let s = s.strip_prefix('P')?;
    let mut secs = 0f64;
    let mut in_time = false;
    let mut num = String::new();
    for c in s.chars() {
        match c {
            'T' => in_time = true,
            '0'..='9' | '.' | ',' => num.push(if c == ',' { '.' } else { c }),
            _ => {
                let n: f64 = num.parse().ok()?;
                num.clear();
                secs += n * match (c, in_time) {
                    ('Y', false) => 365.0 * 86400.0,
                    ('M', false) => 30.0 * 86400.0,
                    ('W', false) => 7.0 * 86400.0,
                    ('D', false) => 86400.0,
                    ('H', true) => 3600.0,
                    ('M', true) => 60.0,
                    ('S', true) => 1.0,
                    _ => return None,
                };
            }
        }
    }
    Some(Duration::from_secs_f64(secs))
}

pub fn de_iso_duration<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    Ok(s.and_then(|s| parse_iso8601_duration(&s)).unwrap_or_default())
}

pub fn de_opt_iso_duration<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    Ok(s.and_then(|s| parse_iso8601_duration(&s)))
}

pub type DateTime = chrono::DateTime<chrono::Utc>;

pub fn de_datetime<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    Ok(s.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|t| t.with_timezone(&chrono::Utc)))
}

pub fn de_date_only<'de, D: Deserializer<'de>>(d: D) -> Result<Option<chrono::NaiveDate>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    Ok(s.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()))
}

// ---------- external links ----------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ExternalLink {
    pub href: String,
    pub meta: ExternalLinkMeta,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ExternalLinkMeta {
    #[serde(rename = "type")]
    pub type_: String,
}

// ---------- albums ----------

pub type Album = Response<AlbumData>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AlbumData {
    pub attributes: AlbumAttributes,
    pub id: String,
    pub relationships: AlbumRelationships,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AlbumAttributes {
    pub access_type: String,
    pub ai: bool,
    pub availability: Vec<String>,
    pub barcode_id: String,
    #[serde(deserialize_with = "de_iso_duration")]
    pub duration: Duration,
    pub explicit: bool,
    pub external_links: Vec<ExternalLink>,
    #[serde(rename = "media_tags")]
    pub media_tags: Vec<String>,
    pub number_of_items: i64,
    pub number_of_volumes: i64,
    pub popularity: f64,
    #[serde(deserialize_with = "de_date_only")]
    pub release_date: Option<chrono::NaiveDate>,
    pub title: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AlbumRelationships {
    pub artists: Relationships,
    pub cover_art: Relationships,
    pub genres: Relationships,
    pub items: Relationships,
}

// ---------- artists ----------

pub type Artist = Response<ArtistData>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ArtistData {
    pub attributes: ArtistAttributes,
    pub id: String,
    pub relationships: ArtistRelationships,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistAttributes {
    pub contributions_enabled: bool,
    pub external_links: Vec<ExternalLink>,
    pub name: String,
    pub popularity: f64,
    pub spotlighted: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistRelationships {
    pub albums: Relationships,
    pub profile_art: Relationships,
    pub radio: Relationships,
    pub similar_artists: Relationships,
    pub tracks: Relationships,
}

// ---------- artworks ----------

pub const ARTWORK_MEDIA_TYPE_IMAGE: &str = "IMAGE";
pub const ARTWORK_MEDIA_TYPE_VIDEO: &str = "VIDEO";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ArtworkData {
    pub attributes: ArtworkAttributes,
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtworkAttributes {
    pub files: Vec<ArtworkFile>,
    pub media_type: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ArtworkFile {
    pub href: String,
    pub meta: ArtworkFileMeta,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ArtworkFileMeta {
    pub height: i64,
    pub width: i64,
}

/// Same selection algorithm as `ArtworkFiles.AtLeast` in Go.
pub fn files_at_least(files: &[ArtworkFile], size: i64) -> Option<ArtworkFile> {
    if files.is_empty() {
        return None;
    }
    let mut square: Vec<ArtworkFile> = files.iter().filter(|f| f.meta.height == f.meta.width).cloned().collect();
    if square.is_empty() {
        square = files.to_vec();
    }
    square.sort_by_key(|a| (a.meta.height, a.meta.width));
    if size == i64::MAX {
        let mut best = ArtworkFile::default();
        let mut max_h = 0;
        for f in &square {
            if f.meta.height > max_h {
                max_h = f.meta.height;
                best = f.clone();
            }
        }
        return Some(best);
    }
    for f in &square {
        if f.meta.height.min(f.meta.width) >= size {
            return Some(f.clone());
        }
    }
    Some(square[0].clone())
}

#[derive(Debug, Clone, Default)]
pub struct Artworks(pub Vec<ArtworkData>);

impl Artworks {
    pub fn at_least(&self, size: i64, media_type: &str) -> String {
        for a in &self.0 {
            if a.attributes.media_type == media_type
                && let Some(f) = files_at_least(&a.attributes.files, size) {
                    return f.href;
                }
        }
        String::new()
    }
}

// ---------- lyrics ----------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LyricsData {
    pub attributes: LyricsAttributes,
    pub id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LyricsAttributes {
    pub direction: String,
    pub lrc_text: String,
    pub provider: LyricsProvider,
    pub technical_status: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LyricsProvider {
    pub common_track_id: String,
    pub lyrics_id: String,
    pub name: String,
    pub source: String,
}

// ---------- playlists ----------

pub type Playlist = Response<PlaylistData>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PlaylistData {
    pub attributes: PlaylistAttributes,
    pub id: String,
    pub relationships: PlaylistRelationships,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistAttributes {
    pub access_type: String,
    pub bounded: bool,
    #[serde(deserialize_with = "de_datetime")]
    pub created_at: Option<DateTime>,
    pub description: String,
    #[serde(deserialize_with = "de_opt_iso_duration")]
    pub duration: Option<Duration>,
    pub external_links: Vec<ExternalLink>,
    #[serde(deserialize_with = "de_datetime")]
    pub last_modified_at: Option<DateTime>,
    pub name: String,
    pub number_of_followers: i64,
    pub number_of_items: i64,
    pub playlist_type: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistRelationships {
    pub cover_art: Relationships,
    pub items: Relationships,
    pub owner_profiles: Relationships,
    pub owners: Relationships,
}

// ---------- search ----------

pub type SearchResult = Response<Vec<SearchResultData>>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SearchResultData {
    pub attributes: SearchResultAttributes,
    pub id: String,
    pub relationships: SearchResultRelationships,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchResultAttributes {
    pub did_you_mean: String,
    pub query: String,
    pub tracking_id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchResultRelationships {
    pub albums: Relationships,
    pub artists: Relationships,
    pub playlists: Relationships,
    pub top_hits: Relationships,
    pub tracks: Relationships,
    pub videos: Relationships,
}

/// Single-object variant (the Go struct decodes `data` either as object or array depending on endpoint).
pub type SearchResultSingle = Response<SearchResultData>;

pub fn first_search_result(r: SearchResult) -> SearchResultSingle {
    Response { data: r.data.into_iter().next().unwrap_or_default(), included: r.included, links: r.links }
}

// ---------- tracks ----------

pub type Track = Response<TrackData>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TrackData {
    pub attributes: TrackAttributes,
    pub id: String,
    pub relationships: TrackRelationships,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackAttributes {
    pub access_type: String,
    pub ai: bool,
    pub availability: Vec<String>,
    pub bpm: f64,
    #[serde(deserialize_with = "de_iso_duration")]
    pub duration: Duration,
    pub explicit: bool,
    pub external_links: Vec<ExternalLink>,
    pub isrc: String,
    pub key: String,
    pub key_scale: String,
    pub popularity: f64,
    pub spotlighted: bool,
    pub title: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackRelationships {
    pub albums: Relationships,
    pub artists: Relationships,
    pub lyrics: Relationships,
    pub radio: Relationships,
    pub similar_tracks: Relationships,
}

// ---------- track manifests ----------

pub type TrackManifest = Response<TrackManifestData>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TrackManifestData {
    pub attributes: TrackManifestAttributes,
    pub id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackManifestAttributes {
    pub album_audio_normalization_data: Option<AudioNormalizationData>,
    pub formats: Vec<String>,
    pub hash: String,
    pub preview_reason: String,
    pub track_audio_normalization_data: Option<AudioNormalizationData>,
    pub track_presentation: String,
    pub uri: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AudioNormalizationData {
    pub peak_amplitude: f32,
    pub replay_gain: f32,
}

pub const MANIFEST_FORMAT_HEAACV1: &str = "HEAACV1";
pub const MANIFEST_FORMAT_AACLC: &str = "AACLC";
pub const MANIFEST_FORMAT_FLAC: &str = "FLAC";
pub const MANIFEST_FORMAT_FLAC_HIRES: &str = "FLAC_HIRES";

// ---------- user collections ----------

pub type UserCollection = Response<UserCollectionData>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UserCollectionData {
    pub id: String,
    pub relationships: UserCollectionRelationships,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UserCollectionRelationships {
    pub albums: Relationships,
    pub artists: Relationships,
    pub playlists: Relationships,
    pub tracks: Relationships,
}

pub const TRACK_AVAILABILITY_STREAM: &str = "STREAM";

pub fn quality_from_formats(formats: &[String]) -> crate::model::TrackQuality {
    use crate::model::TrackQuality;
    let has = |f: &str| formats.iter().any(|x| x == f);
    if has(MANIFEST_FORMAT_FLAC_HIRES) {
        TrackQuality::HighResLossless
    } else if has(MANIFEST_FORMAT_FLAC) {
        TrackQuality::Lossless
    } else if has(MANIFEST_FORMAT_AACLC) {
        TrackQuality::High
    } else {
        TrackQuality::Lossy
    }
}

pub fn manifest_formats_for_quality(q: crate::model::TrackQuality) -> Vec<&'static str> {
    use crate::model::TrackQuality;
    match q {
        TrackQuality::HighResLossless => vec![MANIFEST_FORMAT_HEAACV1, MANIFEST_FORMAT_AACLC, MANIFEST_FORMAT_FLAC, MANIFEST_FORMAT_FLAC_HIRES],
        TrackQuality::Lossless => vec![MANIFEST_FORMAT_HEAACV1, MANIFEST_FORMAT_AACLC, MANIFEST_FORMAT_FLAC],
        TrackQuality::High => vec![MANIFEST_FORMAT_HEAACV1, MANIFEST_FORMAT_AACLC],
        TrackQuality::Lossy => vec![MANIFEST_FORMAT_HEAACV1],
    }
}
pub const MANIFEST_TYPE_MPEG_DASH: &str = "MPEG_DASH";

#[cfg(test)]
mod tests {
    #[test]
    fn search_result_data_is_an_array() {
        let j = r#"{"data":[{"id":"abc","type":"searchResults","relationships":{"topHits":{"data":[{"id":"1","type":"artists"}]}}}],"included":[{"id":"1","type":"artists","attributes":{"name":"Franz Ferdinand"}}]}"#;
        let r = super::first_search_result(serde_json::from_str(j).unwrap());
        assert_eq!(r.data.id, "abc");
        assert_eq!(r.included.artists(&r.data.relationships.top_hits.data).len(), 1);
    }

    #[test]
    fn null_fields_do_not_drop_objects() {
        let j = r#"{"data":[{"id":"1","type":"tracks"}],"included":[{"id":"1","type":"tracks","attributes":{"title":"Take Me Out","version":null,"key":null,"duration":"PT3M57S"}}]}"#;
        let r: super::Response<Vec<super::Relationship>> = super::from_slice_lenient(j.as_bytes()).unwrap();
        let t = r.included.tracks(&r.data);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].data.attributes.title, "Take Me Out");
    }

    use super::*;

    #[test]
    fn iso_duration() {
        assert_eq!(parse_iso8601_duration("PT3M25S"), Some(Duration::from_secs(205)));
        assert_eq!(parse_iso8601_duration("PT1H"), Some(Duration::from_hours(1)));
        assert_eq!(parse_iso8601_duration("P1DT1S"), Some(Duration::from_secs(86401)));
    }

    #[test]
    fn included_merge() {
        let json = r#"{"data":[],"included":[
            {"id":"1","type":"tracks","attributes":{"title":"a"}},
            {"id":"1","type":"tracks","relationships":{"albums":{"data":[{"id":"9","type":"albums"}]}}}
        ]}"#;
        let r: Relationships = serde_json::from_str(json).unwrap();
        assert_eq!(r.included.0.len(), 1);
        let t = r.included.tracks(&[Relationship { id: "1".into(), meta: None, type_: "tracks".into() }]);
        assert_eq!(t[0].data.attributes.title, "a");
        assert_eq!(t[0].data.relationships.albums.data[0].id, "9");
    }

    #[test]
    fn artwork_selection() {
        let f = |w, h, s: &str| ArtworkFile { href: s.into(), meta: ArtworkFileMeta { width: w, height: h } };
        let files = vec![f(1280, 1280, "big"), f(80, 80, "small"), f(320, 320, "mid"), f(100, 50, "rect")];
        assert_eq!(files_at_least(&files, 200).unwrap().href, "mid");
        assert_eq!(files_at_least(&files, 0).unwrap().href, "small");
        assert_eq!(files_at_least(&files, 5000).unwrap().href, "small");
        assert_eq!(files_at_least(&files, i64::MAX).unwrap().href, "big");
    }
}

