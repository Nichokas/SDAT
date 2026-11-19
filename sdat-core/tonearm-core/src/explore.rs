//! Recently played (from the home feed) and the Explore pages (genres, moods, decades).

use crate::api::{self, legacy};
use crate::error::CoreResult as Result;
use crate::model::{self, Album, ArtistInfo, Playlist, Track};

/// Something TIDAL lists as recently played.
#[derive(Debug, Clone)]
pub enum RecentItem {
    Album(Album),
    Playlist(Playlist),
    Artist(ArtistInfo),
    Mix { id: String, title: String, subtitle: String, image_url: String },
}

/// Whether a home feed row is the "recently played" module.
fn is_recent_row(row: &legacy::PageItem) -> bool {
    let t = row.title.to_lowercase();
    row.type_ == legacy::item_type::SHORTCUT_LIST
        || ["recent", "jump back", "reciente", "escuchado", "continue listening"].iter().any(|k| t.contains(k))
}

/// Items of the home feed's recently played rows, in order, without duplicates.
pub fn home_recents(page: &legacy::Page2) -> Vec<RecentItem> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for row in page.items.iter().filter(|r| is_recent_row(r)) {
        push_recents(&row.items, &mut seen, &mut out);
    }
    out
}

fn push_recents(items: &[legacy::Item2], seen: &mut std::collections::HashSet<String>, out: &mut Vec<RecentItem>) {
    for it in items {
        let item = match &it.data {
            legacy::ItemData::Album(a) => RecentItem::Album(model::album_from_v2(a)),
            legacy::ItemData::Playlist(p) => RecentItem::Playlist(model::playlist_from_v2(p)),
            legacy::ItemData::Artist(a) => RecentItem::Artist(model::artist_from_v2(a)),
            legacy::ItemData::Mix(m) => RecentItem::Mix {
                id: m.id.clone(),
                title: m.title_text_info.text.clone(),
                subtitle: m.subtitle_text_info.text.clone(),
                image_url: m.mix_images.last().map(|i| i.url.clone()).unwrap_or_default(),
            },
            _ => continue,
        };
        let key = match &item {
            RecentItem::Album(a) => format!("a{}", a.info.id),
            RecentItem::Playlist(p) => format!("p{}", p.info.id),
            RecentItem::Artist(a) => format!("r{}", a.id),
            RecentItem::Mix { id, .. } => format!("m{id}"),
        };
        if seen.insert(key) {
            out.push(item);
        }
    }
}

/// The signed-in user's recently played items.
/// The home row only holds a handful; its "view all" page is followed (paged) for the full history.
pub fn get_home_recents() -> Result<Vec<RecentItem>> {
    let page = api::home_feed_static()?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for row in page.items.iter().filter(|r| is_recent_row(r)) {
        push_recents(&row.items, &mut seen, &mut out);
        if row.view_all.is_empty() {
            continue;
        }
        let mut offset = 0;
        while offset < 500 {
            let Ok(items) = api::view_all_items(&row.view_all, offset, 50) else { break };
            let n = items.len();
            push_recents(&items, &mut seen, &mut out);
            if n < 50 {
                break;
            }
            offset += n;
        }
    }
    Ok(out)
}

/// A link to another Explore page (a genre, mood or decade).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLink {
    pub title: String,
    pub path: String,
}

/// One module of an Explore page.
#[derive(Debug, Clone, Default)]
pub struct ExploreSection {
    pub title: String,
    pub links: Vec<PageLink>,
    pub albums: Vec<Album>,
    pub playlists: Vec<Playlist>,
    pub artists: Vec<ArtistInfo>,
    pub tracks: Vec<Track>,
}

/// An Explore page: `explore` itself or a linked one such as `genre_hip_hop`.
#[derive(Debug, Clone, Default)]
pub struct ExplorePage {
    pub title: String,
    pub sections: Vec<ExploreSection>,
}

