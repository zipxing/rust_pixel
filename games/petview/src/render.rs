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
    render::{adapter::Adapter, buffer::Buffer, panel::Panel, sprite::Sprite, style::Color},
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

        let mut p3 = Sprite::new(96, 40, PIXW, PIXH);
        p3.set_hidden(true);
        panel.add_pixel_sprite(p3, "petimg3");

        let mut p4 = Sprite::new(160u16, 450u16, PIXW, 1u16);
        p4.set_color_str(
            1,
            0,
            "RustPixel - x.com/PETSCIIWORLD",
            Color::Rgba(0, 205, 0, 255),
            Color::Reset,
        );
        panel.add_pixel_sprite(p4, "pet-msg");
        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self { panel }
    }
}

impl Render for PetviewRender {
    type Model = PetviewModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(PETW + 2, PETH, 1.0, 1.0, "petview".to_string());
        self.panel.init(ctx);

        let p1 = self.panel.get_pixel_sprite("petimg1");
        asset2sprite!(p1, ctx, "1.pix");

        let p2 = self.panel.get_pixel_sprite("petimg2");
        asset2sprite!(p2, ctx, "2.pix");
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        if !model.tex_ready {
            if let Some(pix) = &mut ctx.adapter.get_base().gl_pixel {
                pix.set_render_texture_hidden(3, false);
            }
            let p1 = self.panel.get_pixel_sprite("petimg1");
            asset2sprite!(p1, ctx, &format!("{}.pix", model.img_cur + 1));
            let l1 = p1.check_asset_request(&mut ctx.asset_manager);
            if l1 {
                ctx.adapter.draw_buffer_to_texture(&p1.content, 0);
            }

            let p2 = self.panel.get_pixel_sprite("petimg2");
            asset2sprite!(p2, ctx, &format!("{}.pix", model.img_next + 1));
            let l2 = p2.check_asset_request(&mut ctx.asset_manager);
            if l2 {
                ctx.adapter.draw_buffer_to_texture(&p2.content, 1);
            }

            let p3 = self.panel.get_pixel_sprite("petimg3");
            asset2sprite!(p3, ctx, &format!("{}.pix", model.img_next + 1));
            p3.set_hidden(true);

            if l1 && l2 {
                model.tex_ready = true;
                info!("tex_ready.........");
            }
        }
        if event_check("PetView.Timer", "pet_timer") {
            let sa = ctx.adapter.get_base();
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            if let (Some(pix), Some(gl)) = (&mut sa.gl_pixel, &mut sa.gl) {
                pix.bind_target(gl, 3);
                match PetviewState::from_usize(ctx.state as usize).unwrap() {
                    PetviewState::Normal => {
                        pix.set_render_texture_hidden(3, false);
                        let p3 = self.panel.get_pixel_sprite("petimg3");
                        p3.set_hidden(true);
                        pix.render_trans_frame(&gl, 0, 1.0);
                    }
                    PetviewState::TransBuf => {
                        pix.set_render_texture_hidden(3, true);
                        let p3 = self.panel.get_pixel_sprite("petimg3");
                        let cclen = p3.content.content.len();
                        for i in 0..10 {
                            let idx = ctx.rand.rand() as usize % cclen;
                            p3.content.content[idx].fg =
                                Color::Rgba((model.transbuf_stage % 255) as u8, 255, 255, 255);
                            p3.content.content[idx].bg =
                                Color::Rgba((model.transbuf_stage % 255) as u8, 255, 255, 255);
                        }
                        p3.set_hidden(false);
                    }
                    PetviewState::TransGl => {
                        pix.set_render_texture_hidden(3, false);
                        let p3 = self.panel.get_pixel_sprite("petimg3");
                        p3.set_hidden(true);
                        pix.render_trans_frame(&gl, model.trans_effect, model.progress);
                    }
                }
            }
            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
