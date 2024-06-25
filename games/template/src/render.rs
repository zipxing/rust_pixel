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
};

pub struct TemplateRender {
    pub panel: Panel,
}

impl TemplateRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // background...
        let mut gb = Sprite::new(0, 0, TEMPLATEW, TEMPLATEH);
        gb.set_alpha(30);
        panel.add_sprite(gb, "back");
        panel.add_sprite(Sprite::new(0, 0, CARDW as u16, CARDH as u16), "t0");

        // msg...
        let adj = 2u16;
        let mut msg1 = Sprite::new(0 + adj, 14, 40, 1);
        msg1.dstr("press N for next card");
        panel.add_sprite(msg1, "msg1");
        let mut msg2 = Sprite::new(40 + adj, 14, 40, 1);
        msg2.dstr("press S shuffle cards");
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
            let ss = &mut self.panel.get_sprite("back");
            asset2sprite!(ss, ctx, "1.ssf", (ctx.stage / 3) as usize, 40, 1);
        }

        self.panel.draw(ctx).unwrap();
    }
}
