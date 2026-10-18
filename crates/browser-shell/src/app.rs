//! Application core: winit window + wgpu GPU context + render loop.
//!
//! Fetches a URL, parses HTML into a DOM tree, and renders structured
//! elements (headings, paragraphs, links, lists) with GPU text rendering.

use nabrowse_paint::{FontRasterizer, GraphicsContext, TextRenderer};
use nabrowse_shared::{Color, Size};
use nabrowse_html::{HtmlParser, Node};
use nabrowse_css::{CssParser, Stylesheet};
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, ElementState, MouseButton, KeyEvent, ButtonSource};
use winit::keyboard::{Key, NamedKey};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

/// A renderable element extracted from the DOM tree.
struct RenderElement {
    tag: String,
    text: String,
    href: Option<String>,
    classes: Vec<String>,
    id: Option<String>,
}

/// Style for a renderable element.
struct ElementStyle {
    font_size: f32,
    color: Color,
    background_color: Option<Color>,
    margin_top: f32,
    margin_bottom: f32,
    padding_top: f32,
    padding_bottom: f32,
    indent: f32,
    prefix: String,
}

/// A browser tab with its own page state.
struct Tab {
    url: String,
    render_elements: Vec<RenderElement>,
    page_status: String,
    stylesheet: Option<nabrowse_css::Stylesheet>,
}

impl Tab {
    fn new(url: String) -> Self {
        Self {
            url,
            render_elements: Vec::new(),
            page_status: String::new(),
            stylesheet: None,
        }
    }
}

pub struct App {
    window: Option<Box<dyn Window>>,
    graphics: Option<GraphicsContext>,
    size: Size,
    clear_color: Color,
    exiting: bool,
    font_rasterizer: Option<FontRasterizer>,
    // Navigation history
    history: Vec<String>,
    history_index: usize,
    // Tabs
    tabs: Vec<Tab>,
    current_tab: usize,
    // URL bar
    url_input: String,
    url_bar_focused: bool,
    cursor_pos: Option<(f32, f32)>,
}

impl App {
    pub fn new(url: String) -> Self {
        Self {
            window: None,
            graphics: None,
            size: Size::new(1024.0, 768.0),
            clear_color: Color::nabrowse_bg(),
            exiting: false,
            font_rasterizer: None,
            history: vec![url.clone()],
            history_index: 0,
            tabs: vec![Tab::new(url.clone())],
            current_tab: 0,
            url_input: url,
            url_bar_focused: false,
            cursor_pos: None,
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
        let url = self.tabs[self.current_tab].url.clone();
        tracing::info!("Fetching: {}", url);
        self.tabs[self.current_tab].page_status = format!("Fetching {}...", self.tabs[self.current_tab].url);

        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                self.tabs[self.current_tab].page_status = format!("Failed to create runtime: {}", err);
                return;
            }
        };

