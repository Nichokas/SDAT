//! Port of `internal/ui/routes` and `internal/ui/pages`.
use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use chrono::{DateTime, Datelike, Duration as CDuration, Local, NaiveTime};

use crate::i18n::{tr, trf};
use crate::model::{self, Track, PlaybackSource};
use crate::router::{self, Response};
use crate::service::{self, Paginator, SharedPaginator};
use crate::cache;
use crate::ui::components::horizontal_list::HorizontalList;
use crate::ui::components::tracklist::{Col, TrackList};
use crate::ui::components::{self as comp, RowTitle, media_card, tracklist_header};
use crate::ui::dsl::{W, vstack, label, spacing, hstack, clamp, spacer};
use crate::{api, imgutil, player, secrets};

pub fn register() {
    router::register("home", |_| home());
    router::register("explore", |_| explore("explore".into()));
    router::register("explore/:page", |a| explore(a[0].clone()));
    router::register("album/:id", |a| album(a[0].clone()));
    router::register("artist/:id", |a| artist(a[0].clone()));
    router::register("playlist/:id", |a| playlist(a[0].clone()));
    router::register("feed", |_| feed());
    router::register("search", |_| async { search() });
    router::register("search/:query/albums", |a| search_albums(a[0].clone()));
    router::register("search/:query/artists", |a| search_artists(a[0].clone()));
    router::register("search/:query/playlists", |a| search_playlists(a[0].clone()));
    router::register("search/:query/tracks", |a| search_tracks(a[0].clone()));
    router::register("my-collection", |_| my_collection());
    router::register("my-collection/albums", |_| my_albums());
    router::register("my-collection/artists", |_| my_artists());
    router::register("my-collection/playlists", |_| my_playlists());
    router::register("my-collection/tracks", |_| my_tracks());
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let s = gtk::ScrolledWindow::new();
    s.set_child(Some(child));
    // External: still scrolls (wheel, touchpad, keyboard, kinetic) and keeps the
    // vadjustment working for scroll-to-load, but draws no scrollbar/indicator.
    s.set_policy(gtk::PolicyType::Never, gtk::PolicyType::External);
    s
}

fn toast(msg: &str) {
    player::toasts().notify(&msg.to_string());
}

/// Spawns a player future, toasting and logging on failure.
fn play(fut: impl std::future::Future<Output = anyhow::Result<()>> + 'static, toast_msg: String, log_msg: &'static str) {
    glib::spawn_future_local(async move {
        if let Err(e) = fut.await {
            toast(&toast_msg);
            log::error!("{log_msg} error={e:#}");
        }
    });
}

// ------------------------------------------------------------------ pages

/// How many viewport heights ahead of the end we keep loaded. Go loaded at 80%
/// of the content, which left the spinner visible on long lists.
const PREFETCH_SCREENS: f64 = 3.0;

/// Whether to fetch another page: one request at a time, never after the end,
/// and whenever less than `PREFETCH_SCREENS` viewports remain below the view
/// (which also covers content that doesn't fill the viewport yet).
fn should_fetch(busy: bool, done: bool, value: f64, page: f64, upper: f64) -> bool {
    !busy && !done && upper - (value + page) <= PREFETCH_SCREENS * page.max(1.0)
}

/// Port of ConnectReachEdgeSoon(PosBottom), prefetching ahead. Calls `more` with a
/// completion callback (true = more pages may exist). After each page it checks
/// again, so pages chain until the buffer is filled.
fn on_reach_bottom_soon(sw: &gtk::ScrolledWindow, more: impl Fn(Rc<dyn Fn(bool)>) + 'static) {
    let adj = sw.vadjustment();
    let busy = Rc::new(Cell::new(false));
    let done = Rc::new(Cell::new(false));
    let more = Rc::new(more);
    let check = {
        let (busy, done) = (Rc::clone(&busy), Rc::clone(&done));
        Rc::new(move |adj: &gtk::Adjustment| {
            if !should_fetch(busy.get(), done.get(), adj.value(), adj.page_size(), adj.upper()) {
                return;
            }
            busy.set(true);
            let (busy, done, adj) = (Rc::clone(&busy), Rc::clone(&done), adj.clone());
            more(Rc::new(move |has_more| {
                busy.set(false);
                if has_more {
                    // Chain: new rows change `upper`, which re-runs the check; also
                    // nudge it in case the layout didn't change.
                    adj.emit_by_name::<()>("changed", &[]);
                } else {
                    done.set(true);
                }
            }));
        })
    };
    let c = Rc::clone(&check);
    adj.connect_value_changed(move |a| c(a));
    let c = Rc::clone(&check);
    adj.connect_changed(move |a| c(a));
    // Start prefetching right away, not only once the user scrolls.
    glib::idle_add_local_once(move || check(&adj));
}

