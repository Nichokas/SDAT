//! Listen Together for Swift: host/guest handles over `tonearm_core::live`.

use std::sync::Arc;

use tonearm_core::live::drift::Correction;
use tonearm_core::live::net::{self, LiveEvent, LiveListener, NetConfig};
use tonearm_core::live::proto::{EndReason, GuestCommand, RejectReason};
use tonearm_core::live::session::GuestEvent;

use crate::FfiError;

#[derive(Debug, Clone, uniffi::Record)]
pub struct LiveGuestInfo {
    pub peer: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum LiveEnd {
    /// The host ended the session.
    HostEnded,
    /// Lost contact with the host.
    HostLost,
    Left,
    /// Host refused: wrong or old invitation.
    BadInvite,
    /// Host runs another protocol version: update SDAT.
    Incompatible,
    Full,
}

/// Playback request from a guest (host side) or to the host (guest side).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum LiveCommand {
    Play,
    Pause,
    Next,
    /// Append these TIDAL track ids to the shared queue (max 50, digits only).
    Enqueue { track_ids: Vec<String> },
}

fn command_of(c: GuestCommand) -> Option<LiveCommand> {
    Some(match c {
        GuestCommand::Play => LiveCommand::Play,
        GuestCommand::Pause => LiveCommand::Pause,
        GuestCommand::Next => LiveCommand::Next,
        GuestCommand::Enqueue { track_ids } => LiveCommand::Enqueue { track_ids },
        GuestCommand::Unknown => return None,
    })
}

