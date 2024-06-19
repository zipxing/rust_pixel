// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! Implements load/save of image files in esc format
//!
//! esc file stores the ascii art images in terminal mode, saving esc terminal sequences
//! and UTF8 text. Run this CMD to check:
//! $ cat assets/tetris/back.txt

use crate::{
    asset::{Asset, AssetBase},
    render::image::find_vaild_area,
    render::buffer::Buffer,
    render::style::{Color, Style},
    util::Rect,
};
use log::info;
use regex::Regex;
use std::io::{BufRead, BufReader, Write};
use unicode_width::UnicodeWidthStr;

pub struct EscAsset {
    base: AssetBase,
}

impl Asset for EscAsset {
    fn new(ab: AssetBase) -> Self {
        Self { base: ab }
    }


    fn get_base(&mut self) -> &mut AssetBase {
        &mut self.base
    }

    fn parse(&mut self) {
        self.base.parsed_buffers.clear();
        let size = Rect::new(0, 0, 500, 300);
        let mut sp = Buffer::empty(size);
        let reader = BufReader::new(&self.base.raw_data[..]);
        let mut row = 0;
        let mut max_width: u16 = 0;
        for line in reader.lines() {
            let l = line.unwrap();
            let lw = escstr_to_buffer(&l, &mut sp, row, 0, 0);
            if lw > max_width {
                max_width = lw;
            }
            row += 1;
        }
        let nsize = Rect::new(0, 0, max_width, row);
        let mut nsp = Buffer::empty(nsize);
        let _ = nsp.blit(0, 0, &sp, nsize, 255);
        self.base.parsed_buffers.push(nsp);
    }

    fn save(&mut self, content: &Buffer) {
        self.base.raw_data.clear();
        let mut ptr = std::io::Cursor::new(&mut self.base.raw_data);
        let width = content.area.width;
        //let rowcnt = self.content.area.height;
        let (x1, x2, y1, y2) = find_vaild_area(content);
        for row in y1..y2 + 1 {
            let line =
                &content.content[(row * width + x1) as usize..(row * width + x2 + 1) as usize];
            let mut fg = Color::Reset;
            let mut bg = Color::Reset;
            let mut span = String::new();
            let mut skip = 0i8;
            for (_i, cell) in line.iter().enumerate() {
                info!(
                    "save_esc symbol={} symwidth={}",
                    cell.symbol,
                    cell.symbol.width()
                );
                //Skip processing the space after monospace chinese font
                if skip > 0 {
                    info!("skip, skip={}", skip);
                    skip -= 1;
                    continue;
                }
                let sw = cell.symbol.width();
                if sw > 1 {
                    skip = sw as i8;
                    skip -= 1;
                    info!("set skip, skip={}", skip);
                }
                if cell.fg != fg || cell.bg != bg {
                    if span.len() != 0 {
                        if fg == Color::Reset && bg == Color::Reset {
                            let _ = ptr.write_all(span.as_bytes());
                        } else {
                            let ss = format!(
                                "\x1b[38;5;{}m\x1b[48;5;{}m{}\x1b[0m",
                                u8::from(fg),
                                u8::from(bg),
                                span
                            );
                            let _ = ptr.write_all(ss.as_bytes());
                        }
                        span.clear();
                    }
                    fg = cell.fg;
                    bg = cell.bg;
                    span.push_str(&cell.symbol);
                } else {
                    span.push_str(&cell.symbol);
                }
            }
            if span.len() != 0 {
                if fg == Color::Reset && bg == Color::Reset {
                    let _ = ptr.write_all(span.as_bytes());
                } else {
                    let ss = format!(
                        "\x1b[38;5;{}m\x1b[48;5;{}m{}\x1b[0m",
                        u8::from(fg),
                        u8::from(bg),
                        span
                    );
                    let _ = ptr.write_all(ss.as_bytes());
                }
                span.clear();
            }
            let _ = ptr.write_all("\n".as_bytes());
        }
    }
}

pub fn escstr_to_buffer(l: &String, content: &mut Buffer, row: u16, off_x: u16, off_y: u16) -> u16 {
    let mut pos = 0;
    let mut cell_pos = 0;
    let mut lpos = 0;
    let mut lcell_pos = 0;
    let re = Regex::new(r"\x1b\[38;5;(\d+)m\x1b\[48;5;(\d+)m(.*?)\x1b\[0m").unwrap();
    for cap in re.captures_iter(l) {
        let cr = cap.get(0).unwrap();
        //info!("load_esc set1 x={} str={}", cell_pos + off_x, &l[pos..cr.start()]);
        content.set_str(
            cell_pos + off_x,
            row + off_y,
            &l[pos..cr.start()],
            Style::default(),
        );
        //注意要使用unicode的长度，不能直接使用byte长度
        //例如♥的正确长度是1，而byte长度是3
        //let graphemes = UnicodeSegmentation::graphemes(&l[pos..cr.start()], true)
        //    .collect::<Vec<&str>>();
        //采用宽字符个数也不行
        //cell_pos += graphemes.len() as u16;
        //采用width返回真正的字符宽度,

        //beware of the length of unicode，can not use the length of a byte
        //e.g. the correct length of ♥ is 1，while the length of byte is 3
        //let graphemes = UnicodeSegmentation::graphemes(&l[pos..cr.start()], true)
        //    .collect::<Vec<&str>>();
        //can not use the count of wide characters as well
        //cell_pos += graphemes.len() as u16;
        //use width to return the true char width
        cell_pos += l[pos..cr.start()].width() as u16;
        //info!("load_esc set2 x={} str={}", cell_pos + off_x, &cap[3]);
        content.set_str(
            cell_pos + off_x,
            row + off_y,
            &cap[3],
            Style::default()
                .fg(Color::Indexed(cap[1].parse::<u8>().unwrap()))
                .bg(Color::Indexed(cap[2].parse::<u8>().unwrap())),
        );
        //let graphemes = UnicodeSegmentation::graphemes(&cap[3], true)
        //    .collect::<Vec<&str>>();
        //cell_pos += graphemes.len() as u16;
        cell_pos += cap[3].width() as u16;
        pos = cr.end();
        lpos = pos;
        lcell_pos = cell_pos;
    }
    //info!("load_esc set3 x={} str={}", lcell_pos + off_x, &l[lpos..l.len()]);
    content.set_str(
        lcell_pos + off_x,
        row + off_y,
        &l[lpos..l.len()],
        Style::default(),
    );
    //info!("load_esc line width = {}", lcell_pos + l[lpos..l.len()].width() as u16);
    lcell_pos + l[lpos..l.len()].width() as u16
}

