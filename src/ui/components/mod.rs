//! Port of `internal/ui/components`.
#![allow(dead_code)]

pub mod favourite_button;
pub mod horizontal_list;
pub mod linking;
pub mod media_card;
pub mod media_viewer;
pub mod shortcut_list;
pub mod sidebar;
pub mod tracklist;
pub mod tracklist_header;

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib, pango};

use crate::api::legacy::{self, item_type, ItemData};
use crate::i18n::tr;
use crate::model::{self, Cover};
use crate::ui::dsl::*;
use crate::{imgutil, router};

pub use horizontal_list::HorizontalList;
pub use tracklist::{Col, TrackList};

/// Cancels an in-flight cover render.
#[derive(Clone, Default)]
pub struct CancelToken(Rc<Cell<bool>>);
impl CancelToken {
    pub fn cancel(&self) {
        self.0.set(true);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.get()
    }
}

/// Port of RenderCover: sets the static cover, then the animated one once prepared.
pub fn render_cover(cover: Cover, target: impl Fn(gdk::Paintable) + 'static) -> CancelToken {
    let token = CancelToken::default();
    if !cover.has_static() && !cover.has_animated() {
        target(imgutil::missing_album());
        return token;
    }
    let t = token.clone();
    glib::spawn_future_local(async move {
        if cover.has_static() {
            match imgutil::fetch_cropped(&cover.static_).await {
                Some(tex) if !t.is_cancelled() => target(tex.upcast()),
                Some(_) => return,
                None => return,
            }
        }
        if !cover.has_animated() || t.is_cancelled() {
            return;
        }
        let media = gtk::MediaFile::for_file(&gio::File::for_uri(&cover.animated));
        media.set_loop(true);
        media.set_muted(true);
        media.play();
        let target = Rc::new(target);
        let handler: Rc<Cell<Option<glib::SignalHandlerId>>> = Rc::new(Cell::new(None));
        let h2 = handler.clone();
        let id = media.connect_prepared_notify(move |m| {
            if let Some(id) = h2.take() {
                m.disconnect(id);
            }
            if !t.is_cancelled() {
                target(m.clone().upcast());
            }
        });
        handler.set(Some(id));
    });
    token
}

pub fn auth_required(description: &str) -> adw::StatusPage {
    let btn = gtk::Button::with_label(&tr("Sign In…")).class("pill").class("suggested-action").halign(gtk::Align::Center);
    btn.set_action_name(Some("win.sign-in"));
    adw::StatusPage::builder()
        .icon_name("avatar-default-symbolic")
        .title(tr("Authentication Required"))
        .description(description)
        .child(&btn)
        .build()
}

pub fn main_content(body: &impl IsA<gtk::Widget>) -> gtk::Widget {
    adw::Clamp::builder()
        .orientation(gtk::Orientation::Horizontal)
        .maximum_size(2000)
        .tightening_threshold(2000)
        .child(body)
        .build()
        .upcast()
}

pub fn main_content_scrollable(body: &impl IsA<gtk::Widget>) -> gtk::Widget {
    adw::ClampScrollable::builder()
        .orientation(gtk::Orientation::Horizontal)
        .maximum_size(2000)
        .tightening_threshold(2000)
        .child(body)
        .build()
        .upcast()
}

pub fn squircle_picture(p: gtk::Picture) -> gtk::Picture {
    p.background("alpha(var(--view-fg-color), 0.1)").overflow_hidden().corner_radius(10)
}

// ---------------------------------------------------------------- route button

/// Port of RouteButton: a header button that is "raised" while its route is active.
pub struct RouteButton {
    pub button: gtk::Button,
    image: gtk::Image,
    title: gtk::Label,
}

impl RouteButton {
    pub fn new(path: &'static str, root: bool) -> Self {
        let image = gtk::Image::from_icon_name("image-missing-symbolic");
        let title = label("").visible(false).pstart(7).pend(7);
        let content = spacing(hstack(&[clamp(16, &image).upcast_ref(), title.upcast_ref()]), 7).hmargin(9).vmargin(2);
        let button = gtk::Button::new().pstart(0).pend(0).min_height(24).class("flat");
        button.set_child(Some(&content));
        button.connect_clicked(move |_| {
            if root {
                router::clear();
            }
            router::navigate(path);
        });
        on_signal(router::navigation(), &button, move |b, ev| {
            if ev.path.starts_with(path) {
                b.remove_css_class("flat");
                b.add_css_class("raised");
            } else {
                b.remove_css_class("raised");
                b.add_css_class("flat");
            }
        });
        RouteButton { button, image, title }
    }
    pub fn title(self, t: &str) -> Self {
        self.title.set_text(t);
        self.title.set_visible(!t.is_empty());
        if t.is_empty() {
            self.title.remove_css_class("title");
        } else {
            self.title.add_css_class("title");
        }
        self
    }
    pub fn icon(self, i: &str) -> Self {
        self.image.set_icon_name(Some(i));
        self
    }
    pub fn tooltip(self, t: &str) -> Self {
        self.button.set_tooltip_text(Some(t));
        self
    }
}

