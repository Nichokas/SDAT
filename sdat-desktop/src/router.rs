//! Port of `internal/router`: path based navigation with a history stack.

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Instant;

use gtk::prelude::*;
use regex::Regex;

use crate::i18n::tr;
use crate::signals::Signal;
use crate::{global_signal, settings};

pub type LocalBoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;
pub type Handler = Rc<dyn Fn(Vec<String>) -> LocalBoxFuture<Response>>;

#[derive(Default)]
pub struct Response {
    pub error: Option<anyhow::Error>,
    pub expires_at: Option<Instant>,
    pub page_title: String,
    pub toolbar: Option<gtk::Widget>,
    pub view: Option<gtk::Widget>,
}

impl Response {
    pub fn new(page_title: impl Into<String>, view: impl IsA<gtk::Widget>) -> Self {
        Response { page_title: page_title.into(), view: Some(view.upcast()), ..Default::default() }
    }
    pub fn from_error(page_title: impl Into<String>, err: anyhow::Error) -> Self {
        Response { error: Some(err), page_title: page_title.into(), ..Default::default() }
    }
    pub fn toolbar(mut self, t: impl IsA<gtk::Widget>) -> Self {
        self.toolbar = Some(t.upcast());
        self
    }
}

#[derive(Clone)]
pub struct HistoryEntry {
    pub expires_at: Option<Instant>,
    pub page_title: String,
    pub path: String,
    pub view: Option<gtk::Widget>,
    pub toolbar: Option<gtk::Widget>,
}

#[derive(Default)]
pub struct History {
    pub current: Option<HistoryEntry>,
    pub entries: Vec<HistoryEntry>,
}

pub struct NavigationEvent {
    pub completed: bool,
    pub path: String,
    pub result: Option<HistoryEntry>,
}

global_signal!(pub fn navigation() -> Signal<NavigationEvent> = Signal::default());
// Carries the number of back entries.
global_signal!(pub fn history_updated() -> Signal<usize> = Signal::default());

thread_local! {
    static ROUTES: RefCell<Vec<(Regex, Handler)>> = const { RefCell::new(Vec::new()) };
    static HISTORY: RefCell<History> = RefCell::new(History::default());
    static GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Loaded top-level pages, reused on revisit so switching tabs shows no spinner.
    static KEPT: RefCell<std::collections::HashMap<String, HistoryEntry>> = RefCell::default();
}

/// Top-level tabs whose loaded page is kept alive (widgets, data and scroll position).
pub fn is_kept_route(path: &str) -> bool {
    matches!(path, "home" | "explore" | "my-collection")
}

/// Drops the kept pages (sign in/out, region change); the next visit reloads them.
pub fn invalidate_kept_pages() {
    KEPT.with(|k| k.borrow_mut().clear());
}

/// Registers `path` (with `:arg` placeholders) to an async handler.
pub fn register<F, Fut>(path: &str, handler: F)
where
    F: Fn(Vec<String>) -> Fut + 'static,
    Fut: Future<Output = Response> + 'static,
{
    let arg = Regex::new(r"(:[^/]+)").expect("valid route-argument regex");
    let pattern = format!("^{}$", arg.replace_all(path, r"([^/]+)"));
    let h: Handler = Rc::new(move |args| Box::pin(handler(args)));
    ROUTES.with(|r| r.borrow_mut().push((Regex::new(&pattern).expect("route patterns are built from literal paths"), h)));
}

fn find_handler(path: &str) -> Option<(Handler, Vec<String>)> {
    ROUTES.with(|r| {
        for (re, h) in r.borrow().iter() {
            if let Some(c) = re.captures(path) {
                let args = c.iter().skip(1).map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default()).collect();
                return Some((std::rc::Rc::clone(h), args));
            }
        }
        None
    })
}

fn status_page(title: &str, description: &str) -> gtk::Widget {
    adw::StatusPage::builder()
        .title(title)
        .description(description)
        .icon_name("sentiment-dissatisfied-symbolic")
        .build()
        .upcast()
}

fn error_response(err: &anyhow::Error) -> Response {
    Response {
        page_title: tr("Internal Error"),
        view: Some(status_page(
            &tr("Internal Error"),
            &(tr("Unfortunately an error occurred while loading this view. Please try again later. If the error persists, please open an issue!\n\nError Message: ") + &format!("{err:#}")),
        )),
        ..Default::default()
    }
}

fn not_found_response() -> Response {
    Response {
        page_title: tr("Not Found"),
        view: Some(status_page(&tr("Not found"), &tr("The requested deeplink did not have any available handlers."))),
        ..Default::default()
    }
}

fn notify_history() {
    let n = HISTORY.with(|h| h.borrow().entries.len());
    history_updated().notify(&n);
}

fn complete(entry: HistoryEntry) {
    navigation().notify(&NavigationEvent { completed: true, path: entry.path.clone(), result: Some(entry) });
}

pub fn navigate(path: &str) {
    let path = path.strip_prefix("tidal://").unwrap_or(path);
    navigate_inner(path.to_string(), false);
}