fn to_core(c: LiveCommand) -> GuestCommand {
    match c {
        LiveCommand::Play => GuestCommand::Play,
        LiveCommand::Pause => GuestCommand::Pause,
        LiveCommand::Next => GuestCommand::Next,
        LiveCommand::Enqueue { track_ids } => GuestCommand::Enqueue { track_ids },
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum LiveUpdate {
    Guests { guests: Vec<LiveGuestInfo> },
    Joined { host_name: String },
    QueueChanged,
    PlaybackChanged,
    Reconnecting,
    Ended { reason: LiveEnd },
}

/// Implemented in Swift; called from a background thread.
#[uniffi::export(with_foreign)]
pub trait LiveObserver: Send + Sync {
    fn on_update(&self, update: LiveUpdate);
}

/// Guest playback control, kept apart from [`LiveObserver`] so adding it
/// does not change [`LiveUpdate`]. Implemented in Swift; called from a
/// background thread. Install it with `set_control_observer`.
#[uniffi::export(with_foreign)]
pub trait LiveControlObserver: Send + Sync {
    /// Guest: the host allowed (`true`) or forbade guest playback control.
    fn on_control_changed(&self, allowed: bool);
    /// Host: a guest asked for a playback change. Permission, validation
    /// and rate limits are already checked; apply it through the normal
    /// player path, which then updates every guest.
    fn on_guest_command(&self, peer: String, name: String, command: LiveCommand);
}

/// Host: guest Play Next, kept apart from [`LiveControlObserver`] so apps
/// that do not handle it are unaffected (they keep getting `Enqueue`).
/// Install it with `LiveHost::set_play_next_observer`.
#[uniffi::export(with_foreign)]
pub trait LivePlayNextObserver: Send + Sync {
    /// A guest asked to play these tracks next (permission, validation and
    /// rate limits already checked). Insert them at
    /// `index_of_playing + 1 + LiveHost::play_next_offset(track_ids)`.
    fn on_guest_play_next(&self, peer: String, name: String, track_ids: Vec<String>);
}

type PlayNextSlot = Arc<std::sync::Mutex<Option<Arc<dyn LivePlayNextObserver>>>>;

type ControlSlot = Arc<std::sync::Mutex<Option<Arc<dyn LiveControlObserver>>>>;

fn control(slot: &ControlSlot) -> Option<Arc<dyn LiveControlObserver>> {
    slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LiveRelayConfig {
    /// Preferred relay URLs; empty uses the built-in default.
    pub relays: Vec<String>,
    pub n0_fallback: bool,
}

fn cfg(c: Option<LiveRelayConfig>) -> NetConfig {
    match c {
        Some(c) if !c.relays.is_empty() => NetConfig { relays: c.relays, n0_fallback: c.n0_fallback },
        Some(c) => NetConfig { n0_fallback: c.n0_fallback, ..NetConfig::default() },
        None => NetConfig::default(),
    }
}

struct Bridge(Arc<dyn LiveObserver>, ControlSlot, PlayNextSlot);

fn end_of(r: EndReason) -> LiveEnd {
    match r {
        EndReason::HostEnded | EndReason::Other => LiveEnd::HostEnded,
        EndReason::HostLost => LiveEnd::HostLost,
        EndReason::Left => LiveEnd::Left,
    }
}

fn reject_of(r: RejectReason) -> LiveEnd {
    match r {
        RejectReason::IncompatibleVersion => LiveEnd::Incompatible,
        RejectReason::Full => LiveEnd::Full,
        RejectReason::BadSecret | RejectReason::Other => LiveEnd::BadInvite,
    }
}

impl LiveListener for Bridge {
    fn on_event(&self, ev: LiveEvent) {
        let u = match ev {
            LiveEvent::Command(c) => {
                let next = if c.next { self.2.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone() } else { None };
                match (next, command_of(c.cmd)) {
                    (Some(p), Some(LiveCommand::Enqueue { track_ids })) => p.on_guest_play_next(c.peer, c.name, track_ids),
                    (_, Some(command)) => {
                        if let Some(o) = control(&self.1) {
                            o.on_guest_command(c.peer, c.name, command);
                        }
                    }
                    (_, None) => {}
                }
                return;
            }
            LiveEvent::Guest(GuestEvent::ControlChanged(allowed)) => {
                if let Some(o) = control(&self.1) {
                    o.on_control_changed(allowed);
                }
                return;
            }
            LiveEvent::Guests(g) => LiveUpdate::Guests { guests: g.into_iter().map(|g| LiveGuestInfo { peer: g.peer, name: g.name }).collect() },
            LiveEvent::Reconnecting => LiveUpdate::Reconnecting,
            LiveEvent::Guest(GuestEvent::Joined { host_name }) => LiveUpdate::Joined { host_name },
            LiveEvent::Guest(GuestEvent::QueueChanged) => LiveUpdate::QueueChanged,
            LiveEvent::Guest(GuestEvent::PlaybackChanged) => LiveUpdate::PlaybackChanged,
            LiveEvent::Guest(GuestEvent::Ended(r)) => LiveUpdate::Ended { reason: end_of(r) },
            LiveEvent::Guest(GuestEvent::Rejected(r)) => LiveUpdate::Ended { reason: reject_of(r) },
        };
        self.0.on_update(u);
    }
}

fn err(e: net::NetError) -> FfiError {
    FfiError::Other { message: e.to_string() }
}

/// A session this device hosts.
#[derive(uniffi::Object)]
pub struct LiveHost(net::LiveHost, ControlSlot, PlayNextSlot);

impl std::fmt::Debug for LiveHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LiveHost").field(&self.0).finish_non_exhaustive()
    }
}

#[uniffi::export]
impl LiveHost {
    /// Opens a session (blocks up to ~5 s while the endpoint goes online).
    #[uniffi::constructor]
    pub async fn start(name: String, relays: Option<LiveRelayConfig>, observer: Arc<dyn LiveObserver>) -> Result<Arc<Self>, FfiError> {
        let c = cfg(relays);
        let slot = ControlSlot::default();
        let next = PlayNextSlot::default();
        let bridge = Arc::new(Bridge(observer, slot.clone(), Arc::clone(&next)));
        let h = blocking::unblock(move || net::LiveHost::start(name, &c, bridge)).await.map_err(err)?;
        Ok(Arc::new(Self(h, slot, next)))
    }
    /// `sdat://join/...`, for the QR code.
    pub fn invite_url(&self) -> String {
        self.0.ticket_url()
    }
    pub fn guests(&self) -> Vec<LiveGuestInfo> {
        self.0.guests().into_iter().map(|g| LiveGuestInfo { peer: g.peer, name: g.name }).collect()
    }
    /// Replaces the shared queue (max 500); returns one entry id per track.
    pub fn set_queue(&self, track_ids: Vec<String>) -> Result<Vec<String>, FfiError> {
        self.0.replace_queue(track_ids).map_err(err)
    }
    /// Call on play, pause, seek and track change.
    pub fn set_playback(&self, item_id: Option<String>, track_id: Option<String>, position_ms: i64, playing: bool) {
        self.0.set_playback(item_id, track_id, position_ms, playing);
    }
    /// Receives guest commands (dropped while none is installed).
    pub fn set_control_observer(&self, observer: Option<Arc<dyn LiveControlObserver>>) {
        *self.1.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = observer;
    }
    /// Lets guests play/pause, skip and add tracks (off by default).
    pub fn set_guest_control(&self, allowed: bool) {
        self.0.set_guest_control(allowed);
    }
    pub fn guest_control(&self) -> bool {
        self.0.guest_control()
    }
    /// Receives guest Play Next (off while none is installed: guests then
    /// append). Install before guests join so they learn it in the Welcome.
    pub fn set_play_next_observer(&self, observer: Option<Arc<dyn LivePlayNextObserver>>) {
        self.0.set_play_next(observer.is_some());
        *self.2.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = observer;
    }
    /// Offset after the playing entry at which to insert these guest Play
    /// Next tracks; keeps several requests in order. Call `set_queue` after
    /// each insertion so the next offset sees it.
    pub fn play_next_offset(&self, track_ids: Vec<String>) -> u32 {
        u32::try_from(self.0.play_next_offset(&track_ids)).unwrap_or(u32::MAX)
    }
    pub fn end(&self) {
        self.0.end();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LiveQueueEntry {
    pub item_id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LiveTarget {
    pub item_id: Option<String>,
    pub track_id: Option<String>,
    pub position_ms: i64,
    pub playing: bool,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum LiveCorrection {
    None,
    Rate { rate: f64 },
    Seek { position_ms: i64 },
}

/// A session this device joined.
#[derive(uniffi::Object)]
pub struct LiveGuest(net::LiveGuest, ControlSlot);

impl std::fmt::Debug for LiveGuest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LiveGuest").field(&self.0).finish_non_exhaustive()
    }
}

#[uniffi::export]
impl LiveGuest {
    /// Starts joining from an `sdat://join/...` link or QR content.
    #[uniffi::constructor]
    pub fn join(invite: String, name: String, relays: Option<LiveRelayConfig>, observer: Arc<dyn LiveObserver>) -> Result<Arc<Self>, FfiError> {
        let slot = ControlSlot::default();
        let g = net::LiveGuest::join(&invite, name, &cfg(relays), Arc::new(Bridge(observer, slot.clone(), PlayNextSlot::default()))).map_err(err)?;
        Ok(Arc::new(Self(g, slot)))
    }
    pub fn host_name(&self) -> String {
        self.0.host_name()
    }
    /// Shared queue in order; `item_id` matches [`LiveTarget::item_id`], so
    /// repeated tracks can be told apart.
    pub fn queue(&self) -> Vec<LiveQueueEntry> {
        self.0.queue().into_iter().map(|i| LiveQueueEntry { item_id: i.item_id, track_id: i.track_id }).collect()
    }
    /// Shared queue as track ids, in order.
    pub fn queue_track_ids(&self) -> Vec<String> {
        self.0.queue().into_iter().map(|i| i.track_id).collect()
    }
    /// Where the player should be now; `None` until synced.
    pub fn target(&self, latency_ms: i64) -> Option<LiveTarget> {
        self.0.target(latency_ms).map(|t| LiveTarget { item_id: t.item_id, track_id: t.track_id, position_ms: t.position_ms, playing: t.playing })
    }
    /// Drift correction for a player at `actual_ms` on the target track.
    pub fn correction(&self, latency_ms: i64, actual_ms: i64) -> LiveCorrection {
        match self.0.correction(latency_ms, actual_ms) {
            Correction::None => LiveCorrection::None,
            Correction::Rate(rate) => LiveCorrection::Rate { rate },
            Correction::Seek(position_ms) => LiveCorrection::Seek { position_ms },
        }
    }
    /// Low power mode: no rate nudging, seeks only.
    pub fn set_seek_only(&self, on: bool) {
        self.0.set_seek_only(on);
    }
    /// Hears when the host allows or forbids guest control. Read
    /// [`Self::guest_control`] after installing it for the current value.
    pub fn set_control_observer(&self, observer: Option<Arc<dyn LiveControlObserver>>) {
        *self.1.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = observer;
    }
    /// Whether the host allows our playback commands (lock controls otherwise).
    pub fn guest_control(&self) -> bool {
        self.0.guest_control()
    }
    /// Asks the host for a playback change; `false` if it was not sent.
    pub fn send_command(&self, command: LiveCommand) -> bool {
        self.0.send_command(to_core(command))
    }
    /// Whether the host places our Play Next after the playing track
    /// (otherwise `send_play_next` appends, like `Enqueue`).
    pub fn host_supports_play_next(&self) -> bool {
        self.0.host_supports_play_next()
    }
    /// Asks the host to play these tracks next; `false` if it was not sent.
    pub fn send_play_next(&self, track_ids: Vec<String>) -> bool {
        self.0.send_play_next(track_ids)
    }
    pub fn leave(&self) {
        self.0.leave();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_map_both_ways() {
        for c in [LiveCommand::Play, LiveCommand::Pause, LiveCommand::Next, LiveCommand::Enqueue { track_ids: vec!["1".into()] }] {
            assert_eq!(command_of(to_core(c.clone())), Some(c));
        }
        assert_eq!(command_of(GuestCommand::Unknown), None);
    }
}

/// Validates an invitation without joining (QR scanner feedback).
#[uniffi::export]
pub fn live_is_invite(text: String) -> bool {
    tonearm_core::live::ticket::Ticket::parse(&text).is_ok()
}

/// One track by id (guests resolve the shared queue's ids with their own account).
#[uniffi::export]
pub async fn live_track(id: String) -> Result<crate::Track, FfiError> {
    let t = crate::run(move || tonearm_core::service::get_track(&id)).await?;
    Ok(crate::convert::tracks(std::slice::from_ref(&t)).remove(0))
}
