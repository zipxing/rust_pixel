use crate::model::{get_xy, CityModel, CityState, CELLH, CELLW, LEVELUP_TIME, NCOL, NROW};
use log::info;
use rust_pixel::{
    GAME_FRAME,
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_percent, timer_rstage},
    game::{Model, Render},
    render::sprite::{Sprites, Sprite},
    render::style::{Color, Style},
    render::panel::Panel,
    util::Rect,
};

//用到的颜色
const COLORS: [Color; 8] = [
    Color::LightRed,
    Color::LightGreen,
    Color::LightBlue,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightCyan,
    Color::Indexed(38),
    Color::Indexed(202),
];

//COLORS中的索引
const TOWER_COLOR: i8 = 6;
const WONDER_COLOR: i8 = 7;

const SYMS: [&str; 8] = ["🏠", "🏡", "🏘", "🏢", "🏰", "🏯", "🎡", "🗽"];
const TOWER_SYMS: &str = "🏛";
const WONDER_SYMS: &str = "W";

pub fn level_info(l: i16) -> String {
    let dcl = (l as f32 / 4.0).ceil() as usize;
    let mut msg = format!("{}", SYMS[dcl % SYMS.len()]);
    if l == 30 {
        msg = TOWER_SYMS.to_string();
    } else if l > 30 {
        msg = format!("{}{:02}", WONDER_SYMS, l / 30);
    }
    msg
}

pub struct CityRender {
    pub panel: Panel,
    pub main_scene: Sprites,
    //存储17种不同边框的方块，用于绘制
    // pub boxes: Sprites,
}

impl CityRender {
    pub fn new() -> Self {
        info!("create city render...");
        let t = Panel::new();

        //背景
        let mut s = Sprites::new("main");
        let tsback = Sprite::new(0, 0, 70, 40);
        s.add_by_tag(tsback, "back");

        //25个单元块
        for i in 0..NCOL * NROW {
            s.add_by_tag(
                Sprite::new(0, 0, CELLW as u16, CELLH as u16),
                &format!("cc{}", i),
            );
        }

        //msg块
        s.add_by_tag(
            Sprite::new(0, (NROW + 3) as u16, NCOL as u16, 1u16),
            "msg",
        );

        //注册重绘事件
        event_register("redraw_grid", "draw_grid");

        Self {
            panel: t,
            main_scene: s,
            // boxes: bs,
        }
    }

    pub fn draw_movie<G: Model>(&mut self, ctx: &mut Context, data: &mut G) {
        let ss: CityState = ctx.state.into();
        match ss {
            //飞行合并
            CityState::MergeMovie => {
                self.draw_moving(ctx, data, ss, timer_percent("merge"));
            }
            //数字升级
            CityState::LevelUpMovie => {
                self.draw_moving(ctx, data, ss, timer_percent("levelup"));
            }
            //掉落补齐
            CityState::DropMovie => {
                self.draw_moving(ctx, data, ss, timer_percent("drop"));
            }
            _ => {
                //当model.ready2t时工作
                //对于将要合并成T的unit，闪烁边框
                self.draw_ready2t(ctx, data);
            }
        }
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
        is_del: bool,
    ) {
        let l = self.main_scene.get_by_tag(&format!("cc{}", id));
        let area = Rect::new(0, 0, 10, 5);
        l.content.resize(area);
        l.content.reset();
        let cn = format!("cc{}.txt", border_type);
        asset2sprite!(l, ctx, &cn);
        l.set_pos(x, y);
        //设置颜色
        #[cfg(not(feature = "sdl"))]
        l.content.set_style(
            l.content.area,
            Style::default().fg(COLORS[border_color as usize % COLORS.len()]),
        );
        #[cfg(feature = "sdl")]
        l.content.set_style(
            l.content.area,
            Style::default().fg(COLORS[border_color as usize % COLORS.len()]).bg(Color::Indexed(1)),
        );
        //设置内容
        l.content.set_str(
            3,
            2,
            msg,
            Style::default().fg(COLORS[msg_color as usize % COLORS.len()]),
        );
        //绘制是否删除标记
        if is_del {
            l.content
                .set_str(3, 0, "DEL?", Style::default().fg(COLORS[7]));
        }
    }

