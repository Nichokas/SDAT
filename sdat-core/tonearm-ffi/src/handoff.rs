//! Continue playback on another device, over `tonearm_core::handoff`.

use std::sync::Arc;

use tonearm_core::handoff as core;

use crate::{FfiError, Track};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HandoffDevice {
    pub id: String,
    pub name: String,
}

impl From<core::Device> for HandoffDevice {
    fn from(d: core::Device) -> Self {
        Self { id: d.id, name: d.name }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HandoffRepeat {
    Off,
    One,
    All,
}

impl From<core::Repeat> for HandoffRepeat {
    fn from(r: core::Repeat) -> Self {
        match r {
            core::Repeat::Off => Self::Off,
            core::Repeat::One => Self::One,
            core::Repeat::All => Self::All,
        }
    }
}

impl From<HandoffRepeat> for core::Repeat {
    fn from(r: HandoffRepeat) -> Self {
        match r {
            HandoffRepeat::Off => Self::Off,
            HandoffRepeat::One => Self::One,
            HandoffRepeat::All => Self::All,
        }
    }
}

/// What another device plays, or played last.
#[derive(Debug, Clone, uniffi::Record)]
pub struct HandoffElsewhere {
    pub device: HandoffDevice,
    pub track: Track,
    /// Position at `updated_at_ms`.
    pub position_ms: i64,
    pub playing: bool,
    /// Unix ms of the last position write ("2 h ago").
    pub updated_at_ms: i64,
}

/// The local player.
#[derive(Debug, Clone, uniffi::Record)]
pub struct HandoffLocal {
    /// Track id; `None` when nothing is loaded.
    pub current: Option<String>,
    /// Track ids after the current one, in play order (play-next first).
    pub future: Vec<String>,
    /// Track ids repeat all plays again after the last of `future`, in that
    /// order: the part of the list before the current track.
    pub past: Vec<String>,
    /// The queue is Discover's endless shuffle.
    pub discover: bool,
    pub position_ms: i64,
    pub playing: bool,
    pub repeat: HandoffRepeat,
}

/// Playback taken over from another device: the current track only, so it
/// starts sooner. What follows it comes after, as `HandoffEvent::Upcoming`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct HandoffResume {
    pub from: HandoffDevice,
    pub current: Track,
    pub position_ms: i64,
    pub repeat: HandoffRepeat,
    /// The queue is Discover's endless shuffle: go on with this device's own
    /// once it runs out.
    pub discover: bool,
}

impl From<core::Resume> for HandoffResume {
    fn from(r: core::Resume) -> Self {
        Self {
            from: r.from.into(),
            current: Track::from(&r.current),
            position_ms: r.position_ms,
            repeat: r.repeat.into(),
            discover: r.discover,
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum HandoffEvent {
    /// The other players of the account ("Send to" targets).
    Devices { devices: Vec<HandoffDevice> },
    /// What another device plays or last played; `None` hides the banner.
    Elsewhere { elsewhere: Option<HandoffElsewhere> },
    /// Another device sent its playback here: load and play it.
    Arrived { resume: HandoffResume },
    /// Another device became the active player: pause here.
    MovedTo { device: HandoffDevice },
    /// What follows `current_id`, the track of the last `Arrived` or
    /// `continue_here`, read after it. Append it while that track is still
    /// the current one. Without tracks when nothing follows it or that could
    /// not be read: the app need not wait for them.
    Upcoming { current_id: String, tracks: Vec<Track> },
}

/// Implemented by the app; called on a background thread.
#[uniffi::export(with_foreign)]
pub trait HandoffObserver: Send + Sync {
    fn on_event(&self, event: HandoffEvent);
}

struct Bridge(Arc<dyn HandoffObserver>);

impl core::HandoffListener for Bridge {
    fn on_event(&self, ev: core::HandoffEvent) {
        self.0.on_event(match ev {
            core::HandoffEvent::Devices(d) => HandoffEvent::Devices { devices: d.into_iter().map(Into::into).collect() },
            core::HandoffEvent::Elsewhere(e) => HandoffEvent::Elsewhere {
                elsewhere: e.map(|e| HandoffElsewhere {
                    device: e.device.into(),
                    track: Track::from(&e.track),
                    position_ms: e.position_ms,
                    playing: e.playing,
                    updated_at_ms: e.updated_at_ms,
                }),
            },
            core::HandoffEvent::Arrived(r) => HandoffEvent::Arrived { resume: r.into() },
            core::HandoffEvent::MovedTo(d) => HandoffEvent::MovedTo { device: d.into() },
            core::HandoffEvent::Upcoming(u) => HandoffEvent::Upcoming { current_id: u.current, tracks: crate::convert::tracks(&u.tracks) },
        });
    }
}

/// This device as a player of the TIDAL account. Start it when signed in
/// and in the foreground (or playing); stop it when backgrounded without
/// playing, on sign-out and on quit.
#[derive(Debug, uniffi::Object)]
pub struct Handoff(Arc<core::Handoff>);

#[uniffi::export]
impl Handoff {
    /// Registers this install and attaches it. `installation_key` comes
    /// from `handoff_new_installation_key`, kept in the app's settings.
    #[uniffi::constructor]
    pub async fn start(installation_key: String, device_name: String, observer: Arc<dyn HandoffObserver>) -> Result<Arc<Self>, FfiError> {
        let h = crate::run(move || core::Handoff::start(&installation_key, &device_name, Arc::new(Bridge(observer)))).await?;
        Ok(Arc::new(Self(Arc::new(h))))
    }

    /// Call on every track, queue, play, pause and seek change. Cheap.
    pub fn report(&self, local: HandoffLocal) {
        self.0.report(core::LocalPlayback {
            current: local.current,
            future: local.future,
            past: local.past,
            discover: local.discover,
            position_ms: local.position_ms,
            playing: local.playing,
            repeat: local.repeat.into(),
        });
    }

    pub fn devices(&self) -> Vec<HandoffDevice> {
        self.0.devices().into_iter().map(Into::into).collect()
    }

    /// Push the playback last reported to `device_id`; pause here after it returns.
    pub async fn send_to(&self, device_id: String) -> Result<(), FfiError> {
        let h = Arc::clone(&self.0);
        crate::run(move || h.send_to(&device_id)).await
    }

    /// Pull: makes this device active and returns what to play.
    pub async fn continue_here(&self) -> Result<HandoffResume, FfiError> {
        let h = Arc::clone(&self.0);
        crate::run(move || h.continue_here()).await.map(Into::into)
    }

    pub fn stop(&self) {
        self.0.stop();
    }
}

/// A new random installation key (UUID) to keep in the app's settings.
#[uniffi::export]
pub fn handoff_new_installation_key() -> String {
    core::new_installation_key()
}
