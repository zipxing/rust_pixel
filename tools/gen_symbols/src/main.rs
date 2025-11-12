// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! symbols.png texture generator for TUI architecture
//! 
//! Generates a 1024x1024 RGBA texture with three regions:
//! - TUI region (rows 0-127): 1024 characters (8x16 pixels each)
//! - Emoji region (rows 128-191): 256 emojis (16x16 pixels each)
//! - Sprite region (rows 192-1023): 13,312 characters (8x8 pixels each)

use fontdue::{Font, FontSettings};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::fs;
use std::path::Path;

const TEXTURE_SIZE: u32 = 1024;

// Region definitions
const TUI_START_Y: u32 = 0;
const TUI_HEIGHT: u32 = 128; // 8 rows × 16px per char
const TUI_CHAR_WIDTH: u32 = 8;
const TUI_CHAR_HEIGHT: u32 = 16;
const TUI_COLS: u32 = 128; // 1024 / 8

const EMOJI_START_Y: u32 = 128;
const EMOJI_HEIGHT: u32 = 64; // 4 rows × 16px per emoji
const EMOJI_SIZE: u32 = 16;
const EMOJI_COLS: u32 = 64; // 1024 / 16

const SPRITE_START_Y: u32 = 192;
const SPRITE_HEIGHT: u32 = 832; // 104 rows × 8px per char
const SPRITE_CHAR_SIZE: u32 = 8;
const SPRITE_COLS: u32 = 128; // 1024 / 8

