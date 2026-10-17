//! NaBrowse browser shell.
//!
//! Entry point: parses CLI arguments, fetches a URL, and displays
//! the page content in a GPU-rendered window.

mod app;

use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Parse CLI arguments: nabrowse [URL]
    let args: Vec<String> = std::env::args().collect();
    let url = args.get(1).cloned().unwrap_or_else(|| "https://example.com".to_string());

    tracing::info!("Starting NaBrowse shell");
    tracing::info!("URL: {}", url);

    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            tracing::error!("Failed to create event loop: {}", err);
            std::process::exit(1);
        }
    };

    let app = app::App::new(url);

    if let Err(err) = event_loop.run_app(app) {
        tracing::error!("Event loop error: {}", err);
        std::process::exit(1);
    }

    tracing::info!("NaBrowse shutting down");
}
