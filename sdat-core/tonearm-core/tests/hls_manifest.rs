//! HLS track manifests (the iOS `AVPlayer` path), decoded from fixtures.

use tonearm_core::api::{client::manifest_query, openapi};
use tonearm_core::model::{TrackQuality, manifest_from_openapi};

fn decode(name: &str) -> tonearm_core::model::TrackManifest {
    let path = format!("{}/tests/fixtures/manifest_hls_{name}.json", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(path).expect("fixture");
    let m: openapi::TrackManifest = openapi::from_slice_lenient(&bytes).expect("decodes");
    manifest_from_openapi(&m)
}

#[test]
fn hls_query_asks_for_an_hls_master_playlist() {
    let f = openapi::manifest_formats_for_quality(TrackQuality::Lossless);
    let q = manifest_query(&f, openapi::MANIFEST_TYPE_HLS);
    let get = |k: &str| q.iter().find(|(n, _)| *n == k).map(|(_, v)| v.as_str());
    assert_eq!(get("manifestType"), Some("HLS"));
    assert_eq!(get("formats"), Some("HEAACV1,AACLC,FLAC"));
    assert_eq!(get("adaptive"), Some("false"));
    assert_eq!(get("uriScheme"), Some("HTTPS"));
    assert_eq!(get("usage"), Some("PLAYBACK"));
}

#[test]
fn each_tier_maps_to_its_formats() {
    let f = |q| openapi::manifest_formats_for_quality(q).join(",");
    assert_eq!(f(TrackQuality::High), "HEAACV1,AACLC");
    assert_eq!(f(TrackQuality::Lossless), "HEAACV1,AACLC,FLAC");
    assert_eq!(f(TrackQuality::HighResLossless), "HEAACV1,AACLC,FLAC,FLAC_HIRES");
}

#[test]
fn hls_fixtures_decode_for_all_three_tiers() {
    for (name, quality, last) in [
        ("aac", TrackQuality::High, "AACLC"),
        ("flac", TrackQuality::Lossless, "FLAC"),
        ("hires", TrackQuality::HighResLossless, "FLAC_HIRES"),
    ] {
        let m = decode(name);
        assert!(m.url.ends_with("master.m3u8?token=fixture"), "{name}: {}", m.url);
        assert_eq!(m.quality, quality, "{name}");
        assert_eq!(m.formats.last().map(String::as_str), Some(last));
        let t = m.track_normalization.expect("track replay gain");
        assert!((t.gain - -8.43).abs() < 1e-3 && (t.peak - 0.977_203).abs() < 1e-5);
        assert!(m.album_normalization.is_some());
    }
}
