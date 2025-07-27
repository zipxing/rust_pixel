// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # WGPU Texture Management Module
//! 
//! Handles texture creation, loading from files, format conversion, and
//! GPU texture resource management with automatic cleanup for WGPU.

/// WGPU Texture wrapper with render target support
/// 
/// This structure wraps WGPU texture resources and provides additional
/// functionality for render target management and texture operations.
pub struct WgpuTexture {
    /// WGPU texture object
    pub texture: wgpu::Texture,
    /// Texture view for shader access
    pub view: wgpu::TextureView,
    /// Optional sampler for texture filtering
    pub sampler: Option<wgpu::Sampler>,
    /// Texture width in pixels
    pub width: u32,
    /// Texture height in pixels
    pub height: u32,
}

/// WGPU Render Texture wrapper with OpenGL-compatible interface
/// 
/// This structure provides a render target texture similar to OpenGL's GlRenderTexture,
/// including support for hidden/visible state management for transition effects.
pub struct WgpuRenderTexture {
    /// WGPU texture object
    pub texture: wgpu::Texture,
    /// Texture view for rendering and shader access
    pub view: wgpu::TextureView,
    /// Optional sampler for texture filtering
    pub sampler: Option<wgpu::Sampler>,
    /// Texture width in pixels
    pub width: u32,
    /// Texture height in pixels
    pub height: u32,
    /// Whether this render texture is hidden from final rendering
    /// (matches OpenGL GlRenderTexture.is_hidden)
    pub is_hidden: bool,
}

impl WgpuTexture {
    /// Create a new render target texture
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `width`: Texture width in pixels
    /// - `height`: Texture height in pixels
    /// 
    /// # Returns
    /// New WgpuTexture instance configured as render target
    pub fn new_render_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RustPixel Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // Linear format to match GL mode
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            sampler: None,
            width,
            height,
        }
    }

    /// Create a new render target texture with custom format
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `width`: Texture width in pixels
    /// - `height`: Texture height in pixels
    /// - `format`: Texture format
    /// 
    /// # Returns
    /// New WgpuTexture instance configured as render target
    pub fn new_render_target_with_format(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RustPixel Custom Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            sampler: None,
            width,
            height,
        }
    }

    /// Create texture from image data
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `queue`: WGPU queue handle
    /// - `width`: Image width in pixels
    /// - `height`: Image height in pixels
    /// - `data`: Image data as RGBA bytes
    /// 
    /// # Returns
    /// New WgpuTexture instance with loaded image data
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RustPixel Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // Linear format to match GL mode
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            texture.as_image_copy(),
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RustPixel Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler: Some(sampler),
            width,
            height,
        }
    }

    /// Add sampler to texture
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    pub fn add_sampler(&mut self, device: &wgpu::Device) {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RustPixel Render Target Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixel art should use nearest neighbor
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        self.sampler = Some(sampler);
    }
}

impl WgpuRenderTexture {
    /// Create a new render texture with hidden state
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `width`: Texture width in pixels
    /// - `height`: Texture height in pixels
    /// - `is_hidden`: Whether this render texture should be hidden
    /// 
    /// # Returns
    /// New WgpuRenderTexture instance configured as render target
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        is_hidden: bool,
    ) -> Result<Self, String> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RustPixel Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // Linear format to match GL mode
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler for render texture
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RustPixel Render Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixel art should use nearest neighbor
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler: Some(sampler),
            width,
            height,
            is_hidden,
        })
    }

    /// Create a new render texture with custom format and hidden state
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `width`: Texture width in pixels
    /// - `height`: Texture height in pixels
    /// - `format`: Texture format
    /// - `is_hidden`: Whether this render texture should be hidden
    /// 
    /// # Returns
    /// New WgpuRenderTexture instance configured as render target
    pub fn new_with_format(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        is_hidden: bool,
    ) -> Result<Self, String> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RustPixel Custom Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler for render texture
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RustPixel Custom Render Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixel art should use nearest neighbor
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler: Some(sampler),
            width,
            height,
            is_hidden,
        })
    }

    /// Get the WGPU texture handle
    /// 
    /// # Returns
    /// Reference to the underlying WGPU texture
    pub fn get_texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Get the texture view for rendering
    /// 
    /// # Returns
    /// Reference to the texture view
    pub fn get_view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Get the texture sampler
    /// 
    /// # Returns
    /// Optional reference to the sampler
    pub fn get_sampler(&self) -> Option<&wgpu::Sampler> {
        self.sampler.as_ref()
    }

    /// Check if this render texture is hidden
    /// 
    /// # Returns
    /// True if hidden, false if visible
    pub fn is_hidden(&self) -> bool {
        self.is_hidden
    }

    /// Set the hidden state of this render texture
    /// 
    /// # Parameters
    /// - `hidden`: New hidden state
    pub fn set_hidden(&mut self, hidden: bool) {
        self.is_hidden = hidden;
    }

    /// Get texture dimensions
    /// 
    /// # Returns
    /// (width, height) tuple
    pub fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
} 