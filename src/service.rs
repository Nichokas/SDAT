//! Blocking catalogue calls live in [`tonearm_core::service`]; this adds the
//! main-loop helper that runs them on a worker thread.

use anyhow::Result;

pub use tonearm_core::service::*;

use crate::api;

/// Fetches the next page on a worker thread.
pub async fn next_page<T: Clone + Send + 'static>(p: &SharedPaginator<T>) -> Result<Vec<T>> {
    let p = std::sync::Arc::clone(p);
    api::run(move || p.lock().unwrap_or_else(std::sync::PoisonError::into_inner).next_page()).await
}