/// After a failed page load, allow another attempt in a few seconds instead of
/// retrying in a tight loop or giving up for good.
fn retry_later(finish: Rc<dyn Fn(bool)>) {
    glib::timeout_add_local_once(std::time::Duration::from_secs(5), move || finish(true));
}

#[cfg(test)]
mod pagination_tests {
    use super::should_fetch;

    #[test]
    fn fetches_only_when_close_to_the_end() {
        // 600px viewport at the top of 10000px of content: far from the end.
        assert!(!should_fetch(false, false, 0.0, 600.0, 10_000.0));
        // Within three screens of the end: prefetch.
        assert!(should_fetch(false, false, 7_700.0, 600.0, 10_000.0));
        // Content shorter than the viewport.
        assert!(should_fetch(false, false, 0.0, 600.0, 300.0));
        // Just opened, first page fills 2 screens: prefetch the next one.
        assert!(should_fetch(false, false, 0.0, 600.0, 1_200.0));
    }

    #[test]
    fn single_flight_and_stops_at_end() {
        assert!(!should_fetch(true, false, 0.0, 600.0, 300.0));
        assert!(!should_fetch(false, true, 0.0, 600.0, 300.0));
    }

    #[test]
    fn unrealized_viewport_still_fetches_small_content() {
        assert!(should_fetch(false, false, 0.0, 0.0, 0.0));
    }
}

/// Port of `NewPaginatedMediaCardPage`.
async fn paginated_media_card_page<T: Clone + Send + 'static>(p: Paginator<T>, factory: fn(&T) -> gtk::Button) -> anyhow::Result<gtk::Widget> {
    let p = service::shared(p);
    let first = service::next_page(&p).await?;
    let list = adw::WrapBox::new().vmargin(20).hmargin(40).valign(gtk::Align::Start);
    list.set_justify(adw::JustifyMode::Fill);
    list.set_justify_last_line(true);
    let add = move |list: &adw::WrapBox, items: &[T]| {
        for i in items {
            let c = gtk::CenterBox::new();
            c.set_center_widget(Some(&factory(i)));
            list.append(&c);
        }
    };
    // Later pages are appended a few cards per main-loop iteration to avoid jank.
    let add_batched = move |list: &adw::WrapBox, items: Vec<T>, finish: Rc<dyn Fn(bool)>| {
        const BATCH: usize = 12;
        let lw = list.downgrade();
        let mut rest = items.into_iter();
        glib::idle_add_local(move || {
            let chunk: Vec<T> = rest.by_ref().take(BATCH).collect();
            match lw.upgrade() {
                Some(l) if !chunk.is_empty() => {
                    add(&l, &chunk);
                    glib::ControlFlow::Continue
                }
                _ => {
                    finish(true);
                    glib::ControlFlow::Break
                }
            }
        });
    };
    add(&list, &first);
    let sw = scrolled(&comp::main_content(&list));
    let lw = list.downgrade();
    on_reach_bottom_soon(&sw, move |finish| {
        let p: SharedPaginator<T> = std::sync::Arc::clone(&p);
        let lw = lw.clone();
        glib::spawn_future_local(async move {
            if p.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_consumed() {
                log::debug!("No more items to fetch");
                return finish(false);
            }
            match service::next_page(&p).await {
                Ok(items) => match lw.upgrade() {
                    Some(l) => add_batched(&l, items, finish),
                    None => finish(false),
                },
                Err(e) => {
                    log::error!("loading next page failed error={e:#}");
                    retry_later(finish);
                }
            }
        });
    });
    Ok(sw.upcast())
}

/// Port of `NewPaginatedTracklistPage`. Returns the scrolled window and the list.
///
/// With a `cache` job, a cached list shows instantly and is revalidated in the
/// background; otherwise pages load with foreground priority and the complete
/// list is cached once the last page arrives.
/// Appends the paginator's items the list doesn't show yet. Idempotent, so the
/// scroll loader and a click-triggered full fetch can't duplicate rows.
fn sync_from_paginator(list: &TrackList, p: &service::SharedPaginator<Track>) {
    let guard = p.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    for t in guard.items().iter().skip(list.len()).cloned() {
        list.add_track(t);
    }
}

/// Clicking a row plays the list from it right away with what is loaded; more
/// pages are fetched lazily (off the main thread) as the queue runs low and
/// appended to both the list and the play queue.
fn play_whole_list_on_click(list: &TrackList, p: Option<SharedPaginator<Track>>, source: PlaybackSource) {
    list.set_list_click_handler(move |l, t, pos| {
        if let Some(p) = &p {
            sync_from_paginator(l, p);
        }
        let Some(generation) = player::play_list_from(source.clone(), l.tracks(), t, pos) else { return };
        if let Some(p) = &p {
            player::set_list_feeder(generation, page_feeder(Some(l.downgrade()), std::sync::Arc::clone(p), generation));
        }
    });
}

