use crate::model::{GinRummyModel, CARDH, CARDW};
// use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::sprite::{Sprite, Sprites},
    render::style::{Color, Style},
    render::panel::Panel,
};

pub struct GinRummyRender {
    pub panel: Panel,
    pub sprites: Sprites,
}

impl GinRummyRender {
    pub fn new() -> Self {
        let t = Panel::new();
        let mut s = Sprites::new("main");

        let gb = Sprite::new(0, 0, 50, 35);
        s.add_by_tag(gb, "back");

        //red 10张牌, black 10张牌
        for i in 0..20 {
            s.add_by_tag(
                Sprite::new(0, 0, CARDW as u16, CARDH as u16),
                &format!("t{}", i),
            );
        }

        let adj = 2u16;
        let msgred = Sprite::new(0 + adj, 2, 40, 1);
        s.add_by_tag(msgred, "msgred");
        let msgblack = Sprite::new(0 + adj, 6 + CARDH as u16, 40, 1);
        s.add_by_tag(msgblack, "msgblack");

        event_register("GinRummy.RedrawTile", "draw_tile");

        Self {
            panel: t,
            sprites: s,
        }
    }

    pub fn draw_tile<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<GinRummyModel>().unwrap();
        let ts = [&d.cards_a, &d.cards_b];
        let msg = ["msgred", "msgblack"];
        let mut pv = vec![];
        let mut i = 0usize;
        for n in 0..2usize {
            let mut xadj = 0;
            for v in &ts[n].best_melds {
                let mut vs = v.clone();
                vs.sort();
                for p in vs {
                    let bi = p.to_u8() as usize;
                    pv.push((i, bi, n, xadj));
                    i += 1;
                }
                xadj += 4;
            }
            let mut vs = ts[n].best_deadwood.clone();
            vs.sort();
            for p in vs {
                let bi = p.to_u8() as usize;
                pv.push((i, bi, n, xadj));
                i += 1;
            }

            let m = self.sprites.get_by_tag(msg[n]);
            m.content.set_str(
                0,
                0,
                format!(
                    "deadwood{}:{:?}",
                    if n == 0 { "" } else { "(freeze)" },
                    ts[n].best
                ),
                Style::default().fg(Color::Indexed(222)),
            );
        }
        for p in pv {
            let (i, bi, n, xadj) = p;
            let l = self.sprites.get_by_tag(&format!("t{}", i));
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            let ext = "pix";
            #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
            let ext = "txt";
            let cn = if bi == 0 {
                format!("assets/poker/back.{}", ext)
            } else {
                format!("assets/poker/{}.{}", bi, ext)
            };
            asset2sprite!(l, ctx, &cn);
            let x = ((i % 10) * (CARDW - 2)) as u16 + 1u16;
            l.set_pos(x + xadj, 3u16 + (n as u16 * (CARDH + 4) as u16));
        }
    }
}

impl Render for GinRummyRender {
    fn init<G: Model>(&mut self, context: &mut Context, _dat: &mut G) {
        context
            .adapter
            .init(65, 25, 1.0, 1.0, "gin_rummy".to_string());
        self.panel.init(context);
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("GinRummy.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, _data: &mut G, _dt: f32) {
        self.panel
            .draw(ctx, |a, f| {
                self.sprites.render_all(a, f);
            })
            .unwrap();
    }
}
