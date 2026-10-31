//! Port of `internal/services/tidal`: maps API responses to the domain model.

use crate::error::{bail, CoreResult as Result};

use crate::api::{self, openapi};
use crate::model::{Track, track_from_openapi, Album, album_from_openapi, AlbumInfo, album_info_from_openapi, Artist, artist_page_from_v2, ArtistInfo, artist_from_openapi, Playlist, playlist_from_openapi, PlaylistInfo, playlist_info_from_openapi, TrackManifest, TrackQuality, manifest_from_openapi};

pub const CURSOR_END: &str = "END_REACHED";

type PageFn = Box<dyn Fn(&str) -> Result<openapi::Relationships> + Send + Sync>;
type ResolveFn<T> = Box<dyn Fn(&openapi::Relationships) -> Vec<T> + Send + Sync>;

/// Cursor based paginator (paginatorOpenAPI). Blocking; share it behind
/// `Arc<Mutex<_>>` to use it from worker threads.
pub struct Paginator<T> {
    fetch: PageFn,
    resolve: ResolveFn<T>,
    cursor: String,
    items: Vec<T>,
    fixed: bool,
}

impl<T> std::fmt::Debug for Paginator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Paginator").field("cursor", &self.cursor).field("items", &self.items.len()).field("fixed", &self.fixed).finish_non_exhaustive()
    }
}

impl<T: Clone> Paginator<T> {
    pub fn new(fetch: PageFn, resolve: ResolveFn<T>) -> Self {
        Paginator { fetch, resolve, cursor: String::new(), items: vec![], fixed: false }
    }
    pub fn is_consumed(&self) -> bool {
        self.cursor == CURSOR_END
    }
    pub fn next_page(&mut self) -> Result<Vec<T>> {
        if self.fixed {
            return Ok(self.items.clone());
        }
        if self.is_consumed() {
            bail!("no more pages");
        }
        let resp = (self.fetch)(&self.cursor)?;
        let res = (self.resolve)(&resp);
        self.cursor = resp.links.meta.as_ref().and_then(|m| m.next_cursor.clone()).unwrap_or_else(|| CURSOR_END.into());
        self.items.extend(res.iter().cloned());
        Ok(res)
    }
    /// Items fetched so far.
    pub fn items(&self) -> &[T] {
        &self.items
    }
    pub fn get_all(&mut self) -> Result<Vec<T>> {
        while !self.is_consumed() {
            self.next_page()?;
        }
        Ok(self.items.clone())
    }
}

pub type SharedPaginator<T> = std::sync::Arc<std::sync::Mutex<Paginator<T>>>;

pub fn shared<T>(p: Paginator<T>) -> SharedPaginator<T> {
    std::sync::Arc::new(std::sync::Mutex::new(p))
}



fn resolve_tracks(r: &openapi::Relationships) -> Vec<Track> {
    r.included.tracks(&r.data).iter().map(track_from_openapi).collect()
}

const TRACK_INCLUDES: [&str; 2] = ["items.artists.profileArt", "items.albums.coverArt"];

pub fn get_album(id: &str) -> Result<Album> {
    Ok(album_from_openapi(&api::album(id, &["artists.profileArt", "coverArt"])?))
}
pub fn get_album_info(id: &str) -> Result<AlbumInfo> {
    Ok(album_info_from_openapi(&api::album(id, &["coverArt"])?))
}
pub fn get_album_tracks(id: &str) -> Paginator<Track> {
    let id = id.to_string();
    Paginator::new(Box::new(move |c| api::album_items(&id, c, &TRACK_INCLUDES)), Box::new(resolve_tracks))
}
pub fn get_artist(id: &str) -> Result<Artist> {
    Ok(artist_page_from_v2(&api::artist_page(id)?))
}
pub fn get_artist_info(id: &str) -> Result<ArtistInfo> {
    Ok(artist_from_openapi(&api::artist(id, &["profileArt"])?))
}
pub fn get_playlist(id: &str) -> Result<Playlist> {
    Ok(playlist_from_openapi(&api::playlist(id, &["coverArt", "ownerProfiles.profileArt"])?))
}
pub fn get_playlist_info(id: &str) -> Result<PlaylistInfo> {
    Ok(playlist_info_from_openapi(&api::playlist(id, &["coverArt"])?))
}
pub fn get_playlist_tracks(id: &str) -> Paginator<Track> {
    let id = id.to_string();
    Paginator::new(Box::new(move |c| api::playlist_items(&id, c, &TRACK_INCLUDES)), Box::new(resolve_tracks))
}
pub fn get_track(id: &str) -> Result<Track> {
    Ok(track_from_openapi(&api::track(id, &["albums.coverArt", "artists.profileArt"])?))
}
pub fn get_track_manifest(id: &str) -> Result<TrackManifest> {
    let formats = openapi::manifest_formats_for_quality(crate::config::get().audio_quality());
    Ok(manifest_from_openapi(&api::track_manifest(id, &formats)?))
}
/// Lyrics of a track: synced LRC text when available, else plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lyrics {
    pub text: String,
    /// `text` is LRC (`[mm:ss.xx] line`).
    pub synced: bool,
}

