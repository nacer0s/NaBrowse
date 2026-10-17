//! GPU paint pipeline for NaBrowse.
//!
//! Wraps wgpu device, queue, surface, and swap chain. Provides resize and
//! basic frame presentation. This is the foundation the layout/paint engine
//! will draw into.

use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::window::Window;

/// Graphics context: device, queue, surface, and configuration.
pub struct GraphicsContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl GraphicsContext {
    /// Create a new graphics context from raw window handles.
    ///
    /// The handles must remain valid for the lifetime of the graphics context.
    pub async fn new(
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::empty(),
            backend_options: Default::default(),
            display: None,
            memory_budget_thresholds: Default::default(),
        });

        // Safety: the raw handles must outlive the surface. The caller owns
        // the window and guarantees the handles remain valid.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(raw_display_handle),
                raw_window_handle,
            })
        }?;

        let _size = winit::dpi::PhysicalSize::new(1, 1); // placeholder, resized by caller

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("failed to request adapter: {:?}", e))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("nabrowse_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: 1,
            height: 1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size: _size,
        })
    }

    /// Create from a winit window (convenience wrapper).
    pub async fn from_window(window: &dyn Window) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw_display_handle: RawDisplayHandle = window.display_handle()?.into();
        let raw_window_handle: RawWindowHandle = window.window_handle()?.into();
        Self::new(raw_display_handle, raw_window_handle).await
    }

    /// Resize the surface and swap chain.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.size = winit::dpi::PhysicalSize::new(width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}
