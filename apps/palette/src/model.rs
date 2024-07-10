use keyframe::{functions::*, AnimationSequence};
use log::info;
use palette_lib::PaletteData;
use rust_pixel::event::{Event, KeyCode};
use rust_pixel::render::style::{
    ColorDataWrap, ColorPro, ColorSpace::*, COLOR_SPACE_COUNT, COLOR_SPACE_NAME,
};
use rust_pixel::util::PointF32;
use rust_pixel::{algorithm::draw_bezier_curves, context::Context, event::event_emit, game::Model};
use std::any::Any;

pub const CARDW: usize = 7;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub const CARDH: usize = 7;
#[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
pub const CARDH: usize = 5;
pub const PALETTEW: u16 = 100;
pub const PALETTEH: u16 = 40;

#[repr(u8)]
enum PaletteState {
    Normal,
}

pub struct PaletteModel {
    pub data: PaletteData,
    pub bezier: AnimationSequence<PointF32>,
    pub card: u8,
}

impl PaletteModel {
    pub fn new() -> Self {
        Self {
            data: PaletteData::new(),
            bezier: AnimationSequence::new(),
            card: 0,
        }
    }
}

impl Model for PaletteModel {
    fn init(&mut self, _context: &mut Context) {
        let in_points = [
            PointF32 { x: 10.0, y: 30.0 },
            PointF32 { x: 210.0, y: 450.0 },
            PointF32 { x: 110.0, y: 150.0 },
            PointF32 {
                x: 1200.0,
                y: 150.0,
            },
            PointF32 {
                x: PALETTEW as f32 * 16.0,
                y: PALETTEH as f32 * 16.0,
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

        let mut color = ColorPro::from_space_data(SRGBA, [0.5, 0.5, 0.5, 1.0]);
        let _ = color.fill_all_spaces();
        for i in 0..COLOR_SPACE_COUNT {
            info!(
                "{}:{:?}",
                COLOR_SPACE_NAME[i],
                ColorDataWrap(color.space_matrix[i].unwrap())
            );
        }
        event_emit("Palette.RedrawTile");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    KeyCode::Char('s') => {
                        self.data.shuffle();
                        self.card = self.data.next();
                        event_emit("Palette.RedrawTile");
                    }
                    KeyCode::Char('n') => {
                        self.card = self.data.next();
                        event_emit("Palette.RedrawTile");
                    }
                    _ => {
                        context.state = PaletteState::Normal as u8;
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
