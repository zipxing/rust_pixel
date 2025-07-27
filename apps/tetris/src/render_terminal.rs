use crate::model::TetrisModel;
use tetris_lib::constant::*;
//use std::fs::File;
//use std::io::Write;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_exdata, timer_stage},
    game::Render,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};

pub struct TetrisRender {
    pub panel: Panel,
}

impl TetrisRender {
    pub fn new() -> Self {
        let mut t = Panel::new();

        let tsback = Sprite::new(0, 0, 80, 30);
        t.add_sprite(tsback, "back");

        let l0 = Sprite::new(2, 7, HENG * 2, ZONG);
        t.add_sprite(l0, "grid0");

        let l1 = Sprite::new(55, 7, HENG * 2, ZONG);
        t.add_sprite(l1, "grid1");

        let l2 = Sprite::new(27, 8, 8, 4);
        t.add_sprite(l2, "next");

        let l3 = Sprite::new(42, 8, 8, 4);
        t.add_sprite(l3, "hold");

        event_register("Tetris.RedrawNext", "redraw_next");
        event_register("Tetris.RedrawHold", "redraw_hold");
        event_register("Tetris.RedrawMsg", "redraw_msg");

        Self { panel: t }
    }

    fn set_block(&mut self, sname: &str, x: u16, y: u16, c: u8) {
        let cv = vec![
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

        let l = self.panel.get_sprite(sname);

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
                c1 = "[";
                c2 = "]";
                fg = cv[(c % 100) as usize % cv.len()];
                bg = Color::Reset;
            }
        }

        if x < HENG * 2 && y < ZONG {
            l.set_color_str(x, y, c1, fg, bg);
        }
        if x + 1 < HENG * 2 && y < ZONG {
            l.set_color_str(x + 1, y, c2, fg, bg);
        }
    }

    pub fn redraw_hold(&mut self, d: &mut TetrisModel) {
        for i in 0..4 {
            for j in 0..4 {
                let rx = j * 2;
                if d.sides[0].get_md(d.sides[0].core.save_block, 0, i * 4 + j) != 0 {
                    self.set_block(
                        "hold",
                        rx as u16,
                        i as u16,
                        d.sides[0].core.save_block as u8 + 1,
                    );
                } else {
                    self.set_block("hold", rx as u16, i as u16, 0);
                }
            }
        }
    }

    pub fn redraw_next(&mut self, d: &mut TetrisModel) {
        for i in 0..4 {
            for j in 0..4 {
                let rx = j * 2;
                if d.sides[0].get_md(d.sides[0].core.next_block, 0, i * 4 + j) != 0 {
                    self.set_block(
                        "next",
                        rx as u16,
                        i as u16,
                        d.sides[0].core.next_block as u8 + 1,
                    );
                } else {
                    self.set_block("next", rx as u16, i as u16, 0);
                }
            }
        }
    }

    pub fn draw_grid(&mut self, _context: &mut Context, d: &mut TetrisModel) {
        for n in 0..2 {
            let frs = timer_stage(&format!("clear-row{}", n));
            let mut fri: Vec<i8> = vec![];
            if frs != 0 {
                let fr = timer_exdata(&format!("clear-row{}", n)).unwrap();
                // TODO: Fix for new bincode API
                // fri = bincode::deserialize(&fr).unwrap();
                fri = vec![];
                //info!("frs..{} fri..{:?}", frs, fri);
            }
            for i in 0..ZONG {
                for j in 0..HENG {
                    let rx = j * 2;
                    let gv = d.sides[n].get_gd(i as i8, (j + 2) as i8);
                    match gv {
                        0 => {
                            self.set_block(&format!("grid{}", n), rx, i, 0);
                        }
                        _ => {
                            let mut hidden_fullrow = false;
                            if frs != 0 {
                                if fri.contains(&(i as i8)) && frs / 3 % 2 == 0 {
                                    hidden_fullrow = true;
                                }
                            }
                            if hidden_fullrow {
                                self.set_block(&format!("grid{}", n), rx, i, 30);
                            } else {
                                self.set_block(&format!("grid{}", n), rx, i, gv % 100);
                            }
                        }
                    }
                }
            }
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
                            //Ensure that when the shadow and the normal block overlap, the shadow
                            //does not cover the normal block...
                            if d.sides[n].get_gd(tty, ttx) == 0 {
                                self.set_block(&format!("grid{}", n), rx as u16, tty as u16, 20);
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
        context.adapter.init(80, 30, 1.0, 1.0, "tetris".to_string());
        self.panel.init(context);
        let l = self.panel.get_sprite("back");
        let bp = "back.txt";
        asset2sprite!(l, context, &bp);
    }

    fn draw(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        self.draw_grid(context, data);
        self.panel.draw(context).unwrap();
    }

    fn handle_event(&mut self, _context: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("Tetris.RedrawNext", "redraw_next") {
            self.redraw_next(data);
        }
        if event_check("Tetris.RedrawHold", "redraw_hold") {
            self.redraw_hold(data);
        }
    }

    fn handle_timer(&mut self, _context: &mut Context, _data: &mut Self::Model, _dt: f32) {}
}
