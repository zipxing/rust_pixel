// RustPixel
// copyright zipxing@hotmail.com 2022~2024

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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use lazy_static::lazy_static;
// use log::info;

lazy_static! {
    /// For some common chars, you can also search the char in SDL_SYM_MAP to get the offset in assets/c64.png
    /// instead of using unicode chars
    /// Some common chars a-Z and tabs are preset in SDL_SYM_MAP,
    /// for easier set of latin letters using set_str in SDL mode
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
}

/// sym_index, texture_index, fg_color, bg_color
pub type CellInfo = (u8, u8, Color, Color);

/// returns a cellsym string by index
/// 256 unicode chars mark the index of a symbol in a SDL texture
/// unicode: 0x2200 ~ 0x22FF
/// maps to a 3 byte UTF8: 11100010 100010xx 10xxxxxx
/// an 8-bits index gets from the UTF8 code is used to mark the offset in its texture
pub fn cellsym(idx: u8) -> &'static str {
    const SSYM: [&str; 256] = [
        "∀", "∁", "∂", "∃", "∄", "∅", "∆", "∇", "∈", "∉", "∊", "∋", "∌", "∍", "∎", "∏", "∐", "∑",
        "−", "∓", "∔", "∕", "∖", "∗", "∘", "∙", "√", "∛", "∜", "∝", "∞", "∟", "∠", "∡", "∢", "∣",
        "∤", "∥", "∦", "∧", "∨", "∩", "∪", "∫", "∬", "∭", "∮", "∯", "∰", "∱", "∲", "∳", "∴", "∵",
        "∶", "∷", "∸", "∹", "∺", "∻", "∼", "∽", "∾", "∿", "≀", "≁", "≂", "≃", "≄", "≅", "≆", "≇",
        "≈", "≉", "≊", "≋", "≌", "≍", "≎", "≏", "≐", "≑", "≒", "≓", "≔", "≕", "≖", "≗", "≘", "≙",
        "≚", "≛", "≜", "≝", "≞", "≟", "≠", "≡", "≢", "≣", "≤", "≥", "≦", "≧", "≨", "≩", "≪", "≫",
        "≬", "≭", "≮", "≯", "≰", "≱", "≲", "≳", "≴", "≵", "≶", "≷", "≸", "≹", "≺", "≻", "≼", "≽",
        "≾", "≿", "⊀", "⊁", "⊂", "⊃", "⊄", "⊅", "⊆", "⊇", "⊈", "⊉", "⊊", "⊋", "⊌", "⊍", "⊎", "⊏",
        "⊐", "⊑", "⊒", "⊓", "⊔", "⊕", "⊖", "⊗", "⊘", "⊙", "⊚", "⊛", "⊜", "⊝", "⊞", "⊟", "⊠", "⊡",
        "⊢", "⊣", "⊤", "⊥", "⊦", "⊧", "⊨", "⊩", "⊪", "⊫", "⊬", "⊭", "⊮", "⊯", "⊰", "⊱", "⊲", "⊳",
        "⊴", "⊵", "⊶", "⊷", "⊸", "⊹", "⊺", "⊻", "⊼", "⊽", "⊾", "⊿", "⋀", "⋁", "⋂", "⋃", "⋄", "⋅",
        "⋆", "⋇", "⋈", "⋉", "⋊", "⋋", "⋌", "⋍", "⋎", "⋏", "⋐", "⋑", "⋒", "⋓", "⋔", "⋕", "⋖", "⋗",
        "⋘", "⋙", "⋚", "⋛", "⋜", "⋝", "⋞", "⋟", "⋠", "⋡", "⋢", "⋣", "⋤", "⋥", "⋦", "⋧", "⋨", "⋩",
        "⋪", "⋫", "⋬", "⋭", "⋮", "⋯", "⋰", "⋱", "⋲", "⋳", "⋴", "⋵", "⋶", "⋷", "⋸", "⋹", "⋺", "⋻",
        "⋼", "⋽", "⋾", "⋿",
    ];
    SSYM[idx as usize]
}

/// get index idx from a symbol string
/// return idx, if it is a unicode char
/// otherwise get index from CELL_SYM_MAP
fn symidx(symbol: &String) -> u8 {
    let sbts = symbol.as_bytes();
    // unicode graphical symbol
    if sbts.len() == 3 && sbts[0] == 0xe2 && (sbts[1] >> 2 == 0x22) {
        let idx = ((sbts[1] & 3) << 6) + (sbts[2] & 0x3f);
        return idx;
    }
    let mut ret = 0u8;
    // search in CELL_SYM_MAP
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

    /// refers to the comments in buffer.rs, works in graphical mode
    /// returns offset and texture id
    ///
    /// maps to a 3 byte UTF8: 11100010 100010xx 10xxxxxx
    /// an 8-digits index gets from the UTF8 code is used to mark the offset in its texture
    ///
    /// refers to the flush method in panel.rs
    ///
    /// sym_index, texture_index, fg_color, bg_color
    pub fn get_cell_info(&self) -> CellInfo {
        (symidx(&self.symbol), self.tex, self.fg, self.bg)
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
        self.modifier = Modifier::empty();
    }

    #[cfg(any(target_arch = "wasm32", feature = "sdl"))]
    pub fn is_blank(&self) -> bool {
        let bg = u8::from(self.bg);
        (self.symbol == " " || self.symbol == cellsym(32)) && (bg == 0 || bg == 1)
    }

    #[cfg(all(not(target_arch = "wasm32"), not(feature = "sdl")))]
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
