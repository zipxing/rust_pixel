// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! 实现了加载和保存.pix文件的方法
//!
//! pix文件存储图形模式下的字符图，按行存储cell序列，
//! cell: 字符sym索引，前景色，背景色（图形模式下背景色用来标识纹理）
//!
//! Implements load/save of image files in pix format
//!
//! pix file stores the ascii art images in graphical mode, saving the cell sequence row by row
//! cell: char sym index, fore- and background colors (background color is used to mark texture in graphical mode)
//! $ cat assets/snake/back.pix

use crate::{
    asset::{Asset, AssetBase},
    render::buffer::Buffer,
    render::cell::cellsym,
    render::image::find_vaild_area,
    render::style::{Color, Style},
    util::Rect,
};
use log::info;
use regex::Regex;
use std::io::{BufRead, BufReader, Write};

pub struct PixAsset {
    base: AssetBase,
}

impl Asset for PixAsset {
    fn new(ab: AssetBase) -> Self {
        Self { base: ab }
    }

    fn get_base(&mut self) -> &mut AssetBase {
        &mut self.base
    }

    fn parse(&mut self) {
        self.base.parsed_buffers.clear();
        let size = Rect::new(0, 0, 0, 0);
        let mut sp = Buffer::empty(size);

        let reader = BufReader::new(&self.base.raw_data[..]);
        let re = Regex::new(r"width=(\d+),height=(\d+),texture=(\d+)").unwrap();
        let rel0 = Regex::new(r"(\d+),(\d+)(.*?)").unwrap();
        let rel1 = Regex::new(r"(\d+),(\d+),(\d+)(.*?)").unwrap();
        let mut width: u16;
        let mut height: u16;
        let mut texid: u8 = 0;
        let mut lineidx = 0;
        //info!("begin load_pix....");
        for line in reader.lines() {
            let l = line.unwrap();
            //info!("load_pix line={}", l);
            if lineidx == 0 {
                for cap in re.captures_iter(&l) {
                    width = cap[1].parse::<u16>().unwrap();
                    height = cap[2].parse::<u16>().unwrap();
                    texid = cap[3].parse::<u8>().unwrap();
                    info!("w..{} h..{} l..{}", width, height, texid);
                    let size = Rect::new(0, 0, width, height);
                    sp.resize(size);
                }
            } else {
                let mut col = 0;
                if texid < 255 {
                    for cap in rel0.captures_iter(&l) {
                        let idx = cap[1].parse::<u8>().unwrap();
                        let fgc = cap[2].parse::<u8>().unwrap();
                        sp.set_str(
                            col,
                            lineidx - 1,
                            cellsym(idx),
                            Style::default()
                                .fg(Color::Indexed(fgc))
                                .bg(Color::Indexed(texid)),
                        );
                        col += 1;
                    }
                } else {
                    for cap in rel1.captures_iter(&l) {
                        let idx = cap[1].parse::<u8>().unwrap();
                        let fgc = cap[2].parse::<u8>().unwrap();
                        let bgc = cap[3].parse::<u8>().unwrap();
                        sp.set_str(
                            col,
                            lineidx - 1,
                            cellsym(idx),
                            Style::default()
                                .fg(Color::Indexed(fgc))
                                .bg(Color::Indexed(bgc)),
                        );
                        col += 1;
                    }
                }
            }
            lineidx += 1;
        }
        self.base.parsed_buffers.push(sp);
    }

    fn save(&mut self, content: &Buffer) {
        self.base.raw_data.clear();
        let mut ptr = std::io::Cursor::new(&mut self.base.raw_data);
        let (x1, x2, y1, y2) = find_vaild_area(content);
        let width = content.area.width;
        let _ = writeln!(
            ptr,
            "width={},height={},texture={}",
            x2 - x1 + 1,
            y2 - y1 + 1,
            255
        );
        for row in y1..y2 + 1 {
            let line =
                &content.content[(row * width + x1) as usize..(row * width + x2 + 1) as usize];
            for (_i, cell) in line.iter().enumerate() {
                let (idx, _, _) = cell.get_cell_info();
                let _ = write!(ptr, "{},{},{} ", idx, u8::from(cell.fg), u8::from(cell.bg));
            }
            let _ = write!(ptr, "\n");
        }
    }
}
