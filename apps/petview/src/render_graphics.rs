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
use crate::model::{PetviewModel, PetviewState, PETH, PETW, IMAGES_PER_SCREEN};
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
        style::{Color, Modifier, Style},
    },
    util::{ARect, Rect},
    LOGO_FRAME,
};

// Single image dimensions
const IMG_W: u16 = 40;
const IMG_H: u16 = 25;

// 2x2 grid layout - each image scaled 0.5, total display = 40x25 (same as original single image)
const GRID_COLS: usize = 2;

// Each cell in the 2x2 grid (after 0.5 scale)
const CELL_W: u16 = (IMG_W as f32 * 0.5) as u16;  // 20
const CELL_H: u16 = (IMG_H as f32 * 0.5) as u16;  // 12 (actually 12.5, but using 12)

// Display dimensions (same as original single image area)
const PIXW: u16 = 40;

// Sprite scale factor for GPU scaling
const IMG_SCALE: f32 = 0.5;

// Frame border dimensions (original 52x32 canvas)
const FRAME_LEFT: u16 = 6;    // Left border width
const FRAME_TOP: u16 = 3;     // Top border height
const FRAME_RIGHT: u16 = 6;   // Right border width
const FRAME_BOTTOM: u16 = 4;  // Bottom border height (includes info bar)

// Frame colors
const FRAME_COLOR: Color = Color::Rgba(0x33, 0x33, 0x33, 255);  // Dark gray
const BACK_COLOR: Color = Color::Rgba(5, 5, 5, 1);
const FOOT_COLOR: Color = Color::Rgba(80, 80, 80, 255);
// const INFO_COLOR: Color = Color::Rgba(100, 100, 100, 255);       // Light gray for info text

// Matrix rain: PETSCII graphic chars (block 1) for digital rain effect
const RAIN_BLOCK: u8 = 1;
const RAIN_CHARS: [u8; 20] = [
    65, 81, 83, 90, 88,  // ♠ ● ♥ ♣ +cross
    66, 78, 69, 70, 76,  // vbar quarter arc-UL arc-UR slash
    77, 79, 84, 86, 85,  // backslash 3quarter hbar-top pi corner-TL
    73, 74, 75, 67, 87,  // corner-TR corner-BL corner-BR hbar-B checker
];

// Sprite scale factors for PETSCII chars (16×16px)
const TITLE_SCALE: f32 = 1.0;
const FOOTER_SCALE: f32 = 0.62;
const FRAME_SCALE: f32 = 0.5;  // Frame sprite scale: 0.5 → 2x buffer → denser rain

// Matrix rain animation parameters
const RAIN_SPEED_BASE: f32 = 0.15;      // Base falling speed
const RAIN_SPEED_VARIANCE: f32 = 0.30;  // Speed variance (total speed = base + rand * variance)
const RAIN_TRAIL_MIN: i32 = 8;          // Minimum trail length
const RAIN_TRAIL_VARIANCE: i32 = 11;    // Trail length variance
const RAIN_GAP_MIN: i32 = 8;            // Minimum gap between trails
const RAIN_GAP_VARIANCE: i32 = 8;       // Gap variance
const RAIN_CHAR_CHANGE_FRAMES: usize = 6;  // Character changes every N frames

// Matrix rain colors - HEAD (brightest, with GLOW effect)
const RAIN_HEAD_R: u8 = 80;
const RAIN_HEAD_G: u8 = 140;
const RAIN_HEAD_B: u8 = 90;

// Matrix rain colors - NEAR HEAD (bright green, fading)
const RAIN_NEAR_G_BASE: f32 = 120.0;     // Base green value for near-head
const RAIN_NEAR_G_FADE: f32 = 60.0;     // Fade amount for near-head
const RAIN_NEAR_B: u8 = 8;              // Blue tint for near-head

// Matrix rain colors - TRAIL (fading dark green)
const RAIN_TRAIL_G_MIN: f32 = 48.0;      // Minimum green (at tail end)
const RAIN_TRAIL_G_RANGE: f32 = 90.0;   // Green range (total = min + brightness * range)

// GLOW effect zone (cells from frame edge where GLOW is disabled)
const RAIN_GLOW_MARGIN: u16 = 4;

