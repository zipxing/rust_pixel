use crate::model::MdptModel;
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

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        let w = data.presentation.front_matter.width;
        let h = data.presentation.front_matter.height;
        context
            .adapter
            .init(w, h, 0.5, 0.5, "mdpt".to_string());
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

        // Draw status bar or minimal page number
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
            let status_y = data.presentation.front_matter.height - 1;
            let status_bg = " ".repeat(data.presentation.front_matter.width as usize);
            let status_fg = Color::Rgba(140, 140, 140, 255);
            let status_bg_color = Color::Rgba(30, 30, 35, 255);
            tui_buffer.set_color_str(0, status_y, &status_bg, status_fg, status_bg_color);
            tui_buffer.set_color_str(0, status_y, &info, status_fg, status_bg_color);
        } else {
            let page_info = format!(
                "{}/{}",
                data.current_slide + 1,
                data.total_slides().max(1),
            );
            let w = data.presentation.front_matter.width;
            let h = data.presentation.front_matter.height;
            let x = w.saturating_sub(page_info.len() as u16 + 1);
            let y = h - 1;
            let fg = Color::Rgba(100, 100, 100, 255);
            let bg = Color::Reset;
            tui_buffer.set_color_str(x, y, &page_info, fg, bg);
        }

        self.scene.draw(ctx).unwrap();
    }
}
