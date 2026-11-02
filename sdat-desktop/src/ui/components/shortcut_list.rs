//! Port of `components/shortcut_list`.
use adw::prelude::*;
use gtk::pango;

use crate::api::legacy;
use crate::i18n::{tr, trn};
use crate::model::image_url;
use crate::ui::dsl::{W, label, vstack, hstack};
use crate::{imgutil, router, settings};

pub struct ShortcutList {
    pub widget: adw::WrapBox,
}

impl ShortcutList {
    pub fn new() -> Self {
        let widget = adw::WrapBox::new();
        widget.set_child_spacing(10);
        widget.set_line_spacing(10);
        ShortcutList { widget }
    }
    pub fn append(&self, w: &impl IsA<gtk::Widget>) {
        self.widget.append(w);
    }
}

fn texts(title: &str, subtitle: &str, align: gtk::Align) -> gtk::Box {
    let t = label(title).halign(align).class("heading");
    t.set_ellipsize(pango::EllipsizeMode::End);
    let s = label(subtitle).halign(align).visible(!subtitle.is_empty()).font_weight(500).class("dimmed");
    s.set_ellipsize(pango::EllipsizeMode::End);
    vstack(&[t.upcast_ref(), s.upcast_ref()]).halign(align).valign(gtk::Align::Center).hexp(true)
}

pub fn shortcut(title: &str, subtitle: &str, cover_url: &str) -> gtk::Button {
    let image = gtk::Image::from_paintable(Some(&imgutil::missing_album()));
    image.set_pixel_size(54);
    let url = cover_url.to_string();
    image.connect_realize(move |i| {
        if settings::allow_shortcut_images() {
            imgutil::load_into_image(&url, i, true);
        }
    });
    let frame = gtk::AspectFrame::new(0.5, 0.5, 1.0, false);
    frame.set_child(Some(&image));
    let frame = frame.corner_radius(10).overflow_hidden().halign(gtk::Align::End).mstart(10);
    let b = gtk::Button::new().hexp(true).min_width(300).pstart(15).pend(5).vpadding(5);
    b.set_child(Some(&hstack(&[texts(title, subtitle, gtk::Align::Start).upcast_ref(), frame.upcast_ref()])));
    b
}

pub fn text_shortcut(title: &str, subtitle: &str) -> gtk::Button {
    let b = gtk::Button::new().hexp(true).pstart(15).pend(15).vpadding(5);
    b.set_child(Some(&hstack(&[texts(title, subtitle, gtk::Align::Center).upcast_ref()])));
    b
}

fn with_action(b: gtk::Button, action: &str, target: &str) -> gtk::Button {
    b.set_action_name(Some(action));
    b.set_action_target_value(Some(&glib::Variant::from(target)));
    b
}

pub fn legacy_album(a: &legacy::AlbumItemData) -> gtk::Button {
    let artists: Vec<_> = a.artists.iter().map(|a| a.name.clone()).collect();
    with_action(shortcut(&a.title, &artists.join(", "), &image_url(&a.cover)), "win.route.album", &a.id.to_string())
}

pub fn legacy_artist(a: &legacy::ArtistItemData) -> gtk::Button {
    with_action(shortcut(&a.name, "", &image_url(&a.picture)), "win.route.artist", &a.id.to_string())
}

pub fn legacy_mix(m: &legacy::MixItemData) -> gtk::Button {
    let url = m.mix_images.first().map(|i| i.url.clone()).unwrap_or_default();
    with_action(shortcut(&m.title_text_info.text, &m.subtitle_text_info.text, &url), "win.route.playlist", &m.id)
}

pub fn legacy_playlist(p: &legacy::PlaylistItemData) -> gtk::Button {
    let sub = trn("%d Track", "%d Tracks", p.number_of_tracks, &[&p.number_of_tracks]);
    with_action(shortcut(&p.title, &sub, &image_url(&p.square_image)), "win.route.playlist", &p.uuid)
}

pub fn legacy_deeplink(d: &legacy::DeepLinkItemData) -> gtk::Button {
    let cover = if d.id == "tidal://my-collection/tracks" { "https://tidal.com/assets/my-tracks-DTG3pLQW.png" } else { "" };
    let sub = if d.external_url { tr("External Link") } else { String::new() };
    let b = shortcut(&d.title, &sub, cover);
    let (external, url) = (d.external_url, d.url.clone());
    b.connect_clicked(move |b| {
        if external {
            let win = b.root().and_downcast::<gtk::Window>();
            gtk::UriLauncher::new(&url).launch(win.as_ref(), gtk::gio::Cancellable::NONE, |_| {});
        } else {
            router::navigate(&url);
        }
    });
    b
}
