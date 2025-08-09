use crate::model::{UiDemoModel, UI_DEMO_HEIGHT, UI_DEMO_WIDTH};
use log::info;
use rust_pixel::{context::Context, game::Render, render::panel::Panel, render::sprite::Sprite};

pub struct UiDemoRender {
    pub panel: Panel,
}

impl UiDemoRender {
    pub fn new() -> Self {
        Self {
            panel: Panel::new(),
        }
    }
}

impl Render for UiDemoRender {
    type Model = UiDemoModel;

    fn init(&mut self, ctx: &mut Context, _model: &mut UiDemoModel) {
        info!("UI Demo render initialized (graphics mode)");

        // Initialize adapter for graphics mode
        ctx.adapter.init(
            UI_DEMO_WIDTH as u16,
            UI_DEMO_HEIGHT as u16,
            1.0,
            1.0,
            String::new(),
        );

        // Initialize the panel to cover the full screen
        self.panel.init(ctx);

        // Add a UI sprite to the pixel layer
        let mut ui_sprite = Sprite::new(0, 0, UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16);
        // Test UI scaling - you can change this scale factor to test different scales
        ui_sprite.set_scale(0.5); // Scale UI to 150%
        self.panel.add_layer_sprite(ui_sprite, "pixel", "UI");
    }

    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {
        // Events are handled in the model
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {
        // Timer events
    }

    fn draw(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        // This is the main drawing method
        self.update(ctx, model, _dt);
    }

    fn update(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        // Clear the current buffer
        let buffer = self.panel.current_buffer_mut();
        buffer.reset();

        // Get the UI sprite and render UI directly into its content buffer (zero-copy)
        let ui_sprite = self.panel.get_layer_sprite("pixel", "UI");
        let _ = model.ui_app.render_into(&mut ui_sprite.content);

        // Draw to screen
        let _ = self.panel.draw(ctx);
    }
}
