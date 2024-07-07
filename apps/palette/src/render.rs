#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PaletteModel, CARDH, CARDW, PALETTEH, PALETTEW};
use palette_lib::COLORS_WITH_NAME;
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

pub struct PaletteRender {
    pub panel: Panel,
}

impl PaletteRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // Test pixel sprite in graphic mode...
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            for i in 0..15 {
                let mut pl = Sprite::new(4, 6, 1, 1);
                pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(14));
                pl.set_alpha(255 - 15 * (15 - i));
                panel.add_pixel_sprite(pl, &format!("PL{}", i + 1));
            }
        }

        let adjx = 1;
        let adjy = 20;

        for row in 0..15 {
            for col in 0..10 {
                let mut pl = Sprite::new(adjx + col * 10, adjy + row, 10, 1);
                let idx = (row * 10 + col) as usize;
                if idx >= COLORS_WITH_NAME.len() {
                    break;
                }
                let s = COLORS_WITH_NAME[idx].0;
                let r = COLORS_WITH_NAME[idx].1;
                let g = COLORS_WITH_NAME[idx].2;
                let b = COLORS_WITH_NAME[idx].3;
                let cr = Color::Rgba(r, g, b, 255);
                pl.set_color_str(
                    0,
                    0,
                    &format!("{:10}", s),
                    if cr.is_dark() {Color::White} else {Color::Rgba(0, 0, 0, 255)},
                    cr
                );
                panel.add_sprite(pl, &format!("{}", idx));
            }
        }

        // background...
        let mut gb = Sprite::new(0, 0, PALETTEW, PALETTEH);
        gb.set_alpha(30);
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

        event_register("Palette.RedrawTile", "draw_tile");

        Self { panel }
    }

    pub fn draw_tile<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();

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

impl Render for PaletteRender {
    fn init<G: Model>(&mut self, context: &mut Context, _data: &mut G) {
        context
            .adapter
            .init(PALETTEW + 2, PALETTEH, 1.0, 1.0, "palette".to_string());
        self.panel.init(context);

        // set a static back img for text mode...
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        {
            let gb = self.panel.get_sprite("back");
            asset2sprite!(gb, context, "back.txt");
        }
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Palette.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        // set a animate back img for graphic mode...
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            let d = data.as_any().downcast_mut::<PaletteModel>().unwrap();
            let ss = &mut self.panel.get_sprite("back");
            asset2sprite!(ss, ctx, "1.ssf", (ctx.stage / 3) as usize, 40, 1);
            for i in 0..15 {
                let pl = &mut self.panel.get_pixel_sprite(&format!("PL{}", i + 1));
                d.bezier
                    .advance_and_maybe_reverse(dt as f64 * 0.1 + 0.01 * i as f64);
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
