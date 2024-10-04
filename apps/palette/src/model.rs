// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::select::*;
use log::info;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use palette_lib::{
    find_similar_colors, golden, gradient, random, PaletteData, COLORS_WITH_NAME,
    COLORS_WITH_NAME_RGB_INDEX,
};
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode, MouseButton, MouseEventKind::*},
    game::Model,
    render::style::{ColorPro, ColorSpace, ColorSpace::*, COLOR_SPACE_COUNT},
};
use PaletteState::*;

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
    PickerA,
    PickerB,
    Random,
    Gradient,
    Golden,
}

pub enum MouseArea {
    Menu(u16),                  // x
    Named(u16, u16),            // x, y
    Picker(u16, u16, u16, u16), // picker type, area, x, y
    Random(u16, u16),           // x, y
    Gradient(u16, u16, u16),    // area, x, y
    Golden(u16, u16),           // x, y
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

    fn mouse_in(&mut self, ctx: &Context, x: u16, y: u16) -> Option<MouseArea> {
        let st = PaletteState::from_usize(ctx.state as usize).unwrap();
        // Menu(u16)
        if y == 0 {
            let menuidx = match x {
                0..=22 => 0,
                23..=33 => 1,
                34..=44 => 2,
                45..=57 => 3,
                58.. => 4,
            };
            return Some(MouseArea::Menu(menuidx));
        }
        match st {
            // Named(u16, u16) x, y
            NameA | NameB => {
                let a = (x - 2) / C_WIDTH;
                let b = y - 2;
                if (b as usize * self.select.cur().width + a as usize) < self.select.cur().count {
                    return Some(MouseArea::Named(a, b));
                }
            }
            // Picker(u16, u16, u16, u16) picker type, area, x, y
            PickerA => {
                let a = 0;
                let c = x - 2;
                let d = y - 2;
                if (2..20).contains(&y) && (2..=77).contains(&x) {
                    let b = 0;
                    return Some(MouseArea::Picker(a, b, c, d));
                }
                if y == 20 && (2..=77).contains(&x) {
                    let b = 1;
                    return Some(MouseArea::Picker(a, b, (c as f64 * 4.0) as u16, 0));
                }
            }
            PickerB => {
                let c = ((x - 1) as f64 / PICKER_COUNT_X as f64 * 255.0) as u16;
                if y == 9 && (2..=77).contains(&x) {
                    return Some(MouseArea::Picker(1, 0, c, 0));
                }
                if y == 11 && (2..=77).contains(&x) {
                    return Some(MouseArea::Picker(1, 1, c, 0));
                }
                if y == 13 && (2..=77).contains(&x) {
                    return Some(MouseArea::Picker(1, 2, c, 0));
                }
            }
            // Random(u16, u16) x, y
            Random => {
                if (2..=5).contains(&y) && (1..=78).contains(&x) {
                    let a = (x - 1) / RANDOM_W;
                    let b = y - 2;
                    return Some(MouseArea::Random(a, b));
                }
            }
            // Golden(u16, u16) x, y
            Golden => {
                if (2..=5).contains(&y) && (1..=78).contains(&x) {
                    let a = (x - 1) / RANDOM_W;
                    let b = y - 2;
                    return Some(MouseArea::Golden(a, b));
                }
            }
            // Gradient(u16, u16, u16) area, x, y
            Gradient => {
                if (2..=19).contains(&y) && (2..=58).contains(&x) {
                    return Some(MouseArea::Gradient(0, x - 2, y - 2));
                }
                if y == 20 && (2..=58).contains(&x) {
                    return Some(MouseArea::Gradient(1, ((x - 2) as f64 * 4.0) as u16, 0));
                }
                if (2..=9).contains(&y) && (60..=67).contains(&x) && ((y - 2) as usize) < self.select.ranges[2].count {
                    return Some(MouseArea::Gradient(2, 0, y - 2));
                }
                if (2..=20).contains(&y) && (69..=77).contains(&x) && ((y - 2) as usize) < self.select.ranges[3].count {
                    return Some(MouseArea::Gradient(3, 0, y - 2));
                }
            }
        }
        info!("mouse...x{}, y{}", x, y);
        None
    }

    fn do_random(&mut self, context: &mut Context) {
        if context.state != Random as u8 {
            return;
        }
        random(
            RANDOM_X as usize * RANDOM_Y as usize,
            &mut self.data.rand,
            &mut self.random_colors,
        );
    }

    fn do_golden(&mut self, context: &mut Context) {
        if context.state != Golden as u8 {
            return;
        }
        golden(
            RANDOM_X as usize * RANDOM_Y as usize,
            &mut self.data.rand,
            &mut self.random_colors,
        );
    }

    fn do_gradient(&mut self, context: &mut Context) {
        if context.state != Gradient as u8 {
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
            self.gradient_colors.len(),
            self.gradient_colors.len(),
        );
        self.update_main_color(context);
        event_emit("Palette.RedrawGradient");
    }

