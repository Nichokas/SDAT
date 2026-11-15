//! Continue playback on another device, through TIDAL's own per-account
//! playback state (no SDAT server, no pairing, any network).
//!
//! Every install is a player of the account. Only the active player writes
//! the queue, the current item, the position and the status; the others
//! read them. Starting to play makes a device the active player, which
//! tells the previous one to pause.

mod follow;
mod mirror;
mod wire;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::error::{CoreError, CoreResult as Result};
use crate::model::Track;
use follow::Follower;
use mirror::{Mirror, Out, Write, MAX_FUTURE};

/// Reports closer together than this are written as one.
const THROTTLE: Duration = Duration::from_millis(500);
/// Events come in bursts; one read after the last of them.
const SETTLE: Duration = Duration::from_millis(200);
/// TIDAL drops an attachment about 90 s after the last refresh.
const ATTACH_EVERY: Duration = Duration::from_secs(30);
/// State reads while an event stream is down.
const POLL_EVERY: Duration = Duration::from_secs(15);
const CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Another player of the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    #[default]
    Off,
    One,
    All,
}

/// What another device plays, or played last.
#[derive(Debug, Clone)]
pub struct Elsewhere {
    pub device: Device,
    pub track: Track,
    /// Position at `updated_at_ms`.
    pub position_ms: i64,
    pub playing: bool,
    /// Unix ms of the last position write.
    pub updated_at_ms: i64,
}

/// The local player, as the app sees it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalPlayback {
    /// Track id; `None` when nothing is loaded.
    pub current: Option<String>,
    /// Track ids after the current one, in play order (play-next first).
    pub future: Vec<String>,
    pub position_ms: i64,
    pub playing: bool,
    pub repeat: Repeat,
}

/// Playback taken over from another device.
#[derive(Debug, Clone)]
pub struct Resume {
    pub from: Device,
    pub current: Track,
    pub future: Vec<Track>,
    pub position_ms: i64,
    pub repeat: Repeat,
}

#[derive(Debug, Clone)]
pub enum HandoffEvent {
    /// The other players of the account.
    Devices(Vec<Device>),
    /// What another device plays or last played. `None` hides the banner.
    Elsewhere(Option<Elsewhere>),
    /// Another device sent its playback here: load and play it.
    Arrived(Resume),
    /// Another device became the active player: pause here.
    MovedTo(Device),
}

/// Called on a background thread.
pub trait HandoffListener: Send + Sync {
    fn on_event(&self, ev: HandoffEvent);
}

/// A random installation key (UUID v4) for the app to keep in its settings.
pub fn new_installation_key() -> String {
    let b = rand::random::<u128>() & !(0xf000 << 64) & !(0xc << 60) | (0x4000 << 64) | (0x8 << 60);
    let h = format!("{b:032x}");
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

enum Cmd {
    Report(LocalPlayback),
    Changed,
    SendTo(String, Sender<Result<()>>),
    ContinueHere(Sender<Result<Resume>>),
    /// With the access token at the time of the stop, for the detach.
    Stop(Option<String>),
}

/// Workers told to stop that have not detached yet.
static STOPPING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

fn stopping_done() {
    *STOPPING.0.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
    STOPPING.1.notify_all();
}

/// Waits up to `timeout` until every stopped [`Handoff`] has detached
/// (before quitting); `false` on timeout.
pub fn wait_detached(timeout: Duration) -> bool {
    let n = STOPPING.0.lock().unwrap_or_else(PoisonError::into_inner);
    let (n, _) = STOPPING.1.wait_timeout_while(n, timeout, |n| *n > 0).unwrap_or_else(PoisonError::into_inner);
    *n == 0
}

/// This device as a player of the account. Dropping it detaches.
pub struct Handoff {
    tx: Sender<Cmd>,
    devices: Arc<Mutex<Vec<Device>>>,
    stopped: AtomicBool,
}

impl std::fmt::Debug for Handoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handoff").finish_non_exhaustive()
    }
}

