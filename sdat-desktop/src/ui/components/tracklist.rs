//! Port of `components/tracklist`: a `ListView` of tracks with configurable columns
//! and optional drag-and-drop reordering.
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::{gdk, gio, graphene, pango};

use crate::i18n::{tr, trf};
use crate::model::{format_duration, Track};
use crate::player;
use crate::signals::State;
use crate::ui::dsl::{W, label, hstack, spacing, vstack, spacer, on_state};
use crate::{imgutil, settings};

type BindFn = Box<dyn Fn(&Track, usize)>;
type RepositionFn = Box<dyn Fn(usize)>;
type LivePos = Rc<Cell<Option<usize>>>;

/// One recycled cell: built once per row widget, then re-bound as the row scrolls.
pub struct ColCell {
    pub widget: gtk::Widget,
    bind: BindFn,
    reposition: RepositionFn,
    unbind: Box<dyn Fn()>,
    /// Shares the row's flexible space equally with the other flexible columns.
    flex: bool,
}

impl ColCell {
    fn new(widget: impl IsA<gtk::Widget>, bind: impl Fn(&Track, usize) + 'static) -> Self {
        ColCell {
            widget: widget.upcast(),
            bind: Box::new(bind),
            reposition: Box::new(|_| {}),
            unbind: Box::new(|| {}),
            flex: false,
        }
    }
    fn flexible(mut self) -> Self {
        self.flex = true;
        self
    }
    fn with_reposition(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.reposition = Box::new(f);
        self
    }
    fn with_unbind(mut self, f: impl Fn() + 'static) -> Self {
        self.unbind = Box::new(f);
        self
    }
}

type ColFn = dyn Fn() -> ColCell;

/// A column (port of `ColumnFunc`). Rows are recycled by the `ListView`, so a column
/// creates its widgets once and only updates them when a row is bound to a new track.
#[derive(Clone)]
pub struct Col(pub Rc<ColFn>);

fn ellipsized(l: &gtk::Label) {
    l.set_ellipsize(pango::EllipsizeMode::End);
    l.set_xalign(0.0);
}

impl Col {
    pub fn new(f: impl Fn() -> ColCell + 'static) -> Self {
        Col(Rc::new(f))
    }
    fn build(&self) -> ColCell {
        (self.0)()
    }

    pub fn artists() -> Self {
        Col::new(|| {
            let l = label("").halign(gtk::Align::Start).valign(gtk::Align::Center).margin(10).hexp(true);
            ellipsized(&l);
            let l2 = l.clone();
            ColCell::new(l, move |t, _| l2.set_label(&t.artist_names().join(", "))).flexible()
        })
    }

    pub fn controls() -> Self {
        Col::new(|| {
            let queue = gtk::Button::from_icon_name("queue-symbolic")
                .tooltip(&tr("Add to Queue"))
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .class("flat");
            queue.set_action_name(Some("win.player.queue-track"));
            let more = gtk::MenuButton::new()
                .tooltip(&tr("More…"))
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .class("flat");
            more.set_icon_name("view-more-symbolic");
            let current: Rc<RefCell<Option<Track>>> = Rc::default();
            // Build the menu only when it is opened, for the track bound right now.
            let cur = Rc::clone(&current);
            more.set_create_popup_func(move |mb| {
                let Some(track) = cur.borrow().clone() else { return };
                let popover = track_menu_popover(&track);
                // Hang below the button, right-aligned to it, so the menu and its submenus
                // grow only leftwards/downwards and stay inside the window (GTK flips up if needed).
                popover.set_position(gtk::PositionType::Bottom);
                popover.set_halign(gtk::Align::End);
                mb.set_popover(Some(&popover));
            });
            let (fav, set_fav) = super::favourite_button::track_favourite_button();
            let fav = fav.halign(gtk::Align::Center).valign(gtk::Align::Center);
            let row = hstack(&[fav.upcast_ref(), queue.upcast_ref(), more.upcast_ref()]).margin(10).halign(gtk::Align::End);
            let (q, m, cur, sf) = (queue, more, Rc::clone(&current), Rc::clone(&set_fav));
            ColCell::new(row, move |t, _| {
                q.set_action_target_value(Some(&t.id().to_variant()));
                if cur.borrow().as_ref().map(Track::id) != Some(t.id()) {
                    m.popdown();
                    m.set_popover(gtk::Popover::NONE);
                }
                *cur.borrow_mut() = Some(t.clone());
                sf(Some(t.id()));
            })
            .with_unbind(move || {
                set_fav(None);
                *current.borrow_mut() = None;
            })
        })
    }

