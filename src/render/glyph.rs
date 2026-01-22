//! # Dynamic Font Rendering Module
//!
//! This module provides dynamic font rasterization capabilities for rust_pixel,
//! enabling high-quality text rendering with full Unicode/CJK support.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Glyph Rendering System                    │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌─────────────────┐    ┌─────────────────────────────────┐ │
//! │  │  GlyphRenderer  │    │     DynamicTextureAtlas         │ │
//! │  │                 │    │                                 │ │
//! │  │  - fontdue Font │───►│  - 1024x1024 texture            │ │
//! │  │  - LRU cache    │    │  - 32x32 glyph slots            │ │
//! │  │  - DPI aware    │    │  - Block-based layout           │ │
//! │  └─────────────────┘    └─────────────────────────────────┘ │
//! │           │                          │                      │
//! │           ▼                          ▼                      │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │              TextureUploader (trait)                    ││
//! │  │  - WgpuTextureUploader                                  ││
//! │  │  - GlowTextureUploader                                  ││
//! │  │  - SdlTextureUploader                                   ││
//! │  │  - WebGlTextureUploader                                 ││
//! │  └─────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Components
//!
//! - **GlyphSource**: Enum indicating where a glyph comes from (static atlas or dynamic)
//! - **GlyphRenderer**: Main interface for font rasterization and caching
//! - **DynamicTextureAtlas**: Manages the dynamic glyph texture with LRU eviction
//! - **TextureUploader**: Trait for backend-specific texture upload operations

use fontdue::{Font, FontSettings};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::Path;
use unicode_width::UnicodeWidthChar;

/// Default glyph cache capacity (number of cached glyphs)
pub const DEFAULT_GLYPH_CACHE_CAPACITY: usize = 1000;

/// Dynamic texture atlas size at 1x scale (pixels)
/// At higher DPI, this scales proportionally
pub const DYNAMIC_ATLAS_SIZE: u32 = 1024;

/// Glyph slot size in the dynamic atlas (pixels at 1x scale)
/// - Half-width characters (ASCII): 16x32, occupy left half of slot
/// - Full-width characters (CJK): 32x32, occupy entire slot
pub const GLYPH_SLOT_SIZE: u32 = 32;

/// Number of glyph slots per row in the atlas
pub const SLOTS_PER_ROW: u32 = DYNAMIC_ATLAS_SIZE / GLYPH_SLOT_SIZE; // 32

/// Total number of glyph slots in the atlas
pub const TOTAL_SLOTS: u32 = SLOTS_PER_ROW * SLOTS_PER_ROW; // 1024

/// UV rectangle for texture coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UVRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl UVRect {
    pub fn new(u0: f32, v0: f32, u1: f32, v1: f32) -> Self {
        Self { u0, v0, u1, v1 }
    }

    /// Create UV rect from slot index in the dynamic atlas
    pub fn from_slot(slot_idx: u32, is_fullwidth: bool) -> Self {
        let row = slot_idx / SLOTS_PER_ROW;
        let col = slot_idx % SLOTS_PER_ROW;

        let slot_uv_size = GLYPH_SLOT_SIZE as f32 / DYNAMIC_ATLAS_SIZE as f32;

        let u0 = col as f32 * slot_uv_size;
        let v0 = row as f32 * slot_uv_size;

        // Half-width uses left half, full-width uses entire slot
        let u1 = if is_fullwidth {
            u0 + slot_uv_size
        } else {
            u0 + slot_uv_size / 2.0
        };
        let v1 = v0 + slot_uv_size;

        Self { u0, v0, u1, v1 }
    }
}

/// Glyph source enumeration
///
/// Indicates where a rendered glyph comes from:
/// - Static atlas (symbols.png) for Sprites and Emoji
/// - Dynamic atlas for all text characters
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlyphSource {
    /// Static Sprite symbols from symbols.png (Block 0-47)
    /// texidx: block index (0-47)
    /// symidx: symbol index within block (0-255)
    SpriteAtlas { texidx: u8, symidx: u8 },

    /// Static Emoji from symbols.png (Block 53-55)
    /// texidx: block index (53-55)
    /// symidx: symbol index within block (0-63)
    EmojiAtlas { texidx: u8, symidx: u8 },

    /// Dynamically rasterized glyph
    /// slot_idx: index in the dynamic texture atlas (0-1023)
    /// is_fullwidth: true for CJK characters (32x32), false for ASCII (16x32)
    Dynamic { slot_idx: u32, is_fullwidth: bool },
}

