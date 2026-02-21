use crate::model::TetrisModel;
use tetris_lib::constant::*;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_exdata, timer_stage},
    game::Render,
    render::scene::Scene,
    render::sprite::Sprite,
    render::style::Color,
    ui::{Panel, Widget},
    util::Rect,
};

const CANVAS_W: u16 = 80;
const CANVAS_H: u16 = 30;

// Colors for stats overlay
const VALUE_FG: Color = Color::White;
const DIM_FG: Color = Color::Indexed(244);
const BG: Color = Color::Reset;

// Grid positions (absolute coordinates within the 80×30 canvas)
const GRID0_X: u16 = 2;
const GRID0_Y: u16 = 7;
const GRID1_X: u16 = 55;
const GRID1_Y: u16 = 7;
const NEXT_X: u16 = 27;
const NEXT_Y: u16 = 8;
const HOLD_X: u16 = 42;
const HOLD_Y: u16 = 8;

pub struct TetrisRender {
    pub scene: Scene,
    /// Main panel for TUI character rendering
    pub panel: Panel,
    /// Background buffer cloned from back.txt asset
    pub back_buf: Option<rust_pixel::render::buffer::Buffer>,
}

impl TetrisRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Temp sprite for loading back.txt asset
        let tsback = Sprite::new(0, 0, CANVAS_W, CANVAS_H);
        scene.add_sprite(tsback, "back");

        // Main panel covering the whole canvas (no border — back.txt provides borders)
        let panel = Panel::new()
            .with_bounds(Rect::new(0, 0, CANVAS_W, CANVAS_H));

        event_register("Tetris.RedrawNext", "redraw_next");
        event_register("Tetris.RedrawHold", "redraw_hold");
        event_register("Tetris.RedrawMsg", "redraw_msg");

        Self {
            scene,
            panel,
            back_buf: None,
        }
    }

    fn draw_stats(&mut self, data: &TetrisModel) {
        let stat = &data.sides[0].stat;
        let sx: u16 = 28;

        self.panel.set_color_str(sx, 16, "Lines:", DIM_FG, BG);
        let lines_str = format!("  {:<6}", stat.clear_lines);
        self.panel.set_color_str(sx, 17, &lines_str, VALUE_FG, BG);

        self.panel.set_color_str(sx, 19, "Score:", DIM_FG, BG);
        let score_str = format!("  {:<6}", stat.score);
        self.panel.set_color_str(sx, 20, &score_str, VALUE_FG, BG);

        self.panel.set_color_str(sx, 22, "Level:", DIM_FG, BG);
        let level_str = format!("  {:<6}", stat.level);
        self.panel.set_color_str(sx, 23, &level_str, VALUE_FG, BG);

        if stat.combo_current > 0 {
            let combo_str = format!("Combo: {:<3}", stat.combo_current);
            self.panel.set_color_str(sx, 25, &combo_str, Color::LightYellow, BG);
        } else {
            self.panel.set_color_str(sx, 25, "              ", BG, BG);
        }

        if data.sides[0].core.game_over {
            self.panel.set_color_str(sx, 15, " GAME OVER! ", Color::LightRed, BG);
        }

        // Keybinds
        let kx: u16 = 39;
        let key_fg = Color::LightCyan;
        self.panel.set_color_str(kx, 16, "KEYS", VALUE_FG, BG);
        self.panel.set_color_str(kx, 17, "────────", DIM_FG, BG);
        self.panel.set_color_str(kx, 18, "o", key_fg, BG);
        self.panel.set_color_str(kx + 1, 18, "/", DIM_FG, BG);
        self.panel.set_color_str(kx + 2, 18, "i", key_fg, BG);
        self.panel.set_color_str(kx + 4, 18, "turn", DIM_FG, BG);
        self.panel.set_color_str(kx, 19, "j", key_fg, BG);
        self.panel.set_color_str(kx + 1, 19, "/", DIM_FG, BG);
        self.panel.set_color_str(kx + 2, 19, "l", key_fg, BG);
        self.panel.set_color_str(kx + 4, 19, "move", DIM_FG, BG);
        self.panel.set_color_str(kx, 20, "k", key_fg, BG);
        self.panel.set_color_str(kx + 4, 20, "down", DIM_FG, BG);
        self.panel.set_color_str(kx, 21, "s", key_fg, BG);
        self.panel.set_color_str(kx + 4, 21, "hold", DIM_FG, BG);
        self.panel.set_color_str(kx, 22, "spc", key_fg, BG);
        self.panel.set_color_str(kx + 4, 22, "drop", DIM_FG, BG);
        self.panel.set_color_str(kx, 23, "r", key_fg, BG);
        self.panel.set_color_str(kx + 4, 23, "new", DIM_FG, BG);
    }

    fn set_block_on(panel: &mut Panel, bx: u16, by: u16, x: u16, y: u16, c: u8) {
        let cv = [
            Color::Magenta,
            Color::Cyan,
            Color::LightRed,
            Color::LightGreen,
            Color::LightBlue,
            Color::LightYellow,
            Color::LightMagenta,
            Color::LightCyan,
        ];

        let c1: &str;
        let c2: &str;
        let bg: Color;
        let fg: Color;

        match c {
            0 => {
                c1 = " ";
                c2 = " ";
                fg = Color::Reset;
                bg = Color::Reset;
            }
            11 => {
                c1 = "█";
                c2 = "█";
                fg = Color::Indexed(240);
                bg = Color::Reset;
            }
            20 => {
                c1 = "░";
                c2 = "░";
                fg = Color::Indexed(242);
                bg = Color::Reset;
            }
            30 => {
                c1 = "-";
                c2 = "=";
                fg = Color::Indexed(231);
                bg = Color::Reset;
            }
            _ => {
                c1 = "█";
                c2 = "█";
                fg = cv[(c % 100) as usize % cv.len()];
                bg = Color::Reset;
            }
        }

        let ax = bx + x;
        let ay = by + y;
        if x < HENG * 2 {
            panel.set_color_str(ax, ay, c1, fg, bg);
        }
        if x + 1 < HENG * 2 {
            panel.set_color_str(ax + 1, ay, c2, fg, bg);
        }
    }

    pub fn redraw_hold(&mut self, d: &mut TetrisModel) {
        // Clear hold area
        for iy in 0..4u16 {
            for ix in 0..8u16 {
                self.panel.set_color_str(HOLD_X + ix, HOLD_Y + iy, " ", BG, BG);
            }
        }
        if d.sides[0].core.save_block < 0 {
            return;
        }
        for i in 0..4 {
            for j in 0..4 {
                let rx = j * 2;
                if d.sides[0].get_md(d.sides[0].core.save_block, 0, i * 4 + j) != 0 {
                    Self::set_block_on(
                        &mut self.panel,
                        HOLD_X, HOLD_Y,
                        rx as u16, i as u16,
                        d.sides[0].core.save_block as u8 + 1,
                    );
                } else {
                    Self::set_block_on(&mut self.panel, HOLD_X, HOLD_Y, rx as u16, i as u16, 0);
                }
            }
        }
    }

    pub fn redraw_next(&mut self, d: &mut TetrisModel) {
        // Clear next area
        for iy in 0..4u16 {
            for ix in 0..8u16 {
                self.panel.set_color_str(NEXT_X + ix, NEXT_Y + iy, " ", BG, BG);
            }
        }
        for i in 0..4 {
            for j in 0..4 {
                let rx = j * 2;
                if d.sides[0].get_md(d.sides[0].core.next_block, 0, i * 4 + j) != 0 {
                    Self::set_block_on(
                        &mut self.panel,
                        NEXT_X, NEXT_Y,
                        rx as u16, i as u16,
                        d.sides[0].core.next_block as u8 + 1,
                    );
                } else {
                    Self::set_block_on(&mut self.panel, NEXT_X, NEXT_Y, rx as u16, i as u16, 0);
                }
            }
        }
    }

    /// Compute gradient color for hard-drop trail using xterm-256 6x6x6 cube.
    fn trail_color(color_idx: usize, ratio: f32) -> Color {
        const BASE_RGB: [[f32; 3]; 8] = [
            [5.0, 0.0, 5.0],
            [0.0, 5.0, 5.0],
            [5.0, 1.0, 1.0],
            [1.0, 5.0, 1.0],
            [1.0, 1.0, 5.0],
            [5.0, 5.0, 1.0],
            [5.0, 1.0, 5.0],
            [1.0, 5.0, 5.0],
        ];
        let rgb = BASE_RGB[color_idx % 8];
        let scale = 1.0 - ratio * 0.8;
        let r = (rgb[0] * scale).round().clamp(0.0, 5.0) as u8;
        let g = (rgb[1] * scale).round().clamp(0.0, 5.0) as u8;
        let b = (rgb[2] * scale).round().clamp(0.0, 5.0) as u8;
        Color::Indexed(16 + 36 * r + 6 * g + b)
    }

    const TRAIL_GLYPHS: [(&'static str, &'static str); 8] = [
        ("█", "█"),
        ("▓", "▓"),
        ("▒", "▒"),
        ("░", "░"),
        ("│", "│"),
        (":", ":"),
        ("·", "·"),
        (".", "."),
    ];

    pub fn draw_grid(&mut self, _context: &mut Context, d: &mut TetrisModel) {
        let grid_pos: [(u16, u16); 2] = [(GRID0_X, GRID0_Y), (GRID1_X, GRID1_Y)];

        for n in 0..2 {
            let (gx, gy) = grid_pos[n];
            let frs = timer_stage(&format!("clear-row{}", n));
            let mut fri: Vec<i8> = vec![];
            if frs != 0 {
                let fr = timer_exdata(&format!("clear-row{}", n)).unwrap();
                fri = bincode::serde::decode_from_slice(&fr, bincode::config::standard())
                    .unwrap()
                    .0;
            }
            for i in 0..ZONG {
                for j in 0..HENG {
                    let rx = j * 2;
                    let gv = d.sides[n].get_gd(i as i8, (j + 2) as i8);
                    match gv {
                        0 => {
                            Self::set_block_on(&mut self.panel, gx, gy, rx, i, 0);
                        }
                        _ => {
                            let mut hidden_fullrow = false;
                            if frs != 0 {
                                if fri.contains(&(i as i8)) && frs / 3 % 2 == 0 {
                                    hidden_fullrow = true;
                                }
                            }
                            if hidden_fullrow {
                                Self::set_block_on(&mut self.panel, gx, gy, rx, i, 30);
                            } else {
                                Self::set_block_on(&mut self.panel, gx, gy, rx, i, gv % 100);
                            }
                        }
                    }
                }
            }
            // Hard-drop trail
            let fall_stage = timer_stage(&format!("fall{}", n));
            if fall_stage != 0 {
                let block = d.sides[n].core.cur_block;
                let z = d.sides[n].core.cur_z;
                let cy = d.sides[n].core.cur_y;
                let cx = d.sides[n].core.cur_x;
                let sy = d.sides[n].core.shadow_y;
                let mut trail_cells: Vec<(u16, u16, usize, usize)> = Vec::new();
                for j in 0..4i8 {
                    let mut bottom = -1i8;
                    let mut trail_md: u8 = 0;
                    for i in (0..4i8).rev() {
                        if d.sides[n].get_md(block, z, i * 4 + j) != 0 {
                            bottom = i;
                            trail_md = d.sides[n].get_md(block, z, i * 4 + j);
                            break;
                        }
                    }
                    if bottom >= 0 {
                        let col = cx + j;
                        let trail_start = cy + bottom + 1;
                        let trail_end = sy + bottom;
                        let ci = (trail_md as usize) % 8;
                        for ty in trail_start..trail_end {
                            if ty >= 0 && (ty as u16) < ZONG
                                && d.sides[n].is_in_grid(ty, col)
                                && d.sides[n].get_gd(ty, col) == 0
                            {
                                let rx = (col - 2) * 2;
                                if rx >= 0 && (rx as u16) < HENG * 2 {
                                    let y_offset = (trail_end - 1 - ty) as usize;
                                    trail_cells.push((rx as u16, ty as u16, ci, y_offset));
                                }
                            }
                        }
                    }
                }
                let dt = (6u32.saturating_sub(fall_stage)) as usize;
                for &(rx, ty, ci, y_offset) in &trail_cells {
                    let glyph_idx = y_offset * 4 / 7 + dt;
                    let ax = gx + rx;
                    let ay = gy + ty;
                    if glyph_idx >= Self::TRAIL_GLYPHS.len() {
                        self.panel.set_color_str(ax, ay, " ", Color::Reset, Color::Reset);
                        self.panel.set_color_str(ax + 1, ay, " ", Color::Reset, Color::Reset);
                    } else {
                        let ratio = glyph_idx as f32 / 7.0;
                        let fg = Self::trail_color(ci, ratio);
                        let (c1, c2) = Self::TRAIL_GLYPHS[glyph_idx];
                        self.panel.set_color_str(ax, ay, c1, fg, Color::Reset);
                        self.panel.set_color_str(ax + 1, ay, c2, fg, Color::Reset);
                    }
                }
            }

            // Shadow (same color as block, dimmed)
            let shadow_ci = (d.sides[n].core.cur_block as usize + 1) % 8;
            let shadow_fg = Self::trail_color(shadow_ci, 0.6);
            for i in 0..4 {
                for j in 0..4 {
                    let ttx = d.sides[n].core.shadow_x + j;
                    let tty = d.sides[n].core.shadow_y + i;
                    if d.sides[n].is_in_grid(tty, ttx) {
                        if d.sides[n].get_md(
                            d.sides[n].core.cur_block,
                            d.sides[n].core.cur_z,
                            i * 4 + j,
                        ) != 0
                        {
                            let rx = ttx * 2 - 4;
                            if d.sides[n].get_gd(tty, ttx) == 0 {
                                let ax = gx + rx as u16;
                                let ay = gy + tty as u16;
                                self.panel.set_color_str(ax, ay, "░", shadow_fg, Color::Reset);
                                self.panel.set_color_str(ax + 1, ay, "░", shadow_fg, Color::Reset);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Render for TetrisRender {
    type Model = TetrisModel;

    fn init(&mut self, context: &mut Context, _data: &mut Self::Model) {
        context.adapter.get_base().gr.set_use_tui_height(true);
        context
            .adapter
            .init(CANVAS_W, CANVAS_H, 2.0, 2.0, "tetris".to_string());
        self.scene.init(context);

        // Load back.txt via temp sprite, clone buffer, hide sprite
        let l = self.scene.get_sprite("back");
        let bp = "back.txt";
        asset2sprite!(l, context, &bp);
        self.back_buf = Some(l.content.clone());
        l.set_hidden(true);
    }

    fn draw(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        // Reset canvas and merge background
        {
            let canvas = self.panel.canvas_mut();
            canvas.reset();
            if let Some(ref back) = self.back_buf {
                canvas.merge(back, 255, true);
            }
        }
        // Draw all dynamic content onto panel canvas
        self.draw_stats(data);
        self.draw_grid(context, data);
        self.redraw_next(data);
        self.redraw_hold(data);
        // Render panel to tui_buffer, then draw scene
        let buffer = self.scene.tui_buffer_mut();
        let _ = self.panel.render(buffer, context);
        self.scene.draw(context).unwrap();
    }

    fn handle_event(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {
        // Consume events (drawing happens in draw() every frame)
        event_check("Tetris.RedrawNext", "redraw_next");
        event_check("Tetris.RedrawHold", "redraw_hold");
    }

    fn handle_timer(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}
}
