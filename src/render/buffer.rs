// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Buffer is a basic rendering unit in RustPixel, represents a rectangle area.
//! A buffer comprises a cell vector with width * height elements.
//! A cell stores key data such as symbol, fg, bg.
//!
//! Almost all Unicode chars can be drawn in text mode, depending on the terminal apps
//! (use of iTerm2 in macOS is recommended). For example:
//! ```
//! my_buffer.set_str(0, 0, "Hello world 😃.",
//!     Style::default().fg(Color::Red).bg(Color::Reset))
//! ```
//!
//! Beware of the display width and height of Unicode chars.
//! Display width is a bit tricky, very much relying on the terminal apps (currently the development
//! work uses iTerm2 in macOS). Moreover, bold and italics fonts are also supported in text mode.
//!
//! In graphics mode,
//! 256 unicode chars mark the index of a symbol in a texture block
//! unicode: 0xE000 ~ 0xE0FF (Private Use Area)
//! maps to a 3 byte UTF8: 11101110 100000xx 10xxxxxx
//! an 8-digits index gets from the UTF8 code is used to mark the offset in its block
//!
//! Using Private Use Area avoids conflicts with standard Unicode characters,
//! allowing TUI mode to display mathematical symbols and other special characters.
//!
//! # Block System
//!
//! The tex field indicates the texture block index (0-255) in the 4096x4096 unified texture:
//! - **Block 0-159**: Sprite region (160 blocks, 256×256px each, 16×16 chars per block)
//! - **Block 160-169**: TUI region (10 blocks, 256×512px each, 16×16 chars per block)
//! - **Block 170-175**: Emoji region (6 blocks, 256×512px each, 8×16 emojis per block)
//! - **Block 176-239**: CJK region (64 blocks, 256×256px each, 8×8 chars per block)
//! - **Block 240-255**: Reserved for future use
//!
//! See `render::symbol_map` module for detailed block layout and symbol mapping.
//!
//! # Example
//! ```ignore
//! // Set a character using block 0 (Sprite region)
//! my_buffer.set_str_tex(0, 0, cellsym(0), Style::default().fg(Color::Red), 0);
//!
//! // For common ASCII characters, use the default block (automatically mapped)
//! my_buffer.set_str(0, 0, "Hello world.", Style::default().fg(Color::Red));
//! ```
//!
//! Note: When using symbol_map lookups (Emoji, TUI, CJK), the block index is automatically
//! determined by `get_cell_info()` based on the character's region.
//!
#[allow(unused_imports)]
use crate::{
    render::cell::{cellsym, is_prerendered_emoji, Cell},
    render::style::{Color, Style},
    util::{Rect, PointU16},
    util::shape::{circle, line, prepare_line},
};
use bitflags::bitflags;
use log::info;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Common line-drawing symbols (in text mode)
pub const SYMBOL_LINE: [&str; 37] = [
    "│", "║", "┃", "─", "═", "━", "┐", "╮", "╗", "┓", "┌", "╭", "╔", "┏", "┘", "╯", "╝", "┛", "└",
    "╰", "╚", "┗", "┤", "╣", "┫", "├", "╠", "┣", "┬", "╦", "┳", "┴", "╩", "┻", "┼", "╬", "╋",
];

