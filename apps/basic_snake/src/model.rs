use rust_pixel::{
    context::Context,
    game::Model,
};
use pixel_basic::GameBridge;
use log::{info, debug, error};

/// BasicSnakeModel - Game model that integrates BASIC script via GameBridge
pub struct BasicSnakeModel {
    /// GameBridge manages BASIC script execution
    pub bridge: GameBridge,

    /// Frame counter for debugging
    pub frame_count: u64,
}

impl BasicSnakeModel {
    pub fn new() -> Self {
        Self {
            bridge: GameBridge::new(),
            frame_count: 0,
        }
    }

    /// Load BASIC program from string
    pub fn load_program(&mut self, program: &str) {
        info!("Loading BASIC program...");
        if let Err(e) = self.bridge.load_program(program) {
            error!("Failed to load BASIC program: {:?}", e);
        } else {
            info!("BASIC program loaded successfully");
        }
    }
}

impl Model for BasicSnakeModel {
    fn init(&mut self, _ctx: &mut Context) {
        info!("BasicSnakeModel::init() called");
        // Load the BASIC program
        let program = include_str!("../assets/game.bas");
        self.load_program(program);
    }

    fn update(&mut self, _ctx: &mut Context, dt: f32) {
        self.frame_count += 1;
        debug!("Model::update() called, frame={}, dt={}", self.frame_count, dt);

        // Update BASIC script execution
        // This will call ON_TICK (line 2000) every frame
        if let Err(e) = self.bridge.update(dt) {
            error!("BASIC runtime error in update (frame {}): {:?}", self.frame_count, e);
        }

        // ESC key handling removed - will be handled by framework
    }

    fn handle_input(&mut self, _ctx: &mut Context, _dt: f32) {
        // Input handling is done by GameContext in the render layer
    }

    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {
        // Not used
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {
        // Not used
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {
        // Not used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_creation() {
        let model = BasicSnakeModel::new();
        assert_eq!(model.frame_count, 0);
    }

    #[test]
    fn test_load_program() {
        let mut model = BasicSnakeModel::new();
        model.load_program("10 PRINT \"HELLO\"\n20 END");
        // Should not panic
    }
}