    pub fn cover() -> Self {
        Col::new(|| {
            let img = gtk::Image::from_paintable(Some(&imgutil::missing_album()));
            img.set_pixel_size(54);
            let img = img.hexp(false).vexp(false).corner_radius(10).margin(10).halign(gtk::Align::Start).overflow_hidden();
            // Bumped on every (un)bind so a slow load can't land on a recycled row.
            let token = Rc::new(Cell::new(0u64));
            let (i, tok) = (img.clone(), Rc::clone(&token));
            ColCell::new(img, move |t, _| {
                tok.set(tok.get() + 1);
                i.set_paintable(Some(&imgutil::missing_album()));
                let cover = t.cover(80);
                if !(settings::allow_tracklist_images() && cover.has_static()) {
                    return;
                }
                let (w, tok, mine) = (i.downgrade(), Rc::clone(&tok), tok.get());
                let url = cover.static_;
                glib::spawn_future_local(async move {
                    if tok.get() != mine {
                        return;
                    }
                    let tex = imgutil::fetch(&url).await;
                    if let (Some(tex), Some(i)) = (tex, w.upgrade())
                        && tok.get() == mine
                    {
                        i.set_paintable(Some(&tex));
                    }
                });
            })
            .with_unbind(move || token.set(token.get() + 1))
        })
    }

    /// Port of `CustomWidgetButtonColumn` (rebuilt per bind; used by the short queue lists).
    pub fn custom_widget_button(button: impl Fn(&str, LivePos) -> gtk::Widget + 'static) -> Self {
        let button = Rc::new(button);
        Col::new(move || {
            let bin = adw::Bin::new().halign(gtk::Align::Center).valign(gtk::Align::Center);
            let current_pos: Rc<RefCell<Option<LivePos>>> = Rc::default();
            let (b, button, pos_bind) = (bin.clone(), Rc::clone(&button), Rc::clone(&current_pos));
            let pos_reposition = Rc::clone(&current_pos);
            let (b2, pos_unbind) = (bin.clone(), current_pos);
            ColCell::new(bin, move |t, p| {
                let live_pos = Rc::new(Cell::new(Some(p)));
                if let Some(prev) = pos_bind.borrow_mut().replace(Rc::clone(&live_pos)) {
                    prev.set(None);
                }
                b.set_child(Some(&hstack(&[&button(t.id(), live_pos)])));
            })
            .with_reposition(move |p| {
                if let Some(live_pos) = pos_reposition.borrow().as_ref() {
                    live_pos.set(Some(p));
                }
            })
            .with_unbind(move || {
                if let Some(prev) = pos_unbind.borrow_mut().take() {
                    prev.set(None);
                }
                b2.set_child(gtk::Widget::NONE);
            })
        })
    }

    pub fn duration() -> Self {
        Col::new(|| {
            // A fixed minimum width keeps the column aligned across rows without a
            // SizeGroup (size groups made every scroll step re-measure all rows).
            let l = label("").margin(10);
            l.set_width_chars(5);
            l.set_xalign(1.0);
            let l2 = l.clone();
            ColCell::new(l, move |t, _| l2.set_label(&format_duration(t.duration())))
        })
    }

    pub fn position() -> Self {
        Col::new(|| {
            let l = label("").font_weight(500).halign(gtk::Align::Start).hexp(false).margin(10).hpadding(10);
            l.set_width_chars(4);
            l.set_xalign(0.0);
            let l2 = l.clone();
            let l3 = l.clone();
            ColCell::new(l, move |_, p| l2.set_label(&(p + 1).to_string()))
                .with_reposition(move |p| l3.set_label(&(p + 1).to_string()))
        })
    }

    pub fn title() -> Self {
        Col::new(|| {
            let l = label("").font_weight(500).halign(gtk::Align::Start).hexp(true).margin(10);
            ellipsized(&l);
            let l2 = l.clone();
            ColCell::new(l, move |t, _| l2.set_label(&t.formatted_title())).flexible()
        })
    }

