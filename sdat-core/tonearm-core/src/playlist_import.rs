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
///
/// `cancelled` is polled between requests until [`Stage::Saving`], which always finishes:
/// nothing is created on TIDAL before it, and stopping inside it would leave half a playlist.
/// Saving writes to the account signed in when it starts, whatever the user does meanwhile.
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
    let matching = |done| progress(Progress { stage: Stage::Matching, done, total });
    matching(0);
    let mut by_isrc: HashMap<String, String> = HashMap::new();
    let mut isrcs: Vec<&str> = source.tracks.iter().map(|t| t.isrc.as_str()).filter(|i| !i.is_empty()).collect();
    isrcs.sort_unstable();
    isrcs.dedup();
    for chunk in isrcs.chunks(api::ISRCS_PER_REQUEST) {
        stop()?;
        by_isrc.extend(best_by_isrc(&patiently(cancelled, || api::tracks_by_isrc(chunk))?));
        matching(source.tracks.iter().filter(|t| by_isrc.contains_key(&t.isrc)).count());
    }
    let mut found: Vec<Option<String>> = source.tracks.iter().map(|t| by_isrc.get(&t.isrc).cloned()).collect();
    let mut done = found.iter().flatten().count();
    let mut missing = Vec::new();
    // A playlist can repeat a track; each one is searched once.
    let mut searched: HashMap<&SpotifyTrack, Option<String>> = HashMap::new();
    for (track, id) in source.tracks.iter().zip(&mut found) {
        if id.is_some() {
            continue;
        }
        stop()?;
        *id = if let Some(known) = searched.get(track) {
            known.clone()
        } else {
            let result = search(track, cancelled)?;
            searched.insert(track, result.clone());
            result
        };
        if id.is_none() {
            missing.push(format!("{} – {}", track.artists.join(", "), track.title));
        }
        done += 1;
        matching(done);
    }
    let matched: Vec<&str> = found.iter().flatten().map(String::as_str).collect();
    if matched.is_empty() {
        bail!("TIDAL has none of the tracks of this playlist.");
    }

    stop()?;
    let account = api::account()?;
    let saving = |done| progress(Progress { stage: Stage::Saving, done, total: matched.len() });
    saving(0);
    let name = if source.name.trim().is_empty() { "Spotify playlist" } else { source.name.trim() };
    let playlist_id = api::playlist_create(&account, name, "Imported from Spotify")?;
    for (i, chunk) in matched.chunks(api::PLAYLIST_TRACKS_PER_REQUEST).enumerate() {
        api::playlist_add_tracks(&account, &playlist_id, chunk).map_err(|e| CoreError::Other(format!("The playlist \"{name}\" was created on TIDAL but is incomplete: {e}")))?;
        saving(i * api::PLAYLIST_TRACKS_PER_REQUEST + chunk.len());
    }
    Ok(Report { playlist_id, name: name.to_owned(), imported: matched.len(), missing, skipped: source.skipped })
}

/// Runs a TIDAL request again, after a pause, while TIDAL answers that there were too many.
fn patiently<T>(cancelled: &dyn Fn() -> bool, request: impl Fn() -> Result<T>) -> Result<T> {
    const PAUSES_SECS: [u32; 3] = [2, 5, 10];
    const TICK: Duration = Duration::from_millis(250);
    for secs in PAUSES_SECS {
        match request() {
            Err(CoreError::Http { status: 429, .. }) => {
                for _ in 0..secs * 4 {
                    if cancelled() {
                        return Err(CoreError::Cancelled("Spotify import cancelled".into()));
                    }
                    std::thread::sleep(TICK);
                }
            }
            other => return other,
        }
    }
    request()
}

fn streamable(t: &openapi::TrackData) -> bool {
    t.attributes.availability.iter().any(|a| a == "STREAM")
}

/// For each ISRC, the ID of its most popular streamable track.
fn best_by_isrc(tracks: &[openapi::TrackData]) -> HashMap<String, String> {
    let mut best: HashMap<String, &openapi::TrackData> = HashMap::new();
    // A track without an ISRC must not match the Spotify tracks that have none either.
    for t in tracks.iter().filter(|t| streamable(t) && !t.attributes.isrc.is_empty()) {
        best.entry(t.attributes.isrc.to_ascii_uppercase()).and_modify(|b| if t.attributes.popularity > b.attributes.popularity { *b = t }).or_insert(t);
    }
    best.into_iter().map(|(isrc, t)| (isrc, t.id.clone())).collect()
}

