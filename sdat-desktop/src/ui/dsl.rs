//! Small helpers replacing schwifty's fluent widget modifiers.

use gtk::prelude::*;

/// Fluent modifiers implemented for every widget (schwifty's template.go/css.go).
pub trait W: IsA<gtk::Widget> + Sized {
    fn hmargin(self, m: i32) -> Self {
        self.set_margin_start(m);
        self.set_margin_end(m);
        self
    }
    fn vmargin(self, m: i32) -> Self {
        self.set_margin_top(m);
        self.set_margin_bottom(m);
        self
    }
    fn margin(self, m: i32) -> Self {
        self.hmargin(m).vmargin(m)
    }
    fn mtop(self, m: i32) -> Self {
        self.set_margin_top(m);
        self
    }
    fn mbottom(self, m: i32) -> Self {
        self.set_margin_bottom(m);
        self
    }
    fn mstart(self, m: i32) -> Self {
        self.set_margin_start(m);
        self
    }
    fn mend(self, m: i32) -> Self {
        self.set_margin_end(m);
        self
    }
    fn class(self, c: &str) -> Self {
        self.add_css_class(c);
        self
    }
    fn hexp(self, e: bool) -> Self {
        self.set_hexpand(e);
        self
    }
    fn vexp(self, e: bool) -> Self {
        self.set_vexpand(e);
        self
    }
    fn halign(self, a: gtk::Align) -> Self {
        self.set_halign(a);
        self
    }
    fn valign(self, a: gtk::Align) -> Self {
        self.set_valign(a);
        self
    }
    fn size(self, w: i32, h: i32) -> Self {
        self.set_size_request(w, h);
        self
    }
    fn visible(self, v: bool) -> Self {
        self.set_visible(v);
        self
    }
    fn tooltip(self, t: &str) -> Self {
        self.set_tooltip_text(Some(t));
        self
    }
    fn overflow_hidden(self) -> Self {
        self.set_overflow(gtk::Overflow::Hidden);
        self
    }
    /// Per-widget CSS (schwifty CSSWithCallback): `rule` receives the css name.
    #[expect(deprecated, reason = "GTK has no non-deprecated per-widget style provider; mirrors schwifty")]
    fn css_with(self, rule: impl FnOnce(&str) -> String) -> Self {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&rule(self.css_name().as_str()));
        self.style_context().add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        self
    }
    fn css(self, css: &str) -> Self {
        let css = css.to_string();
        self.css_with(move |_| css)
    }
    fn padding(self, p: i32) -> Self {
        self.css_with(|n| format!("{n} {{ padding: {p}px; }}"))
    }
    fn hpadding(self, p: i32) -> Self {
        self.css_with(|n| format!("{n} {{ padding-left: {p}px; padding-right: {p}px; }}"))
    }
    fn vpadding(self, p: i32) -> Self {
        self.css_with(|n| format!("{n} {{ padding-bottom: {p}px; padding-top: {p}px; }}"))
    }
    fn pstart(self, p: i32) -> Self {
        self.css_with(|n| format!("{n} {{ padding-left: {p}px; }}"))
    }
    fn pend(self, p: i32) -> Self {
        self.css_with(|n| format!("{n} {{ padding-right: {p}px; }}"))
    }
    fn ptop(self, p: i32) -> Self {
        self.css_with(|n| format!("{n} {{ padding-top: {p}px; }}"))
    }
    fn pbottom(self, p: i32) -> Self {
        self.css_with(|n| format!("{n} {{ padding-bottom: {p}px; }}"))
    }
    fn min_height(self, h: i32) -> Self {
        self.css_with(|n| format!("{n} {{ min-height: {h}px; }}"))
    }
    fn min_width(self, w: i32) -> Self {
        self.css_with(|n| format!("{n} {{ min-width: {w}px; }}"))
    }
    fn corner_radius(self, r: i32) -> Self {
        self.css_with(|n| format!("{n} {{ border-radius: {r}px; }}"))
    }
    fn background(self, c: &str) -> Self {
        self.css_with(|n| format!("{n} {{ background-color: {c}; }}"))
    }
    fn font_weight(self, w: i32) -> Self {
        self.css_with(|n| format!("{n} {{ font-weight: {w}; }}"))
    }
}

impl<T: IsA<gtk::Widget>> W for T {}

pub fn hstack(children: &[&gtk::Widget]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    for c in children {
        b.append(*c);
    }
    b
}

pub fn vstack(children: &[&gtk::Widget]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    for c in children {
        b.append(*c);
    }
    b
}

pub fn spacing(b: gtk::Box, s: i32) -> gtk::Box {
    b.set_spacing(s);
    b
}

pub fn spacer() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, 0).hexp(true).vexp(true)
}

pub fn label(t: &str) -> gtk::Label {
    gtk::Label::new(Some(t))
}


pub fn clamp(max: i32, child: &impl IsA<gtk::Widget>) -> adw::Clamp {
    let c = adw::Clamp::new();
    c.set_maximum_size(max);
    c.set_child(Some(child));
    c
}


/// Weak-ref helper: `with_weak!(widget, |w| ...)` inside a closure.
#[macro_export]
macro_rules! weak {
    ($w:expr) => {
        glib::clone::Downgrade::downgrade(&$w)
    };
}

use crate::signals::{Signal, State, CONTINUE, UNSUBSCRIBE};

/// Subscribes to a global state for as long as `obj` is alive (calls immediately).
pub fn on_state<T: Clone + 'static, O: IsA<glib::Object>>(st: &'static State<T>, obj: &O, f: impl Fn(&O, &T) + 'static) {
    let w = obj.downgrade();
    st.on(move |v| match w.upgrade() {
        Some(o) => {
            f(&o, v);
            CONTINUE
        }
        None => UNSUBSCRIBE,
    });
}

/// Like [`on_state`] but without the initial call.
pub fn on_state_lazy<T: Clone + 'static, O: IsA<glib::Object>>(st: &'static State<T>, obj: &O, f: impl Fn(&O, &T) + 'static) {
    let w = obj.downgrade();
    st.on_lazy(move |v| match w.upgrade() {
        Some(o) => {
            f(&o, v);
            CONTINUE
        }
        None => UNSUBSCRIBE,
    });
}

/// Subscribes to a stateless global signal for as long as `obj` is alive.
pub fn on_signal<T: 'static, O: IsA<glib::Object>>(sig: &'static Signal<T>, obj: &O, f: impl Fn(&O, &T) + 'static) {
    let w = obj.downgrade();
    sig.on(move |v| match w.upgrade() {
        Some(o) => {
            f(&o, v);
            CONTINUE
        }
        None => UNSUBSCRIBE,
    });
}
