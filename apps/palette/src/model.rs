// use log::info;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use palette_lib::{find_similar_colors, gradient, PaletteData, COLORS_WITH_NAME};
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
    render::style::{ColorPro, ColorSpace::*},
};
use std::any::Any;

pub const PALETTEW: u16 = 80;
pub const PALETTEH: u16 = 40;
pub const MENUX: u16 = 12;
pub const MENUY: u16 = 0;
pub const MENUW: u16 = 70;
pub const GRADIENT_COUNT: usize = 40;
pub const PICKER_COUNT: u16 = 19;
pub const ADJX: u16 = 2;
pub const ADJY: u16 = 2;
pub const COL_COUNT: u16 = 4;
pub const ROW_COUNT: u16 = 19;
pub const C_WIDTH: u16 = 19;

#[repr(u8)]
#[derive(Debug, Clone, Copy, FromPrimitive)]
pub enum PaletteState {
    NameA,
    NameB,
    Picker,
    Random,
    Gradient,
    Smart,
}

#[derive(Debug, Clone, Copy)]
pub struct SelectRange {
    pub area_count: usize,
    pub width: usize,
    pub height: usize,
    pub count: usize,
    pub area: usize,
    pub x: usize,
    pub y: usize,
}

impl SelectRange {
    pub fn new(a: usize, w: usize, h: usize, c: usize) -> Self {
        Self {
            area_count: a,
            width: w,
            height: h,
            count: c,
            area: 0,
            x: 0,
            y: 0,
        }
    }

    pub fn switch_area(&mut self) {
        if self.area_count == 0 {
            return;
        }
        self.area = (self.area + 1) % self.area_count;
    }

    pub fn forward_x(&mut self) {
        let count_last_row = self.count % self.width;
        if self.y == self.height - 1 && count_last_row != 0 {
            if self.x == count_last_row - 1 {
                self.x = 0;
            } else {
                self.x += 1;
            }
        } else {
            if self.x == self.width - 1 {
                self.x = 0;
            } else {
                self.x += 1;
            }
        }
    }

    pub fn backward_x(&mut self) {
        let count_last_row = self.count % self.width;
        if self.y == self.height - 1 && count_last_row != 0 {
            if self.x == 0 {
                self.x = count_last_row - 1;
            } else {
                self.x -= 1;
            }
        } else {
            if self.x == 0 {
                self.x = self.width - 1;
            } else {
                self.x -= 1;
            }
        }
    }

    pub fn forward_y(&mut self) {
        let count_last_col = self.height - 1;
        let modx = self.count % self.width;
        let mx = if modx == 0 { self.width } else { modx };
        if self.x >= mx {
            if self.y == count_last_col - 1 {
                self.y = 0;
            } else {
                self.y += 1;
            }
        } else {
            if self.y == self.height - 1 {
                self.y = 0;
            } else {
                self.y += 1;
            }
        }
    }

    pub fn backward_y(&mut self) {
        let count_last_col = self.height - 1;
        let modx = self.count % self.width;
        let mx = if modx == 0 { self.width } else { modx };
        if self.x >= mx {
            if self.y == 0 {
                self.y = count_last_col - 1;
            } else {
                self.y -= 1;
            }
        } else {
            if self.y == 0 {
                self.y = self.height - 1;
            } else {
                self.y -= 1;
            }
        }
    }
}

pub struct PaletteModel {
    pub data: PaletteData,
    pub card: u8,
    pub main_color: ColorPro,
    pub main_color_similar: (usize, usize, usize),
    pub named_colors: Vec<(&'static str, ColorPro)>,
    pub gradient_input_colors: Vec<ColorPro>,
    pub gradient_colors: Vec<ColorPro>,
    pub picker_color_hsv: (f64, f64, f64),
    pub picker_colors: Vec<ColorPro>,
    pub select: SelectRange,
}

impl PaletteModel {
    pub fn new() -> Self {
        let mut ncolors = COLORS_WITH_NAME.clone();
        ncolors.sort_by_key(|nc| (1000.0 - nc.1.brightness() * 1000.0) as i32);
        // ncolors.sort_by_key(|nc| (1000.0 - nc.1.hue() * 1000.0) as i32);
        // ncolors.sort_by_key(|nc| (nc.1.chroma() * 1000.0) as i32);

        Self {
            data: PaletteData::new(),
            card: 0,
            main_color: COLORS_WITH_NAME[0].1,
            main_color_similar: (0, 0, 0),
            named_colors: ncolors,
            gradient_input_colors: vec![],
            gradient_colors: vec![],
            picker_color_hsv: (0.0, 1.0, 1.0),
            picker_colors: vec![],
            select: SelectRange::new(0, 0, 0, 0),
        }
    }

