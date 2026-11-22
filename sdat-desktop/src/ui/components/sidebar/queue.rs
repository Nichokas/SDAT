//! Port of `components/sidebar/queue`.
use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;

use crate::i18n::tr;
use crate::player::{self, QueueKind};
use crate::ui::components::{Col, TrackList};
use crate::ui::dsl::{W, vstack, on_state};

const SCROLL_MARGIN: f64 = 75.0;
const MAX_SCROLL_SPEED: f64 = 4000.0;

fn empty_page() -> adw::StatusPage {
    adw::StatusPage::builder().title(tr("No Tracks in Queue")).icon_name("music-queue-empty-symbolic").vexpand(true).build()
}

fn entries(kind: QueueKind) -> &'static crate::signals::State<Vec<crate::model::Track>> {
    match kind {
        QueueKind::User => player::user_queue().entries(),
        QueueKind::Base => player::base_queue().entries(),
    }
}

fn queue_tracklist(kind: QueueKind) -> TrackList {
    let list = TrackList::new(vec![
        Col::cover(),
        Col::title_album(),
        Col::custom_widget_button(move |_, pos, _| {
            let b = gtk::Button::from_icon_name("user-trash-symbolic").tooltip(&tr("Remove Track from Queue")).class("flat").mend(10);
            b.connect_clicked(move |_| {
                let r = match kind {
                    QueueKind::User => player::remove_from_user_queue(pos),
                    QueueKind::Base => player::base_queue().remove_at(pos),
                };
                if let Err(e) = r {
                    log::warn!("failed to remove track from queue: {e}");
                }
            });
            b.upcast()
        }),
    ]);
    list.set_click_handler(move |_, pos| player::skip_through_queue(kind, pos));
    list.bind_tracks(entries(kind));
    list.set_reorder_callback(Some(Box::new(move |src, dst, _| match kind {
        QueueKind::User => player::user_queue_moved(src, dst),
        QueueKind::Base => player::base_queue().move_entry(src, dst),
    })));
    list
}

fn queue_list() -> gtk::ScrolledWindow {
    let user = queue_tracklist(QueueKind::User);
    let base = queue_tracklist(QueueKind::Base);
    // The rest of the playing list that is not loaded yet shows as placeholder
    // rows; scrolling onto them loads the next page.
    base.on_placeholder_shown(|_| player::feed(true));
    let b = base.downgrade();
    let sub = player::list_unloaded().on(move |n| {
        let Some(b) = b.upgrade() else { return true };
        b.set_placeholders(*n);
        crate::signals::CONTINUE
    });
    base.widget().connect_destroy(move |_| player::list_unloaded().unsubscribe(sub));
    let user_w = user.widget().clone().background("alpha(var(--view-bg-color), 0.9)").corner_radius(10).mbottom(10);
    let content = vstack(&[&user_w, base.widget()]).valign(gtk::Align::Start);
    let sw = gtk::ScrolledWindow::new().hmargin(12).vexp(true);
    sw.set_child(Some(&content));
    // Auto-scroll on the display frame clock only while dragging near an edge.
    let speed = Rc::new(Cell::new(0.0));
    let tick = Rc::new(Cell::new(None));
    let motion = gtk::DropControllerMotion::new();
    let (speed_cb, tick_cb, sw_weak) = (Rc::clone(&speed), Rc::clone(&tick), sw.downgrade());
    motion.connect_motion(move |_, _, y| {
        let Some(sw) = sw_weak.upgrade() else { return };
        let height = f64::from(sw.height());
        let next_speed = if y < SCROLL_MARGIN {
            (SCROLL_MARGIN - y) / SCROLL_MARGIN * -MAX_SCROLL_SPEED
        } else if height - y < SCROLL_MARGIN {
            (SCROLL_MARGIN - (height - y)) / SCROLL_MARGIN * MAX_SCROLL_SPEED
        } else {
            0.0
        };
        speed_cb.set(next_speed);
        if next_speed != 0.0 {
            let id = tick_cb.take().unwrap_or_else(|| {
                let speed = Rc::clone(&speed_cb);
                let tick = Rc::clone(&tick_cb);
                let last_us = Cell::new(None);
                sw.add_tick_callback(move |sw, clock| {
                    let cur_speed = speed.get();
                    if cur_speed == 0.0 {
                        tick.set(None);
                        return glib::ControlFlow::Break;
                    }
                    let now = clock.frame_time();
                    let dt_secs = last_us
                        .replace(Some(now))
                        .map_or(1.0 / 60.0, |prev| (now.saturating_sub(prev) as f64 / 1_000_000.0).clamp(0.0, 0.05));
                    let adj = sw.vadjustment();
                    adj.set_value(adj.value() + cur_speed * dt_secs);
                    glib::ControlFlow::Continue
                })
            });
            tick_cb.set(Some(id));
        } else if let Some(id) = tick_cb.take() {
            id.remove();
        }
    });
    motion.connect_leave(move |_| {
        speed.set(0.0);
        if let Some(id) = tick.take() {
            id.remove();
        }
    });
    sw.add_controller(motion);
    // Keep the tracklists alive with the scrolled window.
    sw.connect_destroy(move |_| {
        let _keep_alive = (&user, &base);
    });
    sw
}

pub fn new_queue() -> gtk::Box {
    let mini = super::mini_player();
    on_state(player::track_changed(), &mini, |m, t| m.set_visible(t.is_some()));
    let bin = adw::Bin::new();
    let list: Rc<std::cell::RefCell<Option<gtk::ScrolledWindow>>> = Rc::default();
    let update = move |bin: &adw::Bin| {
        let filled = !player::user_queue().is_empty() || !player::base_queue().is_empty();
        if filled {
            let sw = list.borrow_mut().get_or_insert_with(queue_list).clone();
            if bin.child().as_ref() != Some(sw.upcast_ref()) {
                bin.set_child(Some(&sw));
            }
        } else {
            bin.set_child(Some(&empty_page()));
        }
    };
    let update = Rc::new(update);
    let u = Rc::clone(&update);
    on_state(player::user_queue().entries(), &bin, move |b, _| u(b));
    on_state(player::base_queue().entries(), &bin, move |b, _| update(b));
    let listen = super::live::new_button().hmargin(12).mtop(6);
    vstack(&[listen.upcast_ref(), mini.upcast_ref(), bin.upcast_ref()])
}
