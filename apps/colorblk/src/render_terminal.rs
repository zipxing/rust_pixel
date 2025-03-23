#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{ColorblkModel, CARDH, CARDW, CELLH, CELLW, COLORBLKH, COLORBLKW};
use colorblk_lib::{Block, Direction, Gate, BOARD_HEIGHT, BOARD_WIDTH, SHAPE, SHAPE_IDX};
use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_percent, timer_rstage},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::{Color, Style},
    util::Rect,
    GAME_FRAME,
};

fn calculate_border_type(grid: &[[u8; 5]; 5], x: usize, y: usize) -> u8 {
    // 检查四个方向的邻居
    let mut border_bits = 0u8;

    // 上邻居
    if y == 0 || grid[y - 1][x] == 0 {
        border_bits |= 0b1000;
    }

    // 下邻居
    if y == 4 || grid[y + 1][x] == 0 {
        border_bits |= 0b0100;
    }

    // 左邻居
    if x == 0 || grid[y][x - 1] == 0 {
        border_bits |= 0b0010;
    }

    // 右邻居
    if x == 4 || grid[y][x + 1] == 0 {
        border_bits |= 0b0001;
    }
    border_bits
}

// 颜色定义
const COLORS: [Color; 8] = [
    Color::LightRed,     // 红色方块
    Color::LightBlue,    // 蓝色方块
    Color::LightGreen,   // 绿色方块
    Color::LightYellow,  // 黄色方块
    Color::LightMagenta, // 紫色方块
    Color::LightCyan,    // 青色方块
    Color::Indexed(38),  // 门颜色
    Color::Indexed(202), // 边框颜色
];

// 方块符号
const BLOCK_SYMS: [&str; 6] = ["█", "█", "█", "█", "█", "█"];
const GATE_SYMS: &str = "═";
const GRID_SYMS: &str = "·";

const RESET: &str = "\x1b[0m";

// 获取颜色ANSI代码的辅助函数
fn get_color_code(color: u8) -> &'static str {
    match color {
        1 => "\x1b[31m", // 红色
        2 => "\x1b[34m", // 蓝色
        3 => "\x1b[32m", // 绿色
        4 => "\x1b[33m", // 黄色
        5 => "\x1b[33m", // 为颜色5使用黄色
        _ => "\x1b[0m",  // 默认
    }
}

pub struct ColorblkRender {
    pub panel: Panel,
}

impl ColorblkRender {
    pub fn new() -> Self {
        info!("create colorblk render...");
        let mut t = Panel::new();

        // 创建背景精灵
        let tsback = Sprite::new(0, 0, 70, 40);
        t.add_sprite(tsback, "back");

        // 为每个格子创建精灵
        for i in 0..BOARD_WIDTH * BOARD_HEIGHT {
            t.add_sprite(
                Sprite::new(0, 0, CELLW as u16, CELLH as u16),
                &format!("cc{}", i),
            );
        }

        // 创建消息精灵
        t.add_sprite(Sprite::new(0, 35, 70, 1), "msg");

        // 注册重绘网格事件
        event_register("redraw_grid", "draw_grid");

        Self { panel: t }
    }

    pub fn draw_solution(&mut self, ctx: &mut Context, data: &mut ColorblkModel) {
        if data.solution.is_some() {
            self.draw_moving(ctx, data, timer_percent("solution"));
        } else {
            self.draw_ready(ctx, data);
        }
        self.draw_status(ctx, data);
    }

    pub fn draw_cell(
        &mut self,
        ctx: &mut Context,
        id: i16,
        x: u16,
        y: u16,
        border_type: u8,
        border_color: i8,
        msg: &str,
        msg_color: i8,
    ) {
        let l = self.panel.get_sprite(&format!("cc{}", id));
        let area = Rect::new(0, 0, CELLW as u16, CELLH as u16);
        l.content.resize(area);
        l.content.reset();
        let cn = format!("cc{}.txt", border_type);
        info!("cn....{}", cn);
        asset2sprite!(l, ctx, &cn);
        l.set_pos(x, y);
        //设置颜色
        l.content.set_style(
            l.content.area,
            Style::default().fg(COLORS[border_color as usize % COLORS.len()]),
        );
        //设置内容
        l.set_color_str(
            3,
            2,
            msg,
            COLORS[msg_color as usize % COLORS.len()],
            Color::Reset,
        );
    }

    pub fn draw_grid(&mut self, ctx: &mut Context) {
        // for y in 0..BOARD_HEIGHT {
        //     for x in 0..BOARD_WIDTH {
        //         let sx = x * 8;
        //         let sy = y * 4;
        //         let l = self.panel.get_sprite(&format!("cc{}", y * 8 + x));
        //         let area = Rect::new(0, 0, 8, 4);
        //         l.content.resize(area);
        //         l.content.reset();
        //         l.set_pos(sx as u16, sy as u16);
        //         l.content
        //             .set_style(l.content.area, Style::default().fg(COLORS[7]));
        //         l.set_color_str(3, 2, GRID_SYMS, COLORS[7], Color::Reset);
        //     }
        // }
    }

