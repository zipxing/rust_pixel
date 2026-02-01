//! PetView - Custom Rendering Pipeline Demo
//!
//! This demonstrates mixing Scene-based rendering with direct primitive usage:
//! - Scene.draw() for standard rendering pipeline
//! - Direct RT operations (copy_rt, blend_rts) for advanced effects
//! - buf2rbuf + rbuf2rt for custom sprite rendering in TransBuf mode

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
        scene::Scene,
        sprite::Sprite,
        style::Color,
    },
};

const PIXW: u16 = 40;
const PIXH: u16 = 25;

fn wave_distortion(x: f32, y: f32, time: f32, amplitude: f32, frequency: f32) -> (f32, f32) {
    let offset_x = x + amplitude * (frequency * y + time).sin();
    let offset_y = y;
    (offset_x, offset_y)
}

fn ripple_distortion(u: f32, v: f32, time: f32, amplitude: f32, frequency: f32) -> (f32, f32) {
    let cx = 0.5;
    let cy = 0.5;
    let dx = u - cx;
    let dy = v - cy;
    let distance = (dx * dx + dy * dy).sqrt();

    let offset = amplitude * (frequency * distance - time).sin();
    let du = u + (dx / distance) * offset;
    let dv = v + (dy / distance) * offset;
    (du, dv)
}

fn apply_distortion(
    src_buffer: &Buffer,
    dest_buffer: &mut Buffer,
    distortion_fn: &dyn Fn(f32, f32) -> (f32, f32),
) {
    let width = src_buffer.area.width as i32;
    let height = src_buffer.area.height as i32;

    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;

            let (du, dv) = distortion_fn(u, v);

            let src_x = (du * width as f32).round() as i32;
            let src_y = (dv * height as f32).round() as i32;

            let src_x = src_x.clamp(0, width - 1);
            let src_y = src_y.clamp(0, height - 1);

            let src_index = (src_y * width + src_x) as usize;
            let dest_index = (y * width + x) as usize;

            if let (Some(src_cell), Some(dest_cell)) = (
                src_buffer.content.get(src_index),
                dest_buffer.content.get_mut(dest_index),
            ) {
                *dest_cell = src_cell.clone();
            }
        }
    }
}

/// Apply CPU-based buffer distortion and render using primitives
fn process_buffer_transition(
    scene: &mut Scene,
    ctx: &mut Context,
    transbuf_stage: usize,
) {
    let p4 = scene.get_sprite("petimg4");
    let time = (ctx.rand.rand() % 300) as f32 / 100.0;

    // Apply ripple distortion
    let distortion_fn1 = |u: f32, v: f32| ripple_distortion(u, v, 0.5 - time, 0.05, 10.0);
    let mut tbuf = p4.content.clone();
    let clen = tbuf.content.len();
    apply_distortion(&p4.content, &mut tbuf, &distortion_fn1);

    // Apply wave distortion
    let distortion_fn2 = |u: f32, v: f32| wave_distortion(u, v, 0.5 - time, 0.03, 15.0);
    apply_distortion(&p4.content, &mut tbuf, &distortion_fn2);

    // Add random noise symbols
    for _ in 0..transbuf_stage / 2 {
        tbuf.content[ctx.rand.rand() as usize % clen]
            .set_symbol(&cellsym((ctx.rand.rand() % 255) as u8))
            .set_fg(Color::Rgba(155, 155, 155, 155));
    }

    // Update p3 sprite with distorted content
    let p3 = scene.get_sprite("petimg3");
    p3.content = tbuf;
    p3.set_alpha(((0.5 + transbuf_stage as f32 / 120.0) * 255.0) as u8);
    p3.set_hidden(false);
}