fn main() {
    println!("🎨 Generating symbols.png texture (1024x1024)...");
    
    // Create blank RGBA image
    let mut img: RgbaImage = ImageBuffer::new(TEXTURE_SIZE, TEXTURE_SIZE);
    
    // Fill with transparent black
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }
    
    // Check for font file
    let font_path = find_font();
    
    match font_path {
        Some(path) => {
            println!("✅ Found font: {}", path);
            
            // Load font
            match fs::read(&path) {
                Ok(font_data) => {
                    match Font::from_bytes(font_data, FontSettings::default()) {
                        Ok(font) => {
                            println!("✅ Font loaded successfully");
                            
                            // Generate TUI characters
                            generate_tui_chars(&mut img, &font);
                            
                            // Generate Emoji placeholders
                            generate_emoji_placeholders(&mut img);
                            
                            // Generate Sprite placeholders
                            generate_sprite_placeholders(&mut img);
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to parse font: {}", e);
                            generate_fallback_texture(&mut img);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to read font file: {}", e);
                    generate_fallback_texture(&mut img);
                }
            }
        }
        None => {
            println!("⚠️  No suitable font found, generating placeholder texture");
            generate_fallback_texture(&mut img);
        }
    }
    
    // Save the image to current directory
    let output_path = "symbols_new.png";
    
    match img.save(output_path) {
        Ok(_) => {
            println!("✅ Saved texture to: {}", output_path);
            println!("\n💡 To use this texture, copy it to assets/pix/:");
            println!("   cp {} ../../assets/pix/symbols.png", output_path);
        }
        Err(e) => eprintln!("❌ Failed to save texture: {}", e),
    }
    
    println!("\n📊 Texture Layout:");
    println!("  TUI Region:    rows 0-127   (1024 chars, 8x16px)");
    println!("  Emoji Region:  rows 128-191 (256 emojis, 16x16px)");
    println!("  Sprite Region: rows 192-1023 (13,312 chars, 8x8px)");
}

fn find_font() -> Option<String> {
    // Priority order for fonts
    let font_candidates = vec![
        // User specified font
        "DroidSansMono Nerd Font.ttf",
        "assets/DroidSansMono Nerd Font.ttf",
        // Project fonts
        "assets/cp437ibm8x8.ttf",
        "assets/pixel8.ttf",
        // System fonts (macOS)
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Courier.ttc",
        // System fonts (Linux)
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        // System fonts (Windows)
        "C:\\Windows\\Fonts\\consola.ttf",
        "C:\\Windows\\Fonts\\cour.ttf",
    ];
    
    for font_path in font_candidates {
        if Path::new(font_path).exists() {
            return Some(font_path.to_string());
        }
    }
    
    None
}

fn generate_tui_chars(img: &mut RgbaImage, font: &Font) {
    println!("📝 Generating TUI characters (0-1023)...");
    
    // Generate ASCII printable characters (32-126) first
    for i in 0..1024 {
        let char_code = if i < 95 {
            // ASCII printable: space (32) to tilde (126)
            32 + i
        } else if i < 128 {
            // Extended ASCII: 128-159
            128 + (i - 95)
        } else {
            // Fill remaining with space for now
            32
        };
        
        let ch = char::from_u32(char_code as u32).unwrap_or(' ');
        
        // Calculate position in texture
        let col = i % TUI_COLS;
        let row = i / TUI_COLS;
        let x = col * TUI_CHAR_WIDTH;
        let y = TUI_START_Y + row * TUI_CHAR_HEIGHT;
        
        // Render character
        render_char(img, font, ch, x, y, TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT);
    }
    
    println!("  ✅ Generated 1024 TUI characters");
}

fn render_char(
    img: &mut RgbaImage,
    font: &Font,
    ch: char,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) {
    let (metrics, bitmap) = font.rasterize(ch, height as f32);
    
    // Calculate centering offsets
    let offset_x = ((width as i32 - metrics.width as i32) / 2).max(0) as u32;
    let offset_y = ((height as i32 - metrics.height as i32) / 2).max(0) as u32;
    
    // Draw character
    for (i, &alpha) in bitmap.iter().enumerate() {
        let px = (i % metrics.width) as u32;
        let py = (i / metrics.width) as u32;
        
        let img_x = x + offset_x + px;
        let img_y = y + offset_y + py;
        
        if img_x < TEXTURE_SIZE && img_y < TEXTURE_SIZE {
            // White color with alpha from glyph
            img.put_pixel(img_x, img_y, Rgba([255, 255, 255, alpha]));
        }
    }
}

fn generate_emoji_placeholders(img: &mut RgbaImage) {
    println!("😀 Generating Emoji symbols (1024-1279)...");
    
    // Common emoji characters that we'll try to render
    // Using Unicode emoji codepoints
    let common_emojis = get_common_emoji_list();
    
    for i in 0..256usize {
        let col = (i as u32) % EMOJI_COLS;
        let row = (i as u32) / EMOJI_COLS;
        let x = col * EMOJI_SIZE;
        let y = EMOJI_START_Y + row * EMOJI_SIZE;
        
        if i < common_emojis.len() {
            // Draw a placeholder box
            draw_placeholder_box(img, x, y, EMOJI_SIZE, EMOJI_SIZE, Rgba([64, 64, 64, 128]));
            
            // Draw emoji character or symbol in the center
            let emoji_char = common_emojis[i];
            draw_emoji_placeholder(img, x, y, emoji_char, i);
        } else {
            // Empty slot - draw subtle outline
            draw_placeholder_box(img, x, y, EMOJI_SIZE, EMOJI_SIZE, Rgba([32, 32, 32, 64]));
        }
    }
    
    println!("  ✅ Generated 256 Emoji placeholders (175 defined + 81 reserved)");
    println!("  ℹ️  Note: For colorful emojis, prepare 16x16 PNG images from Twemoji/Noto Emoji");
}

fn get_common_emoji_list() -> Vec<char> {
    // 175 most common emojis
    // Using simple Unicode representation (actual rendering will be placeholder)
    vec![
        // Emotions & Smileys (50)
        '😀', '😃', '😄', '😁', '😆', '😅', '🤣', '😂', '🙂', '🙃',
        '😉', '😊', '😇', '🥰', '😍', '🤩', '😘', '😗', '😚', '😙',
        '😋', '😛', '😜', '🤪', '😝', '🤑', '🤗', '🤭', '🤫', '🤔',
        '🤐', '🤨', '😐', '😑', '😶', '😏', '😒', '🙄', '😬', '🤥',
        '😌', '😔', '😪', '🤤', '😴', '😷', '🤒', '🤕', '🤢', '🤮',
        
        // Symbols & Signs (30)
        '✅', '❌', '⭐', '🌟', '✨', '💫', '⚡', '🔥', '💥', '💢',
        '💯', '💤', '💨', '🕳', '💬', '👁', '🗨', '🗯', '💭', '💡',
        '🔔', '🔕', '📢', '📣', '📯', '🎵', '🎶', '🔇', '🔈', '🔉',
        
        // Arrows & Indicators (20)
        '⬆', '↗', '➡', '↘', '⬇', '↙', '⬅', '↖', '↕', '↔',
        '↩', '↪', '⤴', '⤵', '🔃', '🔄', '🔙', '🔚', '🔛', '🔜',
        
        // Food & Drink (20)
        '🍇', '🍈', '🍉', '🍊', '🍋', '🍌', '🍍', '🥭', '🍎', '🍏',
        '🍐', '🍑', '🍒', '🍓', '🫐', '🥝', '🍅', '🫒', '🥥', '🥑',
        
        // Nature & Animals (20)
        '🐶', '🐱', '🐭', '🐹', '🐰', '🦊', '🐻', '🐼', '🐨', '🐯',
        '🦁', '🐮', '🐷', '🐸', '🐵', '🐔', '🐧', '🐦', '🐤', '🦆',
        
        // Objects & Tools (20)
        '📱', '💻', '⌨', '🖥', '🖨', '🖱', '🖲', '🕹', '🗜', '💾',
        '💿', '📀', '📼', '📷', '📸', '📹', '🎥', '📞', '☎', '📟',
        
        // Activities & Sports (15)
        '⚽', '🏀', '🏈', '⚾', '🥎', '🎾', '🏐', '🏉', '🥏', '🎱',
        '🏓', '🏸', '🏒', '🏑', '🥍',
    ]
}

fn draw_emoji_placeholder(img: &mut RgbaImage, x: u32, y: u32, _emoji: char, index: usize) {
    // Draw a simple colored square based on emoji category
    let color = match index {
        0..=49 => Rgba([255, 200, 100, 255]),   // Emotions: yellow-orange
        50..=79 => Rgba([100, 200, 255, 255]),  // Symbols: blue
        80..=99 => Rgba([200, 100, 255, 255]),  // Arrows: purple
        100..=119 => Rgba([255, 150, 150, 255]), // Food: red
        120..=139 => Rgba([150, 255, 150, 255]), // Nature: green
        140..=159 => Rgba([200, 200, 200, 255]), // Objects: gray
        160..=174 => Rgba([255, 100, 100, 255]), // Sports: red
        _ => Rgba([128, 128, 128, 255]),         // Reserved: gray
    };
    
    // Draw a filled square with a bit of padding
    for py in (y + 2)..(y + EMOJI_SIZE - 2) {
        for px in (x + 2)..(x + EMOJI_SIZE - 2) {
            if px < TEXTURE_SIZE && py < TEXTURE_SIZE {
                img.put_pixel(px, py, color);
            }
        }
    }
    
    // Draw index number in corner for identification
    if index < 100 {
        draw_emoji_index(img, x + 1, y + 1, index);
    }
}

fn draw_emoji_index(img: &mut RgbaImage, x: u32, y: u32, index: usize) {
    // Draw a small index number (2 digits max, simplified)
    let tens = (index / 10) as u8;
    let ones = (index % 10) as u8;
    
    if tens > 0 {
        draw_small_number(img, x, y, tens);
        draw_small_number(img, x + 4, y, ones);
    } else {
        draw_small_number(img, x, y, ones);
    }
}

fn generate_sprite_placeholders(img: &mut RgbaImage) {
    println!("🎮 Generating Sprite placeholders (1280-14591)...");
    
    // Load existing symbols.png if available
    if let Ok(old_img) = image::open("../../assets/pix/symbols.png") {
        println!("  📦 Found existing symbols.png, copying Sprite data...");
        let old_rgba = old_img.to_rgba8();
        
        // Copy sprite data from old texture
        // Assuming old texture has sprites starting from top or specific location
        copy_sprite_data(&old_rgba, img);
    } else {
        // Generate simple placeholder sprites
        let total_sprites = (SPRITE_HEIGHT / SPRITE_CHAR_SIZE * SPRITE_COLS) as usize;
        
        for i in 0..total_sprites.min(13312) {
            let col = (i as u32) % SPRITE_COLS;
            let row = (i as u32) / SPRITE_COLS;
            let x = col * SPRITE_CHAR_SIZE;
            let y = SPRITE_START_Y + row * SPRITE_CHAR_SIZE;
            
            // Draw placeholder
            if i % 10 == 0 {
                draw_placeholder_box(img, x, y, SPRITE_CHAR_SIZE, SPRITE_CHAR_SIZE, Rgba([64, 64, 64, 255]));
            }
        }
    }
    
    println!("  ✅ Generated Sprite region");
}

fn copy_sprite_data(src: &RgbaImage, dst: &mut RgbaImage) {
    // Try to copy sprite data from various locations in old texture
    let (src_w, src_h) = src.dimensions();
    
    // Assume old sprites might be in various formats, copy what we can
    for y in 0..src_h.min(SPRITE_HEIGHT) {
        for x in 0..src_w.min(TEXTURE_SIZE) {
            let pixel = src.get_pixel(x, y);
            let dst_y = SPRITE_START_Y + y;
            if dst_y < TEXTURE_SIZE {
                dst.put_pixel(x, dst_y, *pixel);
            }
        }
    }
}

fn draw_placeholder_box(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    // Draw outline
    for px in x..(x + w).min(TEXTURE_SIZE) {
        if y < TEXTURE_SIZE {
            img.put_pixel(px, y, color);
        }
        let by = y + h - 1;
        if by < TEXTURE_SIZE {
            img.put_pixel(px, by, color);
        }
    }
    for py in y..(y + h).min(TEXTURE_SIZE) {
        if x < TEXTURE_SIZE {
            img.put_pixel(x, py, color);
        }
        let rx = x + w - 1;
        if rx < TEXTURE_SIZE {
            img.put_pixel(rx, py, color);
        }
    }
}

fn draw_small_number(img: &mut RgbaImage, x: u32, y: u32, num: u8) {
    // Simple 3x5 digit rendering
    let digit_patterns = [
        // 0
        vec![
            0b111,
            0b101,
            0b101,
            0b101,
            0b111,
        ],
        // 1
        vec![
            0b010,
            0b110,
            0b010,
            0b010,
            0b111,
        ],
        // 2
        vec![
            0b111,
            0b001,
            0b111,
            0b100,
            0b111,
        ],
        // Add more digits as needed...
    ];
    
    if (num as usize) < digit_patterns.len() {
        let pattern = &digit_patterns[num as usize];
        for (row, &bits) in pattern.iter().enumerate() {
            for col in 0..3 {
                if (bits >> (2 - col)) & 1 == 1 {
                    let px = x + col;
                    let py = y + row as u32;
                    if px < TEXTURE_SIZE && py < TEXTURE_SIZE {
                        img.put_pixel(px, py, Rgba([255, 255, 0, 255]));
                    }
                }
            }
        }
    }
}

fn generate_fallback_texture(img: &mut RgbaImage) {
    println!("🔧 Generating fallback texture (no font available)...");
    
    // Generate simple TUI placeholders
    for i in 0..1024 {
        let col = i % TUI_COLS;
        let row = i / TUI_COLS;
        let x = col * TUI_CHAR_WIDTH;
        let y = TUI_START_Y + row * TUI_CHAR_HEIGHT;
        
        draw_placeholder_box(img, x, y, TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT, Rgba([128, 128, 128, 255]));
    }
    
    generate_emoji_placeholders(img);
    generate_sprite_placeholders(img);
}

