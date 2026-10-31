//! TIDAL API access: the HTTP client and token cache, the legacy v1/v2
//! JSON models and the JSON:API (`openapi`) models.

pub mod client;
pub mod legacy;
pub mod openapi;

pub use client::*;

/// Runs blocking API work on a worker thread and awaits it from the main loop.
pub async fn run<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> anyhow::Result<T> {
    match gtk::gio::spawn_blocking(f).await {
        Ok(r) => r,
        Err(_) => Err(anyhow::anyhow!("worker thread panicked")),
    }
}
