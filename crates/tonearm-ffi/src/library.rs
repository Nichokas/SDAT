//! Discover, Search, My Collection, Album, Playlist and Artist data, plus
//! favourites and "add to playlist".

use std::sync::{Arc, Mutex, PoisonError};

use tonearm_core::service::{self, Paginator};
use tonearm_core::{api, discover, explore, model, state};

use crate::convert::tracks;
use crate::{Album, Artist, ArtistRef, FfiError, Mix, Playlist, Track, run};

macro_rules! pager {
    ($(#[$m:meta])* $name:ident, $core:ty, $rec:ty) => {
        $(#[$m])*
        #[derive(Debug, uniffi::Object)]
        pub struct $name(Arc<Mutex<Paginator<$core>>>);

        impl $name {
            pub(crate) fn new(p: Paginator<$core>) -> Arc<Self> {
                Arc::new($name(Arc::new(Mutex::new(p))))
            }
        }

        #[uniffi::export]
        impl $name {
            /// The next page, `None` when there are no more.
            pub async fn next_page(&self) -> Result<Option<Vec<$rec>>, FfiError> {
                let p = Arc::clone(&self.0);
                let page = run(move || {
                    let mut p = p.lock().unwrap_or_else(PoisonError::into_inner);
                    if p.is_consumed() { Ok(None) } else { p.next_page().map(Some) }
                })
                .await?;
                Ok(page.map(|v| v.iter().map(<$rec>::from).collect()))
            }
        }
    };
}

pager!(/** Pages of tracks. */ TrackPager, model::Track, Track);
pager!(/** Pages of albums. */ AlbumPager, model::Album, Album);
pager!(/** Pages of artists. */ ArtistPager, model::ArtistInfo, ArtistRef);
pager!(/** Pages of playlists. */ PlaylistPager, model::Playlist, Playlist);

// ---- Discover

/// Mixes of the home feed (My Mixes, My Daily Discovery, My New Arrivals...).
#[uniffi::export]
pub async fn home_mixes() -> Result<Vec<Mix>, FfiError> {
    Ok(run(discover::get_home_mixes).await?.iter().map(Mix::from).collect())
}

/// Songs of a mix or playlist.
#[uniffi::export]
pub fn playlist_tracks(id: String) -> Arc<TrackPager> {
    TrackPager::new(service::get_playlist_tracks(&id))
}

// ---- Search

#[derive(Debug, Clone, uniffi::Record)]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<ArtistRef>,
    pub playlists: Vec<Playlist>,
}

const SEARCH_INCLUDES: [&str; 6] = ["topHits", "topHits.profileArt", "topHits.coverArt", "topHits.albums.coverArt", "topHits.albums.artists", "topHits.artists"];

/// Top hits of each kind (one request, like the desktop search page); use the
/// `search_*` pagers for "see all".
#[uniffi::export]
pub async fn search(query: String) -> Result<SearchResults, FfiError> {
    run(move || {
        let r = api::search(&query, &SEARCH_INCLUDES)?;
        let hits = &r.data.relationships.top_hits.data;
        Ok(SearchResults {
            tracks: r.included.tracks(hits).iter().map(|t| Track::from(&model::track_from_openapi(t))).collect(),
            albums: r.included.albums(hits).iter().map(|a| Album::from(&model::album_from_openapi(a))).collect(),
            artists: r.included.artists(hits).iter().map(|a| ArtistRef::from(&model::artist_from_openapi(a))).collect(),
            playlists: r.included.playlists(hits).iter().map(|p| Playlist::from(&model::playlist_from_openapi(p))).collect(),
        })
    })
    .await
}

#[uniffi::export]
pub fn search_tracks(query: String) -> Arc<TrackPager> {
    TrackPager::new(service::search_tracks(&query))
}
#[uniffi::export]
pub fn search_albums(query: String) -> Arc<AlbumPager> {
    AlbumPager::new(service::search_albums(&query))
}
#[uniffi::export]
pub fn search_artists(query: String) -> Arc<ArtistPager> {
    ArtistPager::new(service::search_artists(&query))
}
#[uniffi::export]
pub fn search_playlists(query: String) -> Arc<PlaylistPager> {
    PlaylistPager::new(service::search_playlists(&query))
}

// ---- My Collection

#[uniffi::export]
pub fn collection_tracks() -> Arc<TrackPager> {
    TrackPager::new(service::user_collection_tracks("me"))
}
#[uniffi::export]
pub fn collection_albums() -> Arc<AlbumPager> {
    AlbumPager::new(service::user_collection_albums("me"))
}
#[uniffi::export]
pub fn collection_artists() -> Arc<ArtistPager> {
    ArtistPager::new(service::user_collection_artists("me"))
}
#[uniffi::export]
pub fn collection_playlists() -> Arc<PlaylistPager> {
    PlaylistPager::new(service::user_collection_playlists("me"))
}

// ---- Album / Playlist / Artist

#[derive(Debug, Clone, uniffi::Record)]
pub struct AlbumDetail {
    pub album: Album,
    pub tracks: Vec<Track>,
}

#[uniffi::export]
pub async fn album(id: String) -> Result<AlbumDetail, FfiError> {
    run(move || {
        let a = service::get_album(&id)?;
        let t = service::get_album_tracks(&id).get_all()?;
        Ok(AlbumDetail { album: Album::from(&a), tracks: tracks(&t) })
    })
    .await
}

/// Playlist header; songs via [`playlist_tracks`].
#[uniffi::export]
pub async fn playlist(id: String) -> Result<Playlist, FfiError> {
    Ok(Playlist::from(&run(move || service::get_playlist(&id)).await?))
}

#[uniffi::export]
pub async fn artist(id: String) -> Result<Artist, FfiError> {
    Ok(Artist::from(&run(move || service::get_artist(&id)).await?))
}

// ---- Favourites and playlists

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum FavouriteKind {
    Track,
    Album,
    Artist,
    Playlist,
    Mix,
}

fn cache(k: FavouriteKind) -> &'static state::FavouriteCache {
    match k {
        FavouriteKind::Track => &state::TRACKS,
        FavouriteKind::Album => &state::ALBUMS,
        FavouriteKind::Artist => &state::ARTISTS,
        FavouriteKind::Playlist => &state::PLAYLISTS,
        FavouriteKind::Mix => &state::MIXES,
    }
}

