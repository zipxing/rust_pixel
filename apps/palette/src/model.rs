use log::info;
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

    pub fn forward_x(&mut self) {
        let count_last_row = self.count % self.width;
        if self.y == self.height - 1 {
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
        if self.y == self.height - 1 {
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

    pub forward_y(&mut self) {
        let count_last_col = self.height - 1;
        let modx = self.count % self.width;
        let mx = if modx == 0 { self.width } else { modx }; 
        if self.x >= mx {


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
    pub select_area: SelectRange,
    pub select_x: SelectRange,
    pub select_y: SelectRange,
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
            select_area: SelectRange::new(0, 0),
            select_x: SelectRange::new(0, 0),
            select_y: SelectRange::new(0, 0),
        }
    }

    fn update_main_color(&mut self, context: &mut Context) {
        match PaletteState::from_usize(context.state as usize).unwrap() {
            PaletteState::NameA => {
                self.main_color = self.named_colors
                    [self.select_y.value * self.select_x.max + self.select_x.value]
                    .1;
            }
            PaletteState::NameB => {
                let idx = 76 + self.select_y.value * self.select_x.max + self.select_x.value;
                self.main_color = self.named_colors[idx].1;
            }
            _ => {}
        }
        // find similar colors by ciede2000...
        self.main_color_similar = find_similar_colors(&self.main_color);
        event_emit("Palette.RedrawMainColor");
        event_emit("Palette.RedrawSelect");
    }

    fn check_range(&self, context: &mut Context) -> bool {
        match PaletteState::from_usize(context.state as usize).unwrap() {
            PaletteState::NameB => {
                let idx = 76 + self.select_y.value * self.select_x.max + self.select_x.value;
                if idx >= self.named_colors.len() {
                    return false; 
                }
            }
            _ => {}
        }
        true
    }

    fn movexy(&mut self, mv: &str, context: &mut Context) {
        let bx = self.select_x.clone();
        let by = self.select_y.clone();
        match mv {
            "w" => {self.select_y.backward();}
            "s" => {self.select_y.forward();}
            "a" => {self.select_x.backward();}
            "d" => {self.select_x.forward();}
            _ => {}
        }
        if !self.check_range(context) {
            self.select_x = bx.clone();
            self.select_y = by.clone();
        } else {
            self.update_main_color(context);
        }
    }

    fn switch_state(&mut self, context: &mut Context, st: u8) {
        context.state = st;
        self.select_area.value = 0;
        self.select_x.value = 0;
        self.select_y.value = 0;
        match PaletteState::from_usize(st as usize).unwrap() {
            PaletteState::NameA => {
                self.select_area.max = 0;
                self.select_x.max = COL_COUNT as usize;
                self.select_y.max = ROW_COUNT as usize;
                self.update_main_color(context);
            }
            PaletteState::NameB => {
                self.select_area.max = 0;
                self.select_x.max = COL_COUNT as usize;
                self.select_y.max = ROW_COUNT as usize - 3;
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
                    KeyCode::Char('w') => {
                        self.movexy("w", context);
                    }
                    KeyCode::Char('s') => {
                        self.movexy("s", context);
                    }
                    KeyCode::Char('a') => {
                        self.movexy("a", context);
                    }
                    KeyCode::Char('d') => {
                        self.movexy("d", context);
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
