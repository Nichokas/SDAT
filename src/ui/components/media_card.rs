//! Port of `components/media_card`.
use adw::prelude::*;
use gtk::{glib, pango};

use crate::api::legacy;
use crate::i18n::trn;
use crate::model::{Album, ArtistInfo, Cover, Playlist, Track};
use crate::ui::dsl::*;
use crate::{imgutil, router, settings};

pub fn subtitle(text: &str) -> gtk::Label {
    let l = label(text).font_weight(400).class("dimmed").halign(gtk::Align::Start);
    l.set_max_width_chars(15);
    l.set_ellipsize(pango::EllipsizeMode::End);
    l
}

pub fn card(title: &str, sub: &impl IsA<gtk::Widget>, cover: Cover) -> gtk::Button {
    let image = gtk::Image::from_paintable(Some(&imgutil::missing_album()));
    image.set_pixel_size(172);
    let image = image.background("var(--sidebar-bg-color)").corner_radius(10).overflow_hidden();
    image.connect_realize(move |i| {
        if settings::allow_mediacard_images() {
            if cover.has_static() {
                imgutil::load_into_image(&cover.static_, i, true);
            } else {
                i.set_paintable(Some(&imgutil::missing_album()));
            }
        }
    });
    let t = label(title).class("heading").mtop(10).halign(gtk::Align::Start);
    t.set_max_width_chars(15);
    t.set_ellipsize(pango::EllipsizeMode::End);
    sub.as_ref().set_margin_top(2);
    let b = gtk::Button::new().padding(10).hexp(false).vexp(false).class("flat");
    b.set_child(Some(&vstack(&[image.upcast_ref(), t.upcast_ref(), sub.upcast_ref()])));
    b
}

fn with_action(b: gtk::Button, action: &str, target: &str) -> gtk::Button {
    b.set_action_name(Some(action));
    b.set_action_target_value(Some(&glib::Variant::from(target)));
    b
}

pub fn album(a: &Album) -> gtk::Button {
    let names = crate::model::artist_names(&a.artists).join(", ");
    let year = a.info.released_at.map(|d| d.format("%Y").to_string()).unwrap_or_default();
    let sub = vstack(&[subtitle(&names).upcast_ref(), subtitle(&year).upcast_ref()]);
    with_action(card(&a.info.title, &sub, a.info.cover(172)), "win.route.album", &a.info.id)
}

pub fn artist(a: &ArtistInfo) -> gtk::Button {
    with_action(card(a.title(), &gtk::Box::new(gtk::Orientation::Horizontal, 0), a.cover(192)), "win.route.artist", &a.id)
}

pub fn playlist(p: &Playlist) -> gtk::Button {
    let creator = p.creator.as_ref().map(|c| c.title().to_string()).unwrap_or_else(|| "TIDAL".into());
    let sub = vstack(&[
        subtitle(&creator).upcast_ref(),
        subtitle(&trn("%d Track", "%d Tracks", p.count, &[&p.count])).upcast_ref(),
    ]);
    with_action(card(&p.info.title, &sub, p.info.cover(192)), "win.route.playlist", &p.info.id)
}

pub fn track(t: &Track) -> gtk::Button {
    with_action(card(t.title(), &subtitle(&t.artist_names().join(", ")), t.cover(192)), "win.player.play-track", t.id())
}

pub fn mix_generic(id: &str, title: &str, sub: &str, cover_url: &str) -> gtk::Button {
    let s = subtitle(sub);
    s.set_lines(2);
    with_action(card(title, &s, Cover::new("", cover_url)), "win.route.playlist", id)
}

pub fn legacy_mix(m: &legacy::MixItemData) -> gtk::Button {
    let url = m.mix_images.first().map(|i| i.url.clone()).unwrap_or_default();
    mix_generic(&m.id, &m.title_text_info.text, &m.subtitle_text_info.text, &url)
}

pub fn deeplink(url: &str, name: &str, cover_url: &str) -> gtk::Button {
    let b = card(name, &gtk::Box::new(gtk::Orientation::Horizontal, 0), Cover::new("", cover_url));
    let url = url.to_string();
    b.connect_clicked(move |_| router::navigate(&url));
    b
}

pub fn legacy_deeplink(d: &legacy::DeepLinkItemData) -> gtk::Button {
    let cover = if d.id == "tidal://my-collection/tracks" { "https://tidal.com/assets/my-tracks-DTG3pLQW.png" } else { "" };
    deeplink(&d.url, &d.title, cover)
}