/// A [`player::ListFeeder`] that loads the next page of `p`, shows it in the
/// list (if still open) and appends it to the queue of list `generation`.
fn page_feeder(list: Option<crate::ui::components::tracklist::WeakTrackList>, p: SharedPaginator<Track>, generation: u64) -> player::ListFeeder {
    Rc::new(move |done| {
        let (list, p) = (list.clone(), std::sync::Arc::clone(&p));
        glib::spawn_future_local(async move {
            if p.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_consumed() {
                return done(false);
            }
            let pp = std::sync::Arc::clone(&p);
            let page = api::run(move || pp.lock().unwrap_or_else(std::sync::PoisonError::into_inner).next_page()).await;
            let page = match page {
                Ok(page) => page,
                Err(e) => {
                    log::error!("loading more of the playing list failed, keeping what we have error={e:#}");
                    return done(false);
                }
            };
            if let Some(l) = list.as_ref().and_then(crate::ui::components::tracklist::WeakTrackList::upgrade) {
                sync_from_paginator(&l, &p);
            }
            let alive = player::extend_list(generation, &page);
            done(alive && !p.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_consumed());
        });
    })
}

async fn paginated_tracklist_page(p: Paginator<Track>, columns: Vec<Col>, margin: i32, cache: Option<cache::Job>, source: Option<PlaybackSource>) -> anyhow::Result<(gtk::ScrolledWindow, TrackList)> {
    let key = cache.as_ref().map(cache::Job::key);
    if let (Some(job), Some(cached)) = (cache, key.as_deref().and_then(cache::get)) {
        let list = TrackList::new(columns);
        for t in cached.iter().cloned() {
            list.add_track(t);
        }
        if let Some(s) = source {
            play_whole_list_on_click(&list, None, s);
        }
        let sw = tracklist_scroller(&list, margin);
        revalidate(job, cached, &list);
        return Ok((sw, list));
    }
    let generation_at_start = key.as_deref().map(cache::generation);
    let p = service::shared(p);
    let first = {
        let _fg = cache::Foreground::begin();
        service::next_page(&p).await?
    };
    let list = TrackList::new(columns);
    for t in first {
        list.add_track(t);
    }
    if let Some(s) = source {
        play_whole_list_on_click(&list, Some(std::sync::Arc::clone(&p)), s);
    }
    let sw = tracklist_scroller(&list, margin);
    let l = list.clone();
    on_reach_bottom_soon(&sw, move |finish| {
        let (p, l, key) = (std::sync::Arc::clone(&p), l.clone(), key.clone());
        glib::spawn_future_local(async move {
            if p.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_consumed() {
                log::debug!("No more items to fetch");
                return finish(false);
            }
            let fg = cache::Foreground::begin();
            let page = service::next_page(&p).await;
            drop(fg);
            match page {
                Ok(_) => {
                    sync_from_paginator(&l, &p);
                    let guard = p.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    if guard.is_consumed()
                        && let (Some(k), Some(g)) = (key.as_deref(), generation_at_start)
                    {
                        cache::put(k, g, guard.items().to_vec());
                    }
                    drop(guard);
                    finish(true);
                }
                Err(e) => {
                    log::error!("loading next page failed error={e:#}");
                    retry_later(finish);
                }
            }
        });
    });
    Ok((sw, list))
}

fn tracklist_scroller(list: &TrackList, margin: i32) -> gtk::ScrolledWindow {
    list.widget().add_css_class("page");
    list.widget().set_margin_start(margin);
    list.widget().set_margin_end(margin);
    list.widget().set_valign(gtk::Align::Start);
    scrolled(&comp::main_content_scrollable(list.widget()))
}

/// Refetches a cached list in the background and updates the page if it changed.
fn revalidate(job: cache::Job, cached: Vec<Track>, list: &TrackList) {
    let l = list.clone();
    glib::spawn_future_local(async move {
        let key = job.key();
        let g = cache::generation(&key);
        let mut p = job.paginator();
        match api::run(move || p.get_all()).await {
            Ok(fresh) => {
                if !cache::same_ids(&cached, &fresh) {
                    l.replace_all(&fresh);
                }
                cache::put(&key, g, fresh);
            }
            Err(e) => log::debug!("revalidating {key} failed error={e:#}"),
        }
    });
}

fn std_columns() -> Vec<Col> {
    vec![Col::cover(), Col::title_album(), Col::artists(), Col::duration(), Col::controls()]
}

// ------------------------------------------------------------------ home / explore

