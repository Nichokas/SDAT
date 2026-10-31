//! Port of `components/tracklist`: a ListView of tracks with configurable columns
//! and optional drag-and-drop reordering.
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::{gdk, gio, glib, graphene, pango};

use crate::i18n::{tr, trf};
use crate::model::{format_duration, Track};
use crate::player;
use crate::signals::State;
use crate::state;
use crate::ui::dsl::*;
use crate::{imgutil, settings};

type ColFn = dyn Fn(Option<&Track>, &gtk::Grid, usize, i32) -> i32;

/// A column renderer (port of ColumnFunc).
#[derive(Clone)]
pub struct Col(pub Rc<ColFn>);

impl Col {
    pub fn new(f: impl Fn(Option<&Track>, &gtk::Grid, usize, i32) -> i32 + 'static) -> Self {
        Col(Rc::new(f))
    }
    fn call(&self, t: Option<&Track>, g: &gtk::Grid, pos: usize, col: i32) -> i32 {
        (self.0)(t, g, pos, col)
    }

    pub fn artists() -> Self {
        Col::new(|t, g, _, c| {
            let l = label(&t.map(|t| t.artist_names().join(", ")).unwrap_or_default())
                .halign(gtk::Align::Start)
                .valign(gtk::Align::Center)
                .margin(10)
                .hexp(true);
            l.set_ellipsize(pango::EllipsizeMode::End);
            g.attach(&l, c, 0, 1, 1);
            1
        })
    }

    pub fn button() -> Self {
        Col::new(|t, g, _, c| {
            let b = gtk::Button::new().class("flat");
            b.set_action_name(Some("win.player.play-track"));
            b.set_action_target_value(Some(&t.map(|t| t.id()).unwrap_or_default().to_variant()));
            g.attach(&b, 0, 0, c, 1);
            0
        })
    }

    pub fn expand_button(extra: i32) -> Self {
        let inner = Col::button();
        Col::new(move |t, g, p, c| inner.call(t, g, p, c + extra))
    }

