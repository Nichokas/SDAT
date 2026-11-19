//! Importing a Spotify playlist into the user's TIDAL library.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tonearm_core::{playlist_import, spotify};

use crate::{FfiError, run};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ImportStage {
    /// Reading the playlist from Spotify.
    Reading,
    /// Finding its tracks on TIDAL.
    Matching,
    /// Writing the TIDAL playlist.
    Saving,
}

/// `total` is 0 until the stage knows its size.
#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct ImportProgress {
    pub stage: ImportStage,
    pub done: u32,
    pub total: u32,
}

/// Called on a worker thread.
#[uniffi::export(with_foreign)]
pub trait ImportListener: Send + Sync {
    fn on_progress(&self, progress: ImportProgress);
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ImportReport {
    /// The new TIDAL playlist.
    pub playlist_id: String,
    pub name: String,
    /// Tracks added to it.
    pub imported: u32,
    /// `Artist – Title` of each track TIDAL does not have.
    pub missing: Vec<String>,
    /// Spotify entries that are not catalogue tracks (local files, podcast episodes).
    pub skipped: u32,
}

/// Whether `link` is a Spotify playlist link, URI or ID.
#[uniffi::export]
pub fn is_spotify_playlist_link(link: String) -> bool {
    spotify::playlist_id(&link).is_some()
}

/// An import of one public Spotify playlist into a new TIDAL playlist.
#[derive(Debug, uniffi::Object)]
pub struct SpotifyImport {
    link: String,
    cancelled: Arc<AtomicBool>,
}

#[uniffi::export]
pub fn spotify_import(link: String) -> Arc<SpotifyImport> {
    Arc::new(SpotifyImport { link, cancelled: Arc::new(AtomicBool::new(false)) })
}

#[uniffi::export]
impl SpotifyImport {
    /// Runs the import; `Cancelled` after [`Self::cancel`], which no longer stops it once it is saving.
    pub async fn run(&self, listener: Arc<dyn ImportListener>) -> Result<ImportReport, FfiError> {
        let (link, cancelled) = (self.link.clone(), Arc::clone(&self.cancelled));
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let report = run(move || {
            playlist_import::import_spotify_playlist(
                &link,
                &|p| {
                    let stage = match p.stage {
                        playlist_import::Stage::Reading => ImportStage::Reading,
                        playlist_import::Stage::Matching => ImportStage::Matching,
                        playlist_import::Stage::Saving => ImportStage::Saving,
                    };
                    listener.on_progress(ImportProgress { stage, done: count(p.done), total: count(p.total) });
                },
                &|| cancelled.load(Ordering::SeqCst),
            )
        })
        .await?;
        Ok(ImportReport { playlist_id: report.playlist_id, name: report.name, imported: count(report.imported), missing: report.missing, skipped: count(report.skipped) })
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}
