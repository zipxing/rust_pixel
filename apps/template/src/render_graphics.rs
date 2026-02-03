#![allow(unused_imports)]
#![allow(unused_variables)]
// use log::info;
use crate::model::{TemplateModel, CARDH, CARDW, TEMPLATEH, TEMPLATEW};
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

        // create pixel sprites in graphic mode...
        for i in 0..15 {
            let mut pl = Sprite::new(4, 6, 1, 1);
            // Use set_graph_sym set char content in graphics mode
            pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(14));
            // Alpha only support in graphics mode
            pl.set_alpha(255 - 15 * (15 - i));
            scene.add_sprite(pl, &format!("PL{}", i + 1));
        }

        // background...
        let mut gb = Sprite::new(0, 0, TEMPLATEW, TEMPLATEH);
        // Alpha only support in graphics mode
        gb.set_alpha(30);
        scene.add_sprite(gb, "back");
        scene.add_sprite(Sprite::new(0, 0, CARDW as u16, CARDH as u16), "t0");

        // msg, work on both text and graphics mode...
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

        // register Block.RedrawTile event, associated draw_tile method
        event_register("Template.RedrawTile", "draw_tile");

        // register a timer, then fire it...
        timer_register("Template.TestTimer", 0.1, "test_timer");
        timer_fire("Template.TestTimer", 0);

        Self { scene }
    }

    // create sprites for particles...
    // objpool can also be used to manage drawing other objects
    pub fn create_sprites(&mut self, _ctx: &mut Context, d: &mut TemplateModel) {
        // create objpool sprites and init
        self.scene
            .creat_objpool_sprites(&d.pats.particles, 1, 1, |bl| {
                bl.set_graph_sym(0, 0, 2, 25, Color::Indexed(10));
            });
    }

    // draw particles
    pub fn draw_movie(&mut self, _ctx: &mut Context, d: &mut TemplateModel) {
        // draw objects
        self.scene.draw_objpool(&mut d.pats.particles, |pl, m| {
            pl.set_pos(m.obj.loc[0] as u16, m.obj.loc[1] as u16);
        });
    }

    pub fn draw_tile(&mut self, ctx: &mut Context, d: &mut TemplateModel) {
        let l = self.scene.get_sprite("t0");

        // make asset identifier...
        // in graphics mode, poker card asset file named n.pix
        // in text mode, poker card asset file named n.txt
        let ext = "pix";
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
            .init(TEMPLATEW, TEMPLATEH, 1.0, 1.0, "template".to_string());
        self.create_sprites(context, data);
        self.scene.init(context);
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
        // set a animate background for graphic mode...
        // asset file is 1.ssf
        let ss = &mut self.scene.get_sprite("back");
        asset2sprite!(ss, ctx, "1.ssf", (ctx.stage / 3) as usize, 40, 1);

        // set a bezier animation for graphic mode...
        for i in 0..15 {
            let pl = &mut self.scene.get_sprite(&format!("PL{}", i + 1));
            d.bezier
                .advance_and_maybe_reverse(dt as f64 * 0.1 + 0.01 * i as f64);
            let kf_now = d.bezier.now_strict().unwrap();
            pl.set_pos(kf_now.x as u16, kf_now.y as u16);
            let c = ((ctx.stage / 10) % 255) as u8;
            pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(c));
            d.bezier.advance_and_maybe_reverse(-0.01 * i as f64);
        }
        self.draw_movie(ctx, d);

        // draw all compents in panel...
        self.scene.draw(ctx).unwrap();
    }
}
