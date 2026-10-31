//! Port of `components/linking`: the QR device-linking dialog.
use std::rc::Rc;

use adw::prelude::*;
use gtk::gdk;

use crate::i18n::tr;
use crate::imgutil;
use crate::ui::dsl::{W, label, spacing, vstack};

const MODULE: usize = 20;
const QUIET: usize = 2;

/// Is (x, y) inside one of the three finder patterns?
fn in_finder(x: usize, y: usize, w: usize) -> bool {
    (y < 7 && (x < 7 || x >= w - 7)) || (x < 7 && y >= w - 7)
}

/// Rounded-rectangle hit test in module units.
fn in_rounded(px: f64, py: f64, x0: f64, y0: f64, size: f64, r: f64) -> bool {
    if px < x0 || py < y0 || px >= x0 + size || py >= y0 + size {
        return false;
    }
    let cx = px.clamp(x0 + r, x0 + size - r);
    let cy = py.clamp(y0 + r, y0 + size - r);
    (px - cx).powi(2) + (py - cy).powi(2) <= r * r
}

/// Renders a QR code with rounded finders and liquid (rounded) blocks,
/// approximating go-qrcode's `RoundedFinder` + `LiquidBlock` shapes.
pub fn render_qr(data: &str) -> Option<gdk::Texture> {
    let code = qrcode::QrCode::new(data.as_bytes()).ok()?;
    let w = code.width();
    let colors = code.to_colors();
    let dark = |x: usize, y: usize| colors[y * w + x] == qrcode::Color::Dark;
    let size = (w + QUIET * 2) * MODULE;
    let mut buf = vec![255u8; size * size * 4];
    for py in 0..size {
        for px in 0..size {
            let mx = px as f64 / MODULE as f64 - QUIET as f64;
            let my = py as f64 / MODULE as f64 - QUIET as f64;
            if mx < 0.0 || my < 0.0 || mx >= w as f64 || my >= w as f64 {
                continue;
            }
            let (ix, iy) = (mx as usize, my as usize);
            let on = if in_finder(ix, iy, w) {
                let (ox, oy) = (if ix < 7 { 0.0 } else { (w - 7) as f64 }, if iy < 7 { 0.0 } else { (w - 7) as f64 });
                let (lx, ly) = (mx - ox, my - oy);
                (in_rounded(lx, ly, 0.0, 0.0, 7.0, 2.0) && !in_rounded(lx, ly, 1.0, 1.0, 5.0, 1.4)) || in_rounded(lx, ly, 2.0, 2.0, 3.0, 1.0)
            } else if dark(ix, iy) {
                // Round only corners that have no dark neighbours on both adjacent sides.
                let (fx, fy) = (mx - ix as f64, my - iy as f64);
                let n = |dx: i64, dy: i64| {
                    let (x, y) = (ix as i64 + dx, iy as i64 + dy);
                    x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < w && !in_finder(x as usize, y as usize, w) && dark(x as usize, y as usize)
                };
                let r = 0.5;
                let corner = |cx: f64, cy: f64, sx: i64, sy: i64| {
                    if n(sx, 0) || n(0, sy) {
                        return true;
                    }
                    (fx - cx).powi(2) + (fy - cy).powi(2) <= r * r
                };
                if fx < r && fy < r {
                    corner(r, r, -1, -1)
                } else if fx >= 1.0 - r && fy < r {
                    corner(1.0 - r, r, 1, -1)
                } else if fx < r && fy >= 1.0 - r {
                    corner(r, 1.0 - r, -1, 1)
                } else if fx >= 1.0 - r && fy >= 1.0 - r {
                    corner(1.0 - r, 1.0 - r, 1, 1)
                } else {
                    true
                }
            } else {
                false
            };
            if on {
                let o = (py * size + px) * 4;
                buf[o..o + 3].fill(0);
            }
        }
    }
    let bytes = glib::Bytes::from_owned(buf);
    Some(gdk::MemoryTexture::new(size as i32, size as i32, gdk::MemoryFormat::R8g8b8a8, &bytes, size * 4).upcast())
}

pub fn new_linking(window: Option<gtk::Window>, code: &str, link: &str, cancel: Rc<dyn Fn()>) -> adw::AlertDialog {
    let link = if link.contains("://") { link.to_string() } else { format!("https://{link}") };
    let qr = gtk::Image::new();
    qr.set_pixel_size(186);
    if let Some(t) = render_qr(&link) { qr.set_paintable(Some(&t)) } else {
        log::error!("could not generate QR code to sign in");
        qr.set_paintable(Some(&imgutil::missing_album()));
    }
    let frame = gtk::AspectFrame::new(0.5, 0.5, 1.0, true);
    frame.set_child(Some(&qr));
    let frame = frame.background("alpha(var(--view-fg-color), 0.1)").halign(gtk::Align::Center).corner_radius(10).overflow_hidden();
    let spaced: Vec<String> = code.chars().map(|c| c.to_string()).collect();
    let code_l = label(&spaced.join("  ")).class("title-1");
    let helper = label(&tr("You can also open the linking page using the button below"));
    let open = gtk::Button::with_label(&tr("Open TIDAL Page")).class("suggested-action").hpadding(20).vpadding(10);
    let l2 = link;
    open.connect_clicked(move |_| {
        gtk::UriLauncher::new(&l2).launch(window.as_ref(), gtk::gio::Cancellable::NONE, |_| {});
    });
    let cancel_b = gtk::Button::with_label(&tr("Cancel Login")).hpadding(20).vpadding(10);
    let c = Rc::clone(&cancel);
    cancel_b.connect_clicked(move |_| c());
    let body = spacing(
        vstack(&[frame.upcast_ref(), code_l.upcast_ref(), helper.upcast_ref(), spacing(vstack(&[open.upcast_ref(), cancel_b.upcast_ref()]), 10).upcast_ref()]),
        20,
    );
    let d = adw::AlertDialog::new(Some(&tr("Sign In")), Some(&tr("Scan this QR code to sign into your TIDAL account")));
    d.add_css_class("no-response");
    d.set_can_close(false);
    d.connect_close_attempt(move |_| cancel());
    d.set_extra_child(Some(&body));
    d
}

#[cfg(test)]
mod tests {
    #[test]
    fn rounded_hit() {
        assert!(super::in_rounded(3.5, 3.5, 0.0, 0.0, 7.0, 2.0));
        assert!(!super::in_rounded(0.05, 0.05, 0.0, 0.0, 7.0, 2.0));
    }
}