/// TIDAL's first search hit that is the same recording as `track`.
fn search(track: &SpotifyTrack, cancelled: &dyn Fn() -> bool) -> Result<Option<String>> {
    let query = telling(&track.title).chain(track.artists.first().map(String::as_str)).collect::<Vec<_>>().join(" ");
    let found = match patiently(cancelled, || api::search(query.trim(), &["tracks.artists"])) {
        Ok(found) => found,
        // TIDAL rejects some queries; that track is simply not found.
        Err(CoreError::Http { status: 400 | 404, .. }) => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(found.included.tracks(&found.data.relationships.tracks.data).into_iter().find_map(|t| {
        let artists: Vec<String> = t.included.artists(&t.data.relationships.artists.data).into_iter().map(|a| a.data.attributes.name).collect();
        let a = &t.data.attributes;
        let title = with_version(&a.title, &a.version);
        (streamable(&t.data) && same_recording(track, &title, &artists, a.duration)).then_some(t.data.id)
    }))
}

fn same_recording(track: &SpotifyTrack, title: &str, artists: &[String], duration: Duration) -> bool {
    let wanted: Vec<String> = track.artists.iter().map(|a| fold(a)).collect();
    comparable(&track.title) == comparable(title) && track.duration.abs_diff(duration) <= LENGTH_TOLERANCE && artists.iter().any(|a| wanted.contains(&fold(a)))
}

/// TIDAL keeps `Live` or `Remastered 2011` in a track's version, and some titles repeat it
/// after the name.
fn with_version(title: &str, version: &str) -> String {
    let words = spaced(version);
    if words.trim().is_empty() || parts(title).skip(1).any(|part| spaced(part).contains(&words)) {
        title.to_owned()
    } else {
        format!("{title} ({version})")
    }
}

/// The name of a title and what follows it in brackets or after a dash: `Song (feat. X) - Live`.
fn parts(title: &str) -> impl Iterator<Item = &str> {
    title.split(['(', ')', '[', ']']).flat_map(|part| part.split(" - ")).map(str::trim).filter(|part| !part.is_empty())
}

/// Words the services add or leave out for the same recording.
const NEUTRAL: [&str; 13] = ["remaster", "remastered", "mono", "stereo", "deluxe", "bonus", "edition", "track", "version", "digital", "anniversary", "the", "a"];

/// The parts of a title that tell which recording it is.
///
/// It drops only the parts that say nothing about that: featured artists, the film a song
/// is from, and remaster or edition notes. Any other part stays (`Live`, `Part 2`,
/// `Taylor's Version`), so that a doubtful match is reported missing, not imported.
fn telling(title: &str) -> impl Iterator<Item = &str> {
    let says_nothing = |part: &str| {
        let part = part.to_lowercase();
        let words: Vec<&str> = part.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
        // A year alone can tell two recordings apart; beside `Remastered` it only dates the note.
        matches!(words.first(), Some(&("feat" | "ft" | "featuring" | "from"))) || (words.iter().all(|w| NEUTRAL.contains(w) || is_year(w)) && !words.iter().all(|w| is_year(w)))
    };
    parts(title).enumerate().filter(move |(i, part)| *i == 0 || !says_nothing(part)).map(|(_, part)| part)
}

/// The title in a form equal for the same recording on both services.
fn comparable(title: &str) -> String {
    telling(title).map(fold).collect()
}

/// Lower-case words between single spaces, with one at each end, to look for whole words in.
fn spaced(s: &str) -> String {
    let lower = s.to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    format!(" {} ", words.join(" "))
}

fn is_year(word: &str) -> bool {
    word.len() == 4 && word.bytes().all(|b| b.is_ascii_digit())
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
            tidal("5", "", 0.9, &["STREAM"]),
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
    fn another_rendition_of_the_song_does_not_match() {
        let same = |spotify_title: &str, tidal_title: &str| same_recording(&spotify(spotify_title, &["Band"], 200), tidal_title, &["Band".into()], Duration::from_secs(200));
        assert!(same("Song (Live)", "Song - Live"));
        assert!(!same("Song (Live)", "Song (Acoustic)"));
        assert!(!same("Song (Live)", "Song"));
        assert!(!same("Song", "Song - Sped Up"));
        assert!(!same("Song (Part 1)", "Song (Part 2)"));
        assert!(!same("Intro (Dawn)", "Intro (Dusk)"));
        assert!(!same("Love - Hate", "Love"));
        assert!(!same("Song (Taylor's Version)", "Song"));
        assert!(!same("Song (1st Version)", "Song (2nd Version)"));
        assert!(!same("Song (1980)", "Song (1981)"));
    }

    #[test]
    fn a_tidal_version_joins_the_title_unless_the_title_repeats_it() {
        assert_eq!(with_version("Live Forever", "Live"), "Live Forever (Live)");
        assert_eq!(with_version("Song (Live at Wembley)", "Live"), "Song (Live at Wembley)");
        assert_eq!(with_version("Song (Credit Mix)", "Edit"), "Song (Credit Mix) (Edit)");
        assert_eq!(with_version("Song", ""), "Song");
    }

    #[test]
    fn notes_that_say_nothing_about_the_recording_are_ignored() {
        assert_eq!(comparable("Song (feat. Live) [2011 Remaster]"), "song");
        assert_eq!(comparable("Song - From \"The Film\""), "song");
        assert_eq!(comparable("Song (Deluxe Edition) - Remastered Version"), "song");
        assert_eq!(comparable("(I Can't Get No) Satisfaction - Mono"), "icantgetnosatisfaction");
        assert_eq!(comparable("Love - Hate - Radio Edit"), "lovehateradioedit");
        assert_eq!(telling("(I Can't Get No) Satisfaction - Mono").collect::<Vec<_>>(), ["I Can't Get No", "Satisfaction"]);
    }
}
