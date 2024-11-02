#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::PaletteModel;
use log::info;
use num_traits::FromPrimitive;
use palette_lib::COLORS_WITH_NAME;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::{Color, ColorData, ColorPro, ColorSpace::*, Style},
    util::Rect,
};
use std::cell::Cell;

pub struct PaletteRender {
    pub panel: Panel,
}

impl PaletteRender {
    pub fn new() -> Self {
        let panel = Panel::new();
        Self { panel }
    }
}

impl Render for PaletteRender {
    type Model = PaletteModel;

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        context.adapter.init(2, 2, 1.0, 1.0, "palette".to_string());
        self.panel.init(context);
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
