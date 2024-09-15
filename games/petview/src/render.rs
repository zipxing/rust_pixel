#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::gl::GlTransition;
use crate::model::{PetviewModel, PETH, PETW};
use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};

pub struct PetviewRender {
    pub panel: Panel,
    pub glt: GlTransition,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // background...
        let mut gb = Sprite::new(0, 0, PETW, PETH);
        gb.set_alpha(230);
        panel.add_sprite(gb, "back");

        let p1 = Sprite::new(100, 50, PETW, PETH);
        panel.add_pixel_sprite(p1, "petimg1");
        let p2 = Sprite::new(100, 50, PETW, PETH);
        panel.add_pixel_sprite(p2, "petimg2");

        let glt = GlTransition::new(40, 25);

        timer_register("PetView.Timer", 1.2, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self { panel, glt }
    }
}

impl Render for PetviewRender {
    fn init<G: Model>(&mut self, ctx: &mut Context, _data: &mut G) {
        ctx.adapter
            .init(PETW + 2, PETH, 1.0, 1.0, "petview".to_string());
        self.panel.init(ctx);

        let p1 = self.panel.get_pixel_sprite("petimg1");
        asset2sprite!(p1, ctx, "1.pix");
    }

    fn handle_event<G: Model>(&mut self, ctx: &mut Context, data: &mut G, _dt: f32) {}

    fn handle_timer<G: Model>(&mut self, ctx: &mut Context, _model: &mut G, _dt: f32) {
        if event_check("PetView.Timer", "pet_timer") {
            let p1 = self.panel.get_pixel_sprite("petimg1");
            asset2sprite!(p1, ctx, &format!("{}.pix", (ctx.stage / 13 % 20) + 1));
            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
