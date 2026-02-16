// RustPixel
// copyright zipxing@hotmail.com 2022～2025

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
    event::{Event, KeyCode, MouseButton, MouseEventKind::*},
    game::Model,
    render::Buffer,
    render::style::{Color, ColorPro, ColorSpace, ColorSpace::*, Style, COLOR_SPACE_COUNT},
    ui::UIPage,
    ui::components::panel::{Panel, BorderStyle},
    util::Rect,
};
use PaletteState::*;

// Selection indicator
pub const PL_ARROW: &str = "▸";
// Menu tab separator: "\u{E0B0}", fallback: "▸" or "|"
pub const MENU_SEP: &str = "";
pub const PALETTEW: u16 = 80;
pub const PALETTEH: u16 = 35;
pub const MENUX: u16 = 12;
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
// Frame divider positions (shared by Panel and mouse_in)
pub const HDIV1: u16 = 22;  // upper horizontal divider
pub const HDIV2: u16 = 32;  // lower horizontal divider
pub const VDIV1: u16 = 59;  // vertical divider (between info and similar)
// Derived content boundaries
pub const CONTENT_X1: u16 = ADJX;                   // content left edge
pub const CONTENT_X2: u16 = PALETTEW - 2;           // content right edge
pub const CONTENT_Y1: u16 = ADJY;                   // content top edge
pub const CONTENT_Y2: u16 = HDIV1 - 1;              // main area bottom (row 21)
pub const HUE_BAR_Y: u16 = ADJY + PICKER_COUNT_Y;  // hue bar row (row 20)
pub const RGB_BAR_Y: u16 = 9;                       // RGB picker bars start row

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
    pub main_color: ColorPro,
    pub main_color_similar: (usize, usize, usize),
    pub named_colors: Vec<(&'static str, ColorPro)>,
    pub gradient_input_colors: Vec<ColorPro>,
    pub gradient_colors: Vec<ColorPro>,
    pub random_colors: Vec<ColorPro>,
    pub picker_colors: Vec<ColorPro>,
    pub select: Select,
    pub page: UIPage,
    pub need_redraw: bool,
    pub state: PaletteState,
}

impl PaletteModel {
    pub fn new() -> Self {
        let mut ncolors = COLORS_WITH_NAME.clone();
        ncolors.sort_by_key(|nc| (1000.0 - nc.1.brightness() * 1000.0) as i32);

        let mut page = UIPage::new(PALETTEW, PALETTEH);
        let frame = Panel::new()
            .with_bounds(Rect::new(0, 0, PALETTEW, PALETTEH))
            .with_border(BorderStyle::Rounded)
            .with_style(Style::default().fg(Color::Indexed(250)).bg(Color::Black))
            .with_hdivider(HDIV1)
            .with_hdivider(HDIV2)
            .with_vdivider(VDIV1, HDIV1, HDIV2);
        page.set_root_widget(Box::new(frame));

        Self {
            data: PaletteData::new(),
            main_color: COLORS_WITH_NAME[0].1,
            main_color_similar: (0, 0, 0),
            named_colors: ncolors,
            gradient_input_colors: vec![],
            gradient_colors: vec![],
            random_colors: vec![],
            picker_colors: vec![],
            select: Select::new(),
            page,
            need_redraw: true,
            state: NameA,
        }
    }

    // ========== Buffer rendering (replaces UIPage.render) ==========

    pub fn get_rendered_buffer(&mut self) -> &Buffer {
        if self.need_redraw {
            self.redraw_all();
            self.need_redraw = false;
        }
        self.page.buffer()
    }