    pub fn draw_status(&mut self, ctx: &mut Context, data: &mut ColorblkModel) {
        let msg = if let Some(solution) = &data.solution {
            format!("Step: {}/{}", data.current_step, solution.len())
        } else {
            "No solution found".to_string()
        };

        let l = self.panel.get_sprite("msg");
        l.set_color_str(0, 0, &msg, Color::White, Color::Reset);
    }

    pub fn draw_moving(&mut self, ctx: &mut Context, d: &mut ColorblkModel, per: f32) {
        // 绘制网格
        self.draw_grid(ctx);

        // // 绘制门
        // for gate in &d.gates {
        //     let sx = gate.x * 8;
        //     let sy = gate.y * 4;
        //     self.draw_cell(
        //         ctx,
        //         (gate.y * 8 + gate.x) as i16,
        //         sx as u16,
        //         sy as u16,
        //         0,
        //         gate.color as i8,
        //         "",
        //         0,
        //         true,
        //     );
        // }

        // 绘制移动中的方块
        if let Some(solution) = &d.solution {
            if d.current_step < solution.len() {
                let (block_id, direction, steps) = solution[d.current_step];
                if let Some(block) = d.initial_blocks.iter().find(|b| b.id == block_id) {
                    if let Some(dir) = direction {
                        // 计算移动后的位置
                        let (dx, dy) = match dir {
                            Direction::Up => (0, -1),
                            Direction::Down => (0, 1),
                            Direction::Left => (-1, 0),
                            Direction::Right => (1, 0),
                        };
                        let new_x = block.x as i16 + dx;
                        let new_y = block.y as i16 + dy;

                        let sx = new_x * 8;
                        let sy = new_y * 4;
                        self.draw_cell(
                            ctx,
                            (new_y * 8 + new_x) as i16,
                            sx as u16,
                            sy as u16,
                            0,
                            block.color as i8,
                            BLOCK_SYMS[block.color as usize % BLOCK_SYMS.len()],
                            0,
                        );
                    }
                }
            }
        }
    }

    pub fn draw_ready(&mut self, ctx: &mut Context, d: &mut ColorblkModel) {
        // 绘制网格
        self.draw_grid(ctx);

        // 绘制门
        let back = self.panel.get_sprite("back");
        for gate in &d.gates {
            let color_code = get_color_code(gate.color);
            if gate.height == 0 {
                // 上下门：绘制一行彩色字符
                for x in gate.x..gate.x + gate.width {
                    let screen_x = (x as usize + 1) * 2; // 向右偏移一个字符
                    let screen_y = if gate.y == 0 { 0 } else { BOARD_HEIGHT };
                    back.set_color_str(
                        screen_x as u16,
                        screen_y as u16,
                        "▔▔",
                        COLORS[gate.color as usize % COLORS.len()],
                        Color::Reset,
                    );
                }
            } else {
                // 左右门：绘制一列彩色字符
                for y in gate.y..gate.y + gate.height {
                    let screen_x = if gate.x == 0 { 0 } else { BOARD_WIDTH * 2 };
                    let screen_y = y as usize + 1; // 向下偏移一个字符
                    back.set_color_str(
                        screen_x as u16,
                        screen_y as u16,
                        "▏",
                        COLORS[gate.color as usize % COLORS.len()],
                        Color::Reset,
                    );
                }
            }
        }

        // 绘制方块
        for block in &d.initial_blocks {
            let shape_data = &SHAPE[block.shape as usize];

            // 遍历形状的每个格子
            for grid_y in 0..5 {
                for grid_x in 0..5 {
                    if shape_data.grid[grid_y][grid_x] == 1 {
                        // 计算棋盘上的实际坐标
                        let board_x = block.x as usize + (grid_x - shape_data.rect.x);
                        let board_y = block.y as usize + (grid_y - shape_data.rect.y);

                        // 计算屏幕上的坐标（向右下偏移一个字符）
                        let sx = (board_x + 1) * CELLW;
                        let sy = (board_y + 1) * CELLH;

                        // 计算边框类型
                        let border_type = calculate_border_type(&shape_data.grid, grid_x, grid_y);

                        // 绘制格子
                        self.draw_cell(
                            ctx,
                            (board_y * BOARD_WIDTH + board_x) as i16,
                            sx as u16,
                            sy as u16,
                            border_type,
                            block.color as i8,
                            BLOCK_SYMS[block.color as usize % BLOCK_SYMS.len()],
                            0,
                        );
                    }
                }
            }
        }
    }
}

impl Render for ColorblkRender {
    type Model = ColorblkModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter.init(70, 40, 1.0, 1.0, "colorblk".to_string());
        self.panel.init(ctx);
        // let l = self.panel.get_sprite("back");
        // asset2sprite!(l, ctx, &format!("back.txt"));
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("redraw_grid", "draw_grid") {
            self.draw_ready(ctx, data);
        }
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        // self.draw_solution(ctx, data);
        self.draw_ready(ctx, data);
        self.panel.draw(ctx).unwrap();
    }
}
