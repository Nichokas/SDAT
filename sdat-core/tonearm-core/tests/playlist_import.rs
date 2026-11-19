//! Importing a playlist asks TIDAL for the tracks by ISRC, searches the ones it does not
//! find, and writes a new playlist with the matches in the original order.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tonearm_core::auth::{self, TokenStore};
use tonearm_core::config::{self, Config};
use tonearm_core::playlist_import::{Stage, import_playlist};
use tonearm_core::spotify::{SpotifyPlaylist, SpotifyTrack};

struct Local(String);

impl Config for Local {
    fn api_base_url(&self) -> String { self.0.clone() }
    fn auth_base_url(&self) -> String { self.0.clone() }
    fn openapi_base_url(&self) -> String { self.0.clone() }
}

struct SignedIn;

impl TokenStore for SignedIn {
    fn has_refresh_token(&self) -> bool { true }
    fn get_refresh_token(&self) -> String { "refresh".into() }
    fn user_id(&self) -> String { "7".into() }
}

fn track(title: &str, artist: &str, secs: u64, isrc: &str) -> SpotifyTrack {
    SpotifyTrack { title: title.into(), artists: vec![artist.into()], duration: Duration::from_secs(secs), isrc: isrc.into() }
}

/// Answers each request by its path and records the writes.
fn answer(request_line: &str, body: &str, writes: &Mutex<Vec<String>>) -> (&'static str, &'static str) {
    let path = request_line.split(' ').nth(1).unwrap_or_default();
    if request_line.starts_with("POST /v1/oauth2/token") {
        ("200 OK", r#"{"access_token":"user-token","expires_in":3600}"#)
    } else if path.starts_with("/v1/sessions") {
        ("200 OK", r#"{"countryCode":"ES"}"#)
    } else if path.starts_with("/v2/tracks?filter%5Bisrc%5D=AAA%2CBBB%2CCCC&") {
        // BBB is unknown to TIDAL and CCC cannot be streamed.
        ("200 OK", r#"{"data":[
            {"id":"10","type":"tracks","attributes":{"isrc":"AAA","availability":["STREAM"]}},
            {"id":"30","type":"tracks","attributes":{"isrc":"CCC","availability":[]}}]}"#)
    } else if path.starts_with("/v2/searchResults?filter%5Bquery%5D=Second+Song+Band&") {
        ("200 OK", r#"{"data":[{"id":"r","relationships":{"tracks":{"data":[{"id":"21","type":"tracks"},{"id":"20","type":"tracks"}]}}}],"included":[
            {"id":"21","type":"tracks","attributes":{"title":"Second Song","duration":"PT9M","availability":["STREAM"]},"relationships":{"artists":{"data":[{"id":"a","type":"artists"}]}}},
            {"id":"20","type":"tracks","attributes":{"title":"Second Song (Remastered)","duration":"PT3M21S","availability":["STREAM"]},"relationships":{"artists":{"data":[{"id":"a","type":"artists"}]}}},
            {"id":"a","type":"artists","attributes":{"name":"Band"}}]}"#)
    } else if path.starts_with("/v2/searchResults?") {
        ("200 OK", r#"{"data":[{"id":"r"}]}"#)
    } else if request_line.starts_with("POST /v2/playlists?") {
        writes.lock().expect("writes").push(format!("create {body}"));
        ("201 Created", r#"{"data":{"id":"new-playlist","type":"playlists"}}"#)
    } else if request_line.starts_with("POST /v2/playlists/new-playlist/relationships/items?") {
        writes.lock().expect("writes").push(format!("add {body}"));
        ("200 OK", r#"{"data":[]}"#)
    } else {
        ("404 Not Found", "{}")
    }
}

#[test]
fn imports_matches_in_order_and_reports_the_rest() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock TIDAL");
    assert!(config::install(Box::new(Local(format!("http://{}", listener.local_addr().expect("mock address"))))));
    assert!(auth::install(Box::new(SignedIn)));
    let writes = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&writes);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("accept request");
            let mut request = BufReader::new(&mut stream);
            let mut request_line = String::new();
            request.read_line(&mut request_line).expect("read request line");
            let mut content_length = 0;
            loop {
                let mut header = String::new();
                request.read_line(&mut header).expect("read request header");
                if header == "\r\n" {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    content_length = value.trim().parse().expect("request body size");
                }
            }
            let mut body = vec![0; content_length];
            request.read_exact(&mut body).expect("read request body");
            let (status, answer) = answer(&request_line, &String::from_utf8_lossy(&body), &recorded);
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}", answer.len())
                .expect("write mock response");
        }
    });

    let source = SpotifyPlaylist {
        name: " Road trip ".into(),
        tracks: vec![
            track("Second Song - Remastered 2011", "Band", 200, "BBB"),
            track("First Song", "Band", 180, "AAA"),
            track("Third Song", "Band", 240, "CCC"),
            track("First Song", "Band", 180, "AAA"),
        ],
        skipped: 1,
    };
    let stages = Mutex::new(Vec::new());
    let report = import_playlist(&source, &|p| stages.lock().expect("stages").push((p.stage, p.done, p.total)), &|| false).expect("import");

    assert_eq!((report.playlist_id.as_str(), report.name.as_str(), report.imported, report.skipped), ("new-playlist", "Road trip", 3, 1));
    assert_eq!(report.missing, ["Band – Third Song"]);
    assert_eq!(
        *writes.lock().expect("writes"),
        [
            r#"create {"data":{"attributes":{"accessType":"UNLISTED","description":"Imported from Spotify","name":"Road trip"},"type":"playlists"}}"#,
            r#"add {"data":[{"id":"20","type":"tracks"},{"id":"10","type":"tracks"},{"id":"10","type":"tracks"}],"meta":{"onDuplicates":"ADD"}}"#,
        ]
    );
    let stages = stages.into_inner().expect("stages");
    assert_eq!(stages.first(), Some(&(Stage::Matching, 0, 4)));
    assert_eq!(stages.last(), Some(&(Stage::Saving, 3, 3)));
    assert!(stages.contains(&(Stage::Matching, 4, 4)), "every track is accounted for: {stages:?}");
}