    fn update_main_color(&mut self, context: &mut Context) {
        match PaletteState::from_usize(context.state as usize).unwrap() {
            PaletteState::NameA => {
                self.main_color =
                    self.named_colors[self.select.y * self.select.width + self.select.x].1;
            }
            PaletteState::NameB => {
                let idx = (COL_COUNT * ROW_COUNT) as usize
                    + self.select.y * self.select.width
                    + self.select.x;
                self.main_color = self.named_colors[idx].1;
            }
            _ => {}
        }
        // find similar colors by ciede2000...
        self.main_color_similar = find_similar_colors(&self.main_color);
        event_emit("Palette.RedrawMainColor");
        event_emit("Palette.RedrawSelect");
    }

    fn switch_state(&mut self, context: &mut Context, st: u8) {
        context.state = st;
        match PaletteState::from_usize(st as usize).unwrap() {
            PaletteState::NameA => {
                self.select = SelectRange::new(
                    1,
                    COL_COUNT as usize,
                    ROW_COUNT as usize,
                    (COL_COUNT * ROW_COUNT) as usize,
                );
                self.update_main_color(context);
            }
            PaletteState::NameB => {
                self.select = SelectRange::new(
                    1,
                    COL_COUNT as usize,
                    ROW_COUNT as usize - 3,
                    self.named_colors.len() - (COL_COUNT * ROW_COUNT) as usize,
                );
                self.update_main_color(context);
            }
            PaletteState::Picker => {}
            PaletteState::Random => {}
            PaletteState::Gradient => {}
            PaletteState::Smart => {}
        }
        event_emit("Palette.RedrawMenu");
        event_emit("Palette.RedrawPanel");
    }
}

impl Model for PaletteModel {
    fn init(&mut self, context: &mut Context) {
        self.data.shuffle();
        self.card = self.data.next();

        context.state = PaletteState::NameA as u8;

        // get gradient colors...
        self.gradient_input_colors = vec![
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 1.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 0.0, 1.0, 1.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0),
        ];
        gradient(
            &self.gradient_input_colors,
            GRADIENT_COUNT,
            &mut self.gradient_colors,
        );

        for y in 0..PICKER_COUNT {
            for x in 0..PICKER_COUNT {
                let rx = (x as f64) / (PICKER_COUNT as f64);
                let ry = (y as f64) / (PICKER_COUNT as f64);

                let h = 360.0 * rx;
                let s = 0.6;
                let l = 0.95 * ry;

                // Start with HSL
                let color1 = ColorPro::from_space_f64(HSLA, h, s, l, 1.0);

                // But (slightly) normalize the luminance
                let mut l = color1[LchA].unwrap().v[0];
                l = (l + ry * 100.0) / 2.0;
                let color = ColorPro::from_space_f64(
                    LchA,
                    l,
                    color1[LchA].unwrap().v[1],
                    color1[LchA].unwrap().v[2],
                    1.0,
                );
                self.picker_colors.push(color);
            }
        }

        self.switch_state(context, 0);
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    KeyCode::Char('1') => {
                        context.state = PaletteState::NameA as u8;
                        self.switch_state(context, 0);
                    }
                    KeyCode::Char('2') => {
                        context.state = PaletteState::NameB as u8;
                        self.switch_state(context, 1);
                    }
                    KeyCode::Char('3') => {
                        context.state = PaletteState::Picker as u8;
                        self.switch_state(context, 2);
                    }
                    KeyCode::Up => {
                        self.select.backward_y();
                        self.update_main_color(context);
                    }
                    KeyCode::Down => {
                        self.select.forward_y();
                        self.update_main_color(context);
                    }
                    KeyCode::Left => {
                        self.select.backward_x();
                        self.update_main_color(context);
                    }
                    KeyCode::Right => {
                        self.select.forward_x();
                        self.update_main_color(context);
                    }
                    KeyCode::Char('n') => {
                        self.select.switch_area();
                        self.update_main_color(context);
                    }
                    _ => {
                        // context.state = PaletteState::Picker as u8;
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