    pub fn controls() -> Self {
        Col::new(|t, g, _, c| {
            let Some(t) = t else {
                g.attach(&gtk::Box::new(gtk::Orientation::Horizontal, 0), c, 0, 1, 1);
                return 1;
            };
            let model = gio::Menu::new();
            let album_id = t.album.as_ref().map(|a| a.id.clone()).unwrap_or_default();
            let item = gio::MenuItem::new(Some(&tr("Navigate to Album")), Some("win.route.album"));
            item.set_action_and_target_value(Some("win.route.album"), Some(&album_id.to_variant()));
            model.append_item(&item);
            if t.artists.len() > 1 {
                let sub = gio::Menu::new();
                for a in &t.artists {
                    let item = gio::MenuItem::new(Some(&trf("Navigate to %s", &[&a.title()])), Some("win.route.artist"));
                    item.set_action_and_target_value(Some("win.route.artist"), Some(&a.id.to_variant()));
                    sub.append_item(&item);
                }
                model.append_submenu(Some(&tr("Navigate to Artist")), &sub);
            } else if let Some(a) = t.artists.first() {
                let item = gio::MenuItem::new(Some(&tr("Navigate to Artist")), Some("win.route.artist"));
                item.set_action_and_target_value(Some("win.route.artist"), Some(&a.id.to_variant()));
                model.append_item(&item);
            }
            let popover = gtk::PopoverMenu::from_model(Some(&model));
            let queue = gtk::Button::from_icon_name("queue-symbolic")
                .tooltip(&tr("Add to Queue"))
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .class("flat");
            queue.set_action_name(Some("win.player.queue-track"));
            queue.set_action_target_value(Some(&t.id().to_variant()));
            let more = gtk::MenuButton::new()
                .tooltip(&tr("More…"))
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .class("flat");
            more.set_icon_name("view-more-symbolic");
            more.set_popover(Some(&popover));
            let fav = super::favourite_button::favourite_button(&state::TRACKS, t.id())
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center);
            let row = hstack(&[fav.upcast_ref(), queue.upcast_ref(), more.upcast_ref()]).margin(10).halign(gtk::Align::End);
            g.attach(&row, c, 0, 1, 1);
            1
        })
    }

    pub fn cover() -> Self {
        Col::new(|t, g, _, c| {
            let img = gtk::Image::from_paintable(Some(&imgutil::missing_album()));
            img.set_pixel_size(54);
            let img = img.hexp(false).vexp(false).corner_radius(10).margin(10).halign(gtk::Align::Start).overflow_hidden();
            let cover = t.map(|t| t.cover(80));
            img.connect_realize(move |i| {
                if let Some(cover) = &cover
                    && settings::allow_tracklist_images()
                    && cover.has_static()
                {
                    imgutil::load_into_image(&cover.static_, i, false);
                }
            });
            g.attach(&img, c, 0, 1, 1);
            1
        })
    }

    /// Port of CustomButtonColumn.
    pub fn custom_button(on_click: impl Fn(&str, usize, i32) + 'static) -> Self {
        let on_click = Rc::new(on_click);
        Col::new(move |t, g, p, c| {
            let Some(t) = t else {
                g.attach(&gtk::Box::new(gtk::Orientation::Horizontal, 0), 0, 0, c, 1);
                return 1;
            };
            let b = gtk::Button::new().class("flat");
            let (id, f) = (t.id().to_string(), on_click.clone());
            b.connect_clicked(move |_| f(&id, p, c));
            g.attach(&b, 0, 0, c, 1);
            0
        })
    }

    pub fn expand_custom_button(extra: i32, on_click: impl Fn(&str, usize, i32) + 'static) -> Self {
        let inner = Col::custom_button(on_click);
        Col::new(move |t, g, p, c| inner.call(t, g, p, c + extra))
    }

    /// Port of CustomWidgetButtonColumn.
    pub fn custom_widget_button(button: impl Fn(&str, usize, i32) -> gtk::Widget + 'static) -> Self {
        Col::new(move |t, g, p, c| {
            let Some(t) = t else {
                g.attach(&gtk::Box::new(gtk::Orientation::Horizontal, 0), c, 0, 1, 1);
                return 1;
            };
            let w = button(t.id(), p, c);
            g.attach(&hstack(&[&w]).halign(gtk::Align::Center).valign(gtk::Align::Center), c, 0, 1, 1);
            0
        })
    }

    pub fn duration() -> Self {
        Col::new(|t, g, _, c| {
            g.attach(&label(&format_duration(t.map(|t| t.duration()).unwrap_or_default())).margin(10), c, 0, 1, 1);
            1
        })
    }

    pub fn grouped(width: i32, align: gtk::Align, cols: Vec<Col>) -> Self {
        Col::new(move |t, g, p, c| {
            let sub = gtk::Grid::new();
            sub.set_valign(gtk::Align::Center);
            sub.set_halign(align);
            for (i, col) in cols.iter().enumerate() {
                col.call(t, &sub, p, c + i as i32);
            }
            g.attach(&sub, c, 0, width, 1);
            width
        })
    }

    pub fn position() -> Self {
        Col::new(|_, g, p, c| {
            let l = label(&(p + 1).to_string()).font_weight(500).halign(gtk::Align::Start).hexp(false).margin(10).hpadding(10);
            g.attach(&l, c, 0, 1, 1);
            1
        })
    }

    pub fn title() -> Self {
        Col::new(|t, g, _, c| {
            let l = label(&t.map(|t| t.formatted_title()).unwrap_or_default())
                .font_weight(500)
                .halign(gtk::Align::Start)
                .hexp(true)
                .margin(10);
            l.set_ellipsize(pango::EllipsizeMode::End);
            g.attach(&l, c, 0, 1, 1);
            1
        })
    }

    pub fn title_album() -> Self {
        Col::new(|t, g, _, c| {
            let title = label(&t.map(|t| t.formatted_title()).unwrap_or_default()).font_weight(500).halign(gtk::Align::Start);
            title.set_ellipsize(pango::EllipsizeMode::End);
            let album = label(&t.and_then(|t| t.album.as_ref()).map(|a| a.title.clone()).unwrap_or_default())
                .class("dimmed")
                .halign(gtk::Align::Start);
            album.set_ellipsize(pango::EllipsizeMode::End);
            let frame = spacing(vstack(&[title.upcast_ref(), album.upcast_ref()]), 3)
                .valign(gtk::Align::Center)
                .halign(gtk::Align::Start)
                .hexp(true)
                .margin(10);
            g.attach(&hstack(&[frame.upcast_ref(), spacer().upcast_ref()]), c, 0, 1, 1);
            1
        })
    }
}