async fn home() -> Response {
    let feed = match api::run(api::home_feed_static).await {
        Ok(f) => f,
        Err(e) => return Response::from_error(tr("Home"), e),
    };
    let body = gtk::Box::new(gtk::Orientation::Vertical, 25).vmargin(20);
    for item in &feed.items {
        body.append(&comp::for_page_item(item));
    }
    if !crate::secrets::user_id().is_empty() {
        cache::start_prefetch();
    }
    Response::new(tr("Home"), scrolled(&comp::main_content(&body)))
}

async fn explore(name: String) -> Response {
    let page = match api::run(move || api::page_v1(&name)).await {
        Ok(p) => p,
        Err(e) => return Response::from_error(tr("Explore"), e),
    };
    let body = gtk::Box::new(gtk::Orientation::Vertical, 25).vmargin(20);
    for row in &page.rows {
        for m in &row.modules {
            body.append(&vstack(&[&comp::for_module(m)]));
        }
    }
    Response::new(page.title, scrolled(&comp::main_content(&body)))
}

// ------------------------------------------------------------------ album / playlist / artist

async fn album(id: String) -> Response {
    let i = id.clone();
    let a = match api::run(move || service::get_album(&i)).await {
        Ok(a) => a,
        Err(e) => return Response::from_error(tr("Album"), e),
    };
    let i = id.clone();
    let (page, list) = match paginated_tracklist_page(
        service::get_album_tracks(&i),
        vec![Col::position(), Col::title(), Col::artists(), Col::duration(), Col::controls()],
        30,
        Some(cache::Job::Album(id.clone())),
        None,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return Response::from_error(a.info.title.clone(), e),
    };
    let i = id.clone();
    list.set_click_handler(move |_, pos| {
        let i = i.clone();
        play(async move { player::play_album(&i, false, pos).await }, tr("An error occurred while playing the track"), "An error occurred while playing the album");
    });
    let (i1, i2) = (id.clone(), id);
    let header = tracklist_header::album(
        &a,
        move || {
            let i = i1.clone();
            play(async move { player::play_album(&i, false, 0).await }, tr("An error occurred while playing the album"), "An error occurred while playing the album");
        },
        move || {
            let i = i2.clone();
            play(async move { player::play_album(&i, true, 0).await }, tr("An error occurred while playing the album"), "An error occurred while playing the album");
        },
    )
    .hmargin(40);
    page.set_vexpand(true);
    page.set_margin_top(20);
    Response::new(a.info.title.clone(), vstack(&[&comp::main_content(&header), page.upcast_ref()]))
}

async fn playlist(id: String) -> Response {
    let i = id.clone();
    let p = match api::run(move || service::get_playlist(&i)).await {
        Ok(p) => p,
        Err(e) => return Response::from_error(tr("Playlist"), e),
    };
    let title = p.info.title.clone();
    let (page, list) = match paginated_tracklist_page(service::get_playlist_tracks(&id), std_columns(), 30, Some(cache::Job::Playlist(id.clone())), None).await {
        Ok(r) => r,
        Err(e) => return Response::from_error(title, e),
    };
    let i = id.clone();
    list.set_click_handler(move |_, pos| {
        let i = i.clone();
        play(async move { player::play_playlist(&i, false, pos).await }, tr("An error occurred while playing the track"), "An error occurred while playing the playlist");
    });
    let (i1, i2) = (id.clone(), id);
    let header = tracklist_header::playlist(
        &p,
        move || {
            let i = i1.clone();
            play(async move { player::play_playlist(&i, false, 0).await }, tr("An error occurred while playing the playlist"), "An error occurred while playing the playlist");
        },
        move || {
            let i = i2.clone();
            play(async move { player::play_playlist(&i, true, 0).await }, tr("An error occurred while playing the playlist"), "An error occurred while playing the playlist");
        },
    )
    .hmargin(40);
    page.set_vexpand(true);
    page.set_margin_top(20);
    Response::new(title, vstack(&[&comp::main_content(&header), page.upcast_ref()]))
}

async fn artist(id: String) -> Response {
    let i = id.clone();
    let r = api::run(move || Ok((service::get_artist(&i)?, api::artist_page(&i)?))).await;
    let (a, page) = match r {
        Ok(v) => v,
        Err(e) => return Response::from_error(tr("Artist"), e),
    };
    let body = gtk::Box::new(gtk::Orientation::Vertical, 25).mtop(20).pbottom(20);
    for item in &page.items {
        body.append(&comp::for_page_item(item));
    }
    let (i1, i2) = (id.clone(), id);
    let header = tracklist_header::artist(
        &a,
        move || {
            let i = i1.clone();
            play(async move { player::play_artist_top_songs(&i, false, 0).await }, tr("An error occurred while playing the top tracks"), "An error occurred while playing the top tracks");
        },
        move || {
            let i = i2.clone();
            play(async move { player::play_artist_top_songs(&i, true, 0).await }, tr("An error occurred while playing the top tracks"), "An error occurred while playing the top tracks");
        },
    )
    .hmargin(40);
    let sw = scrolled(&comp::main_content(&body)).vexp(true).mtop(20);
    Response::new(tr("Artist"), vstack(&[&comp::main_content(&header), sw.upcast_ref()]))
}

