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

    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            tracing::error!("Failed to create event loop: {}", err);
            std::process::exit(1);
        }
    };

    let app = app::App::new();

    if let Err(err) = event_loop.run_app(app) {
        tracing::error!("Event loop error: {}", err);
        std::process::exit(1);
    }

    tracing::info!("NaBrowse shutting down");
}
