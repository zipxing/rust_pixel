use crate::model::MdptModel;
use rust_pixel::{
    context::Context,
    game::Render,
    render::scene::Scene,
    render::style::Color,
};

#[cfg(graphics_mode)]
use rust_pixel::{asset::AssetType, asset2sprite, render::sprite::Sprite};

#[cfg(graphics_mode)]
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
        #[cfg(graphics_mode)]
        context.adapter.get_base().gr.set_use_tui_height(true);

        let w = data.presentation.front_matter.width;
        let h = data.presentation.front_matter.height;

        #[cfg(graphics_mode)]
        let scale = if rust_pixel::init::get_game_config().fullscreen {
            1.0
        } else {
            2.0
        };
        #[cfg(not(graphics_mode))]
        let scale = 0.5;

        context
            .adapter
            .init(w, h, scale, scale, "mdpt".to_string());
        self.scene.init(context);

        #[cfg(graphics_mode)]
        {
            // Pre-create image sprite (hidden by default)
            let mut sprite = Sprite::new(0, 0, w, 50u16.max(h));
            sprite.set_hidden(true);
            self.scene.add_sprite(sprite, IMAGE_TAG);
        }
    }

    fn handle_event(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        #[cfg(graphics_mode)]
        {
            if data.transition.active && data.use_gpu_transition {
                self.draw_gpu_transition(ctx, data);
                return;
            }
        }
        self.draw_normal(ctx, data);
    }
}

impl MdptRender {
    /// Normal rendering: current page → tui_buffer → scene.draw()
    fn draw_normal(&mut self, ctx: &mut Context, data: &mut MdptModel) {
        #[cfg(graphics_mode)]
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

        // Handle image sprite (graphics mode only)
        #[cfg(graphics_mode)]
        {
            let sprite = self.scene.get_sprite(IMAGE_TAG);
            if let Some(placement) = data.image_placements.first() {
                sprite.set_hidden(false);
                sprite.set_cell_pos(placement.x, placement.y);
                let frame_idx = (ctx.stage / 3) as usize;
                asset2sprite!(sprite, ctx, &placement.path, frame_idx);
            } else {
                sprite.set_hidden(true);
            }
        }

        self.scene.draw(ctx).unwrap();
    }

    /// GPU transition rendering (graphics mode only)
    #[cfg(graphics_mode)]
    fn draw_gpu_transition(&mut self, ctx: &mut Context, data: &mut MdptModel) {
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

        // Step 3: Render empty scene to RT2
        let tui_buffer = self.scene.tui_buffer_mut();
        tui_buffer.reset();
        self.scene.draw_to_rt(ctx).unwrap();

        // Step 4: GPU shader blend RT0 + RT1 → RT2
        ctx.adapter.blend_rts(
            0, 1, 2,
            data.gpu_effect.effect_type(),
            data.gpu_effect.progress,
        );

        // Step 5: Present
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

    /// Draw status bar on the last row
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
