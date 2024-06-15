use crate::model::TowerModel;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
use rust_pixel::render::cell::cellsym;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::sprite::{Sprite, Sprites},
    render::style::{Color, Style},
    render::panel::Panel,
    util::shape::lightning,
};
use tower_lib::*;
// use log::info;


pub struct TowerRender {
    pub panel: Panel,
    pub sprites: Sprites,
}

impl TowerRender {
    pub fn new() -> Self {
        let t = Panel::new();
        let mut s = Sprites::new("main");

        s.add_by_tag(Sprite::new(1, 1, TOWERW as u16, TOWERH as u16), "TOWER");
        s.add_by_tag(
            Sprite::new(0, (TOWERH + 3) as u16, TOWERW as u16, 1u16),
            "TOWER-MSG",
        );
        event_register("Tower.RedrawGrid", "draw_grid");
        timer_register("Tower.TestTimer", 0.1, "test_timer");
        timer_fire("Tower.TestTimer", 8u8);

        Self {
            panel: t,
            sprites: s,
        }
    }

    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    pub fn create_sprites<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<TowerModel>().unwrap();
        let w = BW as u16;
        let h = BH as u16;
        self.panel.create_sprites(&d.blocks, w, h, |bl| {
            asset2sprite!(bl, ctx, "pix/block.pix");
        });
        self.panel.create_sprites(&d.towers, w, h, |_bl| {});
        self.panel.create_sprites(&d.monsters, 1, 2, |pl| {
            pl.set_sdl_content(0, 0, 15, 15, 2);
            pl.set_sdl_content(0, 1, 7, 15, 2);
        });
        self.panel.create_sprites(&d.bullets, 1, 1, |pl| {
            pl.set_sdl_content(0, 0, 29, 10, 2);
        });
        self.panel.create_sprites(
            &d.lasers,
            TOWERW as u16,
            TOWERH as u16,
            |_pl| {},
        );
        self.panel.create_sprites(&d.bombs, 1, 1, |_pl| {});
    }

    pub fn draw_movie<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<TowerModel>().unwrap();

        self.panel.draw_objs(
            &mut d.monsters,
            |pl, m| {
                let li = [9u8, 10, 11, 12, 13, 14, 15, 22, 23];
                pl.set_pos(
                    m.obj.pixel_pos.x as u16,
                    m.obj.pixel_pos.y as u16 - ctx.adapter.cell_height() as u16,
                );
                let step = m.obj.max_life as usize / 8 + 1;
                pl.set_sdl_content(0, 0, li[8 - m.obj.life as usize / step], 15, 2);
                if m.obj.mtype == 0 {
                    pl.set_sdl_content(0, 1, 6, 15, 2);
                } else {
                    pl.set_sdl_content(0, 1, 7, 15, 2);
                }
            },
        );

        self.panel.draw_objs(
            &mut d.bombs,
            |pl, b| {
                let li = [27u8, 26, 25, 24];
                if b.obj.btype == 0 {
                    // 怪物死掉后的炸弹波纹...
                    let sym = li[b.obj.stage as usize / 4];
                    pl.set_pos(b.obj.pixel_pos.x as u16, b.obj.pixel_pos.y as u16);
                    pl.set_sdl_content(0, 0, sym, 15, 2);
                } else {
                    // 怪物中弹的炸弹波纹...
                    pl.set_pos(
                        b.obj.pixel_pos.x as u16 + ctx.adapter.cell_width() as u16 / 4,
                        b.obj.pixel_pos.y as u16 + ctx.adapter.cell_height() as u16 / 4,
                    );
                    pl.set_sdl_content(0, 0, 25, 8, 2);
                }
            },
        );

        self.panel.draw_objs(
            &mut d.lasers,
            |pl, l| {
                pl.content.reset();
                // pl.set_pos(l.obj.pixel_pos.x, l.obj.pixel_pos.y);
                pl.set_pos(0, 0);
                let x0 = l.obj.src_pos.x * BW as u16 + 2;
                let y0 = l.obj.src_pos.y * BH as u16 + 2;
                let x1 = l.obj.dst_pos.x + 1;
                let y1 = l.obj.dst_pos.y + 1;
                let pts = lightning(x0, y0, x1, y1, 10, 8);
                for p in pts {
                    pl.draw_line(p.0, p.1, p.2, p.3, None, 45, 1);
                }
            },
        );

        self.panel.draw_objs(
            &mut d.bullets,
            |pl, b| {
                if b.obj.btype == 0 {
                    pl.set_sdl_content(0, 0, 8, 15, 2);
                } else {
                    pl.set_sdl_content(0, 0, 29, 10, 2);
                }
                pl.set_pos(b.obj.pixel_pos.x as u16, b.obj.pixel_pos.y as u16);
                pl.set_angle(b.obj.angle as f64 / 3.1415926 * 180.0 + 90.0);
            },
        );
    }

    pub fn draw_tower<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<TowerModel>().unwrap();
        self.panel.draw_objs(
            &mut d.towers,
            |pl, m| {
                asset2sprite!(pl, ctx, &format!("pix/tower{}.pix", m.obj.ttype + 1));
                pl.set_pos(
                    ((m.obj.pos.x * BW as u16 + 1) as f32 * ctx.adapter.cell_width()) as u16,
                    ((m.obj.pos.y * BH as u16 + 1) as f32 * ctx.adapter.cell_width()) as u16,
                );
                if !m.obj.target.is_none() {
                    pl.set_angle((ctx.stage % 20 * 18) as f64);
                }
            },
        );
    }

    pub fn draw_grid<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<TowerModel>().unwrap();

        let l = self.sprites.get_by_tag("TOWER");
        for i in 0..TOWERH {
            for j in 0..TOWERW {
                if d.grid[i][j] == 0 {
                    let mut sym = 32u8;
                    if i % 3 == 0 && j % 3 == 0 {
                        sym = 102u8;
                    }
                    l.content.set_str(
                        j as u16,
                        i as u16,
                        cellsym(sym),
                        Style::default()
                            .fg(Color::Indexed(235))
                            .bg(Color::Indexed(0)),
                    );
                }
            }
        }

        self.panel.draw_objs(
            &mut d.blocks,
            |pl, m| {
                pl.set_pos(
                    ((m.obj.pos.x * BW as u16 + 1) as f32 * ctx.adapter.cell_width()) as u16,
                    ((m.obj.pos.y * BH as u16 + 1) as f32 * ctx.adapter.cell_width()) as u16,
                );
            },
        );
    }
}