type ClickFn = Rc<dyn Fn(&Track, usize)>;
type ReorderFn = Rc<dyn Fn(usize, usize, &Track)>;

struct Inner {
    columns: Vec<Col>,
    size_groups: Vec<gtk::SizeGroup>,
    store: gio::ListStore,
    tracks: RefCell<Vec<Track>>,
    click: RefCell<ClickFn>,
    reorder: RefCell<Option<ReorderFn>>,
    reorderable: State<bool>,
    moving_source: Cell<i64>,
    moving_target: Cell<i64>,
    being_moved: RefCell<Option<Track>>,
    view: gtk::ListView,
    drop_target: RefCell<Option<gtk::DropTargetAsync>>,
}

/// Port of TrackList. Cheap to clone.
#[derive(Clone)]
pub struct TrackList(Rc<Inner>);

fn empty() -> glib::Object {
    gtk::StringObject::new("").upcast()
}

impl TrackList {
    pub fn new(columns: Vec<Col>) -> Self {
        let store = gio::ListStore::new::<gtk::StringObject>();
        let factory = gtk::SignalListItemFactory::new();
        let view = gtk::ListView::new(Some(gtk::NoSelection::new(Some(store.clone()))), Some(factory.clone()));
        view.set_single_click_activate(true);
        view.set_orientation(gtk::Orientation::Vertical);
        view.add_css_class("tracklist");
        let size_groups = columns.iter().map(|_| gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal)).collect();
        let inner = Rc::new(Inner {
            columns,
            size_groups,
            store,
            tracks: RefCell::new(Vec::new()),
            click: RefCell::new(Rc::new(|t: &Track, _| player::play_track(t.clone()))),
            reorder: RefCell::new(None),
            reorderable: State::new(false),
            moving_source: Cell::new(-1),
            moving_target: Cell::new(-1),
            being_moved: RefCell::new(None),
            view: view.clone(),
            drop_target: RefCell::new(None),
        });
        let w = Rc::downgrade(&inner);
        factory.connect_setup(move |_, item| {
            if let Some(i) = w.upgrade() {
                setup(&i, &w, item.downcast_ref().unwrap());
            }
        });
        let w = Rc::downgrade(&inner);
        factory.connect_bind(move |_, item| {
            if let Some(i) = w.upgrade() {
                bind(&i, item.downcast_ref().unwrap());
            }
        });
        factory.connect_unbind(|_, item| {
            let item: &gtk::ListItem = item.downcast_ref().unwrap();
            if let Some(bin) = item.child().and_downcast::<adw::Bin>() {
                bin.set_child(gtk::Widget::NONE);
            }
        });
        let w = Rc::downgrade(&inner);
        view.connect_activate(move |_, idx| {
            if let Some(i) = w.upgrade() {
                let t = i.tracks.borrow().get(idx as usize).cloned();
                let cb = i.click.borrow().clone();
                if let Some(t) = t {
                    cb(&t, idx as usize);
                }
            }
        });
        let w = Rc::downgrade(&inner);
        inner.reorderable.on(move |on| {
            let Some(i) = w.upgrade() else { return true };
            toggle_drop_target(&i, &w, *on);
            false
        });
        TrackList(inner)
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.0.view.upcast_ref()
    }

    pub fn list_view(&self) -> &gtk::ListView {
        &self.0.view
    }

    pub fn add_track(&self, t: Track) {
        self.0.tracks.borrow_mut().push(t);
        self.0.store.append(&empty());
    }

    /// Keeps the list in sync with `state` for the lifetime of the widget.
    pub fn bind_tracks(&self, st: &'static State<Vec<Track>>) {
        let w = Rc::downgrade(&self.0);
        let alive = Rc::new(Cell::new(true));
        let a = alive.clone();
        self.0.view.connect_destroy(move |_| a.set(false));
        st.on(move |new| {
            let Some(i) = w.upgrade().filter(|_| alive.get()) else { return true };
            sync(&i, new);
            false
        });
    }

    /// Sets the tracks directly (diffing like BindTracks).
    pub fn set_tracks(&self, new: &[Track]) {
        sync(&self.0, new);
    }

    pub fn clear(&self) {
        self.0.store.remove_all();
        self.0.tracks.borrow_mut().clear();
    }

    pub fn set_click_handler(&self, f: impl Fn(&Track, usize) + 'static) {
        *self.0.click.borrow_mut() = Rc::new(f);
    }

    pub fn set_reorder_callback(&self, f: Option<Box<dyn Fn(usize, usize, &Track)>>) {
        let on = f.is_some();
        *self.0.reorder.borrow_mut() = f.map(|f| Rc::from(f) as ReorderFn);
        self.0.reorderable.set(on);
    }
}

