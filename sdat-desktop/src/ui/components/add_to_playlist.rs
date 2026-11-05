//! "Add to Playlist" submenu for the track context menu.
//!
//! The user's own playlists are fetched lazily the first time a track menu
//! opens and then cached for the session.

use std::cell::{Cell, RefCell};

use gtk::gio;
use gtk::prelude::*;

use crate::api::{self, client::OwnPlaylist};
use crate::i18n::tr;
use crate::player;

/// Action that receives `(playlist_id, track_id)`.
pub const ACTION: &str = "win.playlist.add-track";
/// Unregistered action, so items pointing at it render disabled.
const DISABLED: &str = "win.playlist.unavailable";

#[derive(Default)]
enum Cache {
    #[default]
    Empty,
    Loading,
    Ready(Vec<OwnPlaylist>),
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::default();
    static WAITING: RefCell<Vec<(glib::WeakRef<gio::Menu>, String)>> = const { RefCell::new(Vec::new()) };
    /// Bumped on sign in/out; a load started before it fetched the previous account's list.
    static ACCOUNT_EPOCH: Cell<u64> = const { Cell::new(0) };
}

/// Forgets the previous account's playlists (sign in/out).
pub fn reset() {
    ACCOUNT_EPOCH.set(ACCOUNT_EPOCH.get() + 1);
    CACHE.with_borrow_mut(|c| *c = Cache::Empty);
    WAITING.with_borrow_mut(Vec::clear);
}

/// What the submenu should show.
#[derive(Debug, PartialEq, Eq)]
enum Entry {
    Disabled(String),
    Playlist { label: String, playlist_id: String },
}

fn entries(cache: Option<&[OwnPlaylist]>) -> Vec<Entry> {
    match cache {
        None => vec![Entry::Disabled(tr("Loading…"))],
        Some([]) => vec![Entry::Disabled(tr("No playlists"))],
        Some(list) => list.iter().map(|p| Entry::Playlist { label: shorten(&p.name), playlist_id: p.id.clone() }).collect(),
    }
}

/// Longest playlist name shown before it is cut with "…", so the menu doesn't widen.
const MAX_LABEL_CHARS: usize = 24;

fn shorten(name: &str) -> String {
    if name.chars().count() <= MAX_LABEL_CHARS {
        return name.to_owned();
    }
    let mut s: String = name.chars().take(MAX_LABEL_CHARS - 1).collect();
    s.truncate(s.trim_end().len());
    s.push('…');
    s
}

fn fill(menu: &gio::Menu, track_id: &str) {
    menu.remove_all();
    let es = CACHE.with_borrow(|c| entries(if let Cache::Ready(l) = c { Some(l.as_slice()) } else { None }));
    for e in es {
        match e {
            Entry::Disabled(label) => menu.append(Some(&label), Some(DISABLED)),
            Entry::Playlist { label, playlist_id } => {
                let item = gio::MenuItem::new(Some(&label), None);
                item.set_action_and_target_value(Some(ACTION), Some(&(playlist_id, track_id.to_owned()).to_variant()));
                menu.append_item(&item);
            }
        }
    }
}

/// Builds the submenu for `track_id`; call [`ensure_loaded`] when the menu opens.
pub fn submenu(track_id: &str) -> gio::Menu {
    let menu = gio::Menu::new();
    fill(&menu, track_id);
    menu
}

/// Starts loading the playlists if needed and refreshes `menu` when done.
pub fn ensure_loaded(menu: &gio::Menu, track_id: &str) {
    let start = CACHE.with_borrow_mut(|c| match c {
        Cache::Ready(_) => false,
        Cache::Loading => {
            WAITING.with_borrow_mut(|w| w.push((menu.downgrade(), track_id.to_owned())));
            false
        }
        Cache::Empty => {
            *c = Cache::Loading;
            WAITING.with_borrow_mut(|w| w.push((menu.downgrade(), track_id.to_owned())));
            true
        }
    });
    if !start {
        return;
    }
    let epoch = ACCOUNT_EPOCH.get();
    glib::spawn_future_local(async move {
        finish_load(epoch, api::run(api::client::own_playlists).await);
    });
}

