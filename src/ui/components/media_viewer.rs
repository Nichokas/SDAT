//! Port of `components/media_viewer.go`: a full-window image viewer overlay.
use std::cell::{Cell, RefCell};

use adw::prelude::*;
use gtk::{gdk, gio, gsk};

use crate::ui::accels::PRIMARY;

pub struct MediaViewer {
    revealer: gtk::Revealer,
    picture: gtk::Picture,
    back: gtk::Button,
    zoom: Cell<f64>,
    base: Cell<(i32, i32)>,
    media: RefCell<Option<gdk::Texture>>,
}

thread_local! {
    static VIEWER: &'static MediaViewer = Box::leak(Box::new(MediaViewer::build()));
}

pub fn get() -> &'static MediaViewer {
    VIEWER.with(|v| *v)
}

impl MediaViewer {
    fn build() -> Self {
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_action_name(Some("mediaviewer.close"));
        let model = gio::Menu::new();
        model.append_item(&gio::MenuItem::new(Some("Copy Media"), Some("mediaviewer.copy")));
        let more = gtk::MenuButton::new();
        more.set_menu_model(Some(&model));
        more.set_icon_name("view-more-symbolic");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&adw::Bin::new()));
        header.set_decoration_layout(Some(""));
        header.pack_start(&back);
        header.pack_end(&more);
        let picture = gtk::Picture::new();
        picture.set_halign(gtk::Align::Center);
        picture.set_valign(gtk::Align::Center);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Contain);
        let dbl = gtk::GestureClick::new();
        dbl.set_button(1);
        dbl.connect_pressed(|g, n, _, _| {
            g.set_state(gtk::EventSequenceState::Claimed);
            if n == 2 {
                get().reset_zoom();
            }
        });
        picture.add_controller(dbl);
        let frame = gtk::AspectFrame::new(0.5, 0.5, 1.0, true);
        frame.set_child(Some(&picture));
        let sw = gtk::ScrolledWindow::new();
        sw.set_child(Some(&frame));
        sw.set_hexpand(true);
        sw.set_vexpand(true);
        sw.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        let click = gtk::GestureClick::new();
        click.connect_pressed(|_, _, _, _| get().hide());
        sw.add_controller(click);
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        scroll.connect_scroll(|c, _, dy| {
            if c.current_event_state().intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::META_MASK) {
                get().zoom_by(if dy < 0.0 { 1.1 } else { 0.9 });
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        sw.add_controller(scroll);
        let zoom = gtk::GestureZoom::new();
        let start = std::rc::Rc::new(Cell::new(1.0));
        let s = std::rc::Rc::clone(&start);
        zoom.connect_begin(move |_, _| s.set(get().zoom.get()));
        zoom.connect_scale_changed(move |_, scale| get().set_zoom(start.get() * scale));
        sw.add_controller(zoom);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&sw));
        let overlay = adw::ToastOverlay::new();
        overlay.set_child(Some(&toolbar));
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.append(&overlay);
        vbox.set_halign(gtk::Align::Fill);
        vbox.set_valign(gtk::Align::Fill);
        vbox.set_hexpand(true);
        vbox.set_vexpand(true);
        vbox.add_css_class("mediaviewer");
        let revealer = gtk::Revealer::new();
        revealer.set_child(Some(&vbox));
        revealer.set_visible(false);
        revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);
        let group = gio::SimpleActionGroup::new();
        let copy = gio::SimpleAction::new("copy", None);
        let ov = overlay.downgrade();
        copy.connect_activate(move |_, _| {
            get().copy_media();
            if let Some(o) = ov.upgrade() {
                let t = adw::Toast::new("Copied image to clipboard");
                t.set_timeout(3);
                o.add_toast(t);
            }
        });
        group.add_action(&copy);
        let close = gio::SimpleAction::new("close", None);
        close.connect_activate(|_, _| get().hide());
        group.add_action(&close);
        revealer.insert_action_group("mediaviewer", Some(&group));
        let sc = gtk::ShortcutController::new();
        for (accel, action) in [(format!("{PRIMARY}C"), "mediaviewer.copy"), ("Escape".to_string(), "mediaviewer.close")] {
            sc.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(&accel), Some(gtk::NamedAction::new(action))));
        }
        revealer.add_controller(sc);
        revealer.connect_show(|_| {
            get().back.grab_focus();
        });
        MediaViewer { revealer, picture, back, zoom: Cell::new(1.0), base: Cell::new((0, 0)), media: RefCell::new(None) }
    }

    pub fn widget(&self) -> &gtk::Revealer {
        &self.revealer
    }

    fn copy_media(&self) {
        if let (Some(d), Some(t)) = (gdk::Display::default(), self.media.borrow().as_ref()) {
            d.clipboard().set_texture(t);
        }
    }

    pub fn show_file(&self, paintable: &gdk::Paintable) {
        self.reset_zoom();
        self.picture.set_paintable(Some(paintable));
        self.revealer.set_visible(true);
        self.revealer.set_reveal_child(true);
        let mut w = f64::from(paintable.intrinsic_width());
        let mut h = f64::from(paintable.intrinsic_height());
        if w <= 0.0 {
            w = 1440.0;
        }
        if h <= 0.0 {
            h = 1440.0;
        }
        let snap = gtk::Snapshot::new();
        paintable.snapshot(&snap, w, h);
        if let Some(node) = snap.to_node() {
            let r = gsk::CairoRenderer::new();
            if r.realize_for_display(&gdk::Display::default().unwrap()).is_ok() {
                *self.media.borrow_mut() = Some(r.render_texture(&node, None));
                r.unrealize();
            }
        }
    }

    pub fn hide(&self) {
        if !self.revealer.reveals_child() {
            return;
        }
        self.revealer.set_reveal_child(false);
        let r = self.revealer.downgrade();
        glib::timeout_add_local_once(std::time::Duration::from_millis(u64::from(self.revealer.transition_duration())), move || {
            if let Some(r) = r.upgrade() {
                r.set_visible(false);
            }
        });
    }

    fn reset_zoom(&self) {
        self.zoom.set(1.0);
        self.base.set((0, 0));
        self.picture.set_size_request(-1, -1);
    }

    fn zoom_by(&self, f: f64) {
        self.set_zoom(self.zoom.get() * f);
    }

    fn set_zoom(&self, level: f64) {
        let level = level.clamp(1.0, 10.0);
        self.zoom.set(level);
        let (mut bw, mut bh) = self.base.get();
        if bw == 0 || bh == 0 {
            bw = self.picture.width();
            bh = self.picture.height();
            self.base.set((bw, bh));
        }
        self.picture.set_size_request((f64::from(bw) * level) as i32, (f64::from(bh) * level) as i32);
    }
}
