#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{ColorblkModel, CELLH, CELLW, COLORBLKH, COLORBLKW};
use colorblk_lib::{Block, Direction, Gate, SHAPE, SHAPE_IDX};
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

// 颜色定义
const COLORS: [Color; 15] = [
    Color::Rgba(0xe9, 0x05, 0x05, 0xff), // Red
    Color::Rgba(0x19, 0xce, 0x27, 0xff), // Green
    Color::Rgba(0x10, 0x73, 0xdd, 0xff), // Blue
    Color::Rgba(0xff, 0xd3, 0x22, 0xff), // Yellow
    Color::Rgba(0xb3, 0x52, 0xff, 0xff), // Purple
    Color::Rgba(0xfc, 0x86, 0x2a, 0xff), // Orange
    Color::Rgba(0xe5, 0x5a, 0xf1, 0xff), // LightPurple
    Color::Rgba(0x97, 0x51, 0x3b, 0xff), // Brown
    Color::Rgba(0xf0, 0xf0, 0xf0, 0xff), // Gray
    Color::Rgba(0x33, 0x33, 0x33, 0xff), // Black
    Color::Rgba(0x21, 0x79, 0x37, 0xff), // DarkGreen
    Color::Rgba(0x73, 0xfb, 0xfd, 0xff), // LightBlue
    Color::Rgba(0xff, 0xff, 0xff, 0xff), // White
    Color::Rgba(0x28, 0xc8, 0xff, 0xff), // Skyblue
    Color::Rgba(0x66, 0x66, 0x66, 0xff), // DarkBlack
];

pub struct ColorblkRender {
    pub panel: Panel,
}

impl ColorblkRender {
    pub fn new() -> Self {
        info!("create colorblk render...");
        let mut t = Panel::new();

        // 创建背景精灵
        let tsback = Sprite::new(0, 0, COLORBLKW, COLORBLKH);
        t.add_sprite(tsback, "back");

        // 为每个格子创建精灵（这里使用足够大的数量以支持各种棋盘大小）
        for i in 0..300 {
            // 最大支持8x8的棋盘
            t.add_sprite(
                Sprite::new(0, 0, CELLW as u16, CELLH as u16),
                &format!("cc{}", i),
            );
        }

        // 创建消息精灵
        // t.add_sprite(Sprite::new(0, 35, 70, 1), "msg");

        // 注册重绘网格事件
        event_register("redraw_grid", "draw_grid");

        Self { panel: t }
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
        // if id >= 60 {
        //     return;
        // }
        let l = self.panel.get_sprite(&format!("cc{}", id));
        let area = Rect::new(0, 0, CELLW as u16, CELLH as u16);
        l.content.resize(area);
        l.content.reset();
        let cn = format!("cc{}.txt", border_type);
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

    pub fn draw_grid(&mut self, ctx: &mut Context, d: &mut ColorblkModel) {
        // for y in 0..d.stage.board_height {
        //     for x in 0..d.stage.board_width {
        //         let sx = (x + 1) * 10;
        //         let sy = (y + 1) * 5;
        //         let l = self
        //             .panel
        //             .get_sprite(&format!("cc{}", y * d.stage.board_width + x));
        //         let area = Rect::new(0, 0, CELLW as u16, CELLH as u16);
        //         l.content.resize(area);
        //         l.content.reset();
        //         l.set_pos(sx as u16, sy as u16);
        //         l.content
        //             .set_style(l.content.area, Style::default().fg(COLORS[7]));
        //         l.set_color_str(3, 2, GRID_SYMS, COLORS[7], Color::Reset);
        //     }
        // }
    }

    pub fn draw_ready(&mut self, ctx: &mut Context, d: &mut ColorblkModel) {
        // 清空所有内容
        {
            let back = self.panel.get_sprite("back");
            back.content.reset();

            // 清空所有格子
            for i in 0..d.stage.board_width * d.stage.board_height {
                let l = self.panel.get_sprite(&format!("cc{}", i));
                l.content.reset();
            }
        }

        // 绘制网格
        self.draw_grid(ctx, d);
        let wallcolor = 8;

        // 绘制门
        {
            let back = self.panel.get_sprite("back");
            for i in 0..d.stage.board_width * 10 + 2 {
                back.set_color_str(
                    i as u16, // 居中显示
                    0u16,
                    "░", // 使用10个字符的宽度
                    COLORS[wallcolor],
                    Color::Reset,
                );
                back.set_color_str(
                    i as u16, // 居中显示
                    d.stage.board_height as u16 * 5 + 1,
                    "░", // 使用10个字符的宽度
                    COLORS[wallcolor],
                    Color::Reset,
                );
            }
            for i in 0..d.stage.board_height * 5 + 2 {
                back.set_color_str(
                    0u16, 
                    i as u16, // 居中显示
                    "░", // 使用10个字符的宽度
                    COLORS[wallcolor],
                    Color::Reset,
                );
                back.set_color_str(
                    d.stage.board_width as u16 * 10 + 1,
                    i as u16, // 居中显示
                    "░", // 使用10个字符的宽度
                    COLORS[wallcolor],
                    Color::Reset,
                );
            }
            for gate in &d.stage.gates {
                if gate.height == 0 {
                    // 上下门：绘制一行彩色字符
                    for x in gate.x..gate.x + gate.width {
                        let screen_x = (x as usize * 10) as u16 + 1; // 每个单元格宽度为10
                        let screen_y = if gate.y == 0 {
                            0
                        } else {
                            (d.stage.board_height * 5) as u16 + 1
                        }; // 每个单元格高度为5
                        back.set_color_str(
                            screen_x as u16, // 居中显示
                            screen_y as u16,
                            "██████████", // 使用10个字符的宽度
                            COLORS[gate.color as usize % COLORS.len()],
                            Color::Reset,
                        );
                    }
                } else {
                    // 左右门：绘制一列彩色字符
                    for y in gate.y..gate.y + gate.height {
                        let screen_x = if gate.x == 0 {
                            0
                        } else {
                            (d.stage.board_width * 10) as u16 + 1
                        }; // 每个单元格宽度为10
                        let screen_y = (y as usize * 5) as u16 + 1; // 每个单元格高度为5

                        for r in 0..5 {
                            back.set_color_str(
                                screen_x as u16, // 居中显示
                                screen_y + r as u16,
                                "█", // 使用单个字符的宽度
                                COLORS[gate.color as usize % COLORS.len()],
                                Color::Reset,
                            );
                        }
                    }
                }
            }
        }

        // 使用model中的render_state绘制方块
        for i in 0..d.stage.board_width * d.stage.board_height {
            let (border_type, color) = d.render_state[i as usize];
            if color >= 0 {
                let x = i % d.stage.board_width;
                let y = i / d.stage.board_width;
                let sx = (x * 10) as u16 + 1;
                let sy = (y * 5) as u16 + 1;
                self.draw_cell(ctx, i as i16, sx, sy, border_type, color, "", 0);
            }
        }
    }
}

impl Render for ColorblkRender {
    type Model = ColorblkModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(COLORBLKW, COLORBLKH, 1.0, 1.0, "colorblk".to_string());
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