impl GlyphSource {
    /// Check if this glyph uses the static atlas (symbols.png)
    pub fn is_static(&self) -> bool {
        matches!(self, GlyphSource::SpriteAtlas { .. } | GlyphSource::EmojiAtlas { .. })
    }

    /// Check if this glyph uses the dynamic atlas
    pub fn is_dynamic(&self) -> bool {
        matches!(self, GlyphSource::Dynamic { .. })
    }

    /// Get UV coordinates for dynamic glyphs
    pub fn get_dynamic_uv(&self) -> Option<UVRect> {
        if let GlyphSource::Dynamic { slot_idx, is_fullwidth } = self {
            Some(UVRect::from_slot(*slot_idx, *is_fullwidth))
        } else {
            None
        }
    }
}

/// Cached glyph information
#[derive(Debug, Clone)]
pub struct CachedGlyph {
    /// Slot index in the dynamic texture atlas
    pub slot_idx: u32,
    /// Whether this is a full-width character
    pub is_fullwidth: bool,
    /// Glyph metrics from fontdue
    pub metrics: fontdue::Metrics,
    /// Scale factor at which this glyph was rasterized
    pub scale_factor: f32,
}

/// Dynamic texture atlas for glyph caching
///
/// Manages a texture atlas with LRU eviction for dynamically rasterized glyphs.
/// Uses a block-based layout matching the existing symbols.png structure.
pub struct DynamicTextureAtlas {
    /// Atlas width in pixels (scaled by DPI)
    pub width: u32,
    /// Atlas height in pixels (scaled by DPI)
    pub height: u32,
    /// Current scale factor (DPI)
    pub scale_factor: f32,
    /// Bitmap data (RGBA format, 4 bytes per pixel)
    pub bitmap: Vec<u8>,
    /// Free slot indices (available for new glyphs)
    free_slots: Vec<u32>,
    /// Whether the texture needs to be re-uploaded to GPU
    pub dirty: bool,
    /// Dirty region for partial upload optimization (x, y, w, h)
    pub dirty_region: Option<(u32, u32, u32, u32)>,
}

impl DynamicTextureAtlas {
    /// Create a new dynamic texture atlas
    ///
    /// # Parameters
    /// - `scale_factor`: DPI scale factor (1.0 for standard, 2.0 for Retina, etc.)
    pub fn new(scale_factor: f32) -> Self {
        let scale = scale_factor.max(1.0);
        let width = (DYNAMIC_ATLAS_SIZE as f32 * scale) as u32;
        let height = (DYNAMIC_ATLAS_SIZE as f32 * scale) as u32;

        // Initialize bitmap with transparent black (RGBA)
        let bitmap = vec![0u8; (width * height * 4) as usize];

        // Initialize free slots (all slots available initially)
        let total_slots = TOTAL_SLOTS;
        let free_slots: Vec<u32> = (0..total_slots).collect();

        Self {
            width,
            height,
            scale_factor: scale,
            bitmap,
            free_slots,
            dirty: false,
            dirty_region: None,
        }
    }

    /// Allocate a slot for a new glyph
    ///
    /// Returns the slot index if available, or None if the atlas is full
    pub fn allocate_slot(&mut self) -> Option<u32> {
        self.free_slots.pop()
    }

    /// Free a slot (return it to the pool)
    pub fn free_slot(&mut self, slot_idx: u32) {
        if slot_idx < TOTAL_SLOTS {
            self.free_slots.push(slot_idx);
        }
    }

    /// Get pixel coordinates for a slot
    fn slot_to_pixels(&self, slot_idx: u32) -> (u32, u32) {
        let scaled_slot_size = (GLYPH_SLOT_SIZE as f32 * self.scale_factor) as u32;
        let slots_per_row = self.width / scaled_slot_size;

        let row = slot_idx / slots_per_row;
        let col = slot_idx % slots_per_row;

        (col * scaled_slot_size, row * scaled_slot_size)
    }

