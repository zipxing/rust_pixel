use crate::model::{UiDemoModel, UI_DEMO_HEIGHT, UI_DEMO_WIDTH};
use log::info;
use rust_pixel::{
    context::Context,
    game::Render,
    render::scene::Scene,
};

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
        info!("UI Demo render initialized (graphics mode)");

        // Enable TUI character height mode (32px) for UI components
        ctx.adapter.get_base().gr.set_use_tui_height(true);

        // Initialize adapter for graphics mode
        ctx.adapter.init(
            UI_DEMO_WIDTH as u16,
            UI_DEMO_HEIGHT as u16,
            1.0,
            1.0,
            String::new(),
        );

        // Initialize the scene to cover the full screen
        self.scene.init(ctx);
    }

    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {}

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        self.update(ctx, model, _dt);
    }

    fn update(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        // Clear the TUI buffer
        let buffer = self.scene.tui_buffer_mut();
        buffer.reset();

        // Render UI directly into the TUI buffer.
        // AnimatedLabel handles per-cell scale animation via Style automatically.
        let _ = model.ui_app.render_into(buffer);

        // Draw to screen
        let _ = self.scene.draw(ctx);
    }
}
