use crate::model::{CharTestModel, CHAR_TEST_HEIGHT, CHAR_TEST_WIDTH};
use log::info;
use rust_pixel::{
    context::Context,
    game::Render,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::{Color, Style},
};

pub struct CharTestRender {
    pub panel: Panel,
}

impl CharTestRender {
    pub fn new() -> Self {
        Self {
            panel: Panel::new(),
        }
    }
}

impl Render for CharTestRender {
    type Model = CharTestModel;

    fn init(&mut self, ctx: &mut Context, _model: &mut CharTestModel) {
        info!("CharTest render initialized (graphics mode)");

        // Enable TUI character height mode (32px) for proper character rendering
        ctx.adapter.get_base().gr.set_use_tui_height(true);

        ctx.adapter.init(
            CHAR_TEST_WIDTH as u16,
            CHAR_TEST_HEIGHT as u16,
            1.0,
            1.0,
            String::new(),
        );

        self.panel.init(ctx);

        // Create main sprite for text rendering
        let main_sprite = Sprite::new(0, 0, CHAR_TEST_WIDTH as u16, CHAR_TEST_HEIGHT as u16);
        self.panel.add_layer_sprite(main_sprite, "main", "text");
    }

    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut CharTestModel, _dt: f32) {}

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut CharTestModel, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, model: &mut CharTestModel, _dt: f32) {
        self.update(ctx, model, _dt);
    }

    fn update(&mut self, ctx: &mut Context, _model: &mut CharTestModel, _dt: f32) {
        // Debug: Log sprite content area and panel buffer area
        static LOG_ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOG_ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
            let sprite = self.panel.get_layer_sprite("main", "text");
            log::info!("Sprite content area: {:?}", sprite.content.area);
            let buffer = self.panel.current_buffer_mut();
            log::info!("Panel buffer area: {:?}", buffer.area);
        }

        self.panel.current_buffer_mut().reset();

        // Get the sprite for drawing
        let sprite = self.panel.get_layer_sprite("main", "text");
        render_test_content(&mut sprite.content);

        let _ = self.panel.draw(ctx);
    }
}

/// Render test content to the buffer
fn render_test_content(buffer: &mut rust_pixel::render::buffer::Buffer) {
    // Test 1: Only emoji (should work)
    // Row 2: Emoji (static rendering from symbols.png)
    buffer.set_str(0, 2, "😀😊😂🤔😭🥺🎉🔥", Style::default().fg(Color::White));

    // Test 2: Add English text to see if it breaks emoji
    // Row 0: English text (dynamic rendering)
    buffer.set_str(0, 0, "Hello, RustPixel!", Style::default().fg(Color::Yellow));

    // Uncomment to test Chinese
    // // Row 1: Chinese text (dynamic rendering)
    // buffer.set_str(0, 1, "你好，世界！", Style::default().fg(Color::Cyan));

    // // Row 3: Mixed content - English + Emoji
    // buffer.set_str(0, 3, "Happy 😀 Coding 🔥", Style::default().fg(Color::Green));

    // // Row 4: Mixed content - Chinese + Emoji
    // buffer.set_str(0, 4, "编程 💻 快乐 🎉", Style::default().fg(Color::Magenta));

    // // Row 5: More emoji variety
    // buffer.set_str(0, 5, "🚀🌟⭐💡🎯🎨🎵🎮", Style::default().fg(Color::White));

    // // Row 6: Numbers and symbols (static PETSCII)
    // buffer.set_str(0, 6, "0123456789 !@#$%", Style::default().fg(Color::Red));
}
