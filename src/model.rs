//! Service-independent domain model (port of `pkg/tonearm`). Go used interfaces
//! with one implementation per backend (openapi / v2); here every concept is a
//! concrete struct that both backends convert into.
#![allow(dead_code)]

use std::time::Duration;

use crate::api::{legacy, openapi};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrackQuality {
    #[default]
    HighResLossless,
    Lossless,
    High,
    Lossy,
}

impl TrackQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            TrackQuality::HighResLossless => "HI_RES_LOSSLESS",
            TrackQuality::Lossless => "LOSSLESS",
            TrackQuality::High => "HIGH",
            TrackQuality::Lossy => "LOW",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "HI_RES_LOSSLESS" => TrackQuality::HighResLossless,
            "LOSSLESS" => TrackQuality::Lossless,
            "HIGH" => TrackQuality::High,
            "LOW" => TrackQuality::Lossy,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrackFlag(pub u8);
impl TrackFlag {
    pub const NONE: TrackFlag = TrackFlag(0);
    pub const EXPLICIT: u8 = 1;
    pub const AI: u8 = 2;
    pub fn is_ai(self) -> bool {
        self.0 & Self::AI != 0
    }
    pub fn is_explicit(self) -> bool {
        self.0 & Self::EXPLICIT != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Album,
    Track,
    Artist,
    Playlist,
    Unknown,
}

impl SourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceType::Album => "album",
            SourceType::Track => "track",
            SourceType::Artist => "artist",
            SourceType::Playlist => "playlist",
            SourceType::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cover {
    pub animated: String,
    pub static_: String,
}

impl Cover {
    pub fn new(animated: impl Into<String>, static_: impl Into<String>) -> Self {
        Cover { animated: animated.into(), static_: static_.into() }
    }
    pub fn has_animated(&self) -> bool {
        !self.animated.is_empty()
    }
    pub fn has_static(&self) -> bool {
        !self.static_.is_empty()
    }
}

/// Where a cover comes from. OpenAPI objects carry several artwork sizes, legacy
/// ones only image/video IDs.
#[derive(Debug, Clone, Default)]
pub enum CoverSource {
    #[default]
    None,
    Artworks(openapi::Artworks),
    Ids { image: String, video: String },
    Url(String),
}

pub fn image_url(id: &str) -> String {
    format!("{}/images/{}/160x160.jpg", crate::settings::resources_base_url(), id.replace('-', "/"))
}
pub fn video_url(id: &str) -> String {
    format!("{}/videos/{}/160x160.mp4", crate::settings::resources_base_url(), id.replace('-', "/"))
}

impl CoverSource {
    pub fn cover(&self, preferred_size: i32) -> Cover {
        let size = preferred_size.max(0) as i64;
        match self {
            CoverSource::None => Cover::default(),
            CoverSource::Artworks(a) => Cover::new(
                a.at_least(size, openapi::ARTWORK_MEDIA_TYPE_VIDEO),
                a.at_least(size, openapi::ARTWORK_MEDIA_TYPE_IMAGE),
            ),
            CoverSource::Ids { image, video } => Cover::new(
                if video.is_empty() { String::new() } else { video_url(video) },
                if image.is_empty() { String::new() } else { image_url(image) },
            ),
            CoverSource::Url(u) => Cover::new("", u.clone()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ArtistInfo {
    pub id: String,
    pub name: String,
    pub cover: CoverSource,
}

impl ArtistInfo {
    pub fn title(&self) -> &str {
        &self.name
    }
    pub fn route(&self) -> String {
        format!("artist/{}", self.id)
    }
    pub fn url(&self) -> String {
        format!("https://tidal.com/artist/{}", self.id)
    }
    pub fn cover(&self, size: i32) -> Cover {
        self.cover.cover(size)
    }
}

pub fn artist_names(a: &[ArtistInfo]) -> Vec<String> {
    a.iter().map(|a| a.name.clone()).collect()
}

#[derive(Debug, Clone, Default)]
pub struct Artist {
    pub info: ArtistInfo,
    pub description: String,
    pub follower_count: i64,
}

#[derive(Debug, Clone, Default)]
pub struct AlbumInfo {
    pub id: String,
    pub title: String,
    pub duration: Duration,
    pub released_at: Option<chrono::NaiveDate>,
    pub cover: CoverSource,
}

impl AlbumInfo {
    pub fn route(&self) -> String {
        format!("album/{}", self.id)
    }
    pub fn url(&self) -> String {
        format!("https://tidal.com/album/{}", self.id)
    }
    pub fn cover(&self, size: i32) -> Cover {
        self.cover.cover(size)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Album {
    pub info: AlbumInfo,
    pub artists: Vec<ArtistInfo>,
    pub count: i64,
}

#[derive(Debug, Clone, Default)]
pub struct PlaylistInfo {
    pub id: String,
    pub title: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration: Duration,
    pub cover: CoverSource,
    /// Legacy (v2) playlists never report as mix.
    pub legacy: bool,
}

impl PlaylistInfo {
    pub fn is_mix(&self) -> bool {
        !self.legacy && !self.id.contains('-')
    }
    pub fn route(&self) -> String {
        format!("playlist/{}", self.id)
    }
    pub fn url(&self) -> String {
        if self.is_mix() {
            format!("https://tidal.com/mix/{}", self.id)
        } else {
            format!("https://tidal.com/playlist/{}", self.id)
        }
    }
    pub fn cover(&self, size: i32) -> Cover {
        self.cover.cover(size)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Playlist {
    pub info: PlaylistInfo,
    pub count: i64,
    pub creator: Option<ArtistInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct TrackInfo {
    pub id: String,
    pub title: String,
    pub version: String,
    pub duration: Duration,
    pub flags: TrackFlag,
    pub streamable: bool,
}

impl TrackInfo {
    pub fn url(&self) -> String {
        format!("https://tidal.com/track/{}", self.id)
    }
    /// Port of tonearm.FormatTitle.
    pub fn formatted_title(&self) -> String {
        let mut suffix = String::new();
        if !self.version.is_empty() {
            suffix = format!(" ({})", self.version);
        }
        if self.flags.is_ai() {
            suffix = " \u{1F170}".into();
        }
        if self.flags.is_explicit() {
            suffix.push_str(" \u{1F174}");
        }
        format!("{}{}", self.title, suffix)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Track {
    pub info: TrackInfo,
    pub artists: Vec<ArtistInfo>,
    pub album: Option<AlbumInfo>,
}

impl Track {
    pub fn id(&self) -> &str {
        &self.info.id
    }
    pub fn title(&self) -> &str {
        &self.info.title
    }
    pub fn formatted_title(&self) -> String {
        self.info.formatted_title()
    }
    pub fn duration(&self) -> Duration {
        self.info.duration
    }
    pub fn url(&self) -> String {
        self.info.url()
    }
    pub fn cover(&self, size: i32) -> Cover {
        self.album.as_ref().map(|a| a.cover(size)).unwrap_or_default()
    }
    pub fn route(&self) -> String {
        format!("album/{}", self.album.as_ref().map(|a| a.id.as_str()).unwrap_or(""))
    }
    pub fn artist_names(&self) -> Vec<String> {
        artist_names(&self.artists)
    }
}

/// A thing that can be played/navigated to (tonearm.PlaybackSource).
#[derive(Debug, Clone)]
pub enum PlaybackSource {
    Album(AlbumInfo),
    Artist(ArtistInfo),
    Playlist(PlaylistInfo),
    Track(Box<Track>),
    MyTracks,
}

impl PlaybackSource {
    pub fn title(&self) -> String {
        match self {
            PlaybackSource::Album(a) => a.title.clone(),
            PlaybackSource::Artist(a) => a.name.clone(),
            PlaybackSource::Playlist(p) => p.title.clone(),
            PlaybackSource::Track(t) => t.info.title.clone(),
            PlaybackSource::MyTracks => crate::i18n::tr("My Tracks"),
        }
    }
    pub fn route(&self) -> String {
        match self {
            PlaybackSource::Album(a) => a.route(),
            PlaybackSource::Artist(a) => a.route(),
            PlaybackSource::Playlist(p) => p.route(),
            PlaybackSource::Track(t) => t.route(),
            PlaybackSource::MyTracks => "my-collection/tracks".into(),
        }
    }
    pub fn cover(&self, size: i32) -> Cover {
        match self {
            PlaybackSource::Album(a) => a.cover(size),
            PlaybackSource::Artist(a) => a.cover(size),
            PlaybackSource::Playlist(p) => p.cover(size),
            PlaybackSource::Track(t) => t.cover(size),
            PlaybackSource::MyTracks => Cover::new("", "https://tidal.com/assets/my-tracks-DTG3pLQW.png"),
        }
    }
    pub fn source_type(&self) -> SourceType {
        match self {
            PlaybackSource::Album(_) => SourceType::Album,
            PlaybackSource::Artist(_) => SourceType::Artist,
            PlaybackSource::Playlist(_) | PlaybackSource::MyTracks => SourceType::Playlist,
            PlaybackSource::Track(_) => SourceType::Track,
        }
    }
    pub fn url(&self) -> String {
        match self {
            PlaybackSource::Album(a) => a.url(),
            PlaybackSource::Artist(a) => a.url(),
            PlaybackSource::Playlist(p) => p.url(),
            PlaybackSource::Track(t) => t.url(),
            PlaybackSource::MyTracks => "https://tidal.com/my-collection/tracks".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Normalization {
    pub peak: f64,
    pub gain: f64,
}

#[derive(Debug, Clone, Default)]
pub struct TrackManifest {
    pub album_normalization: Option<Normalization>,
    pub track_normalization: Option<Normalization>,
    pub quality: TrackQuality,
    pub url: String,
}

/// Port of tidalapi.FormatDuration.
pub fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let days = total / 86400;
    let rem = total % 86400;
    let (h, m, s) = (rem / 3600, (rem / 60) % 60, rem % 60);
    if days > 0 {
        format!("{days}d {h}:{m:02}:{s:02}")
    } else if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

// ---------------- conversions: OpenAPI ----------------

pub fn artist_from_openapi(a: &openapi::Artist) -> ArtistInfo {
    ArtistInfo {
        id: a.data.id.clone(),
        name: a.data.attributes.name.clone(),
        cover: CoverSource::Artworks(a.included.plain_artworks(&a.data.relationships.profile_art.data)),
    }
}

pub fn album_info_from_openapi(a: &openapi::Album) -> AlbumInfo {
    AlbumInfo {
        id: a.data.id.clone(),
        title: a.data.attributes.title.clone(),
        duration: a.data.attributes.duration,
        released_at: a.data.attributes.release_date,
        cover: CoverSource::Artworks(a.included.plain_artworks(&a.data.relationships.cover_art.data)),
    }
}

pub fn album_from_openapi(a: &openapi::Album) -> Album {
    Album {
        info: album_info_from_openapi(a),
        artists: a.included.artists(&a.data.relationships.artists.data).iter().map(artist_from_openapi).collect(),
        count: a.data.attributes.number_of_items,
    }
}

pub fn playlist_info_from_openapi(p: &openapi::Playlist) -> PlaylistInfo {
    PlaylistInfo {
        id: p.data.id.clone(),
        title: p.data.attributes.name.clone(),
        created_at: p.data.attributes.created_at,
        duration: p.data.attributes.duration.unwrap_or_default(),
        cover: CoverSource::Artworks(p.included.plain_artworks(&p.data.relationships.cover_art.data)),
        legacy: false,
    }
}

pub fn playlist_from_openapi(p: &openapi::Playlist) -> Playlist {
    Playlist {
        info: playlist_info_from_openapi(p),
        count: p.data.attributes.number_of_items,
        creator: p.included.artists(&p.data.relationships.owner_profiles.data).first().map(artist_from_openapi),
    }
}

pub fn track_info_from_openapi(t: &openapi::Track) -> TrackInfo {
    let a = &t.data.attributes;
    let mut flags = 0;
    if a.ai {
        flags |= TrackFlag::AI;
    }
    if a.explicit {
        flags |= TrackFlag::EXPLICIT;
    }
    TrackInfo {
        id: t.data.id.clone(),
        title: a.title.clone(),
        version: a.version.clone(),
        duration: a.duration,
        flags: TrackFlag(flags),
        streamable: a.availability.iter().any(|x| x == openapi::TRACK_AVAILABILITY_STREAM),
    }
}

pub fn track_from_openapi(t: &openapi::Track) -> Track {
    Track {
        info: track_info_from_openapi(t),
        artists: t.included.artists(&t.data.relationships.artists.data).iter().map(artist_from_openapi).collect(),
        album: t.included.albums(&t.data.relationships.albums.data).first().map(album_info_from_openapi),
    }
}

pub fn manifest_from_openapi(m: &openapi::TrackManifest) -> TrackManifest {
    let a = &m.data.attributes;
    let norm = |n: &Option<openapi::AudioNormalizationData>| n.as_ref().map(|n| Normalization { peak: n.peak_amplitude as f64, gain: n.replay_gain as f64 });
    TrackManifest {
        album_normalization: norm(&a.album_audio_normalization_data),
        track_normalization: norm(&a.track_audio_normalization_data),
        quality: openapi::quality_from_formats(&a.formats),
        url: a.uri.clone(),
    }
}

// ---------------- conversions: legacy v2 ----------------

pub fn artist_from_v2(a: &legacy::ArtistItemData) -> ArtistInfo {
    ArtistInfo {
        id: a.id.to_string(),
        name: a.name.clone(),
        cover: CoverSource::Ids { image: a.picture.clone(), video: String::new() },
    }
}

pub fn album_from_v2(a: &legacy::AlbumItemData) -> Album {
    Album {
        info: AlbumInfo {
            id: a.id.to_string(),
            title: a.title.clone(),
            duration: Duration::from_secs(a.duration.max(0) as u64),
            released_at: a.release_date,
            cover: CoverSource::Ids { image: a.cover.clone(), video: a.video_cover.clone() },
        },
        artists: a.artists.iter().map(artist_from_v2).collect(),
        count: 0,
    }
}

pub fn playlist_from_v2(p: &legacy::PlaylistItemData) -> Playlist {
    let created_at = chrono::DateTime::parse_from_str(&p.created_at, "%Y-%m-%dT%H:%M:%S%.3f%z").ok().map(|d| d.with_timezone(&chrono::Utc));
    Playlist {
        info: PlaylistInfo {
            id: p.uuid.clone(),
            title: p.title.clone(),
            created_at,
            duration: Duration::from_secs(p.duration.max(0) as u64),
            cover: CoverSource::Ids { image: p.square_image.clone(), video: String::new() },
            legacy: true,
        },
        count: p.number_of_tracks,
        creator: if p.creator.id == 0 {
            None
        } else {
            Some(artist_from_v2(&legacy::ArtistItemData { id: p.creator.id, name: p.creator.name.clone(), picture: p.creator.picture.clone() }))
        },
    }
}

pub fn track_from_v2(t: &legacy::TrackItemData) -> Track {
    let mut flags = 0;
    if t.ai {
        flags |= TrackFlag::AI;
    }
    if t.explicit {
        flags |= TrackFlag::EXPLICIT;
    }
    Track {
        info: TrackInfo {
            id: t.id.to_string(),
            title: t.title.clone(),
            version: t.version.clone(),
            duration: Duration::from_secs(t.duration.max(0) as u64),
            flags: TrackFlag(flags),
            streamable: t.allow_streaming,
        },
        artists: t.artists.iter().map(artist_from_v2).collect(),
        album: Some(AlbumInfo {
            id: t.album.id.to_string(),
            title: t.album.title.clone(),
            duration: Duration::ZERO,
            released_at: t.album.release_date,
            cover: CoverSource::Ids { image: t.album.cover.clone(), video: t.album.video_cover.clone() },
        }),
    }
}

pub fn artist_page_from_v2(p: &legacy::ArtistPage) -> Artist {
    let info = match &p.item.data {
        legacy::ItemData::Artist(a) => artist_from_v2(a),
        _ => ArtistInfo::default(),
    };
    Artist { info, description: p.header.biography.text.clone(), follower_count: p.header.followers_amount }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durations() {
        assert_eq!(format_duration(Duration::from_secs(65)), "1:05");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1:02:05");
        assert_eq!(format_duration(Duration::from_secs(90000)), "1d 1:00:00");
    }
    #[test]
    fn titles() {
        let mut t = TrackInfo { title: "Song".into(), version: "Live".into(), ..Default::default() };
        assert_eq!(t.formatted_title(), "Song (Live)");
        t.flags = TrackFlag(TrackFlag::AI | TrackFlag::EXPLICIT);
        assert_eq!(t.formatted_title(), "Song \u{1F170} \u{1F174}");
    }
}
