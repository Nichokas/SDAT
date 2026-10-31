//! Models of the legacy TIDAL v1 / v2 JSON APIs (tidal.com/v1, tidal.com/v2).
#![allow(dead_code)]

use serde::{Deserialize, Deserializer};
use serde_json::Value;

fn de_null_default<'de, D: Deserializer<'de>, T: Default + Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

pub fn de_date_only<'de, D: Deserializer<'de>>(d: D) -> Result<Option<chrono::NaiveDate>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    Ok(s.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()))
}

// ---------------- v1 ----------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FavouritesIdLists {
    #[serde(rename = "ARTIST", deserialize_with = "de_null_default")]
    pub artist: Vec<String>,
    #[serde(rename = "ALBUM", deserialize_with = "de_null_default")]
    pub album: Vec<String>,
    #[serde(rename = "PLAYLIST", deserialize_with = "de_null_default")]
    pub playlist: Vec<String>,
    #[serde(rename = "TRACK", deserialize_with = "de_null_default")]
    pub track: Vec<String>,
    #[serde(rename = "VIDEO", deserialize_with = "de_null_default")]
    pub video: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Page {
    pub id: String,
    #[serde(deserialize_with = "de_null_default")]
    pub rows: Vec<Row>,
    pub title: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Row {
    #[serde(deserialize_with = "de_null_default")]
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Module {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub description: String,
    #[serde(deserialize_with = "de_null_default")]
    pub items: Vec<Item>,
    #[serde(deserialize_with = "de_null_default")]
    pub paged_list: PagedList,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Item {
    pub header: String,
    pub short_header: String,
    pub short_sub_header: String,
    pub image_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub artifact_id: String,
    pub text: String,
    pub featured: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PagedList {
    #[serde(deserialize_with = "de_null_default")]
    pub items: Vec<PagedItem>,
    pub total_number_of_items: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct IdName {
    #[serde(deserialize_with = "de_null_default")]
    pub id: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PagedAlbum {
    #[serde(deserialize_with = "de_null_default")]
    pub cover: String,
    pub id: i64,
    pub title: String,
    #[serde(deserialize_with = "de_null_default")]
    pub release_date: String,
}

/// An ID which may be encoded either as string or as number.
#[derive(Debug, Clone, Default)]
pub struct MagicId {
    pub string: String,
    pub int: i64,
}

impl<'de> Deserialize<'de> for MagicId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        Ok(match v {
            Value::String(s) => MagicId { int: s.parse().unwrap_or(0), string: s },
            Value::Number(n) => {
                let i = n.as_f64().unwrap_or(0.0) as i64;
                MagicId { int: i, string: i.to_string() }
            }
            _ => MagicId::default(),
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PagedItem {
    #[serde(deserialize_with = "de_null_default")]
    pub album: PagedAlbum,
    #[serde(deserialize_with = "de_null_default")]
    pub api_path: String,
    #[serde(deserialize_with = "de_null_default")]
    pub artists: Vec<IdName>,
    #[serde(deserialize_with = "de_null_default")]
    pub cover: String,
    #[serde(deserialize_with = "de_null_default")]
    pub creators: Vec<IdName>,
    #[serde(deserialize_with = "de_null_default")]
    pub creator: IdName,
    #[serde(deserialize_with = "de_null_default")]
    pub duration: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    pub id: MagicId,
    #[serde(deserialize_with = "de_null_default")]
    pub icon: String,
    #[serde(deserialize_with = "de_null_default")]
    pub name: String,
    #[serde(deserialize_with = "de_null_default")]
    pub number_of_tracks: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub picture: String,
    #[serde(deserialize_with = "de_null_default")]
    pub release_date: String,
    #[serde(deserialize_with = "de_null_default")]
    pub uuid: String,
    #[serde(deserialize_with = "de_null_default")]
    pub square_image: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TrackMix {
    pub id: String,
}

// ---------------- v2 ----------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FavouritesIds {
    #[serde(deserialize_with = "de_null_default")]
    pub cursor: String,
    #[serde(deserialize_with = "de_null_default")]
    pub content: Vec<String>,
}

pub mod item_type {
    pub const ALBUM: &str = "ALBUM";
    pub const ARTIST: &str = "ARTIST";
    pub const DEEP_LINK: &str = "DEEP_LINK";
    pub const HORIZONTAL_LIST: &str = "HORIZONTAL_LIST";
    pub const HORIZONTAL_LIST_WITH_CONTEXT: &str = "HORIZONTAL_LIST_WITH_CONTEXT";
    pub const MIX: &str = "MIX";
    pub const PLAYLIST: &str = "PLAYLIST";
    pub const SHORTCUT_LIST: &str = "SHORTCUT_LIST";
    pub const TRACK: &str = "TRACK";
    pub const TRACK_LIST: &str = "TRACK_LIST";
}

#[derive(Debug, Clone)]
pub enum ItemData {
    Artist(ArtistItemData),
    Album(AlbumItemData),
    DeepLink(DeepLinkItemData),
    Mix(MixItemData),
    Playlist(PlaylistItemData),
    Track(TrackItemData),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Item2 {
    pub following: bool,
    pub number_of_followers: i64,
    pub type_: String,
    pub data: ItemData,
}

impl Default for Item2 {
    fn default() -> Self {
        Item2 { following: false, number_of_followers: 0, type_: String::new(), data: ItemData::Unknown }
    }
}

impl<'de> Deserialize<'de> for Item2 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let type_ = v.get("type").and_then(Value::as_str).unwrap_or_default().to_string();
        let raw = v.get("data").cloned().unwrap_or(Value::Null);
        fn p<T: for<'a> Deserialize<'a>>(raw: Value) -> Option<T> {
            super::openapi::from_value_lenient(raw).ok()
        }
        let data = match type_.as_str() {
            item_type::ARTIST => p(raw).map(ItemData::Artist),
            item_type::DEEP_LINK => p(raw).map(ItemData::DeepLink),
            item_type::MIX => p(raw).map(ItemData::Mix),
            item_type::PLAYLIST => p(raw).map(ItemData::Playlist),
            item_type::TRACK => p(raw).map(ItemData::Track),
            item_type::ALBUM => p(raw).map(ItemData::Album),
            other => {
                log::error!("Unknown item type item_type={other}");
                None
            }
        }
        .unwrap_or(ItemData::Unknown);
        Ok(Item2 {
            following: v.get("following").and_then(Value::as_bool).unwrap_or(false),
            number_of_followers: v.get("numberOfFollowers").and_then(Value::as_i64).unwrap_or(0),
            type_,
            data,
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ArtistItemData {
    #[serde(deserialize_with = "de_null_default")]
    pub id: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub name: String,
    #[serde(deserialize_with = "de_null_default")]
    pub picture: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AlbumItemData {
    pub ai: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub artists: Vec<ArtistItemData>,
    #[serde(deserialize_with = "de_null_default")]
    pub cover: String,
    pub id: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub duration: i64,
    pub explicit: bool,
    #[serde(deserialize_with = "de_date_only")]
    pub release_date: Option<chrono::NaiveDate>,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(rename = "type", deserialize_with = "de_null_default")]
    pub type_: String,
    #[serde(deserialize_with = "de_null_default")]
    pub video_cover: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DeepLinkItemData {
    #[serde(rename = "externalUrl")]
    pub external_url: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub id: String,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(deserialize_with = "de_null_default")]
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TextInfo {
    #[serde(deserialize_with = "de_null_default")]
    pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UrlHolder {
    #[serde(deserialize_with = "de_null_default")]
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MixItemData {
    #[serde(deserialize_with = "de_null_default")]
    pub id: String,
    #[serde(deserialize_with = "de_null_default")]
    pub mix_images: Vec<UrlHolder>,
    #[serde(deserialize_with = "de_null_default")]
    pub title_text_info: TextInfo,
    #[serde(deserialize_with = "de_null_default")]
    pub subtitle_text_info: TextInfo,
    #[serde(deserialize_with = "de_null_default")]
    pub short_subtitle_text_info: TextInfo,
    #[serde(rename = "type", deserialize_with = "de_null_default")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Creator {
    #[serde(deserialize_with = "de_null_default")]
    pub id: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub name: String,
    #[serde(deserialize_with = "de_null_default")]
    pub picture: String,
    #[serde(rename = "type", deserialize_with = "de_null_default")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistItemData {
    #[serde(deserialize_with = "de_null_default")]
    pub creator: Creator,
    #[serde(rename = "created", deserialize_with = "de_null_default")]
    pub created_at: String,
    #[serde(deserialize_with = "de_null_default")]
    pub description: String,
    #[serde(deserialize_with = "de_null_default")]
    pub duration: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub number_of_tracks: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub square_image: String,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(rename = "type", deserialize_with = "de_null_default")]
    pub type_: String,
    #[serde(deserialize_with = "de_null_default")]
    pub uuid: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackItemData {
    pub ai: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub album: TrackItemDataAlbum,
    pub allow_streaming: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub artists: Vec<ArtistItemData>,
    #[serde(deserialize_with = "de_null_default")]
    pub duration: i64,
    pub explicit: bool,
    pub following: bool,
    pub id: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(deserialize_with = "de_null_default")]
    pub version: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackItemDataAlbum {
    pub ai: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub cover: String,
    pub id: i64,
    #[serde(deserialize_with = "de_date_only")]
    pub release_date: Option<chrono::NaiveDate>,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(deserialize_with = "de_null_default")]
    pub video_cover: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PageCursor {
    #[serde(deserialize_with = "de_null_default")]
    pub cursor: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Page2 {
    #[serde(deserialize_with = "de_null_default")]
    pub uuid: String,
    #[serde(deserialize_with = "de_null_default")]
    pub page: PageCursor,
    #[serde(deserialize_with = "de_null_default")]
    pub items: Vec<PageItem>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Biography {
    #[serde(deserialize_with = "de_null_default")]
    pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistHeader {
    #[serde(deserialize_with = "de_null_default")]
    pub biography: Biography,
    #[serde(deserialize_with = "de_null_default")]
    pub followers_amount: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ArtistPage {
    #[serde(deserialize_with = "de_null_default")]
    pub uuid: String,
    #[serde(deserialize_with = "de_null_default")]
    pub items: Vec<PageItem>,
    #[serde(deserialize_with = "de_null_default")]
    pub header: ArtistHeader,
    pub item: Item2,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PageItem {
    pub header: Option<Item2>,
    #[serde(deserialize_with = "de_null_default")]
    pub items: Vec<Item2>,
    #[serde(deserialize_with = "de_null_default")]
    pub module_id: String,
    pub show_quick_play: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub subtitle: String,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(rename = "type", deserialize_with = "de_null_default")]
    pub type_: String,
    #[serde(deserialize_with = "de_null_default")]
    pub view_all: String,
}

// ---------------- v2 feed ----------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Activity {
    pub followable_activity: FollowableActivity,
    pub seen: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FollowableActivity {
    pub history_mix: Option<HistoryMix>,
    pub album: Option<FeedAlbum>,
    #[serde(deserialize_with = "de_null_default")]
    pub activity_type: String,
    #[serde(deserialize_with = "de_null_default")]
    pub occurred_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FeedArtist {
    pub id: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub name: String,
    pub main: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FeedAlbum {
    pub ai: bool,
    pub id: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(deserialize_with = "de_null_default")]
    pub artists: Vec<FeedArtist>,
    #[serde(deserialize_with = "de_null_default")]
    pub main_artists: Vec<IdName>,
    #[serde(rename = "type", deserialize_with = "de_null_default")]
    pub type_: String,
    #[serde(deserialize_with = "de_null_default")]
    pub release_date: String,
    #[serde(deserialize_with = "de_null_default")]
    pub stream_start_date: String,
    pub allow_streaming: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub cover: String,
    pub number_of_tracks: i64,
    pub explicit: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub video_cover: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ImageData {
    pub width: i64,
    pub height: i64,
    #[serde(deserialize_with = "de_null_default")]
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ImageSet {
    #[serde(rename = "SMALL", alias = "small")]
    pub small: ImageData,
    #[serde(rename = "MEDIUM", alias = "medium")]
    pub medium: ImageData,
    #[serde(rename = "LARGE", alias = "large")]
    pub large: ImageData,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryMix {
    #[serde(deserialize_with = "de_null_default")]
    pub id: String,
    #[serde(deserialize_with = "de_null_default")]
    pub mix_type: String,
    #[serde(deserialize_with = "de_null_default")]
    pub updated: String,
    #[serde(deserialize_with = "de_null_default")]
    pub images: ImageSet,
    #[serde(deserialize_with = "de_null_default")]
    pub detail_images: ImageSet,
    pub master: bool,
    #[serde(deserialize_with = "de_null_default")]
    pub title: String,
    #[serde(deserialize_with = "de_null_default")]
    pub subtitle: String,
}