fn navigate_inner(path: String, off_record: bool) {
    let on_same = HISTORY.with(|h| h.borrow().current.as_ref().is_some_and(|c| c.path == path));
    if on_same && !off_record {
        log::debug!("skipped navigation as we are already on the same page");
        return;
    }
    if !off_record && is_kept_route(&path)
        && let Some(entry) = KEPT.with(|k| k.borrow().get(&path).cloned())
    {
        // Already loaded once: show it instantly, no loading state.
        GENERATION.with(|g| g.set(g.get() + 1));
        push(entry.clone());
        complete(entry);
        return;
    }
    navigation().notify(&NavigationEvent { completed: false, path: path.clone(), result: None });
    let generation = GENERATION.with(|g| {
        g.set(g.get() + 1);
        g.get()
    });
    let started = Instant::now();
    let found = find_handler(&path);
    glib::spawn_future_local(async move {
        let (response, should_cache) = match found {
            None => {
                log::info!("no handler found path={path}");
                (not_found_response(), true)
            }
            Some((h, args)) => {
                let r = h(args).await;
                match &r.error {
                    Some(e) => {
                        log::error!("handler failed path={path} error={e:#}");
                        (error_response(e), false)
                    }
                    None => (r, true),
                }
            }
        };
        log::info!("navigation completed path={path} duration_ms={} should_cache={should_cache}", started.elapsed().as_millis());
        let mut entry = HistoryEntry {
            expires_at: response.expires_at,
            page_title: response.page_title,
            path: path.clone(),
            view: response.view,
            toolbar: response.toolbar,
        };
        if !should_cache {
            entry.expires_at = Some(Instant::now());
        }
        // A newer navigation superseded this one: drop the result so neither
        // history nor the kept pages hold a page (or account) the user never saw.
        if GENERATION.with(std::cell::Cell::get) != generation {
            return;
        }
        if should_cache && is_kept_route(&path) {
            KEPT.with(|k| k.borrow_mut().insert(path.clone(), entry.clone()));
        }
        if off_record {
            HISTORY.with(|h| {
                if let Some(c) = h.borrow_mut().current.as_mut().filter(|c| c.path == path) {
                    c.view.clone_from(&entry.view);
                    c.toolbar.clone_from(&entry.toolbar);
                    c.expires_at = entry.expires_at;
                }
            });
        } else {
            push(entry.clone());
        }
        complete(entry);
    });
}

fn push(entry: HistoryEntry) {
    HISTORY.with(|h| {
        let mut h = h.borrow_mut();
        if let Some(cur) = h.current.take() {
            if h.entries.len() as i32 >= settings::max_router_history_size() && !h.entries.is_empty() {
                h.entries.remove(0);
            }
            h.entries.push(cur);
        }
        h.current = Some(entry);
    });
    notify_history();
}

fn pop() -> Option<HistoryEntry> {
    let r = HISTORY.with(|h| {
        let mut h = h.borrow_mut();
        let mut prev = h.entries.pop()?;
        if prev.expires_at.is_some_and(|e| e < Instant::now()) {
            prev.view = None;
            prev.toolbar = None;
        }
        h.current = Some(prev.clone());
        Some(prev)
    });
    notify_history();
    r
}

pub fn back() {
    if HISTORY.with(|h| h.borrow().entries.is_empty()) {
        return;
    }
    let Some(mut prev) = pop() else { return };
    if prev.view.is_none()
        && let Some(kept) = KEPT.with(|k| k.borrow().get(&prev.path).cloned())
    {
        prev = kept.clone();
        HISTORY.with(|h| h.borrow_mut().current = Some(kept));
    }
    if prev.view.is_some() {
        GENERATION.with(|g| g.set(g.get() + 1));
        complete(prev);
    } else {
        navigate_inner(prev.path, true);
    }
}

pub fn refresh() {
    let Some(path) = HISTORY.with(|h| {
        let mut h = h.borrow_mut();
        let c = h.current.as_mut()?;
        c.view = None;
        c.toolbar = None;
        KEPT.with(|k| k.borrow_mut().remove(&c.path));
        Some(c.path.clone())
    }) else {
        return;
    };
    navigate_inner(path, true);
}

pub fn current() -> Option<HistoryEntry> {
    HISTORY.with(|h| h.borrow().current.clone())
}

/// Drops the Back entries (sign in/out) so the previous account's pages are unreachable.
pub fn clear_back_history() {
    HISTORY.with(|h| h.borrow_mut().entries.clear());
    notify_history();
}

pub fn clear() {
    HISTORY.with(|h| *h.borrow_mut() = History::default());
    notify_history();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_patterns() {
        let arg = Regex::new(r"(:[^/]+)").unwrap();
        let p = format!("^{}$", arg.replace_all("album/:id", r"([^/]+)"));
        let re = Regex::new(&p).unwrap();
        let c = re.captures("album/123").unwrap();
        assert_eq!(&c[1], "123");
        assert!(!re.is_match("album/123/x"));
    }
}

#[cfg(test)]
mod kept_tests {
    use super::*;
    use std::cell::Cell;

    fn pump() {
        let ctx = glib::MainContext::default();
        for _ in 0..50 {
            ctx.iteration(false);
        }
    }

    #[test]
    #[ignore = "needs a display (gtk::init)"]
    fn revisiting_a_tab_reuses_the_page_without_loading() {
        gtk::init().expect("gtk");
        let builds = Rc::new(Cell::new(0));
        for p in ["home", "explore"] {
            let b = Rc::clone(&builds);
            register(p, move |_| {
                b.set(b.get() + 1);
                async { Response::new("t", gtk::Label::new(None)) }
            });
        }
        let loading = Rc::new(Cell::new(0));
        let l = Rc::clone(&loading);
        let _h = navigation().on(move |ev| {
            if !ev.completed {
                l.set(l.get() + 1);
            }
            false
        });
        navigate("home");
        pump();
        let first = current().and_then(|c| c.view).expect("home view");
        navigate("explore");
        pump();
        navigate("home");
        pump();
        assert_eq!(builds.get(), 2);
        assert_eq!(loading.get(), 2);
        assert_eq!(current().and_then(|c| c.view), Some(first));
        invalidate_kept_pages();
        navigate("explore");
        pump();
        assert_eq!(builds.get(), 3);
    }
}
