//! Streams (HLS for `AVPlayer`), lyrics and the infinite shuffle queue.

use std::sync::{Arc, Mutex, PoisonError};

use tonearm_core::discover::{self, InfiniteShuffle};
use tonearm_core::service;

use crate::convert::tracks;
use crate::{FfiError, Mix, Quality, Track, TrackPager, run};

/// `ReplayGain` of a track or album (dB and linear peak).
#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct Gain {
    pub gain_db: f64,
    pub peak: f64,
}

/// A playable stream: an HLS master playlist (fMP4, unencrypted).
#[derive(Debug, Clone, uniffi::Record)]
pub struct Stream {
    pub url: String,
    /// Tier actually delivered (may be lower than requested).
    pub quality: Quality,
    /// `HEAACV1`, `AACLC`, `FLAC`, `FLAC_HIRES`.
    pub formats: Vec<String>,
    pub track_gain: Option<Gain>,
    pub album_gain: Option<Gain>,
}

fn gain(n: Option<tonearm_core::model::Normalization>) -> Option<Gain> {
    n.map(|n| Gain { gain_db: n.gain, peak: n.peak })
}

/// HLS stream of a track at `quality`. URLs expire: fetch right before playing.
#[uniffi::export]
pub async fn stream(track_id: String, quality: Quality) -> Result<Stream, FfiError> {
    let m = run(move || service::get_track_manifest_hls(&track_id, quality.into())).await?;
    Ok(Stream { url: m.url, quality: m.quality.into(), formats: m.formats, track_gain: gain(m.track_normalization), album_gain: gain(m.album_normalization) })
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Lyrics {
    pub text: String,
    /// `text` is LRC (`[mm:ss.xx] line`).
    pub synced: bool,
}

#[uniffi::export]
pub async fn lyrics(track_id: String) -> Result<Option<Lyrics>, FfiError> {
    Ok(run(move || service::get_lyrics(&track_id)).await?.map(|l| Lyrics { text: l.text, synced: l.synced }))
}

/// Radio (mix) of a track, `None` if TIDAL has none.
#[uniffi::export]
pub async fn track_radio(track_id: String) -> Result<Option<Arc<TrackPager>>, FfiError> {
    let r = run(move || service::get_track_radio(&track_id)).await?;
    Ok(r.map(|p| TrackPager::new(service::get_playlist_tracks(&p.id))))
}

/// Endless shuffle over Discover's mixes, then radios of what it played.
#[derive(Debug, uniffi::Object)]
pub struct Shuffle(Arc<Mutex<InfiniteShuffle>>);

#[uniffi::export]
impl Shuffle {
    #[uniffi::constructor]
    pub fn new(mixes: Vec<Mix>) -> Arc<Self> {
        let refs: Vec<discover::MixRef> = mixes.iter().map(Into::into).collect();
        Arc::new(Shuffle(Arc::new(Mutex::new(InfiniteShuffle::from_mixes(&refs)))))
    }
    /// At least `min` new tracks; empty once every source is exhausted.
    pub async fn next_batch(&self, min: u32) -> Result<Vec<Track>, FfiError> {
        let s = Arc::clone(&self.0);
        let v = run(move || s.lock().unwrap_or_else(PoisonError::into_inner).next_batch(min as usize)).await?;
        Ok(tracks(&v))
    }
}
