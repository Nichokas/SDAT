//! Port of `internal/ui/accels`.
#[cfg(target_os = "macos")]
pub const PRIMARY: &str = "<Meta>";
#[cfg(not(target_os = "macos"))]
pub const PRIMARY: &str = "<Primary>";