// ------------------------------------------------------------------ feed

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum TimeStage {
    Today,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    Older,
}

impl TimeStage {
    fn next(self) -> Self {
        use TimeStage::{Today, ThisWeek, LastWeek, ThisMonth, LastMonth, Older};
        match self {
            Today => ThisWeek,
            ThisWeek => LastWeek,
            LastWeek => ThisMonth,
            ThisMonth => LastMonth,
            LastMonth | Older => Older,
        }
    }
    /// Same thresholds as Go (today is truncated to UTC midnight like time.Truncate).
    pub fn threshold(self, now: DateTime<Local>) -> Option<DateTime<chrono::Utc>> {
        use TimeStage::{Today, ThisWeek, LastWeek, ThisMonth, LastMonth, Older};
        let today = now.with_timezone(&chrono::Utc).date_naive().and_time(NaiveTime::MIN).and_utc();
        Some(match self {
            Today => today,
            ThisWeek => today - CDuration::days(i64::from(now.weekday().num_days_from_monday())),
            LastWeek => Self::ThisWeek.threshold(now)? - CDuration::days(7),
            ThisMonth => today - CDuration::days(i64::from(now.day())),
            LastMonth => Self::ThisMonth.threshold(now)?.checked_sub_months(chrono::Months::new(1))?,
            Older => return None,
        })
    }
    fn label(self) -> String {
        use TimeStage::{Today, ThisWeek, LastWeek, ThisMonth, LastMonth, Older};
        tr(match self {
            Today => "Today",
            ThisWeek => "This Week",
            LastWeek => "Last Week",
            ThisMonth => "This Month",
            LastMonth => "Last Month",
            Older => "Older",
        })
    }
}

pub fn join_artists(names: &[String]) -> String {
    if names.len() >= 2 {
        trf("%s and %s", &[&names[..names.len() - 1].join(", "), &names[names.len() - 1]])
    } else {
        names.join(", ")
    }
}

fn feed_entry(cover: &gtk::Image, title: &str, subtitle: &str, ellipsize: bool) -> gtk::Button {
    cover.set_pixel_size(54);
    let frame = gtk::AspectFrame::new(0.5, 0.5, 1.0, false).overflow_hidden().corner_radius(6);
    frame.set_child(Some(cover));
    let t = label(title).halign(gtk::Align::Start).class("heading");
    let s = label(subtitle).halign(gtk::Align::Start).class("body");
    if ellipsize {
        t.set_ellipsize(gtk::pango::EllipsizeMode::End);
        s.set_ellipsize(gtk::pango::EllipsizeMode::End);
    }
    let text = vstack(&[t.upcast_ref(), s.upcast_ref()]).valign(gtk::Align::Center);
    let b = gtk::Button::new().hexp(true);
    b.set_child(Some(&spacing(hstack(&[frame.upcast_ref(), text.upcast_ref()]), 16).vpadding(6)));
    b
}

fn stage_box(stage: TimeStage) -> gtk::Box {
    spacing(vstack(&[label(&stage.label()).class("title-2").upcast_ref()]), 12)
}

