use crate::model::{TemplateModel, CARDH, CARDW, TEMPLATEH, TEMPLATEW};
// use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::{Sprite, Sprites},
};

pub struct TemplateRender {
    pub panel: Panel,
    pub sprites: Sprites,
}

impl TemplateRender {
    pub fn new() -> Self {
        let t = Panel::new();
        let mut s = Sprites::new("main");

        // background...
        let mut gb = Sprite::new(0, 0, TEMPLATEW, TEMPLATEH);
        // In text mode "alpha" does not affect
        gb.set_alpha(30);
        s.add_by_tag(gb, "back");

        for i in 0..1 {
            s.add_by_tag(
                Sprite::new(0, 0, CARDW as u16, CARDH as u16),
                &format!("t{}", i),
            );
        }

        let adj = 2u16;
        let mut msg1 = Sprite::new(0 + adj, 14, 40, 1);
        msg1.content.dstr("press N for next card");
        s.add_by_tag(msg1, "msg1");
        let mut msg2 = Sprite::new(40 + adj, 14, 40, 1);
        msg2.content.dstr("press S shuffle cards");
        s.add_by_tag(msg2, "msg2");

        event_register("Template.RedrawTile", "draw_tile");

        Self {
            panel: t,
            sprites: s,
        }
    }

    pub fn draw_tile<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<TemplateModel>().unwrap();
        let bi = d.card;
        let l = self.sprites.get_by_tag("t0");

        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        let ext = "pix";
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        let ext = "txt";
        let cn = if bi == 0 {
            format!("poker/back.{}", ext)
        } else {
            format!("poker/{}.{}", bi, ext)
        };
        asset2sprite!(l, ctx, &cn);

        let x = (0 * CARDW) as u16 + 1u16 + 0 as u16 * 40u16;
        l.set_pos(x, 7);
    }
}

impl Render for TemplateRender {
    fn init<G: Model>(&mut self, context: &mut Context, _data: &mut G) {
        context
            .adapter
            .init(TEMPLATEW + 2, TEMPLATEH, 1.0, 1.0, "redblack".to_string());
        self.panel.init(context);

        // set a static back img for text mode...
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        {
            let gb = self.sprites.get_by_tag("back");
            asset2sprite!(gb, context, "back.txt");
        }
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Template.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, _data: &mut G, _dt: f32) {
        // set a animate back img for graphic mode...
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            let ss = &mut self.sprites.get_by_tag("back");
            asset2sprite!(
                ss,
                ctx,
                "1.ssf",
                (ctx.stage / 3) as usize,
                40,
                1
            );
        }

        self.panel
            .draw(ctx, |a, f| {
                self.sprites.render_all(a, f);
            })
            .unwrap();
    }
}
