//! Port of `components/favourite_button`.
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use adw::prelude::*;

use crate::{api, global_signal, secrets};
use crate::i18n::tr;
use crate::signals::{CONTINUE, Signal, State, UNSUBSCRIBE};
use crate::state::{self, FavouriteCache};
use crate::ui::dsl::{W, on_state};

fn render(b: &gtk::Button, loading: bool, fav: bool) {
    if loading {
        b.set_child(Some(&adw::Spinner::new()));
        b.remove_css_class("accent");
    } else if fav {
        b.set_icon_name("heart-filled-symbolic");
        b.add_css_class("accent");
        b.set_tooltip_text(Some(&tr("Remove from Collection")));
    } else {
        b.set_icon_name("heart-outline-thick-symbolic");
        b.remove_css_class("accent");
        b.set_tooltip_text(Some(&tr("Add to Collection")));
    }
}

pub fn favourite_button(cache: &'static FavouriteCache, id: &str) -> gtk::Button {
    let b = gtk::Button::from_icon_name("heart-outline-thick-symbolic").class("flat");
    on_state(state::signed_in_changed(), &b, |b, s| b.set_sensitive(*s));
    let fav = Rc::new(Cell::new(false));
    let loading = Rc::new(Cell::new(false));
    let id = id.to_string();
    {
        let (fav, loading, id) = (Rc::clone(&fav), Rc::clone(&loading), id.clone());
        b.connect_realize(move |b| {
            loading.set(true);
            render(b, true, fav.get());
            let w = b.downgrade();
            let (fav, loading, id) = (Rc::clone(&fav), Rc::clone(&loading), id.clone());
            glib::spawn_future_local(async move {
                let id2 = id.clone();
                match api::run(move || cache.contains(&id2)).await {
                    Ok(v) => fav.set(v),
                    Err(e) => log::error!("Failed to get favourites error={e:#}"),
                }
                loading.set(false);
                if let Some(b) = w.upgrade() {
                    render(&b, false, fav.get());
                }
            });
        })
    };
    b.connect_clicked(move |b| {
        if loading.get() {
            return;
        }
        loading.set(true);
        render(b, true, fav.get());
        let w = b.downgrade();
        let (fav, loading, id) = (Rc::clone(&fav), Rc::clone(&loading), id.clone());
        glib::spawn_future_local(async move {
            let was = fav.get();
            let id2 = id.clone();
            let r = api::run(move || if was { cache.remove(&id2) } else { cache.add(&id2) }).await;
            match r {
                Ok(()) => fav.set(!was),
                Err(e) if was => log::error!("error while removing item from favourites error={e:#}"),
                Err(e) => log::error!("error while adding item to favourites error={e:#}"),
            }
            loading.set(false);
            if let Some(b) = w.upgrade() {
                render(&b, false, fav.get());
            }
        });
    });
    b
}

struct TrackFavouriteChange {
    id: String,
    favourite: bool,
}

global_signal!(fn track_favourite_changed() -> Signal<TrackFavouriteChange> = Signal::default());
global_signal!(pub(crate) fn track_favourites_available() -> State<bool> = State::new(state::signed_in_changed().get()));

thread_local! {
    static TRACK_WRITES: RefCell<HashSet<(u64, String)>> = RefCell::default();
}

#[derive(Default)]
struct TrackBinding {
    id: RefCell<Option<String>>,
    revision: Cell<u64>,
    favourite: Cell<Option<bool>>,
}

struct PendingTrackWrite {
    key: (u64, String),
    button: glib::WeakRef<gtk::Button>,
    binding: Rc<TrackBinding>,
    revision: u64,
    was_favourite: bool,
}

impl Drop for PendingTrackWrite {
    fn drop(&mut self) {
        TRACK_WRITES.with_borrow_mut(|writes| writes.remove(&self.key));
        if self.binding.revision.get() == self.revision {
            self.binding.favourite.set(Some(self.was_favourite));
            if let Some(button) = self.button.upgrade() {
                render(&button, false, self.was_favourite);
            }
        }
    }
}

