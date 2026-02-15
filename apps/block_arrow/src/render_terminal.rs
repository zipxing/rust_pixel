#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(non_camel_case_types)]
use crate::model::{Block_arrowModel, FlyAnim, CELLH, CELLW, BLOCK_ARROWH, BLOCK_ARROWW};
use block_arrow_lib::Direction;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::scene::Scene,
    render::sprite::Sprite,
    render::style::{Color, Style},
    util::Rect,
};

// Bitmap color index: 1-蓝 2-绿 3-红 4-木头褐 5-黄 6-深灰 7-亮红 8-白
const COLORS: [Color; 15] = [
    Color::Rgba(0xc0, 0xc0, 0xc0, 0xff), // 0: (unused/wall)
    Color::Rgba(0x10, 0x73, 0xdd, 0xff), // 1: 蓝 Blue
    Color::Rgba(0x19, 0xce, 0x27, 0xff), // 2: 绿 Green
    Color::Rgba(0xe9, 0x05, 0x05, 0xff), // 3: 红 Red
    Color::Rgba(0x8B, 0x5E, 0x3C, 0xff), // 4: 木头褐 Wood Brown
    Color::Rgba(0xff, 0xd3, 0x22, 0xff), // 5: 黄 Yellow
    Color::Rgba(0x66, 0x66, 0x66, 0xff), // 6: 深灰 Dark Gray
    Color::Rgba(0xff, 0x45, 0x45, 0xff), // 7: 亮红 Bright Red
    Color::Rgba(0xff, 0xff, 0xff, 0xff), // 8: 白 White
    Color::Rgba(0xb3, 0x52, 0xff, 0xff), // 9: Purple
    Color::Rgba(0xfc, 0x86, 0x2a, 0xff), // 10: Orange
    Color::Rgba(0xe5, 0x5a, 0xf1, 0xff), // 11: LightPurple
    Color::Rgba(0x28, 0xc8, 0xff, 0xff), // 12: Skyblue
    Color::Rgba(0x21, 0x79, 0x37, 0xff), // 13: DarkGreen
    Color::Rgba(0x73, 0xfb, 0xfd, 0xff), // 14: LightBlue
];

// Max cells: 16×16 = 256
const MAX_CELLS: usize = 256;

pub struct Block_arrowRender {
    pub scene: Scene,
}