fn sync(i: &Inner, new: &[Track]) {
    let mut tracks = i.tracks.borrow_mut();
    for (idx, t) in new.iter().enumerate() {
        if idx >= tracks.len() {
            tracks.push(t.clone());
            i.store.append(&empty());
        } else if tracks[idx].id() != t.id() {
            tracks[idx] = t.clone();
            i.store.splice(idx as u32, 1, &[empty()]);
        }
    }
    while tracks.len() > new.len() {
        let last = tracks.len() - 1;
        tracks.pop();
        i.store.remove(last as u32);
    }
}

fn bind(i: &Inner, item: &gtk::ListItem) {
    let pos = item.position() as usize;
    let Some(track) = i.tracks.borrow().get(pos).cloned() else { return };
    let Some(container) = item.child().and_downcast::<adw::Bin>() else { return };
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let id = track.id().to_string();
    body.connect_realize(move |b| {
        let id = id.clone();
        on_state(player::track_changed(), b, move |b, ev| {
            if ev.as_ref().is_some_and(|e| e.track.id() == id) {
                b.add_css_class("playing");
            } else {
                b.remove_css_class("playing");
            }
        });
    });
    for (idx, col) in i.columns.iter().enumerate() {
        let grid = gtk::Grid::new();
        grid.set_valign(gtk::Align::Center);
        col.call(Some(&track), &grid, pos, 0);
        i.size_groups[idx].add_widget(&grid);
        body.append(&grid);
    }
    let streamable = track.info.streamable;
    item.set_activatable(streamable);
    container.set_sensitive(streamable);
    if streamable {
        container.set_tooltip_text(Some(""));
    } else {
        container.set_tooltip_text(Some(&tr("This track is not available for streaming")));
    }
    container.set_child(Some(&body));
}

const DRAG_MIME: &str = "application/x-tonearm-track";

