// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Cell is the basic unit for rendering in RustPixel.
//! Each cell stores a character/symbol, foreground color, background color,
//! and texture information for graphics mode.

//! Cell is the basic rendering data structure in RustPixel, represents a char
//! Cell stores some key data such as symbol, tex(graph mode only), fg, bg.
//! Many Cells form a buffer to manage rendering.
//!
//! A buffer comprises a cell vector with width * height elements
//!
//! Please refer to the code (cellsym, symidx, get_cell_info, CELL_SYM_MAP) for
//! how to use cell.
//!

use crate::render::style::{Color, Modifier, Style};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use log::info;

lazy_static! {
    /// For some common chars, you can also search the char in SDL_SYM_MAP to get the offset in assets/pix/symbols.png
    /// instead of using unicode chars
    /// Some common chars a-Z and tabs are preset in SDL_SYM_MAP,
    /// for easier set of latin letters using set_str in GRAPH mode
    /// refer to comments for more details
    static ref CELL_SYM_MAP: HashMap<String, u8> = {
        let syms = "@abcdefghijklmnopqrstuvwxyz[£]↑← !\"#$%&'()*+,-./0123456789:;<=>?─ABCDEFGHIJKLMNOPQRSTUVWXYZ┼";
        let mut sm: HashMap<String, u8> = HashMap::from([
            ("▇".to_string(), 209),
            ("▒".to_string(), 94),
            ("∙".to_string(), 122),
            ("│".to_string(), 93),
            ("┐".to_string(), 110),
            ("╮".to_string(), 73),
            ("┌".to_string(), 112),
            ("╭".to_string(), 85),
            ("└".to_string(), 109),
            ("╰".to_string(), 74),
            ("┘".to_string(), 125),
            ("╯".to_string(), 75),
        ]);
        for (i, s) in syms.chars().enumerate() {
            sm.insert(s.to_string(), i as u8);
        }
        sm
    };

    /// Emoji mapping table for pre-rendered Emoji in the unified texture
    /// 
    /// Maps common Emoji characters to texture indices in the Emoji region (1024-1279).
    /// The Emoji region occupies rows 128-191 of the 1024x1024 unified texture,
    /// with each Emoji being 16x16 pixels in RGBA color format.
    /// 
    /// Total capacity: 256 Emoji positions
    /// - 175 common Emoji (mapped below)
    /// - 81 reserved for future expansion
    /// 
    /// Emoji categories:
    /// - Emotions & Faces (50): 😀😊😂🤣😍🥰😘😎🤔😭🥺😤😡🤯😱 etc.
    /// - Symbols & Signs (30): ✅❌⚠️🔥⭐🌟✨💫🎯🚀⚡💡🔔📌🔗🔒 etc.
    /// - Arrows & Indicators (20): ➡️⬅️⬆️⬇️↗️↘️↙️↖️🔄🔃 etc.
    /// - Food & Drink (20): 🍕🍔🍟🍿🍩🍪🍰🎂🍭🍫☕🍺🍷 etc.
    /// - Nature & Animals (20): 🌈🌸🌺🌻🌲🌳🍀🐱🐶🐭🐹🦊🐻 etc.
    /// - Objects & Tools (20): 📁📂📄📊📈📉🔧🔨⚙️🖥️💻⌨️🖱️ etc.
    /// - Activities & Sports (15): ⚽🏀🏈⚾🎮🎲🎯🎨🎭🎪 etc.
    static ref EMOJI_MAP: HashMap<String, (u8, u8)> = {
        let mut map = HashMap::new();
        // Start from Block 53, Index 0
        let mut block = 53u8;
        let mut idx = 0u8;
        
        // Emotions & Faces (50)
        let emotions = ["😀", "😊", "😂", "🤣", "😍", "🥰", "😘", "😎", "🤔", "😭",
                       "🥺", "😤", "😡", "🤯", "😱", "😨", "😰", "😥", "😢", "😓",
                       "😩", "😫", "🥱", "😴", "😪", "🤐", "😬", "🙄", "😏", "😒",
                       "😞", "😔", "😟", "😕", "🙁", "☹️", "😣", "😖", "😫", "😩",
                       "🥳", "😇", "🤠", "🤡", "🤥", "🤫", "🤭", "🧐", "🤓", "😈"];
        for emoji in &emotions {
            map.insert(emoji.to_string(), (block, idx));
            idx += 1;
            if idx == 128 { // Each Emoji block holds 128 emojis (8x16 grid of 16x16px)
                idx = 0;
                block += 1;
            }
        }
        
        // Symbols & Signs (30)
        let symbols = ["✅", "❌", "⚠️", "🔥", "⭐", "🌟", "✨", "💫", "🎯", "🚀",
                      "⚡", "💡", "🔔", "📌", "🔗", "🔒", "🔓", "🔑", "🎁", "🎈",
                      "🎉", "🎊", "💯", "🆕", "🆓", "🆒", "🆗", "🆙", "🔴", "🟢"];
        for emoji in &symbols {
            map.insert(emoji.to_string(), (block, idx));
            idx += 1;
            if idx == 128 {
                idx = 0;
                block += 1;
            }
        }
        
        // Arrows & Indicators (20)
        let arrows = ["➡️", "⬅️", "⬆️", "⬇️", "↗️", "↘️", "↙️", "↖️", "🔄", "🔃",
                     "⏪", "⏩", "⏫", "⏬", "▶️", "◀️", "🔼", "🔽", "⏸️", "⏹️"];
        for emoji in &arrows {
            map.insert(emoji.to_string(), (block, idx));
            idx += 1;
            if idx == 128 {
                idx = 0;
                block += 1;
            }
        }
        
        // Food & Drink (20)
        let food = ["🍕", "🍔", "🍟", "🍿", "🍩", "🍪", "🍰", "🎂", "🍭", "🍫",
                   "☕", "🍺", "🍷", "🍹", "🥤", "🍎", "🍌", "🍇", "🍓", "🍉"];
        for emoji in &food {
            map.insert(emoji.to_string(), (block, idx));
            idx += 1;
            if idx == 128 {
                idx = 0;
                block += 1;
            }
        }
        
        // Nature & Animals (20)
        let nature = ["🌈", "🌸", "🌺", "🌻", "🌲", "🌳", "🍀", "🐱", "🐶", "🐭",
                     "🐹", "🦊", "🐻", "🐼", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸"];
        for emoji in &nature {
            map.insert(emoji.to_string(), (block, idx));
            idx += 1;
            if idx == 128 {
                idx = 0;
                block += 1;
            }
        }
        
        // Objects & Tools (20)
        let objects = ["📁", "📂", "📄", "📊", "📈", "📉", "🔧", "🔨", "⚙️", "🖥️",
                      "💻", "⌨️", "🖱️", "📱", "☎️", "📞", "📟", "📠", "🔋", "🔌"];
        for emoji in &objects {
            map.insert(emoji.to_string(), (block, idx));
            idx += 1;
            if idx == 128 {
                idx = 0;
                block += 1;
            }
        }
        
        // Activities & Sports (15)
        let activities = ["⚽", "🏀", "🏈", "⚾", "🎮", "🎲", "🎯", "🎨", "🎭", "🎪",
                         "🎬", "🎤", "🎧", "🎼", "🎹"];
        for emoji in &activities {
            map.insert(emoji.to_string(), (block, idx));
            idx += 1;
            if idx == 128 {
                idx = 0;
                block += 1;
            }
        }
        
        map
    };

    /// TUI Symbol Map for TUI mode (Block 48+)
    /// Matches the layout of symbols/tui.txt
    static ref TUI_CELL_SYM_MAP: HashMap<String, (u8, u8)> = {
        // Note: The order must exactly match symbols/tui.txt
        // We construct the string by concatenating lines from tui.txt
        // Line 3:  |!|#|$|%|&|(|)|*|+|,|-|.|/|0|1|2|3
        // Line 4: 4|5|6|7|8|9|:|;|"|'|<|=|>|?|@|[|\|]
        // Line 5: ^|_|`|{|||}|~|⌐|¬|½|¼|¡|«|»|∙|·|※|⦿
        // ... and so on.
        // Note: The first character in Line 3 is Space? User says symbols.png starts with !.
        // If tui.txt starts with Space but symbols.png starts with !, then Space was skipped.
        // However, tui.txt Line 15 has a Space explicitly.
        // Let's assume the first char is ! as per user feedback, and handle Space separately.
        
        let syms = concat!(
            " !#$%&()*+,-./01",
            "23456789:;\"'<=>?",  
            "@[\\]^_`{|}~⌐¬½¼¡",  
            "«»∙·※⦿ABCDEFGHIJ",
            "KLMNOPQRSTUVWXYZ",
            "abcdefghijklmnop",
            "qrstuvwxyz▀▄äà",
            "åçêëèïîìÄÅÉæÆôöò",
            "ûùÿÖÜ¢£¥₧ƒáíóúñÑ",
            "ªº¿αßΓπΣσµτΦΘΩδ∞",
            "φε∩≡±≥≤⌠⌡÷≈‾√ⁿ²♠",
            "♣♥♦░▒▓▙▟▛▜⚆⚇⚈⚉",
            "◐◑◓◒▴▾◂▸←↑→↓⭠⭡⭢⭣",
            "⠁⠂⠄⠈⠐⠠⡀⢀█▉▊▋▌▍▎▏",
            "█▇▆▅▄▃▂▁│║┃─═━┐╮",
            "╗┓┌╭╔┏┘╯╝┛└╰╚┗┤╣",
            "┫├╠┣┬╦┳┴╩┻┼╬╋≋"
        );
        
        let mut sm = HashMap::new();
        for (i, c) in syms.chars().enumerate() {
            let block = 48 + (i / 256) as u8;
            let idx = (i % 256) as u8;
            sm.insert(c.to_string(), (block, idx));
        }
        
        sm
    };
}

/// Get TUI symbol index and block
pub fn tui_symidx(symbol: &str) -> Option<(u8, u8)> {
    TUI_CELL_SYM_MAP.get(symbol).copied()
}

/// sym_index, texture_index, fg_color, bg_color, modifier
pub type CellInfo = (u8, u8, Color, Color, Modifier);

/// returns a cellsym string by index
/// 256 unicode chars mark the index of a symbol in a SDL texture
/// unicode: 0xE000 ~ 0xE0FF (Private Use Area)
/// maps to a 3 byte UTF8: 11101110 100000xx 10xxxxxx
/// an 8-bits index gets from the UTF8 code is used to mark the offset in its texture
/// 
/// Using Private Use Area ensures no conflict with standard Unicode characters,
/// allowing applications to display mathematical symbols (∀∃∈∞≈≤≥⊕⊗) or other
/// special characters in TUI mode without interference.
pub fn cellsym(idx: u8) -> String {
    // U+E000 + idx
    let codepoint = 0xE000u32 + idx as u32;
    char::from_u32(codepoint).unwrap().to_string()
}

/// Check if a symbol is a pre-rendered Emoji
///
/// Returns true if the symbol exists in the EMOJI_MAP, meaning it has
/// a pre-rendered 16x16 RGBA image in the Emoji region of the texture.
pub fn is_prerendered_emoji(symbol: &str) -> bool {
    EMOJI_MAP.contains_key(symbol)
}

/// Get the texture index and symbol index for a pre-rendered Emoji
///
/// Returns Some((block_idx, sym_idx)) if the Emoji is in the EMOJI_MAP,
/// or None if the Emoji is not pre-rendered.
///
/// This function handles emoji with variation selectors (U+FE0F) by
/// stripping them if direct lookup fails.
pub fn emoji_texidx(symbol: &str) -> Option<(u8, u8)> {
    // Try direct lookup first
    if let Some(result) = EMOJI_MAP.get(symbol) {
        return Some(*result);
    }

    // If not found, try stripping variation selector (U+FE0F) and ZWJ (U+200D)
    let stripped: String = symbol.chars()
        .filter(|&c| c != '\u{FE0F}' && c != '\u{200D}')
        .collect();

    if stripped != symbol {
        EMOJI_MAP.get(&stripped).copied()
    } else {
        None
    }
}

/// get index idx from a symbol string
/// return idx, if it is a unicode char in Private Use Area (U+E000~U+E0FF)
/// otherwise get index from CELL_SYM_MAP
fn symidx(symbol: &String) -> u8 {
    let sbts = symbol.as_bytes();
    // Private Use Area: U+E000~U+E0FF
    // UTF-8: 11101110 100000xx 10xxxxxx (0xEE 0x80~0x83 0x80~0xBF)
    if sbts.len() == 3 && sbts[0] == 0xEE && (sbts[1] >> 2 == 0x20) {
        let idx = ((sbts[1] & 3) << 6) + (sbts[2] & 0x3f);
        return idx;
    }
    let mut ret = 0u8;
    // search in CELL_SYM_MAP for common ASCII chars
    if let Some(idx) = CELL_SYM_MAP.get(symbol) {
        ret = *idx;
    }
    ret
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    pub tex: u8,
}

impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        self
    }

    /// refers to the comments in buffer.rs, works in graphics mode
    /// returns offset, texture id, colors, and modifier
    ///
    /// maps to a 3 byte UTF8: 11100010 100010xx 10xxxxxx
    /// an 8-digits index gets from the UTF8 code is used to mark the offset in its texture
    ///
    /// refers to the flush method in panel.rs
    ///
    /// sym_index, texture_index, fg_color, bg_color, modifier
    pub fn get_cell_info(&self) -> CellInfo {
        // Check for Emoji first
        if let Some((block, idx)) = EMOJI_MAP.get(&self.symbol) {
            return (*idx, *block, self.fg, self.bg, self.modifier);
        }
        
        // For Sprite texture (tex=0), handle space character specially
        // CELL_SYM_MAP maps space to index 33, but in Sprite texture it should be 32
        if self.tex == 0 && self.symbol == " " {
            return (32, 0, self.fg, self.bg, self.modifier);
        }
        
        (symidx(&self.symbol), self.tex, self.fg, self.bg, self.modifier)
    }

    pub fn set_char(&mut self, ch: char) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push(ch);
        self
    }

    pub fn set_texture(&mut self, tex: u8) -> &mut Cell {
        self.tex = tex;
        self
    }

    pub fn set_fg(&mut self, color: Color) -> &mut Cell {
        self.fg = color;
        self
    }

    pub fn set_bg(&mut self, color: Color) -> &mut Cell {
        self.bg = color;
        self
    }

    pub fn set_style(&mut self, style: Style) -> &mut Cell {
        if let Some(c) = style.fg {
            self.fg = c;
        }
        if let Some(c) = style.bg {
            self.bg = c;
        }
        self.modifier.insert(style.add_modifier);
        self.modifier.remove(style.sub_modifier);
        self
    }

    pub fn style(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg)
            .add_modifier(self.modifier)
    }

    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.tex = 0;
        self.modifier = Modifier::empty();
    }

    #[cfg(graphics_mode)]
    pub fn is_blank(&self) -> bool {
        (self.symbol == " " || self.symbol == cellsym(32))
            && (self.tex == 0 || self.tex == 1)
            && self.bg == Color::Reset
    }

    #[cfg(not(graphics_mode))]
    pub fn is_blank(&self) -> bool {
        self.symbol == " " && self.fg == Color::Reset && self.bg == Color::Reset
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell {
            symbol: " ".into(),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
            tex: 0,
        }
    }
}
