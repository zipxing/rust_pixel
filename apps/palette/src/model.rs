use log::info;
use num_traits::FromPrimitive;
use palette_lib::{PaletteData, COLORS_WITH_NAME};
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
    render::style::{
        delta_e_cie76, delta_e_ciede2000, ColorPro, ColorScale, ColorSpace, ColorSpace::*,
        Fraction, COLOR_SPACE_COUNT,
    },
};
use std::any::Any;

pub const PALETTEW: u16 = 100;
pub const PALETTEH: u16 = 40;
pub const CCOUNT: usize = 40;

#[repr(u8)]
enum PaletteState {
    Normal,
}

pub struct PaletteModel {
    pub data: PaletteData,
    pub card: u8,
    pub colors: Vec<ColorPro>,
}

impl PaletteModel {
    pub fn new() -> Self {
        Self {
            data: PaletteData::new(),
            card: 0,
            colors: vec![],
        }
    }
}

impl Model for PaletteModel {
    fn init(&mut self, _context: &mut Context) {
        self.data.shuffle();
        self.card = self.data.next();

        // test ...
        let color = ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0);
        for i in 0..COLOR_SPACE_COUNT {
            info!(
                "{}:{:?}",
                ColorSpace::from_usize(i).unwrap(),
                color.space_matrix[i].unwrap()
            );
        }

        let c1 = ColorPro::from_space_f64(LabA, 50.0, 0.8, -80.0, 1.0);
        let c2 = ColorPro::from_space_f64(LabA, 100.0, 1.2, 90.0, 1.0);
        let d1 = delta_e_cie76(c1[LabA].unwrap(), c2[LabA].unwrap());
        let d2 = delta_e_ciede2000(c1[LabA].unwrap(), c2[LabA].unwrap());
        info!("d76...{}, d2000...{}", d1, d2);

        let mut deltas: Vec<(usize, f64)> = vec![];
        let mut count = 0usize;
        for idx in 0..COLORS_WITH_NAME.len() {
            let c = COLORS_WITH_NAME[idx];
            let d = delta_e_ciede2000(c1[LabA].unwrap(), c.1[LabA].unwrap());
            deltas.push((count, d));
            count += 1;
        }
        deltas.sort_by_key(|nc| (nc.1 * 1000.0) as i32);
        for i in 0..3 {
            let id = deltas[i];
            info!("deltas....{:?}", COLORS_WITH_NAME[id.0].0);
        }

        let colors = vec![
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 1.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 0.0, 1.0, 1.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0),
        ];
        let color_count = colors.len();

        let mut color_scale = ColorScale::empty();

        for (i, color) in colors.into_iter().enumerate() {
            let position = Fraction::from(i as f64 / (color_count as f64 - 1.0));
            color_scale.add_stop(color, position);
        }

        info!("color_stop.....{:?}", color_scale);

        for i in 0..CCOUNT {
            let position = Fraction::from(i as f64 / (CCOUNT as f64 - 1.0));
            let color = color_scale
                .sample(position, OKLchA)
                .expect("gradient color");
            let cp = ColorPro::from_space(OKLchA, color);
            self.colors.push(cp);
            info!("color_sample_oklch.....{:?}", cp[OKLchA].unwrap());
            info!("color_sample_xyz.....{:?}", cp[XYZA].unwrap());
            info!("color_sample_oklab.....{:?}", cp[OKLabA].unwrap());
            info!("color_sample_srgba.....{:?}", cp[SRGBA].unwrap());
            info!("------------------------------")
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
