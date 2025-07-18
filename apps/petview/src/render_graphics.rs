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
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::{
        adapter::{Adapter, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH},
        buffer::Buffer,
        cell::cellsym,
        panel::Panel,
        sprite::Sprite,
        style::Color,
    },
};

use std::fmt::Write;
use std::io::Cursor;

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
            // 计算归一化坐标（0.0 到 1.0）
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;

            // 应用扭曲函数
            let (du, dv) = distortion_fn(u, v);

            // 将归一化的源坐标转换为实际坐标
            let src_x = (du * width as f32).round() as i32;
            let src_y = (dv * height as f32).round() as i32;

            // 边界处理
            let src_x = src_x.clamp(0, width - 1);
            let src_y = src_y.clamp(0, height - 1);

            // 计算索引
            let src_index = (src_y * width + src_x) as usize;
            let dest_index = (y * width + x) as usize;

            if let (Some(src_cell), Some(dest_cell)) = (
                src_buffer.content.get(src_index),
                dest_buffer.content.get_mut(dest_index),
            ) {
                // 复制源 Cell 到目标 Cell
                *dest_cell = src_cell.clone();
                // 如果需要，可以更新 dest_cell 的其他属性
            }
        }
    }
}

pub struct PetviewRender {
    pub panel: Panel,
    pub init: bool,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        let mut p1 = Sprite::new(0, 0, PIXW, PIXH);
        p1.set_hidden(true);
        panel.add_pixel_sprite(p1, "petimg1");

        let mut p2 = Sprite::new(0, 0, PIXW, PIXH);
        p2.set_hidden(true);
        panel.add_pixel_sprite(p2, "petimg2");

        let mut p3 = Sprite::new(0, 0, PIXW, PIXH);
        p3.set_hidden(true);
        panel.add_pixel_sprite(p3, "petimg3");

        let mut p4 = Sprite::new(0, 0, PIXW, PIXH);
        p4.set_hidden(true);
        panel.add_pixel_sprite(p4, "petimg4");