impl Handoff {
    /// Registers this install (idempotent), attaches it and follows the
    /// account's state. Blocking. `installation_key` comes from
    /// [`new_installation_key`], kept by the app.
    pub fn start(installation_key: &str, device_name: &str, listener: Arc<dyn HandoffListener>) -> Result<Self> {
        // A previous instance's late detach would undo this attach.
        wait_detached(Duration::from_secs(5));
        let me = wire::register(installation_key, device_name)?;
        wire::attach(&me)?;
        let (tx, rx) = mpsc::channel();
        let devices = Arc::new(Mutex::new(Vec::new()));
        let mut w = Worker {
            mirror: Mirror::new(me),
            listener,
            devices: Arc::clone(&devices),
            tx: tx.clone(),
            streams: Vec::new(),
            last_local: None,
            names: HashMap::new(),
        };
        std::thread::Builder::new().name("handoff".into()).spawn(move || w.run(&rx)).map_err(|e| CoreError::Other(e.to_string()))?;
        Ok(Self { tx, devices, stopped: AtomicBool::new(false) })
    }

    /// Every local change (track, queue, play, pause, seek). Cheap: only
    /// differences are written, and only while this device is active.
    pub fn report(&self, local: LocalPlayback) {
        let _ = self.tx.send(Cmd::Report(local));
    }

    pub fn devices(&self) -> Vec<Device> {
        self.devices.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Push the playback last reported to `device_id`. Pause here after it returns.
    pub fn send_to(&self, device_id: &str) -> Result<()> {
        self.call(|tx| Cmd::SendTo(device_id.to_string(), tx))
    }

    /// Pull: makes this device active and returns what to play.
    pub fn continue_here(&self) -> Result<Resume> {
        self.call(Cmd::ContinueHere)
    }

    /// Detach (backgrounded, quit, sign-out). Drop does the same. Returns
    /// at once; call it before removing the credentials.
    pub fn stop(&self) {
        if self.stopped.swap(true, Ordering::Relaxed) {
            return;
        }
        *STOPPING.0.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        if self.tx.send(Cmd::Stop(crate::api::cached_user_token())).is_err() {
            stopping_done();
        }
    }

    fn call<T>(&self, cmd: impl FnOnce(Sender<Result<T>>) -> Cmd) -> Result<T> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(cmd(tx)).map_err(|_stopped| CoreError::Other("handoff stopped".into()))?;
        rx.recv_timeout(CALL_TIMEOUT).map_err(|e| CoreError::Network(format!("handoff: {e}")))?
    }
}

