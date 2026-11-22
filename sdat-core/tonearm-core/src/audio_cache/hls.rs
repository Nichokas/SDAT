//! The parts of TIDAL's HLS streams a cache fill needs (port of the Android app's `Hls.kt`).

use base64::Engine;
use url::Url;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub uri: Url,
    pub bandwidth: Option<u32>,
    pub codecs: Option<String>,
}

impl Variant {
    fn lossless(&self) -> bool {
        self.codecs.as_deref().map(str::to_lowercase).is_some_and(|c| c.contains("flac") || c.contains("alac"))
    }
}

/// `duration` is the EXTINF value as written, so the local playlist repeats it exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub uri: Url,
    pub duration: String,
    pub discontinuity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
    pub map: Url,
    pub segments: Vec<Segment>,
}

pub(crate) fn unsupported(what: &str) -> CoreError {
    CoreError::Other(format!("Unsupported stream format ({what})."))
}

pub(crate) fn encrypted() -> CoreError {
    CoreError::Other("This track's stream is encrypted (DRM) and can't be saved.".into())
}

/// A `data:` manifest's text, or `None` for a URL to fetch.
pub fn decode_data_url(url: &str) -> CoreResult<Option<String>> {
    let Some(rest) = url.strip_prefix("data:") else { return Ok(None) };
    let (meta, body) = rest.split_once(',').ok_or_else(|| unsupported("manifest"))?;
    let text = if meta.ends_with(";base64") {
        let bytes = base64::engine::general_purpose::STANDARD.decode(body).map_err(|_e| unsupported("manifest"))?;
        String::from_utf8(bytes).map_err(|_e| unsupported("manifest"))?
    } else {
        urlencoding::decode(body).map_err(|_e| unsupported("manifest"))?.into_owned()
    };
    Ok(Some(text))
}

pub fn is_master(text: &str) -> bool {
    text.contains("#EXT-X-STREAM-INF")
}

/// Lossless codec first, then the highest BANDWIDTH; `None` when `text` has no variants.
pub fn best_variant(text: &str, base: Option<&Url>) -> Option<Variant> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("#EXT-X-STREAM-INF:"))
        .filter_map(|(i, l)| {
            let uri = lines[i + 1..].iter().find(|n| !n.is_empty() && !n.starts_with('#'))?;
            Some(Variant { uri: resolve(base, uri)?, bandwidth: attr(l, "BANDWIDTH").and_then(|b| b.parse().ok()), codecs: attr(l, "CODECS") })
        })
        .max_by_key(|v| (v.lossless(), v.bandwidth.unwrap_or(0)))
}

/// An fMP4 media playlist; encrypted, byte-range, MPEG-TS and multi-init playlists are refused.
pub fn media(text: &str, base: Option<&Url>) -> CoreResult<Media> {
    let mut map: Option<Url> = None;
    let mut segments = Vec::new();
    let mut duration: Option<String> = None;
    let mut discontinuity = false;
    for l in text.lines().map(str::trim) {
        if l.starts_with("#EXT-X-KEY") || l.starts_with("#EXT-X-SESSION-KEY") {
            if attr(l, "METHOD").as_deref() != Some("NONE") {
                return Err(encrypted());
            }
        } else if l.starts_with("#EXT-X-BYTERANGE") || (l.starts_with("#EXT-X-MAP") && l.contains("BYTERANGE")) {
            return Err(unsupported("byte ranges"));
        } else if l.starts_with("#EXT-X-MAP") {
            let next = attr(l, "URI").and_then(|u| resolve(base, &u)).ok_or_else(|| unsupported("init segment URL"))?;
            // The cached file holds one init segment, so every fragment must follow the same one.
            if map.as_ref().is_some_and(|m| *m != next) {
                return Err(unsupported("several init segments"));
            }
            map = Some(next);
        } else if l.starts_with("#EXT-X-DISCONTINUITY") && !l.starts_with("#EXT-X-DISCONTINUITY-SEQUENCE") {
            discontinuity = true;
        } else if let Some(d) = l.strip_prefix("#EXTINF:") {
            duration = Some(d.split(',').next().unwrap_or_default().trim().to_owned());
        } else if !l.is_empty() && !l.starts_with('#') {
            let uri = resolve(base, l).ok_or_else(|| unsupported("segment URL"))?;
            let duration = duration.take().ok_or_else(|| unsupported("segment without duration"))?;
            segments.push(Segment { uri, duration, discontinuity });
            discontinuity = false;
        }
    }
    let Some(map) = map else { return Err(unsupported(if segments.is_empty() { "empty playlist" } else { "MPEG-TS" })) };
    if segments.is_empty() {
        return Err(unsupported("empty playlist"));
    }
    Ok(Media { map, segments })
}

