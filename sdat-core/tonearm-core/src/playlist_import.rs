//! Copies a Spotify playlist into a new TIDAL playlist of the signed-in user.
//!
//! A track is matched by its ISRC, the recording's identifier on both services. A track
//! TIDAL does not have under that ISRC is searched by title and artist instead, and taken
//! only when title, an artist and the length agree.

use std::collections::HashMap;
use std::time::Duration;

use crate::api::{self, openapi};
use crate::error::{bail, CoreError, CoreResult as Result};
use crate::spotify::{self, SpotifyPlaylist, SpotifyTrack};

/// Longest difference in length between the same recording on both services.
const LENGTH_TOLERANCE: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Reading the playlist from Spotify.
    Reading,
    /// Finding its tracks on TIDAL.
    Matching,
    /// Writing the TIDAL playlist.
    Saving,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub stage: Stage,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The new TIDAL playlist.
    pub playlist_id: String,
    pub name: String,
    /// Tracks added to it.
    pub imported: usize,
    /// `Artist – Title` of each track TIDAL does not have.
    pub missing: Vec<String>,
    /// Spotify entries that are not catalogue tracks (local files, podcast episodes).
    pub skipped: usize,
}

/// Imports the public Spotify playlist behind `link` (see [`spotify::playlist_id`]).
/// `cancelled` is polled between requests; nothing is created on TIDAL before the last stage.
pub fn import_spotify_playlist(link: &str, progress: &(dyn Fn(Progress) + Sync), cancelled: &(dyn Fn() -> bool + Sync)) -> Result<Report> {
    api::client::user_bearer()?;
    let Some(id) = spotify::playlist_id(link) else { bail!("This is not a Spotify playlist link.") };
    progress(Progress { stage: Stage::Reading, done: 0, total: 0 });
    let source = spotify::fetch_playlist(&id, &|done, total| progress(Progress { stage: Stage::Reading, done, total }), cancelled)?;
    import_playlist(&source, progress, cancelled)
}

/// Creates a TIDAL playlist with the tracks of `source` that TIDAL has, in the same order.
pub fn import_playlist(source: &SpotifyPlaylist, progress: &dyn Fn(Progress), cancelled: &dyn Fn() -> bool) -> Result<Report> {
    if source.tracks.is_empty() {
        bail!("The Spotify playlist has no tracks.");
    }
    let stop = || if cancelled() { Err(CoreError::Cancelled("Spotify import cancelled".into())) } else { Ok(()) };

    let total = source.tracks.len();
    let mut done = 0;
    let mut advance = |by| {
        done += by;
        progress(Progress { stage: Stage::Matching, done, total });
    };
    advance(0);
    let mut by_isrc: HashMap<String, String> = HashMap::new();
    let mut isrcs: Vec<&str> = source.tracks.iter().map(|t| t.isrc.as_str()).filter(|i| !i.is_empty()).collect();
    isrcs.sort_unstable();
    isrcs.dedup();
    for chunk in isrcs.chunks(api::ISRCS_PER_REQUEST) {
        stop()?;
        by_isrc.extend(best_by_isrc(&api::tracks_by_isrc(chunk)?));
        advance(source.tracks.iter().filter(|t| chunk.contains(&t.isrc.as_str()) && by_isrc.contains_key(&t.isrc)).count());
    }
    let mut found: Vec<Option<String>> = source.tracks.iter().map(|t| by_isrc.get(&t.isrc).cloned()).collect();
    let mut missing = Vec::new();
    for (track, id) in source.tracks.iter().zip(&mut found) {
        if id.is_some() {
            continue;
        }
        stop()?;
        *id = search(track)?;
        if id.is_none() {
            missing.push(format!("{} – {}", track.artists.join(", "), track.title));
        }
        advance(1);
    }
    let matched: Vec<&str> = found.iter().flatten().map(String::as_str).collect();
    if matched.is_empty() {
        bail!("TIDAL has none of the tracks of this playlist.");
    }

    stop()?;
    let saving = |done| progress(Progress { stage: Stage::Saving, done, total: matched.len() });
    saving(0);
    let name = if source.name.trim().is_empty() { "Spotify playlist" } else { source.name.trim() };
    let playlist_id = api::playlist_create(name, "Imported from Spotify")?;
    for (i, chunk) in matched.chunks(api::PLAYLIST_TRACKS_PER_REQUEST).enumerate() {
        api::playlist_add_tracks(&playlist_id, chunk).map_err(|e| CoreError::Other(format!("The playlist \"{name}\" was created on TIDAL but is incomplete: {e}")))?;
        saving(i * api::PLAYLIST_TRACKS_PER_REQUEST + chunk.len());
    }
    Ok(Report { playlist_id, name: name.to_owned(), imported: matched.len(), missing, skipped: source.skipped })
}

fn streamable(t: &openapi::TrackData) -> bool {
    t.attributes.availability.iter().any(|a| a == "STREAM")
}

