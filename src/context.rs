// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Context encapsulates several public variables
//! including stage，state，input events, etc.
//! For simplicity, state is set to u8 type，you can create your own states using enums in your games.
//! Context also integrates an RNG for user's convenience
//! An render adapter is also provided
//! to make it compatible with web, SDL, or terminal modes.
//! Finally, an asset_manager is included as well.

use crate::{asset::AssetManager, event::Event, render::adapter::Adapter, util::Rand};

#[cfg(all(not(target_arch = "wasm32"), not(feature = "sdl"), not(feature = "wgpu")))]
use crate::render::adapter::cross::CrosstermAdapter;

#[cfg(all(not(target_arch = "wasm32"), feature = "sdl"))]
use crate::render::adapter::sdl::SdlAdapter;

#[cfg(all(not(target_arch = "wasm32"), feature = "wgpu"))]
use crate::render::adapter::wgpu::WgpuAdapter;

#[cfg(target_arch = "wasm32")]
use crate::render::adapter::web::WebAdapter;

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
            #[cfg(target_arch = "wasm32")]
            adapter: Box::new(WebAdapter::new(name, project_path)),
            #[cfg(all(not(target_arch = "wasm32"), feature = "sdl"))]
            adapter: Box::new(SdlAdapter::new(name, project_path)),
            #[cfg(all(not(target_arch = "wasm32"), feature = "wgpu"))]
            adapter: Box::new(WgpuAdapter::new(name, project_path)),
            #[cfg(all(not(target_arch = "wasm32"), not(feature = "sdl"), not(feature = "wgpu")))]
            adapter: Box::new(CrosstermAdapter::new(name, project_path)),
        }
    }

    pub fn set_asset_path(&mut self, project_path: &str) {
        self.project_path = project_path.to_string();
    }
}
