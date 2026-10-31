//! Port of `pkg/mpris` and `internal/player/mpris.go` using zbus. The D-Bus
//! connection lives on its own thread; the GTK main thread sends property
//! updates to it and receives method calls back through a channel.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use gtk::prelude::*;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

use super::*;

const BUS_NAME: &str = "org.mpris.MediaPlayer2.dev.dergs.Tonearm";
const PATH: &str = "/org/mpris/MediaPlayer2";
const DESKTOP_ENTRY: &str = "dev.dergs.Tonearm";
const IDENTITY: &str = "Tonearm";

#[derive(Debug)]
enum Cmd {
    Raise,
    Quit,
    Next,
    Previous,
    Pause,
    PlayPause,
    Stop,
    Play,
    Seek(i64),
    SetPosition(i64),
    LoopStatus(String),
    Shuffle(bool),
    Volume(f64),
}

#[derive(Debug)]
enum Update {
    Status(&'static str),
    Loop(&'static str),
    Shuffle(bool),
    Position(i64, bool),
    Volume(f64),
    Metadata(Option<Meta>),
}

#[derive(Debug, Clone)]
struct Meta {
    id: String,
    art_url: String,
    length_us: i64,
    album: String,
    artists: Vec<String>,
    title: String,
    url: String,
}

struct Root {
    tx: async_channel::Sender<Cmd>,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {
        let _ = self.tx.send_blocking(Cmd::Raise);
    }
    fn quit(&self) {
        let _ = self.tx.send_blocking(Cmd::Quit);
    }
    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_set_fullscreen(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn fullscreen(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn identity(&self) -> String {
        IDENTITY.into()
    }
    #[zbus(property)]
    fn desktop_entry(&self) -> String {
        DESKTOP_ENTRY.into()
    }
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec![]
    }
    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![]
    }
}

struct PlayerIface {
    tx: async_channel::Sender<Cmd>,
    status: String,
    loop_status: String,
    shuffle: bool,
    position: i64,
    volume: f64,
    metadata: HashMap<String, OwnedValue>,
}

impl PlayerIface {
    fn send(&self, c: Cmd) {
        let _ = self.tx.send_blocking(c);
    }
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl PlayerIface {
    fn next(&self) {
        self.send(Cmd::Next)
    }
    fn previous(&self) {
        self.send(Cmd::Previous)
    }
    fn pause(&self) {
        self.send(Cmd::Pause)
    }
    fn play_pause(&self) {
        self.send(Cmd::PlayPause)
    }
    fn stop(&self) {
        self.send(Cmd::Stop)
    }
    fn play(&self) {
        self.send(Cmd::Play)
    }
    fn seek(&self, offset: i64) {
        self.send(Cmd::Seek(offset))
    }
    fn set_position(&self, _track_id: ObjectPath<'_>, position: i64) {
        self.send(Cmd::SetPosition(position))
    }
    fn open_uri(&self, _uri: &str) {}

    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.status.clone()
    }
    #[zbus(property)]
    fn loop_status(&self) -> String {
        self.loop_status.clone()
    }
    #[zbus(property)]
    fn set_loop_status(&mut self, v: String) {
        self.loop_status = v.clone();
        self.send(Cmd::LoopStatus(v));
    }
    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn set_rate(&mut self, _v: f64) {}
    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.shuffle
    }
    #[zbus(property)]
    fn set_shuffle(&mut self, v: bool) {
        self.shuffle = v;
        self.send(Cmd::Shuffle(v));
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.metadata.clone()
    }
    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.volume
    }
    #[zbus(property)]
    fn set_volume(&mut self, v: f64) {
        self.volume = v;
        self.send(Cmd::Volume(v));
    }
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        self.position
    }
    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }
    #[zbus(property(emits_changed_signal = "const"))]
    fn can_control(&self) -> bool {
        true
    }
}

fn ov(v: Value<'_>) -> OwnedValue {
    OwnedValue::try_from(v).expect("no fds in metadata")
}

fn build_metadata(m: &Meta) -> HashMap<String, OwnedValue> {
    let mut h = HashMap::new();
    let path = format!("/org/mpris/MediaPlayer2/TrackList/Track{}", m.id.replace(|c: char| !c.is_ascii_alphanumeric(), "_"));
    if let Ok(p) = ObjectPath::try_from(path) {
        h.insert("mpris:trackid".into(), ov(Value::from(p)));
    }
    h.insert("mpris:artUrl".into(), ov(Value::from(m.art_url.clone())));
    h.insert("mpris:length".into(), ov(Value::from(m.length_us)));
    h.insert("xesam:album".into(), ov(Value::from(m.album.clone())));
    h.insert("xesam:artist".into(), ov(Value::from(m.artists.clone())));
    h.insert("xesam:title".into(), ov(Value::from(m.title.clone())));
    h.insert("xesam:url".into(), ov(Value::from(m.url.clone())));
    h
}

