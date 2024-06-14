// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! asset provides the resource manager.
//! It supports async load. It calls JavaScript methods to load resources asynchronously when runs in wasm mode.
//! https://www.reddit.com/r/rust/comments/8ymzwg/common_data_and_behavior/

#[cfg(not(target_arch = "wasm32"))]
use crate::util::get_abs_path;
use crate::{
    render::buffer::Buffer,
    render::image::{EscAsset, PixAsset, SeqFrameAsset},
    render::sprite::Sprite,
};
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
// use log::debug;

#[derive(PartialEq, Clone, Copy)]
pub enum AssetState {
    Loading,
    Parsing,
    Ready,
}

#[derive(PartialEq, Clone, Copy)]
pub enum AssetType {
    ImgPix,
    ImgEsc,
    ImgSsf,
}

pub struct AssetBase {
    // web url or file pathname...
    pub location: String,
    pub asset_type: AssetType,
    // raw data in resource file
    pub raw_data: Vec<u8>,
    // parse only once after get the raw data.
    // Each frame is buffered here for further use
    pub parsed_buffers: Vec<Buffer>,
    pub frame_count: usize,
    pub state: AssetState,
}

impl AssetBase {
    pub fn new(t: AssetType, loc: &str) -> Self {
        Self {
            location: loc.to_string(),
            asset_type: t,
            raw_data: vec![],
            parsed_buffers: vec![],
            frame_count: 1,
            state: AssetState::Loading,
        }
    }
}

pub trait Asset {
    fn new(ab: AssetBase) -> Self
    where
        Self: Sized;

    fn set_sprite(&mut self, sp: &mut Sprite, frame_idx: usize, off_x: u16, off_y: u16) {
        let bs = self.get_base();
        let _ = sp.content.blit(
            off_x,
            off_y,
            &bs.parsed_buffers[frame_idx % bs.frame_count],
            bs.parsed_buffers[frame_idx % bs.frame_count].area,
        );
    }

    fn get_base(&mut self) -> &mut AssetBase;

    fn set_data(&mut self, data: &[u8]) {
        let bs = self.get_base();
        bs.raw_data.clear();
        bs.raw_data.extend(data);
        bs.state = AssetState::Parsing;
    }

    fn set_state(&mut self, st: AssetState) {
        self.get_base().state = st;
    }

    fn get_state(&mut self) -> AssetState {
        self.get_base().state
    }

    fn parse(&mut self);

    fn save(&mut self, buf: &Buffer);
}

pub struct AssetManager {
    pub assets: Vec<Box<dyn Asset>>,
    pub assets_index: HashMap<String, usize>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            assets: vec![],
            assets_index: HashMap::new(),
        }
    }

    #[allow(unused_mut)]
    pub fn load(&mut self, t: AssetType, loc: &str) {
        match self.assets_index.get(loc) {
            Some(_) => {}
            None => {
                let mut ab = AssetBase::new(t, loc);
                #[cfg(target_arch = "wasm32")]
                {
                    js_load_asset(loc);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {}
                let mut ast: Box<dyn Asset> = match t {
                    AssetType::ImgPix => Box::new(PixAsset::new(ab)),
                    AssetType::ImgEsc => Box::new(EscAsset::new(ab)),
                    AssetType::ImgSsf => Box::new(SeqFrameAsset::new(ab)),
                };
                self.assets.push(ast);
                self.assets_index.insert(loc.to_string(), self.assets.len());
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let fpstr = get_abs_path(loc);
                    let fdata = std::fs::read(fpstr).expect("read file error");
                    self.set_data(loc, &fdata[..]);
                }
            }
        }
    }

    pub fn get(&mut self, loc: &str) -> Option<&mut Box<(dyn Asset)>> {
        match self.assets_index.get(loc) {
            Some(idx) => Some(&mut self.assets[*idx - 1]),
            None => None,
        }
    }

    // when in web mode, this method is called to get the resources ready after async load of js resources
    // when in other modes, this method is called after finishing reading files
    // refer to rust-pixel/web-templates/index.js
    pub fn set_data(&mut self, loc: &str, data: &[u8]) {
        match self.assets_index.get(loc) {
            Some(idx) => {
                self.assets[*idx - 1].set_data(data);
                self.assets[*idx - 1].set_state(AssetState::Parsing);
                self.assets[*idx - 1].parse();
                self.assets[*idx - 1].set_state(AssetState::Ready);
            }
            None => {}
        }
    }
}

// refer to rust-pixel/web-templates/index.js
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(raw_module = "/index.js")]
extern "C" {
    fn js_load_asset(url: &str);
}
