// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Texture configuration for symbol generation

/// Texture configuration, supports 4096 and 8192 sizes
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TextureConfig {
    pub size: u32,
    pub scale: u32, // 1 for 4096, 2 for 8192

    // Grid constants (don't change with size)
    pub blocks_per_row: u32,
    pub sprite_chars_per_block: u32,
    pub sprite_blocks: u32,
    pub sprite_rows: u32,
    pub tui_chars_per_block: u32,
    pub tui_blocks_start: u32,
    pub tui_blocks_count: u32,
    pub emoji_chars_per_block: u32,
    pub emoji_blocks_start: u32,
    pub emoji_blocks_count: u32,
    pub cjk_grid_cols: u32,
    pub cjk_grid_rows: u32,

    // Linear index bases (don't change with size)
    pub linear_sprite_base: u32,
    pub linear_tui_base: u32,
    pub linear_emoji_base: u32,
    pub linear_cjk_base: u32,

    // Pixel sizes (scaled)
    pub sprite_block_size: u32,
    pub sprite_char_size: u32,
    pub sprite_area_height: u32,

    pub tui_block_width: u32,
    pub tui_block_height: u32,
    pub tui_char_width: u32,
    pub tui_char_height: u32,
    pub tui_area_start_y: u32,

    pub emoji_block_width: u32,
    pub emoji_block_height: u32,
    pub emoji_char_size: u32,
    pub emoji_area_start_x: u32,
    pub emoji_area_start_y: u32,

    pub cjk_char_size: u32,
    pub cjk_area_start_y: u32,

    // Render parameters
    pub tui_render_width: u32,
    pub tui_render_height: u32,
    pub tui_font_size: u32,

    pub emoji_render_size: u32,
    pub emoji_font_size: u32,

    pub cjk_render_size: u32,
    pub cjk_font_size: u32,
}

impl TextureConfig {
    pub fn new(size: u32) -> Result<Self, String> {
        if size != 4096 && size != 8192 {
            return Err(format!(
                "Unsupported texture size: {}, only 4096 or 8192 supported",
                size
            ));
        }

        let scale = size / 4096;

        let cfg = Self {
            size,
            scale,

            // Grid constants
            blocks_per_row: 16,
            sprite_chars_per_block: 256,
            sprite_blocks: 160,
            sprite_rows: 10,
            tui_chars_per_block: 256,
            tui_blocks_start: 160,
            tui_blocks_count: 10,
            emoji_chars_per_block: 128,
            emoji_blocks_start: 170,
            emoji_blocks_count: 6,
            cjk_grid_cols: 128,
            cjk_grid_rows: 32,

            // Linear index bases
            linear_sprite_base: 0,
            linear_tui_base: 40960,
            linear_emoji_base: 43520,
            linear_cjk_base: 44288,

            // Pixel sizes (scaled)
            sprite_block_size: 256 * scale,
            sprite_char_size: 16 * scale,
            sprite_area_height: 2560 * scale,

            tui_block_width: 256 * scale,
            tui_block_height: 512 * scale,
            tui_char_width: 16 * scale,
            tui_char_height: 32 * scale,
            tui_area_start_y: 2560 * scale,

            emoji_block_width: 256 * scale,
            emoji_block_height: 512 * scale,
            emoji_char_size: 32 * scale,
            emoji_area_start_x: 2560 * scale,
            emoji_area_start_y: 2560 * scale,

            cjk_char_size: 32 * scale,
            cjk_area_start_y: 3072 * scale,

            // Render parameters
            tui_render_width: 40 * scale,
            tui_render_height: 80 * scale,
            tui_font_size: 64 * scale,

            emoji_render_size: 64 * scale,
            emoji_font_size: 64 * scale,

            cjk_render_size: 64 * scale,
            cjk_font_size: 56 * scale,
        };

        Ok(cfg)
    }
}