    fn add_gradient_input(&mut self, context: &mut Context) {
        if context.state != Gradient as u8 {
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
            self.gradient_input_colors.len(),
            self.gradient_input_colors.len(),
        );
        self.do_gradient(context);
    }

    fn del_gradient_input(&mut self, context: &mut Context) {
        if context.state != Gradient as u8 {
            return;
        }
        if self.gradient_input_colors.is_empty() {
            return;
        }
        self.gradient_input_colors.pop();
        self.select.ranges[2] = SelectRange::new(
            1,
            self.gradient_input_colors.len(),
            self.gradient_input_colors.len(),
        );
        self.do_gradient(context);
    }

    fn update_select_by_main_color(&mut self, context: &mut Context, mc: ColorPro) {
        self.main_color = mc;

        match PaletteState::from_usize(context.state as usize).unwrap() {
            NameA => {}
            NameB => {}
            PickerA => {
                let hsv = mc[HSVA].unwrap().v;
                self.select.ranges[0].x = (hsv[1] * PICKER_COUNT_X as f64) as usize;
                self.select.ranges[0].y = ((1.0 - hsv[2]) * PICKER_COUNT_Y as f64) as usize;
                self.select.ranges[1].x = (hsv[0] * PICKER_COUNT_X as f64 / 90.0) as usize;
            }
            PickerB => {
                let rgb = mc[SRGBA].unwrap().v;
                self.select.ranges[0].x = (rgb[0] * 255.0) as usize;
                self.select.ranges[1].x = (rgb[1] * 255.0) as usize;
                self.select.ranges[2].x = (rgb[2] * 255.0) as usize;
            }
            Random | Golden => {}
            Gradient => {}
        }
        // find similar colors by ciede2000...
        self.main_color_similar = find_similar_colors(&self.main_color);
        event_emit("Palette.RedrawMainColor");
        event_emit("Palette.RedrawSelect");
        event_emit("Palette.RedrawPicker");
    }