async fn feed() -> Response {
    let uid = secrets::user_id();
    if uid.is_empty() {
        return Response::new(tr("Feed"), comp::auth_required(&tr("Please sign in to view your feed")));
    }
    let u = uid.clone();
    let mut acts = match api::run(move || api::feed_activities(&u)).await {
        Ok(a) => a,
        Err(e) => return Response::from_error(tr("Feed"), e),
    };
    let parse = |s: &str| DateTime::parse_from_rfc3339(s).map(|d| d.with_timezone(&chrono::Utc)).unwrap_or_default();
    acts.sort_by_key(|a| std::cmp::Reverse(parse(&a.followable_activity.occurred_at)));

    let now = Local::now();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 36).vmargin(20);
    let mut stage = TimeStage::Today;
    let mut bx = stage_box(stage);
    let mut has_elements = false;
    let mut has_unseen = false;
    for (i, a) in acts.iter().enumerate() {
        let fa = &a.followable_activity;
        let at = parse(&fa.occurred_at);
        while let Some(th) = stage.threshold(now)
            && at < th
        {
            stage = stage.next();
            if has_elements {
                body.append(&bx);
            }
            bx = stage_box(stage);
            has_elements = false;
        }
        match (fa.activity_type.as_str(), &fa.album, &fa.history_mix) {
            ("NEW_ALBUM_RELEASE", Some(al), _) => {
                let names: Vec<String> = al.artists.iter().map(|a| a.name.clone()).collect();
                let artists = join_artists(&names);
                let subtitle = match al.type_.as_str() {
                    "SINGLE" => trf("Single by %s", &[&artists]),
                    "ALBUM" => trf("Album by %s", &[&artists]),
                    "EP" => trf("EP by %s", &[&artists]),
                    _ => tr("Unknown album type"),
                };
                let img = gtk::Image::new();
                imgutil::load_into_image(&model::image_url(&al.cover), &img, false);
                let b = feed_entry(&img, &al.title, &subtitle, true);
                let id = al.id;
                b.connect_clicked(move |_| router::navigate(&format!("album/{id}")));
                bx.append(&b);
            }
            ("NEW_HISTORY_MIX", _, Some(mix)) => {
                let Some(tex) = imgutil::fetch(&mix.images.medium.url).await else {
                    return Response::from_error(tr("Feed"), anyhow::anyhow!("failed to load image {}", mix.images.medium.url));
                };
                let img = gtk::Image::from_paintable(Some(&tex));
                let b = feed_entry(&img, &mix.title, &mix.subtitle, false);
                let id = mix.id.clone();
                b.connect_clicked(move |_| router::navigate(&format!("playlist/{id}")));
                bx.append(&b);
            }
            _ => {
                let bin = adw::Bin::new();
                bin.set_child(Some(&label(&tr("Unsupported activity"))));
                bx.append(&bin);
            }
        }
        has_elements = true;
        if !a.seen {
            has_unseen = true;
        }
        if !a.seen && acts.get(i + 1).is_some_and(|n| n.seen) {
            let sep = gtk::Separator::new(gtk::Orientation::Vertical).css("separator { background-color: var(--accent-color); padding-top: 2px; border-radius: 5px; }");
            bx.append(&sep);
        }
    }
    body.append(&bx);
    if has_unseen
        && let Err(e) = api::run(move || api::feed_seen(&uid)).await
    {
        log::warn!("failed to mark the feed as seen: {e:#}");
    }
    let c = adw::Clamp::new().hpadding(8);
    c.set_child(Some(&body));
    c.set_orientation(gtk::Orientation::Horizontal);
    c.set_maximum_size(800);
    Response::new(tr("Feed"), scrolled(&c))
}

// ------------------------------------------------------------------ search

fn prompt_view() -> gtk::Widget {
    adw::StatusPage::builder()
        .icon_name("loupe-symbolic")
        .title(tr("Search"))
        .description(tr("Start typing in the search bar to search for songs, artists, albums or playlists"))
        .build()
        .upcast()
}

fn search_loading_view() -> gtk::Widget {
    clamp(50, &adw::Spinner::new()).upcast()
}

const SEARCH_INCLUDES: [&str; 6] = ["topHits", "topHits.profileArt", "topHits.coverArt", "topHits.albums.coverArt", "topHits.albums.artists", "topHits.artists"];

fn top_hits(r: &api::openapi::SearchResultSingle) -> gtk::Widget {
    let q = &r.data.id;
    let hits = &r.data.relationships.top_hits.data;
    let artists = HorizontalList::new(&tr("Artists"));
    artists.set_page_margin(40).set_view_all_route(&format!("search/{q}/artists"));
    for a in r.included.artists(hits) {
        artists.append(&media_card::artist(&model::artist_from_openapi(&a)));
    }
    let tracks = TrackList::new(std_columns());
    for t in r.included.tracks(hits) {
        tracks.add_track(model::track_from_openapi(&t));
    }
    tracks.play_all_on_click(|| PlaybackSource::List { title: tr("Search"), route: "search".into() });
    let albums = HorizontalList::new(&tr("Albums"));
    albums.set_page_margin(40).set_view_all_route(&format!("search/{q}/albums"));
    for a in r.included.albums(hits) {
        albums.append(&media_card::album(&model::album_from_openapi(&a)));
    }
    let playlists = HorizontalList::new(&tr("Playlists"));
    playlists.set_page_margin(40).set_view_all_route(&format!("search/{q}/playlists"));
    for p in r.included.playlists(hits) {
        playlists.append(&media_card::playlist(&model::playlist_from_openapi(&p)));
    }
    let title = RowTitle::new().set_title(&tr("Tracks")).set_view_all_route(&format!("search/{q}/tracks"));
    let tracks_box = vstack(&[title.widget.upcast_ref(), tracks.widget()]).hmargin(40);
    spacing(
        vstack(&[artists.widget.upcast_ref(), albums.widget.upcast_ref(), playlists.widget.upcast_ref(), tracks_box.upcast_ref(), spacer().upcast_ref()]),
        25,
    )
    .vmargin(20)
    .valign(gtk::Align::Start)
    .upcast()
}

