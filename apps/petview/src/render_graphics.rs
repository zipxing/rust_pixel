//! PetView - Custom Rendering Pipeline Demo
//!
//! This demonstrates mixing Scene-based rendering with custom present flow:
//! - Scene.draw_to_rt() for rendering scene content to RT2
//! - Direct RT operations (copy_rt, blend_rts) for advanced effects
//! - Custom present() with RtComposite for flexible RT compositing
//!
//! ## Custom Present Flow
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  1. RT operations (copy_rt, blend_rts) prepare RT0/RT1/RT3      │
//! │  2. scene.draw_to_rt() renders sprites to RT2                    │
//! │  3. present([RT2, RT3]) composites and displays to screen        │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PetviewModel, PetviewState, PETH, PETW};
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
        buffer::Buffer,
        cell::cellsym,
        // CPU Effects (Buffer级别)
        effect::{BufferEffect, EffectParams, RippleEffect, WaveEffect},
        // GPU Effects (RenderTexture级别)
        effect::{GpuTransition, GpuBlendEffect},
        scene::Scene,
        sprite::Sprite,
        style::{Color, Style},
    },
    util::{ARect, Rect},
    LOGO_FRAME,
};

const PIXW: u16 = 40;
const PIXH: u16 = 25;

// Frame border dimensions
const FRAME_LEFT: u16 = 6;   // Left border width
const FRAME_TOP: u16 = 3;    // Top border height
const FRAME_RIGHT: u16 = 6;  // Right border width
const FRAME_BOTTOM: u16 = 4; // Bottom border height (includes info bar)

// Frame colors
const FRAME_COLOR: Color = Color::Rgba(0x33, 0x33, 0x33, 255);  // Dark gray
const RAND_COLOR: Color = Color::Rgba(155, 55, 155, 255);
const BACK_COLOR: Color = Color::Rgba(35, 35, 35, 255);
const FOOT_COLOR: Color = Color::Rgba(80, 80, 80, 255);
const INFO_COLOR: Color = Color::Rgba(100, 100, 100, 255);       // Light gray for info text

// Title/Footer scale for PETSCII chars (16×16px)
const TITLE_SCALE: f32 = 1.0;
const FOOTER_SCALE: f32 = 0.62;

/// Apply CPU-based buffer distortion effects
///
/// Effects are applied independently from the original source buffer (not chained),
/// so the wave distortion produces a clean, clearly visible wave pattern.
fn process_buffer_transition(
    scene: &mut Scene,
    ctx: &mut Context,
    transbuf_stage: usize,
) {
    let p4 = scene.get_sprite("petimg4");
    let time = (ctx.rand.rand() % 300) as f32 / 100.0;

    let params = EffectParams::new(0.5 - time, ctx.stage as usize)
        .with_seed(ctx.rand.rand());

    let mut tbuf = p4.content.clone();

    // Apply effects independently from source (not chained through EffectChain).
    // Each distortion reads from p4.content, so wave overwrites ripple and
    // produces a clean, visible wave pattern.
    let ripple = RippleEffect::new(0.05, 10.0);
    ripple.apply(&p4.content, &mut tbuf, &params);

    let wave = WaveEffect::new(0.03, 15.0);
    wave.apply(&p4.content, &mut tbuf, &params);

    // Add noise: a few random cells per frame
    let clen = tbuf.content.len();
    for _ in 0..transbuf_stage / 2 {
        let idx = ctx.rand.rand() as usize % clen;
        let sym = (ctx.rand.rand() % 255) as u8;
        tbuf.content[idx]
            .set_symbol(&cellsym(sym))
            .set_fg(RAND_COLOR);
    }

    // Update p3 sprite with distorted content
    let p3 = scene.get_sprite("petimg3");
    p3.content = tbuf;
    p3.set_alpha(((0.5 + transbuf_stage as f32 / 120.0) * 255.0) as u8);
    p3.set_hidden(false);
}

