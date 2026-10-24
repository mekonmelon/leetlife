//! LeetLife core. The Rust side owns all state, scheduling, persistence and
//! performance-sensitive logic; the React UI only renders snapshots it is handed.
//!
//! Layering (top depends on bottom):
//!
//! ```text
//!   app/ (Tauri shell: tray, window, notifier, UI push)   commands/ (IPC surface)
//!                              │
//!                           engine   (orchestrates one transactional reducer per action)
//!        ┌─────────┬──────────┼──────────┬──────────────┬─────────────┐
//!     events   scheduler  recommendation  game        activity     cache / metrics
//!        └─────────┴──────────┴──────────┴──────────────┴─────────────┘
//!                              │
//!                           storage (SQLx + SQLite)
//! ```

pub mod activity;
#[cfg(feature = "desktop")]
pub mod app;
pub mod cache;
pub mod clock;
#[cfg(feature = "desktop")]
pub mod commands;
pub mod engine;
pub mod error;
pub mod events;
pub mod game;
pub mod home;
pub mod metrics;
pub mod models;
pub mod notifications;
pub mod recommendation;
pub mod scheduler;
pub mod storage;

pub use engine::Engine;
pub use error::{Error, Result};

/// Entry point used by `main.rs`.
#[cfg(feature = "desktop")]
pub fn run() {
    app::run();
}
