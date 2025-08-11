use crate::model::{SnakeModel, SNAKEH, SNAKEW};
#[cfg(graphics_mode)]
use rust_pixel::{asset::AssetType, asset2sprite};
use rust_pixel::{
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::Render,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};
use log::info;

const COLORS: [Color; 14] = [
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightBlue,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightCyan,
];

pub struct SnakeRender {
    pub panel: Panel,
}

impl SnakeRender {
    #[allow(unused_mut)]
    pub fn new() -> Self {
        let mut t = Panel::new();

        // Test pixel sprite scaling in graphic mode...
        #[cfg(graphics_mode)]
        {
            // 原始正常尺寸的sprite
            let mut pl1 = Sprite::new(50, 5, 1, 1);
            pl1.set_graph_sym(0, 0, 1, 21, Color::Indexed(222));
            pl1.set_scale_x(2.0);
            pl1.set_scale_y(2.0);
            t.add_pixel_sprite(pl1, "PL1");

            // // 半宽sprite测试
            // let mut pl2 = Sprite::new(52, 5, 1, 1);
            // pl2.set_graph_sym(0, 0, 1, 21, Color::Indexed(10));
            // pl2.set_scale_x(0.5);  // 半宽
            // t.add_pixel_sprite(pl2, "PL2_HALF");


            // // 双宽sprite测试
            // let mut pl3 = Sprite::new(54, 5, 1, 1);
            // pl3.set_graph_sym(0, 0, 1, 22, Color::Indexed(12));
            // pl3.set_scale_x(2.0);  // 双宽
            // t.add_pixel_sprite(pl3, "PL3_DOUBLE");

            // // 半高sprite测试
            // let mut pl4 = Sprite::new(50, 7, 1, 1);
            // pl4.set_graph_sym(0, 0, 1, 23, Color::Indexed(14));
            // pl4.set_scale_y(0.5);  // 半高
            // t.add_pixel_sprite(pl4, "PL4_HALF_HEIGHT");

            // // 完全缩小sprite测试
            // let mut pl5 = Sprite::new(52, 7, 1, 1);
            // pl5.set_graph_sym(0, 0, 1, 24, Color::Indexed(9));
            // pl5.set_scale(0.5);  // 半宽半高
            // t.add_pixel_sprite(pl5, "PL5_SMALL");

            // // 文字sprite半宽测试
            // let mut text_sprite = Sprite::new(50, 10, 12, 1);
            // text_sprite.set_color_str(0, 0, "Half Width:", Color::Yellow, Color::Reset);
            // text_sprite.set_scale_x(0.5);  // 半宽文字
            // t.add_pixel_sprite(text_sprite, "TEXT_HALF");

            // // 文字sprite正常宽度对比
            // let mut text_sprite_normal = Sprite::new(50, 12, 14, 1);
            // text_sprite_normal.set_color_str(0, 0, "Normal Width:", Color::Cyan, Color::Reset);
            // t.add_pixel_sprite(text_sprite_normal, "TEXT_NORMAL");

            // // 标签说明
            // let mut label = Sprite::new(50, 3, 20, 1);
            // label.set_color_str(0, 0, "Scale Test Area:", Color::White, Color::Reset);
            // t.add_pixel_sprite(label, "LABEL");
        }

        // Main screen sprite...
        let mut l = Sprite::new(0, 0, (SNAKEW + 2) as u16, (SNAKEH + 2) as u16);
        // l.set_alpha(160);
        l.set_color_str(
            20,
            0,
            "SNAKE [RustPixel]",
            Color::Indexed(222),
            Color::Reset,
        );
        t.add_sprite(l, "SNAKE-BORDER");
        #[cfg(graphics_mode)]
        t.add_pixel_sprite(Sprite::new(1, 1, SNAKEW as u16, SNAKEH as u16), "SNAKE");
        #[cfg(not(graphics_mode))]
        t.add_sprite(Sprite::new(1, 1, SNAKEW as u16, SNAKEH as u16), "SNAKE");
        t.add_sprite(
            Sprite::new(0, (SNAKEH + 3) as u16, SNAKEW as u16, 1u16),
            "SNAKE-MSG",
        );

        event_register("Snake.RedrawGrid", "draw_grid");
        timer_register("Snake.TestTimer", 0.1, "test_timer");
        timer_fire("Snake.TestTimer", 8u8);

        Self { panel: t }
    }