    pub fn draw_moving<G: Model>(&mut self, ctx: &mut Context, data: &mut G, state: CityState, per: f32) {
        let d = data.as_any().downcast_mut::<CityModel>().unwrap();

        for cid in &d.move_cells {
            let (x, y) = get_xy(*cid);
            let dc = &d.grid[y][x];
            let ctype;
            let mut msg;
            //飞行的块
            if dc.color >= 0 {
                ctype = 15;
                msg = level_info(dc.level);
                //升级变换数字
                if state == CityState::LevelUpMovie {
                    let l;
                    let s = timer_rstage("levelup") as f32;
                    let step = GAME_FRAME as f32 * LEVELUP_TIME;
                    if d.levelup.from == 30 {
                        l = d.levelup.from + ((s / step).floor() * 30.0) as i16;
                    } else {
                        l = d.levelup.from + ((s / step).floor()) as i16;
                    }
                    msg = level_info(l);
                }
            } else {
                ctype = 16;
                msg = format!("");
            }
            if dc.from_id != None && dc.to_id != None {
                let (fx, fy) = get_xy(dc.from_id.unwrap());

                //from_id可能为负，调整fy
                let mut ffy = fy as f32;
                if dc.from_id.unwrap() < 0 {
                    ffy = (dc.from_id.unwrap() as f32 / NCOL as f32).floor();
                }

                //根据per绘制移动中的块
                let (tx, ty) = get_xy(dc.to_id.unwrap());
                let nx = fx as f32 + (tx as f32 - fx as f32) * (1.0 - per);
                let ny = ffy + (ty as f32 - ffy) * (1.0 - per);
                let sx = nx * CELLW as f32 + 3.0;
                let sy = ny * CELLH as f32 + 8.0;
                if sx >= 0.0 && sy >= 0.0 {
                    self.draw_cell(
                        ctx, *cid, sx as u16, sy as u16, ctype, dc.color, &msg, 3, false,
                    );
                }
            } else {
                let sx = x * CELLW + 3;
                let sy = y * CELLH + 8;
                self.draw_cell(
                    ctx, *cid, sx as u16, sy as u16, ctype, dc.color, &msg, 3, false,
                );
            }
        }
    }

    pub fn draw_ready2t<G: Model>(&mut self, ctx: &mut Context, data: &mut G) {
        let d = data.as_any().downcast_mut::<CityModel>().unwrap();
        if !d.ready2t {
            return;
        }
        self.draw_grid(ctx, data);
    }

    //如果有ready2t，则每帧调用
    //其他情况，只在接收到RedrawGrid事件时调用
    pub fn draw_grid<G: Model>(&mut self, ctx: &mut Context, data: &mut G) {
        let d = data.as_any().downcast_mut::<CityModel>().unwrap();
        for i in 0..NCOL * NROW {
            let (x, y) = get_xy(i as i16);
            let dc = &d.grid[y][x];
            let sx = x * CELLW + 3;
            let sy = y * CELLH + 8;

            let mut msg = level_info(dc.level);
            let msgcol;
            let mut bcol = dc.color % 100;
            if dc.level == 30 {
                bcol = TOWER_COLOR;
            } else if dc.level > 30 {
                bcol = WONDER_COLOR;
            }
            if dc.ready2t {
                msgcol = (ctx.stage / 8) % COLORS.len() as u32;
                msg = format!("∙{}", msg);
            } else {
                msgcol = 3u32;
            }

            self.draw_cell(
                ctx,
                i as i16,
                sx as u16,
                sy as u16,
                dc.border,
                bcol,
                &msg,
                msgcol as i8,
                dc.color >= 100,
            );
        }
    }
}

impl Render for CityRender {
    fn init<G: Model>(&mut self, ctx: &mut Context, _data: &mut G) {
        ctx.adapter.init(70, 40, 2.0, 1.0, "city".to_string());
        self.panel.init(ctx);
        let l = self.main_scene.get_by_tag("back");
        asset2sprite!(l, ctx, &format!("back.txt"));
    }

    fn handle_event<G: Model>(&mut self, ctx: &mut Context, data: &mut G, _dt: f32) {
        if event_check("redraw_grid", "draw_grid") {
            self.draw_grid(ctx, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _ctx: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, _dt: f32) {
        self.draw_movie(ctx, data);
        self.panel
            .draw(ctx, |a, f| {
                self.main_scene.render_all(a, f);
            })
            .unwrap();
    }
}
