//! Discover: the mixes of the home feed and the infinite shuffle queue that
//! mixes their songs with track radios.

use std::collections::{HashSet, VecDeque};

use rand::seq::{IndexedRandom, SliceRandom};

use crate::api::{self, legacy};
use crate::error::CoreResult as Result;
use crate::model::Track;
use crate::service::{self, Paginator};
use crate::state;

/// A TIDAL mix (My Mix, My Daily Discovery, My New Arrivals, radios...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixRef {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub image_url: String,
}

/// Every mix on a home feed page, in page order, without duplicates.
pub fn home_mixes(page: &legacy::Page2) -> Vec<MixRef> {
    let mut seen = HashSet::new();
    page.items
        .iter()
        .flat_map(|row| row.header.iter().chain(row.items.iter()))
        .filter_map(|it| match &it.data {
            legacy::ItemData::Mix(m) if seen.insert(m.id.clone()) => Some(MixRef {
                id: m.id.clone(),
                title: m.title_text_info.text.clone(),
                subtitle: m.subtitle_text_info.text.clone(),
                image_url: m.mix_images.last().map(|i| i.url.clone()).unwrap_or_default(),
            }),
            _ => None,
        })
        .collect()
}

/// The signed-in user's home feed mixes.
pub fn get_home_mixes() -> Result<Vec<MixRef>> {
    Ok(home_mixes(&api::home_feed_static()?))
}

/// Next page of a track source, `None` once it is exhausted.
pub type PageSource = Box<dyn FnMut() -> Result<Option<Vec<Track>>> + Send>;
/// Opens the radio of a track (`None` if TIDAL has none).
pub type RadioFn = Box<dyn Fn(&str) -> Result<Option<PageSource>> + Send>;

/// Adapts a paginator to a [`PageSource`].
pub fn paginator_source(mut p: Paginator<Track>) -> PageSource {
    Box::new(move || if p.is_consumed() { Ok(None) } else { p.next_page().map(Some) })
}

/// The radio of `track_id` as a [`PageSource`].
pub fn track_radio_source(track_id: &str) -> Result<Option<PageSource>> {
    Ok(service::get_track_radio(track_id)?.map(|r| paginator_source(service::get_playlist_tracks(&r.id))))
}

/// A source opened on its first turn (`None` from `open` = empty source).
fn lazy(open: impl FnOnce() -> Result<Option<PageSource>> + Send + 'static) -> PageSource {
    let mut open = Some(open);
    let mut inner: Option<PageSource> = None;
    Box::new(move || {
        if let Some(f) = open.take() {
            inner = f()?;
        }
        match inner.as_mut() {
            Some(next) => next(),
            None => Ok(None),
        }
    })
}

fn random_favourite(cache: &state::FavouriteCache) -> Result<Option<String>> {
    Ok(cache.get()?.choose(&mut rand::rng()).cloned())
}

struct Source {
    next: PageSource,
    buffer: Vec<Track>,
    done: bool,
}

/// Songs taken from one source per turn, so batches interleave sources.
const PER_TURN: usize = 3;
/// Every n-th picked track seeds a radio for when the mixes run dry.
const SEED_EVERY: usize = 4;
const MAX_SEEDS: usize = 16;

/// Endless queue over mixes, then radios.
///
/// Round-robins over the sources, a few shuffled songs per source at a time,
/// never repeating a track; when every source is exhausted it continues with
/// radios seeded by songs it already played.
pub struct InfiniteShuffle {
    sources: VecDeque<Source>,
    seeds: VecDeque<String>,
    radio: RadioFn,
    seen: HashSet<String>,
    picked: usize,
}

impl std::fmt::Debug for InfiniteShuffle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InfiniteShuffle").field("sources", &self.sources.len()).field("seen", &self.seen.len()).finish_non_exhaustive()
    }
}

impl InfiniteShuffle {
    pub fn new(sources: Vec<PageSource>, radio: RadioFn) -> Self {
        let mut s: Vec<Source> = sources.into_iter().map(|next| Source { next, buffer: vec![], done: false }).collect();
        s.shuffle(&mut rand::rng());
        InfiniteShuffle { sources: s.into(), seeds: VecDeque::new(), radio, seen: HashSet::new(), picked: 0 }
    }

    /// Wider Discover pool: the mixes plus the user's library, taken deeper and
    /// at random — favourite tracks from a random page on, random favourite
    /// albums, top tracks of random favourite artists and radios of random
    /// favourite tracks — round-robined so sources interleave. Every library
    /// source is lazy (nothing is fetched until its first turn).
    pub fn from_mixes_and_library(mixes: &[MixRef]) -> Self {
        let mut sources: Vec<PageSource> = mixes.iter().map(|m| paginator_source(service::get_playlist_tracks(&m.id))).collect();
        sources.push(lazy(|| {
            let uid = crate::auth::tokens().user_id();
            if uid.is_empty() {
                return Ok(None);
            }
            let mut p = service::user_collection_tracks(&uid);
            // Start somewhere deeper than the newest favourites.
            for _ in 0..rand::random_range(0..4) {
                if p.is_consumed() {
                    break;
                }
                p.next_page()?;
            }
            Ok(Some(paginator_source(p)))
        }));
        for _ in 0..6 {
            sources.push(lazy(|| Ok(random_favourite(&state::ALBUMS)?.map(|id| paginator_source(service::get_album_tracks(&id))))));
            sources.push(lazy(|| {
                Ok(random_favourite(&state::ARTISTS)?.map(|id| {
                    let mut once = Some(id);
                    Box::new(move || once.take().map(|id| crate::explore::get_artist_overview(&id).map(|o| o.1)).transpose()) as PageSource
                }))
            }));
            sources.push(lazy(|| match random_favourite(&state::TRACKS)? {
                Some(id) => track_radio_source(&id),
                None => Ok(None),
            }));
        }
        Self::new(sources, Box::new(track_radio_source))
    }

