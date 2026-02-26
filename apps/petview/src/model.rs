#![allow(dead_code)]
#![allow(unused_variables)]
use rust_pixel::event::Event;
// use log::info;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use petview_lib::PetviewData;
use rust_pixel::{
    context::Context,
    game::Model,
    render::effect::{GpuTransition, GpuBlendEffect},
};

// Gallery mode: 2x2 grid (4 images per screen), each scaled 0.5
// Single image: 40x25 @ scale 0.5 = 20x12.5, Grid: 40x25 total
// Frame border: 6 chars each side
pub const PETW: u16 = 52;  // 40 + 6*2 (left/right border)
pub const PETH: u16 = 32;  // 25 + 4 (top) + 3 (bottom with info)
pub const IMAGES_PER_SCREEN: usize = 4;  // 2x2 grid

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum PetviewState {
    Normal,
    TransBuf,
    TransGl,
}

pub struct PetviewModel {
    pub data: PetviewData,
    pub normal_stage: u32,
    pub transbuf_stage: u32,
    pub img_cur: usize,
    pub img_next: usize,
    pub img_count: usize,
    /// GPU混合特效 (使用新的GpuBlendEffect类型)
    pub gpu_effect: GpuBlendEffect,
    pub tex_ready: bool,
    /// True after transition completes, until new images are loaded
    /// During this time, continue showing "next" sprites to avoid jump-back
    pub show_next_as_cur: bool,
}

impl PetviewModel {
    pub fn new() -> Self {
        Self {
            data: PetviewData::new(),
            normal_stage: 0,
            transbuf_stage: 0,
            img_cur: 0,
            img_next: IMAGES_PER_SCREEN,  // Next group starts at index 4
            img_count: 2099,
            gpu_effect: GpuBlendEffect::default(),
            tex_ready: false,
            show_next_as_cur: false,
        }
    }
}

impl Model for PetviewModel {
    fn init(&mut self, ctx: &mut Context) {
        ctx.state = PetviewState::Normal as u8;
        self.normal_stage = 0;
    }

    fn handle_input(&mut self, ctx: &mut Context, _dt: f32) {
        let es = ctx.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    _ => {
                        ctx.state = PetviewState::Normal as u8;
                    }
                },
                _ => {}
            }
        }
        ctx.input_events.clear();
    }

    fn handle_auto(&mut self, ctx: &mut Context, _dt: f32) {
        let st = PetviewState::from_usize(ctx.state as usize).unwrap();
        match st {
            PetviewState::Normal => {
                self.normal_stage += 1;
                if self.normal_stage > 100 {
                    ctx.state = PetviewState::TransBuf as u8;
                    self.transbuf_stage = 0;
                }
            }
            PetviewState::TransBuf => {
                self.transbuf_stage += 1;
                if self.transbuf_stage > 20 {
                    ctx.state = PetviewState::TransGl as u8;
                    // 使用新的GpuBlendEffect创建随机GPU过渡特效
                    let random_idx = (ctx.rand.rand() % GpuTransition::count() as u32) as usize;
                    self.gpu_effect = GpuBlendEffect::from_index(random_idx, 0.0);
                    self.tex_ready = false;
                }
            }
            PetviewState::TransGl => {
                // 更新GPU特效进度
                self.gpu_effect.progress += 0.01;
                if self.gpu_effect.progress >= 1.0 {
                    ctx.state = PetviewState::Normal as u8;
                    self.normal_stage = 0;
                    // Each screen shows 4 images, advance by 4
                    self.img_cur = (self.img_cur + IMAGES_PER_SCREEN) % self.img_count;
                    self.img_next = (self.img_cur + IMAGES_PER_SCREEN) % self.img_count;
                    // Keep showing "next" sprites until new images loaded
                    self.show_next_as_cur = true;
                    // Trigger reload of images for next cycle
                    self.tex_ready = false;
                }
            }
        }
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}
}
