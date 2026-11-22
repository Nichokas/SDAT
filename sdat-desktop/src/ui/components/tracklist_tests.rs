use super::*;

pub(crate) fn run_gtk_test(f: impl FnOnce() + Send + 'static) {
    use std::sync::{Mutex, OnceLock, mpsc};

    type Job = Box<dyn FnOnce() + Send + 'static>;
    type Reply = mpsc::Sender<std::thread::Result<()>>;
    type Runner = Mutex<mpsc::Sender<(Job, Reply)>>;
    // GTK4 crashes if `gtk::init()` is called a second time after failing headless,
    // and requires all widget operations to stay on the thread that initialized it.
    static RUNNER: OnceLock<Option<Runner>> = OnceLock::new();
    let tx = RUNNER.get_or_init(|| {
        let (init_tx, init_rx) = mpsc::channel();
        let (job_tx, job_rx) = mpsc::channel::<(Job, Reply)>();
        std::thread::spawn(move || {
            let ok = gtk::init().is_ok();
            if ok {
                let _ = adw::init();
            }
            let _ = init_tx.send(ok);
            if !ok {
                return;
            }
            while let Ok((job, done)) = job_rx.recv() {
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                let _ = done.send(res);
            }
        });
        init_rx.recv().is_ok_and(|ok| ok).then(|| Mutex::new(job_tx))
    });
    let Some(tx) = tx else {
        eprintln!("no display; skipping");
        return;
    };
    let (done_tx, done_rx) = mpsc::channel();
    tx.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .send((Box::new(f), done_tx))
        .expect("GTK test thread exited unexpectedly");
    if let Err(payload) = done_rx.recv().expect("GTK test thread dropped result") {
        std::panic::resume_unwind(payload);
    }
}

fn tr(id: i64) -> Track {
    let mut t = Track::default();
    t.info.id = id.to_string();
    t
}

#[test]
fn diff_ops_append_replace_remove() {
    let (a, b, c) = (tr(1), tr(2), tr(3));
    assert_eq!(diff_ops(&[], &[a.clone(), b.clone()]), vec![(0, Op::Append), (1, Op::Append)]);
    assert_eq!(diff_ops(&[a.clone(), b.clone()], &[a.clone(), c.clone()]), vec![(1, Op::Replace)]);
    assert_eq!(diff_ops(&[a.clone(), b, c], std::slice::from_ref(&a)), vec![(2, Op::Remove), (1, Op::Remove)]);
    assert!(diff_ops(std::slice::from_ref(&a), std::slice::from_ref(&a)).is_empty());
}

/// Regression: pages build a `TrackList`, add tracks and drop the handle,
/// keeping only the widget. The row factory must still find its state, or
/// every row binds empty (collection/search "Tracks" showed nothing).
#[test]
fn state_lives_as_long_as_the_view() {
    run_gtk_test(|| {
        let list = TrackList::new(vec![Col::duration()]);
        list.add_track(Track::default());
        let inner = Rc::downgrade(&list.0);
        let view = list.widget().clone();
        drop(list);
        assert!(inner.upgrade().is_some(), "state dropped while the view is alive");
        assert_eq!(inner.upgrade().map(|i| i.tracks.borrow().len()), Some(1));
        drop(view);
        assert!(inner.upgrade().is_none(), "state leaked after the view was destroyed");
    });
}

fn collect_labels(w: &gtk::Widget, out: &mut Vec<String>) {
    if let Some(l) = w.downcast_ref::<gtk::Label>() {
        out.push(l.label().to_string());
    }
    let mut child = w.first_child();
    while let Some(c) = child {
        collect_labels(&c, out);
        child = c.next_sibling();
    }
}