/// PetView Render - Demonstrates custom rendering with RT primitives
///
/// ## Rendering Pipeline (Custom Present Flow)
/// ```text
/// ┌──────────────────────────────────────────────────────────────────┐
/// │  Normal Mode:                                                     │
/// │    copy_rt(1, 3) → draw_to_rt() → present([RT2, RT3])           │
/// │                                                                   │
/// │  TransBuf Mode (CPU distortion):                                  │
/// │    distort(p4→p3) → draw_to_rt() → present([RT2]) (no RT3)      │
/// │                                                                   │
/// │  TransGl Mode (GPU shader):                                       │
/// │    blend_rts(RT0, RT1, RT3) → draw_to_rt() → present([RT2, RT3])│
/// └──────────────────────────────────────────────────────────────────┘
/// ```
pub struct PetviewRender {
    pub scene: Scene,
    pub init: bool,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Gallery frame border (full screen, rendered first as background)
        let frame = Sprite::new(0, 0, PETW, PETH);
        scene.add_sprite(frame, "frame");

        // p1, p2: source images (hidden, used for RT0/RT1)
        let mut p1 = Sprite::new(0, 0, PIXW, PIXH);
        p1.set_hidden(true);
        scene.add_sprite(p1, "petimg1");

        let mut p2 = Sprite::new(0, 0, PIXW, PIXH);
        p2.set_hidden(true);
        scene.add_sprite(p2, "petimg2");

        // p3: transition display sprite (visible during TransBuf)
        let mut p3 = Sprite::new(0, 0, PIXW, PIXH);
        p3.set_hidden(true);
        scene.add_sprite(p3, "petimg3");

        // p4: source for CPU distortion (hidden)
        let mut p4 = Sprite::new(0, 0, PIXW, PIXH);
        p4.set_hidden(true);
        scene.add_sprite(p4, "petimg4");

        // Message sprite (hidden - replaced by info bar in frame)
        let mut p5 = Sprite::new(0, 0, PIXW, 1u16);
        p5.set_hidden(true);
        scene.add_sprite(p5, "pet-msg");

        // Title sprite (positioned in do_init)
        let title_w = (PETW as f32 / TITLE_SCALE) as u16;
        let title_h = (1.0 / TITLE_SCALE) as u16 + 1;
        let title = Sprite::new(0, 0, title_w, title_h);
        scene.add_sprite(title, "title");

