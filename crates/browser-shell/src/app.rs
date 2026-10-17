//! Application core: winit window + wgpu GPU context + render loop.
//!
//! Uses winit 0.31's ApplicationHandler pattern and wgpu 30's surface API.
//! Creates a real OS window on Windows/macOS/Linux via winit and renders
//! a clear color every frame using wgpu.

use nabrowse_paint::GraphicsContext;
use nabrowse_shared::{Color, Size};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

pub struct App {
    window: Option<Box<dyn Window>>,
    graphics: Option<GraphicsContext>,
    size: Size,
    clear_color: Color,
    exiting: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            graphics: None,
            size: Size::new(1024.0, 768.0),
            clear_color: Color::nabrowse_bg(),
            exiting: false,
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

        tracing::info!("NaBrowse window opened");
        // Request first redraw
        if let Some(ref w) = self.window {
            w.request_redraw();
        }
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let graphics = self.graphics.as_mut().ok_or("graphics not initialized")?;
        let window = self.window.as_ref().ok_or("window not initialized")?;

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

                {
                    let _render_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("main_render_pass"),
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
            _ => {
                // Timeout, occluded, validation: skip this frame.
            }
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, cause: winit::event::StartCause) {
        // In winit 0.31, StartCause::Init may not be delivered reliably.
        // Create the window on the first new_events call regardless of cause.
        if self.window.is_none() {
            tracing::info!("Creating window (first new_events, cause: {:?})", cause);
            self.init_window(event_loop);
        } else {
            tracing::debug!("new_events: {:?}", cause);
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
            // Fallback: create window here if new_events didn't trigger it
            tracing::info!("Creating window from about_to_wait");
            self.init_window(event_loop);
            return;
        }
        // Use Wait control flow to avoid CPU spinning. Redraw is requested
        // on-demand (SurfaceResized, RedrawRequested, etc.).
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    }
}