    fn redraw_all(&mut self) {
        // Render Panel frame (border + dividers)
        let _ = self.page.render();

        let buf = self.page.buffer_mut();
        // Draw menu (overwrites row 0)
        Self::draw_menu_to(buf, self.state);
        // Draw checkerboard + "Similar" label
        Self::draw_checkerboard(buf);
        // Draw main color info
        Self::draw_main_color_to(buf, self.main_color, self.main_color_similar);
        // Draw help message
        Self::draw_help_msg(buf, self.state);
        // Draw state-specific content
        match self.state {
            NameA => Self::draw_named_page(buf, &self.named_colors, 0),
            NameB => Self::draw_named_page(buf, &self.named_colors, 1),
            PickerA => Self::draw_picker_hsv(buf, &self.select, PICKER_COUNT_X),
            PickerB => Self::draw_picker_rgb(buf, &self.select),
            Random => Self::draw_random_to(buf, &self.random_colors),
            Gradient => {
                Self::draw_picker_hsv(buf, &self.select, PICKER_COUNT_X_GRADIENT);
                Self::draw_gradient_to(buf, &self.gradient_input_colors, &self.gradient_colors);
            }
            Golden => Self::draw_random_to(buf, &self.random_colors),
        }
        // Draw selection cursor (on top)
        Self::draw_select_to(buf, self.state, &self.select,
            &self.gradient_input_colors, &self.gradient_colors);
    }

    // ========== Checkerboard + labels ==========

    fn draw_checkerboard(buf: &mut Buffer) {
        // Color preview checkerboard frame (rows 24-31, cols 3-18)
        for y in 24..32 {
            buf.set_color_str(3, y, "▀▄", Color::Indexed(15), Color::Indexed(7));
            buf.set_color_str(17, y, "▀▄", Color::Indexed(15), Color::Indexed(7));
        }
        for x in (3..19).step_by(2) {
            buf.set_color_str(x as u16, 24, "▀▄", Color::Indexed(15), Color::Indexed(7));
            buf.set_color_str(x as u16, 31, "▀▄", Color::Indexed(15), Color::Indexed(7));
        }
        // "Similar" label (centered in cols 60-78)
        buf.set_color_str(66, 24, "Similar", Color::Indexed(7), Color::Black);
    }

    // ========== Menu ==========

    fn draw_menu_to(buf: &mut Buffer, state: PaletteState) {
        let cst = match state {
            NameA | NameB => 0,
            PickerA | PickerB => 1,
            Random => 2,
            Gradient => 3,
            Golden => 4,
        };
        // Full-width gray bar as menu background
        let bar_bg = Color::Indexed(237);
        for x in 0..PALETTEW {
            buf.set_color_str(x, 0, " ", Color::Reset, bar_bg);
        }
        // Title "Palette"
        buf.set_color_str(1, 0, "🌈 ", Color::Indexed(236), bar_bg);
        buf.set_color_str(4, 0, "P", Color::Indexed(111), bar_bg);
        buf.set_color_str(5, 0, "a", Color::Indexed(221), bar_bg);
        buf.set_color_str(6, 0, "l", Color::Indexed(116), bar_bg);
        buf.set_color_str(7, 0, "e", Color::Indexed(9), bar_bg);
        buf.set_color_str(8, 0, "t", Color::Indexed(2), bar_bg);
        buf.set_color_str(9, 0, "t", Color::Indexed(3), bar_bg);
        buf.set_color_str(10, 0, "e", Color::Indexed(13), bar_bg);

        let mut xoff = MENUX;
        let mcolor = [237u8, 120, 245, 0, 7, 120];
        if cst == 0 {
            buf.set_color_str(
                xoff, 0, MENU_SEP,
                Color::Indexed(mcolor[0]), Color::Indexed(mcolor[1]),
            );
        } else {
            buf.set_color_str(
                xoff, 0, MENU_SEP,
                Color::Indexed(mcolor[2]), Color::Indexed(mcolor[0]),
            );
        }
        let menu_title = [
            " 1 Named ",
            " 2 Picker ",
            " 3 Random ",
            " 4 Gradient ",
            " 5 Golden ",
        ];
        for i in 0..5 {
            let fg = if cst == i {
                Color::Indexed(mcolor[3])
            } else {
                Color::Indexed(mcolor[4])
            };
            let bg = if cst == i {
                Color::Indexed(mcolor[1])
            } else {
                Color::Indexed(mcolor[0])
            };
            let menu_str = &menu_title[i as usize];
            if cst == i {
                buf.set_color_str(xoff, 0, MENU_SEP, Color::Indexed(mcolor[0]), bg);
            }
            xoff += 1;
            buf.set_color_str(xoff, 0, menu_str, fg, bg);
            xoff += menu_str.len() as u16;
            if cst == i {
                buf.set_color_str(xoff, 0, MENU_SEP, bg, Color::Indexed(mcolor[0]));
            } else {
                buf.set_color_str(xoff, 0, MENU_SEP, Color::Indexed(mcolor[2]), bg);
            }
        }
    }

