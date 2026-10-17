//! Application core: winit window + wgpu GPU context + render loop.
//!
//! Fetches a URL, parses HTML into a DOM tree, and renders structured
//! elements (headings, paragraphs, links, lists) with GPU text rendering.

use nabrowse_paint::{FontRasterizer, GraphicsContext};
use nabrowse_shared::{Color, Size};
use nabrowse_html::{HtmlParser, Node};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

/// A renderable element extracted from the DOM tree.
struct RenderElement {
    tag: String,
    text: String,
    href: Option<String>,
}

/// Style for a renderable element.
struct ElementStyle {
    font_size: f32,
    color: Color,
    indent: f32,
    prefix: String,
}

pub struct App {
    url: String,
    window: Option<Box<dyn Window>>,
    graphics: Option<GraphicsContext>,
    size: Size,
    clear_color: Color,
    exiting: bool,
    // Fetched page content
    render_elements: Vec<RenderElement>,
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
            render_elements: Vec::new(),
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
        if let Some(ref w) = self.window {
            w.request_redraw();
        }
    }

    fn fetch_and_parse(&mut self) {
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
                tracing::info!(
                    "Fetched {} (status: {}, {} bytes)",
                    self.url,
                    response.status,
                    response.body.len()
                );
                self.page_status = format!(
                    "{} | Status: {} | {} bytes",
                    self.url, response.status, response.body.len()
                );

                // Parse HTML into DOM tree
                let html = String::from_utf8_lossy(&response.body);
                let document = HtmlParser::new(html.to_string()).parse();

                // Walk DOM tree and collect renderable elements
                self.render_elements = Vec::new();
                for child in &document.children {
                    self.collect_elements(child);
                }

                tracing::info!("Collected {} render elements", self.render_elements.len());
            }
            Err(err) => {
                tracing::error!("Fetch failed: {}", err);
                self.page_status = format!("Error: {}", err);
                self.render_elements = vec![RenderElement {
                    tag: "error".to_string(),
                    text: self.page_status.clone(),
                    href: None,
                }];
            }
        }
    }

    /// Recursively walk the DOM tree and collect renderable elements.
    fn collect_elements(&mut self, node: &Node) {
        match node {
            Node::Element(el) => {
                let tag = el.tag_name.to_lowercase();

                // Skip non-rendered elements
                if matches!(
                    tag.as_str(),
                    "script" | "style" | "head" | "meta" | "link" | "title"
                ) {
                    return;
                }

                // Void elements (no text content)
                if matches!(
                    tag.as_str(),
                    "br" | "hr" | "img" | "input" | "area" | "base" | "col"
                        | "embed" | "param" | "source" | "track" | "wbr"
                ) {
                    return;
                }

                // Collect text from this element
                let text = self.extract_text(el);
                let href = el
                    .attributes
                    .iter()
                    .find(|(k, _)| k == "href")
                    .map(|(_, v)| v.clone());

                if !text.is_empty() {
                    self.render_elements.push(RenderElement {
                        tag: tag.clone(),
                        text,
                        href,
                    });
                }

                // Recurse into children
                for child in &el.children {
                    self.collect_elements(child);
                }
            }
            Node::Text(t) => {
                let text = t.content.trim();
                if !text.is_empty() {
                    self.render_elements.push(RenderElement {
                        tag: "text".to_string(),
                        text: text.to_string(),
                        href: None,
                    });
                }
            }
            _ => {}
        }
    }

    /// Extract text content from an element (including descendants).
    fn extract_text(&self, el: &nabrowse_html::Element) -> String {
        let mut text = String::new();
        for child in &el.children {
            match child {
                Node::Text(t) => {
                    text.push_str(&t.content);
                    text.push(' ');
                }
                Node::Element(child_el) => {
                    text.push_str(&self.extract_text(child_el));
                }
                _ => {}
            }
        }
        text.trim().to_string()
    }

    /// Get the style for an element based on its tag.
    fn get_style(element: &RenderElement) -> ElementStyle {
        if element.tag == "error" {
            return ElementStyle {
                font_size: 16.0,
                color: Color::new(0.8, 0.0, 0.0, 1.0),
                indent: 10.0,
                prefix: String::new(),
            };
        }

        match element.tag.as_str() {
            "h1" => ElementStyle {
                font_size: 28.0,
                color: Color::new(0.0, 0.0, 0.0, 1.0),
                indent: 10.0,
                prefix: String::new(),
            },
            "h2" => ElementStyle {
                font_size: 24.0,
                color: Color::new(0.0, 0.0, 0.0, 1.0),
                indent: 10.0,
                prefix: String::new(),
            },
            "h3" => ElementStyle {
                font_size: 20.0,
                color: Color::new(0.0, 0.0, 0.0, 1.0),
                indent: 10.0,
                prefix: String::new(),
            },
            "h4" | "h5" | "h6" => ElementStyle {
                font_size: 16.0,
                color: Color::new(0.1, 0.1, 0.1, 1.0),
                indent: 10.0,
                prefix: String::new(),
            },
            "a" => ElementStyle {
                font_size: 14.0,
                color: Color::new(0.0, 0.0, 0.8, 1.0),
                indent: 10.0,
                prefix: String::new(),
            },
            "li" => ElementStyle {
                font_size: 14.0,
                color: Color::new(0.0, 0.0, 0.0, 1.0),
                indent: 30.0,
                prefix: "\u{2022} ".to_string(),
            },
            "p" => ElementStyle {
                font_size: 14.0,
                color: Color::new(0.0, 0.0, 0.0, 1.0),
                indent: 10.0,
                prefix: String::new(),
            },
            _ => ElementStyle {
                font_size: 14.0,
                color: Color::new(0.0, 0.0, 0.0, 1.0),
                indent: 10.0,
                prefix: String::new(),
            },
        }
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let graphics = self.graphics.as_mut().ok_or("graphics not initialized")?;
        let window = self.window.as_ref().ok_or("window not initialized")?;
        let text_renderer = graphics
            .text_renderer
            .as_ref()
            .ok_or("text renderer not initialized")?;
        let rasterizer = self.font_rasterizer.clone();

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
                    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("clear_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
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
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                }

                // Draw status line
                if !self.page_status.is_empty() {
                    if let Some(ref r) = rasterizer {
                        let (bitmap, w, h) =
                            r.render_line(&self.page_status, Color::new(0.3, 0.3, 0.3, 1.0));
                        if w > 0 && h > 0 {
                            let text_texture = text_renderer.create_texture(&bitmap, w, h);
                            text_renderer.draw_texture(
                                &mut encoder,
                                &view,
                                &text_texture,
                                10.0,
                                5.0,
                                self.size.width as u32,
                                self.size.height as u32,
                            );
                        }
                    }
                }

                // Draw render elements
                if let Some(ref r) = rasterizer {
                    let mut y = 25.0;

                    for element in &self.render_elements {
                        let style = Self::get_style(element);
                        let line_height = style.font_size + 6.0;

                        if y + line_height > self.size.height {
                            break; // Stop if we run out of space
                        }

                        let text = if element.tag == "a" {
                            format!("{} [{}]", element.text, element.href.as_deref().unwrap_or(""))
                        } else {
                            element.text.clone()
                        };

                        let full_text = format!("{}{}", style.prefix, text);
                        let (bitmap, w, h) = r.render_line_with_size(
                            &full_text,
                            style.color,
                            style.font_size,
                        );
                        if w > 0 && h > 0 {
                            let text_texture = text_renderer.create_texture(&bitmap, w, h);
                            text_renderer.draw_texture(
                                &mut encoder,
                                &view,
                                &text_texture,
                                style.indent,
                                y,
                                self.size.width as u32,
                                self.size.height as u32,
                            );
                        }

                        y += line_height;
                    }
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
        if self.window.is_none() {
            tracing::info!("Creating window (first new_events, cause: {:?})", cause);
            self.init_window(event_loop);
            self.fetch_and_parse();
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
            self.fetch_and_parse();
            if let Some(ref w) = self.window {
                w.request_redraw();
            }
            return;
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    }
}
