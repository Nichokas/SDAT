//! Handoff: the "continue here" banner and the "Send to" menu.
use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, pango};

use crate::handoff;
use crate::i18n::{tr, trf};
use crate::ui::components::{render_cover, CancelToken};
use crate::ui::dsl::{W, label, on_state, spacing, vstack, hstack};
use crate::{imgutil, player};

fn now_ms() -> i64 {
    glib::real_time() / 1000
}

/// What another device plays, with a "Continue Here" button. Shown above
/// every sidebar page while there is something to continue.
pub fn banner() -> gtk::Revealer {
    let cover = gtk::Image::from_paintable(Some(&imgutil::missing_album()));
    cover.set_pixel_size(48);
    let frame = gtk::AspectFrame::new(0.5, 0.5, 1.0, true);
    frame.set_child(Some(&cover));
    let frame = frame.overflow_hidden().background("alpha(var(--view-fg-color), 0.1)").corner_radius(6).valign(gtk::Align::Center);
    let title = label("").font_weight(600).halign(gtk::Align::Start);
    let artist = label("").class("dimmed").halign(gtk::Align::Start);
    let status = label("").class("caption").class("dimmed").halign(gtk::Align::Start);
    for l in [&title, &artist, &status] {
        l.set_ellipsize(pango::EllipsizeMode::End);
    }
    let texts = spacing(vstack(&[title.upcast_ref(), artist.upcast_ref(), status.upcast_ref()]), 2).valign(gtk::Align::Center).hexp(true);
    let button = gtk::Button::with_label(&tr("Continue Here")).class("suggested-action").class("pill").valign(gtk::Align::Center);
    button.connect_clicked(|_| handoff::continue_here());
    let row = spacing(hstack(&[frame.upcast_ref(), texts.upcast_ref(), button.upcast_ref()]), 12)
        .padding(10)
        .background("alpha(var(--view-fg-color), 0.1)")
        .corner_radius(12)
        .hmargin(12)
        .mtop(6);
    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
    revealer.set_child(Some(&row));

    let token: Rc<RefCell<Option<(String, CancelToken)>>> = Rc::default();
    let (tw, aw, sw, cw) = (title.downgrade(), artist.downgrade(), status.downgrade(), cover.downgrade());
    on_state(handoff::banner(), &revealer, move |r, e| {
        r.set_reveal_child(e.is_some());
        let Some(e) = e else { return };
        let (Some(title), Some(artist), Some(status)) = (tw.upgrade(), aw.upgrade(), sw.upgrade()) else { return };
        title.set_text(&e.track.formatted_title());
        artist.set_text(&e.track.artist_names().join(", "));
        status.set_text(&handoff::status_line(e, now_ms()));
        let id = e.track.id().to_string();
        if token.borrow().as_ref().is_some_and(|(t, _)| *t == id) {
            return;
        }
        if let Some((_, t)) = token.borrow_mut().take() {
            t.cancel();
        }
        let cw = cw.clone();
        *token.borrow_mut() = Some((id, render_cover(e.track.cover(80), move |pt| {
            if let Some(c) = cw.upgrade() {
                c.set_paintable(Some(&pt));
            }
        })));
    });
    // Keeps "2 h ago" current.
    let sw = status.downgrade();
    glib::timeout_add_seconds_local(30, move || {
        let Some(s) = sw.upgrade() else { return glib::ControlFlow::Break };
        if let Some(e) = handoff::banner().get() {
            s.set_text(&handoff::status_line(&e, now_ms()));
        }
        glib::ControlFlow::Continue
    });
    revealer
}

/// Menu button with one "Send to <device>" entry per other device; hidden
/// when there are none or nothing is loaded.
pub fn send_to_button() -> gtk::MenuButton {
    let mb = gtk::MenuButton::new().tooltip(&tr("Send to Another Device")).class("flat");
    mb.set_icon_name("send-to-symbolic");
    on_state(handoff::devices(), &mb, |m, devices| {
        let menu = gio::Menu::new();
        for d in devices {
            // Menu labels treat "_" as a mnemonic.
            let label = trf("Send to %s", &[&handoff::name(d)]).replace('_', "__");
            menu.append_item(&gio::MenuItem::new(Some(&label), Some(&gio::Action::print_detailed_name("win.handoff.send-to", Some(&d.id.to_variant())))));
        }
        m.set_menu_model(Some(&menu));
    });
    let update = |m: &gtk::MenuButton| m.set_visible(!handoff::devices().with(Vec::is_empty) && player::track_changed().with(Option::is_some));
    on_state(handoff::devices(), &mb, move |m, _| update(m));
    on_state(player::track_changed(), &mb, move |m, _| update(m));
    mb
}
