//! Application core: winit window + wgpu GPU context + render loop.
//!
//! Fetches a URL on startup, converts HTML to text, and renders it
//! in the window using the GPU text renderer.

use nabrowse_paint::{GraphicsContext, FontRasterizer, html_to_text};
use nabrowse_shared::{Color, Size};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

pub struct App {
    url: String,
    window: Option<Box<dyn Window>>,
    graphics: Option<GraphicsContext>,
    size: Size,
    clear_color: Color,
    exiting: bool,
    // Fetched page content
    page_text: Vec<String>,
    page_status: String,
    font_rasterizer: Option<FontRasterizer>,
}

impl App {
    pub fn new(url: String) -> Self {
        Self {
            url,
            window: None,
            graphics: None,
            size: Size::new(1024.0, 768.0),
            clear_color: Color::nabrowse_bg(),
            exiting: false,
            page_text: Vec::new(),
            page_status: String::new(),
            font_rasterizer: None,
        }
    }

    fn init_window(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut attrs = WindowAttributes::default();
        attrs.visible = true;
        attrs.title = "NaBrowse".to_string();
        attrs.surface_size = Some(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(1024.0, 768.0)));

        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(err) => {
                tracing::error!("Failed to create window: {}", err);
                event_loop.exit();
                return;
            }
        };
        tracing::info!("Window created successfully");

        let size_px = window.surface_size();
        tracing::info!("Window surface size: {}x{}", size_px.width, size_px.height);
        self.size = Size::new(size_px.width as f32, size_px.height as f32);

        tracing::info!("Initializing graphics...");
        let graphics = match futures::executor::block_on(GraphicsContext::from_window(&*window))
        {
            Ok(g) => g,
            Err(err) => {
                tracing::error!("Failed to initialize graphics: {}", err);
                event_loop.exit();
                return;
            }
        };
        tracing::info!("Graphics initialized successfully");

        self.window = Some(window);
        self.graphics = Some(graphics);

        // Load font rasterizer
        self.font_rasterizer = FontRasterizer::new();
        if self.font_rasterizer.is_none() {
            tracing::error!("No font available - text will not render");
        }

        tracing::info!("NaBrowse window opened");
        // Request first redraw
        if let Some(ref w) = self.window {
            w.request_redraw();
        }
    }

    fn fetch_url(&mut self) {
        tracing::info!("Fetching: {}", self.url);
        self.page_status = format!("Fetching {}...", self.url);

        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                self.page_status = format!("Failed to create runtime: {}", err);
                return;
            }
        };

        match runtime.block_on(async {
            let client = nabrowse_network::NetworkClient::new()?;
            client.fetch(&self.url).await
        }) {
            Ok(response) => {
                tracing::info!("Fetched {} (status: {}, {} bytes)",
                    self.url, response.status, response.body.len());
                self.page_status = format!(
                    "{} | Status: {} | {} bytes",
                    self.url, response.status, response.body.len()
                );
                let html = String::from_utf8_lossy(&response.body);
                self.page_text = html_to_text(&html);
            }
            Err(err) => {
                tracing::error!("Fetch failed: {}", err);
                self.page_status = format!("Error: {}", err);
                self.page_text = vec![self.page_status.clone()];
            }
        }
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let graphics = self.graphics.as_mut().ok_or("graphics not initialized")?;
        let window = self.window.as_ref().ok_or("window not initialized")?;
        let text_renderer = graphics.text_renderer.as_ref()
            .ok_or("text renderer not initialized")?;

        match graphics.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = graphics
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("frame_encoder"),
                    });

                // Clear pass
                {
                    let _render_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("clear_pass"),
                            color_attachments: &[Some(
                                wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                            r: self.clear_color.r as f64,
                                            g: self.clear_color.g as f64,
                                            b: self.clear_color.b as f64,
                                            a: self.clear_color.a as f64,
                                        }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                    depth_slice: None,
                                },
                            )],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                }

                // Draw text lines
                if let Some(ref rasterizer) = self.font_rasterizer {
                    let line_height = rasterizer.line_height() as f32;
                    let mut y = 10.0;

                    // Draw status line (bold/larger)
                    if !self.page_status.is_empty() {
                        let (bitmap, w, h) = rasterizer.render_line(&self.page_status, Color::new(0.0, 0.0, 0.0, 1.0));
                        if w > 0 && h > 0 {
                            let text_texture = text_renderer.create_texture(&bitmap, w, h);
                            text_renderer.draw_texture(
                                &mut encoder, &view, &text_texture,
                                10.0, y, self.size.width as u32, self.size.height as u32,
                            );
                        }
                        y += line_height * 2.0;
                    }

                    // Draw page text lines
                    let max_lines = ((self.size.height as u32 - y as u32) / line_height as u32) as usize;
                    for line in self.page_text.iter().take(max_lines) {
                        if line.is_empty() {
                            y += line_height;
                            continue;
                        }
                        let (bitmap, w, h) = rasterizer.render_line(line, Color::new(0.0, 0.0, 0.0, 1.0));
                        if w > 0 && h > 0 {
                            let text_texture = text_renderer.create_texture(&bitmap, w, h);
                            text_renderer.draw_texture(
                                &mut encoder, &view, &text_texture,
                                10.0, y, self.size.width as u32, self.size.height as u32,
                            );
                        }
                        y += line_height;
                    }
                } else {
                    // No font available - just show status
                    tracing::warn!("No font rasterizer available");
                }

                graphics.queue.submit(std::iter::once(encoder.finish()));
                graphics.queue.present(surface_texture);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                let size_px = window.surface_size();
                graphics.resize(size_px.width, size_px.height);
                self.size = Size::new(size_px.width as f32, size_px.height as f32);
                window.request_redraw();
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                window.request_redraw();
            }
            _ => {}
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, cause: winit::event::StartCause) {
        // Create window on first new_events call regardless of cause
        if self.window.is_none() {
            tracing::info!("Creating window (first new_events, cause: {:?})", cause);
            self.init_window(event_loop);
            // Fetch URL after window is ready
            self.fetch_url();
            // Request redraw to show fetched content
            if let Some(ref w) = self.window {
                w.request_redraw();
            }
        } else {
            if !matches!(cause, winit::event::StartCause::WaitCancelled { .. }) {
                tracing::debug!("new_events: {:?}", cause);
            }
        }
    }

    fn resumed(&mut self, _event_loop: &dyn ActiveEventLoop) {
        tracing::debug!("resumed() called");
    }

    fn can_create_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &dyn ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                self.exiting = true;
            }
            WindowEvent::SurfaceResized(new_size) => {
                if let Some(ref mut g) = self.graphics {
                    g.resize(new_size.width, new_size.height);
                }
                self.size = Size::new(new_size.width as f32, new_size.height as f32);
                if let Some(ref w) = self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let _ = self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.exiting {
            event_loop.exit();
            return;
        }
        if self.window.is_none() {
            tracing::info!("Creating window from about_to_wait");
            self.init_window(event_loop);
            self.fetch_url();
            if let Some(ref w) = self.window {
                w.request_redraw();
            }
            return;
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    }
}
