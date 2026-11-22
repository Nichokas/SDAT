use std::path::PathBuf;

use tonearm_core::audio_cache;

use crate::playback::gain;
use crate::{FfiError, Gain, Quality, Track, run};

/// A cached track. Play `playlist_path` (byte-range HLS) with `ExoPlayer`, `mp4_path` with `AVPlayer`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct CachedTrack {
    pub track: Track,
    pub mp4_path: String,
    pub playlist_path: String,
    /// Tier TIDAL delivered.
    pub quality: Quality,
    pub formats: Vec<String>,
    pub track_gain: Option<Gain>,
    pub album_gain: Option<Gain>,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct CacheUsage {
    pub bytes: u64,
    pub tracks: u32,
    pub limit_bytes: u64,
}

fn cached_track(c: audio_cache::Cached) -> Option<CachedTrack> {
    let track = serde_json::from_slice(&c.label).ok()?;
    Some(CachedTrack {
        track,
        mp4_path: c.mp4.to_string_lossy().into_owned(),
        playlist_path: c.playlist.to_string_lossy().into_owned(),
        quality: c.quality.into(),
        formats: c.formats,
        track_gain: gain(c.track_norm),
        album_gain: gain(c.album_norm),
        bytes: c.bytes,
    })
}

/// Opens the cache in `dir`, a folder of its own inside an OS cache directory: opening deletes every file in `dir`
/// the cache does not list. Call once at startup; a later call only applies the limit.
#[uniffi::export]
pub fn audio_cache_open(dir: String, limit_bytes: u64) -> Result<(), FfiError> {
    Ok(audio_cache::install(PathBuf::from(dir), limit_bytes)?)
}

/// Size limits to offer in Settings; 0 is Off.
#[uniffi::export]
pub fn audio_cache_limit_presets() -> Vec<u64> {
    audio_cache::LIMIT_PRESETS.to_vec()
}

#[uniffi::export]
pub fn audio_cache_limit_default() -> u64 {
    audio_cache::LIMIT_DEFAULT
}

/// The cached copy of a track at `quality` or better, without the network.
///
/// Call it for what plays now and what plays next: the files of the last two tracks looked up stay on disk.
#[uniffi::export]
pub fn audio_cache_lookup(track_id: String, quality: Quality) -> Option<CachedTrack> {
    audio_cache::lookup(&track_id, quality.into()).and_then(cached_track)
}

/// Downloads `track` into the cache unless a copy at `quality` or better is there.
///
/// A fill of a track that is already being filled waits for that one. Fails with `Cancelled` after
/// `audio_cache_cancel` or `audio_cache_purge`, and while the cache is off.
#[uniffi::export]
pub async fn audio_cache_fill(track: Track, quality: Quality) -> Result<CachedTrack, FfiError> {
    let id = track.id.clone();
    let label = serde_json::to_vec(&Track { playlist_item_id: None, ..track }).map_err(|e| FfiError::Decode { detail: e.to_string() })?;
    let c = run(move || audio_cache::fill(&id, quality.into(), label)).await?;
    cached_track(c).ok_or_else(|| FfiError::Decode { detail: "cached track label".into() })
}

/// Stops a running fill of `track_id`; it deletes its partial file.
#[uniffi::export]
pub fn audio_cache_cancel(track_id: String) {
    audio_cache::cancel(&track_id);
}

/// Most recently played first.
#[uniffi::export]
pub fn audio_cache_tracks() -> Vec<CachedTrack> {
    audio_cache::entries().into_iter().filter_map(cached_track).collect()
}

#[uniffi::export]
pub fn audio_cache_usage() -> CacheUsage {
    let u = audio_cache::usage();
    CacheUsage { bytes: u.bytes, tracks: u.tracks, limit_bytes: u.limit }
}

/// Applies a new limit now, evicting the least recently played tracks. 0 turns the cache off.
#[uniffi::export]
pub fn audio_cache_set_limit(bytes: u64) {
    audio_cache::set_limit(bytes);
}

#[uniffi::export]
pub fn audio_cache_remove(track_id: String) {
    audio_cache::remove(&track_id);
}

/// Empties the cache. The files of what is playing go once it stops playing.
#[uniffi::export]
pub fn audio_cache_purge() {
    audio_cache::purge();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonearm_core::model::TrackQuality;

    fn cached(label: Vec<u8>) -> audio_cache::Cached {
        audio_cache::Cached {
            track_id: "7".into(),
            mp4: "/c/7.mp4".into(),
            playlist: "/c/7.m3u8".into(),
            quality: TrackQuality::Lossless,
            formats: vec!["FLAC".into()],
            track_norm: None,
            album_norm: None,
            bytes: 9,
            label,
        }
    }

    #[test]
    fn the_label_round_trips_a_track() {
        let track = Track { id: "7".into(), title: "Song".into(), duration_secs: 200, ..Track::default() };
        let t = cached_track(cached(serde_json::to_vec(&track).unwrap())).unwrap();
        assert_eq!((t.track.id.as_str(), t.track.title.as_str(), t.track.duration_secs), ("7", "Song", 200));
        assert_eq!(t.quality, Quality::Cd);
        assert_eq!(t.playlist_path, "/c/7.m3u8");
        assert!(cached_track(cached(b"{\"id\":\"7\"}".to_vec())).is_some(), "fields a newer app adds decode as defaults");
        assert!(cached_track(cached(Vec::new())).is_none());
    }
}
