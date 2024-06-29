use keyframe_derive::CanTween;
use rust_pixel::event::{Event, KeyCode};
// use rust_pixel::util::PointF32;
use keyframe::{functions::*, AnimationSequence};
// use log::info;
use rust_pixel::{context::Context, event::event_emit, game::Model};
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

#[derive(CanTween, Debug, Clone, Copy, PartialEq, Default)]
pub struct PointF32 {
    pub x: f32,
    pub y: f32,
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
            PointF32 { x: 800.0, y: 100.0 },
            PointF32 {
                x: 1200.0,
                y: 400.0,
            },
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

fn bezier_interpolation_func(t: f32, points: &[PointF32], count: usize) -> PointF32 {
    assert!(count > 0);

    let mut tmp_points = points.to_vec();
    for i in 1..count {
        for j in 0..(count - i) {
            if i == 1 {
                tmp_points[j].x = points[j].x * (1.0 - t) + points[j + 1].x * t;
                tmp_points[j].y = points[j].y * (1.0 - t) + points[j + 1].y * t;
                continue;
            }
            tmp_points[j].x = tmp_points[j].x * (1.0 - t) + tmp_points[j + 1].x * t;
            tmp_points[j].y = tmp_points[j].y * (1.0 - t) + tmp_points[j + 1].y * t;
        }
    }
    tmp_points[0]
}

fn draw_bezier_curves(points: &[PointF32], out_points: &mut [PointF32]) {
    let step = 1.0 / out_points.len() as f32;
    let mut t = 0.0;
    for i in 0..out_points.len() {
        let temp_point = bezier_interpolation_func(t, points, points.len());
        t += step;
        out_points[i] = temp_point;
    }
}

// fn main() {
// 	let in_points = [
// 		PointF32 { x: 100.0, y: 100.0 },
// 		PointF32 { x: 200.0, y: 200.0 },
// 		PointF32 { x: 250.0, y: 250.0 },
// 		PointF32 { x: 280.0, y: 290.0 },
// 		PointF32 { x: 300.0, y: 100.0 },
// 	];
// 	let num = 100;
// 	let mut out_points = vec![PointF32 { x: 0.0, y: 0.0 }; num];

// 	draw_bezier_curves(&in_points, in_points.len(), &mut out_points);

// 	for (j, point) in out_points.iter().enumerate() {
// 		println!("{} \t X={} \t Y={}", j, point.x, point.y);
// 	}
// }
