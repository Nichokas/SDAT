//! Port of `feature_replaygain.go`: rgvolume/rglimiter audio filter with
//! `ReplayGain` tags injected from TIDAL's normalization data.
use anyhow::Result;
use gst::prelude::*;
use gtk::prelude::SettingsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::model::{Normalization, SourceType, TrackManifest};
use crate::settings::{self, ReplayGainMode};

/// State shared with the streaming thread (the Go code read signals directly).
static NORMALIZE: AtomicBool = AtomicBool::new(false);
static ALBUM_MODE: AtomicBool = AtomicBool::new(false);
static ENQUEUED: Mutex<Option<(Option<Normalization>, Option<Normalization>)>> = Mutex::new(None);

pub fn set_enqueued(m: Option<&TrackManifest>) {
    *ENQUEUED.lock().unwrap() = m.map(|m| (m.album_normalization, m.track_normalization));
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
    let Some((album, track)) = *ENQUEUED.lock().unwrap() else { return };
    let mut tags = gst::TagList::new();
    {
        let t = tags.get_mut().unwrap();
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
    let sink = rgvolume.static_pad("sink").unwrap();
    let rgv = rgvolume.downgrade();
    sink.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, info| {
        if let Some(gst::PadProbeData::Event(ev)) = &info.data
            && ev.type_() == gst::EventType::Segment && NORMALIZE.load(Ordering::Relaxed)
                && let Some(rgv) = rgv.upgrade() {
                    inject_tags(&rgv);
                }
        gst::PadProbeReturn::Ok
    });
    ghost(&bin, "sink", &sink)?;
    ghost(&bin, "src", &rglimiter.static_pad("src").unwrap())?;
    Ok(bin)
}

fn build_empty_bin() -> Result<gst::Bin> {
    let bin = gst::Bin::with_name("emptybin");
    let identity = gst::ElementFactory::make("identity").build()?;
    bin.add(&identity)?;
    ghost(&bin, "sink", &identity.static_pad("sink").unwrap())?;
    ghost(&bin, "src", &identity.static_pad("src").unwrap())?;
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
}
