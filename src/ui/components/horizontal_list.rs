//! Port of `components/horizontal_list`.
use adw::prelude::*;
use gtk::pango;

use crate::i18n::tr;
use crate::router;
use crate::ui::dsl::{W, label, hstack, spacer, spacing, vstack};

pub struct HorizontalList {
    pub widget: gtk::Box,
    container: gtk::Box,
    header: gtk::Box,
    route: gtk::CenterBox,
}

fn arrow(icon: &str, tooltip: &str) -> gtk::Button {
    let img = gtk::Image::from_icon_name(icon);
    img.set_pixel_size(10);
    let b = gtk::Button::new().min_height(10).min_width(10).hpadding(10).tooltip(tooltip);
    b.set_child(Some(&img));
    b
}

impl HorizontalList {
    pub fn new(title: &str) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let route = gtk::CenterBox::new().hexp(false).vexp(false);
        let next = arrow("right-symbolic", &tr("Scroll to the Right"));
        let prev = arrow("left-symbolic", &tr("Scroll to the Left"));
        let t = label(title).class("title-2").valign(gtk::Align::Center);
        t.set_ellipsize(pango::EllipsizeMode::End);
        let header = hstack(&[
            t.upcast_ref(),
            spacer().vexp(false).upcast_ref(),
            spacing(hstack(&[route.upcast_ref(), prev.upcast_ref(), next.upcast_ref()]), 10).upcast_ref(),
        ]);
        let sw = gtk::ScrolledWindow::new();
        sw.set_child(Some(&container));
        sw.set_valign(gtk::Align::Start);
        sw.set_policy(gtk::PolicyType::External, gtk::PolicyType::Never);
        sw.set_propagate_natural_width(true);
        sw.connect_realize(|sw| {
            if let Some(c) = sw.child() {
                c.set_overflow(gtk::Overflow::Visible);
            }
        });
        let adj = sw.hadjustment();
        let a = adj.downgrade();
        next.connect_clicked(move |_| {
            if let Some(adj) = a.upgrade() {
                let mut cur = adj.value();
                cur -= cur % 192.0;
                adj.set_step_increment(192.0);
                let target = cur + adj.step_increment();
                // Go sets the value from a goroutine, i.e. after the click settles.
                glib::idle_add_local_once(move || adj.set_value(target));
            }
        });
        let a = adj.downgrade();
        prev.connect_clicked(move |_| {
            if let Some(adj) = a.upgrade() {
                let mut cur = adj.value();
                if cur % 192.0 > 0.0 {
                    cur += 192.0 - cur % 192.0;
                }
                adj.set_step_increment(192.0);
                adj.set_value(cur - adj.step_increment());
            }
        });
        let widget = vstack(&[hstack(&[header.upcast_ref()]).hmargin(10).mbottom(5).upcast_ref(), sw.upcast_ref()]);
        HorizontalList { widget, container, header, route }
    }
    pub fn append(&self, w: &impl IsA<gtk::Widget>) -> &Self {
        self.container.append(w);
        self
    }
    pub fn set_page_margin(&self, m: i32) -> &Self {
        self.container.set_margin_start(m);
        self.container.set_margin_end(m);
        self.header.set_margin_start(m);
        self.header.set_margin_end(m);
        self
    }
    pub fn set_view_all_route(&self, path: &str) -> &Self {
        let b = gtk::Button::new().min_height(10).min_width(10).hpadding(10);
        b.set_child(Some(&label(&tr("View All")).class("caption-heading")));
        let p = path.to_string();
        b.connect_clicked(move |_| router::navigate(&p));
        self.route.set_center_widget(Some(&b));
        self
    }
}
