//! Port of `components/tracklist_header`.
use adw::prelude::*;
use gtk::{gdk, gio, pango};

use crate::i18n::{tr, trn};
use crate::model::{format_duration, Album, Artist, PlaybackSource, Playlist};
use crate::signals::State;
use crate::state;
use crate::ui::dsl::*;
use crate::{global_signal, imgutil, player};

global_signal!(fn controls_available() -> State<bool> = {
    let s = State::new(false);
    s
});

/// Mirrors `!PlaybackState.Loading` (the Go init() subscription).
pub fn init() {
    player::playback_state_changed().on(|ps| {
        controls_available().set(!ps.loading);
        crate::signals::CONTINUE
    });
}

fn controls(play: impl Fn() + 'static, shuffle: impl Fn() + 'static) -> gtk::Box {
    let s = gtk::Button::from_icon_name("playlist-shuffle-symbolic").tooltip(&tr("Shuffle Album")).class("pill");
    on_state(controls_available(), &s, |b, v| b.set_sensitive(*v));
    s.connect_clicked(move |_| shuffle());
    let p = gtk::Button::from_icon_name("play-symbolic").tooltip(&tr("Play Album")).class("pill").class("suggested-action");
    on_state(controls_available(), &p, |b, v| b.set_sensitive(*v));
    p.connect_clicked(move |_| play());
    spacing(hstack(&[s.upcast_ref(), p.upcast_ref()]), 12).halign(gtk::Align::End).hexp(true)
}

fn cover_widget(cover: crate::model::Cover) -> gtk::AspectFrame {
    let img = gtk::Image::from_paintable(Some(&imgutil::missing_album()));
    img.set_pixel_size(146);
    img.connect_realize(move |i| {
        let w = i.downgrade();
        super::render_cover(cover.clone(), move |p| {
            if let Some(i) = w.upgrade() {
                i.set_paintable(Some(&p));
            }
        });
    });
    let f = gtk::AspectFrame::new(0.5, 0.5, 1.0, true);
    f.set_child(Some(&img));
    f.corner_radius(10).overflow_hidden().class("cover")
}

fn secondary(url: String, popover: &gtk::PopoverMenu, mut buttons: Vec<gtk::Widget>) -> gtk::Box {
    let share = gtk::Button::from_icon_name("share-alt-symbolic").tooltip(&tr("Copy URL")).class("flat");
    share.connect_clicked(move |_| {
        if let Some(d) = gdk::Display::default() {
            d.clipboard().set_text(&url);
        }
        player::toasts().notify(&tr("Copied URL to clipboard"));
    });
    let more = gtk::MenuButton::new().tooltip(&tr("More…")).class("flat").class("circular");
    more.set_popover(Some(popover));
    more.set_icon_name("view-more-symbolic");
    buttons.push(share.upcast());
    buttons.push(more.upcast());
    let refs: Vec<&gtk::Widget> = buttons.iter().collect();
    spacing(hstack(&refs), 12).halign(gtk::Align::End).hexp(true).valign(gtk::Align::Center)
}

fn slot(name: &str) -> gtk::Widget {
    adw::LayoutSlot::new(name).upcast()
}

fn desc_label(description: &str) -> gtk::Label {
    let l = label(description).class("dimmed").hexp(true);
    l.set_wrap(true);
    l.set_lines(3);
    l.set_ellipsize(pango::EllipsizeMode::End);
    l
}

fn layout_desktop(description: &str) -> adw::Layout {
    let titles = vstack(&[&slot("title"), &slot("subtitle")]).halign(gtk::Align::Start);
    let child = hstack(&[
        &slot("cover"),
        vstack(&[
            hstack(&[titles.upcast_ref(), &slot("controls")]).upcast_ref(),
            hstack(&[desc_label(description).halign(gtk::Align::Start).upcast_ref(), slot("secondaryControls").mstart(10).upcast_ref()])
                .mtop(20)
                .upcast_ref(),
        ])
        .mstart(20)
        .valign(gtk::Align::Center)
        .upcast_ref(),
    ])
    .class("tracklist_header")
    .hexp(true);
    let l = adw::Layout::new(&child);
    l.set_name(Some("desktop"));
    l
}

fn layout_mobile(description: &str) -> adw::Layout {
    let d = desc_label(description).halign(gtk::Align::Center).size(-1, 54);
    d.set_justify(gtk::Justification::Center);
    let child = spacing(
        vstack(&[
            slot("cover").halign(gtk::Align::Center).upcast_ref(),
            vstack(&[slot("title").halign(gtk::Align::Center).upcast_ref(), slot("subtitle").halign(gtk::Align::Center).upcast_ref()])
                .halign(gtk::Align::Center)
                .mtop(10)
                .upcast_ref(),
            d.upcast_ref(),
            slot("controls").halign(gtk::Align::Center).upcast_ref(),
            slot("secondaryControls").halign(gtk::Align::Center).upcast_ref(),
        ]),
        10,
    )
    .class("tracklist_header")
    .hexp(true);
    let l = adw::Layout::new(&child);
    l.set_name(Some("mobile"));
    l
}

