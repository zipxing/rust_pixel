#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(non_camel_case_types)]
use crate::model::{Block_arrowModel, CELLH, CELLW, BLOCK_ARROWH, BLOCK_ARROWW};
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
        id: usize,
        x: u16,
        y: u16,
        border_type: u8,
        border_color: i8,
        msg: &str,
        msg_color: i8,
    ) {
        let l = self.scene.get_sprite(&format!("cc{}", id));
        let area = Rect::new(0, 0, CELLW as u16, CELLH as u16);
        l.content.resize(area);
        l.content.reset();
        let cn = format!("cc{}.txt", border_type);
        asset2sprite!(l, ctx, &cn);
        l.set_pos(x, y);
        let fg = COLORS[border_color as usize % COLORS.len()];
        let bg = Self::dim_color(fg, 140); // ~55% brightness fill
        l.content.set_style(
            l.content.area,
            Style::default().fg(fg).bg(bg),
        );
        if !msg.is_empty() {
            l.set_color_str(
                3,
                2,
                msg,
                COLORS[msg_color as usize % COLORS.len()],
                bg,
            );
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

        // Clear all cell sprites
        for i in 0..(w * h) {
            let l = self.scene.get_sprite(&format!("cc{}", i));
            l.content.reset();
        }

        // Draw board border walls
        {
            let back = self.scene.get_sprite("back");
            let wall_color = Color::Rgba(0xc0, 0xc0, 0xc0, 0xff);
            // Top and bottom walls
            for i in 0..w * CELLW + 2 {
                back.set_color_str(i as u16, 0, "░", wall_color, Color::Reset);
                back.set_color_str(i as u16, (h * CELLH + 1) as u16, "░", wall_color, Color::Reset);
            }
            // Left and right walls
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
                // Use first cell as arrow position
                arrow_cells.insert(block.id, block.cells[0]);
            }
        }

        // Draw cells
        for i in 0..(w * h) {
            let (border_type, color) = d.render_state[i];
            if color >= 0 {
                let x = i % w;
                let y = i / w;
                let sx = (x * CELLW) as u16 + 1;
                let sy = (y * CELLH) as u16 + 1;

                // Check if this cell should show the arrow
                let mut arrow_str = "";
                if let Some(block_id) = d.board.block_at(x, y) {
                    if let Some(&(ax, ay)) = arrow_cells.get(&block_id) {
                        if ax == x && ay == y {
                            arrow_str = d.board.blocks[block_id].arrow.arrow_char();
                        }
                    }
                }

                self.draw_cell(ctx, i, sx, sy, border_type, color, arrow_str, color);
            }
        }

        // Draw cursor highlight
        if d.cursor_x < w && d.cursor_y < h {
            let cx = (d.cursor_x * CELLW) as u16 + 1;
            let cy = (d.cursor_y * CELLH) as u16 + 1;
            let cursor_id = d.cursor_y * w + d.cursor_x;
            if cursor_id < w * h {
                let l = self.scene.get_sprite(&format!("cc{}", cursor_id));
                // Highlight selected block with white background
                if d.selected_block.is_some() {
                    l.content.set_style(
                        l.content.area,
                        Style::default().bg(Color::Rgba(0x40, 0x40, 0x40, 0xff)),
                    );
                }
            }
        }

        // Draw status bar
        {
            let status = self.scene.get_sprite("status");
            status.content.reset();
            let info = format!(
                " Lv{}  Blocks:{}  [Arrows]Move [Space]Fly [R]Restart [N]Next",
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
