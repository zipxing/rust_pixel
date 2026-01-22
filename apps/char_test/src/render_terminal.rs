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
        info!("CharTest render initialized (terminal mode)");

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
        let buffer = self.panel.current_buffer_mut();
        buffer.reset();

        // Get the sprite for drawing
        let sprite = self.panel.get_layer_sprite("main", "text");
        render_test_content(&mut sprite.content);

        let _ = self.panel.draw(ctx);
    }
}

/// Render test content to the buffer
fn render_test_content(buffer: &mut rust_pixel::render::buffer::Buffer) {
    let mut y = 0;

    // Title
    buffer.set_str(0, y, "=== Character Rendering Test ===", Style::default().fg(Color::Yellow));
    y += 1;

    // Section 1: ASCII characters (should be half-width, 16x32)
    buffer.set_str(0, y, "1. ASCII: ABCDEFGHIJKLMNOPQRSTUVWXYZ", Style::default().fg(Color::White));
    y += 1;
    buffer.set_str(0, y, "   abcdefghijklmnopqrstuvwxyz 0123456789", Style::default().fg(Color::White));
    y += 1;

    // Section 2: CJK characters (should be full-width, 32x32)
    buffer.set_str(0, y, "2. CJK: 中文测试你好世界", Style::default().fg(Color::White));
    y += 1;
    buffer.set_str(0, y, "   日本語こんにちは 한국어안녕", Style::default().fg(Color::White));
    y += 1;

    // Section 3: Mixed content (ASCII + CJK)
    buffer.set_str(0, y, "3. Mixed: Hello世界! Test测试123", Style::default().fg(Color::White));
    y += 1;

    // Section 4: Pre-rendered Emoji (should use Block 53-55)
    buffer.set_str(0, y, "4. Emoji: 😀😊😂🤔😭🥺🎉🔥⭐🌟📁📂", Style::default().fg(Color::White));
    y += 1;

    // Section 5: Special characters (box drawing, etc)
    buffer.set_str(0, y, "5. Box: ┌─────┬─────┐│├─┼─┤└┴┘", Style::default().fg(Color::White));
    y += 1;

    // Section 6: Width alignment test
    buffer.set_str(0, y, "6. Width Test:", Style::default().fg(Color::Cyan));
    y += 1;
    buffer.set_str(0, y, "|12345678901234567890|", Style::default().fg(Color::Green));
    y += 1;
    buffer.set_str(0, y, "|ABCDEFGHIJ1234567890|", Style::default().fg(Color::Green));
    y += 1;
    buffer.set_str(0, y, "|中文五个字1234567890|", Style::default().fg(Color::Green));
    y += 1;
    buffer.set_str(0, y, "|😀😊😂🤔😭1234567890123|", Style::default().fg(Color::Green));
}
