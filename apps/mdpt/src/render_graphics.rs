use crate::model::{MdptModel, MDPTH, MDPTW};
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    game::Render,
    render::scene::Scene,
    render::sprite::Sprite,
    render::style::Color,
};

const IMAGE_TAG: &str = "mdpt_image";

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
        // Enable TUI character height mode for UI components
        context.adapter.get_base().gr.set_use_tui_height(true);

        context
            .adapter
            .init(MDPTW, MDPTH, 1.2, 1.2, "mdpt".to_string());
        self.scene.init(context);

        // Pre-create image sprite (hidden by default)
        // Use a large buffer (80x50) to accommodate SSF assets without clipping
        let mut sprite = Sprite::new(0, 0, 80, 50);
        sprite.set_hidden(true);
        self.scene.add_sprite(sprite, IMAGE_TAG);
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

        // Handle image sprite (SSF/PIX) rendering
        let sprite = self.scene.get_sprite(IMAGE_TAG);
        if let Some(placement) = data.image_placements.first() {
            sprite.set_hidden(false);
            sprite.set_cell_pos(placement.x, placement.y);
            let frame_idx = (ctx.stage / 3) as usize; // ~20fps animation
            asset2sprite!(sprite, ctx, &placement.path, frame_idx);
        } else {
            sprite.set_hidden(true);
        }

        self.scene.draw(ctx).unwrap();
    }
}
