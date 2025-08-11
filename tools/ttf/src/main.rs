use fontdue::{Font, FontSettings};
use image::{ImageBuffer, Rgb, RgbImage};
use std::env;
use std::path::Path;
use ttf_parser::Face;

/// Displays comprehensive usage information for the TTF to PNG converter tool
fn print_ttf_usage() {
    println!("TTF to PNG Converter Tool v1.0");
    println!("Converts TTF font files to PNG character atlas using auto-discovery");
    println!();
    println!("USAGE:");
    println!("    ttf <TTF_FILE> [OUTPUT_FILE] [SIZE] [CHARS_PER_ROW] [VERBOSE]");
    println!();
    println!("ARGUMENTS:");
    println!("    TTF_FILE        Path to input TTF font file (required)");
    println!("    OUTPUT_FILE     Output PNG file path (default: font_atlas.png)");
    println!("    SIZE            Character size in pixels (default: 16)");
    println!("    CHARS_PER_ROW   Number of characters per row (default: 16)");
    println!("    VERBOSE         Show detailed analysis: 0=false, 1=true (default: 0)");
    println!();
    println!("EXAMPLES:");
    println!("    ttf font.ttf                        # Convert with defaults");
    println!("    ttf font.ttf output.png             # Specify output file");
    println!("    ttf font.ttf output.png 8           # 8x8 pixel characters");
    println!("    ttf font.ttf output.png 8 32        # 8x8 characters, 32 per row");
    println!("    ttf font.ttf output.png 8 32 1      # With verbose output");
    println!();
    println!("FEATURES:");
    println!("    • Auto-discovers all available characters using cmap table");
    println!("    • Analyzes maxp table for glyph count information");
    println!("    • Smart character filtering (includes content + whitespace)");
    println!("    • No duplicate characters");
    println!("    • Supports Unicode, extended Latin, symbols, and box drawing");
}

fn main() {
    let input_file;
    let mut output_file = "font_atlas.png".to_string();
    let mut char_size: u32 = 16;
    let mut chars_per_row: u32 = 16;
    let mut verbose = false;

    let args: Vec<String> = env::args().collect();

    // Check for help argument
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_ttf_usage();
        return;
    }

    if args.len() < 2 {
        eprintln!("Error: Missing required TTF_FILE argument");
        eprintln!();
        print_ttf_usage();
        std::process::exit(1);
    }

    input_file = &args[1];
    if !Path::new(input_file).exists() {
        eprintln!("Error: TTF file '{}' does not exist", input_file);
        std::process::exit(1);
    }

    if args.len() > 2 {
        output_file = args[2].clone();
    }
    if args.len() > 3 {
        char_size = args[3].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid SIZE value '{}'", args[3]);
            std::process::exit(1);
        });
    }
    if args.len() > 4 {
        chars_per_row = args[4].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid CHARS_PER_ROW value '{}'", args[4]);
            std::process::exit(1);
        });
    }
    if args.len() > 5 {
        let verbose_arg: u32 = args[5].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid VERBOSE value '{}' (use 0 or 1)", args[5]);
            std::process::exit(1);
        });
        verbose = verbose_arg != 0;
    }

    println!("Processing TTF font: {}", input_file);
    println!("Output PNG: {}", output_file);
    println!("Character size: {}x{} pixels", char_size, char_size);
    println!("Characters per row: {}", chars_per_row);

    match convert_ttf_to_png(input_file, &output_file, char_size, chars_per_row, verbose) {
        Ok(()) => println!("✅ Successfully generated {}", output_file),
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn convert_ttf_to_png(
    input_file: &str,
    output_file: &str,
    char_size: u32,
    chars_per_row: u32,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the TTF font
    let font_data = std::fs::read(input_file)?;
    let font = Font::from_bytes(font_data.clone(), FontSettings::default())?;

    // Use cmap table to discover all available characters
    let codepoints_to_process = discover_available_characters(&font_data, verbose)?;
    
    if verbose {
        println!("Processing {} codepoints...", codepoints_to_process.len());
    }
    
    // Collect valid characters - now using smart filtering
    let mut valid_chars = Vec::new();
    
    for codepoint in codepoints_to_process {
        if let Some(character) = char::from_u32(codepoint) {
            // Test rasterization to ensure the character can be rendered
            let (metrics, bitmap) = font.rasterize(character, char_size as f32);
            
            // Smart inclusion logic: include if it has content OR is whitespace
            let is_whitespace = character.is_whitespace();
            let has_content = metrics.width > 0 && metrics.height > 0 && !bitmap.is_empty();
            
            if has_content || is_whitespace {
                valid_chars.push(character);
                if verbose {
                    let glyph_index = font.lookup_glyph_index(character);
                    if is_whitespace && !has_content {
                        let char_display = if character == ' ' { 
                            "SPACE".to_string() 
                        } else { 
                            format!("{}", character) 
                        };
                        println!("Added whitespace character '{}' (U+{:04X}) - glyph index {}", 
                                 char_display, codepoint, glyph_index);
                    } else {
                        println!("Added character '{}' (U+{:04X}) - glyph index {}, size {}x{}", 
                                 character, codepoint, glyph_index, metrics.width, metrics.height);
                    }
                }
            } else if verbose {
                println!("Skipping character '{}' (U+{:04X}) - no renderable content", character, codepoint);
            }
        }
    }

    if valid_chars.is_empty() {
        return Err("No valid characters found in the specified range".into());
    }

    println!("Found {} valid characters", valid_chars.len());

    // Calculate image dimensions
    let char_count = valid_chars.len() as u32;
    let rows = (char_count + chars_per_row - 1) / chars_per_row; // Ceiling division
    let image_width = chars_per_row * char_size;
    let image_height = rows * char_size;

    println!("Image dimensions: {}x{} pixels ({} rows)", image_width, image_height, rows);

    // Create the image buffer
    let mut img: RgbImage = ImageBuffer::new(image_width, image_height);

    // Fill with white background
    for pixel in img.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }

    // Render each character
    for (index, character) in valid_chars.iter().enumerate() {
        let row = index as u32 / chars_per_row;
        let col = index as u32 % chars_per_row;
        let x_offset = col * char_size;
        let y_offset = row * char_size;

        render_character_to_image(&font, *character, char_size, &mut img, x_offset, y_offset)?;
        
        if index % 10 == 0 {
            print!(".");
        }
    }
    println!(); // New line after progress dots

    // Save the image
    img.save(output_file)?;
    
    Ok(())
}

