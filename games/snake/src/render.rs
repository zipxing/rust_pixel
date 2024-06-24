use crate::model::{SnakeModel, SNAKEH, SNAKEW};
use log::info;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
use rust_pixel::{asset::AssetType, asset2sprite, render::cell::cellsym};
use rust_pixel::{
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::{Sprite, Sprites},
    render::style::{Color, Style},
};

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
    pub main_scene: Sprites,
}

impl SnakeRender {
    #[allow(unused_mut)]
    pub fn new() -> Self {
        let mut t = Panel::new();
        let mut s = Sprites::new("main");

        // Test pixel sprite in graphic mode...
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            let mut pl = Sprite::new(4, 6, 1, 1);
            pl.content.set_str(
                0,
                0,
                cellsym(20),
                Style::default()
                    .fg(Color::Indexed(222))
                    .bg(Color::Indexed(1)),
            );
            t.add_pixel_sprite(pl, "PL1");
        }

        // Main screen sprite...
        let mut l = Sprite::new(0, 0, (SNAKEW + 2) as u16, (SNAKEH + 2) as u16);
        l.set_alpha(160);
        l.content.set_str(
            20,
            0,
            "SNAKE [RustPixel]",
            Style::default().fg(Color::Indexed(222)),
        );
        s.add_by_tag(l, "SNAKE-BORDER");
        s.add_by_tag(Sprite::new(1, 1, SNAKEW as u16, SNAKEH as u16), "SNAKE");
        s.add_by_tag(
            Sprite::new(0, (SNAKEH + 3) as u16, SNAKEW as u16, 1u16),
            "SNAKE-MSG",
        );

        event_register("Snake.RedrawGrid", "draw_grid");
        timer_register("Snake.TestTimer", 0.1, "test_timer");
        timer_fire("Snake.TestTimer", 8u8);

        Self {
            panel: t,
            main_scene: s,
        }
    }

    pub fn create_sprites<G: Model>(&mut self, _ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<SnakeModel>().unwrap();
        self.panel.create_sprites(&d.pats.particles, 1, 1, |bl| {
            bl.set_sdl_content(0, 0, 25, 10, 2);
        });
    }

    pub fn draw_movie<G: Model>(&mut self, _ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<SnakeModel>().unwrap();

        self.panel.draw_objs(
            &mut d.pats.particles,
            |pl, m| {
                pl.set_pos(
                    m.obj.loc[0] as u16,
                    m.obj.loc[1] as u16,
                );
            },
        );
    }

    pub fn draw_grid<G: Model>(&mut self, context: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_ref::<SnakeModel>().unwrap();
        let ml = self.main_scene.get_by_tag("SNAKE-MSG");
        ml.content.set_str(0, 0, "snake", Style::default());
        let l = self.main_scene.get_by_tag("SNAKE");
        info!("draw_grid...");
        for i in 0..SNAKEH {
            for j in 0..SNAKEW {
                let gv = d.grid[i][j];
                match gv {
                    0 => {
                        l.content.set_str(
                            j as u16,
                            i as u16,
                            " ",
                            Style::default().fg(Color::Reset).bg(Color::Reset),
                        );
                    }
                    1 => {
                        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
                        l.content.set_str(
                            j as u16,
                            i as u16,
                            "▇",
                            Style::default().fg(Color::LightGreen),
                        );
                        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                        l.content.set_str(
                            j as u16,
                            i as u16,
                            cellsym(0),
                            Style::default().fg(Color::LightGreen).bg(Color::Red),
                        );
                    }
                    10000 => {
                        let c = COLORS[(context.stage / 5) as usize % COLORS.len()];
                        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
                        l.content
                            .set_str(j as u16, i as u16, "∙", Style::default().fg(c));
                        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                        l.content.set_str(
                            j as u16,
                            i as u16,
                            cellsym(83),
                            Style::default().fg(c).bg(Color::Red),
                        );
                    }
                    _ => {
                        let c = COLORS[gv as usize % COLORS.len()];
                        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
                        l.content
                            .set_str(j as u16, i as u16, "▒", Style::default().fg(c));
                        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                        l.content.set_str(
                            j as u16,
                            i as u16,
                            cellsym(102),
                            Style::default().fg(c).bg(Color::Red),
                        );
                    }
                }
            }
        }
    }
}

impl Render for SnakeRender {
    fn init<G: Model>(&mut self, context: &mut Context, data: &mut G) {
        context.adapter.init(
            SNAKEW as u16 + 2,
            SNAKEH as u16 + 4,
            1.0,
            1.0,
            "snake".to_string(),
        );
        self.create_sprites(context, data);
        self.panel.init(context);
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Snake.RedrawGrid", "draw_grid") {
            self.draw_grid(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, context: &mut Context, _model: &mut G, _dt: f32) {
        if event_check("Snake.TestTimer", "test_timer") {
            let ml = self.main_scene.get_by_tag("SNAKE-MSG");
            ml.content.set_str(
                (context.stage / 6) as u16 % SNAKEW as u16,
                0,
                "snake",
                Style::default().fg(Color::Yellow),
            );
            timer_fire("Snake.TestTimer", 8u8);
        }
    }

    #[allow(unused_variables)]
    fn draw<G: Model>(&mut self, context: &mut Context, model: &mut G, _dt: f32) {
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            let ss = &mut self.main_scene.get_by_tag("SNAKE-BORDER");
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
        self.panel
            .draw(context, |a, f| {
                self.main_scene.render_all(a, f);
            })
            .unwrap();
    }
}
