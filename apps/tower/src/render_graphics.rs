//
// Only support graphics mode!!!
//
use crate::model::TowerModel;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::Render,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
    util::{shape::lightning, PointU16},
};
use tower_lib::*;
// use log::info;

pub struct TowerRender {
    pub panel: Panel,
}

impl TowerRender {
    pub fn new() -> Self {
        let mut t = Panel::new();

        t.add_sprite(Sprite::new(1, 1, TOWERW as u16, TOWERH as u16), "TOWER");
        t.add_sprite(
            Sprite::new(0, (TOWERH + 3) as u16, TOWERW as u16, 1u16),
            "TOWER-MSG",
        );
        event_register("Tower.RedrawGrid", "draw_grid");
        timer_register("Tower.TestTimer", 0.1, "test_timer");
        timer_fire("Tower.TestTimer", 8u8);

        Self { panel: t }
    }

    pub fn create_sprites(&mut self, ctx: &mut Context, d: &mut TowerModel) {
        let w = BW as u16;
        let h = BH as u16;
        self.panel.creat_objpool_sprites(&d.blocks, w, h, |bl| {
            asset2sprite!(bl, ctx, "pix/block.pix");
        });
        self.panel.creat_objpool_sprites(&d.towers, w, h, |_bl| {});
        self.panel.creat_objpool_sprites(&d.monsters, 1, 2, |pl| {
            pl.set_graph_sym(0, 0, 2, 15, Color::Indexed(15));
            pl.set_graph_sym(0, 1, 2, 7, Color::Indexed(15));
        });
        self.panel.creat_objpool_sprites(&d.bullets, 1, 1, |pl| {
            pl.set_graph_sym(0, 0, 2, 29, Color::Indexed(10));
        });
        self.panel
            .creat_objpool_sprites(&d.lasers, TOWERW as u16, TOWERH as u16, |_pl| {});
        self.panel.creat_objpool_sprites(&d.bombs, 1, 1, |_pl| {});
    }

    pub fn draw_movie(&mut self, ctx: &mut Context, d: &mut TowerModel) {
        self.panel.draw_objpool(&mut d.monsters, |pl, m| {
            let li = [9u8, 10, 11, 12, 13, 14, 15, 22, 23];
            pl.set_pos(
                m.obj.pixel_pos.x as u16,
                m.obj.pixel_pos.y as u16 - ctx.cell_height() as u16,
            );
            let step = m.obj.max_life as usize / 8 + 1;
            pl.set_graph_sym(
                0,
                0,
                2,
                li[8 - m.obj.life as usize / step],
                Color::Indexed(15),
            );
            if m.obj.mtype == 0 {
                pl.set_graph_sym(0, 1, 2, 6, Color::Indexed(15));
            } else {
                pl.set_graph_sym(0, 1, 2, 7, Color::Indexed(15));
            }
        });

        self.panel.draw_objpool(&mut d.bombs, |pl, b| {
            let li = [27u8, 26, 25, 24];
            if b.obj.btype == 0 {
                // 怪物死掉后的炸弹波纹...
                let sym = li[b.obj.stage as usize / 4];
                pl.set_pos(b.obj.pixel_pos.x as u16, b.obj.pixel_pos.y as u16);
                pl.set_graph_sym(0, 0, 2, sym, Color::Indexed(15));
            } else {
                // 怪物中弹的炸弹波纹...
                pl.set_pos(
                    b.obj.pixel_pos.x as u16 + ctx.cell_width() as u16 / 4,
                    b.obj.pixel_pos.y as u16 + ctx.cell_height() as u16 / 4,
                );
                pl.set_graph_sym(0, 0, 2, 25, Color::Indexed(8));
            }
        });

        self.panel.draw_objpool(&mut d.lasers, |pl, l| {
            pl.content.reset();
            pl.set_pos(0, 0);
            let x0 = l.obj.src_pos.x * BW as u16 + 2;
            let y0 = l.obj.src_pos.y * BH as u16 + 2;
            let x1 = l.obj.dst_pos.x + 1;
            let y1 = l.obj.dst_pos.y + 1;
            let pts = lightning(x0, y0, x1, y1, 10, 8);
            for p in pts {
                pl.draw_line(
                    PointU16 { x: p.0, y: p.1 },
                    PointU16 { x: p.2, y: p.3 },
                    None,
                    45,
                    1,
                );
            }
        });

        self.panel.draw_objpool(&mut d.bullets, |pl, b| {
            if b.obj.btype == 0 {
                pl.set_graph_sym(0, 0, 2, 8, Color::Indexed(15));
            } else {
                pl.set_graph_sym(0, 0, 2, 29, Color::Indexed(10));
            }
            pl.set_pos(b.obj.pixel_pos.x as u16, b.obj.pixel_pos.y as u16);
            pl.set_angle(b.obj.angle as f64 / std::f64::consts::PI * 180.0 + 90.0);
        });
    }

    pub fn draw_tower(&mut self, ctx: &mut Context, d: &mut TowerModel) {
        self.panel.draw_objpool(&mut d.towers, |pl, m| {
            asset2sprite!(pl, ctx, &format!("pix/tower{}.pix", m.obj.ttype + 1));
            pl.set_pos(
                ((m.obj.pos.x * BW as u16 + 1) as f32 * ctx.cell_width()) as u16,
                ((m.obj.pos.y * BH as u16 + 1) as f32 * ctx.cell_height()) as u16,
            );
            pl.set_scale(3.0);
            if m.obj.target.is_some() {
                pl.set_angle((ctx.stage % 20 * 18) as f64);
            }
        });
    }

    pub fn draw_grid(&mut self, ctx: &mut Context, d: &mut TowerModel) {
        let l = self.panel.get_sprite("TOWER");
        for i in 0..TOWERH {
            for j in 0..TOWERW {
                if d.grid[i][j] == 0 {
                    let mut sym = 32u8;
                    if i % 3 == 0 && j % 3 == 0 {
                        sym = 102u8;
                    }
                    l.set_graph_sym(j as u16, i as u16, 0, sym, Color::Indexed(235));
                }
            }
        }

        self.panel.draw_objpool(&mut d.blocks, |pl, m| {
            pl.set_pos(
                ((m.obj.pos.x * BW as u16 + 1) as f32 * ctx.cell_width()) as u16,
                ((m.obj.pos.y * BH as u16 + 1) as f32 * ctx.cell_height()) as u16,
            );
        });
    }
}

impl Render for TowerRender {
    type Model = TowerModel;

    fn init(&mut self, ctx: &mut Context, data: &mut Self::Model) {
        ctx.adapter.init(
            TOWERW as u16 + 2,
            TOWERH as u16 + 4,
            0.4,
            0.4,
            "tower".to_string(),
        );
        self.create_sprites(ctx, data);
        self.panel.init(ctx);
        // ctx.adapter.only_render_buffer();
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("Tower.RedrawGrid", "draw_grid") {
            self.draw_grid(ctx, data);
        }
    }

    fn handle_timer(&mut self, ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        if event_check("Tower.TestTimer", "test_timer") {
            let ml = self.panel.get_sprite("TOWER-MSG");
            ml.set_color_str(
                (ctx.stage / 6) as u16 % TOWERW as u16,
                0,
                "tower",
                Color::Yellow,
                Color::Reset,
            );
            timer_fire("Tower.TestTimer", 0u8);
        }
    }

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        self.draw_tower(ctx, model);
        self.draw_movie(ctx, model);
        self.panel.draw(ctx).unwrap();
    }
}
