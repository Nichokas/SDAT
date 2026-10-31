//! Port of `internal/services/tidal`: maps API responses to the domain model.
#![allow(dead_code)]

use anyhow::Result;

use crate::api::{self, openapi};
use crate::model::*;

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

impl<T: Clone> Paginator<T> {
    pub fn new(fetch: PageFn, resolve: ResolveFn<T>) -> Self {
        Paginator { fetch, resolve, cursor: String::new(), items: vec![], fixed: false }
    }
    /// ArrayPaginator: already consumed, `next_page` returns the array.
    pub fn from_vec(items: Vec<T>) -> Self {
        Paginator { fetch: Box::new(|_| Ok(Default::default())), resolve: Box::new(|_| vec![]), cursor: CURSOR_END.into(), items, fixed: true }
    }
    pub fn is_consumed(&self) -> bool {
        self.cursor == CURSOR_END
    }
    pub fn next_page(&mut self) -> Result<Vec<T>> {
        if self.fixed {
            return Ok(self.items.clone());
        }
        if self.is_consumed() {
            anyhow::bail!("no more pages");
        }
        let resp = (self.fetch)(&self.cursor)?;
        let res = (self.resolve)(&resp);
        self.cursor = resp.links.meta.as_ref().and_then(|m| m.next_cursor.clone()).unwrap_or_else(|| CURSOR_END.into());
        self.items.extend(res.iter().cloned());
        Ok(res)
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

/// Fetches the next page on a worker thread.
pub async fn next_page<T: Clone + Send + 'static>(p: &SharedPaginator<T>) -> Result<Vec<T>> {
    let p = p.clone();
    api::run(move || p.lock().unwrap().next_page()).await
}

pub async fn get_all<T: Clone + Send + 'static>(p: &SharedPaginator<T>) -> Result<Vec<T>> {
    let p = p.clone();
    api::run(move || p.lock().unwrap().get_all()).await
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
pub fn get_track_info(id: &str) -> Result<TrackInfo> {
    Ok(track_info_from_openapi(&api::track(id, &[])?))
}
pub fn get_track_manifest(id: &str) -> Result<TrackManifest> {
    let formats = openapi::manifest_formats_for_quality(crate::settings::audio_quality());
    Ok(manifest_from_openapi(&api::track_manifest(id, &formats)?))
}
pub fn get_track_radio(id: &str) -> Result<Option<PlaylistInfo>> {
    let r = api::track_radio(id, "", &["radio"])?;
    Ok(r.included.playlists(&r.data).first().map(playlist_info_from_openapi))
}
