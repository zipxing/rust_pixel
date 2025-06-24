use crate::model::{SnakeModel, SNAKEH, SNAKEW};
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
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

        // Test pixel sprite in graphic mode...
        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        {
            let mut pl = Sprite::new(4, 6, 1, 1);
            pl.set_graph_sym(0, 0, 1, 20, Color::Indexed(222));
            t.add_pixel_sprite(pl, "PL1");
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
        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        t.add_pixel_sprite(Sprite::new(1, 1, SNAKEW as u16, SNAKEH as u16), "SNAKE");
        #[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
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
        #[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
        let l = self.panel.get_sprite("SNAKE");
        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
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
                        #[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
                        l.set_color_str(j as u16, i as u16, "▇", Color::LightGreen, Color::Reset);
                        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
                        l.set_graph_sym(j as u16, i as u16, 1, 0, Color::LightGreen);
                    }
                    10000 => {
                        let c = COLORS[(context.stage / 5) as usize % COLORS.len()];
                        #[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
                        l.set_color_str(j as u16, i as u16, "∙", c, Color::Reset);
                        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
                        l.set_graph_sym(j as u16, i as u16, 1, 83, c);
                    }
                    _ => {
                        let c = COLORS[gv as usize % COLORS.len()];
                        #[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
                        l.set_color_str(j as u16, i as u16, "▒", c, Color::Reset);
                        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
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
        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
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