/// For each ISRC, the ID of its most popular streamable track.
fn best_by_isrc(tracks: &[openapi::TrackData]) -> HashMap<String, String> {
    let mut best: HashMap<String, &openapi::TrackData> = HashMap::new();
    for t in tracks.iter().filter(|t| streamable(t)) {
        best.entry(t.attributes.isrc.to_ascii_uppercase()).and_modify(|b| if t.attributes.popularity > b.attributes.popularity { *b = t }).or_insert(t);
    }
    best.into_iter().map(|(isrc, t)| (isrc, t.id.clone())).collect()
}

/// TIDAL's first search hit that is the same recording as `track`.
fn search(track: &SpotifyTrack) -> Result<Option<String>> {
    let query = format!("{} {}", plain_title(&track.title), track.artists.first().map_or("", String::as_str));
    let found = match api::search(query.trim(), &["tracks.artists"]) {
        Ok(found) => found,
        // TIDAL rejects some queries; that track is simply not found.
        Err(CoreError::Http { status: 400 | 404, .. }) => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(found.included.tracks(&found.data.relationships.tracks.data).into_iter().find_map(|t| {
        let artists: Vec<String> = t.included.artists(&t.data.relationships.artists.data).into_iter().map(|a| a.data.attributes.name).collect();
        (streamable(&t.data) && same_recording(track, &t.data.attributes.title, &artists, t.data.attributes.duration)).then_some(t.data.id)
    }))
}

fn same_recording(track: &SpotifyTrack, title: &str, artists: &[String], duration: Duration) -> bool {
    let wanted: Vec<String> = track.artists.iter().map(|a| fold(a)).collect();
    fold(plain_title(&track.title)) == fold(plain_title(title))
        && track.duration.abs_diff(duration) <= LENGTH_TOLERANCE
        && artists.iter().any(|a| wanted.contains(&fold(a)))
}

/// The title without what the services write differently: `(feat. X)`, `[Live]`, ` - Remastered 2011`.
fn plain_title(title: &str) -> &str {
    let end = [" - ", " (", " ["].iter().filter_map(|mark| title.find(mark)).min().unwrap_or(title.len());
    &title[..end]
}

/// Letters and digits in lower case, so punctuation and spacing do not matter.
fn fold(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tidal(id: &str, isrc: &str, popularity: f64, availability: &[&str]) -> openapi::TrackData {
        let mut t = openapi::TrackData { id: id.into(), ..Default::default() };
        t.attributes.isrc = isrc.into();
        t.attributes.popularity = popularity;
        t.attributes.availability = availability.iter().map(|a| (*a).to_owned()).collect();
        t
    }

    fn spotify(title: &str, artists: &[&str], secs: u64) -> SpotifyTrack {
        SpotifyTrack { title: title.into(), artists: artists.iter().map(|a| (*a).to_owned()).collect(), duration: Duration::from_secs(secs), isrc: String::new() }
    }

    #[test]
    fn an_isrc_maps_to_its_most_popular_streamable_track() {
        let tracks = [
            tidal("1", "GBUM71029604", 0.2, &["STREAM"]),
            tidal("2", "gbum71029604", 0.7, &["STREAM", "DJ"]),
            tidal("3", "GBUM71029604", 0.9, &[]),
            tidal("4", "USRC17607839", 0.1, &[]),
        ];
        assert_eq!(best_by_isrc(&tracks), HashMap::from([("GBUM71029604".to_owned(), "2".to_owned())]));
    }

    #[test]
    fn the_same_recording_matches_despite_title_decorations() {
        let track = spotify("Under Pressure - Remastered 2011", &["Queen", "David Bowie"], 248);
        assert!(same_recording(&track, "Under Pressure (Remastered)", &["David Bowie".into()], Duration::from_secs(246)));
        assert!(same_recording(&spotify("Don't Stop Me Now", &["Queen"], 209), "Dont Stop Me Now", &["QUEEN".into()], Duration::from_secs(209)));
    }

    #[test]
    fn another_recording_does_not_match() {
        let track = spotify("Bohemian Rhapsody", &["Queen"], 354);
        assert!(!same_recording(&track, "Bohemian Rhapsody", &["Queen".into()], Duration::from_secs(148)), "a shorter edit");
        assert!(!same_recording(&track, "Bohemian Rhapsody", &["Panic! At The Disco".into()], Duration::from_secs(354)), "a cover");
        assert!(!same_recording(&track, "Somebody to Love", &["Queen".into()], Duration::from_secs(354)), "another song");
    }

    #[test]
    fn decorations_are_cut_from_the_end_of_a_title_only() {
        assert_eq!(plain_title("(I Can't Get No) Satisfaction"), "(I Can't Get No) Satisfaction");
        assert_eq!(plain_title("Song (feat. X) [Live]"), "Song");
    }
}
