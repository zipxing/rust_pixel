// RustPixel
// copyright zipxing@hotmail.com 2022～2026
//
// Character classification utilities

/// Check if character is a graphic character (box drawing, blocks, etc.)
/// These need to fill the cell completely for proper tiling
pub fn is_graphic_char(ch: char) -> bool {
    let cp = ch as u32;
    (0x2500..=0x257F).contains(&cp)  // Box Drawing
        || (0x2580..=0x259F).contains(&cp)  // Block Elements
        || (0x2800..=0x28FF).contains(&cp)  // Braille Patterns
        || cp >= 0xE000  // Private Use / NerdFont / Powerline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_graphic_char() {
        assert!(is_graphic_char('─')); // Box drawing
        assert!(is_graphic_char('█')); // Block element
        assert!(is_graphic_char('⠀')); // Braille
        assert!(!is_graphic_char('A')); // Regular char
        assert!(!is_graphic_char('你')); // CJK
    }
}
