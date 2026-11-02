//! TIDAL API access: the HTTP client and token cache, the legacy v1/v2
//! JSON models and the JSON:API (`openapi`) models.

pub use tonearm_core::api::*;


/// Runs blocking API work on a worker thread and awaits it from the main loop.
/// Accepts core (`CoreError`) and app (`anyhow`) closures alike.
pub async fn run<T: Send + 'static, E: Into<anyhow::Error> + Send + 'static>(f: impl FnOnce() -> Result<T, E> + Send + 'static) -> anyhow::Result<T> {
    match gtk::gio::spawn_blocking(f).await {
        Ok(r) => r.map_err(Into::into),
        Err(_) => Err(anyhow::anyhow!("worker thread panicked")),
    }
}
