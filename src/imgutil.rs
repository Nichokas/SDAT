//! Port of `pkg/utils/imgutil`: image fetching with a 10 minute TTL cache
//! (about 50 MiB) and square cropping.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{gdk, glib};

use crate::{api, settings};

const TTL: Duration = Duration::from_secs(10 * 60);
const MAX_COST: u64 = 50 * 1024 * 1024;

#[derive(Default)]
struct Cache {
    items: HashMap<String, (gdk::Texture, Instant)>,
    pending: HashMap<String, Vec<async_channel::Sender<Option<gdk::Texture>>>>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

fn cost(t: &gdk::Texture) -> u64 {
    t.width() as u64 * t.height() as u64 * 4
}

fn insert(url: &str, t: &gdk::Texture) {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let now = Instant::now();
        c.items.retain(|_, (_, at)| now.duration_since(*at) < TTL);
        c.items.insert(url.to_string(), (t.clone(), now));
        let mut total: u64 = c.items.values().map(|(t, _)| cost(t)).sum();
        while total > MAX_COST {
            let Some(oldest) = c.items.iter().min_by_key(|(_, (_, at))| *at).map(|(k, _)| k.clone()) else { break };
            if let Some((t, _)) = c.items.remove(&oldest) {
                total -= cost(&t);
            }
        }
    });
}

fn fetch_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let resp = reqwest::blocking::get(url)?;
    if !resp.status().is_success() {
        anyhow::bail!("failed to fetch image: {}", resp.status());
    }
    Ok(resp.bytes()?.to_vec())
}

/// Fetches (or returns the cached) texture for `url`.
pub async fn fetch(url: &str) -> Option<gdk::Texture> {
    if url.is_empty() {
        return None;
    }
    let cached = CACHE.with(|c| {
        let c = c.borrow();
        c.items.get(url).filter(|(_, at)| at.elapsed() < TTL).map(|(t, _)| t.clone())
    });
    if cached.is_some() {
        return cached;
    }
    // Deduplicate concurrent loads of the same URL.
    let waiter = CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(list) = c.pending.get_mut(url) {
            let (tx, rx) = async_channel::bounded(1);
            list.push(tx);
            Some(rx)
        } else {
            c.pending.insert(url.to_string(), vec![]);
            None
        }
    });
    if let Some(rx) = waiter {
        return rx.recv().await.ok().flatten();
    }
    let u = url.to_string();
    let bytes = api::run(move || fetch_bytes(&u)).await;
    let texture = match bytes {
        Ok(b) => gdk::Texture::from_bytes(&glib::Bytes::from_owned(b)).map_err(|e| log::debug!("failed to decode image {url}: {e}")).ok(),
        Err(e) => {
            log::debug!("failed to fetch image {url}: {e:#}");
            None
        }
    };
    if let Some(t) = &texture {
        if settings::should_cache_images() {
            insert(url, t);
        }
    }
    let waiters = CACHE.with(|c| c.borrow_mut().pending.remove(url)).unwrap_or_default();
    for w in waiters {
        let _ = w.try_send(texture.clone());
    }
    texture
}

/// Crops a texture to a centred square.
pub fn crop(texture: &gdk::Texture) -> gdk::Texture {
    let (w, h) = (texture.width(), texture.height());
    if w == h {
        return texture.clone();
    }
    let size = w.min(h);
    let (x, y) = ((w - size) / 2, (h - size) / 2);
    let mut dl = gdk::TextureDownloader::new(texture);
    dl.set_format(gdk::MemoryFormat::R8g8b8a8);
    let (bytes, stride) = dl.download_bytes();
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for row in 0..size {
        let start = (y + row) as usize * stride + x as usize * 4;
        out.extend_from_slice(&bytes[start..start + size as usize * 4]);
    }
    gdk::MemoryTexture::new(size, size, gdk::MemoryFormat::R8g8b8a8, &glib::Bytes::from_owned(out), size as usize * 4).upcast()
}

pub async fn fetch_cropped(url: &str) -> Option<gdk::Texture> {
    fetch(url).await.map(|t| crop(&t))
}

/// Loads `url` into `image` asynchronously (weakly referenced).
pub fn load_into_image(url: &str, image: &gtk::Image, cropped: bool) {
    let url = url.to_string();
    let weak = image.downgrade();
    glib::spawn_future_local(async move {
        let t = if cropped { fetch_cropped(&url).await } else { fetch(&url).await };
        if let (Some(t), Some(img)) = (t, weak.upgrade()) {
            img.set_paintable(Some(&t));
        }
    });
}

thread_local! {
    static MISSING: RefCell<Option<gdk::Texture>> = const { RefCell::new(None) };
}

/// resources.MissingAlbum
pub fn missing_album() -> gdk::Paintable {
    MISSING.with(|m| {
        m.borrow_mut()
            .get_or_insert_with(|| gdk::Texture::from_resource("/dev/dergs/Tonearm/icons/hicolor/512x512/state/missing-album.png"))
            .clone()
            .upcast()
    })
}
