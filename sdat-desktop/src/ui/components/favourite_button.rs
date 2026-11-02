//! Port of `components/favourite_button`.
use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;

use crate::i18n::tr;
use crate::state::{self, FavouriteCache};
use crate::ui::dsl::{W, on_state};
use crate::api;

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

/// Rebinds a [`rebindable_favourite_button`] to a track id (`None` = unbound).
pub type SetId = Rc<dyn Fn(Option<&str>)>;

/// A favourite button for recycled list rows: call the setter with the bound id
/// (or `None` on unbind). Stale lookups for a previous id are ignored.
pub fn rebindable_favourite_button(cache: &'static FavouriteCache) -> (gtk::Button, SetId) {
    use std::cell::RefCell;
    let b = gtk::Button::from_icon_name("heart-outline-thick-symbolic").class("flat");
    on_state(state::signed_in_changed(), &b, |b, s| b.set_sensitive(*s));
    let id: Rc<RefCell<Option<String>>> = Rc::default();
    let fav = Rc::new(Cell::new(false));
    let loading = Rc::new(Cell::new(false));
    let gen_ = Rc::new(Cell::new(0u64));
    let set: SetId = {
        let (w, id, fav, loading, gen_) = (b.downgrade(), Rc::clone(&id), Rc::clone(&fav), Rc::clone(&loading), Rc::clone(&gen_));
        Rc::new(move |new: Option<&str>| {
            gen_.set(gen_.get() + 1);
            *id.borrow_mut() = new.map(str::to_string);
            let (Some(new), Some(b)) = (new, w.upgrade()) else { return };
            fav.set(false);
            loading.set(true);
            render(&b, true, false);
            let mine = gen_.get();
            let (w, fav, loading, gen_, id2) = (w.clone(), Rc::clone(&fav), Rc::clone(&loading), Rc::clone(&gen_), new.to_string());
            glib::spawn_future_local(async move {
                let r = api::run(move || cache.contains(&id2)).await;
                if gen_.get() != mine {
                    return;
                }
                match r {
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
        let Some(cur) = id.borrow().clone() else { return };
        if loading.get() {
            return;
        }
        loading.set(true);
        render(b, true, fav.get());
        let mine = gen_.get();
        let (w, fav, loading, gen_) = (b.downgrade(), Rc::clone(&fav), Rc::clone(&loading), Rc::clone(&gen_));
        glib::spawn_future_local(async move {
            let was = fav.get();
            let id2 = cur.clone();
            let r = api::run(move || if was { cache.remove(&id2) } else { cache.add(&id2) }).await;
            if let Err(e) = &r {
                log::error!("error while updating favourites error={e:#}");
            }
            if gen_.get() != mine {
                return; // row was recycled meanwhile; the new binding already rendered
            }
            if r.is_ok() {
                fav.set(!was);
            }
            loading.set(false);
            if let Some(b) = w.upgrade() {
                render(&b, false, fav.get());
            }
        });
    });
    (b, set)
}