        let mut p5 = Sprite::new(0, 0, PIXW, 1u16);
        p5.set_color_str(
            1,
            0,
            "RustPixel - x.com/PETSCIIWORLD",
            Color::Rgba(0, 205, 0, 255),
            Color::Reset,
        );
        panel.add_pixel_sprite(p5, "pet-msg");
        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self { panel, init: false }
    }

    // 不能把这个初始化，直接放在init方法里，因为web模式下
    // 资源是异步加载的，那个时候还没有加载到
    // 所以需要每帧去轮询
    fn do_init(&mut self, ctx: &mut Context) {
        if self.init {
            return;
        }

        let rx = ctx.adapter.get_base().gr.ratio_x;
        let ry = ctx.adapter.get_base().gr.ratio_y;
        let p3 = self.panel.get_pixel_sprite("petimg3");
        p3.set_pos(
            (6.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx) as u16,
            (2.5 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry) as u16,
        );
        let p4 = self.panel.get_pixel_sprite("petimg4");
        p4.set_pos(
            (6.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx) as u16,
            (2.5 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry) as u16,
        );
        let pmsg = self.panel.get_pixel_sprite("pet-msg");
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
            .init(PETW + 2, PETH, 0.4, 0.4, "petview".to_string());
        self.panel.init(ctx);

        let p1 = self.panel.get_pixel_sprite("petimg1");
        asset2sprite!(p1, ctx, "1.pix");

        let p2 = self.panel.get_pixel_sprite("petimg2");
        asset2sprite!(p2, ctx, "2.pix");
        // ctx.adapter.only_render_buffer();
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        if !model.tex_ready {
            // Set render texture 3 visible for Winit + Glow mode
            #[cfg(all(feature = "winit", not(feature = "wgpu")))]
            {
                use rust_pixel::render::adapter::winit_glow::WinitGlowAdapter;
                use std::any::Any;

                if let Some(winit_glow_adapter) = ctx.adapter.as_any().downcast_mut::<WinitGlowAdapter>() {
                    if let Some(gl_pixel_renderer) = &mut winit_glow_adapter.gl_pixel_renderer {
                        gl_pixel_renderer.get_gl_pixel_mut().set_render_texture_hidden(3, false);
                    }
                }
            }

            // Set render texture 3 visible for SDL mode
            #[cfg(feature = "sdl")]
            if let Some(pixel_renderer) = &mut ctx.adapter.get_base().gr.pixel_renderer {
                pixel_renderer.set_render_texture_hidden(3, false);
            }

            // Set render texture 3 visible for Web mode
            #[cfg(target_arch = "wasm32")]
            if let Some(pixel_renderer) = &mut ctx.adapter.get_base().gr.pixel_renderer {
                pixel_renderer.set_render_texture_hidden(3, false);
            }

            #[cfg(feature = "wgpu")]
            {
                use rust_pixel::render::adapter::winit_wgpu::WinitWgpuAdapter;
                use std::any::Any;

                if let Some(winit_wgpu_adapter) = ctx.adapter.as_any().downcast_mut::<WinitWgpuAdapter>() {
                    if let Some(wgpu_pixel_renderer) = &mut winit_wgpu_adapter.wgpu_pixel_renderer {
                        wgpu_pixel_renderer.set_render_texture_hidden(3, false);
                    }
                }
            }

            let p1 = self.panel.get_pixel_sprite("petimg1");
            asset2sprite!(p1, ctx, &format!("{}.pix", model.img_count - model.img_cur));
            let l1 = p1.check_asset_request(&mut ctx.asset_manager);

            // Draw to texture for both modes
            if l1 {
                ctx.adapter.draw_buffer_to_texture_dyn(&p1.content, 0);
            }

            let p2 = self.panel.get_pixel_sprite("petimg2");
            asset2sprite!(
                p2,
                ctx,
                &format!("{}.pix", model.img_count - model.img_next)
            );
            let l2 = p2.check_asset_request(&mut ctx.asset_manager);

            // Draw to texture for both modes
            if l2 {
                ctx.adapter.draw_buffer_to_texture_dyn(&p2.content, 1);
            }

            let p3 = self.panel.get_pixel_sprite("petimg3");
            asset2sprite!(
                p3,
                ctx,
                &format!("{}.pix", model.img_count - model.img_next)
            );
            p3.set_hidden(true);

            let p4 = self.panel.get_pixel_sprite("petimg4");
            asset2sprite!(
                p4,
                ctx,
                &format!("{}.pix", model.img_count - model.img_next)
            );
            p4.set_hidden(true);

            if l1 && l2 {
                model.tex_ready = true;
                info!("tex_ready.........");
            }
        }
        if event_check("PetView.Timer", "pet_timer") {
            // Common transition logic for both OpenGL and WGPU modes
            match PetviewState::from_usize(ctx.state as usize).unwrap() {
                PetviewState::Normal => {
                    // OpenGL mode - 直接访问WinitGlowAdapter
                    #[cfg(all(feature = "winit", not(feature = "wgpu")))]
                    {
                        use rust_pixel::render::adapter::winit_glow::WinitGlowAdapter;
                        use std::any::Any;

                        if let Some(winit_glow_adapter) = ctx.adapter.as_any().downcast_mut::<WinitGlowAdapter>() {
                            if let Some(gl_pixel_renderer) = &mut winit_glow_adapter.gl_pixel_renderer {
                                gl_pixel_renderer.render_normal_transition(3);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);
                            }
                        }
                    }

                    // SDL mode (保持原有逻辑)
                    #[cfg(feature = "sdl")]
                    {
                        let sa = ctx.adapter.get_base();
                        if let Some(pixel_renderer) = &mut sa.gr.pixel_renderer {
                            use rust_pixel::render::adapter::gl::pixel::GlPixelRenderer;
                            if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                                gl_pixel_renderer.render_normal_transition(3);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);
                            }
                        }
                    }

                    // Web mode (保持原有逻辑)
                    #[cfg(target_arch = "wasm32")]
                    {
                        let sa = ctx.adapter.get_base();
                        if let Some(pixel_renderer) = &mut sa.gr.pixel_renderer {
                            use rust_pixel::render::adapter::gl::pixel::GlPixelRenderer;
                            if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                                gl_pixel_renderer.render_normal_transition(3);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);
                            }
                        }
                    }

                    // WGPU mode
                    #[cfg(feature = "wgpu")]
                    {
                        use rust_pixel::render::adapter::winit_wgpu::WinitWgpuAdapter;
                        use std::any::Any;

                        if let Some(winit_wgpu_adapter) =
                            ctx.adapter.as_any().downcast_mut::<WinitWgpuAdapter>()
                        {
                            if let Some(wgpu_pixel_renderer) =
                                &mut winit_wgpu_adapter.wgpu_pixel_renderer
                            {
                                wgpu_pixel_renderer.set_render_texture_hidden(3, false);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);

                                // 直接调用WGPU转场渲染 - 使用简化的参数
                                if let Err(e) = winit_wgpu_adapter.render_transition_to_texture_wgpu(3, 0, 1.0) {
                                    eprintln!("WGPU transition error: {}", e);
                                }

                                info!("WGPU: Normal state - render texture 3 visible");
                            }
                        }
                    }
                }
                PetviewState::TransBuf => {
                    // Winit + Glow mode - 直接访问WinitGlowAdapter
                    #[cfg(all(feature = "winit", not(feature = "wgpu")))]
                    {
                        use rust_pixel::render::adapter::winit_glow::WinitGlowAdapter;
                        use std::any::Any;

                        if let Some(winit_glow_adapter) = ctx.adapter.as_any().downcast_mut::<WinitGlowAdapter>() {
                            if let Some(gl_pixel_renderer) = &mut winit_glow_adapter.gl_pixel_renderer {
                                gl_pixel_renderer.setup_transbuf_rendering(3);
                                let p4 = self.panel.get_pixel_sprite("petimg4");
                                let time = (ctx.rand.rand() % 300) as f32 / 100.0;
                                let distortion_fn1 =
                                    |u: f32, v: f32| ripple_distortion(u, v, 0.5 - time, 0.05, 10.0);
                                let mut tbuf = p4.content.clone();
                                let clen = tbuf.content.len();
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn1);
                                let distortion_fn2 =
                                    |u: f32, v: f32| wave_distortion(u, v, 0.5 - time, 0.03, 15.0);
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn2);

                                for _ in 0..model.transbuf_stage / 2 {
                                    tbuf.content[ctx.rand.rand() as usize % clen]
                                        .set_symbol(cellsym((ctx.rand.rand() % 255) as u8))
                                        .set_fg(Color::Rgba(155, 155, 155, 155));
                                }

                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.content = tbuf.clone();
                                p3.set_alpha(
                                    ((0.5 + model.transbuf_stage as f32 / 120.0) * 255.0) as u8,
                                );
                                p3.set_hidden(false);
                            }
                        }
                    }

                    // SDL mode - complex distortion effects
                    #[cfg(feature = "sdl")]
                    {
                        let sa = ctx.adapter.get_base();
                        if let Some(pixel_renderer) = &mut sa.gr.pixel_renderer {
                            use rust_pixel::render::adapter::gl::pixel::GlPixelRenderer;
                            if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                                gl_pixel_renderer.setup_transbuf_rendering(3);
                                let p4 = self.panel.get_pixel_sprite("petimg4");
                                let time = (ctx.rand.rand() % 300) as f32 / 100.0;
                                let distortion_fn1 =
                                    |u: f32, v: f32| ripple_distortion(u, v, 0.5 - time, 0.05, 10.0);
                                let mut tbuf = p4.content.clone();
                                let clen = tbuf.content.len();
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn1);
                                let distortion_fn2 =
                                    |u: f32, v: f32| wave_distortion(u, v, 0.5 - time, 0.03, 15.0);
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn2);

                                for _ in 0..model.transbuf_stage / 2 {
                                    tbuf.content[ctx.rand.rand() as usize % clen]
                                        .set_symbol(cellsym((ctx.rand.rand() % 255) as u8))
                                        .set_fg(Color::Rgba(155, 155, 155, 155));
                                }

                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.content = tbuf.clone();
                                p3.set_alpha(
                                    ((0.5 + model.transbuf_stage as f32 / 120.0) * 255.0) as u8,
                                );
                                p3.set_hidden(false);
                            }
                        }
                    }

                    // Web mode (保持原有逻辑)
                    #[cfg(target_arch = "wasm32")]
                    {
                        let sa = ctx.adapter.get_base();
                        if let Some(pixel_renderer) = &mut sa.gr.pixel_renderer {
                            use rust_pixel::render::adapter::gl::pixel::GlPixelRenderer;
                            if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                                gl_pixel_renderer.setup_transbuf_rendering(3);
                                let p4 = self.panel.get_pixel_sprite("petimg4");
                                let time = (ctx.rand.rand() % 300) as f32 / 100.0;
                                let distortion_fn1 =
                                    |u: f32, v: f32| ripple_distortion(u, v, 0.5 - time, 0.05, 10.0);
                                let mut tbuf = p4.content.clone();
                                let clen = tbuf.content.len();
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn1);
                                let distortion_fn2 =
                                    |u: f32, v: f32| wave_distortion(u, v, 0.5 - time, 0.03, 15.0);
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn2);

                                for _ in 0..model.transbuf_stage / 2 {
                                    tbuf.content[ctx.rand.rand() as usize % clen]
                                        .set_symbol(cellsym((ctx.rand.rand() % 255) as u8))
                                        .set_fg(Color::Rgba(155, 155, 155, 155));
                                }

                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.content = tbuf.clone();
                                p3.set_alpha(
                                    ((0.5 + model.transbuf_stage as f32 / 120.0) * 255.0) as u8,
                                );
                                p3.set_hidden(false);
                            }
                        }
                    }

                    // WGPU mode - simplified preparation phase
                    #[cfg(feature = "wgpu")]
                    {
                        use rust_pixel::render::adapter::winit_wgpu::WinitWgpuAdapter;
                        use std::any::Any;

                        if let Some(winit_wgpu_adapter) =
                            ctx.adapter.as_any().downcast_mut::<WinitWgpuAdapter>()
                        {
                            if let Some(wgpu_pixel_renderer) =
                                &mut winit_wgpu_adapter.wgpu_pixel_renderer
                            {
                                wgpu_pixel_renderer.set_render_texture_hidden(3, true);
                                
                                // Apply the same distortion effects as OpenGL mode
                                let p4 = self.panel.get_pixel_sprite("petimg4");
                                let time = (ctx.rand.rand() % 300) as f32 / 100.0;
                                
                                // Apply ripple distortion
                                let distortion_fn1 =
                                    |u: f32, v: f32| ripple_distortion(u, v, 0.5 - time, 0.05, 10.0);
                                let mut tbuf = p4.content.clone();
                                let clen = tbuf.content.len();
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn1);
                                
                                // Apply wave distortion
                                let distortion_fn2 =
                                    |u: f32, v: f32| wave_distortion(u, v, 0.5 - time, 0.03, 15.0);
                                apply_distortion(&p4.content, &mut tbuf, &distortion_fn2);

                                // Add random noise symbols like OpenGL mode
                                for _ in 0..model.transbuf_stage / 2 {
                                    tbuf.content[ctx.rand.rand() as usize % clen]
                                        .set_symbol(cellsym((ctx.rand.rand() % 255) as u8))
                                        .set_fg(Color::Rgba(155, 155, 155, 155));
                                }

                                // Apply the distorted buffer to p3 sprite
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.content = tbuf.clone();
                                p3.set_alpha(
                                    ((0.5 + model.transbuf_stage as f32 / 120.0) * 255.0) as u8,
                                );
                                p3.set_hidden(false);
                                
                                info!("WGPU: TransBuf stage - applying distortion effects");
                            }
                        }
                    }
                }
                PetviewState::TransGl => {
                    // Winit + Glow mode - 直接访问WinitGlowAdapter
                    #[cfg(all(feature = "winit", not(feature = "wgpu")))]
                    {
                        use rust_pixel::render::adapter::winit_glow::WinitGlowAdapter;
                        use std::any::Any;

                        if let Some(winit_glow_adapter) = ctx.adapter.as_any().downcast_mut::<WinitGlowAdapter>() {
                            if let Some(gl_pixel_renderer) = &mut winit_glow_adapter.gl_pixel_renderer {
                                gl_pixel_renderer.render_gl_transition(3, model.trans_effect, model.progress);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);
                            }
                        }
                    }

                    // SDL mode
                    #[cfg(feature = "sdl")]
                    {
                        let sa = ctx.adapter.get_base();
                        if let Some(pixel_renderer) = &mut sa.gr.pixel_renderer {
                            use rust_pixel::render::adapter::gl::pixel::GlPixelRenderer;
                            if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                                gl_pixel_renderer.render_gl_transition(3, model.trans_effect, model.progress);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);
                            }
                        }
                    }

                    // Web mode (保持原有逻辑)
                    #[cfg(target_arch = "wasm32")]
                    {
                        let sa = ctx.adapter.get_base();
                        if let Some(pixel_renderer) = &mut sa.gr.pixel_renderer {
                            use rust_pixel::render::adapter::gl::pixel::GlPixelRenderer;
                            if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                                gl_pixel_renderer.render_gl_transition(3, model.trans_effect, model.progress);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);
                            }
                        }
                    }

                    // WGPU mode - full transition effects
                    #[cfg(feature = "wgpu")]
                    {
                        use rust_pixel::render::adapter::winit_wgpu::WinitWgpuAdapter;
                        use std::any::Any;

                        if let Some(winit_wgpu_adapter) =
                            ctx.adapter.as_any().downcast_mut::<WinitWgpuAdapter>()
                        {
                            if let Some(wgpu_pixel_renderer) =
                                &mut winit_wgpu_adapter.wgpu_pixel_renderer
                            {
                                wgpu_pixel_renderer.set_render_texture_hidden(3, false);
                                let p3 = self.panel.get_pixel_sprite("petimg3");
                                p3.set_hidden(true);

                                // 直接调用WGPU转场渲染，使用实际的转场效果和进度
                                if let Err(e) = winit_wgpu_adapter.render_transition_to_texture_wgpu(
                                    3,
                                    model.trans_effect,
                                    model.progress,
                                ) {
                                    eprintln!("WGPU transition error: {}", e);
                                }

                                info!(
                                    "WGPU: TransGl - effect: {}, progress: {:.2}",
                                    model.trans_effect, model.progress
                                );
                            }
                        }
                    }
                }
            }

            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, dt: f32) {
        self.do_init(ctx);
        self.panel.draw(ctx).unwrap();
    }
}
