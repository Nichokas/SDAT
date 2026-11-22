use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use tonearm_core::auth::{self, TokenStore};
use tonearm_core::config::{self, Config};
use tonearm_core::state;

struct Local(String);

impl Config for Local {
    fn api_base_url(&self) -> String {
        self.0.clone()
    }
    fn auth_base_url(&self) -> String {
        self.0.clone()
    }
}

struct SwitchableAccount(Arc<Mutex<String>>);

impl TokenStore for SwitchableAccount {
    fn has_refresh_token(&self) -> bool {
        !self.user_id().is_empty()
    }
    fn get_refresh_token(&self) -> String {
        format!("refresh-{}", self.user_id())
    }
    fn user_id(&self) -> String {
        self.0.lock().expect("account lock").clone()
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> (String, Option<String>) {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).expect("read request line");
    let mut content_length = 0usize;
    let mut if_none_match = None;
    loop {
        let mut header = String::new();
        assert_ne!(reader.read_line(&mut header).expect("read header"), 0);
        if header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().expect("content-length");
            } else if name.eq_ignore_ascii_case("if-none-match") {
                if_none_match = Some(value.trim().to_owned());
            }
        }
    }
    if content_length > 0 {
        reader.read_exact(&mut vec![0; content_length]).expect("read body");
    }
    (request_line, if_none_match)
}

fn write_response(stream: &mut std::net::TcpStream, status: &str, etag: Option<&str>, body: &str) {
    let etag_header = etag.map_or_else(String::new, |e| format!("ETag: {e}\r\n"));
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{etag_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write response");
}

#[test]
fn reset_and_bust_do_not_block_on_http_or_cache_stale_data() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let base = format!("http://{}", listener.local_addr().expect("local addr"));
    assert!(config::install(Box::new(Local(base))));

    let account = Arc::new(Mutex::new("account-a".to_owned()));
    assert!(auth::install(Box::new(SwitchableAccount(Arc::clone(&account)))));

    let (reset_in_flight_tx, reset_in_flight_rx) = mpsc::channel();
    let (reset_release_tx, reset_release_rx) = mpsc::channel();

    let (not_mod_in_flight_tx, not_mod_in_flight_rx) = mpsc::channel();
    let (not_mod_release_tx, not_mod_release_rx) = mpsc::channel();

    let (bust_in_flight_tx, bust_in_flight_rx) = mpsc::channel();
    let (bust_release_tx, bust_release_rx) = mpsc::channel();

    let server = std::thread::spawn(move || {
        let mut fav_calls = 0usize;
        for stream in listener.incoming() {
            let mut stream = stream.expect("accept");
            let (request_line, if_none_match) = read_request(&mut stream);
            if request_line.starts_with("POST /v1/oauth2/token ") {
                write_response(&mut stream, "200 OK", None, r#"{"access_token":"tok","expires_in":3600}"#);
                continue;
            }
            if request_line.starts_with("GET /v1/sessions ") || request_line.starts_with("GET /v1/sessions?") {
                write_response(&mut stream, "200 OK", None, r#"{"countryCode":"US"}"#);
                continue;
            }
            fav_calls += 1;
            match fav_calls {
                1 => {
                    assert!(request_line.starts_with("GET /v1/users/account-a/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    reset_in_flight_tx.send(()).expect("signal reset in-flight");
                    reset_release_rx.recv().expect("release account-a fetch");
                    write_response(&mut stream, "200 OK", Some("\"etag-a\""), r#"{"TRACK":["track-a"]}"#);
                }
                2 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"etag-b\""), r#"{"TRACK":["track-b"]}"#);
                }
                3 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"album-e1\""), r#"{"ALBUM":["album-1"]}"#);
                }
                4 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match.as_deref(), Some("\"album-e1\""));
                    not_mod_in_flight_tx.send(()).expect("signal 304 in-flight");
                    not_mod_release_rx.recv().expect("release 304 fetch");
                    write_response(&mut stream, "304 Not Modified", None, "");
                }
                5 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"album-e2\""), r#"{"ALBUM":["album-2"]}"#);
                }
                6 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    bust_in_flight_tx.send(()).expect("signal bust in-flight");
                    bust_release_rx.recv().expect("release stale artist fetch");
                    write_response(&mut stream, "200 OK", Some("\"artist-e1\""), r#"{"ARTIST":["artist-old"]}"#);
                }
                7 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"artist-e2\""), r#"{"ARTIST":["artist-new"]}"#);
                    break;
                }
                other => panic!("unexpected favourites request #{other}: {request_line}"),
            }
        }
    });

    let fetch_a = std::thread::spawn(|| state::TRACKS.get());
    reset_in_flight_rx.recv_timeout(Duration::from_secs(5)).expect("account-a fetch reached server");

    *account.lock().expect("account lock") = "account-b".to_owned();
    let started = Instant::now();
    state::TRACKS.reset();
    assert!(started.elapsed() < Duration::from_millis(100), "reset() blocked for {:?}", started.elapsed());

    reset_release_tx.send(()).expect("unblock account-a response");
    assert_eq!(fetch_a.join().expect("fetch_a thread").expect("fetch_a result"), ["track-a"]);
    assert_eq!(state::TRACKS.get().expect("account-b fetch"), ["track-b"]);

    assert_eq!(state::ALBUMS.get().expect("seed albums"), ["album-1"]);
    state::ALBUMS.bust();
    let fetch_304 = std::thread::spawn(|| state::ALBUMS.get());
    not_mod_in_flight_rx.recv_timeout(Duration::from_secs(5)).expect("304 fetch reached server");

    *account.lock().expect("account lock") = "account-c".to_owned();
    let started = Instant::now();
    state::ALBUMS.reset();
    assert!(started.elapsed() < Duration::from_millis(100), "reset() blocked for {:?}", started.elapsed());

    not_mod_release_tx.send(()).expect("unblock 304 response");
    assert_eq!(fetch_304.join().expect("fetch_304 thread").expect("fetch_304 result"), ["album-1"]);
    assert_eq!(state::ALBUMS.get().expect("account-c albums"), ["album-2"]);

    let fetch_stale = std::thread::spawn(|| state::ARTISTS.get());
    bust_in_flight_rx.recv_timeout(Duration::from_secs(5)).expect("artist fetch reached server");
    state::ARTISTS.bust();
    bust_release_tx.send(()).expect("unblock stale artist fetch");
    assert_eq!(fetch_stale.join().expect("fetch_stale thread").expect("fetch_stale result"), ["artist-old"]);
    assert_eq!(state::ARTISTS.get().expect("fresh artists"), ["artist-new"]);

    server.join().expect("mock server");
}
