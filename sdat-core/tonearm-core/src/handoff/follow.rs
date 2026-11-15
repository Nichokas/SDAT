//! Follows one of TIDAL's change event streams. Every event only means
//! "read the state again", so the event format does not matter.

use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::Cmd;

/// Each connection is cut after this long and reopened, so a dead one is
/// noticed (the server sends a keepalive about every 15 s). It also bounds
/// how long a stopped follower blocked in a read lives on: a blocking
/// request cannot be interrupted.
const CONNECTION: Duration = Duration::from_mins(1);
const MAX_BACKOFF: Duration = Duration::from_mins(1);

/// A stream thread; it ends within one keepalive after [`Self::stop`] or drop.
#[derive(Debug)]
pub(super) struct Follower {
    pub url: String,
    stop: Arc<AtomicBool>,
    /// Not connected (also before the first connection, and for good when
    /// the thread could not start), so the worker polls instead.
    down: Arc<AtomicBool>,
}

impl Follower {
    pub(super) fn spawn(url: String, tx: Sender<Cmd>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let down = Arc::new(AtomicBool::new(true));
        let (u, s, d) = (url.clone(), Arc::clone(&stop), Arc::clone(&down));
        let spawned = std::thread::Builder::new().name("handoff-follow".into()).spawn(move || run(&u, &tx, &s, &d));
        if let Err(e) = spawned {
            log::warn!("handoff: no stream thread error={e}");
        }
        Self { url, stop, down }
    }

    pub(super) fn is_down(&self) -> bool {
        self.down.load(Ordering::Relaxed)
    }
}

impl Drop for Follower {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The user token goes only to TIDAL hosts over TLS.
fn trusted(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some_and(|h| h == "tidal.com" || h.ends_with(".tidal.com")))
}

fn run(url: &str, tx: &Sender<Cmd>, stop: &AtomicBool, down: &AtomicBool) {
    if !trusted(url) {
        log::warn!("handoff: not following an event stream outside TIDAL url={url}");
        return;
    }
    let client = match crate::http::client_builder().timeout(CONNECTION).build() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("handoff: no stream client error={e}");
            return;
        }
    };
    let mut backoff = Duration::from_secs(1);
    while !stop.load(Ordering::Relaxed) {
        let opened = Instant::now();
        let resp = crate::api::user_bearer().map_err(|e| e.to_string()).and_then(|t| {
            client.get(url).bearer_auth(t).header("Accept", "text/event-stream").send().map_err(|e| e.to_string())
        });
        match resp {
            Ok(r) if r.status().is_success() => {
                down.store(false, Ordering::Relaxed);
                // Changes made while this stream was closed.
                if tx.send(Cmd::Changed).is_err() {
                    return;
                }
                for line in BufReader::new(r).lines() {
                    let Ok(line) = line else { break };
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    if line.starts_with("data:") && !line.contains("SUBSCRIBE_SUCCESS") && tx.send(Cmd::Changed).is_err() {
                        return;
                    }
                }
            }
            Ok(r) => log::info!("handoff: stream refused status={}", r.status()),
            Err(e) => log::info!("handoff: stream failed error={e}"),
        }
        down.store(true, Ordering::Relaxed);
        if opened.elapsed() > Duration::from_secs(10) {
            backoff = Duration::from_secs(1);
            continue;
        }
        let until = Instant::now() + backoff;
        while Instant::now() < until && !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(500));
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn follows_tidal_streams_only() {
        assert!(super::trusted("https://pulsar.tidal.com/userPlaybackStates/x"));
        assert!(!super::trusted("http://pulsar.tidal.com/x"));
        assert!(!super::trusted("https://pulsar.tidal.com.evil.example/x"));
        assert!(!super::trusted("https://eviltidal.com/x"));
    }
}