    /// Write glyph bitmap data to the atlas
    ///
    /// # Parameters
    /// - `slot_idx`: Target slot index
    /// - `glyph_bitmap`: Grayscale bitmap from fontdue (coverage values 0-255)
    /// - `glyph_width`: Width of the glyph bitmap
    /// - `glyph_height`: Height of the glyph bitmap
    /// - `offset_x`: X offset within the slot
    /// - `offset_y`: Y offset within the slot
    /// - `fg_color`: Foreground color (R, G, B)
    pub fn write_glyph(
        &mut self,
        slot_idx: u32,
        glyph_bitmap: &[u8],
        glyph_width: usize,
        glyph_height: usize,
        offset_x: u32,
        offset_y: u32,
        fg_color: (u8, u8, u8),
        is_fullwidth: bool,
    ) {
        let (slot_x, slot_y) = self.slot_to_pixels(slot_idx);
        let atlas_width = self.width as usize;

        // Clear the slot first (transparent)
        // For half-width, only clear left half; for full-width, clear entire slot
        let scaled_slot_size = (GLYPH_SLOT_SIZE as f32 * self.scale_factor) as u32;
        let target_width = if is_fullwidth {
            scaled_slot_size
        } else {
            scaled_slot_size / 2
        };

        for dy in 0..scaled_slot_size {
            for dx in 0..target_width {
                let px = slot_x + dx;
                let py = slot_y + dy;
                let idx = ((py as usize * atlas_width) + px as usize) * 4;
                if idx + 3 < self.bitmap.len() {
                    self.bitmap[idx] = 0;
                    self.bitmap[idx + 1] = 0;
                    self.bitmap[idx + 2] = 0;
                    self.bitmap[idx + 3] = 0;
                }
            }
        }

        // Write glyph pixels, clipping to target area boundaries
        // For half-width characters, clip to left half of slot (16px)
        // For full-width characters, clip to full slot (32px)
        let slot_right = slot_x + target_width;
        let slot_bottom = slot_y + scaled_slot_size;

        for gy in 0..glyph_height {
            for gx in 0..glyph_width {
                let coverage = glyph_bitmap[gy * glyph_width + gx];
                if coverage > 0 {
                    let px = slot_x + offset_x + gx as u32;
                    let py = slot_y + offset_y + gy as u32;

                    // Clip to target area boundaries to prevent bleeding
                    if px >= slot_x && px < slot_right && py >= slot_y && py < slot_bottom
                       && px < self.width && py < self.height {
                        let idx = ((py as usize * atlas_width) + px as usize) * 4;
                        if idx + 3 < self.bitmap.len() {
                            // Premultiplied alpha
                            let alpha = coverage as f32 / 255.0;
                            self.bitmap[idx] = (fg_color.0 as f32 * alpha) as u8;
                            self.bitmap[idx + 1] = (fg_color.1 as f32 * alpha) as u8;
                            self.bitmap[idx + 2] = (fg_color.2 as f32 * alpha) as u8;
                            self.bitmap[idx + 3] = coverage;
                        }
                    }
                }
            }
        }

        // Mark dirty region
        self.mark_dirty(slot_x, slot_y, target_width, scaled_slot_size);
    }

    /// Mark a region as dirty for partial upload
    fn mark_dirty(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.dirty = true;
        if let Some((dx, dy, dw, dh)) = self.dirty_region {
            // Expand dirty region to include new area
            let new_x = dx.min(x);
            let new_y = dy.min(y);
            let new_x2 = (dx + dw).max(x + w);
            let new_y2 = (dy + dh).max(y + h);
            self.dirty_region = Some((new_x, new_y, new_x2 - new_x, new_y2 - new_y));
        } else {
            self.dirty_region = Some((x, y, w, h));
        }
    }

    /// Clear dirty flag after upload
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
        self.dirty_region = None;
    }

    /// Resize the atlas for a new scale factor
    ///
    /// This clears all cached glyphs.
    pub fn resize(&mut self, new_scale_factor: f32) {
        let scale = new_scale_factor.max(1.0);
        let new_width = (DYNAMIC_ATLAS_SIZE as f32 * scale) as u32;
        let new_height = (DYNAMIC_ATLAS_SIZE as f32 * scale) as u32;

        self.width = new_width;
        self.height = new_height;
        self.scale_factor = scale;
        self.bitmap = vec![0u8; (new_width * new_height * 4) as usize];
        self.free_slots = (0..TOTAL_SLOTS).collect();
        self.dirty = true;
        self.dirty_region = Some((0, 0, new_width, new_height));
    }
}

