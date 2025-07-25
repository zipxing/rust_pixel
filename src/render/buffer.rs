// RustPixel
// copyright zipxing@hotmail.com 2022~2024

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
//! 256 unicode chars mark the index of a symbol in a SDL texture
//! unicode: 0x2200 ~ 0x22FF
//! maps to a 3 byte UTF8: 11100010 100010xx 10xxxxxx
//! an 8-digits index gets from the UTF8 code is used to mark the offset in its texture
//!
//! tex field is used to indicate texture
//! 0: assets/c64l.png small case c64 char
//! 1: assets/c64u.png capital case c64 char
//! 2: assets/c64e1.png custom extension 1
//! 3: assets/c64e2.png custom extension 2
//! each texture is an image of 16 row * 16 row = 256 chars
//! # Example
//! ```
//! my_buffer.set_str_tex(0, 0, sdlsym(0), Style::default().fg(Color::Red), 1)
//! ```
//! sets pos(0,0) in the buffer to the 1st char of texture1(assets/c64u.png)
//!
//! For some common chars, you can also search the char in SDL_SYM_MAP to get the offset in assets/c64l.png
//! instead of using unicode chars
//! Some common chars a-Z and tabs are preset in SDL_SYM_MAP,
//! for easier set of latin letters using set_str in SDL mode
//! # Example
//! ```
//! my_buffer.set_str_tex(0, 0, "Hello world.",
//!     Style::default().fg(Color::Red), 0)
//! ```
//! Warning! tex here must be set to 0, because the offset in SDL_SYM_MAP is preset based on
//! texture0(assets/c64l.png). May have display issues if set to another texture.
//!
#[allow(unused_imports)]
use crate::{
    render::cell::{cellsym, Cell},
    render::style::{Color, Style},
    util::Rect,
};
use log::info;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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

    /// convert buffer to a rgba image buffer
    /// use for opengl shader texture
    pub fn get_rgba_image(&self) -> Vec<u8> {
        let mut dat = vec![];
        for c in &self.content {
            // sym tex fg bg
            let ci = c.get_cell_info();
            dat.push(ci.0);
            dat.push(ci.1);
            dat.push(u8::from(ci.2));
            dat.push(u8::from(ci.3));
        }
        dat
    }

    /// convert rgba image to buffer
    /// use for opengl shader output
    pub fn set_rgba_image(&mut self, dat: &[u8], w: u16, h: u16) {
        let mut idx = 0;
        for i in 0..h {
            for j in 0..w {
                self.content[(i * w + j) as usize]
                    .set_symbol(cellsym(dat[idx]))
                    .set_texture(dat[idx + 1])
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

    //relative pos in game sprite, easier to set content
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

    //absolute pos
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

            self.content[index].set_symbol(s);
            self.content[index].set_style(style);
            self.content[index].set_texture(tex);

            // Reset following cells if multi-width (they would be hidden by the grapheme),
            for i in index + 1..index + width {
                self.content[i].reset();
            }
            index += width;
            x_offset += width;
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

    pub fn set_fg(&mut self, color: Color) {
        for c in &mut self.content {
            c.set_fg(color);
        }
    }

    #[allow(unused_variables)]
    pub fn copy_cell(&mut self, pos_self: usize, other: &Buffer, alpha: u8, pos_other: usize) {
        // self.content[pos_self].symbol = other.content[pos_other].symbol.clone();
        // self.content[pos_self].bg = other.content[pos_other].bg;
        self.content[pos_self] = other.content[pos_other].clone();
        #[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
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
