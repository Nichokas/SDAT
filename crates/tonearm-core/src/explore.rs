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
        for it in &row.items {
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
    out
}

/// The signed-in user's recently played items.
pub fn get_home_recents() -> Result<Vec<RecentItem>> {
    Ok(home_recents(&api::home_feed_static()?))
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

/// TIDAL biographies carry `[wimpLink ...]text[/wimpLink]` and `<br/>`; returns plain text.
pub fn plain_bio(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(['[', '<']) {
        out.push_str(&rest[..i]);
        let close = if rest[i..].starts_with('[') { ']' } else { '>' };
        if let Some(j) = rest[i..].find(close) {
            let tag = &rest[i + 1..i + j].to_ascii_lowercase();
            if tag.starts_with("br") || tag.starts_with("/p") {
                out.push('\n');
            }
            rest = &rest[i + j + 1..];
        } else {
            out.push_str(&rest[i..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out.replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bio_markup_becomes_plain_text() {
        let b = "Daft Punk is [wimpLink artistId=\"1\"]an organism[/wimpLink].<br/><br/>Masked &amp; robotic.";
        assert_eq!(plain_bio(b), "Daft Punk is an organism.\n\nMasked & robotic.");
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
