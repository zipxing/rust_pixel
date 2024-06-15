mod model;
mod render;

use crate::{model::GinRummyModel, render::GinRummyRender};
use rust_pixel::game::Game;

#[cfg(target_arch = "wasm32")]
use rust_pixel::render::adapter::web::{input_events_from_web, WebAdapter, WebCell};
use wasm_bindgen::prelude::*;

// wasm can not bind struct with generics or lifetime,
// so encapsulating it as a fixed type
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GinRummyGame {
    g: Game<GinRummyModel, GinRummyRender>,
}

pub fn init_game() -> GinRummyGame {
    let m = GinRummyModel::new();
    let r = GinRummyRender::new();
    let mut g = Game::new(m, r, "gin_rummy");
    g.init();
    GinRummyGame { g }
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GinRummyGame {
    pub fn new() -> Self {
        init_game()
    }

    pub fn tick(&mut self, dt: f32) {
        self.g.on_tick(dt);
    }

    pub fn key_event(&mut self, t: u8, e: web_sys::Event) {
        let abase = &self
            .g
            .context
            .adapter
            .as_any()
            .downcast_ref::<WebAdapter>()
            .unwrap()
            .base;
        if let Some(pe) = input_events_from_web(t, e, abase.ratio_x, abase.ratio_y) {
            self.g.context.input_events.push(pe);
        }
    }

    pub fn on_asset_loaded(&mut self, url: &str, data: &[u8]) {
        debug!("asset({:?}): {:?}!!!", url, data);
        self.g.context.asset_manager.set_data(url, data);
    }

    fn get_wb(&self) -> &Vec<WebCell> {
        &self
            .g
            .context
            .adapter
            .as_any()
            .downcast_ref::<WebAdapter>()
            .unwrap()
            .web_buf
    }

    pub fn web_buffer_len(&self) -> usize {
        self.get_wb().len()
    }

    pub fn web_cell_len(&self) -> usize {
        std::mem::size_of::<WebCell>() / 4
    }

    pub fn get_ratiox(&mut self) -> f32 {
        self.g.context.adapter.get_base().ratio_x
    }

    pub fn get_ratioy(&mut self) -> f32 {
        self.g.context.adapter.get_base().ratio_y
    }

    // web renders buffer, can be accessed in js using the following
    // const wbuflen = sg.web_buffer_len();
    // const wbufptr = sg.web_buffer();
    // let webbuf = new Uint32Array(wasm.memory.buffer, wbufptr, wbuflen);
    pub fn web_buffer(&self) -> *const WebCell {
        self.get_wb().as_slice().as_ptr()
    }
}

pub fn run() -> Result<(), JsValue> {
    let mut g = init_game().g;
    g.run().unwrap();
    g.render.panel.reset(&mut g.context);
    Ok(())
}
