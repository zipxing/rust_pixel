// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Font rendering using fontdue for cross-platform support

use super::config::TextureConfig;
use super::edt::{bitmap_to_sdf, is_graphic_char};
use image::{imageops, ImageBuffer, Rgba, RgbaImage};

/// TUI font paths - prefer NerdFont for complete symbol coverage
const TUI_FONT_PATHS: &[&str] = &[
    // User-specified in symbols directory
    "tools/symbols/TUIFont.ttf",
    "tools/symbols/TUIFont.otf",
    // User fonts (NerdFont recommended for TUI symbols including Braille)
    "$HOME/Library/Fonts/DroidSansMNerdFontMono-Regular.otf",
    "$HOME/Library/Fonts/DejaVuSansMNerdFont-Regular.ttf",
    "$HOME/.local/share/fonts/DroidSansMNerdFontMono-Regular.otf",
    // System fallbacks
    "/System/Library/Fonts/Monaco.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "C:\\Windows\\Fonts\\consola.ttf",
];

/// CJK font paths with optional collection index for TTC files
/// Format: (path, collection_index)
const CJK_FONT_CONFIGS: &[(&str, u32)] = &[
    // macOS - PingFang SC (苹方) - the real one with CJK glyphs
    ("/System/Library/AssetsV2/com_apple_MobileAsset_Font8/86ba2c91f017a3749571a82f2c6d890ac7ffb2fb.asset/AssetData/PingFang.ttc", 0),
    // Fallbacks
    ("/System/Library/Fonts/STHeiti Light.ttc", 0),
    ("/System/Library/Fonts/Supplemental/Songti.ttc", 0),
    // Linux
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", 0),
    // Windows
    ("C:\\Windows\\Fonts\\msyh.ttc", 0),
    ("C:\\Windows\\Fonts\\simsun.ttc", 0),
];

/// Expand $HOME in path
fn expand_home(path: &str) -> String {
    if path.starts_with("$HOME") {
        if let Some(home) = std::env::var_os("HOME") {
            return path.replace("$HOME", home.to_string_lossy().as_ref());
        }
    }
    path.to_string()
}

/// Find and load a TUI font (NerdFont preferred for symbols/Braille)
fn find_tui_font(custom_path: Option<&str>) -> Option<Vec<u8>> {
    // Try custom path first
    if let Some(path) = custom_path {
        if let Ok(data) = std::fs::read(path) {
            println!("  TUI font: {}", path);
            return Some(data);
        }
    }

    // Try font paths
    for path in TUI_FONT_PATHS {
        let expanded = expand_home(path);
        if let Ok(data) = std::fs::read(&expanded) {
            println!("  TUI font: {}", expanded);
            return Some(data);
        }
    }

    None
}

/// Find and load a CJK font (PingFang SC preferred)
fn find_cjk_font() -> Option<fontdue::Font> {
    for (path, collection_index) in CJK_FONT_CONFIGS {
        let expanded = expand_home(path);
        if let Ok(data) = std::fs::read(&expanded) {
            let settings = fontdue::FontSettings {
                collection_index: *collection_index,
                ..fontdue::FontSettings::default()
            };
            if let Ok(font) = fontdue::Font::from_bytes(data.as_slice(), settings) {
                // Verify it can render CJK by testing a common character
                let (metrics, _) = font.rasterize('中', 32.0);
                if metrics.width > 0 {
                    println!("  CJK font: {} (index {})", expanded, collection_index);
                    return Some(font);
                }
            }
        }
    }
    None
}

/// Render TUI characters to SDF images
pub fn render_tui_chars(
    tui_chars: &[char],
    cfg: &TextureConfig,
    pxrange: u32,
    text_padding: f32,
    font_path: Option<&str>,
) -> Vec<RgbaImage> {
    let total = (cfg.tui_blocks_count * cfg.tui_chars_per_block) as usize;
    let mut images = Vec::with_capacity(total);

    // Load font
    let font_data = match find_tui_font(font_path) {
        Some(data) => data,
        None => {
            eprintln!("Error: No monospace font found!");
            eprintln!("Please place a TTF/OTF font at tools/symbols/TUIFont.ttf");
            eprintln!("Or specify with --font <path>");
            for _ in 0..total {
                images.push(ImageBuffer::from_pixel(
                    cfg.tui_char_width,
                    cfg.tui_char_height,
                    Rgba([0, 0, 0, 255]),
                ));
            }
            return images;
        }
    };

    let font = match fontdue::Font::from_bytes(font_data.as_slice(), fontdue::FontSettings::default()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error parsing font: {}", e);
            // Return empty black images
            for _ in 0..total {
                images.push(ImageBuffer::from_pixel(
                    cfg.tui_char_width,
                    cfg.tui_char_height,
                    Rgba([0, 0, 0, 255]),
                ));
            }
            return images;
        }
    };

    // SDF supersampling scale
    let sdf_scale = 2u32;

    for i in 0..total {
        let image = if i < tui_chars.len() {
            let ch = tui_chars[i];
            let fill_cell = is_graphic_char(ch);

            // Render at higher resolution for SDF
            let render_w = cfg.tui_render_width * sdf_scale;
            let render_h = cfg.tui_render_height * sdf_scale;

            // Calculate font size
            let padding = if fill_cell { 1.0 } else { text_padding };
            let font_size = (cfg.tui_font_size as f32 * sdf_scale as f32 * padding) as f32;

            let rendered = render_char(&font, ch, render_w, render_h, font_size, fill_cell);

            // Convert to SDF
            let spread = pxrange as f32 * sdf_scale as f32;
            let sdf = bitmap_to_sdf(&rendered, spread);

            // Resize to target size
            imageops::resize(
                &sdf,
                cfg.tui_char_width,
                cfg.tui_char_height,
                imageops::FilterType::Lanczos3,
            )
        } else {
            // Empty slot - black with full alpha (for SDF mode)
            ImageBuffer::from_pixel(cfg.tui_char_width, cfg.tui_char_height, Rgba([0, 0, 0, 255]))
        };

        images.push(image);

        if (i + 1) % 256 == 0 {
            println!("    Rendering TUI: {}/{}", i + 1, total);
        }
    }

    images
}