/// Glyph cache key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Character codepoint
    pub ch: char,
    /// Scale factor (quantized to avoid floating point issues)
    /// Stored as scale * 100 (e.g., 1.0 -> 100, 2.0 -> 200)
    pub scale_100: u32,
}

impl GlyphKey {
    pub fn new(ch: char, scale_factor: f32) -> Self {
        Self {
            ch,
            scale_100: (scale_factor * 100.0).round() as u32,
        }
    }
}

/// Main glyph renderer
///
/// Manages font loading, glyph rasterization, and caching.
pub struct GlyphRenderer {
    /// The loaded font for text rendering
    font: Font,
    /// Font height in logical pixels (32px to match TUI character height)
    font_height: f32,
    /// Current DPI scale factor
    scale_factor: f32,
    /// Dynamic texture atlas
    pub atlas: DynamicTextureAtlas,
    /// LRU cache mapping characters to cached glyph info
    cache: LruCache<GlyphKey, CachedGlyph>,
}

/// Default font file path (relative to assets directory)
pub const DEFAULT_FONT_PATH: &str = "assets/fonts/default.otf";

impl GlyphRenderer {
    /// Create a new glyph renderer from font file data (bytes)
    ///
    /// # Parameters
    /// - `font_data`: TTF/OTF font file data
    /// - `scale_factor`: Initial DPI scale factor
    pub fn from_bytes(font_data: &[u8], scale_factor: f32) -> Result<Self, String> {
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| format!("Failed to load font: {}", e))?;

        let cache = LruCache::new(
            NonZeroUsize::new(DEFAULT_GLYPH_CACHE_CAPACITY)
                .expect("Cache capacity must be non-zero"),
        );