#[test]
fn shifted_rows_update_live_positions_and_sync_cancels_drag() {
    run_gtk_test(|| {
        let make = |id: &str| {
            let mut t = Track::default();
            t.info.id = id.into();
            t
        };
        let recorded: Rc<RefCell<std::collections::HashMap<String, LivePos>>> = Rc::default();
        let rec = Rc::clone(&recorded);
        let list = TrackList::new(vec![
            Col::position(),
            Col::custom_widget_button(move |id, pos| {
                rec.borrow_mut().insert(id.to_owned(), pos);
                gtk::Button::new().upcast()
            }),
        ]);
        list.replace_all(&[make("a"), make("b"), make("c")]);
        let win = gtk::Window::new();
        win.set_default_size(400, 400);
        win.set_child(Some(list.widget()));
        win.present();
        let ctx = glib::MainContext::default();
        for _ in 0..20 {
            while ctx.iteration(false) {}
            if recorded.borrow().len() == 3 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut before_labels = Vec::new();
        collect_labels(list.widget(), &mut before_labels);
        assert_eq!(before_labels, ["1", "2", "3"]);
        assert_eq!(recorded.borrow().get("a").and_then(|p| p.get()), Some(0));
        assert_eq!(recorded.borrow().get("b").and_then(|p| p.get()), Some(1));
        assert_eq!(recorded.borrow().get("c").and_then(|p| p.get()), Some(2));

        // Drag row 2 ("c") to row 0 without re-binding "a" or "b".
        assert!(start_drag(&list.0, 2));
        assert!(move_drag(&list.0, 0));
        while ctx.iteration(false) {}

        let mut after_labels = Vec::new();
        collect_labels(list.widget(), &mut after_labels);
        assert_eq!(after_labels, ["1", "2", "3"]);
        assert_eq!(recorded.borrow().get("c").and_then(|p| p.get()), Some(0));
        assert_eq!(recorded.borrow().get("a").and_then(|p| p.get()), Some(1));
        assert_eq!(recorded.borrow().get("b").and_then(|p| p.get()), Some(2));

        // External sync during an active drag clears in-flight drag state.
        list.replace_all(&[make("a"), make("b")]);
        assert!(list.0.drag.borrow().is_none());
        win.close();
        while ctx.iteration(false) {}
    });
}

#[test]
fn drag_cancel_never_duplicates_or_corrupts_tracks() {
    run_gtk_test(|| {
        let make = |id: &str| {
            let mut t = Track::default();
            t.info.id = id.into();
            t
        };
        let ids = |list: &TrackList| -> Vec<String> {
            list.tracks().into_iter().map(|t| t.id().to_owned()).collect()
        };
        let list = TrackList::new(vec![Col::position(), Col::title()]);
        list.set_reorder_callback(Some(Box::new(|_, _, _| {})));
        let win = gtk::Window::new();
        win.set_default_size(400, 400);
        win.set_child(Some(list.widget()));
        win.present();
        let ctx = glib::MainContext::default();

        // 1. Drag start -> immediate cancel (before any motion): zero duplicates.
        list.replace_all(&[make("a"), make("b"), make("c")]);
        while ctx.iteration(false) {}
        assert!(start_drag(&list.0, 1));
        cancel_drag(&list.0);
        while ctx.iteration(false) {}
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert_eq!(list.0.store.n_items(), 3);
        assert!(list.0.drag.borrow().is_none());

        // 2. Drag start -> motion -> cancel: exact original order restored, zero duplicates.
        assert!(start_drag(&list.0, 2));
        assert!(move_drag(&list.0, 0));
        assert_eq!(ids(&list), ["c", "a", "b"]);
        assert_eq!(list.0.store.n_items(), 3);
        cancel_drag(&list.0);
        // A follow-up drag_end/cancel is a no-op.
        cancel_drag(&list.0);
        while ctx.iteration(false) {}
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert_eq!(list.0.store.n_items(), 3);
        assert!(list.0.drag.borrow().is_none());

        // 3. Drag start (before motion) -> sync arrives mid-drag -> cancel: synced list intact.
        assert!(start_drag(&list.0, 0));
        list.replace_all(&[make("x"), make("y")]);
        cancel_drag(&list.0);
        while ctx.iteration(false) {}
        assert_eq!(ids(&list), ["x", "y"]);
        assert_eq!(list.0.store.n_items(), 2);
        assert!(list.0.drag.borrow().is_none());

        // 4. Drag start -> motion -> sync arrives mid-drag -> cancel: synced list intact.
        list.replace_all(&[make("a"), make("b"), make("c")]);
        assert!(start_drag(&list.0, 2));
        assert!(move_drag(&list.0, 0));
        list.replace_all(&[make("a"), make("b")]);
        cancel_drag(&list.0);
        while ctx.iteration(false) {}
        assert_eq!(ids(&list), ["a", "b"]);
        assert_eq!(list.0.store.n_items(), 2);
        assert!(list.0.drag.borrow().is_none());

        // 5. Diverged drag state (target out of bounds or mismatched track): never inserts without removing.
        *list.0.drag.borrow_mut() = Some(DragState { source: 0, target: 5, track: make("a"), over_valid_row: true });
        cancel_drag(&list.0);
        assert_eq!(ids(&list), ["a", "b"]);
        assert_eq!(list.0.store.n_items(), 2);

        *list.0.drag.borrow_mut() = Some(DragState { source: 0, target: 1, track: make("a"), over_valid_row: true });
        cancel_drag(&list.0);
        assert_eq!(ids(&list), ["a", "b"]);
        assert_eq!(list.0.store.n_items(), 2);

        // 6. Placeholders stay intact across drag motion + cancel and cannot be dragged.
        list.set_placeholders(2);
        assert_eq!(list.0.store.n_items(), 4);
        assert!(!start_drag(&list.0, 2));
        assert!(start_drag(&list.0, 1));
        assert!(move_drag(&list.0, 0));
        assert!(!move_drag(&list.0, 2));
        assert_eq!(ids(&list), ["b", "a"]);
        assert_eq!(list.0.store.n_items(), 4);
        cancel_drag(&list.0);
        while ctx.iteration(false) {}
        assert_eq!(ids(&list), ["a", "b"]);
        assert_eq!(list.0.store.n_items(), 4);

        win.close();
        while ctx.iteration(false) {}
    });
}

#[test]
fn list_update_during_drag_restores_the_dragged_row() {
    run_gtk_test(|| {
        let make = |id: &str| {
            let mut t = Track::default();
            t.info.id = id.into();
            t
        };
        let ids = |list: &TrackList| -> Vec<String> {
            list.tracks().into_iter().map(|t| t.id().to_owned()).collect()
        };
        let list = TrackList::new(vec![Col::position(), Col::title()]);
        list.set_reorder_callback(Some(Box::new(|_, _, _| {})));
        list.replace_all(&[make("a"), make("b"), make("c")]);
        assert!(start_drag(&list.0, 0));
        assert!(move_drag(&list.0, 2));
        assert_eq!(ids(&list), ["b", "c", "a"]);
        list.add_track(make("d"));
        assert!(list.0.drag.borrow().is_none());
        assert_eq!(ids(&list), ["a", "b", "c", "d"]);
        assert_eq!(list.0.store.n_items(), 4);
    });
}

#[test]
fn invalid_drop_is_rejected_and_restores_original_order() {
    run_gtk_test(|| {
        let make = |id: &str| {
            let mut t = Track::default();
            t.info.id = id.into();
            t
        };
        let ids = |list: &TrackList| -> Vec<String> {
            list.tracks().into_iter().map(|t| t.id().to_owned()).collect()
        };
        let reorders: Rc<RefCell<Vec<(usize, usize, String)>>> = Rc::default();
        let rec = Rc::clone(&reorders);
        let list = TrackList::new(vec![Col::position(), Col::title()]);
        list.set_reorder_callback(Some(Box::new(move |src, dst, t| {
            rec.borrow_mut().push((src, dst, t.id().to_owned()));
        })));
        list.replace_all(&[make("a"), make("b"), make("c")]);
        let win = gtk::Window::new();
        win.set_default_size(400, 400);
        win.set_child(Some(list.widget()));
        win.present();
        let ctx = glib::MainContext::default();
        for _ in 0..20 {
            while ctx.iteration(false) {}
            if list.0.view.upgrade().and_then(|v| v.first_child()).is_some_and(|c| c.height() > 0) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let h = list.0.view.upgrade().and_then(|v| v.first_child()).map_or(1, |c| c.height()).max(1);
        let row_y = |idx: usize| (idx as f64 + 0.5) * f64::from(h);

        // 1. Dropping before any valid enter/motion event is rejected and finishes empty without firing reorder.
        assert!(start_drag(&list.0, 2));
        let finished = Cell::new(None);
        assert!(!drop_drag(&list.0, row_y(0), |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::empty()));
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert_eq!(list.0.store.n_items(), 3);
        assert!(list.0.drag.borrow().is_none());
        assert!(reorders.borrow().is_empty());

        // 2. Dropping after drag-leave rejects the drop, finishes empty, and restores the original order.
        assert!(start_drag(&list.0, 2));
        assert_eq!(drag_motion(&list.0, row_y(0)), gdk::DragAction::MOVE);
        assert_eq!(ids(&list), ["c", "a", "b"]);
        leave_drag(&list.0);
        let finished = Cell::new(None);
        assert!(!drop_drag(&list.0, row_y(0), |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::empty()));
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert_eq!(list.0.store.n_items(), 3);
        assert!(list.0.drag.borrow().is_none());
        assert!(reorders.borrow().is_empty());

        // 3. Moving outside valid rows (onto placeholders or above row 0) rejects a subsequent drop.
        list.set_placeholders(2);
        assert!(start_drag(&list.0, 2));
        assert_eq!(drag_motion(&list.0, row_y(0)), gdk::DragAction::MOVE);
        assert_eq!(ids(&list), ["c", "a", "b"]);
        assert_eq!(drag_motion(&list.0, row_y(3)), gdk::DragAction::empty());
        let finished = Cell::new(None);
        assert!(!drop_drag(&list.0, row_y(0), |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::empty()));
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert_eq!(list.0.store.n_items(), 5);
        assert!(list.0.drag.borrow().is_none());
        assert!(reorders.borrow().is_empty());

        // 4. Dropping directly at an out-of-range y (negative or placeholder) finishes empty and restores order.
        assert!(start_drag(&list.0, 2));
        assert_eq!(drag_motion(&list.0, row_y(0)), gdk::DragAction::MOVE);
        assert_eq!(drag_motion(&list.0, -5.0), gdk::DragAction::empty());
        let finished = Cell::new(None);
        assert!(!drop_drag(&list.0, -5.0, |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::empty()));
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert!(start_drag(&list.0, 2));
        assert_eq!(drag_motion(&list.0, row_y(0)), gdk::DragAction::MOVE);
        let finished = Cell::new(None);
        assert!(!drop_drag(&list.0, row_y(3), |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::empty()));
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert_eq!(list.0.store.n_items(), 5);
        assert!(reorders.borrow().is_empty());

        // 5. Entering directly on the starting row advertises MOVE and finishes MOVE without firing reorder.
        assert!(start_drag(&list.0, 1));
        assert_eq!(drag_motion(&list.0, row_y(1)), gdk::DragAction::MOVE);
        let finished = Cell::new(None);
        assert!(drop_drag(&list.0, row_y(1), |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::MOVE));
        assert_eq!(ids(&list), ["a", "b", "c"]);
        assert!(list.0.drag.borrow().is_none());
        assert!(reorders.borrow().is_empty());

        // 6. Leaving and re-entering on a valid row allows the drop to commit with MOVE.
        assert!(start_drag(&list.0, 2));
        assert_eq!(drag_motion(&list.0, row_y(0)), gdk::DragAction::MOVE);
        leave_drag(&list.0);
        assert_eq!(drag_motion(&list.0, row_y(1)), gdk::DragAction::MOVE);
        let finished = Cell::new(None);
        assert!(drop_drag(&list.0, row_y(1), |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::MOVE));
        assert_eq!(ids(&list), ["a", "c", "b"]);
        assert!(list.0.drag.borrow().is_none());
        assert_eq!(*reorders.borrow(), vec![(2, 1, "c".to_owned())]);

        // 7. Dropping after an external sync cleared drag state finishes empty without altering the synced list.
        assert!(start_drag(&list.0, 0));
        assert_eq!(drag_motion(&list.0, row_y(1)), gdk::DragAction::MOVE);
        list.replace_all(&[make("x"), make("y")]);
        let finished = Cell::new(None);
        assert!(!drop_drag(&list.0, row_y(1), |a| finished.set(Some(a))));
        assert_eq!(finished.get(), Some(gdk::DragAction::empty()));
        assert_eq!(ids(&list), ["x", "y"]);
        assert_eq!(*reorders.borrow(), vec![(2, 1, "c".to_owned())]);

        win.close();
        while ctx.iteration(false) {}
    });
}

#[test]
#[ignore = "perf probe, needs a display"]
fn scroll_cost() {
    run_gtk_test(|| {
        crate::ensure_schema();
        crate::register_resources();
        let cols = match std::env::var("PROBE_COLS").as_deref() {
            Ok("nocontrols") => vec![Col::cover(), Col::title_album(), Col::artists(), Col::duration()],
            Ok("controls") => vec![Col::controls()],
            Ok("cover") => vec![Col::cover()],
            Ok("text") => vec![Col::title_album(), Col::artists(), Col::duration()],
            _ => vec![Col::cover(), Col::title_album(), Col::artists(), Col::duration(), Col::controls()],
        };
        let rows: usize = std::env::var("PROBE_ROWS").ok().and_then(|v| v.parse().ok()).unwrap_or(1000);
        let list = TrackList::new(cols);
        for k in 0..rows {
            let mut t = Track::default();
            t.info.id = k.to_string();
            t.info.title = format!("Track number {k} with a reasonably long title");
            t.info.streamable = k % 7 != 0;
            list.add_track(t);
        }
        // Same nesting as the real pages (routes::tracklist_scroller).
        let sw = gtk::ScrolledWindow::new();
        sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::External);
        sw.set_child(Some(&crate::ui::components::main_content_scrollable(list.widget())));
        let win = gtk::Window::new();
        win.set_default_size(1000, 800);
        win.set_child(Some(&sw));
        win.present();
        let ctx = glib::MainContext::default();
        let settle = || {
            let t = std::time::Instant::now();
            while t.elapsed() < std::time::Duration::from_millis(300) {
                while ctx.iteration(false) {}
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        settle();
        let adj = sw.vadjustment();
        // Scroll the whole list in 300px steps; report per tenth of the list.
        let steps = ((adj.upper() - adj.page_size()) / 300.0).ceil().max(1.0) as usize;
        let mut seg = vec![(0.0f64, 0.0f64, 0usize); 10];
        for k in 0..=steps {
            let t = std::time::Instant::now();
            adj.set_value(k as f64 * 300.0);
            while ctx.iteration(false) {}
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            if ms > 16.0 {
                eprintln!("PROBE slow step={k} ms={ms:.1}");
            }
            let s = &mut seg[(k * 10 / (steps + 1)).min(9)];
            s.0 += ms;
            s.1 = s.1.max(ms);
            s.2 += 1;
        }
        for (i, (tot, worst, n)) in seg.iter().enumerate() {
            eprintln!("PROBE rows={rows} part={}0% avg_step_ms={:.2} worst_ms={:.2}", i, tot / *n.max(&1) as f64, worst);
        }
        eprintln!("PROBE upper={} steps={steps}", adj.upper());
        win.close();
    });
}

#[test]
#[ignore = "needs a display (gtk::init)"]
fn placeholders_are_taken_over_by_loaded_tracks() {
    run_gtk_test(|| {
        let tk = |id: &str| {
            let mut t = Track::default();
            t.info.id = id.into();
            t
        };
        let list = TrackList::new(vec![Col::duration()]);
        list.replace_all(&[tk("a"), tk("b")]);
        list.set_placeholders(3000);
        assert_eq!(list.0.store.n_items(), 3002);
        // a page arrives: rows replace placeholders, the length stays the same
        list.replace_all(&[tk("a"), tk("b"), tk("c"), tk("d")]);
        assert_eq!(list.0.store.n_items(), 3002);
        assert_eq!(list.0.placeholders.get(), 2998);
        // the player then reports the new unloaded count
        list.set_placeholders(2990);
        assert_eq!(list.0.store.n_items(), 2994);
        // playing on pops the head; total shrinks by one
        list.replace_all(&[tk("b"), tk("c"), tk("d")]);
        assert_eq!(list.0.store.n_items(), 2993);
        list.set_placeholders(0);
        assert_eq!(list.0.store.n_items(), 3);
    });
}

/// Renders a track list with real-length texts to a PNG (`PROBE_PNG` path).
#[test]
#[ignore = "visual probe, needs a display"]
fn render_png() {
    run_gtk_test(|| {
        crate::ensure_schema();
        crate::register_resources();
        let data = [
            ("Con Calma", "Con Calma", "Daddy Yankee, Snow"),
            ("No You Girls", "Tonight", "Franz Ferdinand"),
            ("Como Me Ves, Te Veras", "Made in Japan", "Siniestro Total"),
            ("Reptilia", "Room On Fire", "The Strokes"),
            ("Jesus of Suburbia", "American Idiot", "Green Day"),
            ("A very very long track title that should be ellipsized somewhere", "Album", "X"),
            ("Short", "An album with quite a long name indeed", "A, B, C, D, E, F, G and many more artists here"),
        ];
        let position = std::env::var_os("PROBE_POSITION").is_some();
        let cols = if position {
            vec![Col::position(), Col::title(), Col::artists(), Col::duration(), Col::controls()]
        } else {
            vec![Col::cover(), Col::title_album(), Col::artists(), Col::duration(), Col::controls()]
        };
        let list = TrackList::new(cols);
        for (k, (t, a, ar)) in data.iter().enumerate() {
            let mut tr = Track::default();
            tr.info.id = k.to_string();
            tr.info.title = (*t).into();
            tr.info.streamable = true;
            tr.album = Some(crate::model::AlbumInfo { title: (*a).into(), ..Default::default() });
            tr.artists = ar.split(", ").map(|n| crate::model::ArtistInfo { name: n.into(), ..Default::default() }).collect();
            list.add_track(tr);
        }
        if let Some(n) = std::env::var("PROBE_PLACEHOLDERS").ok().and_then(|v| v.parse().ok()) {
            list.set_placeholders(n);
        }
        let sw = gtk::ScrolledWindow::new();
        sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::External);
        sw.set_child(Some(&crate::ui::components::main_content_scrollable(list.widget())));
        let win = gtk::Window::new();
        win.set_default_size(900, 600);
        win.set_child(Some(&sw));
        win.present();
        let ctx = glib::MainContext::default();
        let t = std::time::Instant::now();
        while t.elapsed() < std::time::Duration::from_millis(800) {
            while ctx.iteration(false) {}
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let w = win.upcast_ref::<gtk::Widget>();
        let paintable = gtk::WidgetPaintable::new(Some(w));
        let snap = gtk::Snapshot::new();
        paintable.snapshot(&snap, f64::from(w.width()), f64::from(w.height()));
        let node = snap.to_node().unwrap();
        let renderer = w.native().unwrap().renderer().unwrap();
        let tex = renderer.render_texture(&node, None);
        tex.save_to_png(std::env::var("PROBE_PNG").unwrap_or_else(|_| "/tmp/tracklist.png".into())).unwrap();
        win.close();
    });
}

#[test]
fn drag_targets_the_row_under_the_pointer_in_a_scrolled_list() {
    run_gtk_test(|| {
        let list = TrackList::new(vec![Col::position(), Col::title()]);
        let tracks: Vec<Track> = (0..200)
            .map(|k| {
                let mut t = Track::default();
                t.info.id = k.to_string();
                t
            })
            .collect();
        list.replace_all(&tracks);
        // Same nesting as the real pages (routes::tracklist_scroller).
        let sw = gtk::ScrolledWindow::new();
        sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::External);
        sw.set_child(Some(&crate::ui::components::main_content_scrollable(list.widget())));
        let win = gtk::Window::new();
        win.set_default_size(400, 400);
        win.set_child(Some(&sw));
        win.present();
        let ctx = glib::MainContext::default();
        let settle = || {
            for _ in 0..40 {
                while ctx.iteration(false) {}
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        settle();
        let h = list.0.view.upgrade().and_then(|v| v.first_child()).map_or(1, |c| c.height()).max(1);
        sw.vadjustment().set_value(50.0 * f64::from(h));
        settle();
        let top = sw.vadjustment().value();
        assert!(top > 0.0, "the list must scroll for this test");
        let first_visible = (top / f64::from(h)).floor() as usize;
        assert_eq!(row_index_at(&list.0, 0.5 * f64::from(h)), Some(first_visible));
        win.close();
    });
}
