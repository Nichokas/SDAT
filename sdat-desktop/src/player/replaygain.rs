//! Port of `feature_replaygain.go`: rgvolume/rglimiter audio filter with
//! `ReplayGain` tags injected from TIDAL's normalization data.
use anyhow::Result;
use gst::prelude::*;
use gtk::prelude::SettingsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::model::{Normalization, SourceType, TrackManifest};
use crate::settings::{self, ReplayGainMode};

type NormalizationPair = (Option<Normalization>, Option<Normalization>);

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum PendingGapless {
    #[default]
    Empty,
    Armed(Option<NormalizationPair>),
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct ReplayGainState {
    current: Option<NormalizationPair>,
    pending_gapless: PendingGapless,
}

impl ReplayGainState {
    fn set_current(&mut self, norm: Option<NormalizationPair>) {
        self.current = norm;
        self.pending_gapless = PendingGapless::Empty;
    }

    fn set_pending_gapless(&mut self, norm: Option<NormalizationPair>) {
        self.pending_gapless = PendingGapless::Armed(norm);
    }

    fn promote_gapless(&mut self) {
        if let PendingGapless::Armed(next) = std::mem::take(&mut self.pending_gapless) {
            self.current = next;
        }
    }

    fn clear_pending_gapless(&mut self) {
        self.pending_gapless = PendingGapless::Empty;
    }

    fn active(&self) -> Option<NormalizationPair> {
        self.current
    }
}

/// State shared with the streaming thread (the Go code read signals directly).
static NORMALIZE: AtomicBool = AtomicBool::new(false);
static ALBUM_MODE: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<ReplayGainState> = Mutex::new(ReplayGainState { current: None, pending_gapless: PendingGapless::Empty });

fn norm_pair(m: &TrackManifest) -> Option<NormalizationPair> {
    (m.album_normalization.is_some() || m.track_normalization.is_some())
        .then_some((m.album_normalization, m.track_normalization))
}

pub fn set_current(m: Option<&TrackManifest>) {
    STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).set_current(m.and_then(norm_pair));
}

pub fn set_pending_gapless(m: Option<&TrackManifest>) {
    STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).set_pending_gapless(m.and_then(norm_pair));
}

pub fn promote_gapless() {
    STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).promote_gapless();
}

pub fn clear_pending_gapless() {
    STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clear_pending_gapless();
}

pub fn calculate_album_mode(mode: ReplayGainMode, source: Option<SourceType>) -> bool {
    match source {
        Some(SourceType::Album) => mode != ReplayGainMode::Track,
        None | Some(_) => mode == ReplayGainMode::Album,
    }
}

pub fn update_album_mode() {
    let src = super::source_changed().with(|s| s.as_ref().map(super::super::model::PlaybackSource::source_type));
    ALBUM_MODE.store(calculate_album_mode(settings::replay_gain_mode(), src), Ordering::Relaxed);
}

fn inject_tags(rgvolume: &gst::Element) {
    let Some((album, track)) = STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).active() else { return };
    let mut tags = gst::TagList::new();
    {
        let t = tags.make_mut();
        if let Some(a) = album {
            t.add::<gst::tags::AlbumPeak>(&a.peak, gst::TagMergeMode::Append);
            t.add::<gst::tags::AlbumGain>(&a.gain, gst::TagMergeMode::Append);
        }
        if let Some(tr) = track {
            t.add::<gst::tags::TrackPeak>(&tr.peak, gst::TagMergeMode::Append);
            t.add::<gst::tags::TrackGain>(&tr.gain, gst::TagMergeMode::Append);
        }
    }
    let album_mode = ALBUM_MODE.load(Ordering::Relaxed);
    rgvolume.set_property("album-mode", album_mode);
    let ok = rgvolume.static_pad("sink").is_some_and(|p| p.send_event(gst::event::Tag::new(tags)));
    if ok {
        log::info!("injected ReplayGain tags album={album:?} track={track:?} albumMode={album_mode}");
    } else {
        log::warn!("failed to send ReplayGain tag event");
    }
}

fn ghost(bin: &gst::Bin, name: &str, target: &gst::Pad) -> Result<()> {
    let g = gst::GhostPad::builder_with_target(target)?.name(name).build();
    bin.add_pad(&g)?;
    Ok(())
}

fn build_filter_bin() -> Result<gst::Bin> {
    let bin = gst::Bin::with_name("replaygain-bin");
    let rgvolume = gst::ElementFactory::make("rgvolume")
        .name("rgvolume")
        .property("album-mode", ALBUM_MODE.load(Ordering::Relaxed))
        .property("pre-amp", 0.0f64)
        .property("fallback-gain", 0.0f64)
        .build()?;
    let rglimiter = gst::ElementFactory::make("rglimiter").name("rglimiter").build()?;
    bin.add_many([&rgvolume, &rglimiter])?;
    rgvolume.link(&rglimiter)?;
    let sink = rgvolume.static_pad("sink").expect("rgvolume always has a sink pad");
    let rgv = rgvolume.downgrade();
    sink.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, info| {
        if let Some(gst::PadProbeData::Event(ev)) = &info.data {
            match ev.type_() {
                // Promote before the new stream's Segment arrives so a seek
                // of the outgoing track keeps its own tags while the next
                // stream receives the preloaded ones.
                gst::EventType::StreamStart => promote_gapless(),
                gst::EventType::Segment if NORMALIZE.load(Ordering::Relaxed) => {
                    if let Some(rgv) = rgv.upgrade() {
                        inject_tags(&rgv);
                    }
                }
                _ => {}
            }
        }
        gst::PadProbeReturn::Ok
    });
    ghost(&bin, "sink", &sink)?;
    ghost(&bin, "src", &rglimiter.static_pad("src").expect("rglimiter always has a src pad"))?;
    Ok(bin)
}

