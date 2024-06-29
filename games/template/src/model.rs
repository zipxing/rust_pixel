use rust_pixel::event::{Event, KeyCode};
use rust_pixel::util::PointF32;
use keyframe::{functions::*, AnimationSequence};
// use log::info;
use rust_pixel::{algorithm::draw_bezier_curves, context::Context, event::event_emit, game::Model};
use std::any::Any;
use template_lib::TemplateData;

pub const CARDW: usize = 7;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub const CARDH: usize = 7;
#[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
pub const CARDH: usize = 5;
pub const TEMPLATEW: u16 = 80;
pub const TEMPLATEH: u16 = 40;

#[repr(u8)]
enum TemplateState {
    Normal,
}

pub struct TemplateModel {
    pub data: TemplateData,
    pub bezier: AnimationSequence<PointF32>,
    pub card: u8,
}

impl TemplateModel {
    pub fn new() -> Self {
        Self {
            data: TemplateData::new(),
            bezier: AnimationSequence::new(),
            card: 0,
        }
    }
}

impl Model for TemplateModel {
    fn init(&mut self, _context: &mut Context) {
        let in_points = [
            PointF32 { x: 0.0, y: 0.0 },
            PointF32 { x: 1200.0, y: 100.0 },
            PointF32 {
                x: TEMPLATEW as f32 * 16.0,
                y: TEMPLATEH as f32 * 16.0,
            },
        ];
        let num = 100;
        let mut pts = vec![PointF32 { x: 0.0, y: 0.0 }; num];
        draw_bezier_curves(&in_points, &mut pts);

        let mut ks = Vec::new();

        for i in 0..num {
            ks.push((pts[i], i as f64 / num as f64, EaseIn).into());
            // ks.push((pts[i], i as f64 / num as f64).into());
        }

        self.bezier = AnimationSequence::from(ks);
        self.data.shuffle();
        self.card = self.data.next();
        event_emit("Template.RedrawTile");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
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

