use crate::model::{SnakeModel, SNAKEH, SNAKEW};
use rust_pixel::{
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::Render,
    render::scene::Scene,
    render::style::Color,
    ui::{Panel, BorderStyle, Widget},
    util::Rect,
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
    pub scene: Scene,
    /// Game panel with border and canvas for grid drawing
    pub game_panel: Panel,
    /// Message panel for status display
    pub msg_panel: Panel,
}

impl SnakeRender {
    pub fn new() -> Self {
        let scene = Scene::new();

        // Create game panel with border
        let mut game_panel = Panel::new()
            .with_bounds(Rect::new(0, 0, (SNAKEW + 2) as u16, (SNAKEH + 2) as u16))
            .with_border(BorderStyle::Single)
            .with_title("SNAKE [RustPixel]");
        // Enable canvas for game grid drawing
        game_panel.enable_canvas(SNAKEW as u16, SNAKEH as u16);

        // Create message panel (no border)
        let mut msg_panel = Panel::new()
            .with_bounds(Rect::new(0, (SNAKEH + 3) as u16, SNAKEW as u16, 1));
        msg_panel.enable_canvas(SNAKEW as u16, 1);

        event_register("Snake.RedrawGrid", "draw_grid");
        timer_register("Snake.TestTimer", 0.1, "test_timer");
        timer_fire("Snake.TestTimer", 8u8);

        Self {
            scene,
            game_panel,
            msg_panel,
        }
    }

    pub fn draw_grid(&mut self, context: &mut Context, d: &mut SnakeModel) {
        // Clear and set default message
        self.msg_panel.set_str(0, 0, "snake", Color::White, Color::Reset);

        info!("draw_grid...");
        for i in 0..SNAKEH {
            for j in 0..SNAKEW {
                let gv = d.grid[i][j];
                match gv {
                    0 => {
                        self.game_panel.set_char(j as u16, i as u16, " ", Color::Reset, Color::Reset);
                    }
                    1 => {
                        self.game_panel.set_char(j as u16, i as u16, "▇", Color::LightGreen, Color::Reset);
                    }
                    10000 => {
                        let c = COLORS[(context.stage / 5) as usize % COLORS.len()];
                        self.game_panel.set_char(j as u16, i as u16, "∙", c, Color::Reset);
                    }
                    _ => {
                        let c = COLORS[gv as usize % COLORS.len()];
                        self.game_panel.set_char(j as u16, i as u16, "▒", c, Color::Reset);
                    }
                }
            }
        }
    }
}

impl Render for SnakeRender {
    type Model = SnakeModel;

    fn init(&mut self, context: &mut Context, _data: &mut Self::Model) {
        context.adapter.init(
            SNAKEW as u16 + 2,
            SNAKEH as u16 + 4,
            0.5,
            0.5,
            "snake".to_string(),
        );
        self.scene.init(context);
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("Snake.RedrawGrid", "draw_grid") {
            self.draw_grid(context, data);
        }
    }

    fn handle_timer(&mut self, context: &mut Context, _model: &mut Self::Model, _dt: f32) {
        if event_check("Snake.TestTimer", "test_timer") {
            // Clear message area and draw moving text
            self.msg_panel.clear_canvas();
            let x = (context.stage / 6) as u16 % SNAKEW as u16;
            self.msg_panel.set_str(x, 0, "snake", Color::Yellow, Color::Reset);
            timer_fire("Snake.TestTimer", 8u8);
        }
    }

    fn draw(&mut self, context: &mut Context, _model: &mut Self::Model, _dt: f32) {
        // Render widgets to scene buffer
        let buffer = self.scene.tui_buffer_mut();
        let _ = self.game_panel.render(buffer, context);
        let _ = self.msg_panel.render(buffer, context);

        // Draw scene to screen
        self.scene.draw(context).unwrap();
    }
}
