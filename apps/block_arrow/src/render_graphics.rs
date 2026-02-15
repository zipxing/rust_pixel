#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{Block_arrowModel, BLOCK_ARROWH, BLOCK_ARROWW};
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::scene::Scene,
    render::sprite::Sprite,
    render::style::Color,
};

pub struct Block_arrowRender {
    pub scene: Scene,
}

impl Block_arrowRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();
        let gb = Sprite::new(0, 0, BLOCK_ARROWW, BLOCK_ARROWH);
        scene.add_sprite(gb, "back");
        Self { scene }
    }
}

impl Render for Block_arrowRender {
    type Model = Block_arrowModel;

    fn init(&mut self, context: &mut Context, _data: &mut Self::Model) {
        context
            .adapter
            .init(BLOCK_ARROWW, BLOCK_ARROWH, 1.0, 1.0, "block_arrow".to_string());
        self.scene.init(context);
    }

    fn handle_event(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}
    fn handle_timer(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {
        self.scene.draw(ctx).unwrap();
    }
}
