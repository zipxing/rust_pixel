#![allow(dead_code)]
#![allow(unused_variables)]
use rust_pixel::event::Event;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use petview_lib::PetviewData;
use rust_pixel::{
    context::Context,
    game::Model,
    render::effect::{GpuTransition, GpuBlendEffect},
    util::Rand,
};

// Gallery mode: 2x2 grid (4 images per screen), each scaled 0.5
pub const PETW: u16 = 52;
pub const PETH: u16 = 32;
pub const IMAGES_PER_SCREEN: usize = 4;

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
    pub gpu_effect: GpuBlendEffect,
    pub tex_ready: bool,
    pub show_next_as_cur: bool,
    /// 打乱后的图片索引数组，长度补齐为4的倍数
    pub shuffled_indices: Vec<usize>,
}

impl PetviewModel {
    pub fn new() -> Self {
        Self {
            data: PetviewData::new(),
            normal_stage: 0,
            transbuf_stage: 0,
            img_cur: 0,
            img_next: IMAGES_PER_SCREEN,
            img_count: 2099,
            gpu_effect: GpuBlendEffect::default(),
            tex_ready: false,
            show_next_as_cur: false,
            shuffled_indices: Vec::new(),
        }
    }

    /// 初始化并打乱索引数组
    fn init_shuffled_indices(&mut self, rand: &mut Rand) {
        // 用当前时间做种子（WASM: js_sys::Date::now(), Native: SystemTime）
        rand.srand_now();

        // 创建顺序索引数组并打乱
        let mut indices: Vec<usize> = (0..self.img_count).collect();
        rand.shuffle(&mut indices);

        // 补齐为4的倍数（循环补充）
        let remainder = indices.len() % IMAGES_PER_SCREEN;
        if remainder != 0 {
            let padding = IMAGES_PER_SCREEN - remainder;
            for i in 0..padding {
                indices.push(indices[i % self.img_count]);
            }
        }

        self.shuffled_indices = indices;
    }

    /// 获取打乱后的真实图片索引
    pub fn get_image_index(&self, logical_idx: usize) -> usize {
        if self.shuffled_indices.is_empty() {
            logical_idx % self.img_count
        } else {
            self.shuffled_indices[logical_idx % self.shuffled_indices.len()]
        }
    }
}

impl Model for PetviewModel {
    fn init(&mut self, ctx: &mut Context) {
        ctx.state = PetviewState::Normal as u8;
        self.normal_stage = 0;
        self.init_shuffled_indices(&mut ctx.rand);
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
                    let random_idx = (ctx.rand.rand() % GpuTransition::count() as u32) as usize;
                    self.gpu_effect = GpuBlendEffect::from_index(random_idx, 0.0);
                    self.tex_ready = false;
                }
            }
            PetviewState::TransGl => {
                self.gpu_effect.progress += 0.01;
                if self.gpu_effect.progress >= 1.0 {
                    ctx.state = PetviewState::Normal as u8;
                    self.normal_stage = 0;
                    // 使用补齐后的数组长度
                    let total = self.shuffled_indices.len();
                    self.img_cur = (self.img_cur + IMAGES_PER_SCREEN) % total;
                    self.img_next = (self.img_cur + IMAGES_PER_SCREEN) % total;
                    self.show_next_as_cur = true;
                    self.tex_ready = false;
                }
            }
        }
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}
}