/// PetView Render - Demonstrates custom rendering with RT primitives
///
/// ## Rendering Pipeline
/// ```text
/// ┌──────────────────────────────────────────────────────────────────┐
/// │  Normal Mode:                                                     │
/// │    copy_rt(1, 3) → scene.draw() (RT3 visible)                    │
/// │                                                                   │
/// │  TransBuf Mode (CPU distortion):                                  │
/// │    distort(p4→p3) → scene.draw() renders p3 with alpha           │
/// │                                                                   │
/// │  TransGl Mode (GPU shader):                                       │
/// │    blend_rts(RT0, RT1, RT3) → scene.draw() (RT3 visible)         │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
pub struct PetviewRender {
    pub scene: Scene,
    pub init: bool,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

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

        // Message sprite
        let mut p5 = Sprite::new(0, 0, PIXW, 1u16);
        p5.set_color_str(
            1,
            0,
            "RustPixel - x.com/PETSCIIWORLD",
            Color::Rgba(0, 205, 0, 255),
            Color::Reset,
        );
        scene.add_sprite(p5, "pet-msg");

        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self { scene, init: false }
    }

    fn do_init(&mut self, ctx: &mut Context) {
        if self.init {
            return;
        }

        let rx = ctx.adapter.get_base().gr.ratio_x;
        let ry = ctx.adapter.get_base().gr.ratio_y;

        let p3 = self.scene.get_sprite("petimg3");
        p3.set_pos(
            (6.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx) as u16,
            (2.5 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry) as u16,
        );

        let p4 = self.scene.get_sprite("petimg4");
        p4.set_pos(
            (6.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx) as u16,
            (2.5 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry) as u16,
        );

        let pmsg = self.scene.get_sprite("pet-msg");
        pmsg.set_pos(
            (10.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx) as u16,
            (28.5 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / rx) as u16,
        );

        self.init = true;
        info!("PETVIEW INIT OK...!!!!");
    }
}

impl Render for PetviewRender {
    type Model = PetviewModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(PETW, PETH, 1.0, 1.0, "petview".to_string());
        self.scene.init(ctx);

        let p1 = self.scene.get_sprite("petimg1");
        asset2sprite!(p1, ctx, "1.pix");

        let p2 = self.scene.get_sprite("petimg2");
        asset2sprite!(p2, ctx, "2.pix");
    }

    fn handle_event(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        if !model.tex_ready {
            // Enable RT3 for transition effects
            ctx.adapter.set_rt_visible(3, true);

            // Load current image to RT0
            let p1 = self.scene.get_sprite("petimg1");
            asset2sprite!(p1, ctx, &format!("{}.pix", model.img_count - model.img_cur));
            let l1 = p1.check_asset_request(&mut ctx.asset_manager);
            if l1 {
                ctx.adapter.buf2rt(&p1.content, 0);
            }

            // Load next image to RT1
            let p2 = self.scene.get_sprite("petimg2");
            asset2sprite!(p2, ctx, &format!("{}.pix", model.img_count - model.img_next));
            let l2 = p2.check_asset_request(&mut ctx.asset_manager);
            if l2 {
                ctx.adapter.buf2rt(&p2.content, 1);
            }

            // Load distortion sources
            let p3 = self.scene.get_sprite("petimg3");
            asset2sprite!(p3, ctx, &format!("{}.pix", model.img_count - model.img_next));
            p3.set_hidden(true);

            let p4 = self.scene.get_sprite("petimg4");
            asset2sprite!(p4, ctx, &format!("{}.pix", model.img_count - model.img_next));
            p4.set_hidden(true);

            if l1 && l2 {
                model.tex_ready = true;
                info!("tex_ready.........");
            }
        }

        if event_check("PetView.Timer", "pet_timer") {
            match PetviewState::from_usize(ctx.state as usize).unwrap() {
                PetviewState::Normal => {
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
                    // RT primitive: GPU shader blend RT0 + RT1 → RT3
                    ctx.adapter.blend_rts(0, 1, 3, model.trans_effect, model.progress);
                    let p3 = self.scene.get_sprite("petimg3");
                    p3.set_hidden(true);
                }
            }

            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {
        self.do_init(ctx);
        // scene.draw() handles: sprite layer → RT2 → present
        self.scene.draw(ctx).unwrap();
    }
}