        Ok(Self {
            font,
            // Use 90% of slot height for larger glyphs with minimal padding
            // With a proper monospace font, characters fit well within slot width
            font_height: GLYPH_SLOT_SIZE as f32 * 0.90,
            scale_factor,
            atlas: DynamicTextureAtlas::new(scale_factor),
            cache,
        })
    }

    /// Create a new glyph renderer by loading font from file path
    ///
    /// # Parameters
    /// - `font_path`: Path to TTF/OTF font file
    /// - `scale_factor`: Initial DPI scale factor
    pub fn from_file<P: AsRef<Path>>(font_path: P, scale_factor: f32) -> Result<Self, String> {
        let font_data = std::fs::read(font_path.as_ref())
            .map_err(|e| format!("Failed to read font file {:?}: {}", font_path.as_ref(), e))?;
        Self::from_bytes(&font_data, scale_factor)
    }

    /// Create a glyph renderer with default font from assets directory
    ///
    /// Looks for font file at `assets/fonts/default.ttf`
    ///
    /// # Parameters
    /// - `base_path`: Base directory path (typically the project root)
    /// - `scale_factor`: Initial DPI scale factor
    pub fn with_default_font<P: AsRef<Path>>(base_path: P, scale_factor: f32) -> Result<Self, String> {
        let font_path = base_path.as_ref().join(DEFAULT_FONT_PATH);
        Self::from_file(&font_path, scale_factor)
    }

    /// Alias for from_bytes for backward compatibility
    pub fn new(font_data: &[u8], scale_factor: f32) -> Result<Self, String> {
        Self::from_bytes(font_data, scale_factor)
    }

    /// Get or rasterize a glyph
    ///
    /// Returns the GlyphSource for rendering. If the glyph is not cached,
    /// it will be rasterized and added to the cache.
    ///
    /// # Parameters
    /// - `ch`: The character to render
    ///
    /// # Returns
    /// GlyphSource indicating how to render this glyph
    pub fn get_glyph(&mut self, ch: char) -> GlyphSource {
        let key = GlyphKey::new(ch, self.scale_factor);

        // Check cache first
        if let Some(cached) = self.cache.get(&key) {
            return GlyphSource::Dynamic {
                slot_idx: cached.slot_idx,
                is_fullwidth: cached.is_fullwidth,
            };
        }

        // Need to rasterize
        self.rasterize_glyph(ch)
    }

    /// Rasterize a glyph and add it to the cache
    fn rasterize_glyph(&mut self, ch: char) -> GlyphSource {
        // Determine if this is a full-width character
        let is_fullwidth = ch.width().unwrap_or(1) >= 2;

        // Calculate pixel size for rasterization
        // Use consistent font height, then scale down wide glyphs if needed
        let pixel_size = self.font_height * self.scale_factor;

        // Rasterize the glyph
        let (metrics, bitmap) = self.font.rasterize(ch, pixel_size);

        // Allocate a slot
        let slot_idx = match self.atlas.allocate_slot() {
            Some(idx) => idx,
            None => {
                // Atlas is full, need to evict oldest entry
                if let Some((_, evicted)) = self.cache.pop_lru() {
                    self.atlas.free_slot(evicted.slot_idx);
                    self.atlas.allocate_slot().unwrap_or(0)
                } else {
                    // Cache is empty but atlas is full (shouldn't happen)
                    0
                }
            }
        };

        // Calculate offset to center the glyph in the slot
        let scaled_slot_size = (GLYPH_SLOT_SIZE as f32 * self.scale_factor) as u32;
        let glyph_target_width = if is_fullwidth {
            scaled_slot_size
        } else {
            scaled_slot_size / 2
        };

        // For half-width characters, allow up to 95% of slot width
        // With a monospace font, most characters fit well within this limit
        let max_glyph_width = (glyph_target_width as f32 * 0.95) as usize;

        // If glyph is too wide, re-rasterize at smaller size
        // Use >= to ensure we catch glyphs at exactly max width too (need some margin)
        let (final_metrics, final_bitmap) = if !is_fullwidth && metrics.width >= max_glyph_width {
            // Calculate scale factor to fit glyph within max width with 10% margin
            let target_width = (max_glyph_width as f32 * 0.90) as usize;
            let scale_down = target_width as f32 / metrics.width as f32;
            let smaller_size = pixel_size * scale_down;
            self.font.rasterize(ch, smaller_size)
        } else {
            (metrics, bitmap)
        };

        // Debug: log glyph metrics for problematic characters
        if ch == 'w' || ch == 'm' || ch == 'W' || ch == 'M' || self.cache.len() < 3 {
            log::info!(
                "Glyph '{}': orig_w={}, final_w={}, max_w={}, slot_w={}, is_fullwidth={}",
                ch, metrics.width, final_metrics.width, max_glyph_width, glyph_target_width, is_fullwidth
            );
        }

        // Center horizontally
        let offset_x = if final_metrics.width < glyph_target_width as usize {
            ((glyph_target_width as usize - final_metrics.width) / 2) as u32
        } else {
            0
        };

        // Align to baseline using font metrics
        // The baseline is positioned at approximately 75-80% from the top of the slot
        // This leaves room for ascenders above and descenders below
        //
        // fontdue metrics:
        // - ymin: vertical offset from baseline to bottom of glyph (negative for descenders)
        // - height: glyph bitmap height
        //
        // For proper baseline alignment:
        // baseline_y = slot_height * 0.75 (baseline at 75% from top)
        // glyph_top = baseline_y - (height + ymin) = baseline_y - height - ymin
        //           = baseline_y - height + |ymin| (since ymin is typically negative)
        let baseline_y = (scaled_slot_size as f32 * 0.78) as i32;
        let glyph_top = baseline_y - final_metrics.height as i32 - final_metrics.ymin;
        let offset_y = glyph_top.max(0) as u32;

        // Write to atlas (white foreground, shader will apply color)
        self.atlas.write_glyph(
            slot_idx,
            &final_bitmap,
            final_metrics.width,
            final_metrics.height,
            offset_x,
            offset_y,
            (255, 255, 255),
            is_fullwidth,
        );

        // Add to cache
        let key = GlyphKey::new(ch, self.scale_factor);
        let cached = CachedGlyph {
            slot_idx,
            is_fullwidth,
            metrics: final_metrics,
            scale_factor: self.scale_factor,
        };
        self.cache.put(key, cached);

        GlyphSource::Dynamic {
            slot_idx,
            is_fullwidth,
        }
    }

    /// Update scale factor (e.g., when window moves to different DPI display)
    ///
    /// This clears the cache and resizes the atlas.
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        if (self.scale_factor - scale_factor).abs() > 0.01 {
            self.scale_factor = scale_factor;
            self.atlas.resize(scale_factor);
            self.cache.clear();
        }
    }

    /// Get current scale factor
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Check if atlas texture needs re-upload
    pub fn is_dirty(&self) -> bool {
        self.atlas.dirty
    }

    /// Get atlas bitmap data for texture upload
    pub fn get_atlas_bitmap(&self) -> &[u8] {
        &self.atlas.bitmap
    }

    /// Get atlas dimensions
    pub fn get_atlas_size(&self) -> (u32, u32) {
        (self.atlas.width, self.atlas.height)
    }

    /// Clear dirty flag after texture upload
    pub fn clear_dirty(&mut self) {
        self.atlas.clear_dirty();
    }

    /// Preload common characters for faster initial rendering
    ///
    /// This preloads ASCII printable characters and optionally high-frequency CJK characters.
    pub fn preload_ascii(&mut self) {
        // Preload ASCII printable characters (32-126)
        for ch in ' '..='~' {
            let _ = self.get_glyph(ch);
        }
    }

    /// Preload a custom set of characters
    pub fn preload_chars(&mut self, chars: &str) {
        for ch in chars.chars() {
            let _ = self.get_glyph(ch);
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.cap().get())
    }
}

