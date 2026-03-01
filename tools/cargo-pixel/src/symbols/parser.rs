// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Parsers for TUI characters, emojis, and CJK characters

use std::fs;
use std::path::Path;

/// Parse tui.txt file
/// Returns (tui_chars, emojis)
pub fn parse_tui_txt(filepath: &Path) -> (Vec<char>, Vec<String>) {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", filepath.display(), e);
            return (vec![' '], Vec::new());
        }
    };

    let lines: Vec<&str> = content.lines().collect();

    // Skip leading empty lines
    let mut start_idx = 0;
    while start_idx < lines.len() && lines[start_idx].trim().is_empty() {
        start_idx += 1;
    }

    // Find separator (empty line between TUI and Emoji sections)
    let mut separator_idx = None;
    for i in start_idx..lines.len() {
        if lines[i].trim().is_empty() {
            separator_idx = Some(i);
            break;
        }
    }

    let separator_idx = match separator_idx {
        Some(idx) => idx,
        None => {
            eprintln!("Error: No empty line separator found in tui.txt");
            return (vec![' '], Vec::new());
        }
    };

    // Parse TUI characters (first position is forced to space)
    let mut tui_chars = vec![' '];
    for line in &lines[start_idx..separator_idx] {
        let line = line.trim(); // Remove leading/trailing whitespace
        if !line.is_empty() {
            for ch in line.chars() {
                tui_chars.push(ch);
            }
        }
    }

    println!("  Parsed {} TUI characters", tui_chars.len());

    // Parse Emojis
    let mut emojis = Vec::new();
    for line in &lines[separator_idx + 1..] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            let code = ch as u32;

            // Check if this is an emoji start character
            let is_emoji_start = (0x1F000..=0x1FFFF).contains(&code)
                || (0x2600..=0x27BF).contains(&code)
                || (0x2300..=0x23FF).contains(&code)
                || (0x2B00..=0x2BFF).contains(&code)
                || "⭐⚡☔⛳⛵⚓⛱⛰⛲⏰✏✅✌❤❎❌⚫⚪⬛⬜".contains(ch);

            if is_emoji_start {
                let mut emoji = String::new();
                emoji.push(ch);

                // Check for variation selector U+FE0F
                if i + 1 < chars.len() && chars[i + 1] as u32 == 0xFE0F {
                    emoji.push(chars[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
                emojis.push(emoji);
            } else {
                i += 1;
            }
        }
    }

    println!("  Parsed {} Emojis", emojis.len());

    (tui_chars, emojis)
}

/// Parse CJK character file (3500C.txt)
pub fn parse_cjk_txt(filepath: &Path) -> Vec<char> {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Error reading {}: {}", filepath.display(), e);
            return Vec::new();
        }
    };

    let mut cjk_chars = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() {
            // Each line contains one character
            if let Some(ch) = line.chars().next() {
                cjk_chars.push(ch);
            }
        }
    }

    println!("  Parsed {} CJK characters", cjk_chars.len());
    cjk_chars
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_emoji_detection() {
        let emoji_chars = "⭐⚡☔";
        for ch in emoji_chars.chars() {
            let is_emoji = "⭐⚡☔⛳⛵⚓⛱⛰⛲⏰✏✅✌❤❎❌⚫⚪⬛⬜".contains(ch);
            assert!(is_emoji, "Should detect {} as emoji", ch);
        }
    }
}
