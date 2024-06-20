use rust_pixel::event::{Event, KeyCode};
// use log::info;
use std::any::Any;
use rust_pixel::{context::Context, event::event_emit, game::Model};
use template_lib::TemplateData;

pub const CARDW: usize = 7;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub const CARDH: usize = 7;
#[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
pub const CARDH: usize = 5;

#[repr(u8)]
enum TemplateState {
    Normal,
}

pub struct TemplateModel {
    pub data: TemplateData,
    pub card: u8,
}

impl TemplateModel {
    pub fn new() -> Self {
        Self {
            data: TemplateData::new(),
            card: 0,
        }
    }
}

impl Model for TemplateModel {
    fn init(&mut self, _context: &mut Context) {
        self.data.shuffle();
        self.card = self.data.next();
        event_emit("Template.RedrawTile");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('s') => {
                            self.data.shuffle();
                            self.card = self.data.next();
                            event_emit("Template.RedrawTile");
                        }
                        KeyCode::Char('n') => {
                            self.card = self.data.next();
                            event_emit("Template.RedrawTile");
                        }
                        _ => {
                            context.state = TemplateState::Normal as u8;
                        },
                    }
                }
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
