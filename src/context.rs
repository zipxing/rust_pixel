// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Context encapsulates several public variables
//! including stage, state, input events, etc.
//! For simplicity, state is set to u8 type, you can create your own states using enums in your games.
//! Context also integrates an RNG for user's convenience.
//! A render adapter is also provided
//! to make it compatible with web, SDL, or terminal modes.
//! Finally, an asset_manager is included as well.

use crate::{asset::AssetManager, event::Event, render::adapter::Adapter, util::Rand};

#[cfg(cross_backend)]
use crate::render::adapter::cross_adapter::CrosstermAdapter;

#[cfg(sdl_backend)]
use crate::render::adapter::sdl_adapter::SdlAdapter;

#[cfg(winit_backend)]
use crate::render::adapter::winit_glow_adapter::WinitGlowAdapter;

#[cfg(wgpu_backend)]
use crate::render::adapter::winit_wgpu_adapter::WinitWgpuAdapter;

#[cfg(wasm)]
use crate::render::adapter::web_adapter::WebAdapter;

pub struct Context {
    pub game_name: String,
    pub project_path: String,
    pub stage: u32,
    pub state: u8,
    pub rand: Rand,
    pub asset_manager: AssetManager,
    pub input_events: Vec<Event>,
    pub adapter: Box<dyn Adapter>,
}

impl Context {
    pub fn new(name: &str, project_path: &str) -> Self {
        Self {
            game_name: name.to_string(),
            project_path: project_path.to_string(),
            stage: 0,
            state: 0,
            rand: Rand::new(),
            asset_manager: AssetManager::new(),
            input_events: vec![],
            #[cfg(wasm)]
            adapter: Box::new(WebAdapter::new(name, project_path)),
            #[cfg(sdl_backend)]
            adapter: Box::new(SdlAdapter::new(name, project_path)),
            #[cfg(winit_backend)]
            adapter: Box::new(WinitGlowAdapter::new(name, project_path)),
            #[cfg(wgpu_backend)]
            adapter: Box::new(WinitWgpuAdapter::new(name, project_path)),
            #[cfg(cross_backend)]
            adapter: Box::new(CrosstermAdapter::new(name, project_path)),
        }
    }

    pub fn set_asset_path(&mut self, project_path: &str) {
        self.project_path = project_path.to_string();
    }

    pub fn cell_width(&mut self) -> f32 {
        #[cfg(graphics_mode)]
        let ret = self.adapter.get_base().gr.cell_width();
        #[cfg(not(graphics_mode))]
        let ret = 0.0f32;
        ret
    }

    pub fn cell_height(&mut self) -> f32 {
        #[cfg(graphics_mode)]
        let ret = self.adapter.get_base().gr.cell_height();
        #[cfg(not(graphics_mode))]
        let ret = 0.0f32;
        ret
    }
}