fn finish_load(epoch: u64, result: anyhow::Result<Vec<OwnPlaylist>>) {
    if ACCOUNT_EPOCH.get() != epoch {
        return;
    }
    let list = match result {
        Ok(l) => Some(l),
        Err(e) => {
            log::error!("loading own playlists failed error={e:#}");
            player::toasts().notify(&tr("Could not load your playlists"));
            None
        }
    };
    CACHE.with_borrow_mut(|c| *c = list.map_or(Cache::Empty, Cache::Ready));
    for (m, track) in WAITING.with_borrow_mut(std::mem::take) {
        if let Some(m) = m.upgrade() {
            fill(&m, &track);
        }
    }
}

/// Adds a track to a playlist and reports the result with a toast.
pub fn add(playlist_id: String, track_id: String) {
    let name = CACHE.with_borrow(|c| match c {
        Cache::Ready(l) => l.iter().find(|p| p.id == playlist_id).map(|p| p.name.clone()),
        _ => None,
    });
    glib::spawn_future_local(async move {
        match api::run(move || api::client::playlist_add_track(&playlist_id, &track_id)).await {
            Ok(()) => player::toasts().notify(&match name {
                Some(n) => crate::i18n::trf("Added to %s", &[&n]),
                None => tr("Added to playlist"),
            }),
            Err(e) => {
                log::error!("adding track to playlist failed error={e:#}");
                player::toasts().notify(&tr("Could not add the track to the playlist"));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pl(id: &str, name: &str) -> OwnPlaylist {
        OwnPlaylist { id: id.into(), name: name.into(), number_of_items: 3 }
    }

    fn labels(menu: &gio::Menu) -> Vec<String> {
        (0..menu.n_items())
            .filter_map(|i| menu.item_attribute_value(i, "label", None)?.get())
            .collect()
    }

    #[test]
    fn account_switch_forgets_the_playlists() {
        finish_load(ACCOUNT_EPOCH.get(), Ok(vec![pl("a", "Old account")]));
        assert_eq!(labels(&submenu("t")), ["Old account"]);
        reset();
        assert_eq!(labels(&submenu("t")), [tr("Loading…")]);
    }

    #[test]
    fn a_load_from_the_previous_account_is_dropped() {
        let previous = ACCOUNT_EPOCH.get();
        reset();
        let menu = submenu("t");
        WAITING.with_borrow_mut(|w| w.push((menu.downgrade(), "t".to_owned())));
        finish_load(previous, Ok(vec![pl("a", "Old account")]));
        assert_eq!(labels(&menu), [tr("Loading…")]);
        finish_load(ACCOUNT_EPOCH.get(), Ok(vec![pl("b", "New account")]));
        assert_eq!(labels(&menu), ["New account"]);
    }

    #[test]
    fn long_names_are_shortened() {
        assert_eq!(shorten("Focus"), "Focus");
        let long = "A very long playlist name that keeps on going";
        let s = shorten(long);
        assert!(s.ends_with('…'));
        assert!(s.chars().count() <= MAX_LABEL_CHARS);
        assert_eq!(shorten("ñññññññññññññññññññññññññññññññññññ").chars().count(), MAX_LABEL_CHARS);
    }

    #[test]
    fn loading_none_and_list() {
        assert!(matches!(entries(None).as_slice(), [Entry::Disabled(_)]));
        assert!(matches!(entries(Some(&[])).as_slice(), [Entry::Disabled(_)]));
        let l = [pl("a", "Road trip"), pl("b", "Focus")];
        assert_eq!(
            entries(Some(&l)),
            vec![
                Entry::Playlist { label: "Road trip".into(), playlist_id: "a".into() },
                Entry::Playlist { label: "Focus".into(), playlist_id: "b".into() },
            ]
        );
    }
}
