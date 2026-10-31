//! Port of `components/favourite_button`.
use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::i18n::tr;
use crate::state::{self, FavouriteCache};
use crate::ui::dsl::*;
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
        let (fav, loading, id) = (fav.clone(), loading.clone(), id.clone());
        b.connect_realize(move |b| {
            loading.set(true);
            render(b, true, fav.get());
            let w = b.downgrade();
            let (fav, loading, id) = (fav.clone(), loading.clone(), id.clone());
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
        });
    }
    b.connect_clicked(move |b| {
        if loading.get() {
            return;
        }
        loading.set(true);
        render(b, true, fav.get());
        let w = b.downgrade();
        let (fav, loading, id) = (fav.clone(), loading.clone(), id.clone());
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