    /// Shuffle over the songs of `mixes`, then their radios.
    pub fn from_mixes(mixes: &[MixRef]) -> Self {
        let sources = mixes.iter().map(|m| paginator_source(service::get_playlist_tracks(&m.id))).collect();
        Self::new(sources, Box::new(track_radio_source))
    }

    /// At least `min` new tracks unless every source and seed is exhausted
    /// (then fewer, possibly none). Failing sources are dropped and logged.
    pub fn next_batch(&mut self, min: usize) -> Result<Vec<Track>> {
        let mut out = Vec::new();
        while out.len() < min {
            let Some(mut src) = self.sources.pop_front() else {
                let Some(seed) = self.seeds.pop_front() else { break };
                match (self.radio)(&seed) {
                    Ok(Some(next)) => self.sources.push_back(Source { next, buffer: vec![], done: false }),
                    Ok(None) => {}
                    Err(e) => log::warn!("shuffle: radio of {seed} failed: {e}"),
                }
                continue;
            };
            if src.buffer.is_empty() && !src.done {
                match (src.next)() {
                    Ok(Some(mut page)) => {
                        page.retain(|t| !self.seen.contains(t.id()));
                        page.shuffle(&mut rand::rng());
                        src.buffer = page;
                    }
                    Ok(None) => src.done = true,
                    Err(e) => {
                        log::warn!("shuffle: source failed: {e}");
                        continue;
                    }
                }
            }
            let mut taken = 0;
            while taken < PER_TURN
                && let Some(t) = src.buffer.pop()
            {
                if self.seen.insert(t.id().to_string()) {
                    self.picked += 1;
                    if self.picked.is_multiple_of(SEED_EVERY) && self.seeds.len() < MAX_SEEDS {
                        self.seeds.push_back(t.id().to_string());
                    }
                    out.push(t);
                    taken += 1;
                }
            }
            if !(src.done && src.buffer.is_empty()) {
                self.sources.push_back(src);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TrackInfo;

    fn track(id: &str) -> Track {
        Track { info: TrackInfo { id: id.into(), ..Default::default() }, ..Default::default() }
    }

    /// A source serving `pages` of ids prefixed with `p`.
    fn source(p: &'static str, pages: usize, per_page: usize) -> PageSource {
        let mut n = 0;
        Box::new(move || {
            if n == pages {
                return Ok(None);
            }
            n += 1;
            Ok(Some((0..per_page).map(|i| track(&format!("{p}{n}-{i}"))).collect()))
        })
    }

    #[test]
    fn batches_interleave_sources_without_repeats() {
        let mut s = InfiniteShuffle::new(vec![source("a", 2, 5), source("b", 2, 5), source("c", 2, 5)], Box::new(|_| Ok(None)));
        let first = s.next_batch(9).expect("batch");
        assert!(first.len() >= 9);
        for p in ["a", "b", "c"] {
            assert!(first.iter().any(|t| t.id().starts_with(p)), "{p} missing from the first batch");
        }
        let mut all: Vec<String> = first.iter().map(|t| t.id().to_string()).collect();
        loop {
            let b = s.next_batch(10).expect("batch");
            if b.is_empty() {
                break;
            }
            all.extend(b.iter().map(|t| t.id().to_string()));
        }
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "a track repeated");
        assert_eq!(n, 30, "every song of every source is played once");
    }

    #[test]
    fn continues_with_radios_when_sources_run_dry() {
        let mut s = InfiniteShuffle::new(vec![source("a", 1, 8)], Box::new(|seed| Ok(Some(source(if seed.starts_with('a') { "r" } else { "q" }, 1, 4)))));
        let mut ids = vec![];
        for _ in 0..4 {
            ids.extend(s.next_batch(4).expect("batch").into_iter().map(|t| t.id().to_string()));
        }
        assert!(ids.iter().any(|i| i.starts_with('r')), "radio songs follow the exhausted mix: {ids:?}");
    }

    #[test]
    fn failing_source_is_dropped() {
        let bad: PageSource = Box::new(|| Err(crate::error::CoreError::Network("down".into())));
        let mut s = InfiniteShuffle::new(vec![bad, source("a", 1, 3)], Box::new(|_| Ok(None)));
        assert_eq!(s.next_batch(3).expect("batch").len(), 3);
    }

    #[test]
    fn home_mixes_dedups_in_order() {
        let j = r#"{"items":[{"type":"HORIZONTAL_LIST","items":[
            {"type":"MIX","data":{"id":"m1","titleTextInfo":{"text":"My Mix 1"},"subtitleTextInfo":{"text":"A, B"},"mixImages":[{"url":"s"},{"url":"l"}]}},
            {"type":"ALBUM","data":{"id":1,"title":"x"}},
            {"type":"MIX","data":{"id":"m2","titleTextInfo":{"text":"My Daily Discovery"}}}]},
            {"type":"HORIZONTAL_LIST","items":[{"type":"MIX","data":{"id":"m1","titleTextInfo":{"text":"My Mix 1"}}}]}]}"#;
        let page: legacy::Page2 = serde_json::from_str(j).expect("page");
        let m = home_mixes(&page);
        assert_eq!(m.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["m1", "m2"]);
        assert_eq!(m[0].image_url, "l");
        assert_eq!(m[1].title, "My Daily Discovery");
    }
}
