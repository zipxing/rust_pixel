use crate::model::BasicSnakeModel;
use rust_pixel::{
    context::Context,
    game::Render,
    render::{panel::Panel, sprite::Sprite, style::Color},
};
use pixel_basic::RenderBackend;
use log::{info, debug, error};

/// PanelBackend - Implements RenderBackend for rust_pixel's Panel
/// This backend draws directly to a single Sprite's buffer
pub struct PanelBackend<'a> {
    sprite: &'a mut Sprite,
}

impl<'a> PanelBackend<'a> {
    pub fn new(sprite: &'a mut Sprite) -> Self {
        Self { sprite }
    }
}

impl<'a> RenderBackend for PanelBackend<'a> {
    fn draw_pixel(&mut self, x: u16, y: u16, ch: char, fg: u8, bg: u8) {
        // Draw directly to the sprite's buffer
        self.sprite.set_color_str(
            x,
            y,
            ch.to_string(),
            Color::Indexed(fg),
            Color::Indexed(bg)
        );
    }

    fn clear(&mut self) {
        // Clear the entire sprite by filling with spaces
        let width = self.sprite.content.area.width;
        let height = self.sprite.content.area.height;
        for y in 0..height {
            for x in 0..width {
                self.sprite.set_color_str(x, y, " ", Color::Reset, Color::Reset);
            }
        }
    }

    fn add_sprite(&mut self, _id: u32, x: i32, y: i32, ch: char, fg: u8, bg: u8, _visible: bool) {
        // For now, just draw as a pixel (sprite management not implemented)
        self.draw_pixel(x as u16, y as u16, ch, fg, bg);
    }

    fn update_sprite(&mut self, _id: u32, x: i32, y: i32, ch: char, fg: u8, bg: u8, _visible: bool) {
        // For now, just draw as a pixel
        self.draw_pixel(x as u16, y as u16, ch, fg, bg);
    }

    fn has_sprite(&self, _id: u32) -> bool {
        false
    }
}

/// BasicSnakeRender - Terminal rendering using Panel API
pub struct BasicSnakeRender {
    pub panel: Panel,
}

impl BasicSnakeRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // Create a single sprite that covers the game area (60x25)
        // This sprite will be used as the canvas for BASIC drawing commands
        let canvas = Sprite::new(0, 0, 60, 25);
        panel.add_sprite(canvas, "CANVAS");

        Self { panel }
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
        self.panel.init(ctx);
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

        // Get the canvas sprite from panel
        let canvas = self.panel.get_sprite("CANVAS");

        // Create PanelBackend that wraps this sprite
        let backend = PanelBackend::new(canvas);
        let mut game_ctx = pixel_basic::PixelGameContext::new(backend);

        // Temporarily set game context in executor
        unsafe {
            let ctx_ptr = &mut game_ctx as *mut _ as *mut dyn pixel_basic::GameContext;
            model.bridge.executor_mut().set_game_context(Box::from_raw(ctx_ptr));
        }

        // Call ON_DRAW
        let result = model.bridge.call_subroutine(3500);

        // Reclaim context
        unsafe {
            if let Some(boxed) = model.bridge.executor_mut().game_context_mut() {
                let replacement: Box<dyn pixel_basic::GameContext> = Box::new(pixel_basic::NullGameContext);
                let ptr = Box::into_raw(std::mem::replace(boxed, replacement));
                let _ = ptr; // Don't drop
            }
        }

        if let Err(e) = result {
            error!("Failed to call ON_DRAW (frame {}): {:?}", model.frame_count, e);
        }

        // Draw the panel (which includes our updated canvas)
        if let Err(e) = self.panel.draw(ctx) {
            error!("Failed to draw panel (frame {}): {:?}", model.frame_count, e);
        }
    }
}
