use crate::model::{get_xy, CityModel, CityState, CELLH, CELLW, LEVELUP_TIME, NCOL, NROW};
use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_percent, timer_rstage},
    game::Render,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::{Color, Style},
    util::Rect,
    GAME_FRAME,
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

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn level_info(l: i16) -> String {
    let mut msg = format!("{}", l);
    if l == 30 {
        msg = "T".to_string();
    } else if l > 30 {
        msg = format!("W{:02}", l / 30);
    }
    msg
}

#[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
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
}

impl CityRender {
    pub fn new() -> Self {
        info!("create city render...");
        let mut t = Panel::new();

        let tsback = Sprite::new(0, 0, 70, 40);
        t.add_sprite(tsback, "back");
        for i in 0..NCOL * NROW {
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            t.add_pixel_sprite(
                Sprite::new(0, 0, CELLW as u16, CELLH as u16),
                &format!("cc{}", i),
            );

            #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
            t.add_sprite(
                Sprite::new(0, 0, CELLW as u16, CELLH as u16),
                &format!("cc{}", i),
            );
        }
        t.add_sprite(Sprite::new(0, (NROW + 3) as u16, NCOL as u16, 1u16), "msg");
        event_register("redraw_grid", "draw_grid");
        Self { panel: t }
    }

    pub fn draw_movie(&mut self, ctx: &mut Context, data: &mut CityModel) {
        let ss: CityState = ctx.state.into();
        match ss {
            CityState::MergeMovie => {
                self.draw_moving(ctx, data, ss, timer_percent("merge"));
            }
            CityState::LevelUpMovie => {
                self.draw_moving(ctx, data, ss, timer_percent("levelup"));
            }
            CityState::DropMovie => {
                self.draw_moving(ctx, data, ss, timer_percent("drop"));
            }
            _ => {
                self.draw_ready2t(ctx, data);
            }
        }
    }

    #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
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
        let l = self.panel.get_sprite(&format!("cc{}", id));
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
            Style::default()
                .fg(COLORS[border_color as usize % COLORS.len()])
                .bg(Color::Indexed(1)),
        );
        //设置内容
        l.set_color_str(
            3,
            2,
            msg,
            COLORS[msg_color as usize % COLORS.len()],
            Color::Reset,
        );
        //绘制是否删除标记
        if is_del {
            l.set_color_str(3, 0, "DEL?", COLORS[7], Color::Reset);
        }
    }

    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
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
        let l = self.panel.get_pixel_sprite(&format!("cc{}", id));
        let ss = ["cc", "dd", "ee", "ff", "gg"];
        if border_color <= 5 && border_color >= 1 {
            let cn = format!("pix/{}{}.pix", ss[border_color as usize - 1], border_type);
            asset2sprite!(l, ctx, &cn);
        }
        // wornder...
        if border_color > 5 {
            let cn = format!("pix/hh{}.pix", border_type);
            asset2sprite!(l, ctx, &cn);
            l.set_fg(Color::Red);
        }
        if border_type == 16 {
            asset2sprite!(l, ctx, "pix/cc16.pix");
        }
        l.set_pos(x, y);
        l.set_color_str(3, 3, msg, COLORS[msg_color as usize], Color::Reset);
        if is_del {
            l.set_color_str(3, 0, "DEL?", COLORS[7], Color::Reset);
        }
    }

    pub fn draw_moving(
        &mut self,
        ctx: &mut Context,
        d: &mut CityModel,
        state: CityState,
        per: f32,
    ) {
        for cid in &d.move_cells {
            let (x, y) = get_xy(*cid);
            let dc = &d.grid[y][x];
            let ctype;
            let mut msg;
            if dc.color >= 0 {
                ctype = 15;
                msg = level_info(dc.level);
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

                let mut ffy = fy as f32;
                if dc.from_id.unwrap() < 0 {
                    ffy = (dc.from_id.unwrap() as f32 / NCOL as f32).floor();
                }

                let (tx, ty) = get_xy(dc.to_id.unwrap());
                let nx = fx as f32 + (tx as f32 - fx as f32) * (1.0 - per);
                let ny = ffy + (ty as f32 - ffy) * (1.0 - per);
                let sx = nx * CELLW as f32 + 3.0;
                let sy = ny * CELLH as f32 + 8.0;
                if sx >= 0.0 && sy >= 0.0 {
                    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                    let usx = (sx * 8.0) as u16;
                    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                    let usy = (sy * 8.0) as u16;
                    #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
                    let usx = sx as u16;
                    #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
                    let usy = sy as u16;

                    self.draw_cell(ctx, *cid, usx, usy, ctype, dc.color, &msg, 3, false);
                }
            } else {
                let sx = x * CELLW + 3;
                let sy = y * CELLH + 8;
                #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                let usx = (sx * 8) as u16;
                #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                let usy = (sy * 8) as u16;
                #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
                let usx = sx as u16;
                #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
                let usy = sy as u16;
                self.draw_cell(ctx, *cid, usx, usy, ctype, dc.color, &msg, 3, false);
            }
        }
    }

    pub fn draw_ready2t(&mut self, ctx: &mut Context, d: &mut CityModel) {
        if !d.ready2t {
            return;
        }
        self.draw_grid(ctx, d);
    }

    pub fn draw_grid(&mut self, ctx: &mut Context, d: &mut CityModel) {
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
                msg = format!(".{}", msg);
            } else {
                msgcol = 3u32;
            }

            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            let usx = (sx * 8) as u16;
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            let usy = (sy * 8) as u16;
            #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
            let usx = sx as u16;
            #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
            let usy = sy as u16;

            self.draw_cell(
                ctx,
                i as i16,
                usx,
                usy,
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
    type Model = CityModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        ctx.adapter.init(60, 60, 1.0, 1.0, "city".to_string());
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        ctx.adapter.init(70, 40, 1.0, 1.0, "city".to_string());
        self.panel.init(ctx);
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        let l = self.panel.get_sprite("back");
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        asset2sprite!(l, ctx, &format!("back.txt"));
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("redraw_grid", "draw_grid") {
            self.draw_grid(ctx, data);
        }
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        self.draw_movie(ctx, data);
        self.panel.draw(ctx).unwrap();
    }
}
