//! Terminal mode - minimal placeholder, use graphics mode (wgpu) instead
#![allow(unused_imports)]
use crate::model::{LlmArenaModel, SCREEN_HEIGHT, SCREEN_WIDTH};
use rust_pixel::context::Context;
use rust_pixel::game::Render;
use rust_pixel::render::scene::Scene;
use rust_pixel::render::sprite::Sprite;
use rust_pixel::render::style::Color;

pub struct LlmArenaRender {
    pub scene: Scene,
}

impl Default for LlmArenaRender {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmArenaRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();
        let mut msg = Sprite::new(0, 0, 60, 3);
        msg.set_default_str("LLM Arena - Please run in graphics mode:");
        scene.add_sprite(msg, "msg");
        let mut msg2 = Sprite::new(0, 1, 60, 1);
        msg2.set_default_str("  cargo pixel r llm_arena wg");
        scene.add_sprite(msg2, "msg2");
        Self { scene }
    }
}

impl Render for LlmArenaRender {
    type Model = LlmArenaModel;

    fn init(&mut self, context: &mut Context, _model: &mut Self::Model) {
        context.adapter.init(60, 5, 1.0, 1.0, "LLM Arena".to_string());
        self.scene.init(context);
    }

    fn handle_event(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        self.scene.draw(ctx).unwrap();
    }
}