        // Footer sprite (positioned in do_init)
        let footer_w = (PETW as f32 / FOOTER_SCALE) as u16;
        let footer_h = (4.0 / FOOTER_SCALE) as u16;
        let footer = Sprite::new(0, 0, footer_w, footer_h);
        scene.add_sprite(footer, "footer");

        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self {
            scene,
            init: false,
        }
    }

    /// Draw the gallery frame border with gold styling
    fn draw_frame(&mut self, img_cur: usize, img_count: usize) {
        let frame = self.scene.get_sprite("frame");
        let buf = &mut frame.content;
        buf.reset();

        // PETSCII box drawing characters
        // Block 0: square corners and lines (109-125)
        // Block 1: rounded corners (73,74,75,85)
        let corner_block = 1u8;  // Rounded corners are in Block 1
        let corner_tl = 85u8;    // ╭ top-left corner (rounded)
        let corner_tr = 73u8;    // ╮ top-right corner (rounded)
        let corner_bl = 74u8;    // ╰ bottom-left corner (rounded)
        let corner_br = 75u8;    // ╯ bottom-right corner (rounded)
        let line_block = 0u8;    // Lines are in Block 0
        let horiz = 64u8;        // ─ horizontal line
        let vert = 93u8;         // │ vertical line
        let fill = 102u8;        // filled block for border area

        // Draw outer frame background (dark)
        for y in 0..PETH {
            for x in 0..PETW {
                // Check if in border area
                let in_image_area = x >= FRAME_LEFT && x < PETW - FRAME_RIGHT
                    && y >= FRAME_TOP && y < PETH - FRAME_BOTTOM;

                if !in_image_area {
                    buf.set_graph_sym(x, y, 0, fill, BACK_COLOR);
                }
            }
        }

        // Draw gold frame border around image area
        let left = FRAME_LEFT - 1;
        let right = PETW - FRAME_RIGHT;
        let top = FRAME_TOP - 1;
        let bottom = PETH - FRAME_BOTTOM;

        // Top border line
        buf.set_graph_sym(left, top, corner_block, corner_tl, FRAME_COLOR);
        for x in (left + 1)..right {
            buf.set_graph_sym(x, top, line_block, horiz, FRAME_COLOR);
        }
        buf.set_graph_sym(right, top, corner_block, corner_tr, FRAME_COLOR);

        // Bottom border line
        buf.set_graph_sym(left, bottom, corner_block, corner_bl, FRAME_COLOR);
        for x in (left + 1)..right {
            buf.set_graph_sym(x, bottom, line_block, horiz, FRAME_COLOR);
        }
        buf.set_graph_sym(right, bottom, corner_block, corner_br, FRAME_COLOR);

        // Left and right vertical lines
        for y in (top + 1)..bottom {
            buf.set_graph_sym(left, y, line_block, vert, FRAME_COLOR);
            buf.set_graph_sym(right, y, line_block, vert, FRAME_COLOR);
        }

        // Note: Inner shadow removed - it was overlapping with the image area
        // The outer gold border provides sufficient framing effect

        // Title and footer are rendered as separate sprites in draw() method
    }

    fn do_init(&mut self, ctx: &mut Context) {
        if self.init {
            return;
        }

        let rx = ctx.adapter.get_base().gr.ratio_x;
        let ry = ctx.adapter.get_base().gr.ratio_y;
        let sym_w = *PIXEL_SYM_WIDTH.get().expect("lazylock init");
        let sym_h = *PIXEL_SYM_HEIGHT.get().expect("lazylock init");

        // Position p3 and p4 inside the frame border
        let img_x = (FRAME_LEFT as f32 * sym_w / rx) as u16;
        let img_y = (FRAME_TOP as f32 * sym_h / ry) as u16;

        let p3 = self.scene.get_sprite("petimg3");
        p3.set_pos(img_x, img_y);

        let p4 = self.scene.get_sprite("petimg4");
        p4.set_pos(img_x, img_y);

        // Position title sprite above the frame border
        let title_y_sprite = FRAME_TOP - 2;  // one row above frame top border
        let title_y_px = (title_y_sprite as f32 * sym_h / ry) as u16;
        let title = self.scene.get_sprite("title");
        title.set_pos(0, title_y_px - 12);
        title.set_scale_x(TITLE_SCALE);
        title.set_scale_y(TITLE_SCALE);

        // Position footer sprite below the frame border
        let footer_y_sprite = PETH - FRAME_BOTTOM + 1;
        let footer_y_px = (footer_y_sprite as f32 * sym_h / ry) as u16;
        let footer = self.scene.get_sprite("footer");
        footer.set_pos(0, footer_y_px + 6);
        footer.set_scale_x(FOOTER_SCALE);
        footer.set_scale_y(FOOTER_SCALE);

        self.init = true;
    }
}

impl Render for PetviewRender {
    type Model = PetviewModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(PETW, PETH, 2.0, 2.0, "PETSCII Gallery".to_string());
        self.scene.init(ctx);

        let p1 = self.scene.get_sprite("petimg1");
        asset2sprite!(p1, ctx, "1.pix");