// ---------------------------------------------------------------- row title

fn view_all_button(path: String) -> gtk::Button {
    let b = gtk::Button::new().min_height(10).min_width(10).hpadding(10);
    b.set_child(Some(&label(&tr("View All")).class("caption-heading")));
    b.connect_clicked(move |_| router::navigate(&path));
    b
}

pub struct RowTitle {
    pub widget: gtk::Box,
    title: gtk::Label,
    route: gtk::CenterBox,
}

impl RowTitle {
    pub fn new() -> Self {
        let title = label("").visible(false).valign(gtk::Align::Center).mstart(10).mbottom(5).class("title-2");
        let route = gtk::CenterBox::new().hexp(false).vexp(false);
        let widget = hstack(&[title.upcast_ref(), spacer().vexp(false).upcast_ref(), route.upcast_ref()]);
        RowTitle { widget, title, route }
    }
    pub fn set_title(self, t: &str) -> Self {
        if t.is_empty() {
            self.title.set_visible(false);
            return self;
        }
        self.title.set_text(t);
        self.title.set_visible(true);
        self
    }
    pub fn set_view_all_route(self, path: &str) -> Self {
        self.route.set_center_widget(Some(&view_all_button(path.to_string()).valign(gtk::Align::Center)));
        self
    }
}

impl Default for RowTitle {
    fn default() -> Self {
        Self::new()
    }
}

fn unsupported_card(type_: &str) -> gtk::Widget {
    let l = label(&format!("Unsupported\n{type_}")).padding(30).hexp(true).corner_radius(10);
    l.set_justify(gtk::Justification::Center);
    hstack(&[l.upcast_ref()]).size(192, -1).css("box:hover { background: alpha(var(--view-fg-color), 0.1); }").corner_radius(10).upcast()
}

fn unsupported_element() -> gtk::Widget {
    let l = label("Unsupported Element").background("alpha(var(--view-fg-color), 0.1)").padding(30).hexp(true).corner_radius(10);
    hstack(&[l.upcast_ref()]).hexp(true).hmargin(40).upcast()
}

/// Port of ForPageItem (v2 pages).
pub fn for_page_item(item: &legacy::PageItem) -> gtk::Widget {
    match item.type_.as_str() {
        item_type::HORIZONTAL_LIST | item_type::HORIZONTAL_LIST_WITH_CONTEXT => {
            let mut title = item.title.clone();
            if let Some(h) = &item.header {
                title = match &h.data {
                    ItemData::Album(a) => a.title.clone(),
                    ItemData::Artist(a) => a.name.clone(),
                    ItemData::DeepLink(d) => d.title.clone(),
                    ItemData::Mix(m) => m.title_text_info.text.clone(),
                    ItemData::Playlist(p) => p.title.clone(),
                    ItemData::Track(t) => t.title.clone(),
                    ItemData::Unknown => title,
                };
            }
            let list = HorizontalList::new(&title);
            for it in &item.items {
                let w: gtk::Widget = match &it.data {
                    ItemData::Album(a) => media_card::album(&model::album_from_v2(a)).upcast(),
                    ItemData::Playlist(p) => media_card::playlist(&model::playlist_from_v2(p)).upcast(),
                    ItemData::Artist(a) => media_card::artist(&model::artist_from_v2(a)).upcast(),
                    ItemData::Mix(m) => media_card::legacy_mix(m).upcast(),
                    ItemData::Track(t) => media_card::track(&model::track_from_v2(t)).upcast(),
                    ItemData::DeepLink(d) => media_card::legacy_deeplink(d).upcast(),
                    ItemData::Unknown => unsupported_card(&it.type_),
                };
                list.append(&w);
            }
            list.set_page_margin(40);
            if item.header.is_some() {
                let l = label(&item.title).halign(gtk::Align::Start).class("dimmed").hmargin(50);
                return vstack(&[l.upcast_ref(), list.widget.upcast_ref()]).upcast();
            }
            list.widget.upcast()
        }
        item_type::TRACK_LIST => {
            let list = TrackList::new(vec![Col::cover(), Col::title_album(), Col::artists(), Col::duration(), Col::controls()]);
            for t in &item.items {
                if let ItemData::Track(t) = &t.data {
                    list.add_track(model::track_from_v2(t));
                }
            }
            vstack(&[RowTitle::new().set_title(&item.title).widget.upcast_ref(), list.widget()]).hmargin(40).upcast()
        }
        item_type::SHORTCUT_LIST => {
            let list = shortcut_list::ShortcutList::new();
            for it in &item.items {
                match &it.data {
                    ItemData::DeepLink(d) => list.append(&shortcut_list::legacy_deeplink(d)),
                    ItemData::Album(a) => list.append(&shortcut_list::legacy_album(a)),
                    ItemData::Artist(a) => list.append(&shortcut_list::legacy_artist(a)),
                    ItemData::Playlist(p) => list.append(&shortcut_list::legacy_playlist(p)),
                    ItemData::Mix(m) => list.append(&shortcut_list::legacy_mix(m)),
                    _ => list.append(&label(&format!("Unsupported: {}", it.type_)).hmargin(10)),
                }
            }
            list.widget.clone().hmargin(50).upcast()
        }
        other => {
            log::warn!("Unsupported item type type={other}");
            unsupported_element()
        }
    }
}

