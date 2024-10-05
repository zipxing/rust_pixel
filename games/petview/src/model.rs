#![allow(dead_code)]
#![allow(unused_variables)]
use rust_pixel::event::Event;
// use log::info;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use petview_lib::PetviewData;
use rust_pixel::{context::Context, game::Model};

pub const PETW: u16 = 50;
pub const PETH: u16 = 30;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
enum PetviewState {
    Normal,
    Trans,
}

pub struct PetviewModel {
    pub data: PetviewData,
    pub normal_stage: u32,
    pub img_cur: usize,
    pub img_next: usize,
    pub img_count: usize,
    pub trans_effect: usize,
    pub tex_ready: bool,
    pub progress: f32,
}

impl PetviewModel {
    pub fn new() -> Self {
        Self {
            data: PetviewData::new(),
            normal_stage: 0,
            img_cur: 0,
            img_next: 1,
            img_count: 10,
            trans_effect: 0,
            tex_ready: false,
            progress: 0.0,
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
                if self.normal_stage > 300 {
                    ctx.state = PetviewState::Trans as u8;
                    self.trans_effect = (ctx.rand.rand() % 4) as usize;
                    self.progress = 0.0;
                    self.tex_ready = false;
                }
            }
            PetviewState::Trans => {
                self.progress += 0.01;
                if self.progress >= 1.0 {
                    ctx.state = PetviewState::Normal as u8;
                    self.normal_stage = 0;
                    self.img_cur = (self.img_cur + 1) % self.img_count;
                    self.img_next = (self.img_cur + 1) % self.img_count;
                }
            }
        }
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}
}
