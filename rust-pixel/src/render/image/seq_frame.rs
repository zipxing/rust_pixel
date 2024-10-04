// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements load of .ssf seq frame file
//! 
//! $ cat assets/sdq/1.ssf
//!
//! head of the file describes the width, height, framerate, texture and offset of each frame
//! file content is compressed frame data

use crate::{
    asset::{Asset, AssetBase, AssetState},
    render::buffer::Buffer,
    render::cell::cellsym,
    render::image::esc::escstr_to_buffer,
    render::style::{Color, Style},
    util::Rect,
};
use flate2::read::GzDecoder;
// use log::info;
use regex::Regex;
use std::io::{BufRead, BufReader, Read};

pub struct SeqFrameAsset {
    pub base: AssetBase,
    pub width: u16,
    pub height: u16,
    pub texture_id: u16,
    pub frame_len: Vec<u32>,
    pub frame_offset: Vec<u32>,
    pub frame_data: Vec<u8>,
}

impl Asset for SeqFrameAsset {
    fn new(base: AssetBase) -> Self {
        Self {
            base,
            width: 0,
            height: 0,
            texture_id: 0,
            frame_len: vec![],
            frame_offset: vec![],
            frame_data: vec![],
        }
    }

    fn get_base(&mut self) -> &mut AssetBase {
        &mut self.base
    }

    /// texture_id == 257 表示ESC格式的序列帧,
    /// 每行数据为esc序列(参考fn escstr_to_buffer)
    ///
    /// texture_id == 257 means seq frame in ESC format,
    /// each row represents an ESC seq(refer to fn escstr_to_buffer)
    ///
    /// texture_id == 256 表示utf8文本模式的序列帧,
    /// 每个cell的数据为不定长: fg bg utf8_bytes
    ///
    /// texture_id == 256 means seq frame in UTF8 text format,
    /// each cell may have different lengths: fg bg utf8_bytes
    ///
    /// texture_id == 255 表示Sdl格式的帧数据,每个cell的纹理可以不同，
    /// cell的数据长度为3字节: fg bg(bg即cell_texture_id) cellsym
    /// 当前系统中ssf文件基本上是这种格式
    ///
    /// texture_id == 255 means seq frame in SDL format, each cell may have
    /// different textures, the length of each cell data is 3 bytes
    /// fg bg(bg i.e. cell_texture_id) cellsym
    /// ssf files in modern OSs are usually found in this format
    ///
    /// texture_id < 255 表示Sdl格式的帧数据，每个cell的纹理都是texture_id,
    /// cell的数据长度为2字节: fg cellsym
    /// texture_id < 255 means SDL frame data，each cell's texture is texture_id,
    /// the length of each cell data is 2 bytes : fg cellsym
    fn parse(&mut self) {
        if self.get_state() != AssetState::Parsing {
            return;
        }
        self.frame_len = vec![];
        self.frame_offset = vec![];
        self.frame_data = vec![];
        let mut reader = BufReader::new(&self.base.raw_data[..]);

        let re = Regex::new(r"width=(\d+),height=(\d+),texture=(\d+),frame_count=(\d+)").unwrap();
        let rel = Regex::new(r"(\d+),(.*?)").unwrap();
        let mut file_header = String::new();
        let _ = reader.read_line(&mut file_header);
        for cap in re.captures_iter(&file_header) {
            self.width = cap[1].parse::<u16>().unwrap();
            self.height = cap[2].parse::<u16>().unwrap();
            self.texture_id = cap[3].parse::<u16>().unwrap();
            self.base.frame_count = cap[4].parse::<u16>().unwrap() as usize;
        }
        let mut len_header = String::new();
        let _ = reader.read_line(&mut len_header);
        let mut offset = 0u32;
        for cap in rel.captures_iter(&len_header) {
            let flen = cap[1].parse::<u32>().unwrap();
            self.frame_len.push(flen);
            self.frame_offset.push(offset);
            offset += flen;
        }
        let _ = reader.read_to_end(&mut self.frame_data);
        self.base.parsed_buffers.clear();
        for frame_idx in 0..self.base.frame_count {
            let size = Rect::new(0, 0, self.width, self.height);
            let mut sp = Buffer::empty(size);
            let start = self.frame_offset[frame_idx] as usize;
            let flen = self.frame_len[frame_idx] as usize;
            let mut decoder = GzDecoder::new(&self.frame_data[start..start + flen]);

            if self.texture_id == 257 {
                let reader = BufReader::new(decoder);
                for (row, line) in reader.lines().enumerate() {
                    let l = line.unwrap();
                    escstr_to_buffer(&l, &mut sp, row as u16, 0, 0);
                }
            } else if self.texture_id == 256 {
                let mut decompressed_data = Vec::new();
                decoder.read_to_end(&mut decompressed_data).unwrap();
                let mut bpos = 0usize;
                let mut i = 0u16;
                loop {
                    let fgc = decompressed_data[bpos];
                    bpos += 1;
                    let bgc = decompressed_data[bpos];
                    bpos += 1;
                    let mut utf8_bytes: Vec<u8> = vec![];
                    let first = decompressed_data[bpos];
                    bpos += 1;
                    utf8_bytes.push(first);
                    let mut blen = 0;
                    if first >> 7 == 0 {
                        blen = 0;
                    } else {
                        if first >> 5 == 0b0000_0110 {
                            blen = 1;
                        }
                        if first >> 4 == 0b0000_1110 {
                            blen = 2;
                        }
                        if first >> 3 == 0b0001_1110 {
                            blen = 3;
                        }
                    }
                    for _i in 0..blen {
                        utf8_bytes.push(decompressed_data[bpos]);
                        bpos += 1;
                    }
                    sp.set_str(
                        i % self.width,
                        i / self.width,
                        std::str::from_utf8(&utf8_bytes).unwrap(),
                        Style::default()
                            .fg(Color::Indexed(fgc))
                            .bg(Color::Indexed(bgc)),
                    );
                    i += 1;
                    if bpos == decompressed_data.len() {
                        break;
                    }
                }
            } else {
                let mut decompressed_data = Vec::new();
                decoder.read_to_end(&mut decompressed_data).unwrap();
                let cell_len: usize = if self.texture_id == 255 { 3 } else { 2 };
                for i in 0..decompressed_data.len() as u16 / cell_len as u16 {
                    let bgc: u8 = if self.texture_id == 255 {
                        decompressed_data[i as usize * cell_len + 2]
                    } else {
                        self.texture_id as u8
                    };
                    sp.set_str_tex(
                        i % self.width,
                        i / self.width,
                        cellsym(decompressed_data[i as usize * cell_len]),
                        Style::default()
                            .fg(Color::Indexed(decompressed_data[i as usize * cell_len + 1]))
                            .bg(Color::Reset),
                        bgc
                    );
                }
            }
            self.base.parsed_buffers.push(sp);
        }
    }

    fn save(&mut self, _content: &Buffer) {}
}
