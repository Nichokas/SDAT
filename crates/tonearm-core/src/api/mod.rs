//! TIDAL API models: the legacy v1/v2 JSON models and the JSON:API (`openapi`) models.

pub mod client;
pub mod legacy;
pub mod openapi;

pub use client::*;
