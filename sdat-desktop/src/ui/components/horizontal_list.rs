//! Port of `components/horizontal_list`.
use adw::prelude::*;
use gtk::pango;

use std::cell::Cell;
use std::rc::Rc;

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
        let scroller = Rc::new(SmoothScroll::new(&sw));
        let sc = Rc::clone(&scroller);
        next.connect_clicked(move |_| sc.step(1.0));
        prev.connect_clicked(move |_| scroller.step(-1.0));
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

/// Card width the arrows scroll by (same step as Go).
const STEP: f64 = 192.0;
const SCROLL_MS: u32 = 280;

/// Go's arrow step: snap to the card grid, then move one card in `dir` (±1).
fn step_target(base: f64, dir: f64) -> f64 {
    let rem = base % STEP;
    if dir > 0.0 {
        base - rem + STEP
    } else {
        let snapped = if rem > 0.0 { base + STEP - rem } else { base };
        snapped - STEP
    }
}

/// Animates the carousel's horizontal adjustment towards the next card. Repeated
/// clicks retarget from where the previous click was heading. Adwaita skips the
/// animation when the system has animations disabled.
struct SmoothScroll {
    adj: gtk::Adjustment,
    anim: adw::TimedAnimation,
    target: Cell<Option<f64>>,
}

impl SmoothScroll {
    fn new(sw: &gtk::ScrolledWindow) -> Self {
        let adj = sw.hadjustment();
        let target = adw::PropertyAnimationTarget::new(&adj, "value");
        let anim = adw::TimedAnimation::new(sw, 0.0, 0.0, SCROLL_MS, target);
        anim.set_easing(adw::Easing::EaseOutCubic);
        SmoothScroll { adj, anim, target: Cell::new(None) }
    }

    fn step(&self, dir: f64) {
        let running = self.anim.state() == adw::AnimationState::Playing;
        let base = self.target.get().filter(|_| running).unwrap_or_else(|| self.adj.value());
        let max = (self.adj.upper() - self.adj.page_size()).max(self.adj.lower());
        let to = step_target(base, dir).clamp(self.adj.lower(), max);
        self.target.set(Some(to));
        self.anim.pause();
        self.anim.set_value_from(self.adj.value());
        self.anim.set_value_to(to);
        self.anim.play();
    }
}

#[cfg(test)]
mod tests {
    use super::step_target;

    #[test]
    fn steps_match_go() {
        assert!((step_target(0.0, 1.0) - 192.0).abs() < f64::EPSILON);
        assert!((step_target(100.0, 1.0) - 192.0).abs() < f64::EPSILON);
        assert!((step_target(384.0, -1.0) - 192.0).abs() < f64::EPSILON);
        assert!((step_target(300.0, -1.0) - 192.0).abs() < f64::EPSILON);
        // Retargeting from a pending target keeps whole-card steps.
        assert!((step_target(step_target(0.0, 1.0), 1.0) - 384.0).abs() < f64::EPSILON);
    }
}
