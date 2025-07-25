use fontdue::Font;
use fontdue::FontSettings;

fn main() {
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
