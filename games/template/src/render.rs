#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{TemplateModel, CARDH, CARDW, TEMPLATEH, TEMPLATEW};
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

pub struct TemplateRender {
    pub panel: Panel,
}

impl TemplateRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // Test pixel sprite in graphic mode...
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            for i in 0..15 {
                let mut pl = Sprite::new(4, 6, 1, 1);
                pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(14));
                pl.set_alpha(255 - 15*(15 - i));
                panel.add_pixel_sprite(pl, &format!("PL{}", i+1));
            }
        }

        // background...
        let mut gb = Sprite::new(0, 0, TEMPLATEW, TEMPLATEH);
        gb.set_alpha(30);
        panel.add_sprite(gb, "back");
        let gb2 = Sprite::new(0, 0, TEMPLATEW, TEMPLATEH);
        panel.add_pixel_sprite(gb2, "back2");
        panel.add_sprite(Sprite::new(0, 0, CARDW as u16, CARDH as u16), "t0");

        // msg...
        let adj = 2u16;
        let mut msg1 = Sprite::new(0 + adj, 14, 40, 1);
        msg1.set_default_str("press N for next card");
        panel.add_sprite(msg1, "msg1");
        let mut msg2 = Sprite::new(40 + adj, 14, 40, 1);
        msg2.set_default_str("press S shuffle cards");
        panel.add_sprite(msg2, "msg2");

        event_register("Template.RedrawTile", "draw_tile");

        Self { panel }
    }

    pub fn draw_tile<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<TemplateModel>().unwrap();

        let l = self.panel.get_sprite("t0");

        // make asset identifier...
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        let ext = "pix";
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
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
    fn init<G: Model>(&mut self, context: &mut Context, _data: &mut G) {
        context
            .adapter
            .init(TEMPLATEW + 2, TEMPLATEH, 1.0, 1.0, "template".to_string());
        self.panel.init(context);

        // set a static back img for text mode...
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        {
            let gb = self.panel.get_sprite("back");
            asset2sprite!(gb, context, "back.txt");
        }
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            let gb = self.panel.get_sprite("back");
            asset2sprite!(gb, context, "1.pix");
            let gb2 = self.panel.get_pixel_sprite("back2");
            asset2sprite!(gb2, context, "5.pix");
        }
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Template.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        // set a animate back img for graphic mode...
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            let d = data.as_any().downcast_mut::<TemplateModel>().unwrap();
            let ss = &mut self.panel.get_sprite("back");
            asset2sprite!(ss, ctx, "1.ssf", (ctx.stage / 3) as usize, 40, 1);
            for i in 0..15 {
                let pl = &mut self.panel.get_pixel_sprite(&format!("PL{}", i+1));
                d.bezier.advance_and_maybe_reverse(dt as f64 * 0.1 + 0.01 * i as f64);
                let kf_now = d.bezier.now_strict().unwrap();
                pl.set_pos(kf_now.x as u16, kf_now.y as u16);
                let c = ((ctx.stage / 10) % 255) as u8;
                pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(c));
                d.bezier.advance_and_maybe_reverse(-0.01 * i as f64);
            }
        }

        self.panel.draw(ctx).unwrap();
    }
}
