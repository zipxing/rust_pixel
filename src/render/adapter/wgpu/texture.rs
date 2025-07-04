// RustPixel
// copyright zipxing@hotmail.com 2022~2024

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
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
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
} 