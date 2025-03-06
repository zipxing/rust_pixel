#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{ColorblkModel, CARDH, CARDW, COLORBLKH, COLORBLKW};
// use log::info;
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

pub struct ColorblkRender {
    pub panel: Panel,
}

impl ColorblkRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // background...
        let gb = Sprite::new(0, 0, COLORBLKW, COLORBLKH);
        panel.add_sprite(gb, "back");
        panel.add_sprite(Sprite::new(0, 0, CARDW as u16, CARDH as u16), "t0");

        // msg...
        let adj = 2u16;
        let mut msg1 = Sprite::new(0 + adj, 14, 40, 1);
        msg1.set_default_str("press N for next card");
        panel.add_sprite(msg1, "msg1");
        let mut msg2 = Sprite::new(40 + adj, 14, 40, 1);
        msg2.set_default_str("press S shuffle cards");
        panel.add_sprite(msg2, "msg2");

        panel.add_sprite(
            Sprite::new(0, (COLORBLKH - 3) as u16, COLORBLKW as u16, 1u16),
            "TIMER-MSG",
        );

        // Register Block.RedrawTile event, associated draw_tile method
        event_register("Colorblk.RedrawTile", "draw_tile");

        // Register a timer, then fire it...
        timer_register("Colorblk.TestTimer", 0.1, "test_timer");
        timer_fire("Colorblk.TestTimer", 0);

        Self { panel }
    }

    pub fn draw_tile(&mut self, ctx: &mut Context, d: &mut ColorblkModel) {
        let l = self.panel.get_sprite("t0");

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

impl Render for ColorblkRender {
    type Model = ColorblkModel;

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        context
            .adapter
            .init(COLORBLKW + 2, COLORBLKH, 0.5, 0.5, "colorblk".to_string());
        self.panel.init(context);

        let gb = self.panel.get_sprite("back");
        asset2sprite!(gb, context, "back.txt");
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        // if a Block.RedrawTile event checked, call draw_tile function...
        if event_check("Colorblk.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer(&mut self, context: &mut Context, d: &mut Self::Model, _dt: f32) {
        if event_check("Colorblk.TestTimer", "test_timer") {
            let ml = self.panel.get_sprite("TIMER-MSG");
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
        // draw all compents in panel...
        self.panel.draw(ctx).unwrap();
    }
}
