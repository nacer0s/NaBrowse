//! Text rendering for NaBrowse.
//!
//! Rasterizes text into a bitmap using fontdue, uploads it to a GPU texture,
//! and draws it as a textured quad via a simple WGSL shader.
//!
//! Privacy: all rendering is local, no network, no telemetry.

use std::sync::Arc;

use nabrowse_shared::Color;
use wgpu::util::DeviceExt;

/// A rendered text line as a GPU texture.
pub struct TextTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

/// GPU text renderer: draws textured quads for rasterized text.
pub struct TextRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

/// Vertex for a textured quad (unit square: 0..1 in both axes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

/// Uniforms for positioning a quad on screen (pixel space → NDC).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct QuadUniforms {
    position: [f32; 2], // NDC top-left corner
    size: [f32; 2],     // NDC width/height
}

impl Vertex {
    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    shader_location: 0,
                    offset: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    shader_location: 1,
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// WGSL shader for textured quad rendering.
const TEXT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct QuadUniforms {
    position: vec2<f32>,
    size: vec2<f32>,
};

@group(0) @binding(0) var t_0: texture_2d<f32>;
@group(0) @binding(1) var s_0: sampler;
@group(1) @binding(0) var<uniform> quad_uniforms: QuadUniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let pos = quad_uniforms.position + input.position * quad_uniforms.size;
    output.position = vec4<f32>(pos, 0.0, 1.0);
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_0, s_0, input.uv);
}
"#;

impl TextRenderer {
    /// Create a new GPU text renderer.
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(TEXT_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text_pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertices: [Vertex; 4] = [
            Vertex { position: [0.0, 0.0], uv: [0.0, 0.0] },
            Vertex { position: [1.0, 0.0], uv: [1.0, 0.0] },
            Vertex { position: [1.0, 1.0], uv: [1.0, 1.0] },
            Vertex { position: [0.0, 1.0], uv: [0.0, 1.0] },
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_vertex_buffer"),
            contents: bytemuck_cast(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_index_buffer"),
            contents: bytemuck_cast(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            device,
            queue,
            pipeline,
            vertex_buffer,
            index_buffer,
            sampler,
        }
    }

    /// Create a GPU texture from an RGBA bitmap.
    pub fn create_texture(&self, bitmap: &[u8], width: u32, height: u32) -> TextTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("text_texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bitmap,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        TextTexture {
            texture,
            view,
            width,
            height,
        }
    }

    /// Draw a text texture as a quad at pixel position (x, y) on screen.
    pub fn draw_texture(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        text_texture: &TextTexture,
        x: f32,
        y: f32,
        screen_width: u32,
        screen_height: u32,
    ) {
        let uniforms = QuadUniforms {
            position: [
                x / screen_width as f32 * 2.0 - 1.0,
                1.0 - y / screen_height as f32 * 2.0,
            ],
            size: [
                text_texture.width as f32 / screen_width as f32 * 2.0,
                text_texture.height as f32 / screen_height as f32 * 2.0,
            ],
        };

        let uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_uniforms"),
            size: std::mem::size_of::<QuadUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.queue.write_buffer(&uniform_buffer, 0, bytemuck_cast(&[uniforms]));

        let uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad_uniform_bind_group"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&text_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("text_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &texture_bind_group, &[]);
        render_pass.set_bind_group(1, &uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

/// Helper to cast a slice to bytes without bytemuck dependency.
fn bytemuck_cast<T>(data: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            std::mem::size_of_val(data),
        )
    }
}

/// Font rasterizer: converts text lines to RGBA bitmaps.
#[derive(Clone)]
pub struct FontRasterizer {
    font: fontdue::Font,
    font_size: f32,
}

impl FontRasterizer {
    /// Create a font rasterizer, loading a system font.
    pub fn new() -> Option<Self> {
        let font = load_system_font()?;
        Some(Self {
            font,
            font_size: 14.0,
        })
    }

    /// Set the font size in pixels.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Get the line height in pixels.
    pub fn line_height(&self) -> u32 {
        self.font_size as u32 + 4
    }