/// A VOD playlist over one file holding the init segment and then every segment, back to back.
///
/// Players seek a concatenated fragmented MP4 only through such an index. `sizes` holds the init segment's size,
/// then each segment's.
pub fn local_playlist(file: &str, segments: &[Segment], sizes: &[u64]) -> String {
    use std::fmt::Write;
    let target = segments.iter().filter_map(|s| s.duration.parse::<f64>().ok()).fold(0.0, f64::max).ceil() as u64;
    let init = sizes.first().copied().unwrap_or(0);
    let mut out = format!(
        "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:{target}\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-MAP:URI=\"{file}\",BYTERANGE=\"{init}@0\"\n"
    );
    let mut offset = init;
    for (segment, size) in segments.iter().zip(sizes.iter().skip(1)) {
        if segment.discontinuity {
            out.push_str("#EXT-X-DISCONTINUITY\n");
        }
        let _ = write!(out, "#EXTINF:{},\n#EXT-X-BYTERANGE:{size}@{offset}\n{file}\n", segment.duration);
        offset += size;
    }
    out.push_str("#EXT-X-ENDLIST\n");
    out
}

/// Scanned left to right, so a quoted value is consumed whole and a comma or KEY= inside it is never a boundary.
fn attr(line: &str, key: &str) -> Option<String> {
    let mut rest = line.split_once(':')?.1;
    while !rest.is_empty() {
        let (name, after) = rest.split_once('=')?;
        let (value, next) = if let Some(quoted) = after.strip_prefix('"') {
            let (v, tail) = quoted.split_once('"')?;
            (v, tail.split_once(',').map_or("", |(_, t)| t))
        } else {
            after.split_once(',').unwrap_or((after, ""))
        };
        if name.trim() == key {
            return Some(value.trim().to_owned());
        }
        rest = next;
    }
    None
}