/// Load a single image into a sprite
/// Returns true if loaded successfully
fn load_single_image(
    sprite: &mut Sprite,
    ctx: &mut Context,
    img_idx: usize,
    _img_count: usize,
) -> bool {
    // img_idx 0,1,2,3 → files 1.pix, 2.pix, 3.pix, 4.pix
    let filename = format!("{}.pix", img_idx + 1);

    // Build full path like asset2sprite! macro does
    let full_path = format!(
        "{}{}assets{}{}",
        rust_pixel::get_game_config().project_path,
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR,
        filename
    );

    // Trigger loading if not already loaded
    ctx.asset_manager.load(AssetType::ImgPix, &full_path);

    // Copy image content to sprite
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

// Random color for noise effect
const RAND_COLOR: Color = Color::Rgba(155, 55, 155, 255);

/// Apply CPU-based buffer distortion effects for 4-image grid
///
/// TransBuf效果：对当前图片(cur)应用CPU扭曲特效，让它"溶解/崩坏"
/// 然后TransGl接手，从cur过渡到next
///
/// cur_original: TransBuf开始时保存的原始cur内容，每帧从这里读取避免特效累积
fn process_buffer_transition_grid(
    scene: &mut Scene,
    cur_original: &[Buffer],
    ctx: &mut Context,
    transbuf_stage: usize,
) {
    let time = (ctx.rand.rand() % 300) as f32 / 100.0;
    let params = EffectParams::new(0.5 - time, ctx.stage as usize)
        .with_seed(ctx.rand.rand());

    // 对cur sprites应用CPU特效（从保存的原始内容读取）
    for i in 0..IMAGES_PER_SCREEN {
        if i >= cur_original.len() {
            continue;
        }
        let src_content = &cur_original[i];
        let mut tbuf = src_content.clone();

        // Apply ripple effect
        let ripple = RippleEffect::new(0.05, 10.0);
        ripple.apply(src_content, &mut tbuf, &params);

        // Apply wave effect
        let wave = WaveEffect::new(0.03, 15.0);
        wave.apply(src_content, &mut tbuf, &params);

        // Add noise: a few random cells per frame
        let clen = tbuf.content.len();
        for _ in 0..transbuf_stage / 2 {
            let idx = ctx.rand.rand() as usize % clen;
            let sym = (ctx.rand.rand() % 255) as u8;
            tbuf.content[idx]
                .set_symbol(&cellsym(sym))
                .set_fg(RAND_COLOR);
        }

        // 更新cur sprite显示扭曲内容
        let cur = scene.get_sprite(&format!("img_cur_{}", i));
        cur.content = tbuf;
        cur.set_alpha(255);
        cur.set_hidden(false);
    }

    // next sprites保持隐藏
    for i in 0..IMAGES_PER_SCREEN {
        let next = scene.get_sprite(&format!("img_next_{}", i));
        next.set_hidden(true);
    }
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
    /// 保存TransBuf开始时的原始cur内容，避免特效累积
    pub cur_original: Vec<Buffer>,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Gallery frame border (full screen, rendered first as background)
        // Buffer is 2x size, sprite scale 0.5 → denser Matrix rain
        let frame_w = (PETW as f32 / FRAME_SCALE) as u16;
        let frame_h = (PETH as f32 / FRAME_SCALE) as u16;
        let frame = Sprite::new(0, 0, frame_w, frame_h);
        scene.add_sprite(frame, "frame");

        // 4 current image sprites for 2x2 grid display (each 40x25, scaled 0.5)
        for i in 0..IMAGES_PER_SCREEN {
            let mut sprite = Sprite::new(0, 0, IMG_W, IMG_H);
            sprite.set_hidden(true);
            scene.add_sprite(sprite, &format!("img_cur_{}", i));
        }

        // 4 next image sprites for transition (each 40x25, scaled 0.5)
        for i in 0..IMAGES_PER_SCREEN {
            let mut sprite = Sprite::new(0, 0, IMG_W, IMG_H);
            sprite.set_hidden(true);
            scene.add_sprite(sprite, &format!("img_next_{}", i));
        }

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
            cur_original: Vec::new(),
        }
    }

    /// Draw the gallery frame border with gold styling
    fn draw_frame(&mut self, img_cur: usize, img_count: usize, stage: u32) {
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

        // Internal dimensions (2x due to FRAME_SCALE=0.5)
        let iw = (PETW as f32 / FRAME_SCALE) as u16;
        let ih = (PETH as f32 / FRAME_SCALE) as u16;
        let fl = (FRAME_LEFT as f32 / FRAME_SCALE) as u16;
        let ft = (FRAME_TOP as f32 / FRAME_SCALE) as u16;
        let fr = (FRAME_RIGHT as f32 / FRAME_SCALE) as u16;
        let fb = (FRAME_BOTTOM as f32 / FRAME_SCALE) as u16;

        // First pass: fill all non-image cells with dark background
        for y in 0..ih {
            for x in 0..iw {
                let in_image_area = x >= fl && x < iw - fr
                    && y >= ft && y < ih - fb;
                if !in_image_area {
                    buf.set_graph_sym(x, y, 0, fill, BACK_COLOR);
                }
            }
        }

        // Second pass: Matrix digital rain columns
        let rain_len = RAIN_CHARS.len();
        for x in 0..iw {
            let xh = x as usize;
            // Deterministic per-column parameters from hash
            let speed = RAIN_SPEED_BASE + (((xh.wrapping_mul(73).wrapping_add(17)) % 31) as f32 / 31.0) * RAIN_SPEED_VARIANCE;
            let trail = RAIN_TRAIL_MIN + ((xh.wrapping_mul(37).wrapping_add(7)) % RAIN_TRAIL_VARIANCE as usize) as i32;
            let gap = RAIN_GAP_MIN + ((xh.wrapping_mul(53).wrapping_add(23)) % RAIN_GAP_VARIANCE as usize) as i32;
            let offset = ((xh.wrapping_mul(97).wrapping_add(41)) % 200) as f32;
            let cycle_len = ih as i32 + trail + gap;

            let y_head_f = (stage as f32 * speed + offset) % (cycle_len as f32);
            let y_head = y_head_f as i32;

            for dy in 0..trail {
                let y = y_head - dy;
                if y < 0 || y >= ih as i32 {
                    continue;
                }
                let yu = y as u16;

                // Skip cells inside the image area
                let in_image_area = x >= fl && x < iw - fr
                    && yu >= ft && yu < ih - fb;
                if in_image_area {
                    continue;
                }

                // Character: changes every N frames, varies by position
                let char_seed = xh.wrapping_mul(31)
                    .wrapping_add((y_head - dy) as usize * 17)
                    .wrapping_add((stage as usize) / RAIN_CHAR_CHANGE_FRAMES);
                let ci = char_seed % rain_len;
                let sym = RAIN_CHARS[ci];

                let t = dy as f32 / trail as f32; // 0.0 at head, ~1.0 at tail

                if dy == 0 {
                    // Head: brightest with GLOW effect (disabled near frame)
                    let near_frame = x >= fl - RAIN_GLOW_MARGIN && x <= iw - fr + 1
                        && yu >= ft - RAIN_GLOW_MARGIN && yu <= ih - fb + 1;
                    let head_color = Color::Rgba(RAIN_HEAD_R, RAIN_HEAD_G, RAIN_HEAD_B, 255);
                    if near_frame {
                        buf.set_graph_sym(x, yu, RAIN_BLOCK, sym, head_color);
                    } else {
                        buf.set_str_tex(
                            x, yu,
                            &cellsym(sym),
                            Style::default().fg(head_color).bg(Color::Reset)
                                .add_modifier(Modifier::GLOW),
                            RAIN_BLOCK,
                        );
                    }
                } else if dy <= 2 {
                    // Near-head: bright green, slight fade
                    let g = (RAIN_NEAR_G_BASE - t * RAIN_NEAR_G_FADE) as u8;
                    buf.set_graph_sym(x, yu, RAIN_BLOCK, sym, Color::Rgba(0, g, RAIN_NEAR_B, 255));
                } else {
                    // Trail body: fading dark green
                    let brightness = 1.0 - t;
                    let g = (RAIN_TRAIL_G_MIN + brightness * RAIN_TRAIL_G_RANGE) as u8;
                    buf.set_graph_sym(x, yu, RAIN_BLOCK, sym, Color::Rgba(0, g, 0, 255));
                }
            }
        }

        // Draw gold frame border around image area
        let left = fl - 1;
        let right = iw - fr;
        let top = ft - 1;
        let bottom = ih - fb;

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

        // Set frame sprite scale (0.5 → 2x buffer density for Matrix rain)
        let frame = self.scene.get_sprite("frame");
        frame.set_scale_x(FRAME_SCALE);
        frame.set_scale_y(FRAME_SCALE);

        // Frame border position in pixels
        let frame_x = (FRAME_LEFT as f32 * sym_w / rx) as u16;
        let frame_y = (FRAME_TOP as f32 * sym_h / ry) as u16;

        // Position 4 current and 4 next image sprites in 2x2 grid
        // Each sprite is 40x25, scaled 0.5 → displays as 20x12.5
        for i in 0..IMAGES_PER_SCREEN {
            let col = (i % GRID_COLS) as u16;
            let row = (i / GRID_COLS) as u16;
            // Cell position within image area (in pixels after scale)
            let cell_x = frame_x + (col * CELL_W as u16 * sym_w as u16 / rx as u16);
            let cell_y = frame_y + (row * CELL_H as u16 * sym_h as u16 / ry as u16);

            let cur_sprite = self.scene.get_sprite(&format!("img_cur_{}", i));
            cur_sprite.set_pos(cell_x, cell_y);
            cur_sprite.set_scale_x(IMG_SCALE);
            cur_sprite.set_scale_y(IMG_SCALE);

            let next_sprite = self.scene.get_sprite(&format!("img_next_{}", i));
            next_sprite.set_pos(cell_x, cell_y);
            next_sprite.set_scale_x(IMG_SCALE);
            next_sprite.set_scale_y(IMG_SCALE);
        }

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
        // Canvas is original 52x32 - 4 images each scaled 0.5 fit in same 40x25 area
        ctx.adapter
            .init(PETW, PETH, 2.0, 2.0, "PETSCII Gallery".to_string());
        self.scene.init(ctx);
        // Images are loaded dynamically in handle_timer
    }

    fn handle_event(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        // Load images into sprites
        if !model.tex_ready {
            if model.show_next_as_cur {
                // 过渡刚完成！"next" sprites 包含刚过渡到的图片
                // 复制 next→cur，然后清除标记
                info!("COPY: next→cur (transition just completed, img_cur={}, img_next={})",
                      model.img_cur, model.img_next);
                for i in 0..IMAGES_PER_SCREEN {
                    let next_content = {
                        let next = self.scene.get_sprite(&format!("img_next_{}", i));
                        next.content.content.clone()
                    };
                    let cur = self.scene.get_sprite(&format!("img_cur_{}", i));
                    cur.content.content.clone_from(&next_content);
                }
                // 清除标记，下一帧进入正常加载流程
                model.show_next_as_cur = false;
                // tex_ready 保持 false，下一帧会加载新的 next sprites
            } else {
                // 正常加载流程
                let mut all_loaded = true;

                let cur_files: Vec<usize> = (0..IMAGES_PER_SCREEN)
                    .map(|i| (model.img_cur + i) % model.img_count + 1)
                    .collect();
                let next_files: Vec<usize> = (0..IMAGES_PER_SCREEN)
                    .map(|i| (model.img_next + i) % model.img_count + 1)
                    .collect();
                info!("LOADING: cur={:?}, next={:?}, img_cur={}, img_next={}",
                      cur_files, next_files, model.img_cur, model.img_next);

                // 加载 cur sprites
                for i in 0..IMAGES_PER_SCREEN {
                    let img_idx = (model.img_cur + i) % model.img_count;
                    let sprite = self.scene.get_sprite(&format!("img_cur_{}", i));
                    if !load_single_image(sprite, ctx, img_idx, model.img_count) {
                        all_loaded = false;
                    }
                }

                // 加载 next sprites
                for i in 0..IMAGES_PER_SCREEN {
                    let img_idx = (model.img_next + i) % model.img_count;
                    let sprite = self.scene.get_sprite(&format!("img_next_{}", i));
                    if !load_single_image(sprite, ctx, img_idx, model.img_count) {
                        all_loaded = false;
                    }
                }

                if all_loaded {
                    model.tex_ready = true;
                    info!("LOADED OK: cur={:?}, next={:?}", cur_files, next_files);
                }
            }
        }

        if event_check("PetView.Timer", "pet_timer") {
            // 使用 RT 系统做过渡：
            // RT0 = cur sprites (4张图片) - 只在Normal状态更新
            // RT1 = next sprites (4张图片) - 只在Normal状态更新
            // RT3 = 混合结果
            // RT2 = frame 背景

            // 只有在 logo 动画后才启用 RT3
            if ctx.stage > LOGO_FRAME {
                ctx.adapter.set_rt_visible(3, true);
            }

            let sym_w = *PIXEL_SYM_WIDTH.get().unwrap_or(&16.0);
            let sym_h = *PIXEL_SYM_HEIGHT.get().unwrap_or(&16.0);
            let rx = ctx.adapter.get_base().gr.ratio_x;
            let ry = ctx.adapter.get_base().gr.ratio_y;
            let cell_px_w = (CELL_W as f32 * sym_w / rx) as u16;
            let cell_px_h = (CELL_H as f32 * sym_h / ry) as u16;

            let state = PetviewState::from_usize(ctx.state as usize).unwrap();
            match state {
                PetviewState::Normal => {
                    info!("STATE: Normal, img_cur={}, img_next={}", model.img_cur, model.img_next);

                    // Normal状态清空cur_original，以便下次TransBuf重新保存
                    self.cur_original.clear();

                    // 隐藏所有图片sprites，通过RT3显示
                    for i in 0..IMAGES_PER_SCREEN {
                        let cur = self.scene.get_sprite(&format!("img_cur_{}", i));
                        cur.set_hidden(true);
                        let next = self.scene.get_sprite(&format!("img_next_{}", i));
                        next.set_hidden(true);
                    }

                    // 只在Normal状态渲染RT0/RT1（保持干净内容）
                    // 渲染 cur sprites 到 RT0
                    {
                        let mut rbuf: Vec<RenderCell> = vec![];
                        for i in 0..IMAGES_PER_SCREEN {
                            let col = (i % GRID_COLS) as u16;
                            let row = (i / GRID_COLS) as u16;
                            let px_x = col * cell_px_w;
                            let px_y = row * cell_px_h;

                            let sprite = self.scene.get_sprite(&format!("img_cur_{}", i));
                            let scale_x = sprite.scale_x;
                            let scale_y = sprite.scale_y;
                            let alpha = sprite.alpha;

                            let mut buf_clone = sprite.content.clone();
                            buf_clone.area.x = px_x;
                            buf_clone.area.y = px_y;

                            ctx.adapter.buf2rbuf(&buf_clone, &mut rbuf, false, alpha, scale_x, scale_y, 0.0);
                        }
                        ctx.adapter.rbuf2rt(&rbuf, 0, false);
                    }

                    // 渲染 next sprites 到 RT1
                    {
                        let mut rbuf: Vec<RenderCell> = vec![];
                        for i in 0..IMAGES_PER_SCREEN {
                            let col = (i % GRID_COLS) as u16;
                            let row = (i / GRID_COLS) as u16;
                            let px_x = col * cell_px_w;
                            let px_y = row * cell_px_h;

                            let sprite = self.scene.get_sprite(&format!("img_next_{}", i));
                            let scale_x = sprite.scale_x;
                            let scale_y = sprite.scale_y;
                            let alpha = sprite.alpha;

                            let mut buf_clone = sprite.content.clone();
                            buf_clone.area.x = px_x;
                            buf_clone.area.y = px_y;

                            ctx.adapter.buf2rbuf(&buf_clone, &mut rbuf, false, alpha, scale_x, scale_y, 0.0);
                        }
                        ctx.adapter.rbuf2rt(&rbuf, 1, false);
                    }

                    // Normal: 显示 RT0 (cur sprites)
                    ctx.adapter.copy_rt(0, 3);
                }
                PetviewState::TransBuf => {
                    info!("STATE: TransBuf, stage={}, cur→next: {}→{}",
                          model.transbuf_stage, model.img_cur, model.img_next);
                    // TransBuf: 隐藏 RT3，使用 CPU distortion 特效（直接显示sprite）
                    ctx.adapter.set_rt_visible(3, false);

                    // TransBuf开始时保存原始cur内容（只保存一次）
                    if self.cur_original.is_empty() {
                        for i in 0..IMAGES_PER_SCREEN {
                            let cur = self.scene.get_sprite(&format!("img_cur_{}", i));
                            self.cur_original.push(cur.content.clone());
                        }
                    }

                    // 应用 CPU distortion 特效到cur sprites（从保存的原始内容读取）
                    process_buffer_transition_grid(
                        &mut self.scene,
                        &self.cur_original,
                        ctx,
                        model.transbuf_stage as usize,
                    );
                }
                PetviewState::TransGl => {
                    info!("STATE: TransGl, progress={:.2}, cur→next: {}→{}",
                          model.gpu_effect.progress, model.img_cur, model.img_next);

                    // 隐藏所有sprites，通过RT3显示
                    for i in 0..IMAGES_PER_SCREEN {
                        let cur = self.scene.get_sprite(&format!("img_cur_{}", i));
                        cur.set_hidden(true);
                        let next = self.scene.get_sprite(&format!("img_next_{}", i));
                        next.set_hidden(true);
                    }

                    // TransGl: GPU shader 混合 (从 RT0/cur 到 RT1/next)
                    // RT0/RT1 在Normal状态已渲染好，保持干净内容
                    ctx.adapter.blend_rts(
                        0, 1, 3,
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

        // Draw gallery frame border with current image info
        self.draw_frame(model.img_cur, model.img_count, ctx.stage);

        // Draw footer
        {
            let footer = self.scene.get_sprite("footer");
            let buf = &mut footer.content;
            buf.reset();
            let line1 = "PETSCII ARTS RETRO C64";
            let line2 = "https://github.com/zipxing/rust_pixel";
            let line3 = "";
            let footer_color = FOOT_COLOR;
            let internal_w = (PETW as f32 / FOOTER_SCALE) as usize;
            let x1 = ((internal_w - line1.len()) / 2) as u16;
            let x2 = ((internal_w - line2.len()) / 2) as u16;
            let x3 = ((internal_w - line3.len()) / 2) as u16;
            buf.set_color_str(x1, 0, line1, footer_color, Color::Reset);
            buf.set_color_str(x2, 1, line2, footer_color, Color::Reset);
            buf.set_color_str(x3, 2, line3, footer_color, Color::Reset);
        }

        // 渲染 frame + footer 到 RT2 (图片 sprites 已隐藏)
        self.scene.draw_to_rt(ctx).unwrap();

        // Logo 动画期间只显示 RT2
        if ctx.stage <= LOGO_FRAME {
            ctx.adapter.present(&[RtComposite::fullscreen(2)]);
            return;
        }

        // RT3 显示在图片区域 (FRAME_LEFT, FRAME_TOP 位置)
        let sym_w = *PIXEL_SYM_WIDTH.get().unwrap_or(&16.0);
        let sym_h = *PIXEL_SYM_HEIGHT.get().unwrap_or(&16.0);
        let rx = ctx.adapter.get_base().gr.ratio_x;
        let ry = ctx.adapter.get_base().gr.ratio_y;

        // viewport 位置：图片区域左上角
        let vp_x = (FRAME_LEFT as f32 * sym_w / rx) as i32;
        let vp_y = (FRAME_TOP as f32 * sym_h / ry) as i32;
        // viewport 大小：40x25 单元格
        let vp_w = (PIXW as f32 * sym_w / rx) as u32;
        let vp_h = (IMG_H as f32 * sym_h / ry) as u32;

        let rt3 = RtComposite::with_viewport(3, ARect { x: vp_x, y: vp_y, w: vp_w, h: vp_h });
        ctx.adapter.present(&[RtComposite::fullscreen(2), rt3]);
    }
}