fn setup(i: &Inner, w: &Weak<Inner>, item: &gtk::ListItem) {
    let container = adw::Bin::new();
    let source: Rc<RefCell<Option<gtk::DragSource>>> = Rc::default();
    let (w, cw, li) = (w.clone(), container.downgrade(), item.downgrade());
    i.reorderable.on(move |on| {
        let (Some(inner), Some(container), Some(item)) = (w.upgrade(), cw.upgrade(), li.upgrade()) else { return true };
        let mut src = source.borrow_mut();
        if *on && src.is_none() {
            let ds = gtk::DragSource::new();
            ds.set_actions(gdk::DragAction::MOVE);
            let (w1, li1) = (Rc::downgrade(&inner), item.downgrade());
            ds.connect_prepare(move |_, _, _| {
                let (i, item) = (w1.upgrade()?, li1.upgrade()?);
                let p = item.position() as i64;
                i.moving_source.set(p);
                i.moving_target.set(p);
                *i.being_moved.borrow_mut() = i.tracks.borrow().get(p as usize).cloned();
                // The token identifies this list; drops from other lists are rejected.
                Some(gdk::ContentProvider::for_bytes(DRAG_MIME, &glib::Bytes::from_static(b"track")))
            });
            let cw1 = container.downgrade();
            ds.connect_drag_begin(move |ds, _| {
                let Some(c) = cw1.upgrade() else { return };
                let Some(parent) = c.parent() else { return };
                let snap = gtk::Snapshot::new();
                parent.snapshot_child(&c, &snap);
                let size = graphene::Size::new(c.width() as f32, c.height() as f32);
                if let Some(p) = snap.to_paintable(Some(&size)) {
                    ds.set_icon(Some(&p), p.intrinsic_width() / 2, p.intrinsic_height() / 2);
                }
            });
            let w2 = Rc::downgrade(&inner);
            ds.connect_drag_cancel(move |_, _, _| {
                let Some(i) = w2.upgrade() else { return true };
                if i.being_moved.borrow().is_none() {
                    return true;
                }
                let (s, t) = (i.moving_source.get(), i.moving_target.get());
                if s >= 0 && t >= 0 {
                    let mut tracks = i.tracks.borrow_mut();
                    if (t as usize) < tracks.len() {
                        tracks.remove(t as usize);
                        i.store.remove(t as u32);
                    }
                    if let Some(tr) = i.being_moved.borrow_mut().take() {
                        let s = (s as usize).min(tracks.len());
                        tracks.insert(s, tr);
                        i.store.insert(s as u32, &empty());
                    }
                }
                i.moving_source.set(-1);
                i.moving_target.set(-1);
                true
            });
            container.add_controller(ds.clone());
            *src = Some(ds);
        } else if !*on && let Some(ds) = src.take() {
            container.remove_controller(&ds);
        }
        false
    });
    item.set_child(Some(&container));
}

fn toggle_drop_target(i: &Inner, w: &Weak<Inner>, on: bool) {
    let mut dt = i.drop_target.borrow_mut();
    if on && dt.is_none() {
        let target = gtk::DropTargetAsync::new(Some(gdk::ContentFormats::new(&[DRAG_MIME])), gdk::DragAction::MOVE);
        let w0 = w.clone();
        // Only this list's own drags carry a track being moved.
        target.connect_accept(move |_, _| w0.upgrade().is_some_and(|i| i.being_moved.borrow().is_some()));
        let w1 = w.clone();
        target.connect_drag_motion(move |_, _, _, y| {
            let Some(i) = w1.upgrade() else { return gdk::DragAction::empty() };
            let Some(tr) = i.being_moved.borrow().clone() else { return gdk::DragAction::empty() };
            let h = i.view.first_child().map(|c| c.height()).unwrap_or(1).max(1);
            let idx = (y / h as f64).floor().max(0.0) as i64;
            if idx != i.moving_target.get() {
                let mut tracks = i.tracks.borrow_mut();
                if idx as usize > tracks.len() {
                    return gdk::DragAction::empty();
                }
                let cur = i.moving_target.get();
                if cur >= 0 && (cur as usize) < tracks.len() {
                    i.store.remove(cur as u32);
                    tracks.remove(cur as usize);
                }
                let idx = (idx as usize).min(tracks.len());
                tracks.insert(idx, tr);
                i.store.insert(idx as u32, &empty());
                i.moving_target.set(idx as i64);
            }
            gdk::DragAction::MOVE
        });
        let w2 = w.clone();
        target.connect_drop(move |_, drop, _, _| {
            drop.finish(gdk::DragAction::MOVE);
            let Some(i) = w2.upgrade() else { return false };
            let (s, t) = (i.moving_source.get(), i.moving_target.get());
            let tr = i.being_moved.borrow_mut().take();
            i.moving_source.set(-1);
            i.moving_target.set(-1);
            let cb = i.reorder.borrow().clone();
            if let (Some(cb), Some(tr)) = (cb, tr)
                && s >= 0
                && t >= 0
            {
                cb(s as usize, t as usize, &tr);
            }
            true
        });
        i.view.add_controller(target.clone());
        *dt = Some(target);
    } else if !on && let Some(t) = dt.take() {
        i.view.remove_controller(&t);
    }
}
