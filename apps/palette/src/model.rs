// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use log::info;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use palette_lib::{
    find_similar_colors, golden, gradient, random, PaletteData, COLORS_WITH_NAME,
    COLORS_WITH_NAME_RGB_INDEX,
};
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
    render::style::{ColorPro, ColorSpace, ColorSpace::*, COLOR_SPACE_COUNT},
};
use std::any::Any;
use crate::select::*;

pub const PALETTEW: u16 = 80;
pub const PALETTEH: u16 = 40;
pub const MENUX: u16 = 12;
pub const MENUY: u16 = 0;
pub const MENUW: u16 = 70;
pub const RANDOM_X: u16 = 6;
pub const RANDOM_Y: u16 = 4;
pub const RANDOM_W: u16 = 13;
pub const GRADIENT_X: u16 = 1;
pub const GRADIENT_Y: u16 = 19;
pub const GRADIENT_INPUT_COUNT: u16 = 8;
pub const GRADIENT_COUNT: u16 = GRADIENT_X * GRADIENT_Y;
pub const PICKER_COUNT_X_GRADIENT: u16 = 57;
pub const PICKER_COUNT_X: u16 = 76;
pub const PICKER_COUNT_Y: u16 = 18;
pub const MAIN_COLOR_MSG_X: u16 = 1;
pub const MAIN_COLOR_MSG_Y: u16 = 8;
pub const ADJX: u16 = 2;
pub const ADJY: u16 = 2;
pub const COL_COUNT: u16 = 4;
pub const ROW_COUNT: u16 = 19;
pub const C_WIDTH: u16 = 19;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum PaletteState {
    NameA,
    NameB,
    Picker,
    Random,
    Gradient,
    Golden,
}