fn render_character_to_image(
    font: &Font,
    character: char,
    char_size: u32,
    img: &mut RgbImage,
    x_offset: u32,
    y_offset: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Rasterize the character
    let (metrics, bitmap) = font.rasterize(character, char_size as f32);

    // Skip characters with no renderable content
    if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
        return Ok(());
    }

    // Calculate positioning to center the character (use i32 for calculations)
    let char_x = x_offset as i32 + (char_size as i32 - metrics.width as i32) / 2 + metrics.xmin;
    let char_y = y_offset as i32 + (char_size as i32 - metrics.height as i32) / 2 - metrics.ymin;

    // Draw the character bitmap
    for (y, row) in bitmap.chunks(metrics.width).enumerate() {
        for (x, &alpha) in row.iter().enumerate() {
            let pixel_x = char_x + x as i32;
            let pixel_y = char_y + y as i32;

            // Check bounds
            if pixel_x >= 0 && pixel_y >= 0 && 
               pixel_x < img.width() as i32 && pixel_y < img.height() as i32 {
                
                let pixel_x = pixel_x as u32;
                let pixel_y = pixel_y as u32;

                // Blend alpha with white background
                let intensity = 255 - alpha; // Invert alpha (fontdue gives coverage, we want darkness)
                let pixel = Rgb([intensity, intensity, intensity]);
                
                img.put_pixel(pixel_x, pixel_y, pixel);
            }
        }
    }

    Ok(())
}

