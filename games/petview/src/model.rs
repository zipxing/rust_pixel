use rust_pixel::event::Event;
// use log::info;
use petview_lib::PetviewData;
use rust_pixel::{context::Context, event::event_emit, game::Model};

pub const PETW: u16 = 50;
pub const PETH: u16 = 30;

#[repr(u8)]
enum PetviewState {
    Normal,
}

pub struct PetviewModel {
    pub data: PetviewData,
}

impl PetviewModel {
    pub fn new() -> Self {
        Self {
            data: PetviewData::new(),
        }
    }
}

impl Model for PetviewModel {
    fn init(&mut self, _ctx: &mut Context) {
        event_emit("Petview.RedrawTile");
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
  
    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {}
    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}
    
}
