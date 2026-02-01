#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{TemplateModel, CARDH, CARDW, TEMPLATEH, TEMPLATEW};
// use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::scene::Scene,
    render::sprite::Sprite,
    render::style::Color,
};

pub struct TemplateRender {
    pub scene: Scene,
}

impl TemplateRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // background...
        let gb = Sprite::new(0, 0, TEMPLATEW, TEMPLATEH);
        scene.add_sprite(gb, "back");
        scene.add_sprite(Sprite::new(0, 0, CARDW as u16, CARDH as u16), "t0");

        // msg...
        let adj = 2u16;
        let mut msg1 = Sprite::new(0 + adj, 14, 40, 1);
        msg1.set_default_str("press N for next card");
        scene.add_sprite(msg1, "msg1");
        let mut msg2 = Sprite::new(40 + adj, 14, 40, 1);
        msg2.set_default_str("press S shuffle cards");
        scene.add_sprite(msg2, "msg2");

        scene.add_sprite(
            Sprite::new(0, (TEMPLATEH - 3) as u16, TEMPLATEW as u16, 1u16),
            "TIMER-MSG",
        );

        // Register Block.RedrawTile event, associated draw_tile method
        event_register("Template.RedrawTile", "draw_tile");

        // Register a timer, then fire it...
        timer_register("Template.TestTimer", 0.1, "test_timer");
        timer_fire("Template.TestTimer", 0);

        Self { scene }
    }

    pub fn draw_tile(&mut self, ctx: &mut Context, d: &mut TemplateModel) {
        let l = self.scene.get_sprite("t0");

        // make asset identifier...
        // in text mode, poker card asset file named n.txt
        let ext = "txt";
        let cn = if d.card == 0 {
            format!("poker/back.{}", ext)
        } else {
            format!("poker/{}.{}", d.card, ext)
        };
        // set sprite content by asset identifier...
        asset2sprite!(l, ctx, &cn);

        // set sprite position...
        l.set_pos(1, 7);
    }
}

impl Render for TemplateRender {
    type Model = TemplateModel;

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        context
            .adapter
            .init(TEMPLATEW, TEMPLATEH, 0.5, 0.5, "template".to_string());
        self.scene.init(context);

        let gb = self.scene.get_sprite("back");
        asset2sprite!(gb, context, "back.txt");
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        // if a Block.RedrawTile event checked, call draw_tile function...
        if event_check("Template.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer(&mut self, context: &mut Context, d: &mut Self::Model, _dt: f32) {
        if event_check("Template.TestTimer", "test_timer") {
            let ml = self.scene.get_sprite("TIMER-MSG");
            ml.set_color_str(
                (context.stage / 6) as u16 % TEMPLATEW as u16,
                0,
                "Template",
                Color::Yellow,
                Color::Reset,
            );
            timer_fire("Template.TestTimer", 0);
        }
    }

    fn draw(&mut self, ctx: &mut Context, d: &mut Self::Model, dt: f32) {
        // draw all compents in panel...
        self.scene.draw(ctx).unwrap();
    }
}