/// Discover all available characters by reading the font's cmap table
fn discover_available_characters(font_data: &[u8], verbose: bool) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let face = Face::parse(font_data, 0)?;
    let mut available_chars = Vec::new();
    let mut glyph_to_char_map = std::collections::HashMap::new(); // Track which characters map to each glyph
    
    if verbose {
        println!("Reading cmap table to discover available characters...");
        
        // Read maxp table to get accurate glyph count
        let maxp = face.tables().maxp;
        println!("\n=== MAXP TABLE ===");
        println!("Number of glyphs: {}", maxp.number_of_glyphs);
        
        // Print cmap table information
        println!("\n=== CMAP TABLE ===");
        if let Some(cmap) = face.tables().cmap {
            let mut subtable_count = 0;
            
            // Iterate through all available subtables
            for subtable in cmap.subtables {
                println!("Subtable {}: Platform ID: {:?}, Encoding ID: {}", 
                    subtable_count, subtable.platform_id, subtable.encoding_id);
                
                // Print some sample mappings from this subtable
                println!("  Sample mappings:");
                let mut sample_count = 0;
                for code in 0x20u32..0x7F {  // ASCII printable range
                    if let Some(glyph_id) = subtable.glyph_index(code) {
                        if glyph_id.0 != 0 && sample_count < 5 {
                            if let Some(ch) = char::from_u32(code) {
                                println!("    '{}' (U+{:04X}) -> glyph {}", ch, code, glyph_id.0);
                                sample_count += 1;
                            }
                        }
                    }
                }
                
                subtable_count += 1;
                if subtable_count >= 3 {  // Limit to first 3 subtables
                    break;
                }
            }
        } else {
            println!("No cmap table found!");
        }
        println!();
    }
    
    // Iterate through all possible Unicode codepoints in reasonable ranges
    // We'll check common Unicode blocks that are likely to be supported
    let ranges = [
        (0x0000, 0x007F),   // Basic Latin
        (0x0080, 0x00FF),   // Latin-1 Supplement
        (0x0100, 0x017F),   // Latin Extended-A
        (0x0180, 0x024F),   // Latin Extended-B
        (0x1E00, 0x1EFF),   // Latin Extended Additional
        (0x2000, 0x206F),   // General Punctuation
        (0x2070, 0x209F),   // Superscripts and Subscripts
        (0x20A0, 0x20CF),   // Currency Symbols
        (0x2100, 0x214F),   // Letterlike Symbols
        (0x2150, 0x218F),   // Number Forms
        (0x2190, 0x21FF),   // Arrows
        (0x2200, 0x22FF),   // Mathematical Operators
        (0x2300, 0x23FF),   // Miscellaneous Technical
        (0x2400, 0x243F),   // Control Pictures
        (0x2440, 0x245F),   // Optical Character Recognition
        (0x2460, 0x24FF),   // Enclosed Alphanumerics
        (0x2500, 0x257F),   // Box Drawing
        (0x2580, 0x259F),   // Block Elements
        (0x25A0, 0x25FF),   // Geometric Shapes
        (0x2600, 0x26FF),   // Miscellaneous Symbols
        (0x2700, 0x27BF),   // Dingbats
        (0x27C0, 0x27EF),   // Miscellaneous Mathematical Symbols-A
        (0x27F0, 0x27FF),   // Supplemental Arrows-A
        (0x2800, 0x28FF),   // Braille Patterns
        (0x2900, 0x297F),   // Supplemental Arrows-B
        (0x2980, 0x29FF),   // Miscellaneous Mathematical Symbols-B
        (0x2A00, 0x2AFF),   // Supplemental Mathematical Operators
        (0x2B00, 0x2BFF),   // Miscellaneous Symbols and Arrows
        (0xFB00, 0xFB4F),   // Alphabetic Presentation Forms
        (0xFFF0, 0xFFFF),   // Specials
    ];
    
    for (start, end) in ranges {
        for codepoint in start..=end {
            if let Some(glyph_id) = face.glyph_index(char::from_u32(codepoint).unwrap_or('\0')) {
                if glyph_id.0 != 0 { // glyph_id.0 == 0 means missing glyph
                    // Track glyph to character mapping
                    glyph_to_char_map.entry(glyph_id.0).or_insert_with(Vec::new).push(codepoint);
                    
                    available_chars.push(codepoint);
                    if verbose && available_chars.len() <= 20 {
                        if let Some(ch) = char::from_u32(codepoint) {
                            println!("Found character: '{}' (U+{:04X}) -> glyph {}", ch, codepoint, glyph_id.0);
                        }
                    }
                }
            }
        }
    }
    
    if verbose {
        println!("Discovered {} characters from cmap table", available_chars.len());
        if available_chars.len() > 20 {
            println!("(showing first 20 characters above)");
        }
        
        // Analyze glyph reuse
        let unique_glyphs = glyph_to_char_map.len();
        let total_mappings = available_chars.len();
        println!("Unique glyphs: {}", unique_glyphs);
        println!("Total character mappings: {}", total_mappings);
        println!("Duplicate ratio: {:.1}%", (total_mappings - unique_glyphs) as f32 / total_mappings as f32 * 100.0);
        
        // Show examples of glyph reuse
        let mut reused_glyphs: Vec<_> = glyph_to_char_map.iter()
            .filter(|(_, chars)| chars.len() > 1)
            .collect();
        reused_glyphs.sort_by_key(|(_, chars)| chars.len());
        reused_glyphs.reverse();
        
        if !reused_glyphs.is_empty() {
            println!("\nTop glyph reuse examples:");
            for (glyph_id, chars) in reused_glyphs.iter().take(5) {
                print!("Glyph {}: ", glyph_id);
                for &codepoint in chars.iter().take(3) {
                    if let Some(ch) = char::from_u32(codepoint) {
                        print!("'{}' (U+{:04X}) ", ch, codepoint);
                    }
                }
                if chars.len() > 3 {
                    print!("... (+{} more)", chars.len() - 3);
                }
                println!();
            }
        }
    }
    
    available_chars.sort();
    Ok(available_chars)
}