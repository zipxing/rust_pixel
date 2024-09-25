mod model;
mod render;

use crate::{model::TowerModel, render::TowerRender};
// use log::debug;
use rust_pixel::game::Game;

#[cfg(target_arch = "wasm32")]
use rust_pixel::render::adapter::web::{input_events_from_web, WebAdapter};
#[cfg(target_arch = "wasm32")]
use rust_pixel::render::adapter::RenderCell;
use wasm_bindgen::prelude::*;

use pixel_macro::pixel_game;

pixel_game!(Tower);