    pub fn title_album() -> Self {
        Col::new(|| {
            let title = label("").font_weight(500).halign(gtk::Align::Start);
            ellipsized(&title);
            let album = label("").class("dimmed").halign(gtk::Align::Start);
            ellipsized(&album);
            let frame = spacing(vstack(&[title.upcast_ref(), album.upcast_ref()]), 3)
                .valign(gtk::Align::Center)
                .halign(gtk::Align::Start)
                .hexp(true)
                .margin(10);
            ColCell::new(hstack(&[frame.upcast_ref(), spacer().upcast_ref()]), move |t, _| {
                title.set_label(&t.formatted_title());
                album.set_label(t.album.as_ref().map(|a| a.title.as_str()).unwrap_or_default());
            })
            .flexible()
        })
    }
}

type ClickFn = Rc<dyn Fn(&Track, usize)>;
type PlaceholderFn = Rc<dyn Fn(usize)>;
type ReorderFn = Rc<dyn Fn(usize, usize, &Track)>;

struct DragState {
    source: usize,
    target: usize,
    track: Track,
    over_valid_row: bool,
}

struct Inner {
    columns: Vec<Col>,
    store: gio::ListStore,
    tracks: RefCell<Vec<Track>>,
    click: RefCell<ClickFn>,
    reorder: RefCell<Option<ReorderFn>>,
    reorderable: State<bool>,
    drag: RefCell<Option<DragState>>,
    view: glib::WeakRef<gtk::ListView>,
    drop_target: RefCell<Option<gtk::DropTargetAsync>>,
    /// Rows after the tracks that stand for tracks not loaded yet.
    placeholders: Cell<usize>,
    on_placeholder: RefCell<Option<PlaceholderFn>>,
}

/// The recycled widgets of one list item.
struct Row {
    body: gtk::Box,
    cells: Vec<ColCell>,
    track: Rc<RefCell<Option<Track>>>,
}


pub type BoxedReorderFn = Box<dyn Fn(usize, usize, &Track)>;

/// Port of `TrackList`. Cheap to clone.
///
/// The list view owns its state (`Inner`) through a destroy handler, so the
/// rows keep working after every `TrackList` handle is dropped (Go relies on
/// the GC for this). `Inner` only holds a weak ref back to the view, so there
/// is no reference cycle.
#[derive(Clone)]
pub struct TrackList(Rc<Inner>, gtk::ListView);

/// Weak handle to a [`TrackList`]; does not keep a closed page alive.
#[derive(Clone)]
pub struct WeakTrackList(Weak<Inner>, glib::WeakRef<gtk::ListView>);