/// Rebinds a [`track_favourite_button`] to a track ID (`None` = unbound).
pub type SetId = Rc<dyn Fn(Option<&str>)>;

pub fn track_favourite_button() -> (gtk::Button, SetId) {
    let b = gtk::Button::from_icon_name("heart-outline-thick-symbolic").class("flat");
    let binding = Rc::new(TrackBinding::default());
    b.set_sensitive(false);
    let changes = track_favourite_changed();
    let (w, bound) = (b.downgrade(), Rc::clone(&binding));
    let sub = changes.on(move |change| {
        let Some(b) = w.upgrade() else { return UNSUBSCRIBE };
        if bound.id.borrow().as_deref() == Some(change.id.as_str()) {
            bound.revision.set(bound.revision.get() + 1);
            bound.favourite.set(Some(change.favourite));
            render(&b, false, change.favourite);
        }
        CONTINUE
    });
    let set: SetId = {
        let (w, binding) = (b.downgrade(), Rc::clone(&binding));
        Rc::new(move |id: Option<&str>| {
            let Some(b) = w.upgrade() else { return };
            let revision = binding.revision.get() + 1;
            binding.revision.set(revision);
            *binding.id.borrow_mut() = id.map(str::to_owned);
            binding.favourite.set(None);
            let available = track_favourites_available().get();
            b.set_sensitive(id.is_some() && available);
            let Some(id) = id.filter(|_| available) else {
                render(&b, false, false);
                return;
            };
            render(&b, true, false);
            let (w, binding, id) = (w.clone(), Rc::clone(&binding), id.to_owned());
            glib::spawn_future_local(async move {
                let result = api::run(move || state::TRACKS.contains(&id)).await;
                if binding.revision.get() != revision {
                    return;
                }
                let favourite = result.unwrap_or_else(|e| {
                    log::error!("Failed to get favourites error={e:#}");
                    false
                });
                binding.favourite.set(Some(favourite));
                if let Some(b) = w.upgrade() {
                    render(&b, false, favourite);
                }
            });
        })
    };
    let availability = track_favourites_available();
    let (bound, rebind) = (Rc::clone(&binding), Rc::clone(&set));
    let availability_sub = availability.on_lazy(move |_| {
        let id = bound.id.borrow().clone();
        rebind(id.as_deref());
        CONTINUE
    });
    b.connect_destroy(move |_| {
        changes.unsubscribe(sub);
        availability.unsubscribe(availability_sub);
    });
    b.connect_clicked(move |b| {
        let (Some(id), Some(was_favourite)) = (binding.id.borrow().clone(), binding.favourite.get()) else { return };
        let generation = secrets::session_generation();
        let key = (generation, id);
        if !TRACK_WRITES.with_borrow_mut(|writes| writes.insert(key.clone())) {
            return;
        }
        let pending = PendingTrackWrite {
            key,
            button: b.downgrade(),
            binding: Rc::clone(&binding),
            revision: binding.revision.get(),
            was_favourite,
        };
        binding.favourite.set(None);
        render(b, true, false);
        let request_id = pending.key.1.clone();
        glib::spawn_future_local(async move {
            let result = api::run(move || {
                secrets::with_session(generation, || {
                    if was_favourite { state::TRACKS.remove(&request_id) } else { state::TRACKS.add(&request_id) }
                })
            })
            .await;
            match result {
                Ok(Some(())) if secrets::session_generation() == generation && track_favourites_available().get() => {
                    changes.notify(&TrackFavouriteChange { id: pending.key.1.clone(), favourite: !was_favourite });
                }
                Err(e) => log::error!("error while updating favourites error={e:#}"),
                _ => {}
            }
            drop(pending);
        });
    });
    (b, set)
}
