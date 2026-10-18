//! Desktop viewer with an independently testable network session.
#![forbid(unsafe_code)]
#[cfg(feature = "gui")]
pub mod app;
#[cfg(feature = "gui")]
pub mod host;
pub mod net;
pub mod render;