impl WeakTrackList {
    pub fn upgrade(&self) -> Option<TrackList> {
        Some(TrackList(self.0.upgrade()?, self.1.upgrade()?))
    }
}

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
        let inner = Rc::new(Inner {
            columns,
            store,
            tracks: RefCell::new(Vec::new()),
            click: RefCell::new(Rc::new(|t: &Track, _| player::play_track(t.clone()))),
            reorder: RefCell::new(None),
            reorderable: State::new(false),
            drag: RefCell::new(None),
            view: view.downgrade(),
            drop_target: RefCell::new(None),
            placeholders: Cell::new(0),
            on_placeholder: RefCell::new(None),
        });
        let w = Rc::downgrade(&inner);
        factory.connect_setup(move |_, item| {
            if let Some(i) = w.upgrade() {
                setup(&i, &w, item.downcast_ref().expect("ListView factories hand out ListItems"));
            }
        });
        let w = Rc::downgrade(&inner);
        factory.connect_bind(move |_, item| {
            if let Some(i) = w.upgrade() {
                bind(&i, item.downcast_ref().expect("ListView factories hand out ListItems"));
            }
        });
        factory.connect_unbind(|_, item| {
            let item: &gtk::ListItem = item.downcast_ref().expect("ListView factories hand out ListItems");
            if let Some(row) = item.child().and_downcast::<adw::Bin>().as_ref().and_then(row_of) {
                *row.track.borrow_mut() = None;
                for c in &row.cells {
                    (c.unbind)();
                }
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
        let keep = Rc::clone(&inner);
        view.connect_destroy(move |_| {
            let _ = &keep;
        });
        TrackList(inner, view)
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.1.upcast_ref()
    }


    /// Shows `n` placeholder rows after the tracks (same row widgets and
    /// height, dimmed, inactive) for tracks that are not loaded yet.
    pub fn set_placeholders(&self, n: usize) {
        let i = &self.0;
        let (cur, len) = (i.placeholders.get(), i.tracks.borrow().len());
        i.placeholders.set(n);
        let at = (len + n.min(cur)) as u32;
        if n > cur {
            let extra: Vec<glib::Object> = (0..n - cur).map(|_| empty()).collect();
            i.store.splice(at, 0, &extra);
        } else if n < cur {
            i.store.splice(at, (cur - n) as u32, &[] as &[glib::Object]);
        }
    }

    /// Called (from idle) with the index of a placeholder row that got shown.
    pub fn on_placeholder_shown(&self, f: impl Fn(usize) + 'static) {
        *self.0.on_placeholder.borrow_mut() = Some(Rc::new(f));
    }

    pub fn add_track(&self, t: Track) {
        let i = &self.0;
        cancel_drag(i);
        i.tracks.borrow_mut().push(t);
        i.store.append(&empty());
    }

    /// Keeps the list in sync with `state` for the lifetime of the widget.
    pub fn bind_tracks(&self, st: &'static State<Vec<Track>>) {
        let w = Rc::downgrade(&self.0);
        let alive = Rc::new(Cell::new(true));
        let a = Rc::clone(&alive);
        self.1.connect_destroy(move |_| a.set(false));
        st.on(move |new| {
            let Some(i) = w.upgrade().filter(|_| alive.get()) else { return true };
            sync(&i, new);
            false
        });
    }



    /// Replaces the rows with `tracks` (minimal diff).
    pub fn replace_all(&self, tracks: &[Track]) {
        sync(&self.0, tracks);
    }

    pub fn set_click_handler(&self, f: impl Fn(&Track, usize) + 'static) {
        *self.0.click.borrow_mut() = Rc::new(f);
    }

    /// Like [`Self::set_click_handler`] but also hands out the list (held weakly,
    /// so the handler does not keep the list alive).
    pub fn set_list_click_handler(&self, f: impl Fn(&Self, &Track, usize) + 'static) {
        let (wi, wv) = (Rc::downgrade(&self.0), self.1.downgrade());
        self.set_click_handler(move |t, pos| {
            if let (Some(i), Some(v)) = (wi.upgrade(), wv.upgrade()) {
                f(&Self(i, v), t, pos);
            }
        });
    }

    /// Clicking a row plays the whole list from that row, "playing from" `source`.
    pub fn play_all_on_click(&self, source: impl Fn() -> crate::model::PlaybackSource + 'static) {
        self.set_list_click_handler(move |l, t, pos| {
            player::play_list_from(source(), l.tracks(), t, pos);
        });
    }

    pub fn downgrade(&self) -> WeakTrackList {
        WeakTrackList(Rc::downgrade(&self.0), self.1.downgrade())
    }

    pub fn tracks(&self) -> Vec<Track> {
        self.0.tracks.borrow().clone()
    }

    pub fn len(&self) -> usize {
        self.0.tracks.borrow().len()
    }

    pub fn set_reorder_callback(&self, f: Option<BoxedReorderFn>) {
        let on = f.is_some();
        *self.0.reorder.borrow_mut() = f.map(|f| Rc::from(f) as ReorderFn);
        self.0.reorderable.set(on);
    }
}

fn clear_drag(i: &Inner) {
    i.drag.borrow_mut().take();
}

fn start_drag(i: &Inner, pos: usize) -> bool {
    let Some(track) = i.tracks.borrow().get(pos).cloned() else {
        return false;
    };
    *i.drag.borrow_mut() = Some(DragState { source: pos, target: pos, track, over_valid_row: false });
    true
}

fn row_index_at(i: &Inner, y: f64) -> Option<usize> {
    if !y.is_finite() || y < 0.0 {
        return None;
    }
    let view = i.view.upgrade()?;
    let h = std::iter::successors(view.first_child(), gtk::Widget::next_sibling)
        .map(|c| c.height())
        .find(|&h| h > 0)
        .unwrap_or(1);
    let idx = (y / f64::from(h)).floor() as usize;
    (idx < i.tracks.borrow().len()).then_some(idx)
}

fn leave_drag(i: &Inner) {
    if let Some(drag) = i.drag.borrow_mut().as_mut() {
        drag.over_valid_row = false;
    }
}

/// Moves the row at `from` to `to` in both `tracks` and `store`.
/// Never inserts without removing the expected row first, and aborts if a
/// re-entrant list mutation occurs while the store removes `from`.
fn move_row(i: &Inner, from: usize, to: usize, expected_id: &str) -> Option<usize> {
    let dst = {
        let mut tracks = i.tracks.borrow_mut();
        // Both ends are track rows, so the splice below never touches the placeholders after them.
        if i.store.n_items() as usize != tracks.len() + i.placeholders.get()
            || to >= tracks.len()
            || tracks.get(from).is_none_or(|t| t.id() != expected_id)
        {
            return None;
        }
        let t = tracks.remove(from);
        let dst = to;
        tracks.insert(dst, t);
        dst
    };
    // `tracks` is already final, so the one items-changed below (which re-enters bind())
    // never sees a row missing from it.
    let (lo, hi) = (from.min(dst), from.max(dst));
    let rows: Vec<glib::Object> = (lo..=hi).map(|_| empty()).collect();
    i.store.splice(lo as u32, rows.len() as u32, &rows);
    Some(dst)
}

fn move_drag(i: &Inner, dst: usize) -> bool {
    let Some((cur, expected_id)) = i.drag.borrow().as_ref().map(|d| (d.target, d.track.id().to_owned())) else {
        return false;
    };
    if dst >= i.tracks.borrow().len() {
        leave_drag(i);
        return false;
    }
    let new_pos = if dst == cur {
        let tracks = i.tracks.borrow();
        let valid = i.store.n_items() as usize == tracks.len() + i.placeholders.get()
            && tracks.get(cur).is_some_and(|t| t.id() == expected_id);
        valid.then_some(cur)
    } else {
        move_row(i, cur, dst, &expected_id)
    };
    let Some(new_pos) = new_pos else {
        clear_drag(i);
        return false;
    };
    if let Some(drag) = i.drag.borrow_mut().as_mut() {
        drag.target = new_pos;
        drag.over_valid_row = true;
        true
    } else {
        false
    }
}

fn drag_motion(i: &Inner, y: f64) -> gdk::DragAction {
    if let Some(idx) = row_index_at(i, y)
        && move_drag(i, idx)
    {
        return gdk::DragAction::MOVE;
    }
    leave_drag(i);
    gdk::DragAction::empty()
}

fn cancel_drag(i: &Inner) {
    let Some(drag) = i.drag.borrow_mut().take() else {
        return;
    };
    if drag.source != drag.target {
        let _ = move_row(i, drag.target, drag.source, drag.track.id());
    }
}

fn drop_drag(i: &Inner, y: f64, on_finish: impl FnOnce(gdk::DragAction)) -> bool {
    let accepted = i.drag.borrow().as_ref().is_some_and(|d| d.over_valid_row)
        && row_index_at(i, y).is_some_and(|dst| move_drag(i, dst));
    if !accepted {
        cancel_drag(i);
        on_finish(gdk::DragAction::empty());
        return false;
    }
    let Some(drag) = i.drag.borrow_mut().take() else {
        on_finish(gdk::DragAction::empty());
        return false;
    };
    on_finish(gdk::DragAction::MOVE);
    if drag.source != drag.target
        && let Some(cb) = i.reorder.borrow().clone()
    {
        cb(drag.source, drag.target, &drag.track);
    }
    true
}

/// Diffs `new` into the store. The `RefCell` is never borrowed while the store
/// is mutated: splicing emits items-changed, which synchronously runs `bind()`
/// (and other handlers) that borrow `tracks` again.
fn sync(i: &Inner, new: &[Track]) {
    cancel_drag(i);
    let old = std::mem::replace(&mut *i.tracks.borrow_mut(), new.to_vec());
    for (idx, op) in diff_ops(&old, new) {
        match op {
            // A loaded track takes the place of the first placeholder row.
            Op::Append if i.placeholders.get() > 0 => {
                i.placeholders.set(i.placeholders.get() - 1);
                i.store.splice(idx as u32, 1, &[empty()]);
            }
            Op::Append => i.store.append(&empty()),
            Op::Replace => i.store.splice(idx as u32, 1, &[empty()]),
            Op::Remove => i.store.remove(idx as u32),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Op {
    Append,
    Replace,
    Remove,
}

fn diff_ops(old: &[Track], new: &[Track]) -> Vec<(usize, Op)> {
    let mut ops = Vec::new();
    for (idx, t) in new.iter().enumerate() {
        match old.get(idx) {
            None => ops.push((idx, Op::Append)),
            Some(o) if o.id() != t.id() => ops.push((idx, Op::Replace)),
            _ => {}
        }
    }
    for idx in (new.len()..old.len()).rev() {
        ops.push((idx, Op::Remove));
    }
    ops
}

fn row_of(bin: &adw::Bin) -> Option<Rc<Row>> {
    // SAFETY: "tonearm-row" is only ever set by `build_row` with an `Rc<Row>`.
    unsafe { bin.data::<Rc<Row>>("tonearm-row").map(|p| Rc::clone(p.as_ref())) }
}

fn is_playing(id: &str) -> bool {
    player::track_changed().with(|e| e.as_ref().is_some_and(|e| e.track.id() == id))
}

/// Creates the row widgets once per list item; `bind` only updates them.
fn build_row(i: &Inner, container: &adw::Bin) {
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let track: Rc<RefCell<Option<Track>>> = Rc::default();
    let t = Rc::clone(&track);
    on_signal_state_playing(&body, t);
    let t = Rc::clone(&track);
    attach_context_menu_with(&body, move || t.borrow().clone());
    let mut cells = Vec::with_capacity(i.columns.len());
    // Expanding columns (title, artists) share one homogeneous box, so each gets the
    // same fixed fraction of the row whatever its text: every row's artist column
    // starts at the same x without a SizeGroup re-measuring rows on every scroll.
    let flex = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    flex.set_homogeneous(true);
    flex.set_hexpand(true);
    let mut flex_added = false;
    for col in &i.columns {
        let cell = col.build();
        let grid = gtk::Grid::new();
        grid.set_valign(gtk::Align::Center);
        grid.attach(&cell.widget, 0, 0, 1, 1);
        if cell.flex {
            cell.widget.set_halign(gtk::Align::Fill);
            grid.set_hexpand(true);
            flex.append(&grid);
            if !flex_added {
                body.append(&flex);
                flex_added = true;
            }
        } else {
            body.append(&grid);
        }
        cells.push(cell);
    }
    container.set_child(Some(&body));
    let row = Rc::new(Row { body, cells, track });
    // SAFETY: read back only through `row_of` with the same type.
    unsafe { container.set_data("tonearm-row", row) };
}

fn on_signal_state_playing(body: &gtk::Box, track: Rc<RefCell<Option<Track>>>) {
    on_state(player::track_changed(), body, move |b, ev| {
        let id = track.borrow().as_ref().map(|t| t.id().to_string());
        let playing = id.is_some_and(|id| ev.as_ref().is_some_and(|e| e.track.id() == id));
        if playing {
            b.add_css_class("playing");
        } else {
            b.remove_css_class("playing");
        }
    });
}

fn bind(i: &Inner, item: &gtk::ListItem) {
    let pos = item.position() as usize;
    let Some(container) = item.child().and_downcast::<adw::Bin>() else { return };
    if row_of(&container).is_none() {
        build_row(i, &container);
    }
    let Some(row) = row_of(&container) else { return };
    let Some(track) = i.tracks.borrow().get(pos).cloned() else {
        bind_placeholder(i, item, &container, &row, pos);
        return;
    };
    container.set_opacity(1.0);
    if is_playing(track.id()) {
        row.body.add_css_class("playing");
    } else {
        row.body.remove_css_class("playing");
    }
    for c in &row.cells {
        (c.bind)(&track, pos);
    }
    let streamable = track.info.streamable;
    *row.track.borrow_mut() = Some(track);
    item.set_activatable(streamable);
    container.set_sensitive(streamable);
    if streamable {
        container.set_tooltip_text(None);
    } else {
        container.set_tooltip_text(Some(&tr("This track is not available for streaming")));
    }
}

fn bind_placeholder(i: &Inner, item: &gtk::ListItem, container: &adw::Bin, row: &Row, pos: usize) {
    row.body.remove_css_class("playing");
    let blank = Track::default();
    for c in &row.cells {
        (c.bind)(&blank, pos);
    }
    *row.track.borrow_mut() = None;
    item.set_activatable(false);
    container.set_sensitive(false);
    container.set_opacity(0.35);
    container.set_tooltip_text(Some(&tr("Loading…")));
    if let Some(f) = i.on_placeholder.borrow().clone() {
        glib::idle_add_local_once(move || f(pos));
    }
}

const DRAG_MIME: &str = "application/x-tonearm-track";

fn setup(i: &Inner, w: &Weak<Inner>, item: &gtk::ListItem) {
    let container = adw::Bin::new();
    // Shifting rows in `ListStore` updates `GtkListItem:position` without re-emitting `bind`.
    item.connect_position_notify(|item| {
        let pos = item.position();
        if pos == gtk::INVALID_LIST_POSITION {
            return;
        }
        if let Some(row) = item.child().and_downcast::<adw::Bin>().as_ref().and_then(row_of) {
            for c in &row.cells {
                (c.reposition)(pos as usize);
            }
        }
    });
    let source: Rc<RefCell<Option<gtk::DragSource>>> = Rc::default();
    let (w, cw, li) = (Weak::clone(w), container.downgrade(), item.downgrade());
    i.reorderable.on(move |on| {
        let (Some(inner), Some(container), Some(item)) = (w.upgrade(), cw.upgrade(), li.upgrade()) else { return true };
        let mut src = source.borrow_mut();
        if *on && src.is_none() {
            let ds = gtk::DragSource::new();
            ds.set_actions(gdk::DragAction::MOVE);
            let (w1, li1) = (Rc::downgrade(&inner), item.downgrade());
            ds.connect_prepare(move |_, _, _| {
                let (i, item) = (w1.upgrade()?, li1.upgrade()?);
                let pos = item.position();
                if pos == gtk::INVALID_LIST_POSITION || !start_drag(&i, pos as usize) {
                    return None; // placeholder rows can't be dragged
                }
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
                if let Some(i) = w2.upgrade() {
                    cancel_drag(&i);
                }
                true
            });
            let w3 = Rc::downgrade(&inner);
            ds.connect_drag_end(move |_, _, _| {
                if let Some(i) = w3.upgrade() {
                    cancel_drag(&i);
                }
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
        let w0 = Weak::clone(w);
        // Only this list's own drags carry a track being moved.
        target.connect_accept(move |_, _| w0.upgrade().is_some_and(|i| i.drag.borrow().is_some()));
        let w1 = Weak::clone(w);
        target.connect_drag_enter(move |_, _, _, y| {
            w1.upgrade().map_or_else(gdk::DragAction::empty, |i| drag_motion(&i, y))
        });
        let w2 = Weak::clone(w);
        target.connect_drag_motion(move |_, _, _, y| {
            w2.upgrade().map_or_else(gdk::DragAction::empty, |i| drag_motion(&i, y))
        });
        let w3 = Weak::clone(w);
        target.connect_drag_leave(move |_, _| {
            if let Some(i) = w3.upgrade() {
                leave_drag(&i);
            }
        });
        let w4 = Weak::clone(w);
        target.connect_drop(move |_, drop, _, y| {
            if let Some(i) = w4.upgrade() {
                drop_drag(&i, y, |action| drop.finish(action));
            } else {
                drop.finish(gdk::DragAction::empty());
            }
            true
        });
        if let Some(v) = i.view.upgrade() {
            v.add_controller(target.clone());
        }
        *dt = Some(target);
    } else if !on {
        cancel_drag(i);
        if let Some(t) = dt.take()
            && let Some(v) = i.view.upgrade()
        {
            v.remove_controller(&t);
        }
    }
}

/// The track context menu model (same entries as the "More…" button).
pub fn track_menu(t: &Track) -> (gio::Menu, Option<gio::Menu>) {
    let model = gio::Menu::new();
    let queue = gio::Menu::new();
    let next = gio::MenuItem::new(Some(&tr("Play Next")), Some("win.player.play-next-track"));
    next.set_action_and_target_value(Some("win.player.play-next-track"), Some(&t.id().to_variant()));
    queue.append_item(&next);
    let add = gio::MenuItem::new(Some(&tr("Add to Queue")), Some("win.player.queue-track"));
    add.set_action_and_target_value(Some("win.player.queue-track"), Some(&t.id().to_variant()));
    queue.append_item(&add);
    model.append_section(None, &queue);
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
    let playlists = crate::state::signed_in_changed().get().then(|| {
        let playlists = super::add_to_playlist::submenu(t.id());
        model.append_submenu(Some(&tr("Add to Playlist")), &playlists);
        playlists
    });
    (model, playlists)
}

/// Popover for [`track_menu`] that loads the user's playlists when shown.
pub fn track_menu_popover(t: &Track) -> gtk::PopoverMenu {
    let (model, playlists) = track_menu(t);
    let popover = gtk::PopoverMenu::from_model(Some(&model));
    // Size the pages before the first frame, and fetch the playlists now (once per
    // session) so the shared width doesn't change after the menu is already shown.
    stabilize(popover.upcast_ref());
    if let Some(playlists) = playlists {
        super::add_to_playlist::ensure_loaded(&playlists, t.id());
    }
    popover
}

/// Maximum height of a (sub)menu page before it scrolls.
const MENU_MAX_HEIGHT: i32 = 300;
/// Width of every menu page.
const MENU_WIDTH: i32 = 240;

/// Keeps the popover's size steady when sliding into a submenu: every page gets
/// the same width (no width animation), and tall pages scroll instead of growing.
fn stabilize(w: &gtk::Widget) {
    if let Some(stack) = w.downcast_ref::<gtk::Stack>() {
        stack.set_hhomogeneous(true);
        // Fixed width up front (fits 24-char playlist names plus the arrow), so the
        // menu never resizes when playlists arrive or a submenu opens.
        stack.set_size_request(MENU_WIDTH, -1);
        stack.set_interpolate_size(false);
        if !stack.has_css_class("tonearm-stable") {
            stack.add_css_class("tonearm-stable");
            // Submenu pages can be rebuilt (e.g. playlists arriving); re-apply on navigation.
            stack.connect_visible_child_notify(|s| stabilize(s.upcast_ref()));
        }
    }
    if let Some(sw) = w.downcast_ref::<gtk::ScrolledWindow>() {
        sw.set_hscrollbar_policy(gtk::PolicyType::Never);
        sw.set_propagate_natural_height(true);
        sw.set_max_content_height(MENU_MAX_HEIGHT);
    }
    let mut child = w.first_child();
    while let Some(c) = child {
        stabilize(&c);
        child = c.next_sibling();
    }
}

/// Right-click (and touch long-press) on `w` opens the track menu at the pointer.
pub fn attach_track_context_menu(w: &impl IsA<gtk::Widget>, t: &Track) {
    let t = t.clone();
    attach_context_menu_with(w, move || Some(t.clone()));
}

/// Like [`attach_track_context_menu`] for recycled rows: `current` yields the bound track.
fn attach_context_menu_with(w: &impl IsA<gtk::Widget>, current: impl Fn() -> Option<Track> + 'static) {
    type Open = Rc<dyn Fn(&gtk::Widget, f64, f64)>;
    let open: Open = {
        Rc::new(move |w, x, y| {
            let Some(t) = current() else { return };
            let popover = track_menu_popover(&t);
            popover.set_has_arrow(false);
            popover.set_halign(gtk::Align::Start);
            popover.set_parent(w);
            #[expect(clippy::cast_possible_truncation, reason = "pointer coordinates fit in i32")]
            popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover.connect_closed(|p| {
                let p = p.clone();
                glib::idle_add_local_once(move || p.unparent());
            });
            popover.popup();
        })
    };
    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_SECONDARY);
    let o = Rc::clone(&open);
    click.connect_pressed(move |g, _, x, y| {
        if let Some(w) = g.widget() {
            g.set_state(gtk::EventSequenceState::Claimed);
            o(&w, x, y);
        }
    });
    w.add_controller(click);
    let long = gtk::GestureLongPress::new();
    long.set_touch_only(true);
    long.connect_pressed(move |g, x, y| {
        if let Some(w) = g.widget() {
            g.set_state(gtk::EventSequenceState::Claimed);
            open(&w, x, y);
        }
    });
    w.add_controller(long);
}

#[cfg(test)]
#[path = "tracklist_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) use tests::run_gtk_test;