fn search() -> Response {
    let sw = scrolled(&prompt_view());
    let generation = Rc::new(Cell::new(0u64));
    let swr = sw.downgrade();
    let handler = Rc::new(move |se: &gtk::SearchEntry| {
        let Some(sw) = swr.upgrade() else { return };
        let q = se.text().to_string();
        generation.set(generation.get() + 1);
        if q.is_empty() {
            sw.set_child(Some(&prompt_view()));
            return;
        }
        sw.set_child(Some(&search_loading_view()));
        let (g, cur) = (Rc::clone(&generation), generation.get());
        let swr = sw.downgrade();
        glib::spawn_future_local(async move {
            let r = api::run(move || api::search(&q, &SEARCH_INCLUDES)).await;
            let Some(sw) = swr.upgrade() else { return };
            if g.get() != cur {
                return;
            }
            match r {
                Err(e) => {
                    log::error!("search failed error={e:#}");
                    sw.set_child(Some(&prompt_view()));
                }
                Ok(r) => sw.set_child(Some(&comp::main_content(&top_hits(&r)))),
            }
        });
    });
    let searching = Rc::new(Cell::new(false));
    let entry = gtk::SearchEntry::new().hexp(true);
    entry.set_placeholder_text(Some(&tr("For example, Fox Stevenson")));
    entry.set_search_delay(1000);
    let (h, s) = (Rc::clone(&handler), Rc::clone(&searching));
    entry.connect_activate(move |se| {
        s.set(true);
        let s2 = Rc::clone(&s);
        glib::timeout_add_seconds_local_once(1, move || s2.set(false));
        h(se);
    });
    entry.connect_map(|w| {
        w.grab_focus();
    });
    entry.connect_search_changed(move |se| {
        if searching.get() && !se.text().is_empty() {
            return;
        }
        handler(se);
    });
    let c = adw::Clamp::new();
    c.set_orientation(gtk::Orientation::Horizontal);
    c.set_maximum_size(500);
    c.set_child(Some(&entry));
    Response::new(tr("Search"), sw).toolbar(c)
}

async fn search_albums(q: String) -> Response {
    match paginated_media_card_page(service::search_albums(&q), media_card::album).await {
        Ok(v) => Response::new(tr("Search"), v),
        Err(e) => Response::from_error(tr("Search"), e),
    }
}
async fn search_artists(q: String) -> Response {
    match paginated_media_card_page(service::search_artists(&q), media_card::artist).await {
        Ok(v) => Response::new(tr("Search"), v),
        Err(e) => Response::from_error(tr("Search"), e),
    }
}
async fn search_playlists(q: String) -> Response {
    match paginated_media_card_page(service::search_playlists(&q), media_card::playlist).await {
        Ok(v) => Response::new(tr("Search"), v),
        Err(e) => Response::from_error(tr("Search"), e),
    }
}
async fn search_tracks(q: String) -> Response {
    match paginated_tracklist_page(service::search_tracks(&q), std_columns(), 40, None, Some(PlaybackSource::List { title: tr("Search"), route: format!("search/{q}/tracks") })).await {
        Ok((v, _)) => Response::new(tr("Search"), v),
        Err(e) => Response::from_error(tr("Search"), e),
    }
}

// ------------------------------------------------------------------ collection

fn collection_auth() -> Option<Response> {
    (!secrets::has_refresh_token()).then(|| Response::new(tr("My Collection"), comp::auth_required(&tr("Please sign in to view your collection"))))
}

fn my_collection_err(e: anyhow::Error) -> Response {
    let mut r = Response::new(tr("My Collection"), label(&tr("Error loading collection")));
    r.error = Some(e);
    r
}

