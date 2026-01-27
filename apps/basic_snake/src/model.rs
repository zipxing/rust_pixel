use rust_pixel::{
    context::Context,
    event::{Event, KeyCode, KeyEventKind},
    game::Model,
};
use pixel_basic::GameBridge;
use log::{info, error};

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

    // Use default update() which calls: handle_event -> handle_timer -> handle_input -> handle_auto
    // This ensures handle_input sets key states BEFORE handle_auto runs the BASIC script

    fn handle_input(&mut self, ctx: &mut Context, _dt: f32) {
        // Clear all key states at the start of each frame
        self.bridge.context_mut().clear_key_states();

        // Process input events and forward to BASIC GameContext
        // Only process the LAST key event to avoid conflicts when multiple keys are pressed
        let mut last_direction_key: Option<String> = None;

        for event in &ctx.input_events {
            if let Event::Key(key_event) = event {
                // Only process Press events (not Release)
                if key_event.kind == KeyEventKind::Press {
                    // Map KeyCode to BASIC key name
                    let key_name = match key_event.code {
                        KeyCode::Char(c) => c.to_ascii_uppercase().to_string(),
                        KeyCode::Up => "UP".to_string(),
                        KeyCode::Down => "DOWN".to_string(),
                        KeyCode::Left => "LEFT".to_string(),
                        KeyCode::Right => "RIGHT".to_string(),
                        KeyCode::Esc => "ESC".to_string(),
                        KeyCode::Enter => "ENTER".to_string(),
                        KeyCode::Backspace => "BACKSPACE".to_string(),
                        KeyCode::Tab => "TAB".to_string(),
                        KeyCode::Delete => "DELETE".to_string(),
                        KeyCode::Home => "HOME".to_string(),
                        KeyCode::End => "END".to_string(),
                        KeyCode::PageUp => "PAGEUP".to_string(),
                        KeyCode::PageDown => "PAGEDOWN".to_string(),
                        KeyCode::F(n) => format!("F{}", n),
                        _ => continue,
                    };

                    // Handle space key specially (Char(' ') -> "SPACE")
                    let key_name = if key_name == " " { "SPACE".to_string() } else { key_name };

                    // For direction keys, only keep the last one
                    if key_name == "W" || key_name == "S" || key_name == "A" || key_name == "D"
                       || key_name == "UP" || key_name == "DOWN" || key_name == "LEFT" || key_name == "RIGHT" {
                        last_direction_key = Some(key_name);
                    } else {
                        // Non-direction keys are set immediately
                        self.bridge.context_mut().set_key_state(key_name, true);
                    }
                }
            }
        }

        // Only set the last direction key
        if let Some(key) = last_direction_key {
            self.bridge.context_mut().set_key_state(key, true);
        }

        // Clear input events after processing to prevent accumulation
        ctx.input_events.clear();
    }

    fn handle_auto(&mut self, _ctx: &mut Context, dt: f32) {
        self.frame_count += 1;

        // Update BASIC script execution
        // This runs AFTER handle_input, so key states are already set
        if let Err(e) = self.bridge.update(dt) {
            error!("BASIC runtime error in handle_auto (frame {}): {:?}", self.frame_count, e);
        }
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
