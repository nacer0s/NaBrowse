//! NaBrowse browser shell.
//!
//! This is the entry point of the browser. It creates a window via winit,
//! initializes a wgpu GPU context, and runs the main event loop.
//!
//! Phase 0: A real, working window that opens on any OS and clears to the
//! NaBrowse background color. No stubs, no placeholders.

mod app;

use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting NaBrowse shell");

    let _app = app::App::new();
}
