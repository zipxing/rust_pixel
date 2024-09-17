use crate::model::{PokerModel, CARDH, CARDW};
// use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::Render,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};

pub struct PokerRender {
    pub panel: Panel,
}

impl PokerRender {
    pub fn new() -> Self {
        let mut t = Panel::new();

        let gb = Sprite::new(0, 0, 80, 20);
        t.add_sprite(gb, "back");
        //red 5 cards, black 5 cards
        for i in 0..10 {
            t.add_sprite(
                Sprite::new(0, 0, CARDW as u16, CARDH as u16),
                &format!("t{}", i),
            );
        }

        let adj = 1u16;
        let msgred = Sprite::new(0 + adj, 14, 40, 1);
        t.add_sprite(msgred, "msgred");
        let msgblack = Sprite::new(40 + adj, 14, 40, 1);
        t.add_sprite(msgblack, "msgblack");

        event_register("Poker.RedrawTile", "draw_tile");

        Self { panel: t }
    }

    pub fn draw_tile(&mut self, ctx: &mut Context, d: &mut PokerModel) {
        let ts = [&d.texas_cards_red, &d.texas_cards_black];
        let msg = ["msgred", "msgblack"];
        for n in 0..2usize {
            for i in 0..5 {
                let l = self.panel.get_sprite(&format!("t{}", i + n * 5));
                let bi = ts[n].best[i].to_u8() as usize;

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

                let x = (i * CARDW) as u16 + 1u16 + n as u16 * 40u16;
                l.set_pos(x, 7);
            }
            let m = self.panel.get_sprite(msg[n]);
            m.set_color_str(
                0,
                0,
                format!("{:?}", ts[n].texas),
                Color::Indexed(222),
                Color::Reset,
            );
        }
    }
}

impl Render for PokerRender {
    type Model = PokerModel;

    fn init(&mut self, context: &mut Context, _data: &mut Self::Model) {
        context
            .adapter
            .init(82, 20, 1.0, 1.0, "redblack".to_string());
        self.panel.init(context);
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        {
            let gb = self.panel.get_sprite("back");
            asset2sprite!(gb, context, "back.txt");
        }
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("Poker.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
