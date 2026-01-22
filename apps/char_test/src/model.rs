use rust_pixel::game::Model;
use rust_pixel::context::Context;
use log::info;

pub const CHAR_TEST_WIDTH: usize = 80;
pub const CHAR_TEST_HEIGHT: usize = 30;

/// Model - handles state for character rendering test
pub struct CharTestModel {
    pub initialized: bool,
}

impl CharTestModel {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }
}

impl Model for CharTestModel {
    fn init(&mut self, _ctx: &mut Context) {
        info!("CharTest model initialized");
        self.initialized = true;
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_input(&mut self, ctx: &mut Context, _dt: f32) {
        // Clear input events
        ctx.input_events.clear();
    }

    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {}
}
