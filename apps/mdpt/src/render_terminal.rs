use crate::model::{MdptModel, MDPTH, MDPTW};
use rust_pixel::{
    context::Context,
    game::Render,
    render::scene::Scene,
    render::style::Color,
};

pub struct MdptRender {
    pub scene: Scene,
}

impl MdptRender {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
        }
    }
}

impl Render for MdptRender {
    type Model = MdptModel;

    fn init(&mut self, context: &mut Context, _data: &mut Self::Model) {
        context
            .adapter
            .init(MDPTW, MDPTH, 0.5, 0.5, "mdpt".to_string());
        self.scene.init(context);
    }

    fn handle_event(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        // Get rendered buffer from model (handles transitions)
        let source_buffer = data.get_rendered_buffer();

        // Copy to TUI buffer
        let tui_buffer = self.scene.tui_buffer_mut();
        tui_buffer.reset();
        tui_buffer.merge(source_buffer, 255, true);

        // Draw status bar on the last row
        if data.show_status_bar {
            let info = format!(
                " mdpt | slide {}/{} | step {}/{} | {}",
                data.current_slide + 1,
                data.total_slides().max(1),
                data.current_step + 1,
                data.current_step_count().max(1),
                if data.md_file.is_empty() {
                    "demo.md"
                } else {
                    &data.md_file
                }
            );
            let status_y = MDPTH - 1;
            let status_bg = " ".repeat(MDPTW as usize);
            tui_buffer.set_color_str(0, status_y, &status_bg, Color::White, Color::DarkGray);
            tui_buffer.set_color_str(0, status_y, &info, Color::White, Color::DarkGray);
        }

        self.scene.draw(ctx).unwrap();
    }
}
