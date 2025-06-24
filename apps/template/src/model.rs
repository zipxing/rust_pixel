use keyframe::{functions::*, AnimationSequence};
use rust_pixel::{
    algorithm::draw_bezier_curves,
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
    util::{ParticleSystem, ParticleSystemInfo, PointF32},
};
// use log::info;
use std::f64::consts::PI;
use template_lib::TemplateData;

pub const CARDW: usize = 7;
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const CARDH: usize = 7;
#[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
pub const CARDH: usize = 5;
pub const TEMPLATEW: u16 = 80;
pub const TEMPLATEH: u16 = 40;

#[repr(u8)]
enum TemplateState {
    Normal,
}

pub struct TemplateModel {
    // TemplateData defined in template/lib/src/lib.rs
    pub data: TemplateData,
    pub pats: ParticleSystem,
    pub bezier: AnimationSequence<PointF32>,
    pub count: f64,
    pub card: u8,
}

impl TemplateModel {
    pub fn new() -> Self {
        let particle_system_info = ParticleSystemInfo {
            emission_rate: 100.0,
            lifetime: -1.0,
            particle_life_min: 1.0,
            particle_life_max: 2.0,
            direction: PI / 2.0,
            spread: PI / 4.0,
            relative: false,
            speed_min: 50.0,
            speed_max: 100.0,
            g_min: 9.0,
            g_max: 10.0,
            rad_a_min: 3.0,
            rad_a_max: 5.0,
            tan_a_min: 1.0,
            tan_a_max: 5.0,
            size_start: 1.0,
            size_end: 5.0,
            size_var: 1.0,
            spin_start: 1.0,
            spin_end: 5.0,
            spin_var: 1.0,
            color_start: [0.0, 0.0, 0.0, 0.0],
            color_end: [1.0, 1.0, 1.0, 1.0],
            color_var: 0.1,
            alpha_var: 1.0,
        };
        // create particle system
        let pats = ParticleSystem::new(particle_system_info);

        Self {
            pats,
            data: TemplateData::new(),
            bezier: AnimationSequence::new(),
            count: 0.0,
            card: 0,
        }
    }
}

impl Model for TemplateModel {
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

        // Fire particle system...
        self.pats.fire_at(10.0, 10.0);

        // Emit event...
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
                        // Emit event...
                        event_emit("Template.RedrawTile");
                    }
                    KeyCode::Char('n') => {
                        self.card = self.data.next();
                        // Emit event...
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

    fn handle_auto(&mut self, _context: &mut Context, dt: f32) {
        self.pats.update(dt as f64);
        self.count += 1.0;
        if self.count > 200.0 {
            self.count = 0.0f64;
        }
        self.pats
            .move_to(10.0 + 2.0 * self.count, 10.0 + 2.0 * self.count, false);
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
