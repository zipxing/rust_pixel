//
// Only support graphics mode!!!
//
use crate::model::TowerModel;
use rust_pixel::{context::Context, game::Render, render::scene::Scene};
use tower_lib::*;
// use log::info;

pub struct TowerRender {
    pub scene: Scene,
}

impl TowerRender {
    pub fn new() -> Self {
        let t = Scene::new();
        Self { scene: t }
    }
}

impl Render for TowerRender {
    type Model = TowerModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter.init(
            TOWERW as u16 + 2,
            TOWERH as u16 + 4,
            0.4,
            0.4,
            "tower".to_string(),
        );
        self.scene.init(ctx);
    }

    fn handle_event(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        self.scene.draw(ctx).unwrap();
    }
}
