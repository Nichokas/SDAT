//! Port of `feature_audioquality.go`: reports codec/bit depth/sample rate.
use gst::prelude::*;

use crate::global_signal;
use crate::signals::State;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Codec(pub String);

pub const CODEC_FLAC: &str = "Free Lossless Audio Codec (FLAC)";
pub const CODEC_AAC: &str = "MPEG-4 AAC";

impl std::fmt::Display for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0.as_str() {
            CODEC_FLAC => "FLAC",
            CODEC_AAC => "AAC",
            _ => "unknown codec",
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamQuality {
    pub codec: Codec,
    pub bit_depth: i32,
    pub bit_rate: u32,
    pub sample_rate: i32,
}

global_signal!(pub fn audio_stream_quality() -> State<StreamQuality> = State::new(StreamQuality::default()));

pub fn on_tags_received(tags: &gst::TagListRef) {
    let Some(codec) = tags.get::<gst::tags::AudioCodec>() else {
        log::debug!("failed to get codec from tags");
        return;
    };
    let codec = Codec(codec.get().to_string());
    let is_aac = codec.0 == CODEC_AAC;
    audio_stream_quality().notify(|mut q| {
        q.codec = codec;
        q
    });
    if !is_aac {
        return;
    }
    match tags.get::<gst::tags::MaximumBitrate>() {
        Some(br) => {
            let br = br.get();
            audio_stream_quality().notify(|mut q| {
                q.bit_rate = br;
                q
            });
        }
        None => log::debug!("failed to get bitrate from tags"),
    }
}

pub fn bit_depth_for_format(format: &str) -> i32 {
    match format {
        "S16LE" => 16,
        "S24LE" | "S24_32LE" => 24,
        "S32LE" => 32,
        _ => 0,
    }
}

/// Called (from a streaming thread) for every element added to playbin.
pub fn on_deep_element_added(element: &gst::Element) {
    let Some(factory) = element.factory() else { return };
    if factory.name() != "decodebin3" {
        return;
    }
    element.connect_pad_added(|_, pad| {
        if pad.direction() != gst::PadDirection::Src {
            return;
        }
        pad.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, |pad, info| {
            let Some(gst::PadProbeData::Event(ev)) = &info.data else { return gst::PadProbeReturn::Ok };
            if ev.type_() != gst::EventType::Caps {
                return gst::PadProbeReturn::Ok;
            }
            let Some(caps) = pad.current_caps() else { return gst::PadProbeReturn::Ok };
            let Some(s) = caps.structure(0) else { return gst::PadProbeReturn::Ok };
            if s.name() != "audio/x-raw" {
                return gst::PadProbeReturn::Ok;
            }
            let depth = bit_depth_for_format(s.get::<&str>("format").unwrap_or(""));
            let rate = s.get::<i32>("rate").ok();
            glib::MainContext::default().invoke(move || {
                audio_stream_quality().notify(|mut q| {
                    q.bit_depth = depth;
                    if let Some(r) = rate {
                        q.sample_rate = r;
                    }
                    q
                });
            });
            gst::PadProbeReturn::Ok
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codec_names_and_depths() {
        assert_eq!(Codec(CODEC_FLAC.into()).to_string(), "FLAC");
        assert_eq!(Codec(CODEC_AAC.into()).to_string(), "AAC");
        assert_eq!(Codec("x".into()).to_string(), "unknown codec");
        assert_eq!(bit_depth_for_format("S24_32LE"), 24);
        assert_eq!(bit_depth_for_format("F32LE"), 0);
    }
}