    pub fn create_sprites(&mut self, _ctx: &mut Context, d: &mut SnakeModel) {
        self.panel
            .creat_objpool_sprites(&d.pats.particles, 1, 1, |bl| {
                bl.set_graph_sym(0, 0, 2, 25, Color::Indexed(10));
            });
    }

    pub fn draw_movie(&mut self, _ctx: &mut Context, d: &mut SnakeModel) {
        self.panel.draw_objpool(&mut d.pats.particles, |pl, m| {
            pl.set_pos(m.obj.loc[0] as u16, m.obj.loc[1] as u16);
        });
    }

    pub fn draw_grid(&mut self, context: &mut Context, d: &mut SnakeModel) {
        let ml = self.panel.get_sprite("SNAKE-MSG");
        ml.set_default_str("snake");
        #[cfg(not(graphics_mode))]
        let l = self.panel.get_sprite("SNAKE");
        #[cfg(graphics_mode)]
        let l = self.panel.get_pixel_sprite("SNAKE");
        info!("draw_grid...");
        for i in 0..SNAKEH {
            for j in 0..SNAKEW {
                let gv = d.grid[i][j];
                match gv {
                    0 => {
                        l.set_color_str(j as u16, i as u16, " ", Color::Reset, Color::Reset);
                    }
                    1 => {
                        #[cfg(not(graphics_mode))]
                        l.set_color_str(j as u16, i as u16, "▇", Color::LightGreen, Color::Reset);
                        #[cfg(graphics_mode)]
                        l.set_graph_sym(j as u16, i as u16, 1, 0, Color::LightGreen);
                    }
                    10000 => {
                        let c = COLORS[(context.stage / 5) as usize % COLORS.len()];
                        #[cfg(not(graphics_mode))]
                        l.set_color_str(j as u16, i as u16, "∙", c, Color::Reset);
                        #[cfg(graphics_mode)]
                        l.set_graph_sym(j as u16, i as u16, 1, 83, c);
                    }
                    _ => {
                        let c = COLORS[gv as usize % COLORS.len()];
                        #[cfg(not(graphics_mode))]
                        l.set_color_str(j as u16, i as u16, "▒", c, Color::Reset);
                        #[cfg(graphics_mode)]
                        l.set_graph_sym(j as u16, i as u16, 1, 102, c);
                    }
                }
            }
        }
    }
}

impl Render for SnakeRender {
    type Model = SnakeModel;

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        context.adapter.init(
            SNAKEW as u16 + 2,
            SNAKEH as u16 + 4,
            0.5,
            0.5,
            "snake".to_string(),
        );
        self.create_sprites(context, data);
        self.panel.init(context);
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("Snake.RedrawGrid", "draw_grid") {
            self.draw_grid(context, data);
        }
    }

    fn handle_timer(&mut self, context: &mut Context, _model: &mut Self::Model, _dt: f32) {
        if event_check("Snake.TestTimer", "test_timer") {
            let ml = self.panel.get_sprite("SNAKE-MSG");
            ml.set_color_str(
                (context.stage / 6) as u16 % SNAKEW as u16,
                0,
                "snake",
                Color::Yellow,
                Color::Reset,
            );
            timer_fire("Snake.TestTimer", 8u8);
        }
    }

    #[allow(unused_variables)]
    fn draw(&mut self, context: &mut Context, model: &mut Self::Model, _dt: f32) {
        #[cfg(graphics_mode)]
        {
            let ss = &mut self.panel.get_sprite("SNAKE-BORDER");
            asset2sprite!(
                ss,
                context,
                "sdq/dance.ssf",
                (context.stage / 3) as usize,
                1,
                1
            );
            if context.stage % 8 == 0 {
                let pl = self.panel.get_pixel_sprite("PL1");
                pl.content.area.x += 2;
                pl.content.area.y += 2;
            }
        }
        self.draw_movie(context, model);
        self.panel.draw(context).unwrap();
    }
}
