// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Font rendering using macOS CoreText/Quartz for high-quality output (matching Python)
// Falls back to fontdue on other platforms

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

/// CJK font name for macOS CoreText
const CJK_FONT_NAME: &str = "PingFang SC";

/// Emoji font name for macOS CoreText
const EMOJI_FONT_NAME: &str = "Apple Color Emoji";


/// Expand $HOME in path
fn expand_home(path: &str) -> String {
    if path.starts_with("$HOME") {
        if let Some(home) = std::env::var_os("HOME") {
            return path.replace("$HOME", home.to_string_lossy().as_ref());
        }
    }
    path.to_string()
}

/// Find TUI font path
fn find_tui_font_path(custom_path: Option<&str>) -> Option<String> {
    // Try custom path first
    if let Some(path) = custom_path {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Try font paths
    for path in TUI_FONT_PATHS {
        let expanded = expand_home(path);
        if std::path::Path::new(&expanded).exists() {
            return Some(expanded);
        }
    }

    None
}

// ============================================================================
// macOS CoreText/Quartz implementation
// ============================================================================

#[cfg(target_os = "macos")]
mod quartz {
    use super::*;
    use core_foundation::array::CFArray;
    use core_foundation::attributed_string::CFMutableAttributedString;
    use core_foundation::base::{CFRange, TCFType};
    use core_foundation::string::CFString;
    use core_foundation::url::CFURL;
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use core_graphics::image::CGImageAlphaInfo;
    use core_text::font::CTFont;
    use core_text::font_descriptor::CTFontDescriptor;
    use core_text::line::CTLine;
    use std::os::raw::c_void;

    // CoreText key constants
    extern "C" {
        static kCTFontAttributeName: core_foundation::string::CFStringRef;
        static kCTForegroundColorFromContextAttributeName: core_foundation::string::CFStringRef;
    }

    // CoreText functions not in core-text crate
    #[link(name = "CoreText", kind = "framework")]
    extern "C" {
        fn CTFontManagerCreateFontDescriptorsFromURL(
            url: core_foundation::url::CFURLRef,
        ) -> core_foundation::array::CFArrayRef;

        fn CTLineGetBoundsWithOptions(
            line: core_text::line::CTLineRef,
            options: u64,
        ) -> core_graphics::geometry::CGRect;
    }

    // CTLineBoundsOptions
    const K_CT_LINE_BOUNDS_USE_GLYPH_PATH_BOUNDS: u64 = 1 << 1;

    /// Create CTFont from file path (matching Python's ctfont_from_file)
    pub fn ctfont_from_file(font_path: &str, size: f64) -> Option<CTFont> {
        let url = CFURL::from_path(std::path::Path::new(font_path), false)?;

        unsafe {
            let descriptors_ref = CTFontManagerCreateFontDescriptorsFromURL(url.as_concrete_TypeRef());
            if descriptors_ref.is_null() {
                return None;
            }

            let descriptors: CFArray<CTFontDescriptor> = TCFType::wrap_under_create_rule(descriptors_ref);

            if descriptors.len() == 0 {
                return None;
            }

            // Get first descriptor
            let desc = descriptors.get(0)?;

            Some(core_text::font::new_from_descriptor(&desc, size))
        }
    }

    /// Create CTFont from name (for CJK and Emoji)
    pub fn ctfont_from_name(name: &str, size: f64) -> Option<CTFont> {
        core_text::font::new_from_name(name, size).ok()
    }

    /// Binary search to find font size that fills target height
    /// Matching Python's solve_font_size_for_height()
    pub fn solve_font_size_for_height(font_path: &str, target_h: f32, padding: f32) -> f32 {
        let target = target_h * padding;
        let mut lo = 1.0f64;
        let mut hi = 512.0f64;

        for _ in 0..32 {
            let mid = (lo + hi) / 2.0;
            if let Some(font) = ctfont_from_file(font_path, mid) {
                let h = font.ascent() + font.descent() + font.leading();
                if h < target as f64 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            } else {
                break;
            }
        }

        ((lo + hi) / 2.0) as f32
    }

    /// Binary search to find font size for named font
    pub fn solve_font_size_for_height_by_name(font_name: &str, target_h: f32, padding: f32) -> f32 {
        let target = target_h * padding;
        let mut lo = 1.0f64;
        let mut hi = 512.0f64;

        for _ in 0..32 {
            let mid = (lo + hi) / 2.0;
            if let Some(font) = ctfont_from_name(font_name, mid) {
                let h = font.ascent() + font.descent() + font.leading();
                if h < target as f64 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            } else {
                break;
            }
        }

        ((lo + hi) / 2.0) as f32
    }

    /// Get ink bounds of CTLine (actual glyph bounds)
    fn ct_line_ink_bounds(line: &CTLine) -> CGRect {
        unsafe { CTLineGetBoundsWithOptions(line.as_concrete_TypeRef(), K_CT_LINE_BOUNDS_USE_GLYPH_PATH_BOUNDS) }
    }

    /// Apply width constraint (matching Python's apply_width_constraint)
    pub fn apply_width_constraint(font_path: &str, size: f32, cell_w: f32, margin: f32) -> f32 {
        if let Some(font) = ctfont_from_file(font_path, size as f64) {
            let mut worst = 0.0f64;

            // Test widest characters in monospace fonts
            let test_chars = "W@M#%&QG";
            for ch in test_chars.chars() {
                if let Some(line) = create_ct_line(&font, ch) {
                    let bounds = ct_line_ink_bounds(&line);
                    if bounds.size.width > worst {
                        worst = bounds.size.width;
                    }
                }
            }

            let limit = (cell_w * margin) as f64;
            if worst <= limit {
                return size;
            }

            return size * (limit / worst) as f32;
        }

        size
    }

    /// Create CTLine for a string (supports multi-codepoint emoji)
    fn create_ct_line_str(font: &CTFont, s: &str) -> Option<CTLine> {
        let cf_string = CFString::new(s);

        // Create attributed string with font attribute
        let mut attr_string = CFMutableAttributedString::new();
        attr_string.replace_str(&cf_string, CFRange::init(0, 0));

        let range = CFRange::init(0, cf_string.char_len());

        // Set font attribute
        unsafe {
            core_foundation::attributed_string::CFAttributedStringSetAttribute(
                attr_string.as_concrete_TypeRef(),
                range,
                kCTFontAttributeName,
                font.as_concrete_TypeRef() as *const c_void,
            );

            // Set foreground color from context
            let cf_true = core_foundation::boolean::CFBoolean::true_value();
            core_foundation::attributed_string::CFAttributedStringSetAttribute(
                attr_string.as_concrete_TypeRef(),
                range,
                kCTForegroundColorFromContextAttributeName,
                cf_true.as_concrete_TypeRef() as *const c_void,
            );
        }

        Some(CTLine::new_with_attributed_string(attr_string.as_concrete_TypeRef()))
    }

    /// Create CTLine for a single character
    fn create_ct_line(font: &CTFont, ch: char) -> Option<CTLine> {
        create_ct_line_str(font, &ch.to_string())
    }

    /// Render a string (for emoji) using Quartz
    pub fn render_str_quartz(
        s: &str,
        width: u32,
        height: u32,
        font_name: &str,
        font_size: f32,
    ) -> Option<RgbaImage> {
        let font = ctfont_from_name(font_name, font_size as f64)?;

        let ascent = font.ascent();
        let descent = font.descent();

        // Create bitmap context
        let color_space = CGColorSpace::create_device_rgb();
        let mut context = CGContext::create_bitmap_context(
            None,
            width as usize,
            height as usize,
            8,
            width as usize * 4,
            &color_space,
            CGImageAlphaInfo::CGImageAlphaPremultipliedLast as u32,
        );

        // Clear background (transparent)
        context.clear_rect(CGRect::new(
            &CGPoint::new(0.0, 0.0),
            &CGSize::new(width as f64, height as f64),
        ));

        // Set text color to white
        context.set_rgb_fill_color(1.0, 1.0, 1.0, 1.0);

        // Create CTLine
        let line = create_ct_line_str(&font, s)?;

        // Calculate position (centered)
        let typo_bounds = line.get_typographic_bounds();
        let x = ((width as f64) - typo_bounds.width) / 2.0;
        // Vertical centering: baseline_y from bottom (CGContext origin at bottom-left)
        let baseline_y = ((height as f64) - (ascent + descent)) / 2.0 + descent;

        // Pixel align
        let x = x.round();
        let baseline_y = baseline_y.round();

        // Draw text
        context.set_text_position(x, baseline_y);
        line.draw(&context);

        // Extract image data
        let data = context.data();
        let ptr = data.as_ptr() as *const u8;
        let len = (width * height * 4) as usize;
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };

        // Create RgbaImage from RGBA data
        // CGContext bitmap data is stored with origin at top-left (when created with
        // standard options), so no Y-flip needed
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                let r = slice[offset];
                let g = slice[offset + 1];
                let b = slice[offset + 2];
                let a = slice[offset + 3];

                // Unpremultiply alpha
                let (r, g, b) = if a > 0 {
                    let af = a as f32 / 255.0;
                    (
                        ((r as f32 / af).min(255.0)) as u8,
                        ((g as f32 / af).min(255.0)) as u8,
                        ((b as f32 / af).min(255.0)) as u8,
                    )
                } else {
                    (0, 0, 0)
                };

                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }

        Some(img)
    }

    #[allow(dead_code)]
    fn _create_ct_line_old(font: &CTFont, ch: char) -> Option<CTLine> {
        let s = ch.to_string();
        let cf_string = CFString::new(&s);

        // Create attributed string with font attribute
        let mut attr_string = CFMutableAttributedString::new();
        attr_string.replace_str(&cf_string, CFRange::init(0, 0));

        let range = CFRange::init(0, cf_string.char_len());

        // Set font attribute
        unsafe {
            core_foundation::attributed_string::CFAttributedStringSetAttribute(
                attr_string.as_concrete_TypeRef(),
                range,
                kCTFontAttributeName,
                font.as_concrete_TypeRef() as *const c_void,
            );

            // Set foreground color from context
            let cf_true = core_foundation::boolean::CFBoolean::true_value();
            core_foundation::attributed_string::CFAttributedStringSetAttribute(
                attr_string.as_concrete_TypeRef(),
                range,
                kCTForegroundColorFromContextAttributeName,
                cf_true.as_concrete_TypeRef() as *const c_void,
            );
        }

        Some(CTLine::new_with_attributed_string(attr_string.as_concrete_TypeRef()))
    }

    /// Render a single character using Quartz (matching Python's render_char_quartz)
    pub fn render_char_quartz(
        ch: char,
        width: u32,
        height: u32,
        font_path: Option<&str>,
        font_name: Option<&str>,
        _font_size: f32,
        fill_cell: bool,
        text_padding: f32,
    ) -> Option<RgbaImage> {
        let padding = if fill_cell { 1.0 } else { text_padding };

        // Determine actual font size and create font
        let (_actual_font_size, font) = if let Some(path) = font_path {
            if std::path::Path::new(path).exists() {
                let size = if fill_cell {
                    // Graphic chars: only use height, no width constraint
                    solve_font_size_for_height(path, height as f32, padding)
                } else {
                    // Text chars: binary search + width constraint
                    let size_h = solve_font_size_for_height(path, height as f32, padding);
                    apply_width_constraint(path, size_h, width as f32, 0.98)
                };
                let f = ctfont_from_file(path, size as f64)?;
                (size, f)
            } else if let Some(name) = font_name {
                // Font path doesn't exist, fall back to named font
                let size = solve_font_size_for_height_by_name(name, height as f32, padding);
                let f = ctfont_from_name(name, size as f64)?;
                (size, f)
            } else {
                return None;
            }
        } else if let Some(name) = font_name {
            // Use font by name - directly use font_size * padding (matching Python)
            let size = _font_size * padding;
            let f = ctfont_from_name(name, size as f64)?;
            (size, f)
        } else {
            return None;
        };

        let ascent = font.ascent();
        let descent = font.descent();

        // Create bitmap context
        let color_space = CGColorSpace::create_device_rgb();
        let mut context = CGContext::create_bitmap_context(
            None,
            width as usize,
            height as usize,
            8,
            width as usize * 4,
            &color_space,
            CGImageAlphaInfo::CGImageAlphaPremultipliedLast as u32,
        );

        // Clear background (transparent)
        context.clear_rect(CGRect::new(
            &CGPoint::new(0.0, 0.0),
            &CGSize::new(width as f64, height as f64),
        ));

        // Set text color to white
        context.set_rgb_fill_color(1.0, 1.0, 1.0, 1.0);

        // Create CTLine
        let line = create_ct_line(&font, ch)?;

        // Calculate x position
        let x = if fill_cell {
            // Graphic chars: use typographic width for centering
            let typo_bounds = line.get_typographic_bounds();
            ((width as f64) - typo_bounds.width) / 2.0
        } else if font_path.is_some() && font_path.map(|p| std::path::Path::new(p).exists()).unwrap_or(false) {
            // Text chars with font_path: use ink bounds for visual centering
            let ink = ct_line_ink_bounds(&line);
            ((width as f64) - ink.size.width) / 2.0 - ink.origin.x
        } else {
            // Fallback: use typographic width
            let typo_bounds = line.get_typographic_bounds();
            ((width as f64) - typo_bounds.width) / 2.0
        };

        // Calculate y position (vertical centering) - baseline from bottom
        let baseline_y = ((height as f64) - (ascent + descent)) / 2.0 + descent;

        // Pixel align
        let x = x.round();
        let baseline_y = baseline_y.round();

        // Draw text
        context.set_text_position(x, baseline_y);
        line.draw(&context);

        // Extract image data
        let data = context.data();
        let ptr = data.as_ptr() as *const u8;
        let len = (width * height * 4) as usize;
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };

        // Create RgbaImage from RGBA data - no Y flip
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                let r = slice[offset];
                let g = slice[offset + 1];
                let b = slice[offset + 2];
                let a = slice[offset + 3];

                // Unpremultiply alpha
                let (r, g, b) = if a > 0 {
                    let af = a as f32 / 255.0;
                    (
                        ((r as f32 / af).min(255.0)) as u8,
                        ((g as f32 / af).min(255.0)) as u8,
                        ((b as f32 / af).min(255.0)) as u8,
                    )
                } else {
                    (0, 0, 0)
                };

                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }

        Some(img)
    }
}

