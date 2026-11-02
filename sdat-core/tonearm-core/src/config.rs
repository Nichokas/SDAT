//! Platform configuration injected by the front end (the desktop app reads
//! `GSettings`; another front end can use its own store).

use std::sync::OnceLock;

/// Settings the core reads. Defaults are the official TIDAL endpoints.
pub trait Config: Send + Sync {
    fn api_base_url(&self) -> String {
        "https://tidal.com".into()
    }
    fn openapi_base_url(&self) -> String {
        "https://openapi.tidal.com".into()
    }
    fn resources_base_url(&self) -> String {
        "https://resources.tidal.com".into()
    }
    fn auth_base_url(&self) -> String {
        "https://auth.tidal.com".into()
    }
    /// Streaming quality to request.
    fn audio_quality(&self) -> crate::model::TrackQuality {
        crate::model::TrackQuality::HighResLossless
    }
    /// Directory for the app's small preference files (e.g. the country
    /// override); `None` disables them.
    fn config_dir(&self) -> Option<std::path::PathBuf> {
        None
    }
}

/// [`Config`] with every default.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultConfig;
impl Config for DefaultConfig {}

static CONFIG: OnceLock<Box<dyn Config>> = OnceLock::new();

/// Installs the front end's configuration. Call once at start-up, before any
/// other core call; later calls are ignored (returns `false`).
pub fn install(c: Box<dyn Config>) -> bool {
    CONFIG.set(c).is_ok()
}

/// The installed configuration, or [`DefaultConfig`].
pub fn get() -> &'static dyn Config {
    CONFIG.get_or_init(|| Box::new(DefaultConfig)).as_ref()
}