async fn my_collection() -> Response {
    if let Some(r) = collection_auth() {
        return r;
    }
    let res = api::run(|| {
        Ok((
            api::user_collection_items("Artists", "me", "", &["items.profileArt"])?,
            api::user_collection_items("Albums", "me", "", &["items.coverArt"])?,
            api::user_collection_items("Playlists", "me", "", &["items.coverArt"])?,
            api::user_collection_items("Tracks", "me", "", &["items.artists", "items.albums.coverArt"])?,
        ))
    })
    .await;
    let (ar, al, pl, tr_) = match res {
        Ok(v) => v,
        Err(e) => return my_collection_err(e),
    };
    let artists = HorizontalList::new(&tr("Artists"));
    artists.set_page_margin(40).set_view_all_route("my-collection/artists");
    for a in ar.included.artists(&ar.data) {
        artists.append(&media_card::artist(&model::artist_from_openapi(&a)));
    }
    let albums = HorizontalList::new(&tr("Albums"));
    albums.set_page_margin(40).set_view_all_route("my-collection/albums");
    for a in al.included.albums(&al.data) {
        albums.append(&media_card::album(&model::album_from_openapi(&a)));
    }
    let playlists = HorizontalList::new(&tr("Playlists"));
    playlists.set_page_margin(40).set_view_all_route("my-collection/playlists");
    for p in pl.included.playlists(&pl.data) {
        playlists.append(&media_card::playlist(&model::playlist_from_openapi(&p)));
    }
    let tracks = TrackList::new(std_columns());
    for t in tr_.included.tracks(&tr_.data) {
        tracks.add_track(model::track_from_openapi(&t));
    }
    // The preview shows the first few favourites; play them from the clicked
    // one (the prefetched full list if cached) and load more lazily.
    tracks.set_list_click_handler(|l, t, pos| {
        if let Some(all) = cache::get(&cache::Job::FavouriteTracks.key()) {
            player::play_list_from(PlaybackSource::MyTracks, all, t, pos);
            return;
        }
        let Some(generation) = player::play_list_from(PlaybackSource::MyTracks, l.tracks(), t, pos) else { return };
        let p = service::shared(service::user_collection_tracks("me"));
        player::set_list_feeder(generation, page_feeder(None, p, generation));
    });
    let title = RowTitle::new().set_title(&tr("Tracks")).set_view_all_route("my-collection/tracks");
    let tracks_box = vstack(&[title.widget.upcast_ref(), tracks.widget()]).hmargin(40);
    let body = spacing(
        vstack(&[artists.widget.upcast_ref(), albums.widget.upcast_ref(), playlists.widget.upcast_ref(), tracks_box.upcast_ref(), spacer().upcast_ref()]),
        25,
    )
    .vmargin(20)
    .valign(gtk::Align::Start);
    Response::new(tr("My Collection"), scrolled(&comp::main_content(&body)))
}

async fn my_albums() -> Response {
    if let Some(r) = collection_auth() {
        return r;
    }
    match paginated_media_card_page(service::user_collection_albums("me"), media_card::album).await {
        Ok(v) => Response::new(tr("My Albums"), v),
        Err(e) => Response::from_error(tr("My Albums"), e),
    }
}
async fn my_artists() -> Response {
    if let Some(r) = collection_auth() {
        return r;
    }
    match paginated_media_card_page(service::user_collection_artists("me"), media_card::artist).await {
        Ok(v) => Response::new(tr("My Artists"), v),
        Err(e) => Response::from_error(tr("My Artists"), e),
    }
}
async fn my_playlists() -> Response {
    if let Some(r) = collection_auth() {
        return r;
    }
    match paginated_media_card_page(service::user_collection_playlists("me"), media_card::playlist).await {
        Ok(v) => Response::new(tr("My Playlists"), v),
        Err(e) => Response::from_error(tr("My Playlists"), e),
    }
}

async fn my_tracks() -> Response {
    if let Some(r) = collection_auth() {
        return r;
    }
    let (page, _) = match paginated_tracklist_page(service::user_collection_tracks("me"), std_columns(), 40, Some(cache::Job::FavouriteTracks), Some(PlaybackSource::MyTracks)).await {
        Ok(v) => v,
        Err(e) => return Response::from_error(tr("My Tracks"), e),
    };
    let play_all = |shuffle: bool| {
        move || {
            let msg = if shuffle { tr("An error occurred while shuffling the tracks") } else { tr("An error occurred while playing the tracks") };
            play(
                async move {
                    let tracks = crate::cache::all_tracks(crate::cache::Job::FavouriteTracks).await?;
                    player::play_tracklist(PlaybackSource::MyTracks, tracks, shuffle, 0).await
                },
                msg,
                "An error occurred while playing the tracks",
            );
        }
    };
    let header = tracklist_header::collection(&PlaybackSource::MyTracks, play_all(false), play_all(true)).hmargin(40);
    page.set_vexpand(true);
    Response::new(tr("My Tracks"), spacing(vstack(&[&comp::main_content(&header), page.upcast_ref()]), 20))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn artists_join() {
        assert_eq!(join_artists(&["A".into()]), "A");
        assert_eq!(join_artists(&["A".into(), "B".into(), "C".into()]), "A, B and C");
    }

    #[test]
    fn stages_are_descending() {
        let now = Local.with_ymd_and_hms(2026, 10, 31, 12, 0, 0).unwrap();
        let t: Vec<_> = [TimeStage::Today, TimeStage::ThisWeek, TimeStage::LastWeek].iter().map(|s| s.threshold(now).unwrap()).collect();
        assert!(t[0] >= t[1] && t[1] > t[2]);
        assert_eq!(TimeStage::Older.threshold(now), None);
    }
}