// ============================================================================
// Fontdue fallback for non-macOS platforms
// ============================================================================

#[cfg(not(target_os = "macos"))]
mod fontdue_impl {
    use super::*;

    /// CJK font paths with optional collection index for TTC files
    const CJK_FONT_CONFIGS: &[(&str, u32)] = &[
        // Linux
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
        ("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", 0),
        // Windows
        ("C:\\Windows\\Fonts\\msyh.ttc", 0),
        ("C:\\Windows\\Fonts\\simsun.ttc", 0),
    ];

    /// Find and load a TUI font
    fn find_tui_font(custom_path: Option<&str>) -> Option<Vec<u8>> {
        if let Some(path) = custom_path {
            if let Ok(data) = std::fs::read(path) {
                println!("  TUI font: {}", path);
                return Some(data);
            }
        }

        for path in super::TUI_FONT_PATHS {
            let expanded = super::expand_home(path);
            if let Ok(data) = std::fs::read(&expanded) {
                println!("  TUI font: {}", expanded);
                return Some(data);
            }
        }

        None
    }

    /// Find and load a CJK font
    fn find_cjk_font() -> Option<fontdue::Font> {
        for (path, collection_index) in CJK_FONT_CONFIGS {
            let expanded = super::expand_home(path);
            if let Ok(data) = std::fs::read(&expanded) {
                let settings = fontdue::FontSettings {
                    collection_index: *collection_index,
                    ..fontdue::FontSettings::default()
                };
                if let Ok(font) = fontdue::Font::from_bytes(data.as_slice(), settings) {
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

    /// Binary search to find font size that fills target height
    fn solve_font_size_for_height(font: &fontdue::Font, target_h: f32, padding: f32) -> f32 {
        let target = target_h * padding;
        let mut lo = 1.0f32;
        let mut hi = target * 3.0;

        for _ in 0..30 {
            let mid = (lo + hi) / 2.0;
            let metrics = font.horizontal_line_metrics(mid);
            let total_height = if let Some(m) = metrics {
                m.ascent - m.descent + m.line_gap
            } else {
                mid * 1.2
            };

            if total_height < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        (lo + hi) / 2.0
    }

    /// Render a single character using fontdue
    fn render_char(
        font: &fontdue::Font,
        ch: char,
        width: u32,
        height: u32,
        base_font_size: f32,
        fill_cell: bool,
    ) -> RgbaImage {
        let font_size = if fill_cell {
            solve_font_size_for_height(font, height as f32, 1.0)
        } else {
            base_font_size
        };

        let (metrics, bitmap) = font.rasterize(ch, font_size);
        let mut img = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));

        if bitmap.is_empty() || metrics.width == 0 || metrics.height == 0 {
            return img;
        }

        let glyph_width = metrics.width as i32;
        let glyph_height = metrics.height as i32;
        let offset_x = ((width as i32 - glyph_width) / 2).max(0);

        let offset_y = if fill_cell {
            ((height as i32 - glyph_height) / 2).max(0)
        } else {
            let ascent = font_size * 0.8;
            let baseline_y = (height as f32 / 2.0 + ascent / 2.0) as i32;
            (baseline_y - metrics.height as i32 - metrics.ymin).max(0)
        };

        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let px = offset_x + gx as i32;
                let py = offset_y + gy as i32;

                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let alpha = bitmap[gy * metrics.width + gx];
                    if alpha > 0 {
                        img.put_pixel(px as u32, py as u32, Rgba([255, 255, 255, alpha]));
                    }
                }
            }
        }

        img
    }