    // ========== Help message ==========

    fn draw_help_msg(buf: &mut Buffer, state: PaletteState) {
        let help = match state {
            NameA => "← ↑ → ↓ mouse : select named colors    n : colors list 2",
            NameB => "← ↑ → ↓ mouse : select named colors    n : colors list 1",
            PickerA => "tab : switch select area   ← ↑ → ↓ mouse : change value   n : rgb picker",
            PickerB => "tab : switch select area   ← ↑ → ↓ mouse : change value   n : hsv picker",
            Random => "← ↑ → ↓ mouse : select random colors",
            Gradient => "a : add input color  d : delete input color  tab ← ↑ → ↓ : change value",
            Golden => "← ↑ → ↓ mouse : select PHI(golden ratio) colors",
        };
        buf.set_color_str(ADJX + 1, ADJY + 31, help, Color::Gray, Color::Reset);
    }

    // ========== Main color info ==========

    fn draw_main_color_to(buf: &mut Buffer, main_color: ColorPro, similar: (usize, usize, usize)) {
        // Main color block (6 rows at 4,25)
        for i in 0..6u16 {
            buf.set_color_str(
                5, 25 + i,
                "            ",
                Color::White, Color::from(main_color),
            );
        }

        // Color info string
        buf.set_color_str(
            2, 23,
            &format!("{:width$}", &get_color_info(main_color, 0), width = 18),
            Color::DarkGray, Color::Black,
        );

        // Color space details
        for i in 0..MAIN_COLOR_MSG_Y {
            for j in 0..MAIN_COLOR_MSG_X {
                buf.set_color_str(
                    j * 20 + 22, 24 + i,
                    &format!(
                        "{:width$}",
                        &get_color_info(main_color, i * MAIN_COLOR_MSG_X + j + 1),
                        width = 18
                    ),
                    if i % 2 == 0 {
                        Color::Rgba(125, 130, 130, 255)
                    } else {
                        Color::Rgba(132, 132, 127, 255)
                    },
                    Color::Black,
                );
            }
        }

        // Similar colors (cols 60-78, max 19 chars)
        let ids = [similar.0, similar.1, similar.2];
        let sim_w = 19usize;
        for i in 0..3 {
            let s = COLORS_WITH_NAME[ids[i]].0;
            let cr = COLORS_WITH_NAME[ids[i]].1;
            let color = Color::from(cr);
            let name = if s.len() > sim_w { &s[..sim_w] } else { s };
            buf.set_color_str(
                60, 26 + i as u16 * 2,
                &format!("{:width$}", name, width = sim_w),
                if cr.is_dark() { Color::White } else { Color::Black },
                color,
            );
        }
    }

    // ========== Named colors ==========

    fn draw_named_page(buf: &mut Buffer, named_colors: &[(&str, ColorPro)], page: u16) {
        let base = (ROW_COUNT * COL_COUNT * page) as usize;
        for row in 0..ROW_COUNT {
            for col in 0..COL_COUNT {
                let idx = base + (row * COL_COUNT + col) as usize;
                if idx >= named_colors.len() {
                    break;
                }
                let s = named_colors[idx].0;
                let cr = named_colors[idx].1;
                let color = Color::from(cr);
                let max_w = C_WIDTH as usize - 1;
                let name = if s.len() > max_w { &s[..max_w] } else { s };
                buf.set_color_str(
                    ADJX + col * C_WIDTH + 1, ADJY + row,
                    &format!("{:width$}", name, width = max_w),
                    if cr.is_dark() { Color::White } else { Color::Black },
                    color,
                );
            }
        }
    }

