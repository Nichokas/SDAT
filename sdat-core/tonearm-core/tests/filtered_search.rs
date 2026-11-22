use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Sender};

use tonearm_core::config::{self, Config};
use tonearm_core::diag::{self, Event, Level, Sink};
use tonearm_core::error::CoreError;
use tonearm_core::service::{SearchTarget, search_tracks};

struct Local(String);

impl Config for Local {
    fn api_base_url(&self) -> String { self.0.clone() }
    fn auth_base_url(&self) -> String { self.0.clone() }
    fn openapi_base_url(&self) -> String { self.0.clone() }
}

struct Events(Sender<Event>);

impl Sink for Events {
    fn on_event(&self, event: &Event) {
        let _ = self.0.send(event.clone());
    }
}

#[test]
fn filtered_search_uses_opaque_id_and_reports_a_rejected_page() {
    const TRACKS: &str = "/v2/searchResults/result%2Fid/relationships/tracks";
    const FIRST: &str = r#"{"data":[{"id":"1","type":"tracks"}],"included":[
        {"id":"1","type":"tracks","relationships":{"artists":{"data":[{"id":"a-track","type":"artists"}]},"albums":{"data":[{"id":"alb-1","type":"albums"}]}}},
        {"id":"alb-1","type":"albums","attributes":{"title":"Compilation"},"relationships":{"artists":{"data":[{"id":"a-album","type":"artists"}]}}},
        {"id":"a-track","type":"artists","attributes":{"name":"Track Artist"}},
        {"id":"a-album","type":"artists","attributes":{"name":"Various Artists"}}
    ],"links":{"meta":{"nextCursor":"next+page"}}}"#;
    const SECOND: &str = r#"{"data":[{"id":"2","type":"tracks"}],"included":[{"id":"2","type":"tracks"}]}"#;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock TIDAL");
    assert!(config::install(Box::new(Local(format!("http://{}", listener.local_addr().expect("mock address"))))));
    let (events_tx, events_rx) = mpsc::channel();
    diag::init(None, Some(Box::new(Events(events_tx))));

    let server = std::thread::spawn(move || {
        let first_request = format!("GET {TRACKS}?include=tracks.artists%2Ctracks.albums.coverArt&");
        let next_request = format!("GET {TRACKS}?page%5Bcursor%5D=next%2Bpage&include=tracks.artists%2Ctracks.albums.coverArt&");
        for (expected, status, body) in [
            ("GET /v1/country/context?", "200 OK", r#"{"countryCode":"US"}"#),
            ("POST /v1/oauth2/token ", "200 OK", r#"{"access_token":"test-token","expires_in":3600}"#),
            ("GET /v2/searchResults?filter%5Bquery%5D=just+keep+watching&", "200 OK", r#"{"data":[{"id":"result/id"}]}"#),
            (first_request.as_str(), "200 OK", FIRST),
            (next_request.as_str(), "400 Bad Request", r#"{"errors":[{"detail":"private response body"}]}"#),
            (next_request.as_str(), "200 OK", SECOND),
            (first_request.as_str(), "200 OK", FIRST),
        ] {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = BufReader::new(&mut stream);
            let mut request_line = String::new();
            request.read_line(&mut request_line).expect("read request");
            assert!(request_line.starts_with(expected), "expected {expected}; got {request_line}");
            let mut content_length = 0;
            loop {
                let mut header = String::new();
                assert_ne!(request.read_line(&mut header).expect("read header"), 0);
                if header == "\r\n" {
                    break;
                }
                if let Some(("content-length", value)) = header.split_once(':') {
                    content_length = value.trim().parse().expect("request body size");
                }
            }
            request.read_exact(&mut vec![0; content_length]).expect("read request body");
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                .expect("write mock response");
        }
    });

    let mut tracks = search_tracks(SearchTarget::Query("just keep watching"));
    let first_page = tracks.next_page().expect("first page");
    assert_eq!(first_page[0].id(), "1");
    assert_eq!(first_page[0].artist_names(), ["Track Artist"]);
    assert!(matches!(tracks.next_page(), Err(CoreError::Http { status: 400, .. })));

    let event = events_rx.try_recv().expect("search error reached the diagnostics sink");
    assert_eq!(event.level, Level::Error);
    assert_eq!(event.message, "TIDAL search tracks failed: HTTP 400");
    assert!(events_rx.try_recv().is_err());

    assert_eq!(tracks.next_page().expect("retry the same page")[0].id(), "2");
    assert!(tracks.is_consumed());
    assert_eq!(search_tracks(SearchTarget::ResultId("result/id")).next_page().expect("first page by ID")[0].id(), "1");
    server.join().expect("mock server");
}