    /// Measure the pixel width of a line of text.
    pub fn measure_line(&self, text: &str) -> u32 {
        let mut width = 0.0f32;
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, self.font_size);
            width += metrics.advance_width;
        }
        width as u32
    }

    /// Render a single line of text to an RGBA bitmap (with color applied).
    pub fn render_line(&self, text: &str, color: Color) -> (Vec<u8>, u32, u32) {
        let line_height = self.line_height();
        let width = self.measure_line(text).max(1);

        let mut bitmap = vec![0u8; (width * line_height * 4) as usize];

        let mut x_offset = 0.0f32;
        for ch in text.chars() {
            let (metrics, glyph_bitmap) = self.font.rasterize(ch, self.font_size);
            let glyph_width = metrics.width as u32;
            let glyph_height = metrics.height as u32;

            for gy in 0..glyph_height {
                for gx in 0..glyph_width {
                    let glyph_idx = (gy * glyph_width + gx) as usize;
                    let alpha = glyph_bitmap[glyph_idx];
                    if alpha > 0 {
                        let bx = (x_offset as u32 + gx).min(width - 1);
                        let by = gy.min(line_height - 1);
                        let idx = ((by * width + bx) * 4) as usize;
                        bitmap[idx] = (color.r * 255.0) as u8;
                        bitmap[idx + 1] = (color.g * 255.0) as u8;
                        bitmap[idx + 2] = (color.b * 255.0) as u8;
                        bitmap[idx + 3] = alpha;
                    }
                }
            }

            x_offset += metrics.advance_width;
        }

        (bitmap, width, line_height)
    }

    /// Render a line with a custom font size.
    pub fn render_line_with_size(
        &self,
        text: &str,
        color: Color,
        size: f32,
    ) -> (Vec<u8>, u32, u32) {
        let line_height = size as u32 + 6;
        let width = self.measure_line_with_size(text, size).max(1);

        let mut bitmap = vec![0u8; (width * line_height * 4) as usize];

        let mut x_offset = 0.0f32;
        for ch in text.chars() {
            let (metrics, glyph_bitmap) = self.font.rasterize(ch, size);
            let glyph_width = metrics.width as u32;
            let glyph_height = metrics.height as u32;

            for gy in 0..glyph_height {
                for gx in 0..glyph_width {
                    let glyph_idx = (gy * glyph_width + gx) as usize;
                    let alpha = glyph_bitmap[glyph_idx];
                    if alpha > 0 {
                        let bx = (x_offset as u32 + gx).min(width - 1);
                        let by = gy.min(line_height - 1);
                        let idx = ((by * width + bx) * 4) as usize;
                        bitmap[idx] = (color.r * 255.0) as u8;
                        bitmap[idx + 1] = (color.g * 255.0) as u8;
                        bitmap[idx + 2] = (color.b * 255.0) as u8;
                        bitmap[idx + 3] = alpha;
                    }
                }
            }

            x_offset += metrics.advance_width;
        }

        (bitmap, width, line_height)
    }

    /// Measure the pixel width of a line at a given font size.
    fn measure_line_with_size(&self, text: &str, size: f32) -> u32 {
        let mut width = 0.0f32;
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, size);
            width += metrics.advance_width;
        }
        width as u32
    }

    /// Render multiple lines to a single tall bitmap.
    pub fn render_lines(&self, lines: &[String], color: Color) -> (Vec<u8>, u32, u32) {
        let line_height = self.line_height();
        let max_width = lines
            .iter()
            .map(|l| self.measure_line(l))
            .max()
            .unwrap_or(1);
        let height = lines.len() as u32 * line_height;

        let mut bitmap = vec![0u8; (max_width * height * 4) as usize];

        for (i, line) in lines.iter().enumerate() {
            let y_offset = i as u32 * line_height;
            let (line_bitmap, line_width, lh) = self.render_line_color(line, color);

            // Copy line bitmap into the combined bitmap
            for y in 0..lh {
                for x in 0..line_width {
                    let src_idx = ((y * line_width + x) * 4) as usize;
                    let dst_idx = (((y_offset + y) * max_width + x) * 4) as usize;
                    if dst_idx + 3 < bitmap.len() && src_idx + 3 < line_bitmap.len() {
                        bitmap[dst_idx] = line_bitmap[src_idx];
                        bitmap[dst_idx + 1] = line_bitmap[src_idx + 1];
                        bitmap[dst_idx + 2] = line_bitmap[src_idx + 2];
                        bitmap[dst_idx + 3] = line_bitmap[src_idx + 3];
                    }
                }
            }
        }

        (bitmap, max_width, height)
    }

    /// Render a line with color applied, returning raw RGBA data.
    fn render_line_color(&self, text: &str, color: Color) -> (Vec<u8>, u32, u32) {
        let line_height = self.line_height();
        let width = self.measure_line(text).max(1);

        let mut bitmap = vec![0u8; (width * line_height * 4) as usize];

        let mut x_offset = 0.0f32;
        for ch in text.chars() {
            let (metrics, glyph_bitmap) = self.font.rasterize(ch, self.font_size);
            let glyph_width = metrics.width as u32;
            let glyph_height = metrics.height as u32;

            for gy in 0..glyph_height {
                for gx in 0..glyph_width {
                    let glyph_idx = (gy * glyph_width + gx) as usize;
                    let alpha = glyph_bitmap[glyph_idx];
                    if alpha > 0 {
                        let bx = (x_offset as u32 + gx).min(width - 1);
                        let by = gy.min(line_height - 1);
                        let idx = ((by * width + bx) * 4) as usize;
                        bitmap[idx] = (color.r * 255.0) as u8;
                        bitmap[idx + 1] = (color.g * 255.0) as u8;
                        bitmap[idx + 2] = (color.b * 255.0) as u8;
                        bitmap[idx + 3] = alpha;
                    }
                }
            }

            x_offset += metrics.advance_width;
        }

        (bitmap, width, line_height)
    }
}

/// Load a system font from common paths.
fn load_system_font() -> Option<fontdue::Font> {
    let font_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/Library/Fonts/Arial.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];

    for path in &font_paths {
        if let Ok(bytes) = std::fs::read(path) {
            match fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                Ok(font) => {
                    tracing::info!("Loaded font from: {}", path);
                    return Some(font);
                }
                Err(_) => continue,
            }
        }
    }

    tracing::error!("No system font found! Tried: {:?}", font_paths);
    None
}

/// Simple HTML-to-text conversion: strips tags, decodes common entities.
pub fn html_to_text(html: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_tag = false;
    let mut current_line = String::new();

    let mut chars = html.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if ch == '>' {
            in_tag = false;
            continue;
        }
        if in_tag {
            continue;
        }

        match ch {
            '\n' | '\r' => {
                result.push(current_line.clone());
                current_line.clear();
            }
            '&' => {
                // Decode common HTML entities
                let mut entity = String::new();
                for c in chars.by_ref() {
                    if c == ';' {
                        break;
                    }
                    entity.push(c);
                }
                match entity.as_str() {
                    "amp" => current_line.push('&'),
                    "lt" => current_line.push('<'),
                    "gt" => current_line.push('>'),
                    "quot" => current_line.push('"'),
                    "apos" => current_line.push('\''),
                    "nbsp" => current_line.push(' '),
                    _ => {}
                }
            }
            _ => {
                current_line.push(ch);
            }
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    // Trim whitespace from each line and filter empty ones
    result
        .into_iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}