    // ========== HSV Picker ==========

    fn draw_picker_hsv(buf: &mut Buffer, select: &Select, w: u16) {
        for y in 0..PICKER_COUNT_Y {
            for x in 0..w {
                let cr = get_pick_color(
                    w as usize, x as usize, y as usize,
                    select.ranges[1].x, 0,
                );
                let color = Color::from(cr);
                buf.set_color_str(ADJX + x, ADJY + y, " ", color, color);
            }
        }
        // Hue bar
        for i in 0..w {
            let cr = ColorPro::from_space_f64(
                HSVA, i as f64 * (360.0 / w as f64), 1.0, 1.0, 1.0,
            );
            let color = Color::from(cr);
            buf.set_color_str(ADJX + i, HUE_BAR_Y, " ", color, color);
        }
    }

    // ========== RGB Picker ==========

    fn draw_picker_rgb(buf: &mut Buffer, _select: &Select) {
        let pcs = [Color::Red, Color::Green, Color::Blue];
        for y in 0..3u16 {
            for x in 0..PICKER_COUNT_X {
                buf.set_color_str(ADJX + x, RGB_BAR_Y + y * 2, " ", Color::Reset, pcs[y as usize]);
            }
        }
    }

    // ========== Random / Golden colors ==========

    fn draw_random_to(buf: &mut Buffer, colors: &[ColorPro]) {
        for y in 0..RANDOM_Y {
            for x in 0..RANDOM_X {
                let i = (y * RANDOM_X + x) as usize;
                if i >= colors.len() { break; }
                let cr = colors[i];
                let rgb = cr.get_srgba_u8();
                let hex = format!("{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2);
                let label = format!("{:^width$}", hex, width = RANDOM_W as usize - 1);
                let fg = if cr.is_dark() { Color::White } else { Color::Black };
                buf.set_color_str(
                    ADJX + x * RANDOM_W, ADJY + y * 2,
                    &label,
                    fg, Color::from(cr),
                );
            }
        }
    }

    // ========== Gradient ==========

    fn draw_gradient_to(buf: &mut Buffer, input_colors: &[ColorPro], gradient_colors: &[ColorPro]) {
        // Input colors (right of vdivider, 8 chars wide)
        for i in 0..GRADIENT_INPUT_COUNT as usize {
            if i < input_colors.len() {
                let cr = input_colors[i];
                let rgb = cr.get_srgba_u8();
                let hex = format!("{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2);
                let label = format!("{:^8}", hex);
                let fg = if cr.is_dark() { Color::White } else { Color::Black };
                buf.set_color_str(
                    VDIV1 + 1, i as u16 + ADJY,
                    &label,
                    fg, Color::from(cr),
                );
            }
        }
        // Gradient output (9 chars wide)
        for y in 0..GRADIENT_Y {
            for x in 0..GRADIENT_X {
                let idx = (y * GRADIENT_X + x) as usize;
                if idx < gradient_colors.len() {
                    let cr = gradient_colors[idx];
                    let rgb = cr.get_srgba_u8();
                    let hex = format!("{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2);
                    let label = format!("{:^9}", hex);
                    let fg = if cr.is_dark() { Color::White } else { Color::Black };
                    buf.set_color_str(
                        VDIV1 + 10, y + ADJY,
                        &label,
                        fg, Color::from(cr),
                    );
                }
            }
        }
    }

    // ========== Selection cursor ==========

    fn draw_select_to(
        buf: &mut Buffer,
        state: PaletteState,
        select: &Select,
        gradient_input_colors: &[ColorPro],
        gradient_colors: &[ColorPro],
    ) {
        match state {
            NameA | NameB => {
                let cx = ADJX + select.ranges[0].x as u16 * C_WIDTH;
                let cy = ADJY + select.ranges[0].y as u16;
                buf.set_color_str(cx, cy, PL_ARROW, Color::Green, Color::Black);
            }
            Random | Golden => {
                let cx = ADJX - 1 + select.ranges[0].x as u16 * RANDOM_W;
                let cy = ADJY + select.ranges[0].y as u16 * 2;
                buf.set_color_str(cx, cy, PL_ARROW, Color::Green, Color::Black);
            }
            PickerB => {
                // Area indicator
                let idx = select.area;
                buf.set_color_str(ADJX - 1, RGB_BAR_Y + idx as u16 * 2, PL_ARROW, Color::Green, Color::Black);
                // RGB slider dots
                let bcs = [Color::Red, Color::Green, Color::Blue];
                for i in 0..3 {
                    let x = select.ranges[i].x;
                    let px = (x as f64 / 256.0 * PICKER_COUNT_X as f64) as u16 + ADJX;
                    buf.set_color_str(px, RGB_BAR_Y + i as u16 * 2, "∙", Color::Black, bcs[i]);
                }
            }
            PickerA | Gradient => {
                let idx = select.area;
                // Area indicator arrow
                if state == PickerA {
                    buf.set_color_str(ADJX - 1, idx as u16 * PICKER_COUNT_Y + ADJY, PL_ARROW, Color::Green, Color::Black);
                } else if idx < 3 {
                    buf.set_color_str(
                        ADJX - 1 + idx as u16 / 2 * (PICKER_COUNT_X_GRADIENT + 1),
                        idx as u16 % 2 * PICKER_COUNT_Y + ADJY,
                        PL_ARROW, Color::Green, Color::Black,
                    );
                } else {
                    buf.set_color_str(
                        PICKER_COUNT_X_GRADIENT + 11,
                        (idx - 1) as u16 % 2 * PICKER_COUNT_Y + ADJY,
                        PL_ARROW, Color::Green, Color::Black,
                    );
                }

                // Picker cursor dot
                let w = if state == PickerA { PICKER_COUNT_X } else { PICKER_COUNT_X_GRADIENT };
                let cr = get_pick_color(
                    w as usize,
                    select.ranges[0].x, select.ranges[0].y,
                    select.ranges[1].x, 0,
                );
                buf.set_color_str(
                    select.ranges[0].x as u16 + ADJX,
                    select.ranges[0].y as u16 + ADJY,
                    "∙",
                    if cr.is_dark() { Color::White } else { Color::Black },
                    Color::from(cr),
                );

                // Hue bar cursor dot
                let cr2 = get_pick_color(
                    w as usize,
                    select.ranges[0].x, select.ranges[0].y,
                    select.ranges[1].x, 1,
                );
                buf.set_color_str(
                    (select.ranges[1].x / 4) as u16 + ADJX, HUE_BAR_Y,
                    "∙",
                    if cr2.is_dark() { Color::White } else { Color::Black },
                    Color::from(cr2),
                );

                // Gradient-specific cursors (only show active area)
                if state == Gradient {
                    // Clear old ▸ markers
                    for row in 0..GRADIENT_Y {
                        buf.set_color_str(VDIV1, row + ADJY, " ", Color::Reset, Color::Black);
                        buf.set_color_str(VDIV1 + 9, row + ADJY, " ", Color::Reset, Color::Black);
                    }
                    if idx == 2 && !gradient_input_colors.is_empty() {
                        buf.set_color_str(
                            VDIV1,
                            select.ranges[2].y as u16 + ADJY,
                            PL_ARROW,
                            Color::Green, Color::Black,
                        );
                    }
                    if idx == 3 && !gradient_colors.is_empty() {
                        buf.set_color_str(
                            VDIV1 + 9,
                            select.ranges[3].y as u16 + ADJY,
                            PL_ARROW,
                            Color::Green, Color::Black,
                        );
                    }
                }
            }
        }
    }

    // ========== Mouse hit testing ==========

    fn mouse_in(&mut self, x: u16, y: u16) -> Option<MouseArea> {
        // Menu bar (row 0)
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

        // Main content area: rows ADJY..HDIV1, cols ADJX..CONTENT_X2
        let in_content = x >= CONTENT_X1 && x <= CONTENT_X2 && y >= CONTENT_Y1 && y <= CONTENT_Y2;
        let cx = x.saturating_sub(ADJX); // content-relative x
        let cy = y.saturating_sub(ADJY); // content-relative y

        match self.state {
            NameA | NameB => {
                if x >= ADJX && y >= ADJY {
                    let a = (x - ADJX) / C_WIDTH;
                    let b = y - ADJY;
                    if (b as usize * self.select.cur().width + a as usize) < self.select.cur().count {
                        return Some(MouseArea::Named(a, b));
                    }
                }
            }
            PickerA => {
                // SV picker area
                if in_content && cx < PICKER_COUNT_X && cy < PICKER_COUNT_Y {
                    return Some(MouseArea::Picker(0, 0, cx, cy));
                }
                // Hue bar
                if y == HUE_BAR_Y && x >= ADJX && cx < PICKER_COUNT_X {
                    return Some(MouseArea::Picker(0, 1, (cx as f64 * 4.0) as u16, 0));
                }
            }
            PickerB => {
                let picker_x2 = ADJX + PICKER_COUNT_X - 1;
                if x >= ADJX && x <= picker_x2 {
                    let c = ((x - 1) as f64 / PICKER_COUNT_X as f64 * 255.0) as u16;
                    for i in 0..3u16 {
                        if y == RGB_BAR_Y + i * 2 {
                            return Some(MouseArea::Picker(1, i, c, 0));
                        }
                    }
                }
            }
            Random => {
                if y >= ADJY && y < ADJY + RANDOM_Y * 2 && x >= 1 && x <= CONTENT_X2 {
                    let rel_y = y - ADJY;
                    if rel_y % 2 == 0 {
                        let a = (x - 1) / RANDOM_W;
                        let b = rel_y / 2;
                        return Some(MouseArea::Random(a, b));
                    }
                }
            }
            Golden => {
                if y >= ADJY && y < ADJY + RANDOM_Y * 2 && x >= 1 && x <= CONTENT_X2 {
                    let rel_y = y - ADJY;
                    if rel_y % 2 == 0 {
                        let a = (x - 1) / RANDOM_W;
                        let b = rel_y / 2;
                        return Some(MouseArea::Golden(a, b));
                    }
                }
            }
            Gradient => {
                let gw = PICKER_COUNT_X_GRADIENT;
                let picker_x2 = ADJX + gw - 1;
                // SV picker area
                if y >= ADJY && cy < PICKER_COUNT_Y && x >= ADJX && x <= picker_x2 {
                    return Some(MouseArea::Gradient(0, cx, cy));
                }
                // Hue bar
                if y == HUE_BAR_Y && x >= ADJX && x <= picker_x2 {
                    return Some(MouseArea::Gradient(1, ((cx) as f64 * 4.0) as u16, 0));
                }
                // Input colors (right of vdivider)
                if y >= ADJY && y < ADJY + GRADIENT_INPUT_COUNT
                    && x >= VDIV1 + 1 && x <= VDIV1 + 8
                    && (cy as usize) < self.select.ranges[2].count
                {
                    return Some(MouseArea::Gradient(2, 0, cy));
                }
                // Gradient output
                if y >= ADJY && y <= HUE_BAR_Y
                    && x >= VDIV1 + 10 && x <= CONTENT_X2
                    && (cy as usize) < self.select.ranges[3].count
                {
                    return Some(MouseArea::Gradient(3, 0, cy));
                }
            }
        }
        None
    }

    // ========== State management ==========

    fn do_random(&mut self) {
        if self.state != Random { return; }
        random(
            RANDOM_X as usize * RANDOM_Y as usize,
            &mut self.data.rand,
            &mut self.random_colors,
        );
    }

    fn do_golden(&mut self) {
        if self.state != Golden { return; }
        golden(
            RANDOM_X as usize * RANDOM_Y as usize,
            &mut self.data.rand,
            &mut self.random_colors,
        );
    }

    fn do_gradient(&mut self) {
        if self.state != Gradient { return; }
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
        self.update_main_color();
    }

    fn add_gradient_input(&mut self) {
        if self.state != Gradient { return; }
        if self.gradient_input_colors.len() >= GRADIENT_INPUT_COUNT as usize { return; }
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
        self.do_gradient();
    }

    fn del_gradient_input(&mut self) {
        if self.state != Gradient { return; }
        if self.gradient_input_colors.is_empty() { return; }
        self.gradient_input_colors.pop();
        self.select.ranges[2] = SelectRange::new(
            1,
            self.gradient_input_colors.len(),
            self.gradient_input_colors.len(),
        );
        self.do_gradient();
    }

    fn update_select_by_main_color(&mut self, mc: ColorPro) {
        self.main_color = mc;
        match self.state {
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
            _ => {}
        }
        self.main_color_similar = find_similar_colors(&self.main_color);
        self.need_redraw = true;
    }

    fn update_main_color(&mut self) {
        match self.state {
            NameA => {
                self.main_color = self.named_colors
                    [self.select.cur().y * self.select.cur().width + self.select.cur().x].1;
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
        self.main_color_similar = find_similar_colors(&self.main_color);
        self.need_redraw = true;
    }

    fn switch_state(&mut self, st: PaletteState) {
        self.state = st;
        match st {
            NameA => {
                self.select.clear();
                self.select.add_range(SelectRange::new(
                    COL_COUNT as usize, ROW_COUNT as usize,
                    (COL_COUNT * ROW_COUNT) as usize,
                ));
                self.update_main_color();
            }
            NameB => {
                self.select.clear();
                self.select.add_range(SelectRange::new(
                    COL_COUNT as usize, ROW_COUNT as usize - 3,
                    self.named_colors.len() - (COL_COUNT * ROW_COUNT) as usize,
                ));
                self.update_main_color();
            }
            PickerA => {
                self.select.clear();
                let w = PICKER_COUNT_X as usize;
                let h = PICKER_COUNT_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.select.add_range(SelectRange::new(w * 4, 1, w * 4));
                self.update_main_color();
            }
            PickerB => {
                self.select.clear();
                self.select.add_range(SelectRange::new(256, 1, 256));
                self.select.add_range(SelectRange::new(256, 1, 256));
                self.select.add_range(SelectRange::new(256, 1, 256));
                self.update_main_color();
            }
            Random => {
                self.select.clear();
                let w = RANDOM_X as usize;
                let h = RANDOM_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.do_random();
                self.update_main_color();
            }
            Gradient => {
                self.select.clear();
                let w = PICKER_COUNT_X_GRADIENT as usize;
                let h = PICKER_COUNT_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.select.add_range(SelectRange::new(w * 4, 1, w * 4));
                self.select.add_range(SelectRange::new(
                    1, self.gradient_input_colors.len(),
                    self.gradient_input_colors.len(),
                ));
                self.select.add_range(SelectRange::new(
                    GRADIENT_X as usize, GRADIENT_Y as usize,
                    GRADIENT_COUNT as usize,
                ));
                self.do_gradient();
                self.update_main_color();
            }
            Golden => {
                self.select.clear();
                let w = RANDOM_X as usize;
                let h = RANDOM_Y as usize;
                self.select.add_range(SelectRange::new(w, h, w * h));
                self.do_golden();
                self.update_main_color();
            }
        }
    }
}

impl Model for PaletteModel {
    fn init(&mut self, _context: &mut Context) {
        self.data.shuffle();

        let ctest = ColorPro::from_space_f64(SRGBA, 0.0, 1.0, 0.0, 1.0);
        for i in 0..COLOR_SPACE_COUNT {
            info!(
                "{}:{:?}",
                ColorSpace::from_usize(i).unwrap(),
                ctest.space_matrix[i].unwrap()
            );
        }

        // Init gradient input colors
        self.gradient_input_colors = vec![
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 1.0, 0.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 0.0, 1.0, 1.0, 1.0),
            ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.8, 1.0),
        ];

        // Init hsv picker colors
        for y in 0..PICKER_COUNT_X {
            for x in 0..PICKER_COUNT_Y {
                let rx = (x as f64) / (PICKER_COUNT_X as f64);
                let ry = (y as f64) / (PICKER_COUNT_X as f64);
                let h = 360.0 * rx;
                let s = 0.6;
                let l = 0.95 * ry;
                let color1 = ColorPro::from_space_f64(HSLA, h, s, l, 1.0);
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

        self.switch_state(NameA);
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Mouse(mou) => {
                    if mou.kind == Up(MouseButton::Left) {
                        match self.mouse_in(mou.column, mou.row) {
                            Some(MouseArea::Menu(i)) => {
                                let sts = [NameA, PickerA, Random, Gradient, Golden];
                                self.switch_state(sts[i as usize]);
                            }
                            Some(MouseArea::Named(a, b)) => {
                                self.select.cur().x = a as usize;
                                self.select.cur().y = b as usize;
                                self.update_main_color();
                            }
                            Some(MouseArea::Picker(_a, b, c, d)) => {
                                self.select.area = b as usize;
                                self.select.ranges[b as usize].x = c as usize;
                                self.select.ranges[b as usize].y = d as usize;
                                self.update_main_color();
                            }
                            Some(MouseArea::Random(a, b)) => {
                                self.select.cur().x = a as usize;
                                self.select.cur().y = b as usize;
                                self.update_main_color();
                            }
                            Some(MouseArea::Gradient(b, c, d)) => {
                                self.select.area = b as usize;
                                self.select.ranges[b as usize].x = c as usize;
                                self.select.ranges[b as usize].y = d as usize;
                                self.update_main_color();
                            }
                            Some(MouseArea::Golden(a, b)) => {
                                self.select.cur().x = a as usize;
                                self.select.cur().y = b as usize;
                                self.update_main_color();
                            }
                            _ => {}
                        }
                    }
                }
                Event::Key(key) => match key.code {
                    KeyCode::Char('1') => {
                        self.switch_state(NameA);
                    }
                    KeyCode::Char('n') => {
                        if self.state == NameA {
                            self.switch_state(NameB);
                        } else if self.state == NameB {
                            self.switch_state(NameA);
                        } else if self.state == PickerA {
                            let mc = self.main_color;
                            self.switch_state(PickerB);
                            self.update_select_by_main_color(mc);
                        } else if self.state == PickerB {
                            let mc = self.main_color;
                            self.switch_state(PickerA);
                            self.update_select_by_main_color(mc);
                        }
                    }
                    KeyCode::Char('2') => {
                        self.switch_state(PickerA);
                    }
                    KeyCode::Char('3') => {
                        self.switch_state(Random);
                    }
                    KeyCode::Char('4') => {
                        self.switch_state(Gradient);
                    }
                    KeyCode::Char('5') => {
                        self.switch_state(Golden);
                    }
                    KeyCode::Char('a') => {
                        self.add_gradient_input();
                    }
                    KeyCode::Char('d') => {
                        self.del_gradient_input();
                    }
                    KeyCode::Char('g') => {
                        self.do_gradient();
                    }
                    KeyCode::Up => {
                        self.select.cur().backward_y();
                        self.update_main_color();
                    }
                    KeyCode::Down => {
                        self.select.cur().forward_y();
                        self.update_main_color();
                    }
                    KeyCode::Left => {
                        self.select.cur().backward_x();
                        self.update_main_color();
                    }
                    KeyCode::Right => {
                        self.select.cur().forward_x();
                        self.update_main_color();
                    }
                    KeyCode::Tab => {
                        self.select.switch_area();
                        self.update_main_color();
                    }
                    _ => {}
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
                    rgb.0, rgb.1, rgb.2, rgb.0, rgb.1, rgb.2,
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