    /// Render TUI characters (fontdue fallback)
    pub fn render_tui_chars(
        tui_chars: &[char],
        cfg: &super::TextureConfig,
        pxrange: u32,
        text_padding: f32,
        font_path: Option<&str>,
    ) -> Vec<RgbaImage> {
        let total = (cfg.tui_blocks_count * cfg.tui_chars_per_block) as usize;
        let mut images = Vec::with_capacity(total);

        let font_data = match find_tui_font(font_path) {
            Some(data) => data,
            None => {
                eprintln!("Error: No monospace font found!");
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

        // SDF workflow: render at 2x size, compute SDF, then resize to target
        let sdf_scale = 2u32;
        let render_w = cfg.tui_char_width * sdf_scale;
        let render_h = cfg.tui_char_height * sdf_scale;
        let spread = pxrange as f32 * sdf_scale as f32;

        for i in 0..total {
            let image = if i < tui_chars.len() {
                let ch = tui_chars[i];
                let fill_cell = super::is_graphic_char(ch);
                let padding = if fill_cell { 1.0 } else { text_padding };
                let font_size = cfg.tui_font_size as f32 * padding;

                let rendered = render_char(&font, ch, render_w, render_h, font_size, fill_cell);

                // Compute SDF at render size, then resize
                let sdf = super::bitmap_to_sdf(&rendered, spread);
                image::imageops::resize(
                    &sdf,
                    cfg.tui_char_width,
                    cfg.tui_char_height,
                    image::imageops::FilterType::Lanczos3,
                )
            } else {
                ImageBuffer::from_pixel(cfg.tui_char_width, cfg.tui_char_height, Rgba([0, 0, 0, 255]))
            };

            images.push(image);

            if (i + 1) % 256 == 0 {
                println!("    Rendering TUI: {}/{}", i + 1, total);
            }
        }

        images
    }

    /// Render emojis (placeholder for non-macOS)
    pub fn render_emojis(emojis: &[String], cfg: &super::TextureConfig) -> Vec<RgbaImage> {
        let total = (cfg.emoji_blocks_count * cfg.emoji_chars_per_block) as usize;
        let mut images = Vec::with_capacity(total);

        for i in 0..total {
            images.push(ImageBuffer::from_pixel(
                cfg.emoji_char_size,
                cfg.emoji_char_size,
                Rgba([0, 0, 0, 0]),
            ));

            if (i + 1) % 128 == 0 {
                println!("    Rendering Emoji: {}/{}", i + 1, total);
            }
        }

        let _ = emojis; // Suppress unused warning
        images
    }

    /// Render CJK characters (fontdue fallback)
    pub fn render_cjk_chars(
        cjk_chars: &[char],
        cfg: &super::TextureConfig,
        pxrange: u32,
        text_padding: f32,
    ) -> Vec<RgbaImage> {
        let total = (cfg.cjk_grid_cols * cfg.cjk_grid_rows) as usize;
        let mut images = Vec::with_capacity(total);

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

        // SDF workflow: render at 2x size, compute SDF, then resize to target
        let sdf_scale = 2u32;
        let render_size = cfg.cjk_char_size * sdf_scale;
        let spread = pxrange as f32 * sdf_scale as f32;
        let font_size = cfg.cjk_font_size as f32 * text_padding;

        for i in 0..total {
            let image = if i < cjk_chars.len() {
                let ch = cjk_chars[i];

                let rendered = render_char(&font, ch, render_size, render_size, font_size, false);

                // Compute SDF at render size, then resize
                let sdf = super::bitmap_to_sdf(&rendered, spread);
                image::imageops::resize(
                    &sdf,
                    cfg.cjk_char_size,
                    cfg.cjk_char_size,
                    image::imageops::FilterType::Lanczos3,
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
}

// ============================================================================
// Public API - dispatches to appropriate implementation
// ============================================================================

/// Render TUI characters to SDF images
pub fn render_tui_chars(
    tui_chars: &[char],
    cfg: &TextureConfig,
    pxrange: u32,
    text_padding: f32,
    font_path: Option<&str>,
) -> Vec<RgbaImage> {
    #[cfg(target_os = "macos")]
    {
        render_tui_chars_macos(tui_chars, cfg, pxrange, text_padding, font_path)
    }

    #[cfg(not(target_os = "macos"))]
    {
        fontdue_impl::render_tui_chars(tui_chars, cfg, pxrange, text_padding, font_path)
    }
}

/// Render emojis (bitmap mode - color emojis don't work well with SDF)
pub fn render_emojis(emojis: &[String], cfg: &TextureConfig) -> Vec<RgbaImage> {
    #[cfg(target_os = "macos")]
    {
        render_emojis_macos(emojis, cfg)
    }

    #[cfg(not(target_os = "macos"))]
    {
        fontdue_impl::render_emojis(emojis, cfg)
    }
}

/// Render CJK characters to SDF images
pub fn render_cjk_chars(
    cjk_chars: &[char],
    cfg: &TextureConfig,
    pxrange: u32,
    text_padding: f32,
) -> Vec<RgbaImage> {
    #[cfg(target_os = "macos")]
    {
        render_cjk_chars_macos(cjk_chars, cfg, pxrange, text_padding)
    }

    #[cfg(not(target_os = "macos"))]
    {
        fontdue_impl::render_cjk_chars(cjk_chars, cfg, pxrange, text_padding)
    }
}

// ============================================================================
// macOS implementation functions
// ============================================================================

#[cfg(target_os = "macos")]
fn render_tui_chars_macos(
    tui_chars: &[char],
    cfg: &TextureConfig,
    pxrange: u32,
    text_padding: f32,
    font_path: Option<&str>,
) -> Vec<RgbaImage> {
    let total = (cfg.tui_blocks_count * cfg.tui_chars_per_block) as usize;
    let mut images = Vec::with_capacity(total);

    // Find font path
    let resolved_font_path = find_tui_font_path(font_path);
    if let Some(ref path) = resolved_font_path {
        println!("  TUI font: {}", path);
    } else {
        eprintln!("Error: No TUI font found!");
        for _ in 0..total {
            images.push(ImageBuffer::from_pixel(
                cfg.tui_char_width,
                cfg.tui_char_height,
                Rgba([0, 0, 0, 255]),
            ));
        }
        return images;
    }

    // SDF workflow: render at 2x of TUI_RENDER_SIZE, compute SDF, then resize to target
    // TUI: 160x320 render -> SDF -> 32x64 target (for 8192 texture)
    let sdf_scale = 2u32;
    let render_w = cfg.tui_render_width * sdf_scale;
    let render_h = cfg.tui_render_height * sdf_scale;
    let spread = pxrange as f32 * sdf_scale as f32;

    for i in 0..total {
        let image = if i < tui_chars.len() {
            let ch = tui_chars[i];
            let fill_cell = is_graphic_char(ch);

            // Render bitmap at 2x of TUI_RENDER_SIZE
            let rendered = quartz::render_char_quartz(
                ch,
                render_w,
                render_h,
                resolved_font_path.as_deref(),
                None,
                cfg.tui_font_size as f32,
                fill_cell,
                text_padding,
            );

            if let Some(bitmap) = rendered {
                // Compute SDF at render size, then resize
                let sdf = bitmap_to_sdf(&bitmap, spread);
                imageops::resize(
                    &sdf,
                    cfg.tui_char_width,
                    cfg.tui_char_height,
                    imageops::FilterType::Lanczos3,
                )
            } else {
                ImageBuffer::from_pixel(cfg.tui_char_width, cfg.tui_char_height, Rgba([0, 0, 0, 255]))
            }
        } else {
            ImageBuffer::from_pixel(cfg.tui_char_width, cfg.tui_char_height, Rgba([0, 0, 0, 255]))
        };

        images.push(image);

        if (i + 1) % 256 == 0 {
            println!("    Rendering TUI: {}/{}", i + 1, total);
        }
    }

    images
}

#[cfg(target_os = "macos")]
fn render_emojis_macos(emojis: &[String], cfg: &TextureConfig) -> Vec<RgbaImage> {
    let total = (cfg.emoji_blocks_count * cfg.emoji_chars_per_block) as usize;
    let mut images = Vec::with_capacity(total);

    for i in 0..total {
        let image = if i < emojis.len() {
            let emoji = &emojis[i];
            // Use full emoji string (supports multi-codepoint emoji)
            let rendered = quartz::render_str_quartz(
                emoji,
                cfg.emoji_render_size,
                cfg.emoji_render_size,
                EMOJI_FONT_NAME,
                cfg.emoji_font_size as f32,
            );

            if let Some(bitmap) = rendered {
                // Emojis don't use SDF, just resize
                if bitmap.width() != cfg.emoji_char_size || bitmap.height() != cfg.emoji_char_size {
                    imageops::resize(
                        &bitmap,
                        cfg.emoji_char_size,
                        cfg.emoji_char_size,
                        imageops::FilterType::Lanczos3,
                    )
                } else {
                    bitmap
                }
            } else {
                ImageBuffer::from_pixel(cfg.emoji_char_size, cfg.emoji_char_size, Rgba([0, 0, 0, 0]))
            }
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

#[cfg(target_os = "macos")]
fn render_cjk_chars_macos(
    cjk_chars: &[char],
    cfg: &TextureConfig,
    pxrange: u32,
    text_padding: f32,
) -> Vec<RgbaImage> {
    let total = (cfg.cjk_grid_cols * cfg.cjk_grid_rows) as usize;
    let mut images = Vec::with_capacity(total);

    println!("  CJK font: {}", CJK_FONT_NAME);

    // SDF workflow: render at 2x of CJK_RENDER_SIZE, compute SDF, then resize to target
    // CJK: 256x256 render -> SDF -> 64x64 target (for 8192 texture)
    let sdf_scale = 2u32;
    let render_size = cfg.cjk_render_size * sdf_scale;
    let font_size = cfg.cjk_font_size as f32 * sdf_scale as f32;
    let spread = pxrange as f32 * sdf_scale as f32;

    for i in 0..total {
        let image = if i < cjk_chars.len() {
            let ch = cjk_chars[i];

            let rendered = quartz::render_char_quartz(
                ch,
                render_size,
                render_size,
                None,
                Some(CJK_FONT_NAME),
                font_size,
                false,
                text_padding,
            );

            if let Some(bitmap) = rendered {
                // Compute SDF at render size, then resize
                let sdf = bitmap_to_sdf(&bitmap, spread);
                imageops::resize(
                    &sdf,
                    cfg.cjk_char_size,
                    cfg.cjk_char_size,
                    imageops::FilterType::Lanczos3,
                )
            } else {
                ImageBuffer::from_pixel(cfg.cjk_char_size, cfg.cjk_char_size, Rgba([0, 0, 0, 255]))
            }
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