    fn update_main_color(&mut self, context: &mut Context) {
        match PaletteState::from_usize(context.state as usize).unwrap() {
            NameA => {
                self.main_color = self.named_colors
                    [self.select.cur().y * self.select.cur().width + self.select.cur().x]
                    .1;
            }
            NameB => {
                let idx = (COL_COUNT * ROW_COUNT) as usize
                    + self.select.cur().y * self.select.cur().width
                    + self.select.cur().x;
                self.main_color = self.named_colors[idx].1;
            }
            PickerA => {
                self.main_color = get_pick_color(
                    PICKER_COUNT_X as usize,
                    self.select.ranges[0].x,
                    self.select.ranges[0].y,
                    self.select.ranges[1].x,
                    0,
                );
            }
            PickerB => {
                info!(
                    "r...{}g...{}b...{}",
                    self.select.ranges[0].x, self.select.ranges[1].x, self.select.ranges[2].x
                );
                self.main_color = get_pick_color(
                    PICKER_COUNT_X as usize,
                    self.select.ranges[0].x,
                    self.select.ranges[1].x,
                    self.select.ranges[2].x,
                    2,
                );
            }
            Random | Golden => {
                self.main_color = self.random_colors
                    [self.select.cur().y * self.select.cur().width + self.select.cur().x];
            }
            Gradient => match self.select.area {
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
                    if !self.gradient_input_colors.is_empty() {
                        self.main_color = self.gradient_input_colors[self.select.ranges[2].y];
                    }
                }
                3 => {
                    if !self.gradient_colors.is_empty() {
                        self.main_color = self.gradient_colors[self.select.ranges[3].y];
                    }
                }
                _ => {}
            },
        }
        // find similar colors by ciede2000...
        self.main_color_similar = find_similar_colors(&self.main_color);
        event_emit("Palette.RedrawMainColor");
        event_emit("Palette.RedrawSelect");
        event_emit("Palette.RedrawPicker");
    }

    fn switch_state(&mut self, context: &mut Context, st: PaletteState) {
        context.state = st as u8;
        match st {
            NameA => {
                self.select.clear();
                self.select.add_range(SelectRange::new(
                    COL_COUNT as usize,
                    ROW_COUNT as usize,
                    (COL_COUNT * ROW_COUNT) as usize,
                ));
                self.update_main_color(context);
            }
            NameB => {
                self.select.clear();
                self.select.add_range(SelectRange::new(
                    COL_COUNT as usize,
                    ROW_COUNT as usize - 3,
                    self.named_colors.len() - (COL_COUNT * ROW_COUNT) as usize,
                ));
                self.update_main_color(context);
            }
            PickerA => {
                self.select.clear();
                let w = PICKER_COUNT_X as usize;
                let h = PICKER_COUNT_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.select.add_range(SelectRange::new(w * 4, 1, w * 4));
                self.update_main_color(context);
                event_emit("Palette.RedrawPicker");
            }
            PickerB => {
                self.select.clear();
                self.select.add_range(SelectRange::new(256, 1, 256));
                self.select.add_range(SelectRange::new(256, 1, 256));
                self.select.add_range(SelectRange::new(256, 1, 256));
                self.update_main_color(context);
                event_emit("Palette.RedrawPicker");
            }
            Random => {
                self.select.clear();
                let w = RANDOM_X as usize;
                let h = RANDOM_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.do_random(context);
                self.update_main_color(context);
                event_emit("Palette.RedrawRandom");
            }
            Gradient => {
                self.select.clear();
                let w = PICKER_COUNT_X_GRADIENT as usize;
                let h = PICKER_COUNT_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.select.add_range(SelectRange::new(w * 4, 1, w * 4));
                // gradient input ...
                self.select.add_range(SelectRange::new(
                    1,
                    self.gradient_input_colors.len(),
                    self.gradient_input_colors.len(),
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
            Golden => {
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

        // init hsv picker
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

        self.switch_state(context, NameA);
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Mouse(mou) => {
                    if mou.kind == Up(MouseButton::Left) {
                        match self.mouse_in(context, mou.column, mou.row) {
                            Some(MouseArea::Menu(i)) => {
                                let sts = [NameA, PickerA, Random, Gradient, Golden];
                                self.switch_state(context, sts[i as usize]);
                            }
                            Some(MouseArea::Named(a, b)) => {
                                self.select.cur().x = a as usize;
                                self.select.cur().y = b as usize;
                                self.update_main_color(context);
                            }
                            Some(MouseArea::Picker(_a, b, c, d)) => {
                                self.select.area = b as usize;
                                self.select.ranges[b as usize].x = c as usize;
                                self.select.ranges[b as usize].y = d as usize;
                                self.update_main_color(context);
                            }
                            Some(MouseArea::Random(a, b)) => {
                                self.select.cur().x = a as usize;
                                self.select.cur().y = b as usize;
                                self.update_main_color(context);
                            }
                            Some(MouseArea::Gradient(b, c, d)) => {
                                self.select.area = b as usize;
                                self.select.ranges[b as usize].x = c as usize;
                                self.select.ranges[b as usize].y = d as usize;
                                self.update_main_color(context);
                            }
                            Some(MouseArea::Golden(a, b)) => {
                                self.select.cur().x = a as usize;
                                self.select.cur().y = b as usize;
                                self.update_main_color(context);
                            }
                            _ => {}
                        }
                    }
                }
                Event::Key(key) => match key.code {
                    KeyCode::Char('1') => {
                        self.switch_state(context, NameA);
                    }
                    KeyCode::Char('n') => {
                        if context.state == NameA as u8 {
                            self.switch_state(context, NameB);
                        } else if context.state == NameB as u8 {
                            self.switch_state(context, NameA);
                        } else if context.state == PickerA as u8 {
                            // backup main_color
                            let mc = self.main_color;
                            self.switch_state(context, PickerB);
                            // set select value by backup_main_color
                            self.update_select_by_main_color(context, mc);
                        } else if context.state == PickerB as u8 {
                            // backup main_color
                            let mc = self.main_color;
                            self.switch_state(context, PickerA);
                            // set select value by backup_main_color
                            self.update_select_by_main_color(context, mc);
                        }
                    }
                    KeyCode::Char('2') => {
                        self.switch_state(context, PickerA);
                    }
                    KeyCode::Char('3') => {
                        self.switch_state(context, Random);
                    }
                    KeyCode::Char('4') => {
                        self.switch_state(context, Gradient);
                    }
                    KeyCode::Char('5') => {
                        self.switch_state(context, Golden);
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
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}

pub fn get_pick_color(width: usize, x0: usize, y0: usize, x1: usize, t: usize) -> ColorPro {
    let h = 360.0 / 4.0 / width as f64 * x1 as f64;
    let s = 1.0 / width as f64 * x0 as f64;
    let v = 1.0 / PICKER_COUNT_Y as f64 * y0 as f64;

    let r = x0 as f64;
    let g = y0 as f64;
    let b = x1 as f64;

    match t {
        0 => ColorPro::from_space_f64(HSVA, h, s, 1.0 - v, 1.0),
        1 => ColorPro::from_space_f64(HSVA, h, 1.0, 1.0, 1.0),
        _ => ColorPro::from_space_f64(SRGBA, r / 255.0, g / 255.0, b / 255.0, 1.0),
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
                format!(
                    "#{:02X}{:02X}{:02X} {},{},{} {:20}",
                    rgb.0, rgb.1, rgb.2, rgb.0, rgb.1, rgb.2, " "
                )
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