impl Drop for Handoff {
    fn drop(&mut self) {
        self.stop();
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

struct Worker {
    mirror: Mirror,
    listener: Arc<dyn HandoffListener>,
    devices: Arc<Mutex<Vec<Device>>>,
    tx: Sender<Cmd>,
    streams: Vec<Follower>,
    last_local: Option<LocalPlayback>,
    /// Installation names, for devices that are no longer attached.
    names: HashMap<String, String>,
}

impl Worker {
    fn run(&mut self, rx: &Receiver<Cmd>) {
        let mut pending: Option<Instant> = None;
        let mut dirty: Option<Instant> = Some(Instant::now());
        let mut next_attach = Instant::now() + ATTACH_EVERY;
        let mut next_poll = Instant::now() + POLL_EVERY;
        loop {
            let wake = [pending, dirty, Some(next_attach), Some(next_poll)].into_iter().flatten().min().unwrap_or(next_poll);
            match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
                Ok(Cmd::Report(l)) => {
                    self.last_local = Some(l);
                    pending.get_or_insert_with(|| Instant::now() + THROTTLE);
                }
                Ok(Cmd::Changed) => {
                    dirty.get_or_insert_with(|| Instant::now() + SETTLE);
                }
                Ok(Cmd::SendTo(id, reply)) => {
                    pending = None;
                    let _ = reply.send(self.send_to(&id));
                }
                Ok(Cmd::ContinueHere(reply)) => {
                    pending = None;
                    let _ = reply.send(self.continue_here());
                }
                Ok(Cmd::Stop(token)) => {
                    if pending.is_some() {
                        self.write_local(false);
                    }
                    self.streams.clear();
                    if let Err(e) = wire::detach(self.mirror.me(), token.as_deref()) {
                        log::info!("handoff: detach failed error={e}");
                    }
                    stopping_done();
                    return;
                }
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {}
            }
            let now = Instant::now();
            if pending.is_some_and(|p| p <= now) {
                pending = None;
                self.write_local(false);
            }
            if dirty.is_some_and(|d| d <= now) {
                dirty = None;
                self.refresh();
            }
            if next_attach <= now {
                next_attach = now + ATTACH_EVERY;
                if let Err(e) = wire::attach(self.mirror.me()) {
                    log::info!("handoff: attach failed error={e}");
                }
            }
            if next_poll <= now {
                next_poll = now + POLL_EVERY;
                if self.streams.is_empty() || self.streams.iter().any(Follower::is_down) {
                    self.refresh();
                }
            }
        }
    }

    fn emit(&self, ev: HandoffEvent) {
        if let HandoffEvent::Devices(d) = &ev {
            d.clone_into(&mut self.devices.lock().unwrap_or_else(PoisonError::into_inner));
        }
        self.listener.on_event(ev);
    }

    fn follow(&mut self, urls: &[Option<String>]) {
        let urls: Vec<String> = urls.iter().flatten().cloned().collect();
        self.streams.retain(|s| urls.contains(&s.url));
        for u in urls {
            if !self.streams.iter().any(|s| s.url == u) {
                self.streams.push(Follower::spawn(u, self.tx.clone()));
            }
        }
    }

    /// Reads TIDAL's state and tells the app what changed.
    fn refresh(&mut self) {
        let read = || -> Result<_> {
            let state = wire::state()?;
            let head = state.queue_id.as_deref().map(wire::head).transpose()?;
            Ok((state, head))
        };
        let (state, head) = match read() {
            Ok(r) => r,
            Err(e) => {
                log::info!("handoff: state read failed error={e}");
                return;
            }
        };
        self.follow(&[state.stream.clone(), head.as_ref().and_then(|h| h.stream.clone())]);
        for p in &state.players {
            self.names.insert(p.id.clone(), p.name.clone());
        }
        let by = head.as_ref().and_then(|h| h.updated_by.clone()).map(|id| {
            let name = self.names.entry(id.clone()).or_insert_with(|| wire::installation_name(&id).unwrap_or_default()).clone();
            Device { id, name }
        });
        for o in self.mirror.fetched(state, head, by.clone()) {
            match o {
                Out::Event(ev) => self.emit(*ev),
                Out::Arrive => match self.resume(false, by.clone()) {
                    Ok(r) => self.emit(HandoffEvent::Arrived(r)),
                    Err(e) => log::warn!("handoff: arrived queue unreadable error={e}"),
                },
            }
        }
    }

    /// Reads the queue for playing it here, from the device that wrote it.
    /// `claim` makes this device active once the queue proved readable.
    fn resume(&mut self, claim: bool, from: Option<Device>) -> Result<Resume> {
        let queue = self.mirror.queue_id().ok_or_else(|| CoreError::Other("nothing to continue".into()))?.to_string();
        let head = wire::head(&queue)?;
        let (Some(current), Some(track)) = (head.current.clone(), head.track.clone()) else {
            return Err(CoreError::Other("nothing to continue".into()));
        };
        let (mut entries, future) = wire::future_tracks(&queue, MAX_FUTURE)?;
        if claim {
            let active = wire::set_active(self.mirror.me())?;
            self.mirror.claimed(active);
        }
        let duration = i64::try_from(track.duration().as_millis()).unwrap_or(i64::MAX);
        // A pull takes over a player that may still be playing; a push wrote its exact position.
        let playing = claim && self.mirror.state().status == wire::Status::Playing;
        let now = now_ms();
        let position_ms = mirror::position_now(head.position_ms.unwrap_or(0), head.position_at_ms.unwrap_or(now), now, playing, duration);
        entries.insert(0, current);
        self.mirror.adopt(entries, &head, position_ms, now);
        let from = from
            .or_else(|| head.updated_by.as_deref().and_then(|id| self.mirror.device(id)))
            .unwrap_or_else(|| Device { id: head.updated_by.clone().unwrap_or_default(), name: String::new() });
        Ok(Resume { from, current: track, future, position_ms, repeat: head.repeat })
    }

    fn continue_here(&mut self) -> Result<Resume> {
        self.refresh();
        let from = self.mirror.state().active.as_deref().filter(|a| *a != self.mirror.me()).and_then(|a| self.mirror.device(a));
        self.resume(true, from)
    }

    fn send_to(&mut self, id: &str) -> Result<()> {
        if self.last_local.as_ref().is_none_or(|l| l.current.is_none()) {
            return Err(CoreError::Other("nothing playing".into()));
        }
        if !self.mirror.is_active() {
            let active = wire::set_active(self.mirror.me())?;
            self.mirror.claimed(active);
        }
        if !self.write_local(true) {
            return Err(CoreError::Other("could not write the queue".into()));
        }
        if wire::set_active(id)?.as_deref() != Some(id) {
            // An unattached target leaves no active player: take it back.
            let active = wire::set_active(self.mirror.me())?;
            self.mirror.claimed(active);
            return Err(CoreError::Other("the device is not available".into()));
        }
        self.mirror.released_to(id);
        wire::set_status(false)?;
        self.mirror.status_written(false);
        Ok(())
    }

    /// Writes the last report; `exact` also writes its position. `false` on failure.
    fn write_local(&mut self, exact: bool) -> bool {
        let Some(local) = self.last_local.clone() else { return true };
        if exact {
            self.mirror.force_position();
        }
        let writes = self.mirror.plan(&local, now_ms());
        match self.apply(writes, &local) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("handoff: write failed error={e}");
                self.mirror.write_failed();
                false
            }
        }
    }

    fn apply(&mut self, writes: Vec<Write>, local: &LocalPlayback) -> Result<()> {
        let me = self.mirror.me().to_string();
        for w in writes {
            match w {
                Write::Claim => {
                    let active = wire::set_active(&me)?;
                    self.mirror.claimed(active);
                }
                Write::NewQueue => self.mirror.queue_created(wire::create_queue()?),
                Write::Queue(tracks) => {
                    let q = self.queue()?;
                    wire::rewrite(&q, &tracks, local.repeat)?;
                    let head = wire::head(&q)?;
                    let mut entries = wire::future_entries(&q, MAX_FUTURE)?;
                    entries.splice(0..0, head.current);
                    self.mirror.queue_written(tracks, entries, head.revision, local.repeat);
                }
                Write::Repeat(r) => {
                    let rev = wire::set_repeat(&self.queue()?, r)?;
                    self.mirror.repeat_written(r, rev);
                }
                Write::Current { index, position_ms } => {
                    let e = self.mirror.entry(index).ok_or_else(|| CoreError::Other("queue entry missing".into()))?;
                    let rev = wire::set_current(&self.queue()?, &e, position_ms, &me)?;
                    self.mirror.current_written(index, position_ms, local.playing, now_ms(), rev);
                }
                Write::Status { playing } => {
                    wire::set_status(playing)?;
                    self.mirror.status_written(playing);
                }
            }
        }
        Ok(())
    }

    fn queue(&self) -> Result<String> {
        self.mirror.queue_id().map(str::to_string).ok_or_else(|| CoreError::Other("no queue".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_keys_are_uuid_v4() {
        let k = new_installation_key();
        assert_eq!(k.split('-').map(str::len).collect::<Vec<_>>(), [8, 4, 4, 4, 12]);
        assert_eq!(&k[14..15], "4");
        assert!(matches!(&k[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(k, new_installation_key());
    }

    struct Log(Mutex<Vec<String>>);
    impl HandoffListener for Log {
        fn on_event(&self, ev: HandoffEvent) {
            let line = match ev {
                HandoffEvent::Devices(d) => format!("devices {:?}", d.iter().map(|d| &d.name).collect::<Vec<_>>()),
                HandoffEvent::Elsewhere(e) => format!("elsewhere {:?}", e.map(|e| (e.device.name, e.track.title().to_string(), e.playing))),
                HandoffEvent::Arrived(r) => format!("arrived {} +{} at {}", r.current.title(), r.future.len(), r.position_ms),
                HandoffEvent::MovedTo(d) => format!("moved to {}", d.name),
            };
            eprintln!("{line}");
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push(line);
        }
    }

    /// Two installs of the signed-in account hand playback back and forth.
    /// Needs `SDAT_LIVE_REFRESH_TOKEN`; changes the account's playback state.
    #[test]
    #[ignore = "live: writes the TIDAL account's playback state"]
    fn live_handoff_between_two_installs() {
        struct Env;
        impl crate::auth::TokenStore for Env {
            fn has_refresh_token(&self) -> bool {
                std::env::var("SDAT_LIVE_REFRESH_TOKEN").is_ok()
            }
            fn get_refresh_token(&self) -> String {
                std::env::var("SDAT_LIVE_REFRESH_TOKEN").unwrap_or_default()
            }
            fn user_id(&self) -> String {
                String::new()
            }
        }
        crate::auth::install(Box::new(Env));
        let wait = |log: &Log, what: &str| {
            let until = Instant::now() + Duration::from_secs(20);
            while Instant::now() < until {
                if log.0.lock().unwrap_or_else(PoisonError::into_inner).iter().any(|l| l.starts_with(what) || what.contains('*') && what.split('*').all(|p| l.contains(p))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            panic!("no '{what}' in {:?}", log.0.lock().unwrap_or_else(PoisonError::into_inner));
        };
        let (la, lb) = (Arc::new(Log(Mutex::new(vec![]))), Arc::new(Log(Mutex::new(vec![]))));
        let a = Handoff::start(&std::env::var("SDAT_LIVE_KEY_A").unwrap_or_else(|_| new_installation_key()), "SDAT test A", Arc::clone(&la) as Arc<dyn HandoffListener>).expect("start a");
        let b = Handoff::start(&std::env::var("SDAT_LIVE_KEY_B").unwrap_or_else(|_| new_installation_key()), "SDAT test B", Arc::clone(&lb) as Arc<dyn HandoffListener>).expect("start b");
        let ids: Vec<String> = ["35707532", "60923645", "40475829", "504370449", "162358163"].map(String::from).to_vec();
        a.report(LocalPlayback { current: Some(ids[0].clone()), future: ids[1..].to_vec(), position_ms: 30_000, playing: true, repeat: Repeat::Off });
        // Playing, so not what a previous run left behind.
        wait(&lb, "elsewhere Some((\"SDAT test A\"*true))");
        let r = b.continue_here().expect("continue here");
        assert_eq!(r.current.id(), ids[0]);
        assert_eq!(r.future.iter().map(|t| t.id().to_string()).collect::<Vec<_>>(), ids[1..]);
        assert!(r.position_ms >= 30_000, "position {}", r.position_ms);
        wait(&la, "moved to SDAT test B");
        a.report(LocalPlayback { current: Some(ids[0].clone()), future: ids[1..].to_vec(), position_ms: 31_000, playing: false, repeat: Repeat::Off });
        b.report(LocalPlayback { current: Some(ids[1].clone()), future: ids[2..].to_vec(), position_ms: 12_345, playing: true, repeat: Repeat::Off });
        std::thread::sleep(Duration::from_secs(2));
        let a_id = b.devices().into_iter().find(|d| d.name == "SDAT test A").expect("a listed").id;
        b.send_to(&a_id).expect("send to a");
        wait(&la, "arrived");
        let got = la.0.lock().unwrap_or_else(PoisonError::into_inner).iter().find(|l| l.starts_with("arrived")).cloned().unwrap_or_default();
        assert!(got.ends_with("+3 at 12345"), "{got}");
        a.stop();
        b.stop();
        assert!(wait_detached(Duration::from_secs(10)), "not detached");
    }
}