fn build_empty_bin() -> Result<gst::Bin> {
    let bin = gst::Bin::with_name("emptybin");
    let identity = gst::ElementFactory::make("identity").build()?;
    bin.add(&identity)?;
    ghost(&bin, "sink", &identity.static_pad("sink").expect("identity always has a sink pad"))?;
    ghost(&bin, "src", &identity.static_pad("src").expect("identity always has a src pad"))?;
    Ok(bin)
}

pub fn init(playbin: &gst::Element) -> Result<()> {
    update_album_mode();
    let filter = build_filter_bin()?;
    let empty = build_empty_bin()?;
    let pb = playbin.clone();
    settings::connect_changed(&settings::playback(), "normalize-volume", move |s| {
        let on = s.boolean("normalize-volume");
        NORMALIZE.store(on, Ordering::Relaxed);
        let bin = if on { &filter } else { &empty };
        pb.set_property("audio-filter", bin.upcast_ref::<gst::Element>());
    });
    settings::connect_changed(&settings::playback(), "replay-gain-mode", |_| update_album_mode());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn album_mode_matrix() {
        use ReplayGainMode::*;
        assert!(!calculate_album_mode(Auto, None));
        assert!(calculate_album_mode(Album, None));
        assert!(calculate_album_mode(Auto, Some(SourceType::Album)));
        assert!(!calculate_album_mode(Track, Some(SourceType::Album)));
        assert!(!calculate_album_mode(Auto, Some(SourceType::Playlist)));
        assert!(calculate_album_mode(Album, Some(SourceType::Playlist)));
    }

    #[test]
    fn gapless_preload_does_not_overwrite_current_until_promoted() {
        let a = (Some(Normalization { peak: 0.9, gain: -6.0 }), Some(Normalization { peak: 0.8, gain: -4.0 }));
        let b = (Some(Normalization { peak: 1.0, gain: -1.5 }), Some(Normalization { peak: 0.95, gain: -2.0 }));
        let c = (None, Some(Normalization { peak: 0.5, gain: 1.0 }));
        let mut s = ReplayGainState::default();

        s.set_current(Some(a));
        assert_eq!(s.active(), Some(a));

        // Preloading Track B leaves Track A active for seeks.
        s.set_pending_gapless(Some(b));
        assert_eq!(s.active(), Some(a));

        // StreamStart promotes Track B once; repeating promote is a no-op.
        s.promote_gapless();
        assert_eq!(s.active(), Some(b));
        s.promote_gapless();
        assert_eq!(s.active(), Some(b));

        // Clearing a canceled preload prevents it from applying later.
        s.set_pending_gapless(Some(c));
        s.clear_pending_gapless();
        s.promote_gapless();
        assert_eq!(s.active(), Some(b));

        // Starting a new track replaces current and clears any pending gapless preload.
        s.set_pending_gapless(Some(a));
        s.set_current(Some(c));
        assert_eq!(s.active(), Some(c));
        s.promote_gapless();
        assert_eq!(s.active(), Some(c));
    }

    #[test]
    fn gapless_preload_without_normalization_clears_current_on_promote() {
        let a = (Some(Normalization { peak: 0.9, gain: -6.0 }), Some(Normalization { peak: 0.8, gain: -4.0 }));
        let mut s = ReplayGainState::default();

        s.set_current(Some(a));
        assert_eq!(s.active(), Some(a));

        // Preloading a track without normalization keeps Track A active until StreamStart,
        // then clears current on promotion so Track A's tags are not reused.
        s.set_pending_gapless(None);
        assert_eq!(s.active(), Some(a));
        s.promote_gapless();
        assert_eq!(s.active(), None);
        s.promote_gapless();
        assert_eq!(s.active(), None);

        // A manifest with neither album nor track normalization maps to None and clears current.
        let empty_manifest = TrackManifest::default();
        assert_eq!(norm_pair(&empty_manifest), None);
        s.set_current(Some(a));
        s.set_pending_gapless(norm_pair(&empty_manifest));
        assert_eq!(s.active(), Some(a));
        s.promote_gapless();
        assert_eq!(s.active(), None);

        // Clearing an armed empty preload keeps current intact.
        s.set_current(Some(a));
        s.set_pending_gapless(None);
        s.clear_pending_gapless();
        s.promote_gapless();
        assert_eq!(s.active(), Some(a));
    }
}