fn serve(tx: async_channel::Sender<Cmd>, rx: mpsc::Receiver<Update>) -> zbus::Result<()> {
    let player = PlayerIface {
        tx: tx.clone(),
        status: "Stopped".into(),
        loop_status: "None".into(),
        shuffle: false,
        position: 0,
        volume: 1.0,
        metadata: HashMap::new(),
    };
    let conn = zbus::blocking::connection::Builder::session()?
        .serve_at(PATH, Root { tx })?
        .serve_at(PATH, player)?
        .build()?;
    let iface = conn.object_server().interface::<_, PlayerIface>(PATH)?;
    let mut owned = false;
    for upd in rx {
        let em = iface.signal_emitter();
        let mut p = iface.get_mut();
        let r = zbus::block_on(async {
            match upd {
                Update::Status(s) if p.status != s => {
                    p.status = s.into();
                    p.playback_status_changed(em).await
                }
                Update::Loop(s) if p.loop_status != s => {
                    p.loop_status = s.into();
                    p.loop_status_changed(em).await
                }
                Update::Shuffle(s) if p.shuffle != s => {
                    p.shuffle = s;
                    p.shuffle_changed(em).await
                }
                Update::Volume(v) if p.volume != v => {
                    p.volume = v;
                    p.volume_changed(em).await
                }
                Update::Position(us, seek) => {
                    p.position = us;
                    if seek { PlayerIface::seeked(em, us).await } else { Ok(()) }
                }
                Update::Metadata(m) => {
                    p.metadata = m.as_ref().map(build_metadata).unwrap_or_default();
                    p.metadata_changed(em).await
                }
                _ => Ok(()),
            }
        });
        drop(p);
        if let Err(e) = r {
            log::error!("mpris update failed: {e}");
        }
        // Connect()/Disconnect(): own the well-known name only while a track is loaded.
        let has_track = !iface.get().metadata.is_empty();
        if has_track && !owned {
            match conn.request_name(BUS_NAME) {
                Ok(_) => owned = true,
                Err(e) => log::error!("name already taken {BUS_NAME}: {e}"),
            }
        } else if !has_track && owned {
            let _ = conn.release_name(BUS_NAME);
            owned = false;
        }
    }
    Ok(())
}

fn us(d: Duration) -> i64 {
    d.as_micros() as i64
}

pub fn init() {
    let (cmd_tx, cmd_rx) = async_channel::unbounded::<Cmd>();
    let (upd_tx, upd_rx) = mpsc::channel::<Update>();
    std::thread::Builder::new()
        .name("mpris".into())
        .spawn(move || {
            if let Err(e) = serve(cmd_tx, upd_rx) {
                log::error!("failed to export MPRIS interface: {e}");
            }
        })
        .ok();

    glib::spawn_future_local(async move {
        while let Ok(cmd) = cmd_rx.recv().await {
            handle(cmd);
        }
    });

    let send = move |u: Update| {
        let _ = upd_tx.send(u);
    };
    let s = send.clone();
    playback_state_changed().on(move |st| {
        s(Update::Position(us(st.position), false));
        s(Update::Status(if st.loading {
            "Paused"
        } else {
            match st.status {
                PlaybackStatus::Paused => "Paused",
                PlaybackStatus::Playing => "Playing",
                PlaybackStatus::Stopped => "Stopped",
            }
        }));
        CONTINUE
    });
    let s = send.clone();
    repeat_mode_changed().on(move |rm| {
        s(Update::Loop(match rm {
            RepeatMode::None => "None",
            RepeatMode::Track => "Track",
            RepeatMode::Queue => "Playlist",
        }));
        CONTINUE
    });
    let s = send.clone();
    shuffle_state_changed().on(move |v| {
        s(Update::Shuffle(*v));
        CONTINUE
    });
    let s = send.clone();
    track_changed().on(move |ev| {
        s(Update::Metadata(ev.as_ref().map(|ev| {
            let t = &ev.track;
            Meta {
                id: t.id().to_string(),
                art_url: t.cover(i32::MAX).static_,
                length_us: us(t.duration()),
                album: t.album.as_ref().map(|a| a.title.clone()).unwrap_or_default(),
                artists: t.artist_names(),
                title: t.title().to_string(),
                url: format!("https://tidal.com/track/{}", t.id()),
            }
        })));
        CONTINUE
    });
    let s = send.clone();
    volume_changed().on(move |v| {
        s(Update::Volume(*v));
        CONTINUE
    });
    seeked().on(move |p| {
        send(Update::Position(us(*p), true));
        CONTINUE
    });
}

fn handle(cmd: Cmd) {
    log::debug!("mpris command {cmd:?}");
    match cmd {
        Cmd::Raise => {
            if let Some(w) = gtk::gio::Application::default()
                .and_then(|a| a.downcast::<gtk::Application>().ok())
                .and_then(|a| a.active_window().or_else(|| a.windows().into_iter().next()))
            {
                w.set_visible(true);
                w.present();
            }
        }
        Cmd::Quit => {
            if let Some(a) = gtk::gio::Application::default() {
                a.quit();
            }
        }
        Cmd::Next => next(),
        Cmd::Previous => previous(),
        Cmd::Pause => pause(),
        Cmd::PlayPause => play_pause(),
        Cmd::Stop => super::stop(),
        Cmd::Play => play(),
        Cmd::Seek(off) => {
            let pos = playback_state_changed().get().position.as_micros() as i64 + off;
            seek_to_position(Duration::from_micros(pos.max(0) as u64), false);
        }
        Cmd::SetPosition(p) => seek_to_position(Duration::from_micros(p.max(0) as u64), false),
        Cmd::LoopStatus(s) => match s.as_str() {
            "None" => set_repeat_mode(RepeatMode::None),
            "Track" => set_repeat_mode(RepeatMode::Track),
            "Playlist" => set_repeat_mode(RepeatMode::Queue),
            _ => log::error!("Unexpected LoopStatus value {s}"),
        },
        Cmd::Shuffle(v) => set_shuffle(v, &current_track_id().unwrap_or_default()),
        Cmd::Volume(v) => set_volume(v),
    }
}