/// Port of ForModule (v1 pages).
pub fn for_module(module: &legacy::Module) -> gtk::Widget {
    match module.type_.as_str() {
        "VIDEO_LIST" => gtk::Box::new(gtk::Orientation::Horizontal, 0).upcast(),
        "FEATURED_PROMOTIONS" => {
            let list = HorizontalList::new(&module.title);
            for it in &module.items {
                if it.type_ == "CATEGORY_PAGES" {
                    continue;
                } else if it.type_ == "PLAYLIST" {
                    let url = format!("{}/images/{}/550x400.jpg", crate::settings::resources_base_url(), it.image_id.replace('-', "/"));
                    list.append(&media_card::mix_generic(&it.artifact_id, &it.short_header, &it.short_sub_header, &url));
                } else {
                    list.append(&unsupported_card(&it.type_));
                }
            }
            list.set_page_margin(40);
            list.widget.upcast()
        }
        "PLAYLIST_LIST" => {
            let list = HorizontalList::new(&module.title);
            for it in &module.paged_list.items {
                let fake = legacy::PlaylistItemData {
                    creator: legacy::Creator { id: it.creator.id, name: it.creator.name.clone(), ..Default::default() },
                    duration: it.duration,
                    number_of_tracks: it.number_of_tracks,
                    square_image: it.square_image.clone(),
                    title: it.title.clone(),
                    uuid: it.uuid.clone(),
                    ..Default::default()
                };
                list.append(&media_card::playlist(&model::playlist_from_v2(&fake)));
            }
            list.set_page_margin(40);
            list.widget.upcast()
        }
        "ARTIST_LIST" => {
            let list = HorizontalList::new(&module.title);
            for it in &module.paged_list.items {
                let fake = legacy::ArtistItemData { id: it.id.int, name: it.name.clone(), picture: it.picture.clone() };
                list.append(&media_card::artist(&model::artist_from_v2(&fake)));
            }
            list.set_page_margin(40);
            list.widget.upcast()
        }
        "ALBUM_LIST" => {
            let list = HorizontalList::new(&module.title);
            for it in &module.paged_list.items {
                let fake = legacy::AlbumItemData {
                    artists: it.artists.iter().map(|a| legacy::ArtistItemData { id: a.id, name: a.name.clone(), ..Default::default() }).collect(),
                    cover: it.cover.clone(),
                    id: it.id.int,
                    duration: it.duration,
                    release_date: chrono::NaiveDate::parse_from_str(&it.release_date, "%Y-%m-%d").ok(),
                    title: it.title.clone(),
                    type_: "ALBUM".into(),
                    ..Default::default()
                };
                list.append(&media_card::album(&model::album_from_v2(&fake)));
            }
            list.set_page_margin(40);
            list.widget.upcast()
        }
        "TRACK_LIST" => {
            let list = TrackList::new(vec![Col::cover(), Col::title_album(), Col::artists(), Col::duration(), Col::controls()]);
            for it in &module.paged_list.items {
                let fake = legacy::TrackItemData {
                    album: legacy::TrackItemDataAlbum { cover: it.album.cover.clone(), id: it.album.id, title: it.album.title.clone(), ..Default::default() },
                    artists: it.artists.iter().map(|a| legacy::ArtistItemData { id: a.id, name: a.name.clone(), ..Default::default() }).collect(),
                    duration: it.duration,
                    id: it.id.int,
                    title: it.title.clone(),
                    ..Default::default()
                };
                list.add_track(model::track_from_v2(&fake));
            }
            vstack(&[RowTitle::new().set_title(&module.title).widget.upcast_ref(), list.widget()]).hmargin(40).upcast()
        }
        "PAGE_LINKS_CLOUD" | "PAGE_LINKS" => {
            let list = shortcut_list::ShortcutList::new();
            for it in &module.paged_list.items {
                let path = it.api_path.replace("pages/", "explore/");
                let b = shortcut_list::text_shortcut(&it.title, "");
                b.connect_clicked(move |_| router::navigate(&path));
                list.append(&b);
            }
            let title = if module.type_ == "PAGE_LINKS" { tr("More") } else { module.title.clone() };
            vstack(&[RowTitle::new().set_title(&title).widget.hpadding(40).upcast_ref(), list.widget.clone().hmargin(50).upcast_ref()]).upcast()
        }
        other => {
            log::warn!("Unsupported module type type={other}");
            unsupported_element()
        }
    }
}

pub(crate) fn ellipsize(l: gtk::Label) -> gtk::Label {
    l.set_ellipsize(pango::EllipsizeMode::End);
    l
}
