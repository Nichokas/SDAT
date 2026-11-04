//! Port of `internal/features/scrobbling`: `ListenBrainz` and Last.fm scrobblers
//! driven by a listening clock (scrobble after half the track or 4 minutes).
use std::cell::RefCell;

use crate::model::Track;
use crate::player::{self, PlaybackStatus};
use crate::settings;
pub use tonearm_core::scrobbling::*;

fn listenbrainz_configured() -> bool {
    settings::enable_listenbrainz() && !settings::listenbrainz_token().is_empty()
}

fn session_key() -> String {
    settings::lastfm_token()
}

pub fn lastfm_unconfigure() {
    settings::set_lastfm_token("");
}

// ---------------- dispatch ----------------

#[derive(Clone)]
struct Config {
    lb: bool,
    lb_url: String,
    lb_token: String,
    lfm_enabled: bool,
    lfm_sk: String,
}

fn config() -> Config {
    Config {
        lb: listenbrainz_configured(),
        lb_url: settings::listenbrainz_url(),
        lb_token: settings::listenbrainz_token(),
        lfm_enabled: settings::enable_lastfm(),
        lfm_sk: session_key(),
    }
}

fn now_playing(meta: TrackMeta) {
    let c = config();
    std::thread::spawn(move || {
        if c.lb {
            listenbrainz_send(&c.lb_url, &c.lb_token, &listenbrainz_request(&meta, None));
        }
        if lastfm_configured(c.lfm_enabled, &c.lfm_sk)
            && let Err(e) = lastfm_call("track.updateNowPlaying", &lastfm_track_params(&meta, &c.lfm_sk), true, true)
        {
            log::error!("error while updating now playing error={e:#}");
        }
    });
}

fn scrobble(meta: TrackMeta, listened_at: i64) {
    let c = config();
    std::thread::spawn(move || {
        if c.lb {
            listenbrainz_send(&c.lb_url, &c.lb_token, &listenbrainz_request(&meta, Some(listened_at)));
        }
        if lastfm_configured(c.lfm_enabled, &c.lfm_sk) {
            let mut p = lastfm_track_params(&meta, &c.lfm_sk);
            // Last.fm expects the time the track started playing.
            p.push(("timestamp", listened_at.to_string()));
            if let Err(e) = lastfm_call("track.scrobble", &p, true, true) {
                log::error!("error while scrobbling error={e:#}");
            }
        }
    });
}

/// Listening clock for one track.
pub struct Clock {
    meta: TrackMeta,
    started_at: i64,
    counter: u64,
    scrobbled: bool,
    source: Option<glib::SourceId>,
}

impl Clock {
    fn new(meta: TrackMeta, started_at: i64) -> Self {
        Clock { meta, started_at, counter: 0, scrobbled: false, source: None }
    }

    /// One second of real playback. The timer driving this only runs while the
    /// player is Playing, so pauses add nothing, and seeking never touches the
    /// counter. Returns true exactly once, when the track should be scrobbled.
    fn tick(&mut self) -> bool {
        if self.scrobbled {
            return false;
        }
        self.counter += 1;
        self.scrobbled = should_scrobble(self.counter, self.meta.duration);
        self.scrobbled
    }
}

thread_local! {
    static CLOCK: RefCell<Option<Clock>> = const { RefCell::new(None) };
}

fn clock_start() {
    CLOCK.with(|c| {
        let mut c = c.borrow_mut();
        let Some(clock) = c.as_mut() else { return };
        if clock.source.is_some() || clock.scrobbled {
            return;
        }
        let id = glib::timeout_add_seconds_local(1, || {
            let done = CLOCK.with(|c| {
                let mut c = c.borrow_mut();
                let Some(clock) = c.as_mut() else { return true };
                if clock.tick() {
                    clock.source = None;
                    scrobble(clock.meta.clone(), clock.started_at);
                    return true;
                }
                false
            });
            if done { glib::ControlFlow::Break } else { glib::ControlFlow::Continue }
        });
        clock.source = Some(id);
    });
}

fn clock_stop() {
    CLOCK.with(|c| {
        if let Some(clock) = c.borrow_mut().as_mut()
            && let Some(s) = clock.source.take()
        {
            s.remove();
        }
    });
}

/// Starts a new listen; also called when gapless repeat replays the same track,
/// which does not fire `track_changed`.
pub fn start_listen(track: &Track) {
    let meta = TrackMeta::from_track(track);
    now_playing(meta.clone());
    clock_stop();
    CLOCK.with(|c| {
        *c.borrow_mut() = Some(Clock::new(meta, now_unix()));
    });
    if player::playback_state_changed().get().status == PlaybackStatus::Playing {
        clock_start();
    }
}

pub fn init() {
    player::track_changed().on(|ev| {
        if let Some(ev) = ev {
            start_listen(&ev.track);
        }
        false
    });
    player::playback_state_changed().on(|ps| {
        if ps.status == PlaybackStatus::Playing {
            clock_start();
        } else {
            clock_stop();
        }
        false
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn meta() -> TrackMeta {
        TrackMeta { title: "T".into(), artist: "A, B".into(), album: "Al".into(), url: "https://tidal.com/track/1".into(), duration: Duration::from_secs(200) }
    }


    #[test]
    fn clock_scrobbles_once_after_real_listening() {
        let mut c = Clock::new(meta(), 0); // 200 s track -> threshold 100 s
        for _ in 0..99 {
            assert!(!c.tick());
        }
        // A pause or a seek forward sends no ticks, so nothing is credited.
        assert_eq!(c.counter, 99);
        assert!(c.tick());
        assert!(!c.tick(), "a track is scrobbled only once");
    }



}