// border's bitflags
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Borders: u32 {
        const NONE   = 0b0000_0001;
        const TOP    = 0b0000_0010;
        const RIGHT  = 0b0000_0100;
        const BOTTOM = 0b0000_1000;
        const LEFT   = 0b0001_0000;
        const ALL    = Self::TOP.bits() | Self::RIGHT.bits() | Self::BOTTOM.bits() | Self::LEFT.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderType {
    Plain,
    Rounded,
    Double,
    Thick,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Buffer {
    pub area: Rect,
    pub content: Vec<Cell>,
}

impl Buffer {
    pub fn empty(area: Rect) -> Buffer {
        let cell: Cell = Default::default();
        Buffer::filled(area, &cell)
    }

    pub fn filled(area: Rect, cell: &Cell) -> Buffer {
        let size = area.area() as usize;
        let mut content = Vec::with_capacity(size);
        for _ in 0..size {
            content.push(cell.clone());
        }
        Buffer { area, content }
    }

    pub fn with_lines<S>(lines: Vec<S>) -> Buffer
    where
        S: AsRef<str>,
    {
        let height = lines.len() as u16;
        let width = lines
            .iter()
            .map(|i| i.as_ref().width() as u16)
            .max()
            .unwrap_or_default();
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width,
            height,
        });
        for (y, line) in lines.iter().enumerate() {
            buffer.set_string(0, y as u16, line, Style::default());
        }
        buffer
    }

    pub fn content(&self) -> &[Cell] {
        &self.content
    }

    /// Convert buffer to RGBA image data for OpenGL shader texture.
    ///
    /// Each cell is encoded as 4 bytes:
    /// - Byte 0: symbol_index (0-255, index within block)
    /// - Byte 1: block_index (0-255, texture block)
    /// - Byte 2: foreground color
    /// - Byte 3: background color
    ///
    /// Used by graphics adapters to pass buffer data to GPU shaders.
    pub fn get_rgba_image(&self) -> Vec<u8> {
        let mut dat = vec![];
        for c in &self.content {
            // Get (symbol_index, block_index, fg, bg, modifier)
            let ci = c.get_cell_info();
            dat.push(ci.0); // symbol_index
            dat.push(ci.1); // block_index
            dat.push(u8::from(ci.2)); // fg
            dat.push(u8::from(ci.3)); // bg
        }
        dat
    }

    /// Convert RGBA image data back to buffer (from OpenGL shader output).
    ///
    /// Each cell is decoded from 4 bytes:
    /// - Byte 0: symbol_index → cellsym(index)
    /// - Byte 1: block_index → set_texture(block)
    /// - Byte 2: foreground color
    /// - Byte 3: background color
    pub fn set_rgba_image(&mut self, dat: &[u8], w: u16, h: u16) {
        let mut idx = 0;
        for i in 0..h {
            for j in 0..w {
                self.content[(i * w + j) as usize]
                    .set_symbol(&cellsym(dat[idx]))
                    .set_texture(dat[idx + 1]) // block_index
                    .set_fg(Color::Indexed(dat[idx + 2]))
                    .set_bg(Color::Indexed(dat[idx + 3]));
                idx += 4;
            }
        }
    }

    pub fn area(&self) -> &Rect {
        &self.area
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        let i = self.index_of(x, y);
        &self.content[i]
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        let i = self.index_of(x, y);
        &mut self.content[i]
    }

    //global offset
    pub fn index_of(&self, x: u16, y: u16) -> usize {
        debug_assert!(
            x >= self.area.left()
                && x < self.area.right()
                && y >= self.area.top()
                && y < self.area.bottom(),
            "Trying to access position outside the buffer: x={}, y={}, area={:?}",
            x,
            y,
            self.area
        );
        ((y - self.area.y) * self.area.width + (x - self.area.x)) as usize
    }

    pub fn pos_of(&self, i: usize) -> (u16, u16) {
        debug_assert!(
            i < self.content.len(),
            "Trying to get the coords of a cell outside the buffer: i={} len={}",
            i,
            self.content.len()
        );
        (
            self.area.x + i as u16 % self.area.width,
            self.area.y + i as u16 / self.area.width,
        )
    }

    //relative pos in game sprite, easier to set content
    pub fn dstr<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        self.set_str(0, 0, string, Style::default());
    }

    /// Set string at relative position with specific texture block.
    ///
    /// Coordinates are relative to buffer's area (easier for sprite content).
    ///
    /// # Arguments
    ///
    /// * `x`, `y` - Relative coordinates (offset by self.area.x/y)
    /// * `string` - Text to render
    /// * `style` - Text style (colors, modifiers)
    /// * `tex` - Texture block index (0-255), see module docs for block ranges
    pub fn set_str_tex<S>(&mut self, x: u16, y: u16, string: S, style: Style, tex: u8)
    where
        S: AsRef<str>,
    {
        self.set_stringn(
            x + self.area.x,
            y + self.area.y,
            string,
            usize::MAX,
            style,
            tex,
        );
    }

    //relative pos in game sprite, easier to set content
    pub fn set_str<S>(&mut self, x: u16, y: u16, string: S, style: Style)
    where
        S: AsRef<str>,
    {
        self.set_stringn(
            x + self.area.x,
            y + self.area.y,
            string,
            usize::MAX,
            style,
            0,
        );
    }

    /// Set string at absolute position with specific texture block.
    ///
    /// Coordinates are absolute (global screen coordinates).
    ///
    /// # Arguments
    ///
    /// * `x`, `y` - Absolute coordinates (no offset)
    /// * `string` - Text to render
    /// * `style` - Text style (colors, modifiers)
    /// * `tex` - Texture block index (0-255), see module docs for block ranges
    pub fn set_string_tex<S>(&mut self, x: u16, y: u16, string: S, style: Style, tex: u8)
    where
        S: AsRef<str>,
    {
        self.set_stringn(x, y, string, usize::MAX, style, tex);
    }

    //absolute pos
    pub fn set_string<S>(&mut self, x: u16, y: u16, string: S, style: Style)
    where
        S: AsRef<str>,
    {
        self.set_stringn(x, y, string, usize::MAX, style, 0);
    }

    /// Core method to set string with width limit and block index.
    ///
    /// Used internally by `set_str`, `set_string`, `set_str_tex`, and `set_string_tex`.
    ///
    /// # Arguments
    ///
    /// * `x`, `y` - Absolute coordinates
    /// * `string` - Text to render
    /// * `width` - Maximum width in characters (usize::MAX for no limit)
    /// * `style` - Text style (colors, modifiers)
    /// * `tex` - Texture block index (0-255):
    ///   - For normal text: typically 0 (Sprite block 0)
    ///   - For special symbols: use appropriate block index
    ///   - For Emoji/TUI/CJK: block is auto-determined by `get_cell_info()`
    ///
    /// # Returns
    ///
    /// Final (x, y) position after rendering the string.
    ///
    /// # Note
    ///
    /// Coordinates are converted to buffer-local indices via `index_of(x, y)`.
    pub fn set_stringn<S>(
        &mut self,
        x: u16,
        y: u16,
        string: S,
        width: usize,
        style: Style,
        tex: u8,
    ) -> (u16, u16)
    where
        S: AsRef<str>,
    {
        let mut index = self.index_of(x, y);
        let mut x_offset = x as usize;
        let graphemes = UnicodeSegmentation::graphemes(string.as_ref(), true);
        let max_offset = min(self.area.right() as usize, width.saturating_add(x as usize));
        for s in graphemes {
            let width = s.width();
            if width == 0 {
                continue;
            }
            // `x_offset + width > max_offset` could be integer overflow on 32-bit machines if we
            // change dimenstions to usize or u32 and someone resizes the terminal to 1x2^32.
            if width > max_offset.saturating_sub(x_offset) {
                break;
            }

            // Handle Emoji (which might be single width in unicode-width but we want double width for rendering)
            // OR handle actual double width characters.
            // Our Emoji are pre-rendered as 16x16 (2x1 grid cells).
            // We check if it's a pre-rendered Emoji.
            let is_emoji = is_prerendered_emoji(s);
            
            self.content[index].set_symbol(s);
            self.content[index].set_style(style);
            self.content[index].set_texture(tex);

            // If it's an Emoji, it occupies 2 cells visually (16px width).
            // Even if `unicode-width` says it's width 1 (some emojis are), we force it to take 2 cells
            // if it is in our pre-rendered map.
            // However, `unicode-width` usually reports 2 for emojis.
            // The critical part is clearing the *next* cell so it doesn't overlap.
            
            // Reset following cells if multi-width (they would be hidden by the grapheme),
            // For Emoji, we ensure it clears at least one extra cell if it's width 1 but we treat as 2?
            // Actually, let's stick to `width` from unicode-width for now, assuming it's correct (usually 2 for Emoji).
            // BUT, if `is_emoji` is true, we might want to enforce width=2 behavior for our grid.
            
            let effective_width = if is_emoji { 2 } else { width };

            for i in index + 1..index + effective_width {
                if i < self.content.len() {
                    self.content[i].reset();
                }
            }
            
            index += effective_width;
            x_offset += effective_width;
        }
        (x_offset as u16, y)
    }

    pub fn set_style(&mut self, area: Rect, style: Style) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                self.get_mut(x, y).set_style(style);
            }
        }
    }

    pub fn resize(&mut self, area: Rect) {
        let length = area.area() as usize;
        if self.content.len() > length {
            self.content.truncate(length);
        } else {
            self.content.resize(length, Default::default());
        }
        self.area = area;
    }

    pub fn reset(&mut self) {
        for c in &mut self.content {
            c.reset();
        }
    }

    /// Clear a specific rectangular area in the buffer
    /// More efficient than calling reset() on individual cells
    pub fn clear_area(&mut self, area: Rect) {
        let x_start = area.x.max(self.area.x);
        let y_start = area.y.max(self.area.y);
        let x_end = (area.x + area.width).min(self.area.x + self.area.width);
        let y_end = (area.y + area.height).min(self.area.y + self.area.height);

        for y in y_start..y_end {
            for x in x_start..x_end {
                let idx = self.index_of(x, y);
                self.content[idx].reset();
            }
        }
    }

    pub fn set_fg(&mut self, color: Color) {
        for c in &mut self.content {
            c.set_fg(color);
        }
    }

    // ========== Content-drawing convenience methods ==========

    /// Set string content at (x,y) with fg/bg color.
    /// Coordinates are relative to buffer's area.
    pub fn set_color_str<S>(&mut self, x: u16, y: u16, string: S, fg: Color, bg: Color)
    where
        S: AsRef<str>,
    {
        self.set_str(x, y, string, Style::default().fg(fg).bg(bg));
    }

    /// Set string content at (0,0) with default style.
    pub fn set_default_str<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        self.set_str(0, 0, string, Style::default());
    }

    /// Set graphic mode symbol (texture:texture_id, index:sym) at (x,y) with fg color.
    pub fn set_graph_sym(&mut self, x: u16, y: u16, texture_id: u8, sym: u8, fg: Color) {
        self.set_str_tex(
            x,
            y,
            cellsym(sym),
            Style::default().fg(fg).bg(Color::Reset),
            texture_id,
        );
    }

    // ========== Shape drawing methods ==========

    pub fn draw_circle(
        &mut self,
        x0: u16,
        y0: u16,
        radius: u16,
        sym: &str,
        fg_color: u8,
        bg_color: u8,
    ) {
        for p in circle(x0, y0, radius) {
            if (p.0 as u16) < self.area.width && (p.1 as u16) < self.area.height {
                self.set_str(
                    p.0 as u16,
                    p.1 as u16,
                    sym,
                    Style::default()
                        .fg(Color::Indexed(fg_color))
                        .bg(Color::Indexed(bg_color)),
                );
            }
        }
    }

    pub fn draw_line(
        &mut self,
        p0: PointU16,
        p1: PointU16,
        sym: Option<Vec<Option<u8>>>,
        fg_color: u8,
        bg_color: u8,
    ) {
        let (x0, y0, x1, y1) = prepare_line(p0.x, p0.y, p1.x, p1.y);
        // start, end, v, h, s, bs...
        let mut syms: Vec<Option<u8>> = vec![None, None, Some(119), Some(116), Some(77), Some(78)];
        if let Some(s) = sym {
            syms = s;
        }
        for p in line(x0, y0, x1, y1) {
            let x = p.0 as u16;
            let y = p.1 as u16;
            let sym = syms[p.2 as usize];
            if let Some(s) = sym {
                if x < self.area.width && y < self.area.height {
                    self.set_str_tex(
                        x,
                        y,
                        cellsym(s),
                        Style::default()
                            .fg(Color::Indexed(fg_color))
                            .bg(Color::Reset),
                        bg_color,
                    );
                }
            }
        }
    }

    // ========== Border drawing ==========

    pub fn set_border(&mut self, borders: Borders, border_type: BorderType, style: Style) {
        let lineidx: [usize; 11] = match border_type {
            BorderType::Plain => [0, 3, 6, 10, 14, 18, 22, 25, 28, 31, 34],
            BorderType::Rounded => [0, 3, 7, 11, 15, 19, 22, 25, 28, 31, 34],
            BorderType::Double => [1, 4, 8, 12, 16, 20, 23, 26, 29, 33, 35],
            BorderType::Thick => [2, 5, 9, 13, 17, 21, 24, 27, 30, 34, 36],
        };
        if borders.intersects(Borders::LEFT) {
            for y in 0..self.area.height {
                self.set_str_tex(0, y, SYMBOL_LINE[lineidx[0]], style, 1);
            }
        }
        if borders.intersects(Borders::TOP) {
            for x in 0..self.area.width {
                self.set_str_tex(x, 0, SYMBOL_LINE[lineidx[1]], style, 1);
            }
        }
        if borders.intersects(Borders::RIGHT) {
            let x = self.area.width - 1;
            for y in 0..self.area.height {
                self.set_str_tex(x, y, SYMBOL_LINE[lineidx[0]], style, 1);
            }
        }
        if borders.intersects(Borders::BOTTOM) {
            let y = self.area.height - 1;
            for x in 0..self.area.width {
                self.set_str_tex(x, y, SYMBOL_LINE[lineidx[1]], style, 1);
            }
        }
        if borders.contains(Borders::RIGHT | Borders::BOTTOM) {
            self.set_str_tex(
                self.area.width - 1,
                self.area.height - 1,
                SYMBOL_LINE[lineidx[4]],
                style,
                1,
            );
        }
        if borders.contains(Borders::RIGHT | Borders::TOP) {
            self.set_str_tex(
                self.area.width - 1,
                0,
                SYMBOL_LINE[lineidx[2]],
                style,
                1,
            );
        }
        if borders.contains(Borders::LEFT | Borders::BOTTOM) {
            self.set_str_tex(
                0,
                self.area.height - 1,
                SYMBOL_LINE[lineidx[5]],
                style,
                1,
            );
        }
        if borders.contains(Borders::LEFT | Borders::TOP) {
            self.set_str_tex(0, 0, SYMBOL_LINE[lineidx[3]], style, 1);
        }
    }

    #[allow(unused_variables)]
    pub fn copy_cell(&mut self, pos_self: usize, other: &Buffer, alpha: u8, pos_other: usize) {
        // self.content[pos_self].symbol = other.content[pos_other].symbol.clone();
        // self.content[pos_self].bg = other.content[pos_other].bg;
        self.content[pos_self] = other.content[pos_other].clone();
        #[cfg(graphics_mode)]
        {
            let fc = other.content[pos_other].fg.get_rgba();
            if other.content[pos_other].bg != Color::Reset {
                let bc = other.content[pos_other].bg.get_rgba();
                self.content[pos_self].bg = Color::Rgba(bc.0, bc.1, bc.2, alpha);
            }
            self.content[pos_self].fg = Color::Rgba(fc.0, fc.1, fc.2, alpha);
        }
    }

    pub fn blit(
        &mut self,
        dstx: u16,
        dsty: u16,
        other: &Buffer,
        other_part: Rect,
        alpha: u8,
    ) -> Result<(u16, u16), String> {
        //make sure dstx and dsty are correct
        if dstx >= self.area.width || dsty >= self.area.height {
            return Err(String::from("buffer blit:dstx, dsty too large"));
        }
        //make sure other_part is correct
        let oa = Rect::new(0, 0, other.area.width, other.area.height);
        if !other_part.intersects(oa) {
            info!(
                "buffer blit:error oa = {:?} other_part = {:?}",
                oa, other_part
            );
            return Err(String::from("buffer blit:error other_part"));
        }
        let bw = min(other_part.width, self.area.width - dstx);
        let bh = min(other_part.height, self.area.height - dsty);
        // info!("blit....(bw={} bh={})", bw, bh);

        for i in 0..bh {
            for j in 0..bw {
                let pos_self = (self.area.width * (dsty + i) + dstx + j) as usize;
                let pos_other =
                    // (other.area.width * other_part.y + other_part.x + i * bw + j) as usize;
                    (other.area.width * other_part.y + other_part.x + i * other.area.width + j) as usize;
                // info!("blit...ps{:?} po{:?}", pos_self, pos_other);
                self.copy_cell(pos_self, other, alpha, pos_other);
            }
        }

        Ok((bw, bh))
    }

    pub fn merge(&mut self, other: &Buffer, alpha: u8, fast: bool) {
        let area = self.area.union(other.area);
        let cell: Cell = Default::default();
        self.content.resize(area.area() as usize, cell.clone());
        if !fast {
            let size = self.area.area() as usize;
            for i in (0..size).rev() {
                let (x, y) = self.pos_of(i);
                // New index in content
                let k = ((y - area.y) * area.width + x - area.x) as usize;
                if i != k {
                    self.content[k] = self.content[i].clone();
                    self.content[i] = cell.clone();
                }
            }
        }
        let size = other.area.area() as usize;
        for i in 0..size {
            let (x, y) = other.pos_of(i);
            let k = ((y - area.y) * area.width + x - area.x) as usize;
            // add transparent support...
            if !other.content[i].is_blank() {
                self.copy_cell(k, other, alpha, i);
            }
        }
        self.area = area;
    }

    /// Builds a minimal sequence of coordinates and Cells necessary to update the UI from
    /// self to other.
    pub fn diff<'a>(&self, other: &'a Buffer) -> Vec<(u16, u16, &'a Cell)> {
        let previous_buffer = &self.content;
        let next_buffer = &other.content;
        let width = self.area.width;

        let mut updates: Vec<(u16, u16, &Cell)> = vec![];
        // Cells invalidated by drawing/replacing preceeding multi-width characters:
        let mut invalidated: usize = 0;
        // Cells from the current buffer to skip due to preceeding multi-width characters taking their
        // place (the skipped cells should be blank anyway):
        let mut to_skip: usize = 0;
        for (i, (current, previous)) in next_buffer.iter().zip(previous_buffer.iter()).enumerate() {
            if (current != previous || invalidated > 0) && to_skip == 0 {
                let x = i as u16 % width;
                let y = i as u16 / width;
                updates.push((x, y, &next_buffer[i]));
            }

            to_skip = current.symbol.width().saturating_sub(1);

            let affected_width = std::cmp::max(current.symbol.width(), previous.symbol.width());
            invalidated = std::cmp::max(affected_width, invalidated).saturating_sub(1);
        }
        updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // fn cell(s: &str) -> Cell {
    //     let mut cell = Cell::default();
    //     cell.set_symbol(s);
    //     cell
    // }

    #[test]
    fn it_translates_to_and_from_coordinates() {
        let rect = Rect::new(200, 100, 50, 80);
        let buf = Buffer::empty(rect);

        // First cell is at the upper left corner.
        assert_eq!(buf.pos_of(0), (200, 100));
        assert_eq!(buf.index_of(200, 100), 0);

        // Last cell is in the lower right.
        assert_eq!(buf.pos_of(buf.content.len() - 1), (249, 179));
        assert_eq!(buf.index_of(249, 179), buf.content.len() - 1);
    }
}
