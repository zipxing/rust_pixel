#![allow(unused_imports)]
#![allow(unused_variables)]
// use log::info;
use crate::model::{ColorblkModel, CARDH, CARDW, COLORBLKH, COLORBLKW};
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

pub struct ColorblkRender {
    pub scene: Scene,
}

impl ColorblkRender {
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
        let mut gb = Sprite::new(0, 0, COLORBLKW, COLORBLKH);
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
            Sprite::new(0, (COLORBLKH - 3) as u16, COLORBLKW as u16, 1u16),
            "TIMER-MSG",
        );

        // register Block.RedrawTile event, associated draw_tile method
        event_register("Colorblk.RedrawTile", "draw_tile");

        // register a timer, then fire it...
        timer_register("Colorblk.TestTimer", 0.1, "test_timer");
        timer_fire("Colorblk.TestTimer", 0);

        Self { scene }
    }

    // create sprites for particles...
    // objpool can also be used to manage drawing other objects
    pub fn create_sprites(&mut self, _ctx: &mut Context, d: &mut ColorblkModel) {
        // create objpool sprites and init
    }

    // draw particles
    pub fn draw_movie(&mut self, _ctx: &mut Context, d: &mut ColorblkModel) {
        // draw objects
    }

    pub fn draw_tile(&mut self, ctx: &mut Context, d: &mut ColorblkModel) {
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

impl Render for ColorblkRender {
    type Model = ColorblkModel;

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        context
            .adapter
            .init(COLORBLKW, COLORBLKH, 0.5, 0.5, "colorblk".to_string());
        self.create_sprites(context, data);
        self.scene.init(context);
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        // if a Block.RedrawTile event checked, call draw_tile function...
        if event_check("Colorblk.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer(&mut self, context: &mut Context, d: &mut Self::Model, _dt: f32) {
        if event_check("Colorblk.TestTimer", "test_timer") {
            let ml = self.scene.get_sprite("TIMER-MSG");
            ml.set_color_str(
                (context.stage / 6) as u16 % COLORBLKW as u16,
                0,
                "Colorblk",
                Color::Yellow,
                Color::Reset,
            );
            timer_fire("Colorblk.TestTimer", 0);
        }
    }

    fn draw(&mut self, ctx: &mut Context, d: &mut Self::Model, dt: f32) {
        // set a animate background for graphic mode...
        // asset file is 1.ssf
        let ss = &mut self.scene.get_sprite("back");
        asset2sprite!(ss, ctx, "1.ssf", (ctx.stage / 3) as usize, 40, 1);

        self.draw_movie(ctx, d);

        // draw all compents in scene...
        self.scene.draw(ctx).unwrap();
    }
}