        let p2 = self.scene.get_sprite("petimg2");
        asset2sprite!(p2, ctx, "2.pix");
    }

    fn handle_event(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        if !model.tex_ready {
            // Only enable RT3 after logo animation finishes
            // On Windows Vulkan, showing RT3 with garbage content causes white patches
            if ctx.stage > LOGO_FRAME {
                ctx.adapter.set_rt_visible(3, true);
            }

            // Load all 4 sprites at once
            self.scene.with_sprites(&["petimg1", "petimg2", "petimg3", "petimg4"], |sprites| {
                let l1 = asset2sprite!(sprites[0], ctx, &format!("{}.pix", model.img_count - model.img_cur));
                let l2 = asset2sprite!(sprites[1], ctx, &format!("{}.pix", model.img_count - model.img_next));

                if l1 {
                    ctx.adapter.buf2rt(&sprites[0].content, 0);
                }
                if l2 {
                    ctx.adapter.buf2rt(&sprites[1].content, 1);
                }

                // Load distortion sources
                asset2sprite!(sprites[2], ctx, &format!("{}.pix", model.img_count - model.img_next));
                sprites[2].set_hidden(true);

                asset2sprite!(sprites[3], ctx, &format!("{}.pix", model.img_count - model.img_next));
                sprites[3].set_hidden(true);

                if l1 && l2 {
                    model.tex_ready = true;
                    info!("tex_ready.........");
                }
            });
        }

        if event_check("PetView.Timer", "pet_timer") {
            match PetviewState::from_usize(ctx.state as usize).unwrap() {
                PetviewState::Normal => {
                    // Ensure RT3 is visible for this mode
                    ctx.adapter.set_rt_visible(3, true);
                    // RT primitive: copy RT1 to RT3
                    ctx.adapter.copy_rt(1, 3);
                    let p3 = self.scene.get_sprite("petimg3");
                    p3.set_hidden(true);
                }
                PetviewState::TransBuf => {
                    // Hide RT3, use CPU distortion via p3 sprite
                    ctx.adapter.set_rt_visible(3, false);

                    // Apply CPU distortion effect
                    process_buffer_transition(
                        &mut self.scene,
                        ctx,
                        model.transbuf_stage as usize,
                    );
                }
                PetviewState::TransGl => {
                    // Ensure RT3 is visible for this mode
                    ctx.adapter.set_rt_visible(3, true);
                    // RT primitive: GPU shader blend RT0 + RT1 → RT3
                    // 使用新的GpuBlendEffect获取特效类型和进度
                    ctx.adapter.blend_rts(
                        0, 1, 3,
                        model.gpu_effect.effect_type(),
                        model.gpu_effect.progress,
                    );
                    let p3 = self.scene.get_sprite("petimg3");
                    p3.set_hidden(true);
                }
            }

            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        self.do_init(ctx);

        // Draw gallery frame border with current image info
        self.draw_frame(model.img_cur, model.img_count);

        // Draw title
        {
            let title = self.scene.get_sprite("title");
            let buf = &mut title.content;
            buf.reset();
            let text = "PETSCII ARTS RETRO C64";
            let internal_w = (PETW as f32 / TITLE_SCALE) as usize;
            let tx = ((internal_w - text.len()) / 2) as u16;
            buf.set_color_str(tx, 0, text, INFO_COLOR, Color::Reset);
        }

        // Draw footer
        {
            let footer = self.scene.get_sprite("footer");
            let buf = &mut footer.content;
            buf.reset();
            let line2 = "♦ Tile-first. Retro-ready. Write Once, Run Anywhere-2D Engine...";
            let line1 = "";
            let line3 = "https://github.com/zipxing/rust_pixel";
            // Dimmer color for footer (less prominent than title)
            let footer_color = FOOT_COLOR;
            // Internal coordinate space is 1/FOOTER_SCALE times screen space
            // To center: x = (internal_w - text_len) / 2
            let internal_w = (PETW as f32 / FOOTER_SCALE) as usize;
            let x1 = ((internal_w - line1.len()) / 2) as u16;
            let x2 = ((internal_w - line2.len()) / 2) as u16;
            let x3 = ((internal_w - line3.len()) / 2) as u16;
            buf.set_color_str(x1, 0, line1, footer_color, Color::Reset);
            buf.set_color_str(x2, 1, line2, footer_color, Color::Reset);
            buf.set_color_str(x3, 2, line3, footer_color, Color::Reset);
        }

        // Step 1: Render scene content to RT2 (without present)
        self.scene.draw_to_rt(ctx).unwrap();

        // During logo animation, only render RT2 (skip RT3 which may contain garbage)
        // On Windows Vulkan, uninitialized GPU memory can show as white patches
        if ctx.stage <= LOGO_FRAME {
            ctx.adapter.present(&[RtComposite::fullscreen(2)]);
            return;
        }

        // Step 2: Custom present based on state
        let state = PetviewState::from_usize(ctx.state as usize).unwrap();
        match state {
            PetviewState::TransBuf => {
                // TransBuf: RT3 hidden, only show RT2
                ctx.adapter.present(&[RtComposite::fullscreen(2)]);
            }
            PetviewState::Normal | PetviewState::TransGl => {
                // Position RT3 at the frame's image area
                // centered_rt centers a 40×25 area on a 52×32 canvas:
                // - Horizontal: (52-40)/2 = 6 cells = FRAME_LEFT ✓ (already correct)
                // - Vertical: (32-25)/2 = 3.5 cells, but FRAME_TOP = 3 (need -0.5 cell offset)
                let sym_h = *PIXEL_SYM_HEIGHT.get().unwrap_or(&16.0) as i32;
                let dy = -sym_h / 2;  // shift up by 0.5 cell
                let rt3 = ctx.centered_rt(3, PIXW, PIXH).offset(0, dy);
                ctx.adapter.present(&[RtComposite::fullscreen(2), rt3]);
            }
        }
    }
}