fn section(m: &legacy::Module) -> Option<ExploreSection> {
    let mut s = ExploreSection { title: m.title.clone(), ..Default::default() };
    let items = &m.paged_list.items;
    match m.type_.as_str() {
        "PAGE_LINKS_CLOUD" | "PAGE_LINKS" => {
            s.links = items.iter().map(|it| PageLink { title: it.title.clone(), path: it.api_path.trim_start_matches("pages/").to_string() }).collect();
        }
        "PLAYLIST_LIST" => {
            s.playlists = items
                .iter()
                .map(|it| {
                    model::playlist_from_v2(&legacy::PlaylistItemData {
                        creator: legacy::Creator { id: it.creator.id, name: it.creator.name.clone(), ..Default::default() },
                        duration: it.duration,
                        number_of_tracks: it.number_of_tracks,
                        square_image: it.square_image.clone(),
                        title: it.title.clone(),
                        uuid: it.uuid.clone(),
                        ..Default::default()
                    })
                })
                .collect();
        }
        "ARTIST_LIST" => {
            s.artists = items.iter().map(|it| model::artist_from_v2(&legacy::ArtistItemData { id: it.id.int, name: it.name.clone(), picture: it.picture.clone() })).collect();
        }
        "ALBUM_LIST" => {
            s.albums = items
                .iter()
                .map(|it| {
                    model::album_from_v2(&legacy::AlbumItemData {
                        artists: it.artists.iter().map(|a| legacy::ArtistItemData { id: a.id, name: a.name.clone(), ..Default::default() }).collect(),
                        cover: it.cover.clone(),
                        id: it.id.int,
                        duration: it.duration,
                        release_date: chrono::NaiveDate::parse_from_str(&it.release_date, "%Y-%m-%d").ok(),
                        title: it.title.clone(),
                        type_: "ALBUM".into(),
                        ..Default::default()
                    })
                })
                .collect();
        }
        "TRACK_LIST" => {
            s.tracks = items
                .iter()
                .map(|it| {
                    model::track_from_v2(&legacy::TrackItemData {
                        album: legacy::TrackItemDataAlbum { cover: it.album.cover.clone(), id: it.album.id, title: it.album.title.clone(), ..Default::default() },
                        artists: it.artists.iter().map(|a| legacy::ArtistItemData { id: a.id, name: a.name.clone(), ..Default::default() }).collect(),
                        duration: it.duration,
                        id: it.id.int,
                        title: it.title.clone(),
                        ..Default::default()
                    })
                })
                .collect();
        }
        _ => return None,
    }
    Some(s)
}

/// Supported modules of a parsed Explore page.
pub fn explore_page(page: &legacy::Page) -> ExplorePage {
    ExplorePage { title: page.title.clone(), sections: page.rows.iter().flat_map(|r| r.modules.iter()).filter_map(section).collect() }
}

/// Fetches an Explore page (`explore`, `genre_pop`...).
pub fn get_explore_page(path: &str) -> Result<ExplorePage> {
    Ok(explore_page(&api::page_v1(path)?))
}

/// Top tracks and albums found in the legacy artist page modules (deduped, page order).
pub fn artist_overview(p: &legacy::ArtistPage) -> (Vec<Track>, Vec<Album>) {
    let mut tracks = Vec::new();
    let mut albums = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in &p.items {
        for it in &row.items {
            match &it.data {
                legacy::ItemData::Track(t) if tracks.len() < 10 => {
                    let t = model::track_from_v2(t);
                    if seen.insert(format!("t{}", t.info.id)) {
                        tracks.push(t);
                    }
                }
                legacy::ItemData::Album(a) => {
                    let a = model::album_from_v2(a);
                    if seen.insert(format!("a{}", a.info.id)) {
                        albums.push(a);
                    }
                }
                _ => {}
            }
        }
    }
    (tracks, albums)
}

/// Artist header, top tracks and albums in one call.
pub fn get_artist_overview(id: &str) -> Result<(model::Artist, Vec<Track>, Vec<Album>)> {
    let page = api::artist_page(id)?;
    let (t, a) = artist_overview(&page);
    Ok((model::artist_page_from_v2(&page), t, a))
}

/// Pages a track module's "view all" list for "Show more". Skips tracks already shown and
/// advances by what TIDAL returned, so overlaps and non-track items neither stall nor skip.
#[derive(Debug)]
pub struct ViewAllTracks {
    path: String,
    offset: usize,
    seen: std::collections::HashSet<String>,
    done: bool,
}

impl ViewAllTracks {
    pub fn new(path: String, shown: impl IntoIterator<Item = String>) -> Self {
        ViewAllTracks { path, offset: 0, seen: shown.into_iter().collect(), done: false }
    }

