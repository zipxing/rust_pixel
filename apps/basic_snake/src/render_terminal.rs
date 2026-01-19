use crate::model::BasicSnakeModel;
use rust_pixel::{
    context::Context,
    game::Render,
    render::panel::Panel,
};

/// BasicSnakeRender - Simplified terminal rendering
///
/// Note: This is a minimal implementation. Full PixelGameContext integration
/// with Panel API requires more work to adapt to rust_pixel's current API.
pub struct BasicSnakeRender {
    pub panel: Panel,
}

impl BasicSnakeRender {
    pub fn new() -> Self {
        Self {
            pub panel: Panel::new(),
        }
    }
}

impl Render for BasicSnakeRender {
    type Model = BasicSnakeModel;

    fn init(&mut self, ctx: &mut Context, _model: &mut Self::Model) {
        self.panel.init(ctx);
    }

    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        // TODO: Pass input events to BASIC GameContext
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        // Not used
    }

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        // Call ON_DRAW (line 3500) in BASIC script
        if let Err(e) = model.bridge.call_subroutine(3500) {
            eprintln!("Failed to call ON_DRAW: {:?}", e);
        }

        // Draw the panel
        if let Err(e) = self.panel.draw(ctx) {
            eprintln!("Failed to draw panel: {:?}", e);
        }
    }
}
