//! Records shared by the screens and their conversion from core models.

use tonearm_core::discover::MixRef;
use tonearm_core::model;

/// Cover sizes: list rows / grids and the player / headers.
const SMALL: i32 = 320;
const LARGE: i32 = 1280;

fn opt(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}

/// Streaming quality tiers shown in the quality menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Quality {
    /// AAC (`HEAACV1`, `AACLC`).
    Aac,
    /// CD: 16-bit FLAC.
    Cd,
    /// `HiFi`: Hi-Res FLAC (`FLAC_HIRES`).
    HiFi,
}

impl From<Quality> for model::TrackQuality {
    fn from(q: Quality) -> Self {
        match q {
            Quality::Aac => Self::High,
            Quality::Cd => Self::Lossless,
            Quality::HiFi => Self::HighResLossless,
        }
    }
}

impl From<model::TrackQuality> for Quality {
    fn from(q: model::TrackQuality) -> Self {
        match q {
            model::TrackQuality::HighResLossless => Self::HiFi,
            model::TrackQuality::Lossless => Self::Cd,
            model::TrackQuality::High | model::TrackQuality::Lossy => Self::Aac,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ArtistRef {
    pub id: String,
    pub name: String,
    pub picture_url: Option<String>,
}

impl From<&model::ArtistInfo> for ArtistRef {
    fn from(a: &model::ArtistInfo) -> Self {
        ArtistRef { id: a.id.clone(), name: a.name.clone(), picture_url: opt(a.cover(SMALL).static_) }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct AlbumRef {
    pub id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub cover_url_large: Option<String>,
}

impl From<&model::AlbumInfo> for AlbumRef {
    fn from(a: &model::AlbumInfo) -> Self {
        AlbumRef { id: a.id.clone(), title: a.title.clone(), cover_url: opt(a.cover(SMALL).static_), cover_url_large: opt(a.cover(LARGE).static_) }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Track {
    pub id: String,
    pub title: String,
    /// "(Remastered)" etc., empty if none.
    pub version: String,
    pub duration_secs: u32,
    pub explicit: bool,
    pub streamable: bool,
    pub artists: Vec<ArtistRef>,
    pub album: Option<AlbumRef>,
}

impl From<&model::Track> for Track {
    fn from(t: &model::Track) -> Self {
        Track {
            id: t.info.id.clone(),
            title: t.info.title.clone(),
            version: t.info.version.clone(),
            duration_secs: u32::try_from(t.info.duration.as_secs()).unwrap_or(u32::MAX),
            explicit: t.info.flags.is_explicit(),
            streamable: t.info.streamable,
            artists: t.artists.iter().map(ArtistRef::from).collect(),
            album: t.album.as_ref().map(AlbumRef::from),
        }
    }
}

pub(crate) fn tracks(v: &[model::Track]) -> Vec<Track> {
    v.iter().map(Track::from).collect()
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artists: Vec<ArtistRef>,
    pub cover_url: Option<String>,
    pub cover_url_large: Option<String>,
    /// `YYYY-MM-DD`.
    pub release_date: Option<String>,
    pub track_count: u32,
    pub duration_secs: u32,
}

impl From<&model::Album> for Album {
    fn from(a: &model::Album) -> Self {
        Album {
            id: a.info.id.clone(),
            title: a.info.title.clone(),
            artists: a.artists.iter().map(ArtistRef::from).collect(),
            cover_url: opt(a.info.cover(SMALL).static_),
            cover_url_large: opt(a.info.cover(LARGE).static_),
            release_date: a.info.released_at.map(|d| d.to_string()),
            track_count: u32::try_from(a.count).unwrap_or(0),
            duration_secs: u32::try_from(a.info.duration.as_secs()).unwrap_or(u32::MAX),
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    /// TIDAL mixes and radios are read-only playlists.
    pub is_mix: bool,
    pub creator: Option<String>,
    pub cover_url: Option<String>,
    pub cover_url_large: Option<String>,
    pub track_count: u32,
    pub duration_secs: u32,
    /// Unix seconds of the last modification ("Updated today").
    pub updated_at: Option<i64>,
}

impl From<&model::Playlist> for Playlist {
    fn from(p: &model::Playlist) -> Self {
        Playlist {
            id: p.info.id.clone(),
            title: p.info.title.clone(),
            is_mix: p.info.is_mix(),
            creator: p.creator.as_ref().map(|c| c.name.clone()),
            cover_url: opt(p.info.cover(SMALL).static_),
            cover_url_large: opt(p.info.cover(LARGE).static_),
            track_count: u32::try_from(p.count).unwrap_or(0),
            duration_secs: u32::try_from(p.info.duration.as_secs()).unwrap_or(u32::MAX),
            updated_at: p.info.updated_at.or(p.info.created_at).map(|d| d.timestamp()),
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub picture_url: Option<String>,
    pub bio: String,
    pub followers: i64,
}

impl From<&model::Artist> for Artist {
    fn from(a: &model::Artist) -> Self {
        Artist {
            id: a.info.id.clone(),
            name: a.info.name.clone(),
            picture_url: opt(a.info.cover(LARGE).static_),
            bio: tonearm_core::explore::plain_bio(&a.description),
            followers: a.follower_count,
        }
    }
}

/// A TIDAL mix of the home feed (Discover sources).
#[derive(Debug, Clone, uniffi::Record)]
pub struct Mix {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub image_url: Option<String>,
}

impl From<&MixRef> for Mix {
    fn from(m: &MixRef) -> Self {
        Mix { id: m.id.clone(), title: m.title.clone(), subtitle: m.subtitle.clone(), image_url: opt(m.image_url.clone()) }
    }
}

impl From<&Mix> for MixRef {
    fn from(m: &Mix) -> Self {
        MixRef { id: m.id.clone(), title: m.title.clone(), subtitle: m.subtitle.clone(), image_url: m.image_url.clone().unwrap_or_default() }
    }
}
