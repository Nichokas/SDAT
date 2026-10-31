//! "Add to Playlist" dialog: lists the user's own playlists and appends a track.

use adw::prelude::*;

use crate::api::{self, client::OwnPlaylist};
use crate::i18n::{tr, trf, trn};
use crate::player;

/// Fetches the user's playlists, then lets them pick one to add `track_id` to.
pub fn present(parent: &gtk::Widget, track_id: String) {
    let parent = parent.downgrade();
    glib::spawn_future_local(async move {
        match api::run(api::client::own_playlists).await {
            Ok(list) => {
                if let Some(parent) = parent.upgrade() {
                    dialog(&list, track_id).present(Some(&parent));
                }
            }
            Err(e) => {
                log::error!("loading own playlists failed error={e:#}");
                player::toasts().notify(&tr("Could not load your playlists"));
            }
        }
    });
}

fn dialog(list: &[OwnPlaylist], track_id: String) -> adw::AlertDialog {
    let d = adw::AlertDialog::new(Some(&tr("Add to Playlist")), None);
    d.add_response("cancel", &tr("Cancel"));
    d.set_close_response("cancel");
    if list.is_empty() {
        d.set_body(&tr("You don't have any playlists yet"));
        return d;
    }
    let lb = gtk::ListBox::new();
    lb.add_css_class("boxed-list");
    lb.set_selection_mode(gtk::SelectionMode::None);
    for p in list {
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&p.name).as_str())
            .subtitle(trn("%d Track", "%d Tracks", p.number_of_items, &[&p.number_of_items]))
            .activatable(true)
            .build();
        lb.append(&row);
    }
    let sw = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(360)
        .child(&lb)
        .build();
    d.set_extra_child(Some(&sw));
    let targets: Vec<(String, String)> = list.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
    let weak = d.downgrade();
    lb.connect_row_activated(move |_, row| {
        let Some((id, name)) = usize::try_from(row.index()).ok().and_then(|i| targets.get(i)).cloned() else {
            return;
        };
        if let Some(d) = weak.upgrade() {
            d.close();
        }
        let track = track_id.clone();
        glib::spawn_future_local(async move {
            match api::run(move || api::client::playlist_add_track(&id, &track)).await {
                Ok(()) => player::toasts().notify(&trf("Added to %s", &[&name])),
                Err(e) => {
                    log::error!("adding track to playlist failed error={e:#}");
                    player::toasts().notify(&tr("Could not add the track to the playlist"));
                }
            }
        });
    });
    d
}