fn template(cover: crate::model::Cover, title: &str, subtitle: &str, description: &str, controls: gtk::Box, secondary: gtk::Box) -> gtk::Widget {
    let layout = adw::MultiLayoutView::new();
    layout.set_child("cover", &cover_widget(cover));
    let t = label(title).class("title-2").halign(gtk::Align::Start);
    t.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_child("title", &t);
    let s = label(subtitle).class("heading").class("dimmed").halign(gtk::Align::Start);
    s.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_child("subtitle", &s);
    layout.set_child("controls", &controls);
    layout.set_child("secondaryControls", &secondary);
    layout.add_layout(layout_desktop(description));
    layout.add_layout(layout_mobile(description));
    let bin = adw::BreakpointBin::new();
    bin.set_child(Some(&layout));
    bin.set_size_request(150, 150);
    let bp = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 600px").unwrap());
    bp.add_setter(&layout, "layout-name", Some(&"mobile".to_value()));
    bin.add_breakpoint(bp);
    bin.upcast()
}

fn queue_menu(label: &str, target: &str) -> gtk::PopoverMenu {
    let menu = gio::Menu::new();
    let item = gio::MenuItem::new(Some(label), Some("win.player.queue"));
    item.set_action_and_target_value(Some("win.player.queue"), Some(&target.to_variant()));
    menu.append_item(&item);
    gtk::PopoverMenu::from_model(Some(&menu))
}

fn year(d: Option<String>) -> String {
    d.unwrap_or_else(|| "0001".into())
}

pub fn album(a: &Album, play: impl Fn() + 'static, shuffle: impl Fn() + 'static) -> gtk::Widget {
    let fav = super::favourite_button::favourite_button(&state::ALBUMS, &a.info.id);
    let mut buttons: Vec<gtk::Widget> = Vec::new();
    if a.artists.len() > 1 {
        let menu = gio::Menu::new();
        for ar in &a.artists {
            menu.append_item(&gio::MenuItem::new(Some(ar.title()), Some(&format!("win.route.artist::{}", ar.id))));
        }
        let mb = gtk::MenuButton::new().tooltip(&tr("Navigate to Artist")).class("flat");
        mb.set_icon_name("music-artist2-symbolic");
        mb.set_menu_model(Some(&menu));
        buttons.push(mb.upcast());
    } else if let Some(ar) = a.artists.first() {
        let b = gtk::Button::from_icon_name("music-artist2-symbolic").tooltip(&tr("Navigate to Artist")).class("flat");
        b.set_action_name(Some("win.route.artist"));
        b.set_action_target_value(Some(&ar.id.to_variant()));
        buttons.push(b.upcast());
    }
    buttons.push(fav.upcast());
    let popover = queue_menu(&tr("Add Album to Queue"), &format!("album/{}", a.info.id));
    let release = year(a.info.released_at.map(|d| d.format("%Y").to_string()));
    let artists = crate::model::artist_names(&a.artists).join(", ");
    let desc = trn("%d Track (%s)", "%d Tracks (%s)", a.count, &[&a.count, &format_duration(a.info.duration)]);
    template(
        a.info.cover(154),
        &a.info.title,
        &format!("{release} • {artists}"),
        &format!("\n{desc}\n"),
        controls(play, shuffle),
        secondary(a.info.url(), &popover, buttons),
    )
}

pub fn artist(a: &Artist, play: impl Fn() + 'static, shuffle: impl Fn() + 'static) -> gtk::Widget {
    let fav = super::favourite_button::favourite_button(&state::ARTISTS, &a.info.id);
    let fans = trn("%d Fan", "%d Fans", a.follower_count, &[&a.follower_count]);
    let popover = queue_menu(&tr("Add Top Tracks to Queue"), &format!("artist/{}", a.info.id));
    template(a.info.cover(154), a.info.title(), &fans, &a.description, controls(play, shuffle), secondary(a.info.url(), &popover, vec![fav.upcast()]))
}

pub fn collection(src: &PlaybackSource, play: impl Fn() + 'static, shuffle: impl Fn() + 'static) -> gtk::Widget {
    let popover = queue_menu(&tr("Add My Tracks to Queue"), "my_collection/tracks");
    template(src.cover(154), &src.title(), &tr("My Collection"), "", controls(play, shuffle), secondary(src.url(), &popover, vec![]))
}

pub fn playlist(p: &Playlist, play: impl Fn() + 'static, shuffle: impl Fn() + 'static) -> gtk::Widget {
    let cache = if p.info.is_mix() { &state::MIXES } else { &state::PLAYLISTS };
    let fav = super::favourite_button::favourite_button(cache, &p.info.id);
    let release = year(p.info.created_at.map(|d| d.format("%Y").to_string()));
    let creator = p.creator.as_ref().map(|c| c.title().to_string()).unwrap_or_else(|| "TIDAL".into());
    let desc = if p.info.is_mix() {
        tr("Personal Mix")
    } else {
        format!("{} Track ({})", p.count, format_duration(p.info.duration))
    };
    let popover = queue_menu(&tr("Add Playlist to Queue"), &format!("playlist/{}", p.info.id));
    template(
        p.info.cover(154),
        &p.info.title,
        &format!("{release} • {creator}"),
        &format!("\n{desc}\n"),
        controls(play, shuffle),
        secondary(p.info.url(), &popover, vec![fav.upcast()]),
    )
}