/// Render emojis (bitmap mode - color emojis don't work well with SDF)
pub fn render_emojis(emojis: &[String], cfg: &TextureConfig) -> Vec<RgbaImage> {
    let total = (cfg.emoji_blocks_count * cfg.emoji_chars_per_block) as usize;
    let mut images = Vec::with_capacity(total);

    // For emojis, we'd need system emoji font which is platform-specific
    // For now, create placeholder images
    // TODO: Add platform-specific emoji rendering

    for i in 0..total {
        let image = if i < emojis.len() {
            // Placeholder - transparent for now
            // Real implementation would use platform emoji APIs
            ImageBuffer::from_pixel(cfg.emoji_char_size, cfg.emoji_char_size, Rgba([0, 0, 0, 0]))
        } else {
            ImageBuffer::from_pixel(cfg.emoji_char_size, cfg.emoji_char_size, Rgba([0, 0, 0, 0]))
        };

        images.push(image);

        if (i + 1) % 128 == 0 {
            println!("    Rendering Emoji: {}/{}", i + 1, total);
        }
    }

    images
}

/// Render CJK characters to SDF images
pub fn render_cjk_chars(
    cjk_chars: &[char],
    cfg: &TextureConfig,
    pxrange: u32,
    text_padding: f32,
) -> Vec<RgbaImage> {
    let total = (cfg.cjk_grid_cols * cfg.cjk_grid_rows) as usize;
    let mut images = Vec::with_capacity(total);

    // Load CJK font (PingFang SC preferred)
    let font = match find_cjk_font() {
        Some(f) => f,
        None => {
            eprintln!("Warning: No CJK font found, CJK characters will be empty");
            for _ in 0..total {
                images.push(ImageBuffer::from_pixel(
                    cfg.cjk_char_size,
                    cfg.cjk_char_size,
                    Rgba([0, 0, 0, 255]),
                ));
            }
            return images;
        }
    };

    let sdf_scale = 2u32;

    for i in 0..total {
        let image = if i < cjk_chars.len() {
            let ch = cjk_chars[i];

            let render_size = cfg.cjk_render_size * sdf_scale;
            let font_size = cfg.cjk_font_size as f32 * sdf_scale as f32 * text_padding;

            let rendered = render_char(&font, ch, render_size, render_size, font_size, false);

            let spread = pxrange as f32 * sdf_scale as f32;
            let sdf = bitmap_to_sdf(&rendered, spread);

            imageops::resize(
                &sdf,
                cfg.cjk_char_size,
                cfg.cjk_char_size,
                imageops::FilterType::Lanczos3,
            )
        } else {
            ImageBuffer::from_pixel(cfg.cjk_char_size, cfg.cjk_char_size, Rgba([0, 0, 0, 255]))
        };

        images.push(image);

        if (i + 1) % 512 == 0 {
            println!("    Rendering CJK: {}/{}", i + 1, total);
        }
    }

    images
}

/// Render a single character using fontdue
fn render_char(
    font: &fontdue::Font,
    ch: char,
    width: u32,
    height: u32,
    font_size: f32,
    _fill_cell: bool,
) -> RgbaImage {
    // Rasterize the character
    let (metrics, bitmap) = font.rasterize(ch, font_size);

    // Create output image (transparent background)
    let mut img = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));

    if bitmap.is_empty() || metrics.width == 0 || metrics.height == 0 {
        return img;
    }

    // Calculate position to center the glyph
    let glyph_width = metrics.width as i32;
    let _glyph_height = metrics.height as i32;

    // Center horizontally
    let offset_x = ((width as i32 - glyph_width) / 2).max(0);

    // Center vertically (using baseline)
    let ascent = font_size * 0.8; // Approximate ascent
    let baseline_y = (height as f32 / 2.0 + ascent / 2.0) as i32;
    let offset_y = (baseline_y - metrics.height as i32 - metrics.ymin).max(0);

    // Copy glyph bitmap to image
    for gy in 0..metrics.height {
        for gx in 0..metrics.width {
            let px = offset_x + gx as i32;
            let py = offset_y + gy as i32;

            if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                let alpha = bitmap[gy * metrics.width + gx];
                if alpha > 0 {
                    // White text with alpha
                    img.put_pixel(px as u32, py as u32, Rgba([255, 255, 255, alpha]));
                }
            }
        }
    }

    img
}