    pub fn artist_top_tracks(id: &str, shown: impl IntoIterator<Item = String>) -> Self {
        Self::new(format!("artist/ARTIST_TOP_TRACKS/view-all?artistId={id}"), shown)
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    /// The next `limit` tracks not shown yet, fewer at the end.
    pub fn next(&mut self, limit: usize) -> Result<Vec<Track>> {
        self.next_with(limit.max(1), api::view_all_items)
    }

    fn next_with(&mut self, limit: usize, mut fetch: impl FnMut(&str, usize, usize) -> Result<Vec<legacy::Item2>>) -> Result<Vec<Track>> {
        let mut out = Vec::new();
        // Bounded so a server that never sends a short page can't hold a click forever.
        for _ in 0..5 {
            if out.len() >= limit || self.done {
                break;
            }
            let items = match fetch(&self.path, self.offset, limit) {
                Ok(items) => items,
                Err(e) if out.is_empty() => return Err(e),
                Err(_) => break,
            };
            let mut used = items.len();
            for (i, it) in items.iter().enumerate() {
                if let legacy::ItemData::Track(t) = &it.data {
                    let t = model::track_from_v2(t);
                    if self.seen.insert(t.info.id.clone()) {
                        out.push(t);
                    }
                }
                if out.len() == limit {
                    used = i + 1;
                    break;
                }
            }
            self.offset += used;
            self.done = items.len() < limit && used == items.len();
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(items: &[(&str, i64)]) -> Vec<legacy::Item2> {
        let v: Vec<_> = items.iter().map(|(ty, id)| serde_json::json!({"type": ty, "data": {"id": id, "title": format!("t{id}")}})).collect();
        serde_json::from_value(serde_json::Value::Array(v)).expect("items")
    }

    fn ids(t: &[Track]) -> Vec<String> {
        t.iter().map(|t| t.info.id.clone()).collect()
    }

    #[test]
    fn view_all_skips_shown_tracks_and_pages_by_items_returned() {
        // TIDAL's list: a video, the 2 shown tracks, then 1..=5 unseen; 2 per request.
        let all = [("VIDEO", 99), ("TRACK", 10), ("TRACK", 11), ("TRACK", 1), ("TRACK", 2), ("TRACK", 3), ("TRACK", 4), ("TRACK", 5)];
        let mut requested = Vec::new();
        let mut fetch = |_: &str, offset: usize, limit: usize| {
            requested.push(offset);
            Ok(page(&all[offset.min(all.len())..(offset + limit).min(all.len())]))
        };
        let mut p = ViewAllTracks::new("p".into(), ["10".to_string(), "11".to_string()]);
        assert_eq!(ids(&p.next_with(2, &mut fetch).expect("page")), ["1", "2"], "pages of shown tracks move on instead of stalling");
        assert_eq!(ids(&p.next_with(2, &mut fetch).expect("page")), ["3", "4"], "resumes mid-page after the last track returned");
        assert!(!p.is_done());
        assert_eq!(ids(&p.next_with(2, &mut fetch).expect("page")), ["5"]);
        assert!(p.is_done(), "a short page ends the list");
        assert_eq!(requested, [0, 2, 4, 5, 7], "offsets count every item used, videos and shown tracks included");
    }

    #[test]
    fn view_all_requests_are_bounded_per_call() {
        let mut calls = 0;
        let mut p = ViewAllTracks::new("p".into(), ["1".to_string()]);
        let got = p
            .next_with(2, |_, _, _| {
                calls += 1;
                Ok(page(&[("TRACK", 1), ("VIDEO", 2)]))
            })
            .expect("page");
        assert!(got.is_empty());
        assert_eq!(calls, 5, "a server repeating full pages of nothing new stops after a few requests");
    }

    #[test]
    fn explore_track_list_marks_tracks_streamable() {
        let page: legacy::Page = serde_json::from_str(
            r#"{"rows":[{"modules":[{"type":"TRACK_LIST","pagedList":{"items":[{"id":42,"title":"Track","duration":180}]}}]}]}"#,
        )
        .expect("page");
        let explore = explore_page(&page);
        let tracks = &explore.sections[0].tracks;
        assert_eq!(ids(tracks), ["42"]);
        assert_eq!(tracks.iter().map(|track| track.info.streamable).collect::<Vec<_>>(), [true]);
    }

    #[test]
    fn explore_links_and_recents_parse() {
        let page: legacy::Page = serde_json::from_str(
            r#"{"title":"Explore","rows":[{"modules":[{"type":"PAGE_LINKS_CLOUD","title":"Genres","pagedList":{"items":[{"title":"Pop","apiPath":"pages/genre_pop"}]}},{"type":"VIDEO_LIST","title":"x"}]}]}"#,
        )
        .expect("page");
        let e = explore_page(&page);
        assert_eq!(e.sections.len(), 1);
        assert_eq!(e.sections[0].links, vec![PageLink { title: "Pop".into(), path: "genre_pop".into() }]);

        let feed: legacy::Page2 = serde_json::from_str(
            r#"{"uuid":"u","items":[{"title":"Recently played","type":"HORIZONTAL_LIST","items":[
              {"type":"ALBUM","data":{"id":5,"title":"A","cover":"c","artists":[]}},
              {"type":"MIX","data":{"id":"m1","titleTextInfo":{"text":"My Mix"},"subtitleTextInfo":{"text":"s"},"mixImages":[]}}]},
              {"title":"Because you liked","type":"HORIZONTAL_LIST","items":[{"type":"ALBUM","data":{"id":6,"title":"B","artists":[]}}]}]}"#,
        )
        .expect("feed");
        let r = home_recents(&feed);
        assert_eq!(r.len(), 2, "{r:?}");
        assert!(matches!(&r[0], RecentItem::Album(a) if a.info.id == "5"));
    }
}
