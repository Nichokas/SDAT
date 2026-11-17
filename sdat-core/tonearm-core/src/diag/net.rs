use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::time::Instant;

use reqwest::blocking::{RequestBuilder, Response};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetCategory {
    Tidal,
    Images,
    LastFm,
    ListenBrainz,
    Other,
}

impl NetCategory {
    pub const ALL: [Self; 5] = [Self::Tidal, Self::Images, Self::LastFm, Self::ListenBrainz, Self::Other];

    pub fn name(self) -> &'static str {
        match self {
            Self::Tidal => "TIDAL",
            Self::Images => "Images",
            Self::LastFm => "Last.fm",
            Self::ListenBrainz => "ListenBrainz",
            Self::Other => "Other",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Totals of the requests of one category since the process started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetStat {
    pub category: NetCategory,
    pub requests: u64,
    /// Transport errors and HTTP statuses of 400 and above.
    pub failures: u64,
    /// Sum of the `Content-Length` of the responses that sent one.
    pub bytes_in: u64,
    /// Time until the response headers arrived, summed over all requests.
    pub total_ms: u64,
    pub max_ms: u64,
    pub last_status: Option<u16>,
}

struct Counter {
    requests: AtomicU64,
    failures: AtomicU64,
    bytes_in: AtomicU64,
    total_ms: AtomicU64,
    max_ms: AtomicU64,
    /// 0 until a response arrived.
    last_status: AtomicU32,
}

static COUNTERS: [Counter; 5] = [const {
    Counter {
        requests: AtomicU64::new(0),
        failures: AtomicU64::new(0),
        bytes_in: AtomicU64::new(0),
        total_ms: AtomicU64::new(0),
        max_ms: AtomicU64::new(0),
        last_status: AtomicU32::new(0),
    }
}; 5];

pub(super) fn classify(host: Option<&str>) -> NetCategory {
    match host {
        Some("resources.tidal.com") => NetCategory::Images,
        Some(h) if h == "tidal.com" || h.ends_with(".tidal.com") => NetCategory::Tidal,
        Some("ws.audioscrobbler.com") => NetCategory::LastFm,
        Some("api.listenbrainz.org") => NetCategory::ListenBrainz,
        _ => NetCategory::Other,
    }
}

fn record(c: &Counter, elapsed_ms: u64, outcome: &reqwest::Result<Response>) {
    c.requests.fetch_add(1, Relaxed);
    c.total_ms.fetch_add(elapsed_ms, Relaxed);
    c.max_ms.fetch_max(elapsed_ms, Relaxed);
    match outcome {
        Ok(resp) => {
            let status = resp.status();
            c.last_status.store(u32::from(status.as_u16()), Relaxed);
            if status.as_u16() >= 400 {
                c.failures.fetch_add(1, Relaxed);
            }
            if let Some(len) = resp.content_length() {
                c.bytes_in.fetch_add(len, Relaxed);
            }
        }
        Err(_) => {
            c.failures.fetch_add(1, Relaxed);
        }
    }
}

/// `RequestBuilder::send` that also counts the request under its host's category for the diagnostics report.
/// Every HTTP request of the core and the apps goes through here.
pub fn send(rb: RequestBuilder) -> reqwest::Result<Response> {
    let (client, request) = rb.build_split();
    let request = match request {
        Ok(request) => request,
        Err(e) => {
            let outcome = Err(e);
            record(&COUNTERS[NetCategory::Other.index()], 0, &outcome);
            return outcome;
        }
    };
    let counter = &COUNTERS[classify(request.url().host_str()).index()];
    let started = Instant::now();
    let outcome = client.execute(request);
    record(counter, u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX), &outcome);
    outcome
}

pub(super) fn snapshot() -> Vec<NetStat> {
    NetCategory::ALL
        .into_iter()
        .map(|category| {
            let c = &COUNTERS[category.index()];
            NetStat {
                category,
                requests: c.requests.load(Relaxed),
                failures: c.failures.load(Relaxed),
                bytes_in: c.bytes_in.load(Relaxed),
                total_ms: c.total_ms.load(Relaxed),
                max_ms: c.max_ms.load(Relaxed),
                last_status: u16::try_from(c.last_status.load(Relaxed)).ok().filter(|s| *s != 0),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn hosts_map_to_their_category() {
        for h in ["tidal.com", "api.tidal.com", "openapi.tidal.com", "auth.tidal.com", "login.tidal.com"] {
            assert_eq!(classify(Some(h)), NetCategory::Tidal, "{h}");
        }
        assert_eq!(classify(Some("resources.tidal.com")), NetCategory::Images);
        assert_eq!(classify(Some("ws.audioscrobbler.com")), NetCategory::LastFm);
        assert_eq!(classify(Some("api.listenbrainz.org")), NetCategory::ListenBrainz);
        assert_eq!(classify(Some("example.com")), NetCategory::Other);
        assert_eq!(classify(Some("eviltidal.com")), NetCategory::Other);
        assert_eq!(classify(Some("evil-api.tidal.com.example.com")), NetCategory::Other);
        assert_eq!(classify(None), NetCategory::Other);
    }

    fn serve_once(status_line: &'static str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf);
            let reply = format!("HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            sock.write_all(reply.as_bytes()).expect("write");
        });
        format!("http://{addr}/")
    }

    fn other() -> NetStat {
        snapshot().into_iter().find(|s| s.category == NetCategory::Other).expect("Other row")
    }

    // The counters are process-wide, so this is the only test that sends requests.
    #[test]
    fn sending_counts_requests_failures_bytes_and_status() {
        let before = other();
        let client = reqwest::blocking::Client::new();

        let ok = send(client.get(serve_once("200 OK", "hello"))).expect("ok response");
        assert_eq!(ok.status().as_u16(), 200);
        let mid = other();
        assert_eq!(mid.requests, before.requests + 1);
        assert_eq!(mid.failures, before.failures);
        assert_eq!(mid.bytes_in, before.bytes_in + 5);
        assert_eq!(mid.last_status, Some(200));

        send(client.get(serve_once("503 Service Unavailable", ""))).expect("503 is still a response");
        let after_503 = other();
        assert_eq!(after_503.requests, before.requests + 2);
        assert_eq!(after_503.failures, before.failures + 1);
        assert_eq!(after_503.last_status, Some(503));

        let dead = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}/", dead.local_addr().expect("addr"));
        drop(dead);
        send(client.get(url)).expect_err("nothing listens");
        let end = other();
        assert_eq!(end.requests, before.requests + 3);
        assert_eq!(end.failures, before.failures + 2);
        assert_eq!(end.last_status, Some(503), "a transport error keeps the last status");
    }
}
