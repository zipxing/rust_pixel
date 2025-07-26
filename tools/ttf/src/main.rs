use fontdue::Font;
use fontdue::FontSettings;
use std::env;

fn print_ttf_usage() {
    eprintln!("RustPixel TTF Font Processor");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    ttf");
    eprintln!("    cargo pixel ttf");
    eprintln!("    cargo pixel tf");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Processes TTF font files to generate bitmap character data suitable");
    eprintln!("    for pixel art applications. Converts each character glyph into an 8x8");
    eprintln!("    pixel bitmap representation for use in terminal or retro-style graphics.");
    eprintln!();
    eprintln!("INPUT:");
    eprintln!("    assets/pixel8.ttf  - TTF font file (must exist)");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    - Character bitmap data printed to console");
    eprintln!("    - Each character displayed as 8x8 pixel grid");
    eprintln!("    - Covers codepoints 0x00 to 0xFF");
    eprintln!("    - Uses █ and space characters for visualization");
    eprintln!();
    eprintln!("PROCESSING:");
    eprintln!("    - Loads TTF font from assets/pixel8.ttf");
    eprintln!("    - Rasterizes each character at 8pt size");
    eprintln!("    - Converts to binary 8x8 bitmap");
    eprintln!("    - Displays character with its bitmap pattern");
    eprintln!();
    eprintln!("FEATURES:");
    eprintln!("    - Automatic glyph detection");
    eprintln!("    - Fixed 8x8 output format");
    eprintln!("    - ASCII/Extended ASCII character set");
    eprintln!("    - Terminal-friendly output format");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    ttf                              # Process default font");
    eprintln!("    ttf > font_data.txt             # Save output to file");
    eprintln!();
    eprintln!("REQUIREMENTS:");
    eprintln!("    - TTF font file must exist at: assets/pixel8.ttf");
    eprintln!("    - Font should be monospace for best results");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    When used via cargo-pixel, equivalent to: cargo pixel r ttf t -r");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Check for help argument
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_ttf_usage();
        return;
    }
    
    // font image size
    let n = 8; 

    let font_data = std::fs::read("assets/pixel8.ttf").expect("read ttf error");
    let font = Font::from_bytes(font_data, FontSettings::default()).expect("parse ttf error");

    let start_codepoint = 0x00; 
    let end_codepoint = 0xFF; 

    for codepoint in start_codepoint..=end_codepoint {
        if let Some(character) = char::from_u32(codepoint) {
            if font.lookup_glyph_index(character) != 0 {
                let (metrics, bitmap) = font.rasterize(character, n as f32);

                let bitmap_nxn = gen_bitmap(
                    &bitmap,
                    metrics.xmin as usize,
                    metrics.ymin as usize,
                    metrics.width,
                    metrics.height,
                    n,
                    n,
                );

                println!("char: '{}'", character);
                print_bitmap(&bitmap_nxn);
            }
        }
    }
}

fn gen_bitmap(
    bitmap: &[u8],
    xmin: usize,
    ymin: usize,
    width: usize,
    height: usize,
    new_width: usize,
    new_height: usize,
) -> Vec<Vec<u8>> {
    let mut resized = vec![vec![0u8; new_width]; new_height];
    for y in 0..new_height {
        for x in 0..new_width {
            let src_x = x;
            let src_y = y;
            if src_x < width && src_y < height {
                resized[y + (new_height - ymin - height)][x + xmin] = bitmap[src_y * width + src_x];
            }
        }
    }
    resized
}

fn print_bitmap(bitmap: &[Vec<u8>]) {
    for row in bitmap {
        for &pixel in row {
            if pixel > 128 {
                print!("█");
            } else {
                print!(" ");
            }
        }
        println!();
    }
}
