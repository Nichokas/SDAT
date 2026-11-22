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
fn reset_bust_and_concurrent_fetches_behave_consistently() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let base = format!("http://{}", listener.local_addr().expect("local addr"));
    assert!(config::install(Box::new(Local(base))));

    let account = Arc::new(Mutex::new("account-a".to_owned()));
    assert!(auth::install(Box::new(SwitchableAccount(Arc::clone(&account)))));

    let (gate_entered_tx, gate_entered_rx) = mpsc::channel();
    let (gate_release_tx, gate_release_rx) = mpsc::channel();

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
                // 1-2: 200 OK invalidated by account switch + reset() -> retries for account-b.
                1 => {
                    assert!(request_line.starts_with("GET /v1/users/account-a/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    gate_entered_tx.send(1).expect("signal call 1");
                    gate_release_rx.recv().expect("release call 1");
                    write_response(&mut stream, "200 OK", Some("\"etag-a\""), r#"{"TRACK":["track-a"]}"#);
                }
                2 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"etag-b\""), r#"{"TRACK":["track-b"]}"#);
                }
                // 3: bust() clears ETag; 200 OK invalidated by sign-out + reset() -> returns empty without retrying.
                3 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    gate_entered_tx.send(3).expect("signal call 3");
                    gate_release_rx.recv().expect("release call 3");
                    write_response(&mut stream, "200 OK", Some("\"etag-b2\""), r#"{"TRACK":["track-b-stale"]}"#);
                }
                // 4-8: bust() (call 5) and reset() (call 7) drop ETag so fetches omit If-None-Match
                // and reject rogue 304 responses without corrupting cache; subsequent 200s succeed.
                4 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"album-e1\""), r#"{"ALBUM":["album-1"]}"#);
                }
                5 | 7 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "304 Not Modified", None, "");
                }
                6 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"album-e2\""), r#"{"ALBUM":["album-2"]}"#);
                }
                8 => {
                    assert!(request_line.starts_with("GET /v1/users/account-b/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"album-e3\""), r#"{"ALBUM":["album-3"]}"#);
                }
                // 9: Account switch without reset() omits the previous account's ETag.
                9 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"album-c1\""), r#"{"ALBUM":["album-c"]}"#);
                }
                // 10-11: Same-account bust() during HTTP -> discards stale 200 and retries without ETag.
                10 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    gate_entered_tx.send(10).expect("signal call 10");
                    gate_release_rx.recv().expect("release call 10");
                    write_response(&mut stream, "200 OK", Some("\"artist-e1\""), r#"{"ARTIST":["artist-old"]}"#);
                }
                11 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"artist-e2\""), r#"{"ARTIST":["artist-new"]}"#);
                }
                // 12-14: Continuous invalidation exhausts retry budget instead of returning stale data.
                12..=14 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    gate_entered_tx.send(fav_calls).expect("signal exhaustion call");
                    gate_release_rx.recv().expect("release exhaustion call");
                    write_response(&mut stream, "200 OK", Some("\"artist-stale\""), r#"{"ARTIST":["artist-stale"]}"#);
                }
                // 15: Concurrent cold-cache get() calls coalesce into a single HTTP request.
                15 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    gate_entered_tx.send(15).expect("signal call 15");
                    gate_release_rx.recv().expect("release call 15");
                    write_response(&mut stream, "200 OK", Some("\"pl-1\""), r#"{"PLAYLIST":["playlist-1"]}"#);
                }
                // 16-17: Coalesced callers with bust() omit stale ETag and both receive the newer response, never stale.
                16 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    gate_entered_tx.send(16).expect("signal call 16");
                    gate_release_rx.recv().expect("release call 16");
                    write_response(&mut stream, "200 OK", Some("\"pl-stale\""), r#"{"PLAYLIST":["playlist-stale"]}"#);
                }
                17 => {
                    assert!(request_line.starts_with("GET /v1/users/account-c/favorites/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"pl-2\""), r#"{"PLAYLIST":["playlist-2"]}"#);
                }
                // 18-19: Single-page Mixes fetch invalidated by account switch + reset() retries for account-d.
                18 => {
                    assert!(request_line.starts_with("GET /v2/favorites/mixes/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    gate_entered_tx.send(18).expect("signal call 18");
                    gate_release_rx.recv().expect("release call 18");
                    write_response(&mut stream, "200 OK", Some("\"mix-c\""), r#"{"cursor":"","content":["mix-c-stale"]}"#);
                }
                19 => {
                    assert!(request_line.starts_with("GET /v2/favorites/mixes/ids?"), "got {request_line}");
                    assert_eq!(if_none_match, None);
                    write_response(&mut stream, "200 OK", Some("\"mix-d\""), r#"{"cursor":"","content":["mix-d"]}"#);
                    break;
                }
                other => panic!("unexpected favourites request #{other}: {request_line}"),
            }
        }
        assert_eq!(fav_calls, 19);
    });

    // 1. 200 OK invalidated by account switch + reset().
    let fetch_a = std::thread::spawn(|| state::TRACKS.get());
    assert_eq!(gate_entered_rx.recv_timeout(Duration::from_secs(5)).expect("call 1"), 1);
    *account.lock().expect("account lock") = "account-b".to_owned();
    let started = Instant::now();
    state::TRACKS.reset();
    assert!(started.elapsed() < Duration::from_millis(100), "reset() blocked for {:?}", started.elapsed());
    gate_release_tx.send(()).expect("release call 1");
    assert_eq!(fetch_a.join().expect("fetch_a thread").expect("fetch_a result"), ["track-b"]);
    assert_eq!(state::TRACKS.get().expect("cached account-b tracks"), ["track-b"]);

    // 2. 200 OK invalidated by sign-out + reset().
    state::TRACKS.bust();
    let fetch_signout_200 = std::thread::spawn(|| state::TRACKS.get());
    assert_eq!(gate_entered_rx.recv_timeout(Duration::from_secs(5)).expect("call 3"), 3);
    account.lock().expect("account lock").clear();
    let started = Instant::now();
    state::TRACKS.reset();
    assert!(started.elapsed() < Duration::from_millis(100), "reset() blocked for {:?}", started.elapsed());
    gate_release_tx.send(()).expect("release call 3");
    assert_eq!(fetch_signout_200.join().expect("signout 200 thread").expect("signout 200 result"), Vec::<String>::new());

    // 3. bust() clears ETag and rejects unexpected 304 without restoring stale items.
    *account.lock().expect("account lock") = "account-b".to_owned();
    assert_eq!(state::ALBUMS.get().expect("seed albums"), ["album-1"]);
    state::ALBUMS.bust();
    let err = state::ALBUMS.get().expect_err("rogue 304 after bust must fail");
    assert!(err.to_string().contains("not modified"), "unexpected error: {err}");
    assert_eq!(state::ALBUMS.get().expect("fresh albums after bust"), ["album-2"]);
    assert_eq!(state::ALBUMS.get().expect("cached account-b albums"), ["album-2"]);

    // 4. reset() clears ETag and rejects unexpected 304 without corrupting cache.
    state::ALBUMS.reset();
    let err = state::ALBUMS.get().expect_err("rogue 304 after reset must fail");
    assert!(err.to_string().contains("not modified"), "unexpected error: {err}");
    assert_eq!(state::ALBUMS.get().expect("fresh albums after reset"), ["album-3"]);

    // 5. Account switch without reset() omits the previous account's ETag.
    *account.lock().expect("account lock") = "account-c".to_owned();
    assert_eq!(state::ALBUMS.get().expect("account-c albums"), ["album-c"]);
    assert_eq!(state::ALBUMS.get().expect("cached account-c albums"), ["album-c"]);

    // 6. Same-account bust() during HTTP discards stale response and retries.
    let fetch_stale = std::thread::spawn(|| state::ARTISTS.get());
    assert_eq!(gate_entered_rx.recv_timeout(Duration::from_secs(5)).expect("call 10"), 10);
    let started = Instant::now();
    state::ARTISTS.bust();
    assert!(started.elapsed() < Duration::from_millis(100), "bust() blocked for {:?}", started.elapsed());
    gate_release_tx.send(()).expect("release call 10");
    assert_eq!(fetch_stale.join().expect("fetch_stale thread").expect("fetch_stale result"), ["artist-new"]);
    assert_eq!(state::ARTISTS.get().expect("cached fresh artists"), ["artist-new"]);

    // 7. Continuous invalidation exhausts retries and errors instead of returning stale data.
    state::ARTISTS.reset();
    let fetch_exhausted = std::thread::spawn(|| state::ARTISTS.get());
    for expected in 12..=14 {
        assert_eq!(gate_entered_rx.recv_timeout(Duration::from_secs(5)).expect("exhaustion call"), expected);
        state::ARTISTS.bust();
        gate_release_tx.send(()).expect("release exhaustion call");
    }
    let err = fetch_exhausted.join().expect("exhausted thread").expect_err("expected invalidation error");
    assert!(err.to_string().contains("invalidated"), "unexpected error: {err}");

    // 8. Concurrent cold-cache get() calls coalesce into a single HTTP request.
    let first = std::thread::spawn(|| state::PLAYLISTS.get());
    assert_eq!(gate_entered_rx.recv_timeout(Duration::from_secs(5)).expect("call 15"), 15);
    let (started_tx, started_rx) = mpsc::channel();
    let second = {
        let tx = started_tx.clone();
        std::thread::spawn(move || {
            tx.send(()).expect("signal second");
            state::PLAYLISTS.get()
        })
    };
    let third = {
        let tx = started_tx.clone();
        std::thread::spawn(move || {
            tx.send(()).expect("signal third");
            state::PLAYLISTS.get()
        })
    };
    started_rx.recv_timeout(Duration::from_secs(5)).expect("second started");
    started_rx.recv_timeout(Duration::from_secs(5)).expect("third started");
    std::thread::sleep(Duration::from_millis(30));
    gate_release_tx.send(()).expect("release call 15");
    assert_eq!(first.join().expect("first thread").expect("first result"), ["playlist-1"]);
    assert_eq!(second.join().expect("second thread").expect("second result"), ["playlist-1"]);
    assert_eq!(third.join().expect("third thread").expect("third result"), ["playlist-1"]);

    // 9. Concurrent callers during a mid-flight bust() both receive the newer response without blocking bust().
    state::PLAYLISTS.bust();
    let older = std::thread::spawn(|| state::PLAYLISTS.get());
    assert_eq!(gate_entered_rx.recv_timeout(Duration::from_secs(5)).expect("call 16"), 16);
    let waiter = std::thread::spawn(move || {
        started_tx.send(()).expect("signal waiter");
        state::PLAYLISTS.get()
    });
    started_rx.recv_timeout(Duration::from_secs(5)).expect("waiter started");
    std::thread::sleep(Duration::from_millis(30));
    let started = Instant::now();
    state::PLAYLISTS.bust();
    assert!(started.elapsed() < Duration::from_millis(100), "bust() blocked for {:?}", started.elapsed());
    gate_release_tx.send(()).expect("release call 16");
    assert_eq!(older.join().expect("older thread").expect("older result"), ["playlist-2"]);
    assert_eq!(waiter.join().expect("waiter thread").expect("waiter result"), ["playlist-2"]);
    assert_eq!(state::PLAYLISTS.get().expect("cached newer playlists"), ["playlist-2"]);

    // 10. Single-page Mixes fetch invalidated by account switch + reset() discards stale mixes and retries.
    let fetch_mixes = std::thread::spawn(|| state::MIXES.get());
    assert_eq!(gate_entered_rx.recv_timeout(Duration::from_secs(5)).expect("call 18"), 18);
    *account.lock().expect("account lock") = "account-d".to_owned();
    let started = Instant::now();
    state::MIXES.reset();
    assert!(started.elapsed() < Duration::from_millis(100), "reset() blocked for {:?}", started.elapsed());
    gate_release_tx.send(()).expect("release call 18");
    assert_eq!(fetch_mixes.join().expect("fetch_mixes thread").expect("fetch_mixes result"), ["mix-d"]);
    assert_eq!(state::MIXES.get().expect("cached account-d mixes"), ["mix-d"]);

    server.join().expect("mock server");
}