pub struct PaletteModel {
    pub data: PaletteData,
    pub card: u8,
    pub main_color: ColorPro,
    pub main_color_similar: (usize, usize, usize),
    pub named_colors: Vec<(&'static str, ColorPro)>,
    pub gradient_input_colors: Vec<ColorPro>,
    pub gradient_colors: Vec<ColorPro>,
    pub random_colors: Vec<ColorPro>,
    pub picker_colors: Vec<ColorPro>,
    pub select: Select,
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
            random_colors: vec![],
            picker_colors: vec![],
            select: Select::new(),
        }
    }

    fn do_random(&mut self, context: &mut Context) {
        if context.state != PaletteState::Random as u8 {
            return;
        }
        random(
            RANDOM_X as usize * RANDOM_Y as usize,
            &mut self.data.rand,
            &mut self.random_colors,
        );
    }

    fn do_golden(&mut self, context: &mut Context) {
        if context.state != PaletteState::Golden as u8 {
            return;
        }
        golden(
            RANDOM_X as usize * RANDOM_Y as usize,
            &mut self.data.rand,
            &mut self.random_colors,
        );
    }

    fn do_gradient(&mut self, context: &mut Context) {
        if context.state != PaletteState::Gradient as u8 {
            return;
        }
        info!("do gradient..........");
        gradient(
            &self.gradient_input_colors,
            GRADIENT_COUNT as usize,
            &mut self.gradient_colors,
        );
        self.select.ranges[3] = SelectRange::new(
            1,
            self.gradient_colors.len() as usize,
            self.gradient_colors.len() as usize,
        );
        self.update_main_color(context);
        event_emit("Palette.RedrawGradient");
    }

    fn add_gradient_input(&mut self, context: &mut Context) {
        if context.state != PaletteState::Gradient as u8 {
            return;
        }
        if self.gradient_input_colors.len() >= GRADIENT_INPUT_COUNT as usize {
            return;
        }
        let nc = get_pick_color(
            PICKER_COUNT_X_GRADIENT as usize,
            self.select.ranges[0].x,
            self.select.ranges[0].y,
            self.select.ranges[1].x,
            0,
        );
        self.gradient_input_colors.push(nc);
        self.select.ranges[2] = SelectRange::new(
            1,
            self.gradient_input_colors.len() as usize,
            self.gradient_input_colors.len() as usize,
        );
        self.do_gradient(context);
    }

    fn del_gradient_input(&mut self, context: &mut Context) {
        if context.state != PaletteState::Gradient as u8 {
            return;
        }
        if self.gradient_input_colors.len() == 0 {
            return;
        }
        self.gradient_input_colors.pop();
        self.select.ranges[2] = SelectRange::new(
            1,
            self.gradient_input_colors.len() as usize,
            self.gradient_input_colors.len() as usize,
        );
        self.do_gradient(context);
    }

    fn update_main_color(&mut self, context: &mut Context) {
        match PaletteState::from_usize(context.state as usize).unwrap() {
            PaletteState::NameA => {
                self.main_color = self.named_colors
                    [self.select.cur().y * self.select.cur().width + self.select.cur().x]
                    .1;
            }
            PaletteState::NameB => {
                let idx = (COL_COUNT * ROW_COUNT) as usize
                    + self.select.cur().y * self.select.cur().width
                    + self.select.cur().x;
                self.main_color = self.named_colors[idx].1;
            }
            PaletteState::Picker => {
                self.main_color = get_pick_color(
                    PICKER_COUNT_X as usize,
                    self.select.ranges[0].x,
                    self.select.ranges[0].y,
                    self.select.ranges[1].x,
                    0,
                );
            }
            PaletteState::Random => {
                self.main_color = self.random_colors
                    [self.select.cur().y * self.select.cur().width + self.select.cur().x];
            }
            PaletteState::Gradient => match self.select.area {
                0..=1 => {
                    self.main_color = get_pick_color(
                        PICKER_COUNT_X_GRADIENT as usize,
                        self.select.ranges[0].x,
                        self.select.ranges[0].y,
                        self.select.ranges[1].x,
                        0,
                    );
                }
                2 => {
                    if self.gradient_input_colors.len() != 0 {
                        self.main_color = self.gradient_input_colors[self.select.ranges[2].y];
                    }
                }
                3 => {
                    if self.gradient_colors.len() != 0 {
                        self.main_color = self.gradient_colors[self.select.ranges[3].y];
                    }
                }
                _ => {}
            },
            _ => {}
        }
        // find similar colors by ciede2000...
        self.main_color_similar = find_similar_colors(&self.main_color);
        event_emit("Palette.RedrawMainColor");
        event_emit("Palette.RedrawSelect");
        event_emit("Palette.RedrawPicker");
    }

    fn switch_state(&mut self, context: &mut Context, st: u8) {
        context.state = st;
        match PaletteState::from_usize(st as usize).unwrap() {
            PaletteState::NameA => {
                self.select.clear();
                self.select.add_range(SelectRange::new(
                    COL_COUNT as usize,
                    ROW_COUNT as usize,
                    (COL_COUNT * ROW_COUNT) as usize,
                ));
                self.update_main_color(context);
            }
            PaletteState::NameB => {
                self.select.clear();
                self.select.add_range(SelectRange::new(
                    COL_COUNT as usize,
                    ROW_COUNT as usize - 3,
                    self.named_colors.len() - (COL_COUNT * ROW_COUNT) as usize,
                ));
                self.update_main_color(context);
            }
            PaletteState::Picker => {
                self.select.clear();
                let w = PICKER_COUNT_X as usize;
                let h = PICKER_COUNT_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.select.add_range(SelectRange::new(w * 4, 1, w * 4));
                self.update_main_color(context);
                event_emit("Palette.RedrawPicker");
            }
            PaletteState::Random => {
                self.select.clear();
                let w = RANDOM_X as usize;
                let h = RANDOM_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.do_random(context);
                self.update_main_color(context);
                event_emit("Palette.RedrawRandom");
            }
            PaletteState::Gradient => {
                self.select.clear();
                let w = PICKER_COUNT_X_GRADIENT as usize;
                let h = PICKER_COUNT_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.select.add_range(SelectRange::new(w * 4, 1, w * 4));
                // gradient input ...
                self.select.add_range(SelectRange::new(
                    1,
                    self.gradient_input_colors.len() as usize,
                    self.gradient_input_colors.len() as usize,
                ));
                // gradient ...
                self.select.add_range(SelectRange::new(
                    GRADIENT_X as usize,
                    GRADIENT_Y as usize,
                    GRADIENT_COUNT as usize,
                ));
                self.do_gradient(context);
                self.update_main_color(context);
                event_emit("Palette.RedrawPicker");
            }
            PaletteState::Golden => {
                self.select.clear();
                let w = RANDOM_X as usize;
                let h = RANDOM_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.do_golden(context);
                self.update_main_color(context);
                event_emit("Palette.RedrawRandom");
            }
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

        let ctest = ColorPro::from_space_f64(SRGBA, 0.0, 1.0, 0.0, 1.0);
        for i in 0..COLOR_SPACE_COUNT {
            info!(
                "{}:{:?}",
                ColorSpace::from_usize(i).unwrap(),
                ctest.space_matrix[i].unwrap()
            );
        }

        // get gradient colors...
        self.gradient_input_colors = vec![
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 1.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 0.0, 1.0, 1.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.8, 1.0),
        ];

        for y in 0..PICKER_COUNT_X {
            for x in 0..PICKER_COUNT_Y {
                let rx = (x as f64) / (PICKER_COUNT_X as f64);
                let ry = (y as f64) / (PICKER_COUNT_X as f64);

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
                        self.switch_state(context, PaletteState::NameA as u8);
                    }
                    KeyCode::Char('2') => {
                        self.switch_state(context, PaletteState::NameB as u8);
                    }
                    KeyCode::Char('3') => {
                        self.switch_state(context, PaletteState::Picker as u8);
                    }
                    KeyCode::Char('4') => {
                        self.switch_state(context, PaletteState::Random as u8);
                    }
                    KeyCode::Char('5') => {
                        self.switch_state(context, PaletteState::Gradient as u8);
                        event_emit("Palette.RedrawGradient");
                    }
                    KeyCode::Char('6') => {
                        self.switch_state(context, PaletteState::Golden as u8);
                    }
                    KeyCode::Char('a') => {
                        self.add_gradient_input(context);
                    }
                    KeyCode::Char('d') => {
                        self.del_gradient_input(context);
                    }
                    KeyCode::Char('g') => {
                        self.do_gradient(context);
                    }
                    KeyCode::Up => {
                        self.select.cur().backward_y();
                        self.update_main_color(context);
                    }
                    KeyCode::Down => {
                        self.select.cur().forward_y();
                        self.update_main_color(context);
                    }
                    KeyCode::Left => {
                        self.select.cur().backward_x();
                        self.update_main_color(context);
                    }
                    KeyCode::Right => {
                        self.select.cur().forward_x();
                        self.update_main_color(context);
                    }
                    KeyCode::Tab => {
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

pub fn get_pick_color(width: usize, x0: usize, y0: usize, x1: usize, t: usize) -> ColorPro {
    let h = 360.0 / 4.0 / width as f64 * x1 as f64;
    let s = 1.0 / width as f64 * x0 as f64;
    let v = 1.0 / PICKER_COUNT_Y as f64 * y0 as f64;

    if t == 0 {
        ColorPro::from_space_f64(HSVA, h, s, 1.0 - v, 1.0)
    } else {
        ColorPro::from_space_f64(HSVA, h, 1.0, 1.0, 1.0)
    }
}

pub fn get_color_info(c: ColorPro, idx: u16) -> String {
    match idx {
        0 => {
            let rgb = c.get_srgba_u8();
            if let Some(cp) = COLORS_WITH_NAME_RGB_INDEX.get(&rgb) {
                format!(
                    "#{:02X}{:02X}{:02X} {},{},{} {:20}",
                    rgb.0,
                    rgb.1,
                    rgb.2,
                    rgb.0,
                    rgb.1,
                    rgb.2,
                    COLORS_WITH_NAME[*cp].0.to_string(),
                )
            } else {
                format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2)
            }
        }
        1..=8 => {
            let display_space = [2, 4, 6, 7, 8, 9, 11, 12];
            let cidx = display_space[idx as usize - 1];
            format!(
                "{} :{:?}",
                ColorSpace::from_usize(cidx).unwrap(),
                c.space_matrix[cidx].unwrap()
            )
        }
        _ => "".to_string(),
    }
}
