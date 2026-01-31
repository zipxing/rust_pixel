use crate::model::BasicSnakeModel;
use rust_pixel::{
    context::Context,
    game::Render,
    render::{scene::Scene, sprite::Sprite, style::Color},
};
use pixel_basic::DrawCommand;
use log::{info, debug, error};

/// BasicSnakeRender - Terminal rendering using Scene API
pub struct BasicSnakeRender {
    pub scene: Scene,
}

impl BasicSnakeRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Create a single sprite that covers the game area (60x25)
        // This sprite will be used as the canvas for BASIC drawing commands
        let canvas = Sprite::new(0, 0, 60, 25);
        scene.add_sprite(canvas, "CANVAS");

        Self { scene }
    }

    /// Apply draw commands from BASIC to the canvas sprite
    fn apply_draw_commands(&mut self, model: &mut BasicSnakeModel) {
        let canvas = self.scene.get_sprite("CANVAS");

        // Drain commands from the bridge's context and apply to sprite
        for cmd in model.bridge.context_mut().drain_commands() {
            match cmd {
                DrawCommand::Plot { x, y, ch, fg, bg } => {
                    if x >= 0 && y >= 0 {
                        canvas.set_color_str(
                            x as u16,
                            y as u16,
                            ch.to_string(),
                            Color::Indexed(fg),
                            Color::Indexed(bg),
                        );
                    }
                }
                DrawCommand::Clear => {
                    // Clear the entire sprite by filling with spaces
                    let width = canvas.content.area.width;
                    let height = canvas.content.area.height;
                    for cy in 0..height {
                        for cx in 0..width {
                            canvas.set_color_str(cx, cy, " ", Color::Reset, Color::Reset);
                        }
                    }
                }
            }
        }

        // Also render sprites from BASIC
        for sprite_data in model.bridge.context().sprites().values() {
            if !sprite_data.hidden {
                if sprite_data.x >= 0 && sprite_data.y >= 0 {
                    canvas.set_color_str(
                        sprite_data.x as u16,
                        sprite_data.y as u16,
                        sprite_data.ch.to_string(),
                        Color::Indexed(sprite_data.fg),
                        Color::Indexed(sprite_data.bg),
                    );
                }
            }
        }
    }
}

impl Render for BasicSnakeRender {
    type Model = BasicSnakeModel;

    fn init(&mut self, ctx: &mut Context, _model: &mut Self::Model) {
        info!("BasicSnakeRender::init() called");

        // Initialize adapter first with screen dimensions
        ctx.adapter.init(
            60,  // width (matches game.bas BOX 0, 0, 60, 24)
            25,  // height
            1.0, // scale_x
            1.0, // scale_y
            "Basic Snake".to_string(),
        );
        info!("Adapter initialized: 60x25");

        // Initialize the panel
        self.scene.init(ctx);
        info!("Panel initialized");
    }

    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        // TODO: Pass input events to BASIC GameContext
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        // Not used
    }

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        debug!("Render::draw() called, frame={}", model.frame_count);

        // Call ON_DRAW to collect draw commands
        if let Err(e) = model.bridge.draw() {
            error!("Failed to call ON_DRAW (frame {}): {:?}", model.frame_count, e);
        }

        // Apply the collected draw commands to the canvas
        self.apply_draw_commands(model);

        // Draw the panel (which includes our updated canvas)
        if let Err(e) = self.scene.draw(ctx) {
            error!("Failed to draw panel (frame {}): {:?}", model.frame_count, e);
        }
    }
}