fn resolve(base: Option<&Url>, reference: &str) -> Option<Url> {
    match base {
        Some(b) => b.join(reference).ok(),
        None => Url::parse(reference).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://cdn.example/t/1/master.m3u8").unwrap()
    }

    #[test]
    fn best_variant_prefers_lossless_then_bandwidth() {
        let master = "#EXTM3U\n\
            #EXT-X-STREAM-INF:BANDWIDTH=320000,CODECS=\"mp4a.40.2\"\naac.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=900000,CODECS=\"flac\"\nflac-low.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=2500000,CODECS=\"flac\"\nflac-high.m3u8\n";
        let v = best_variant(master, Some(&base())).unwrap();
        assert_eq!(v.uri.as_str(), "https://cdn.example/t/1/flac-high.m3u8");
        assert_eq!(v.bandwidth, Some(2_500_000));
        assert_eq!(v.codecs.as_deref(), Some("flac"));
        assert!(is_master(master));
        assert_eq!(best_variant("#EXTM3U\n", Some(&base())), None);
    }

    #[test]
    fn media_playlist_resolves_init_and_segments_with_durations() {
        let text = "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:3.996,\nseg-1.mp4\n#EXTINF:2.5,\nhttps://other.example/seg-2.mp4\n#EXT-X-ENDLIST\n";
        let m = media(text, Some(&base())).unwrap();
        assert_eq!(m.map.as_str(), "https://cdn.example/t/1/init.mp4");
        let uris: Vec<&str> = m.segments.iter().map(|s| s.uri.as_str()).collect();
        assert_eq!(uris, ["https://cdn.example/t/1/seg-1.mp4", "https://other.example/seg-2.mp4"]);
        let durations: Vec<&str> = m.segments.iter().map(|s| s.duration.as_str()).collect();
        assert_eq!(durations, ["3.996", "2.5"]);
    }

    #[test]
    fn refuses_what_cant_be_cached() {
        let b = base();
        let refused = |text: &str| media(text, Some(&b)).unwrap_err().to_string();
        assert_eq!(refused("#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://x\"\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:4,\ns.mp4"), encrypted().to_string());
        assert_eq!(refused("#EXTINF:4,\ns.ts"), "Unsupported stream format (MPEG-TS).");
        assert_eq!(refused("#EXT-X-MAP:URI=\"i.mp4\",BYTERANGE=\"800@0\"\n#EXTINF:4,\ns.mp4"), "Unsupported stream format (byte ranges).");
        assert_eq!(
            refused("#EXT-X-MAP:URI=\"a.mp4\"\n#EXTINF:4,\ns1.mp4\n#EXT-X-MAP:URI=\"b.mp4\"\n#EXTINF:4,\ns2.mp4"),
            "Unsupported stream format (several init segments)."
        );
        let disguised = "#EXT-X-KEY:METHOD=AES-128,URI=\"https://cdn/key?METHOD=NONE\"\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:4,\ns.mp4";
        assert_eq!(refused(disguised), encrypted().to_string());
        let quoted_comma = "#EXT-X-MAP:URI=\"i.mp4\"\n#EXT-X-KEY:URI=\"https://cdn/key,METHOD=NONE\",METHOD=AES-128\n#EXTINF:4,\ns.mp4";
        assert_eq!(refused(quoted_comma), encrypted().to_string());
        assert_eq!(refused("#EXTM3U\n"), "Unsupported stream format (empty playlist).");
        media("#EXT-X-KEY:METHOD=NONE\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:4,\ns.mp4", Some(&b)).expect("METHOD=NONE is no encryption");
    }

    #[test]
    fn inline_manifests() {
        let text = "#EXTM3U\n#EXT-X-MAP:URI=\"https://a/i.mp4\"\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        assert_eq!(decode_data_url(&format!("data:application/vnd.apple.mpegurl;base64,{encoded}")).unwrap().as_deref(), Some(text));
        assert_eq!(decode_data_url("data:application/vnd.apple.mpegurl,%23EXTM3U%0A").unwrap().as_deref(), Some("#EXTM3U\n"));
        assert_eq!(decode_data_url("data:application/vnd.apple.mpegurl,https://a/b+c%20d.mp4").unwrap().as_deref(), Some("https://a/b+c d.mp4"));
        assert_eq!(decode_data_url("https://cdn.example/master.m3u8").unwrap(), None);
        decode_data_url("data:no-comma").unwrap_err();
    }

    #[test]
    fn local_playlist_indexes_the_concatenated_file() {
        let text = "#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:3.996,\ns1.mp4\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXT-X-DISCONTINUITY\n#EXTINF:2.5,\ns2.mp4";
        let m = media(text, Some(&base())).unwrap();
        assert_eq!(
            local_playlist("1.mp4", &m.segments, &[700, 1000, 400]),
            "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:4\n#EXT-X-PLAYLIST-TYPE:VOD\n\
             #EXT-X-MAP:URI=\"1.mp4\",BYTERANGE=\"700@0\"\n\
             #EXTINF:3.996,\n#EXT-X-BYTERANGE:1000@700\n1.mp4\n\
             #EXT-X-DISCONTINUITY\n#EXTINF:2.5,\n#EXT-X-BYTERANGE:400@1700\n1.mp4\n\
             #EXT-X-ENDLIST\n"
        );
    }
}