/// Trait for backend-specific texture upload operations
///
/// Each rendering backend (WGPU, Glow, SDL, WebGL) implements this trait
/// to handle uploading the dynamic glyph texture to the GPU.
pub trait TextureUploader {
    /// Upload the entire atlas texture to GPU
    fn upload_full(&mut self, bitmap: &[u8], width: u32, height: u32);

    /// Upload a partial region of the atlas texture (optimization)
    fn upload_region(&mut self, bitmap: &[u8], atlas_width: u32, x: u32, y: u32, w: u32, h: u32);

    /// Get the texture ID/handle for binding during rendering
    fn texture_id(&self) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_rect_from_slot() {
        // Test first slot (0)
        let uv = UVRect::from_slot(0, false);
        assert_eq!(uv.u0, 0.0);
        assert_eq!(uv.v0, 0.0);
        assert!((uv.u1 - 0.015625).abs() < 0.0001); // 1/64 for half-width

        // Test full-width
        let uv_full = UVRect::from_slot(0, true);
        assert!((uv_full.u1 - 0.03125).abs() < 0.0001); // 1/32 for full-width
    }

    #[test]
    fn test_glyph_source() {
        let sprite = GlyphSource::SpriteAtlas {
            texidx: 1,
            symidx: 10,
        };
        assert!(sprite.is_static());
        assert!(!sprite.is_dynamic());

        let dynamic = GlyphSource::Dynamic {
            slot_idx: 5,
            is_fullwidth: true,
        };
        assert!(!dynamic.is_static());
        assert!(dynamic.is_dynamic());
        assert!(dynamic.get_dynamic_uv().is_some());
    }

    #[test]
    fn test_dynamic_atlas_allocation() {
        let mut atlas = DynamicTextureAtlas::new(1.0);
        assert_eq!(atlas.width, DYNAMIC_ATLAS_SIZE);
        assert_eq!(atlas.height, DYNAMIC_ATLAS_SIZE);

        // Allocate all slots
        let mut allocated = Vec::new();
        while let Some(slot) = atlas.allocate_slot() {
            allocated.push(slot);
        }
        assert_eq!(allocated.len(), TOTAL_SLOTS as usize);

        // Free a slot and reallocate
        atlas.free_slot(100);
        let new_slot = atlas.allocate_slot();
        assert_eq!(new_slot, Some(100));
    }

    #[test]
    fn test_glyph_key() {
        let key1 = GlyphKey::new('A', 1.0);
        let key2 = GlyphKey::new('A', 1.0);
        let key3 = GlyphKey::new('A', 2.0);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(key1.scale_100, 100);
        assert_eq!(key3.scale_100, 200);
    }
}