pub fn get_lyrics(id: &str) -> Result<Option<Lyrics>> {
    let t = api::track(id, &["lyrics"])?;
    if let Some(l) = t.included.plain_lyrics(&t.data.relationships.lyrics.data).first() {
        if !l.attributes.lrc_text.is_empty() {
            return Ok(Some(Lyrics { text: l.attributes.lrc_text.clone(), synced: true }));
        } else if !l.attributes.text.is_empty() {
            return Ok(Some(Lyrics { text: l.attributes.text.clone(), synced: false }));
        }
    }
    Ok(None)
}

/// HLS manifest for `quality` (for `AVPlayer`; it cannot read DASH).
pub fn get_track_manifest_hls(id: &str, quality: TrackQuality) -> Result<TrackManifest> {
    let formats = openapi::manifest_formats_for_quality(quality);
    Ok(manifest_from_openapi(&api::track_manifest_hls(id, &formats)?))
}
pub fn get_track_radio(id: &str) -> Result<Option<PlaylistInfo>> {
    let r = api::track_radio(id, "", &["radio"])?;
    Ok(r.included.playlists(&r.data).first().map(playlist_info_from_openapi))
}

fn collection<T: Clone + 'static>(
    kind: &'static str,
    user_id: String,
    include: &'static [&'static str],
    resolve: fn(&openapi::Relationships) -> Vec<T>,
) -> Paginator<T> {
    Paginator::new(Box::new(move |c| api::user_collection_items(kind, &user_id, c, include)), Box::new(resolve))
}

pub fn user_collection_tracks(user_id: &str) -> Paginator<Track> {
    collection("Tracks", user_id.to_string(), &["items.artists", "items.albums.coverArt"], resolve_tracks)
}

pub fn user_collection_albums(user_id: &str) -> Paginator<Album> {
    collection("Albums", user_id.to_string(), &["items.coverArt", "items.artists"], |r| r.included.albums(&r.data).iter().map(album_from_openapi).collect())
}

pub fn user_collection_artists(user_id: &str) -> Paginator<ArtistInfo> {
    collection("Artists", user_id.to_string(), &["items.profileArt"], |r| r.included.artists(&r.data).iter().map(artist_from_openapi).collect())
}

pub fn user_collection_playlists(user_id: &str) -> Paginator<Playlist> {
    collection("Playlists", user_id.to_string(), &["items.coverArt", "items.ownerProfiles"], |r| {
        r.included.playlists(&r.data).iter().map(playlist_from_openapi).collect()
    })
}

fn search_paginator<T: Clone + 'static>(
    query: &str,
    kind: &'static str,
    include: &'static [&'static str],
    resolve: fn(&openapi::Relationships) -> Vec<T>,
) -> Paginator<T> {
    let q = query.to_string();
    Paginator::new(Box::new(move |c| api::search_relationship(&q, kind, c, include)), Box::new(resolve))
}

pub fn search_tracks(q: &str) -> Paginator<Track> {
    search_paginator(q, "tracks", &["tracks.albums.coverArt", "tracks.albums.artists"], resolve_tracks)
}
pub fn search_albums(q: &str) -> Paginator<Album> {
    search_paginator(q, "albums", &["albums.coverArt", "albums.artists"], |r| r.included.albums(&r.data).iter().map(album_from_openapi).collect())
}
pub fn search_artists(q: &str) -> Paginator<ArtistInfo> {
    search_paginator(q, "artists", &["artists.profileArt"], |r| r.included.artists(&r.data).iter().map(artist_from_openapi).collect())
}
pub fn search_playlists(q: &str) -> Paginator<Playlist> {
    search_paginator(q, "playlists", &["playlists.coverArt"], |r| {
        r.included.playlists(&r.data).iter().map(playlist_from_openapi).collect()
    })
}