impl Render for TowerRender {
    fn init<G: Model>(&mut self, ctx: &mut Context, data: &mut G) {
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            ctx.adapter.init(
                TOWERW as u16 + 2,
                TOWERH as u16 + 4,
                1.0,
                1.0,
                "tower".to_string(),
            );
        }
        self.create_sprites(ctx, data);
        self.panel.init(ctx);
    }

    fn handle_event<G: Model>(&mut self, ctx: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Tower.RedrawGrid", "draw_grid") {
            self.draw_grid(ctx, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, ctx: &mut Context, _model: &mut G, _dt: f32) {
        if event_check("Tower.TestTimer", "test_timer") {
            let ml = self.sprites.get_by_tag("TOWER-MSG");
            ml.content.set_str(
                (ctx.stage / 6) as u16 % TOWERW as u16,
                0,
                "tower",
                Style::default().fg(Color::Yellow),
            );
            timer_fire("Tower.TestTimer", 0u8);
        }
    }

    fn draw<G: Model>(&mut self, ctx: &mut Context, model: &mut G, _dt: f32) {
        self.draw_tower(ctx, model);
        self.draw_movie(ctx, model);

        self.panel
            .draw(ctx, |a, f| {
                self.sprites.render_all(a, f);
            })
            .unwrap();
    }
}
