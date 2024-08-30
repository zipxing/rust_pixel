#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PetviewModel, PETVIEWH, PETVIEWW};
// use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};

pub struct PetviewRender {
    pub panel: Panel,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // background...
        let mut gb = Sprite::new(0, 0, PETVIEWW, PETVIEWH);
        gb.set_alpha(230);
        panel.add_sprite(gb, "back");
        let gb2 = Sprite::new(0, 0, PETVIEWW, PETVIEWH);
        panel.add_pixel_sprite(gb2, "back2");

        event_register("Petview.RedrawTile", "draw_tile");

        Self { panel }
    }
}

impl Render for PetviewRender {
    fn init<G: Model>(&mut self, context: &mut Context, _data: &mut G) {
        context
            .adapter
            .init(PETVIEWW + 2, PETVIEWH, 1.0, 1.0, "petview".to_string());
        self.panel.init(context);

        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            let gb = self.panel.get_sprite("back");
            asset2sprite!(gb, context, "1.pix");
            let gb2 = self.panel.get_pixel_sprite("back2");
            // asset2sprite!(gb2, context, "5.pix");
            asset2sprite!(gb2, context, "8.pix");
        }
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
