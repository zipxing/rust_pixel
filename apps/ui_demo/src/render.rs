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
        info!("UI Demo render initialized");

        // Enable TUI character height mode for UI components (graphics mode only)
        #[cfg(graphics_mode)]
        ctx.adapter.get_base().gr.set_use_tui_height(true);

        ctx.adapter.init(
            UI_DEMO_WIDTH as u16,
            UI_DEMO_HEIGHT as u16,
            1.0,
            1.0,
            String::new(),
        );

        self.scene.init(ctx);
    }

    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {}

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        self.update(ctx, model, _dt);
    }

    fn update(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        // Get the current buffer (either single page or transition blend)
        let source_buffer = model.get_rendered_buffer();

        // Copy to TUI buffer
        let tui_buffer = self.scene.tui_buffer_mut();
        tui_buffer.reset();

        // Merge source buffer into TUI buffer
        tui_buffer.merge(source_buffer, 255, true);

        // Draw to screen
        let _ = self.scene.draw(ctx);
    }
}
