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
    // ONLY emoji - no dynamic text to interfere
    // Emoji at (0, 0) should render at pixel (0, 0)
    buffer.set_str(0, 0, "😀", Style::default().fg(Color::White));

    // Emoji at (4, 1) - col 4, row 1
    buffer.set_str(4, 1, "😊", Style::default().fg(Color::White));

    // Multiple emoji on row 2
    buffer.set_str(0, 2, "😀😊😂🤔😭🥺🎉🔥", Style::default().fg(Color::White));
}