        match runtime.block_on(async {
            let client = nabrowse_network::NetworkClient::new()?;
            client.fetch(&url).await
        }) {
            Ok(response) => {
                tracing::info!(
                    "Fetched {} (status: {}, {} bytes)",
                    self.tabs[self.current_tab].url,
                    response.status,
                    response.body.len()
                );
                self.tabs[self.current_tab].page_status = format!(
                    "{} | Status: {} | {} bytes",
                    self.tabs[self.current_tab].url, response.status, response.body.len()
                );

                // Parse HTML into DOM tree
                let html = String::from_utf8_lossy(&response.body);
                let document = HtmlParser::new(html.to_string()).parse();

                // Extract CSS from <style> tags
                let mut css_sources = Vec::new();
                for child in &document.children {
                    Self::extract_stylesheets(child, &mut css_sources);
                }
                let combined_css = css_sources.join("\n");
                if !combined_css.is_empty() {
                    let stylesheet = CssParser::new(combined_css).parse();
                    tracing::info!("Parsed {} CSS rules", stylesheet.rules.len());
                    self.tabs[self.current_tab].stylesheet = Some(stylesheet);
                } else {
                    self.tabs[self.current_tab].stylesheet = Some(nabrowse_css::Stylesheet::default());
                }

                // Walk DOM tree and collect renderable elements
                self.tabs[self.current_tab].render_elements = Vec::new();
                for child in &document.children {
                    self.collect_elements(child);
                }

                tracing::info!("Collected {} render elements", self.tabs[self.current_tab].render_elements.len());
            }
            Err(err) => {
                tracing::error!("Fetch failed: {}", err);
                self.tabs[self.current_tab].page_status = format!("Error: {}", err);
                self.tabs[self.current_tab].render_elements = vec![RenderElement {
                    tag: "error".to_string(),
                    text: self.tabs[self.current_tab].page_status.clone(),
                    href: None,
                    classes: Vec::new(),
                    id: None,
                }];
            }
        }
    }


    /// Navigate to a new URL, truncating forward history.
    fn navigate_to(&mut self, url: String) {
        // Truncate forward history
        self.history.truncate(self.history_index + 1);
        self.history.push(url.clone());
        self.history_index += 1;

        // Update current tab
        self.tabs[self.current_tab].url = url.clone();
        self.tabs[self.current_tab].page_status = String::new();
        self.tabs[self.current_tab].render_elements = Vec::new();
        self.tabs[self.current_tab].stylesheet = None;

        // Update URL bar
        self.url_input = url.clone();
        self.url_bar_focused = false;

        // Fetch new page
        self.fetch_and_parse();

        // Request redraw
        if let Some(ref w) = self.window {
            w.request_redraw();
        }
    }

    /// Navigate back in history.
    fn go_back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            let url = self.history[self.history_index].clone();
            self.tabs[self.current_tab].url = url.clone();
            self.tabs[self.current_tab].page_status = String::new();
            self.tabs[self.current_tab].render_elements = Vec::new();
            self.tabs[self.current_tab].stylesheet = None;
            self.url_input = url.clone();
            self.url_bar_focused = false;
            self.fetch_and_parse();
            if let Some(ref w) = self.window {
                w.request_redraw();
            }
        }
    }

    /// Navigate forward in history.
    fn go_forward(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            let url = self.history[self.history_index].clone();
            self.tabs[self.current_tab].url = url.clone();
            self.tabs[self.current_tab].page_status = String::new();
            self.tabs[self.current_tab].render_elements = Vec::new();
            self.tabs[self.current_tab].stylesheet = None;
            self.url_input = url.clone();
            self.url_bar_focused = false;
            self.fetch_and_parse();
            if let Some(ref w) = self.window {
                w.request_redraw();
            }
        }
    }

    /// Switch to a different tab.
    fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.current_tab = index;
            self.url_input = self.tabs[index].url.clone();
            if let Some(ref w) = self.window {
                w.request_redraw();
            }
        }
    }

    /// Open a new tab.
    fn new_tab(&mut self) {
        self.tabs.push(Tab::new("about:blank".to_string()));
        self.current_tab = self.tabs.len() - 1;
        self.url_input = "about:blank".to_string();
        if let Some(ref w) = self.window {
            w.request_redraw();
        }
    }

    /// Handle keyboard input for the URL bar.
    fn handle_keyboard(&mut self, event: &KeyEvent) {
        if !self.url_bar_focused {
            return;
        }
        match &event.logical_key {
            Key::Character(ch) => {
                if event.state == ElementState::Pressed {
                    self.url_input.push_str(ch);
                }
            }
            Key::Named(NamedKey::Enter) => {
                if event.state == ElementState::Pressed {
                    let url = self.url_input.clone();
                    self.navigate_to(url);
                }
            }
            Key::Named(NamedKey::Backspace) => {
                if event.state == ElementState::Pressed {
                    self.url_input.pop();
                }
            }
            _ => {}
        }
    }

    /// Handle mouse clicks on UI elements.
    fn handle_mouse_click(&mut self, x: f32, y: f32) {
        // Tab bar area: y=0..30
        if y < 30.0 {
            let tab_width = 100.0;
            let tab_index = (x / tab_width) as usize;
            if tab_index < self.tabs.len() {
                self.switch_tab(tab_index);
            } else if x >= self.tabs.len() as f32 * tab_width
                && x < self.tabs.len() as f32 * tab_width + 30.0
            {
                self.new_tab();
            }
            return;
        }

        // URL bar area: y=30..60
        if y >= 30.0 && y < 60.0 {
            // Back button: x=5..30
            if x >= 5.0 && x < 30.0 {
                self.go_back();
                return;
            }
            // Forward button: x=30..55
            if x >= 30.0 && x < 55.0 {
                self.go_forward();
                return;
            }
            // URL input area: x=55..
            self.url_bar_focused = true;
            return;
        }

        // Clicking elsewhere unfocuses URL bar
        self.url_bar_focused = false;
    }

    /// Draw the browser UI (tab bar, URL bar, nav buttons).
    fn draw_ui(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, text_renderer: &TextRenderer, rasterizer: &FontRasterizer, tabs: &[Tab], url_input: &str, url_bar_focused: bool, size: Size) {
        let w = size.width as u32;
        let h = size.height as u32;

        // --- Tab bar background ---
        let tab_bg = vec![200u8; (w * 30 * 4) as usize];
        let tab_bg_tex = text_renderer.create_texture(&tab_bg, w, 30);
        text_renderer.draw_texture(encoder, view, &tab_bg_tex, 0.0, 0.0, w, h);

        // Draw tab labels
        let tab_width = 100.0;
        for (i, tab) in tabs.iter().enumerate() {
            let label = if tab.url.len() > 12 {
                format!("{}...", &tab.url[..12])
            } else {
                tab.url.clone()
            };
            let (bitmap, bw, bh) = rasterizer.render_line(
                &label,
                Color::new(0.0, 0.0, 0.0, 1.0),
            );
            if bw > 0 && bh > 0 {
                let tex = text_renderer.create_texture(&bitmap, bw, bh);
                text_renderer.draw_texture(
                    encoder, view, &tex,
                    i as f32 * tab_width + 5.0, 5.0, w, h,
                );
            }
        }

        // --- New tab button (+) ---
        let (bitmap, bw, bh) = rasterizer.render_line("+", Color::new(0.0, 0.0, 0.0, 1.0));
        if bw > 0 && bh > 0 {
            let tex = text_renderer.create_texture(&bitmap, bw, bh);
            text_renderer.draw_texture(
                encoder, view, &tex,
                tabs.len() as f32 * tab_width + 5.0, 5.0, w, h,
            );
        }

        // --- URL bar background ---
        let url_bg = vec![240u8; (w * 30 * 4) as usize];
        let url_bg_tex = text_renderer.create_texture(&url_bg, w, 30);
        text_renderer.draw_texture(encoder, view, &url_bg_tex, 0.0, 30.0, w, h);

        // --- Back button (<) ---
        let (bitmap, bw, bh) = rasterizer.render_line("<", Color::new(0.0, 0.0, 0.0, 1.0));
        if bw > 0 && bh > 0 {
            let tex = text_renderer.create_texture(&bitmap, bw, bh);
            text_renderer.draw_texture(encoder, view, &tex, 8.0, 35.0, w, h);
        }

        // --- Forward button (>) ---
        let (bitmap, bw, bh) = rasterizer.render_line(">", Color::new(0.0, 0.0, 0.0, 1.0));
        if bw > 0 && bh > 0 {
            let tex = text_renderer.create_texture(&bitmap, bw, bh);
            text_renderer.draw_texture(encoder, view, &tex, 33.0, 35.0, w, h);
        }

        // --- URL text ---
        let display_url = if url_bar_focused {
            format!("{}_", url_input)
        } else {
            url_input.to_string()
        };
        let (bitmap, bw, bh) = rasterizer.render_line(
            &display_url,
            Color::new(0.0, 0.0, 0.0, 1.0),
        );
        if bw > 0 && bh > 0 {
            let tex = text_renderer.create_texture(&bitmap, bw, bh);
            text_renderer.draw_texture(encoder, view, &tex, 60.0, 35.0, w, h);
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
                let classes: Vec<String> = el
                    .attributes
                    .iter()
                    .find(|(k, _)| k == "class")
                    .map(|(_, v)| v.split_whitespace().map(|s| s.to_string()).collect())
                    .unwrap_or_default();
                let id = el
                    .attributes
                    .iter()
                    .find(|(k, _)| k == "id")
                    .map(|(_, v)| v.clone());

                if !text.is_empty() {
                    self.tabs[self.current_tab].render_elements.push(RenderElement {
                        tag: tag.clone(),
                        text,
                        href,
                        classes,
                        id,
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
                    self.tabs[self.current_tab].render_elements.push(RenderElement {
                        tag: "text".to_string(),
                        text: text.to_string(),
                        href: None,
                        classes: Vec::new(),
                        id: None,
                    });
                }
            }
            _ => {}
        }
    }

    /// Extract CSS from all <style> tags in the document.
    fn extract_stylesheets(node: &Node, css_sources: &mut Vec<String>) {
        if let Node::Element(el) = node {
            if el.tag_name == "style" {
                let mut css_text = String::new();
                Self::collect_text_nodes(el, &mut css_text);
                css_sources.push(css_text);
            }
            for child in &el.children {
                Self::extract_stylesheets(child, css_sources);
            }
        }
    }

    /// Collect text content from an element's children (recursively).
    fn collect_text_nodes(el: &nabrowse_html::Element, text: &mut String) {
        for child in &el.children {
            match child {
                Node::Text(t) => text.push_str(&t.content),
                Node::Element(el) => Self::collect_text_nodes(el, text),
                _ => {}
            }
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

    /// Wrap text into lines that fit within the available width.
    fn wrap_text(text: &str, font_size: f32, indent: f32, screen_width: f32) -> Vec<String> {
        let max_width = (screen_width - indent - 10.0).max(50.0) as u32;
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            let candidate = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };
            // Approximate width: each char is roughly 0.6 * font_size
            let candidate_width = (candidate.len() as f32 * font_size * 0.6) as u32;
            if candidate_width <= max_width {
                current_line = candidate;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                }
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    /// Get the style for an element using the CSS engine.
    fn get_style(element: &RenderElement, stylesheet: &nabrowse_css::Stylesheet) -> ElementStyle {
        if element.tag == "error" {
            return ElementStyle {
                font_size: 16.0,
                color: Color::new(0.8, 0.0, 0.0, 1.0),
                background_color: None,
                margin_top: 0.0,
                margin_bottom: 0.0,
                padding_top: 0.0,
                padding_bottom: 0.0,
                indent: 10.0,
                prefix: String::new(),
            };
        }

        let engine = nabrowse_css::StyleEngine::new(nabrowse_css::Stylesheet {
            rules: stylesheet.rules.clone(),
        });
        let computed = engine.compute(&element.tag, &element.classes, element.id.as_deref(), None);

        // Base styles by tag (used as fallback)
        let (base_size, base_color) = match element.tag.as_str() {
            "h1" => (28.0, Color::new(0.0, 0.0, 0.0, 1.0)),
            "h2" => (24.0, Color::new(0.0, 0.0, 0.0, 1.0)),
            "h3" => (20.0, Color::new(0.0, 0.0, 0.0, 1.0)),
            "h4" | "h5" | "h6" => (16.0, Color::new(0.1, 0.1, 0.1, 1.0)),
            "a" => (14.0, Color::new(0.0, 0.0, 0.8, 1.0)),
            "li" => (14.0, Color::new(0.0, 0.0, 0.0, 1.0)),
            "p" => (14.0, Color::new(0.0, 0.0, 0.0, 1.0)),
            _ => (14.0, Color::new(0.0, 0.0, 0.0, 1.0)),
        };

        let font_size = computed.font_size.unwrap_or(base_size);
        let color = computed.color.unwrap_or(base_color);
        let background_color = computed.background_color;
        let indent = 10.0 + computed.margin.left + computed.padding.left;
        let prefix = if element.tag == "li" { "\u{2022} ".to_string() } else { String::new() };

        ElementStyle {
            font_size,
            color,
            background_color,
            margin_top: computed.margin.top,
            margin_bottom: computed.margin.bottom,
            padding_top: computed.padding.top,
            padding_bottom: computed.padding.bottom,
            indent,
            prefix,
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

                // Draw browser UI (tab bar, URL bar, nav buttons)
                {
                    if let Some(ref r) = rasterizer {
                        Self::draw_ui(&mut encoder, &view, text_renderer, r, &self.tabs, &self.url_input, self.url_bar_focused, self.size);
                    }
                }

                // Draw status line
                if !self.tabs[self.current_tab].page_status.is_empty() {
                    if let Some(ref r) = rasterizer {
                        let (bitmap, w, h) =
                            r.render_line(&self.tabs[self.current_tab].page_status, Color::new(0.3, 0.3, 0.3, 1.0));
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
                    let empty = Stylesheet::default();
                    let stylesheet = self.tabs[self.current_tab].stylesheet.as_ref().unwrap_or(&empty);
                    let mut y = 25.0;

                    for element in &self.tabs[self.current_tab].render_elements {
                        let style = Self::get_style(element, stylesheet);
                        let line_height = style.font_size + 6.0;
                        // Apply CSS margin-top
                        y += style.margin_top;
                        let element_start_y = y;

                        if y + line_height > self.size.height {
                            break;
                        }

                        // Draw background rectangle if element has a background color
                        if let Some(bg) = style.background_color {
                            let text = if element.tag == "a" {
                                format!("{} [{}]", element.text, element.href.as_deref().unwrap_or(""))
                            } else {
                                element.text.clone()
                            };
                            let full_text = format!("{}{}", style.prefix, text);
                            let text_width = r.measure_line_with_size(&full_text, style.font_size) as f32;
                            let bg_width = text_width + 20.0;
                            let bg_height = line_height + style.padding_top + style.padding_bottom;

                            let mut bg_rgba = Vec::with_capacity(bg_width as usize * bg_height as usize * 4);
                            for _ in 0..(bg_width as usize * bg_height as usize) {
                                bg_rgba.push((bg.r * 255.0) as u8);
                                bg_rgba.push((bg.g * 255.0) as u8);
                                bg_rgba.push((bg.b * 255.0) as u8);
                                bg_rgba.push((bg.a * 255.0) as u8);
                            }
                            let bg_texture = text_renderer.create_texture(&bg_rgba, bg_width as u32, bg_height as u32);
                            text_renderer.draw_texture(
                                &mut encoder, &view, &bg_texture,
                                style.indent - 10.0, element_start_y,
                                self.size.width as u32, self.size.height as u32,
                            );
                        }

                        // Apply CSS padding-top
                        y += style.padding_top;

                        let text = if element.tag == "a" {
                            format!("{} [{}]", element.text, element.href.as_deref().unwrap_or(""))
                        } else {
                            element.text.clone()
                        };

                        let full_text = format!("{}{}", style.prefix, text);
                        let wrapped_lines = Self::wrap_text(&full_text, style.font_size, style.indent, self.size.width);

                        for line in &wrapped_lines {
                            let (bitmap, w, h) = r.render_line_with_size(
                                line,
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

                        // Apply CSS padding-bottom
                        y += style.padding_bottom;
                        y += line_height;
                        // Apply CSS margin-bottom
                        y += style.margin_bottom;
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
            WindowEvent::PointerButton { state, button, .. } => {
                if state == ElementState::Pressed && button == ButtonSource::Mouse(MouseButton::Left) {
                    if let Some((x, y)) = self.cursor_pos {
                        self.handle_mouse_click(x, y);
                    }
                }
            }
            WindowEvent::PointerMoved { position, .. } => {
                // Track cursor position for click handling
                self.cursor_pos = Some((position.x as f32, position.y as f32));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard(&event);
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
