use rust_pixel::event::Event;
// use log::info;
use rust_pixel::{context::Context, event::event_emit, game::Model};
use std::any::Any;
use petview_lib::PetviewData;

pub const PETVIEWW: u16 = 50;
pub const PETVIEWH: u16 = 30;

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
    fn init(&mut self, _context: &mut Context) {
        event_emit("Petview.RedrawTile");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    _ => {
                        context.state = PetviewState::Normal as u8;
                    }
                },
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

