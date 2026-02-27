//! PetView - Custom Rendering Pipeline Demo
//!
//! Demonstrates mixing Scene-based rendering with custom RT operations:
//! - Normal: RT0(cur) → RT3 → display
//! - TransBuf: CPU distortion on cur sprites → direct display
//! - TransGl: blend_rts(RT0, RT1) → RT3 → display

#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PetviewModel, PetviewState, IMAGES_PER_SCREEN, PETH, PETW};
use log::info;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, timer_fire, timer_register},
    game::{Model, Render},
    render::{
        adapter::{Adapter, RenderCell, RtComposite, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH},
        buffer::{Buffer, Borders, BorderType},
        cell::{cellsym, cellsym_block},
        effect::{BufferEffect, EffectParams, RippleEffect, WaveEffect},
        effect::{GpuBlendEffect, GpuTransition},
        scene::Scene,
        sprite::Sprite,
        style::{Color, Modifier, Style},
    },
    util::{ARect, Rect},
    LOGO_FRAME,
};

// ============================================================================
// Image & Grid Constants
// ============================================================================
const IMG_W: u16 = 40;
const IMG_H: u16 = 25;
const GRID_COLS: usize = 2;
const PIXW: u16 = 40;
const IMG_SCALE: f32 = 0.5;
const YADJ: u16 = 0; // 图片区域整体下移像素数

// ============================================================================
// Frame Border Constants
// ============================================================================
const FRAME_LEFT: u16 = 6;
const FRAME_TOP: u16 = 3;
const FRAME_RIGHT: u16 = 6;
const FRAME_BOTTOM: u16 = 4;
const FRAME_SCALE: f32 = 0.5;

// Frame colors
const FRAME_COLOR: Color = Color::Rgba(0x33, 0x33, 0x33, 255);
const BACK_COLOR: Color = Color::Rgba(35, 35, 35, 30);
const FOOT_COLOR: Color = Color::Rgba(120, 120, 120, 255);

// ============================================================================
// Matrix Rain Constants
// ============================================================================
const RAIN_BLOCK: u8 = 1;
const RAIN_CHARS: [u8; 20] = [
    65, 81, 83, 90, 88, 66, 78, 69, 70, 76, 77, 79, 84, 86, 85, 73, 74, 75, 67, 87,
];
const RAIN_SPEED_BASE: f32 = 0.15;
const RAIN_SPEED_VARIANCE: f32 = 0.30;
const RAIN_TRAIL_MIN: i32 = 8;
const RAIN_TRAIL_VARIANCE: i32 = 11;
const RAIN_GAP_MIN: i32 = 8;
const RAIN_GAP_VARIANCE: i32 = 8;
const RAIN_CHAR_CHANGE_FRAMES: usize = 6;
const RAIN_HEAD_COLOR: Color = Color::Rgba(80, 140, 90, 255);
const RAIN_NEAR_G_BASE: f32 = 120.0;
const RAIN_NEAR_G_FADE: f32 = 60.0;
const RAIN_NEAR_B: u8 = 8;
const RAIN_TRAIL_G_MIN: f32 = 48.0;
const RAIN_TRAIL_G_RANGE: f32 = 90.0;
const RAIN_GLOW_MARGIN: u16 = 4;

// ============================================================================
// TransBuf Effect Constants
// ============================================================================
const TRANSBUF_NOISE_RATE: usize = 2; // noise count = stage / NOISE_RATE
const RAND_COLOR: Color = Color::Rgba(155, 55, 155, 255);

// ============================================================================
// Footer Constants
// ============================================================================
const FOOTER_SCALE: f32 = 0.62;
const FOOTER_LINE1: &str = "PETSCII ARTS RETRO C64";
const FOOTER_LINE2: &str = "https://github.com/zipxing/rust_pixel";

// ============================================================================
// Cached Pixel Metrics (computed once in do_init)
// ============================================================================
#[derive(Default, Clone, Copy)]
struct PixelMetrics {
    sym_w: f32,
    sym_h: f32,
    rx: f32,
    ry: f32,
    cell_px_w: u16,
    cell_px_h: u16,
    frame_x: u16,
    frame_y: u16,
}

