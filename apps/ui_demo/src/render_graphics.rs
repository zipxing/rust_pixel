use crate::model::{UiDemoModel, UI_DEMO_HEIGHT, UI_DEMO_WIDTH};
use log::info;
use rust_pixel::{
    context::Context,
    game::Render,
    render::{
        adapter::{PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH},
        scene::Scene,
        sprite::Sprite,
        style::Color,
    },
};

const DEMO_TEXT: &str = "PER-CELL SCALE!";

pub struct UiDemoRender {
    pub scene: Scene,
    pub init_done: bool,
}

impl UiDemoRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Per-cell scale demo sprite (one row of text)
        let demo_sp = Sprite::new(0, 0, DEMO_TEXT.len() as u16, 1);
        scene.add_sprite(demo_sp, "scale_demo");

        Self {
            scene,
            init_done: false,
        }
    }

    /// Position demo sprite after adapter initialization
    fn do_init(&mut self, _ctx: &mut Context) {
        if self.init_done {
            return;
        }
        let sym_w = *PIXEL_SYM_WIDTH.get().expect("lazylock init");
        let sym_h = *PIXEL_SYM_HEIGHT.get().expect("lazylock init");

        // Window size in TUI height mode: width * sym_w, height * sym_h * 2
        let window_w = UI_DEMO_WIDTH as f32 * sym_w;
        let window_h = UI_DEMO_HEIGHT as f32 * sym_h * 2.0;

        // Center horizontally, position near bottom
        let text_w = DEMO_TEXT.len() as f32 * sym_w;
        let x = ((window_w - text_w) / 2.0) as u16;
        let y = (window_h - sym_h * 4.0) as u16;

        let demo = self.scene.get_sprite("scale_demo");
        demo.set_pos(x, y);

        self.init_done = true;
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
        self.do_init(ctx);

        // Clear the TUI buffer
        let buffer = self.scene.tui_buffer_mut();
        buffer.reset();

        // Render UI directly into the TUI buffer
        let _ = model.ui_app.render_into(buffer);

        // ===== PPT-style sequential spotlight animation =====
        let text_len = DEMO_TEXT.chars().count();
        // Each letter gets 12 frames (~0.2s) of spotlight time for snappy rhythm
        let frames_per_char = 12usize;
        let cycle_len = text_len * frames_per_char;
        let frame_in_cycle = ctx.stage as usize % cycle_len;
        let active_idx = frame_in_cycle / frames_per_char;
        // Progress within the active letter's animation (0.0 → 1.0)
        let progress = (frame_in_cycle % frames_per_char) as f32 / frames_per_char as f32;
        // Smooth scale: ramp up then back down (sine pulse)
        let active_scale = 1.0 + 0.55 * (progress * std::f32::consts::PI).sin();

        // Base color for idle letters, highlight color for active letter
        let base_color = Color::Rgba(200, 200, 200, 255);
        let highlight_color = Color::Rgba(80, 200, 255, 255);

        let demo = self.scene.get_sprite("scale_demo");
        for (i, ch) in DEMO_TEXT.chars().enumerate() {
            // Access raw content array directly (set_pos changes area.x/y to pixel coords,
            // so get_mut(i, 0) would underflow the coordinate subtraction)
            let cell = &mut demo.content.content[i];
            cell.set_char(ch);
            cell.set_bg(Color::Reset);

            if i == active_idx {
                cell.set_fg(highlight_color);
                cell.set_scale_uniform(active_scale);
            } else {
                cell.set_fg(base_color);
                cell.set_scale_uniform(1.0);
            }
        }
        demo.set_hidden(false);

        // Draw to screen
        let _ = self.scene.draw(ctx);
    }
}
