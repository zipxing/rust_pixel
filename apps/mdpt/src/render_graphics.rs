use crate::model::MdptModel;
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

    fn init(&mut self, context: &mut Context, data: &mut Self::Model) {
        // Enable TUI character height mode for UI components
        context.adapter.get_base().gr.set_use_tui_height(true);

        // Use frontmatter dimensions (parsed in Model::init before Render::init)
        let w = data.presentation.front_matter.width;
        let h = data.presentation.front_matter.height;

        let scale = if rust_pixel::init::get_game_config().fullscreen {
            1.0
        } else {
            2.0
        };
        context
            .adapter
            .init(w, h, scale, scale, "mdpt".to_string());
        self.scene.init(context);

        // Pre-create image sprite (hidden by default)
        // Use a large buffer to accommodate SSF assets without clipping
        let mut sprite = Sprite::new(0, 0, w, 50u16.max(h));
        sprite.set_hidden(true);
        self.scene.add_sprite(sprite, IMAGE_TAG);

        // NOTE: RT3 visibility is managed dynamically in draw_normal() and draw_gpu_transition()
        // Do NOT set RT3 visible here - on Windows Vulkan, uninitialized GPU memory causes white patches
        // RT3 will be set visible only when GPU transition is active

        // Enable CAS (Contrast Adaptive Sharpening) for crisp text on high-DPI
        // context.adapter.set_sharpness(0.4);
    }

    fn handle_event(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        if data.transition.active && data.use_gpu_transition {
            self.draw_gpu_transition(ctx, data);
        } else {
            self.draw_normal(ctx, data);
        }
    }
}

impl MdptRender {
    /// Normal rendering: current page → tui_buffer → scene.draw()
    fn draw_normal(&mut self, ctx: &mut Context, data: &mut MdptModel) {
        // Hide RT3 during normal rendering to prevent stale transition overlay
        ctx.adapter.set_rt_visible(3, false);

        let source_buffer = data.get_rendered_buffer();

        // Copy to TUI buffer
        let tui_buffer = self.scene.tui_buffer_mut();
        tui_buffer.reset();
        tui_buffer.merge(source_buffer, 255, true);

        // Draw status bar or minimal page number
        if data.show_status_bar {
            self.draw_status_bar(data);
        } else {
            self.draw_page_number(data);
        }

        // Handle image sprite
        let sprite = self.scene.get_sprite(IMAGE_TAG);
        if let Some(placement) = data.image_placements.first() {
            sprite.set_hidden(false);
            sprite.set_cell_pos(placement.x, placement.y);
            let frame_idx = (ctx.stage / 3) as usize;
            asset2sprite!(sprite, ctx, &placement.path, frame_idx);
        } else {
            sprite.set_hidden(true);
        }

        self.scene.draw(ctx).unwrap();
    }

    /// GPU transition rendering:
    /// prev_page → RT0, current_page → RT1, blend → RT2, present fullscreen
    fn draw_gpu_transition(&mut self, ctx: &mut Context, data: &mut MdptModel) {
        // Hide RT3 — we blend directly into RT2 which present_default()
        // renders fullscreen without ratio scaling
        ctx.adapter.set_rt_visible(3, false);

        // Step 1: Render prev page buffer to RT0
        if let Some(prev) = &mut data.prev_page {
            let _ = prev.render();
            let prev_buf = prev.buffer();
            let mut rbuf = vec![];
            ctx.adapter.buf2rbuf(prev_buf, &mut rbuf, true, 255, 1.0, 1.0, 0.0);
            ctx.adapter.rbuf2rt(&rbuf, 0, false);
        }

        // Step 2: Render current page buffer to RT1
        if let Some(curr) = &mut data.current_page {
            let _ = curr.render();
            let curr_buf = curr.buffer();
            let mut rbuf = vec![];
            ctx.adapter.buf2rbuf(curr_buf, &mut rbuf, true, 255, 1.0, 1.0, 0.0);
            ctx.adapter.rbuf2rt(&rbuf, 1, false);
        }

        // Hide image sprite during transitions
        let sprite = self.scene.get_sprite(IMAGE_TAG);
        sprite.set_hidden(true);

        // Step 3: Render empty scene to RT2 (needed for internal buffer swap)
        let tui_buffer = self.scene.tui_buffer_mut();
        tui_buffer.reset();
        self.scene.draw_to_rt(ctx).unwrap();

        // Step 4: GPU shader blend RT0 + RT1 → RT2 (overwrite RT2)
        // Blending to RT2 instead of RT3 because present_default() renders
        // RT2 fullscreen [0,0,1,1] without ratio scaling, while RT3 gets
        // viewport scaled by 1/ratio (causing ~83% size on web)
        ctx.adapter.blend_rts(
            0, 1, 2,
            data.gpu_effect.effect_type(),
            data.gpu_effect.progress,
        );

        // Step 5: Present — RT2 fullscreen (contains transition result)
        ctx.adapter.present_default();
    }

    /// Draw minimal page number in the bottom-right corner
    fn draw_page_number(&mut self, data: &MdptModel) {
        let page_info = format!(
            "{}/{}",
            data.current_slide + 1,
            data.total_slides().max(1),
        );
        let w = data.presentation.front_matter.width;
        let h = data.presentation.front_matter.height;
        let x = w.saturating_sub(page_info.len() as u16 + 1);
        let y = h - 1;
        let tui_buffer = self.scene.tui_buffer_mut();
        let fg = Color::Rgba(100, 100, 100, 255);
        let bg = Color::Reset;
        tui_buffer.set_color_str(x, y, &page_info, fg, bg);
    }

    /// Draw status bar on the last row of tui_buffer
    fn draw_status_bar(&mut self, data: &MdptModel) {
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
        let tui_buffer = self.scene.tui_buffer_mut();
        let status_fg = Color::Rgba(140, 140, 140, 255);
        let status_bg_color = Color::Rgba(30, 30, 35, 255);
        tui_buffer.set_color_str(0, status_y, &status_bg, status_fg, status_bg_color);
        tui_buffer.set_color_str(0, status_y, &info, status_fg, status_bg_color);
    }
}