impl PixelMetrics {
    fn compute(ctx: &mut Context) -> Self {
        let sym_w = *PIXEL_SYM_WIDTH.get().unwrap_or(&16.0);
        let sym_h = *PIXEL_SYM_HEIGHT.get().unwrap_or(&16.0);
        let rx = ctx.adapter.get_base().gr.ratio_x;
        let ry = ctx.adapter.get_base().gr.ratio_y;
        Self {
            sym_w,
            sym_h,
            rx,
            ry,
            cell_px_w: (IMG_W as f32 * 0.5 * sym_w / rx) as u16,
            cell_px_h: (IMG_H  as f32 * 0.5 * sym_h / ry) as u16,
            frame_x: (FRAME_LEFT as f32 * sym_w / rx) as u16,
            frame_y: (FRAME_TOP as f32 * sym_h / ry) as u16 - YADJ,
        }
    }

    fn viewport(&self) -> ARect {
        ARect {
            x: self.frame_x as i32,
            y: self.frame_y as i32, // frame_y already includes YADJ
            w: (PIXW as f32 * self.sym_w / self.rx) as u32,
            h: (IMG_H as f32 * self.sym_h / self.ry) as u32,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Load a single image into a sprite. Returns true if loaded successfully.
fn load_single_image(sprite: &mut Sprite, ctx: &mut Context, img_idx: usize) -> bool {
    let filename = format!("{}.pix", img_idx + 1);
    let full_path = format!(
        "{}{}assets{}{}",
        rust_pixel::get_game_config().project_path,
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR,
        filename
    );

    ctx.asset_manager.load(AssetType::ImgPix, &full_path);

    if let Some(asset) = ctx.asset_manager.get(&full_path) {
        let base = asset.get_base();
        if !base.parsed_buffers.is_empty() {
            let pix_buf = &base.parsed_buffers[0];
            let copy_len = pix_buf.content.len().min(sprite.content.content.len());
            sprite.content.content[..copy_len].clone_from_slice(&pix_buf.content[..copy_len]);
            return true;
        }
    }
    false
}

/// Render 4 sprites to a RenderTexture in 2x2 grid layout
fn render_grid_to_rt(
    scene: &mut Scene,
    ctx: &mut Context,
    sprite_prefix: &str,
    rt_index: usize,
    metrics: &PixelMetrics,
) {
    let mut rbuf: Vec<RenderCell> = vec![];
    for i in 0..IMAGES_PER_SCREEN {
        let col = (i % GRID_COLS) as u16;
        let row = (i / GRID_COLS) as u16;
        let px_x = col * metrics.cell_px_w;
        let px_y = row * metrics.cell_px_h;

        let sprite = scene.get_sprite(&format!("{}_{}", sprite_prefix, i));
        let mut buf_clone = sprite.content.clone();
        buf_clone.area.x = px_x;
        buf_clone.area.y = px_y;

        ctx.adapter.buf2rbuf(
            &buf_clone,
            &mut rbuf,
            false,
            sprite.alpha,
            sprite.scale_x,
            sprite.scale_y,
            0.0,
        );
    }
    ctx.adapter.rbuf2rt(&rbuf, rt_index, false);
}

/// Hide all image sprites (cur and next)
fn hide_all_sprites(scene: &mut Scene) {
    for i in 0..IMAGES_PER_SCREEN {
        scene.get_sprite(&format!("img_cur_{}", i)).set_hidden(true);
        scene
            .get_sprite(&format!("img_next_{}", i))
            .set_hidden(true);
    }
}

/// Apply CPU-based buffer distortion effects for 4-image grid
fn process_buffer_transition_grid(
    scene: &mut Scene,
    cur_original: &[Buffer],
    ctx: &mut Context,
    transbuf_stage: usize,
) {
    let time = (ctx.rand.rand() % 300) as f32 / 100.0;
    let params = EffectParams::new(0.5 - time, ctx.stage as usize).with_seed(ctx.rand.rand());

    for i in 0..IMAGES_PER_SCREEN {
        if i >= cur_original.len() {
            continue;
        }
        let src_content = &cur_original[i];
        let mut tbuf = src_content.clone();

        // Apply ripple and wave effects
        RippleEffect::new(0.05, 10.0).apply(src_content, &mut tbuf, &params);
        WaveEffect::new(0.03, 15.0).apply(src_content, &mut tbuf, &params);

        // Add noise
        let clen = tbuf.content.len();
        for _ in 0..transbuf_stage / TRANSBUF_NOISE_RATE {
            let idx = ctx.rand.rand() as usize % clen;
            let sym = (ctx.rand.rand() % 255) as u8;
            tbuf.content[idx]
                .set_symbol(&cellsym(sym))
                .set_fg(RAND_COLOR);
        }

        let cur = scene.get_sprite(&format!("img_cur_{}", i));
        cur.content = tbuf;
        cur.set_alpha(255);
        cur.set_hidden(false);
    }

    // Keep next sprites hidden
    for i in 0..IMAGES_PER_SCREEN {
        scene
            .get_sprite(&format!("img_next_{}", i))
            .set_hidden(true);
    }
}

// ============================================================================
// PetviewRender
// ============================================================================

pub struct PetviewRender {
    pub scene: Scene,
    pub init: bool,
    metrics: PixelMetrics,
    /// Saved original cur content at TransBuf start
    pub cur_original: Vec<Buffer>,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Frame sprite (2x buffer for denser Matrix rain)
        let frame_w = (PETW as f32 / FRAME_SCALE) as u16;
        let frame_h = (PETH as f32 / FRAME_SCALE) as u16;
        scene.add_sprite(Sprite::new(0, 0, frame_w, frame_h), "frame");

        // 4 current + 4 next image sprites
        for i in 0..IMAGES_PER_SCREEN {
            let mut cur = Sprite::new(0, 0, IMG_W, IMG_H);
            cur.set_hidden(true);
            scene.add_sprite(cur, &format!("img_cur_{}", i));

            let mut next = Sprite::new(0, 0, IMG_W, IMG_H);
            next.set_hidden(true);
            scene.add_sprite(next, &format!("img_next_{}", i));
        }

        // Footer sprite
        let footer_w = (PETW as f32 / FOOTER_SCALE) as u16;
        let footer_h = (4.0 / FOOTER_SCALE) as u16;
        scene.add_sprite(Sprite::new(0, 0, footer_w, footer_h), "footer");

        // Border sprite (size = image area + 2 for border lines)
        let border_w = (PIXW as f32 / FRAME_SCALE) as u16 + 2;
        let border_h = (IMG_H as f32 / FRAME_SCALE) as u16 + 2;
        scene.add_sprite(Sprite::new(0, 0, border_w, border_h), "border");

        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self {
            scene,
            init: false,
            metrics: PixelMetrics::default(),
            cur_original: Vec::new(),
        }
    }

    /// Draw the gallery frame border with Matrix rain effect
    fn draw_frame(&mut self, stage: u32) {
        let frame = self.scene.get_sprite("frame");
        let buf = &mut frame.content;
        buf.reset();

        // PETSCII characters
        let fill = 102u8;

        // Internal dimensions (2x due to FRAME_SCALE=0.5)
        let iw = (PETW as f32 / FRAME_SCALE) as u16;
        let ih = (PETH as f32 / FRAME_SCALE) as u16;
        let fl = (FRAME_LEFT as f32 / FRAME_SCALE) as u16;
        let ft = (FRAME_TOP as f32 / FRAME_SCALE) as u16;
        let fr = (FRAME_RIGHT as f32 / FRAME_SCALE) as u16;
        let fb = (FRAME_BOTTOM as f32 / FRAME_SCALE) as u16;

        // Fill background
        for y in 0..ih {
            for x in 0..iw {
                let in_image = x >= fl && x < iw - fr && y >= ft && y < ih - fb;
                if !in_image {
                    buf.set_graph_sym(x, y, 0, fill, BACK_COLOR);
                }
            }
        }

        // Matrix rain
        let rain_len = RAIN_CHARS.len();
        for x in 0..iw {
            let xh = x as usize;
            let speed = RAIN_SPEED_BASE
                + (((xh.wrapping_mul(73).wrapping_add(17)) % 31) as f32 / 31.0)
                    * RAIN_SPEED_VARIANCE;
            let trail = RAIN_TRAIL_MIN
                + ((xh.wrapping_mul(37).wrapping_add(7)) % RAIN_TRAIL_VARIANCE as usize) as i32;
            let gap = RAIN_GAP_MIN
                + ((xh.wrapping_mul(53).wrapping_add(23)) % RAIN_GAP_VARIANCE as usize) as i32;
            let offset = ((xh.wrapping_mul(97).wrapping_add(41)) % 200) as f32;
            let cycle_len = ih as i32 + trail + gap;
            let y_head = ((stage as f32 * speed + offset) % (cycle_len as f32)) as i32;

            for dy in 0..trail {
                let y = y_head - dy;
                if y < 0 || y >= ih as i32 {
                    continue;
                }
                let yu = y as u16;

                let in_image = x >= fl && x < iw - fr && yu >= ft && yu < ih - fb;
                if in_image {
                    continue;
                }

                let char_seed = xh
                    .wrapping_mul(31)
                    .wrapping_add((y_head - dy) as usize * 17)
                    .wrapping_add((stage as usize) / RAIN_CHAR_CHANGE_FRAMES);
                let sym = RAIN_CHARS[char_seed % rain_len];
                let t = dy as f32 / trail as f32;

                if dy == 0 {
                    let near_frame = x >= fl - RAIN_GLOW_MARGIN
                        && x <= iw - fr + 1
                        && yu >= ft - RAIN_GLOW_MARGIN
                        && yu <= ih - fb + 1;
                    if near_frame {
                        buf.set_graph_sym(x, yu, RAIN_BLOCK, sym, RAIN_HEAD_COLOR);
                    } else {
                        buf.set_str(
                            x,
                            yu,
                            cellsym_block(RAIN_BLOCK, sym),
                            Style::default()
                                .fg(RAIN_HEAD_COLOR)
                                .add_modifier(Modifier::GLOW),
                        );
                    }
                } else if dy <= 2 {
                    let g = (RAIN_NEAR_G_BASE - t * RAIN_NEAR_G_FADE) as u8;
                    buf.set_graph_sym(x, yu, RAIN_BLOCK, sym, Color::Rgba(0, g, RAIN_NEAR_B, 255));
                } else {
                    let g = (RAIN_TRAIL_G_MIN + (1.0 - t) * RAIN_TRAIL_G_RANGE) as u8;
                    buf.set_graph_sym(x, yu, RAIN_BLOCK, sym, Color::Rgba(0, g, 0, 255));
                }
            }
        }

        // Draw border using set_border
        let border = self.scene.get_sprite("border");
        border.content.reset();
        border.content.set_border(
            Borders::ALL,
            BorderType::Rounded,
            Style::default().fg(FRAME_COLOR),
        );
    }

    fn draw_footer(&mut self) {
        let footer = self.scene.get_sprite("footer");
        let buf = &mut footer.content;
        buf.reset();

        let w = (PETW as f32 / FOOTER_SCALE) as usize;
        let x1 = ((w - FOOTER_LINE1.len()) / 2) as u16;
        let x2 = ((w - FOOTER_LINE2.len()) / 2) as u16;

        buf.set_petscii_str(x1, 0, FOOTER_LINE1, FOOT_COLOR, Color::Reset);
        buf.set_petscii_str(x2, 1, FOOTER_LINE2, FOOT_COLOR, Color::Reset);
    }

    /// Check if adapter ratios changed (e.g. window maximized/restored)
    fn ratios_changed(&self, ctx: &mut Context) -> bool {
        let rx = ctx.adapter.get_base().gr.ratio_x;
        let ry = ctx.adapter.get_base().gr.ratio_y;
        (self.metrics.rx - rx).abs() > 0.001 || (self.metrics.ry - ry).abs() > 0.001
    }

    /// Compute metrics and position all sprites. Called on init and ratio changes.
    fn update_layout(&mut self, ctx: &mut Context) {
        self.metrics = PixelMetrics::compute(ctx);
        let m = &self.metrics;

        // Set frame sprite scale
        let frame = self.scene.get_sprite("frame");
        frame.set_scale_x(FRAME_SCALE);
        frame.set_scale_y(FRAME_SCALE);

        // Position image sprites in 2x2 grid
        for i in 0..IMAGES_PER_SCREEN {
            let col = (i % GRID_COLS) as u16;
            let row = (i / GRID_COLS) as u16;
            let cell_x = m.frame_x + col * m.cell_px_w;
            let cell_y = m.frame_y + row * m.cell_px_h;

            for prefix in &["img_cur", "img_next"] {
                let sprite = self.scene.get_sprite(&format!("{}_{}", prefix, i));
                sprite.set_pos(cell_x, cell_y);
                sprite.set_scale_x(IMG_SCALE);
                sprite.set_scale_y(IMG_SCALE);
            }
        }

        // Position footer
        let footer_y = ((PETH - FRAME_BOTTOM + 1) as f32 * m.sym_h / m.ry) as u16 + 6;
        let footer = self.scene.get_sprite("footer");
        footer.set_pos(0, footer_y);
        footer.set_scale_x(FOOTER_SCALE);
        footer.set_scale_y(FOOTER_SCALE);

        // Position border sprite (half cell offset due to FRAME_SCALE=0.5)
        let border_x = m.frame_x - (0.5 * m.sym_w / m.rx) as u16;
        let border_y = m.frame_y - (0.5 * m.sym_h / m.ry) as u16;
        let border = self.scene.get_sprite("border");
        border.set_pos(border_x, border_y);
        border.set_scale_x(FRAME_SCALE);
        border.set_scale_y(FRAME_SCALE);
    }

    fn do_init(&mut self, ctx: &mut Context) {
        if self.init {
            // Re-layout if ratios changed (window maximized/restored)
            if self.ratios_changed(ctx) {
                self.update_layout(ctx);
            }
            return;
        }

        self.update_layout(ctx);
        self.init = true;
    }
}

impl Render for PetviewRender {
    type Model = PetviewModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(PETW, PETH, 2.0, 2.0, "PETSCII Gallery".to_string());
        self.scene.init(ctx);
    }

    fn handle_event(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        // Load images
        if !model.tex_ready {
            if model.show_next_as_cur {
                // Copy next → cur after transition completes
                for i in 0..IMAGES_PER_SCREEN {
                    let next_content = self
                        .scene
                        .get_sprite(&format!("img_next_{}", i))
                        .content
                        .content
                        .clone();
                    self.scene
                        .get_sprite(&format!("img_cur_{}", i))
                        .content
                        .content
                        .clone_from(&next_content);
                }
                model.show_next_as_cur = false;
            } else {
                let mut all_loaded = true;

                // 使用打乱后的索引加载图片
                for i in 0..IMAGES_PER_SCREEN {
                    let real_idx = model.get_image_index(model.img_cur + i);
                    let sprite = self.scene.get_sprite(&format!("img_cur_{}", i));
                    if !load_single_image(sprite, ctx, real_idx) {
                        all_loaded = false;
                    }
                }
                for i in 0..IMAGES_PER_SCREEN {
                    let real_idx = model.get_image_index(model.img_next + i);
                    let sprite = self.scene.get_sprite(&format!("img_next_{}", i));
                    if !load_single_image(sprite, ctx, real_idx) {
                        all_loaded = false;
                    }
                }

                if all_loaded {
                    model.tex_ready = true;
                }
            }
        }

        if event_check("PetView.Timer", "pet_timer") {
            if ctx.stage > LOGO_FRAME {
                ctx.adapter.set_rt_visible(3, true);
            }

            let state = PetviewState::from_usize(ctx.state as usize).unwrap();
            match state {
                PetviewState::Normal => {
                    self.cur_original.clear();
                    hide_all_sprites(&mut self.scene);

                    // Render cur/next to RT0/RT1
                    render_grid_to_rt(&mut self.scene, ctx, "img_cur", 0, &self.metrics);
                    render_grid_to_rt(&mut self.scene, ctx, "img_next", 1, &self.metrics);

                    ctx.adapter.copy_rt(0, 3);
                }
                PetviewState::TransBuf => {
                    ctx.adapter.set_rt_visible(3, false);

                    // Save original cur content once
                    if self.cur_original.is_empty() {
                        for i in 0..IMAGES_PER_SCREEN {
                            let cur = self.scene.get_sprite(&format!("img_cur_{}", i));
                            self.cur_original.push(cur.content.clone());
                        }
                    }

                    process_buffer_transition_grid(
                        &mut self.scene,
                        &self.cur_original,
                        ctx,
                        model.transbuf_stage as usize,
                    );
                }
                PetviewState::TransGl => {
                    hide_all_sprites(&mut self.scene);
                    ctx.adapter.blend_rts(
                        0,
                        1,
                        3,
                        model.gpu_effect.effect_type(),
                        model.gpu_effect.progress,
                    );
                }
            }

            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        self.do_init(ctx);

        self.draw_frame(ctx.stage);
        self.draw_footer();

        self.scene.draw_to_rt(ctx).unwrap();

        if ctx.stage <= LOGO_FRAME {
            ctx.adapter.present(&[RtComposite::fullscreen(2)]);
            return;
        }

        let rt3 = RtComposite::with_viewport(3, self.metrics.viewport());
        ctx.adapter.present(&[RtComposite::fullscreen(2), rt3]);
    }
}
