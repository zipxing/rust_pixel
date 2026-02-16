#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::*;
use rust_pixel::{
    context::Context,
    game::Render,
    render::scene::Scene,
};

pub struct PaletteRender {
    pub scene: Scene,
}

impl PaletteRender {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
        }
    }
}

impl Render for PaletteRender {
    type Model = PaletteModel;

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        context.adapter.get_base().gr.set_use_tui_height(true);
        context
            .adapter
            .init(PALETTEW + 2, PALETTEH, 2.5, 2.5, "palette".to_string());
        self.scene.init(context);
        data.need_redraw = true;
    }

    fn handle_event(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        let source_buffer = data.get_rendered_buffer();
        let tui_buffer = self.scene.tui_buffer_mut();
        tui_buffer.reset();
        tui_buffer.merge(source_buffer, 255, true);
        self.scene.draw(ctx).unwrap();
    }
}