#[uniffi::export]
pub async fn is_favourite(kind: FavouriteKind, id: String) -> Result<bool, FfiError> {
    run(move || cache(kind).contains(&id)).await
}

#[uniffi::export]
pub async fn set_favourite(kind: FavouriteKind, id: String, favourite: bool) -> Result<(), FfiError> {
    run(move || if favourite { cache(kind).add(&id) } else { cache(kind).remove(&id) }).await
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct OwnPlaylist {
    pub id: String,
    pub name: String,
    pub track_count: i64,
}

/// The user's own playlists (targets of "Add to playlist").
#[uniffi::export]
pub async fn own_playlists() -> Result<Vec<OwnPlaylist>, FfiError> {
    Ok(run(api::own_playlists).await?.into_iter().map(|p| OwnPlaylist { id: p.id, name: p.name, track_count: p.number_of_items }).collect())
}

#[uniffi::export]
pub async fn add_to_playlist(playlist_id: String, track_id: String) -> Result<(), FfiError> {
    run(move || api::playlist_add_track(&playlist_id, &track_id)).await
}

/// Removes the first occurrence of a track from one of the user's playlists.
#[uniffi::export]
pub async fn remove_from_playlist(playlist_id: String, track_id: String) -> Result<(), FfiError> {
    run(move || api::playlist_remove_track(&playlist_id, &track_id)).await
}

// ---- Recents and Explore

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RecentKind {
    Album,
    Playlist,
    Artist,
    Mix,
}

/// A recently played item (home feed), shown as a cover tile.
#[derive(Debug, Clone, uniffi::Record)]
pub struct RecentItem {
    pub kind: RecentKind,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
}

impl From<&explore::RecentItem> for RecentItem {
    fn from(r: &explore::RecentItem) -> Self {
        match r {
            explore::RecentItem::Album(a) => {
                let a = Album::from(a);
                RecentItem { kind: RecentKind::Album, subtitle: a.artists.iter().map(|x| x.name.clone()).collect::<Vec<_>>().join(", "), id: a.id, title: a.title, cover_url: a.cover_url_large.or(a.cover_url) }
            }
            explore::RecentItem::Playlist(p) => {
                let p = Playlist::from(p);
                RecentItem { kind: RecentKind::Playlist, subtitle: p.creator.clone().unwrap_or_default(), id: p.id, title: p.title, cover_url: p.cover_url_large.or(p.cover_url) }
            }
            explore::RecentItem::Artist(a) => {
                let a = ArtistRef::from(a);
                RecentItem { kind: RecentKind::Artist, subtitle: String::new(), id: a.id, title: a.name, cover_url: a.picture_url }
            }
            explore::RecentItem::Mix { id, title, subtitle, image_url } => RecentItem {
                kind: RecentKind::Mix,
                id: id.clone(),
                title: title.clone(),
                subtitle: subtitle.clone(),
                cover_url: (!image_url.is_empty()).then(|| image_url.clone()),
            },
        }
    }
}

/// What TIDAL lists as recently played on the home feed.
#[uniffi::export]
pub async fn recents() -> Result<Vec<RecentItem>, FfiError> {
    Ok(run(explore::get_home_recents).await?.iter().map(RecentItem::from).collect())
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct PageLink {
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ExploreSection {
    pub title: String,
    pub links: Vec<PageLink>,
    pub albums: Vec<Album>,
    pub playlists: Vec<Playlist>,
    pub artists: Vec<ArtistRef>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ExplorePage {
    pub title: String,
    pub sections: Vec<ExploreSection>,
}

/// An Explore page: `explore` (genres, moods, decades) or a linked `path`.
#[uniffi::export]
pub async fn explore_page(path: String) -> Result<ExplorePage, FfiError> {
    let p = run(move || explore::get_explore_page(&path)).await?;
    Ok(ExplorePage {
        title: p.title,
        sections: p
            .sections
            .iter()
            .map(|s| ExploreSection {
                title: s.title.clone(),
                links: s.links.iter().map(|l| PageLink { title: l.title.clone(), path: l.path.clone() }).collect(),
                albums: s.albums.iter().map(Album::from).collect(),
                playlists: s.playlists.iter().map(Playlist::from).collect(),
                artists: s.artists.iter().map(ArtistRef::from).collect(),
                tracks: tracks(&s.tracks),
            })
            .collect(),
    })
}