impl Block_arrowRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Background sprite
        let back = Sprite::new(0, 0, BLOCK_ARROWW, BLOCK_ARROWH);
        scene.add_sprite(back, "back");

        // Cell sprites for board (up to MAX_CELLS)
        for i in 0..MAX_CELLS {
            scene.add_sprite(
                Sprite::new(0, 0, CELLW as u16, CELLH as u16),
                &format!("cc{}", i),
            );
        }

        // Status bar sprites (below the board)
        scene.add_sprite(Sprite::new(1, BLOCK_ARROWH - 2, 90, 1), "status");
        scene.add_sprite(Sprite::new(1, BLOCK_ARROWH - 1, 90, 1), "msg");

        event_register("BlockArrow.Redraw", "redraw");

        Self { scene }
    }

    fn dim_color(c: Color, factor: u8) -> Color {
        match c {
            Color::Rgba(r, g, b, a) => Color::Rgba(
                (r as u16 * factor as u16 / 255) as u8,
                (g as u16 * factor as u16 / 255) as u8,
                (b as u16 * factor as u16 / 255) as u8,
                a,
            ),
            other => other,
        }
    }

    pub fn draw_cell(
        &mut self,
        ctx: &mut Context,
        sprite_key: &str,
        x: u16,
        y: u16,
        border_type: u8,
        border_color: i8,
        msg: &str,
        msg_color: i8,
        flash: bool,
        alpha: u8, // 255 = fully opaque, used for fly fade
    ) {
        let l = self.scene.get_sprite(sprite_key);
        let area = Rect::new(0, 0, CELLW as u16, CELLH as u16);
        l.content.resize(area);
        l.content.reset();
        let cn = format!("cc{}.txt", border_type);
        asset2sprite!(l, ctx, &cn);
        l.set_pos(x, y);
        let base_fg = COLORS[border_color as usize % COLORS.len()];
        let fg = Self::dim_color(base_fg, alpha);
        let bg = Self::dim_color(fg, 140);
        if flash {
            l.content.set_style(
                l.content.area,
                Style::default()
                    .fg(Color::Rgba(0xff, 0xff, 0xff, 0xff))
                    .bg(fg),
            );
        } else {
            l.content.set_style(
                l.content.area,
                Style::default().fg(fg).bg(bg),
            );
        }
        if !msg.is_empty() {
            let msg_bg = if flash { fg } else { bg };
            let msg_fg = if flash {
                Color::Rgba(0xff, 0xff, 0xff, 0xff)
            } else {
                Self::dim_color(COLORS[msg_color as usize % COLORS.len()], alpha)
            };
            l.set_color_str(3, 2, msg, msg_fg, msg_bg);
        }
    }

    pub fn draw_board(&mut self, ctx: &mut Context, d: &mut Block_arrowModel) {
        let w = d.board_width;
        let h = d.board_height;

        // Clear background
        {
            let back = self.scene.get_sprite("back");
            back.content.reset();
        }

        // Clear all cell sprites (board + anim slots) — must resize+reset
        for i in 0..MAX_CELLS {
            let l = self.scene.get_sprite(&format!("cc{}", i));
            l.content.resize(Rect::new(0, 0, CELLW as u16, CELLH as u16));
            l.content.reset();
            l.set_pos(0, 0);
        }

        // Draw board border walls
        {
            let back = self.scene.get_sprite("back");
            let wall_color = Color::Rgba(0xc0, 0xc0, 0xc0, 0xff);
            for i in 0..w * CELLW + 2 {
                back.set_color_str(i as u16, 0, "░", wall_color, Color::Reset);
                back.set_color_str(i as u16, (h * CELLH + 1) as u16, "░", wall_color, Color::Reset);
            }
            for i in 0..h * CELLH + 2 {
                back.set_color_str(0, i as u16, "░", wall_color, Color::Reset);
                back.set_color_str((w * CELLW + 1) as u16, i as u16, "░", wall_color, Color::Reset);
            }
        }

        // Find which cell of each block shows the arrow (first cell of block)
        let mut arrow_cells: std::collections::HashMap<usize, (usize, usize)> =
            std::collections::HashMap::new();
        for block in &d.board.blocks {
            if !d.board.removed[block.id] && !block.cells.is_empty() {
                arrow_cells.insert(block.id, block.cells[0]);
            }
        }

        // Flash state
        let flash_bid = if d.flash_timer > 0 && d.flash_timer % 3 != 0 {
            d.flash_block
        } else {
            None
        };

        // Draw board cells
        for i in 0..(w * h) {
            let (border_type, color) = d.render_state[i];
            if color >= 0 {
                let x = i % w;
                let y = i / w;
                let sx = (x * CELLW) as u16 + 1;
                let sy = (y * CELLH) as u16 + 1;

                let mut arrow_str = "";
                let mut is_flash = false;
                if let Some(block_id) = d.board.block_at(x, y) {
                    if let Some(&(ax, ay)) = arrow_cells.get(&block_id) {
                        if ax == x && ay == y {
                            arrow_str = d.board.blocks[block_id].arrow.arrow_char();
                        }
                    }
                    if flash_bid == Some(block_id) {
                        is_flash = true;
                    }
                }

                self.draw_cell(
                    ctx,
                    &format!("cc{}", i),
                    sx, sy,
                    border_type, color,
                    arrow_str, color,
                    is_flash,
                    255,
                );
            }
        }

        // Draw fly-away animation (use sprite slots after board area)
        if let Some(ref anim) = d.fly_anim {
            let base_slot = w * h; // start after board sprites
            let progress = anim.frame as f32 / 10.0; // 0.0 → 1.0
            // Offset: accelerate over time (ease-in)
            let offset = (progress * progress * 20.0) as i16;
            // Fade: 255 → 40
            let alpha = (255.0 * (1.0 - progress * 0.85)) as u8;

            let (dx, dy): (i16, i16) = match anim.direction {
                Direction::Up => (0, -1),
                Direction::Down => (0, 1),
                Direction::Left => (-1, 0),
                Direction::Right => (1, 0),
            };

            for (ci, cell) in anim.cells.iter().enumerate() {
                let slot = base_slot + ci;
                if slot >= MAX_CELLS {
                    break;
                }
                // Horizontal moves by CELLW-sized steps, vertical by CELLH-sized steps
                let nx = cell.sx as i16 + dx * offset * if dx != 0 { 3 } else { 1 };
                let ny = cell.sy as i16 + dy * offset;
                // Skip if any part of the sprite would be off-screen
                if nx < 0 || ny < 0
                    || nx + CELLW as i16 > BLOCK_ARROWW as i16
                    || ny + CELLH as i16 > BLOCK_ARROWH as i16
                {
                    continue;
                }
                self.draw_cell(
                    ctx,
                    &format!("cc{}", slot),
                    nx as u16, ny as u16,
                    cell.border_type, cell.color,
                    &cell.arrow_str, cell.color,
                    false,
                    alpha,
                );
            }
        }

        // Draw status bar
        {
            let status = self.scene.get_sprite("status");
            status.content.reset();
            let info = format!(
                " Lv{}  Blocks:{}  [Click]Fly [R]Restart [N]Next",
                d.level_index + 1,
                d.board.remaining_count()
            );
            status.set_color_str(0, 0, &info, Color::Cyan, Color::Reset);
        }

        // Draw message
        {
            let msg = self.scene.get_sprite("msg");
            msg.content.reset();
            if !d.message.is_empty() {
                let color = match d.game_state {
                    crate::model::GameState::Won => Color::Green,
                    _ => Color::Yellow,
                };
                msg.set_color_str(1, 0, &d.message, color, Color::Reset);
            }
        }
    }
}

impl Render for Block_arrowRender {
    type Model = Block_arrowModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(BLOCK_ARROWW, BLOCK_ARROWH, 0.5, 0.5, "block_arrow".to_string());
        self.scene.init(ctx);
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("BlockArrow.Redraw", "redraw") {
            self.draw_board(ctx, data);
        }
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        self.draw_board(ctx, data);
        self.scene.draw(ctx).unwrap();
    }
}
