use crate::model::{UiDemoModel, UI_DEMO_WIDTH, UI_DEMO_HEIGHT};
use rust_pixel::{
    context::Context,
    game::Render,
    render::scene::Scene,
};
use log::info;

pub struct UiDemoRender {
    pub scene: Scene,
}

impl UiDemoRender {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
        }
    }
}

impl Render for UiDemoRender {
    type Model = UiDemoModel;

    fn init(&mut self, ctx: &mut Context, _model: &mut UiDemoModel) {
        info!("UI Demo render initialized (terminal mode)");
        // Initialize adapter for large terminal
        ctx.adapter.init(UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16, 1.0, 1.0, String::new());
        // Initialize the scene to cover the full screen
        self.scene.init(ctx);
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
        // Clear the TUI buffer
        let buffer = self.scene.tui_buffer_mut();
        buffer.reset();

        // Render UI directly into the TUI buffer
        let _ = model.ui_app.render_into(buffer);

        // Draw to screen
        let _ = self.scene.draw(ctx);
    }
}