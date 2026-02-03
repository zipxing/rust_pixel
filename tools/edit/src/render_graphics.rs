//! Graphics mode renderer for the edit tool
//!
//! Uses Scene architecture for graphics-based sprite editing with PETSCII symbols.

use crate::model::{TeditModel, TeditPen, COLORH, COLORW, EDITH, EDITW, SYMH, SYMW};
use log::info;
use rust_pixel::{
    asset::{AssetState, AssetType},
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::Render,
    render::{
        cell::cellsym,
        scene::Scene,
        sprite::{BorderType, Borders, Sprite},
        style::{Color, Style},
    },
};
use std::fs;

pub const SYMBOL_SDL: [u8; 18 * 16] = [
    32,
    032,
    001,
    002,
    003,
    004,
    005,
    006,
    007,
    008,
    009,
    010,
    011,
    012,
    013,
    014,
    015,
    32,
    32,
    016,
    017,
    018,
    019,
    020,
    021,
    022,
    023,
    024,
    025,
    026,
    046,
    044,
    059,
    033,
    063,
    32,
    32,
    048,
    049,
    050,
    051,
    052,
    053,
    054,
    055,
    056,
    057,
    034,
    035,
    036,
    037,
    038,
    039,
    32,
    32,
    112,
    110,
    108,
    123,
    085,
    073,
    079,
    080,
    113,
    114,
    040,
    041,
    060,
    062,
    078,
    077,
    32,
    32,
    109,
    125,
    124,
    126,
    074,
    075,
    076,
    122,
    107,
    115,
    027,
    029,
    031,
    030,
    095,
    105,
    32,
    32,
    100,
    111,
    121,
    098,
    120,
    119,
    099,
    116,
    101,
    117,
    097,
    118,
    103,
    106,
    091,
    043,
    32,
    32,
    082,
    070,
    064,
    045,
    067,
    068,
    069,
    084,
    071,
    066,
    093,
    072,
    089,
    047,
    086,
    042,
    32,
    32,
    061,
    058,
    028,
    000,
    127,
    104,
    092,
    102,
    081,
    087,
    065,
    083,
    088,
    090,
    094,
    096,
    32,
    32,
    032 + 128,
    001 + 128,
    002 + 128,
    003 + 128,
    004 + 128,
    005 + 128,
    006 + 128,
    007 + 128,
    008 + 128,
    009 + 128,
    010 + 128,
    011 + 128,
    012 + 128,
    013 + 128,
    014 + 128,
    015 + 128,
    32,
    32,
    016 + 128,
    017 + 128,
    018 + 128,
    019 + 128,
    020 + 128,
    021 + 128,
    022 + 128,
    023 + 128,
    024 + 128,
    025 + 128,
    026 + 128,
    046 + 128,
    044 + 128,
    059 + 128,
    033 + 128,
    063 + 128,
    32,
    32,
    048 + 128,
    049 + 128,
    050 + 128,
    051 + 128,
    052 + 128,
    053 + 128,
    054 + 128,
    055 + 128,
    056 + 128,
    057 + 128,
    034 + 128,
    035 + 128,
    036 + 128,
    037 + 128,
    038 + 128,
    039 + 128,
    32,
    32,
    112 + 128,
    110 + 128,
    108 + 128,
    123 + 128,
    085 + 128,
    073 + 128,
    079 + 128,
    080 + 128,
    113 + 128,
    114 + 128,
    040 + 128,
    041 + 128,
    060 + 128,
    062 + 128,
    078 + 128,
    077 + 128,
    32,
    32,
    109 + 128,
    125 + 128,
    124 + 128,
    126 + 128,
    074 + 128,
    075 + 128,
    076 + 128,
    122 + 128,
    107 + 128,
    115 + 128,
    027 + 128,
    029 + 128,
    031 + 128,
    030 + 128,
    095 + 128,
    105 + 128,
    32,
    32,
    100 + 128,
    111 + 128,
    121 + 128,
    098 + 128,
    120 + 128,
    119 + 128,
    099 + 128,
    116 + 128,
    101 + 128,
    117 + 128,
    097 + 128,
    118 + 128,
    103 + 128,
    106 + 128,
    091 + 128,
    043 + 128,
    32,
    32,
    082 + 128,
    070 + 128,
    064 + 128,
    045 + 128,
    067 + 128,
    068 + 128,
    069 + 128,
    084 + 128,
    071 + 128,
    066 + 128,
    093 + 128,
    072 + 128,
    089 + 128,
    047 + 128,
    086 + 128,
    042 + 128,
    32,
    32,
    061 + 128,
    058 + 128,
    028 + 128,
    000 + 128,
    127 + 128,
    104 + 128,
    092 + 128,
    102 + 128,
    081 + 128,
    087 + 128,
    065 + 128,
    083 + 128,
    088 + 128,
    090 + 128,
    094 + 128,
    096 + 128,
    32,
];

pub const SYMBOL_SDL_LOW: [u8; 18 * 16] = [
    32,
    032,
    001,
    002,
    003,
    004,
    005,
    006,
    007,
    008,
    009,
    010,
    011,
    012,
    013,
    014,
    015,
    32,
    32,
    016,
    017,
    018,
    019,
    020,
    021,
    022,
    023,
    024,
    025,
    026,
    046,
    044,
    059,
    033,
    063,
    32,
    32,
    096,
    065,
    066,
    067,
    068,
    069,
    070,
    071,
    072,
    073,
    074,
    075,
    076,
    077,
    078,
    079,
    32,
    32,
    080,
    081,
    082,
    083,
    084,
    085,
    086,
    087,
    088,
    089,
    090,
    034,
    035,
    036,
    037,
    038,
    32,
    32,
    048,
    049,
    050,
    051,
    052,
    053,
    054,
    055,
    056,
    057,
    043,
    045,
    042,
    061,
    039,
    000,
    32,
    32,
    112,
    110,
    108,
    123,
    113,
    114,
    040,
    041,
    095,
    105,
    092,
    127,
    060,
    062,
    028,
    047,
    32,
    32,
    109,
    125,
    124,
    126,
    107,
    115,
    027,
    029,
    094,
    102,
    104,
    058,
    030,
    031,
    091,
    122,
    32,
    32,
    100,
    111,
    121,
    098,
    099,
    119,
    120,
    101,
    116,
    117,
    097,
    103,
    106,
    118,
    064,
    093,
    32,
    32,
    032 + 128,
    001 + 128,
    002 + 128,
    003 + 128,
    004 + 128,
    005 + 128,
    006 + 128,
    007 + 128,
    008 + 128,
    009 + 128,
    010 + 128,
    011 + 128,
    012 + 128,
    013 + 128,
    014 + 128,
    015 + 128,
    32,
    32,
    016 + 128,
    017 + 128,
    018 + 128,
    019 + 128,
    020 + 128,
    021 + 128,
    022 + 128,
    023 + 128,
    024 + 128,
    025 + 128,
    026 + 128,
    046 + 128,
    044 + 128,
    059 + 128,
    033 + 128,
    063 + 128,
    32,
    32,
    096 + 128,
    065 + 128,
    066 + 128,
    067 + 128,
    068 + 128,
    069 + 128,
    070 + 128,
    071 + 128,
    072 + 128,
    073 + 128,
    074 + 128,
    075 + 128,
    076 + 128,
    077 + 128,
    078 + 128,
    079 + 128,
    32,
    32,
    080 + 128,
    081 + 128,
    082 + 128,
    083 + 128,
    084 + 128,
    085 + 128,
    086 + 128,
    087 + 128,
    088 + 128,
    089 + 128,
    090 + 128,
    034 + 128,
    035 + 128,
    036 + 128,
    037 + 128,
    038 + 128,
    32,
    32,
    048 + 128,
    049 + 128,
    050 + 128,
    051 + 128,
    052 + 128,
    053 + 128,
    054 + 128,
    055 + 128,
    056 + 128,
    057 + 128,
    043 + 128,
    045 + 128,
    042 + 128,
    061 + 128,
    039 + 128,
    000 + 128,
    32,
    32,
    112 + 128,
    110 + 128,
    108 + 128,
    123 + 128,
    113 + 128,
    114 + 128,
    040 + 128,
    041 + 128,
    095 + 128,
    105 + 128,
    092 + 128,
    127 + 128,
    060 + 128,
    062 + 128,
    028 + 128,
    047 + 128,
    32,
    32,
    109 + 128,
    125 + 128,
    124 + 128,
    126 + 128,
    107 + 128,
    115 + 128,
    027 + 128,
    029 + 128,
    094 + 128,
    102 + 128,
    104 + 128,
    058 + 128,
    030 + 128,
    031 + 128,
    091 + 128,
    122 + 128,
    32,
    32,
    100 + 128,
    111 + 128,
    121 + 128,
    098 + 128,
    099 + 128,
    119 + 128,
    120 + 128,
    101 + 128,
    116 + 128,
    117 + 128,
    097 + 128,
    103 + 128,
    106 + 128,
    118 + 128,
    064 + 128,
    093 + 128,
    32,
];

// 用256表示Color::Reset
pub const COLOR_PATTERN: [u16; 270] = [
    256, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 52, 53,
    54, 55, 56, 57, 88, 89, 90, 91, 92, 93, 22, 23, 24, 25, 26, 27, 58, 59, 60, 61, 62, 63, 94, 95,
    96, 97, 98, 99, 28, 29, 30, 31, 32, 33, 64, 65, 66, 67, 68, 69, 100, 101, 102, 103, 104, 105,
    34, 35, 36, 37, 38, 39, 70, 71, 72, 73, 74, 75, 106, 107, 108, 109, 110, 111, 40, 41, 42, 43,
    44, 45, 76, 77, 78, 79, 80, 81, 112, 113, 114, 115, 116, 117, 46, 47, 48, 49, 50, 51, 82, 83,
    84, 85, 86, 87, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 160, 161, 162, 163,
    164, 165, 196, 197, 198, 199, 200, 201, 130, 131, 132, 133, 134, 135, 166, 167, 168, 169, 170,
    171, 202, 203, 204, 205, 206, 207, 136, 137, 138, 139, 140, 141, 172, 173, 174, 175, 176, 177,
    208, 209, 210, 211, 212, 213, 142, 143, 144, 145, 146, 147, 178, 179, 180, 181, 182, 183, 214,
    215, 216, 217, 218, 219, 148, 149, 150, 151, 152, 153, 184, 185, 186, 187, 188, 189, 220, 221,
    222, 223, 224, 225, 154, 155, 156, 157, 158, 159, 190, 191, 192, 193, 194, 195, 226, 227, 228,
    229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 0, 0, 0, 0, 0, 0,
    244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 0, 0, 0, 0, 0, 0,
];

pub const TITLE_COLOR: Color = Color::Indexed(222);
pub const MENUFG_COLOR: Color = Color::Indexed(253);
pub const MENUBG_COLOR: Color = Color::Indexed(0);
pub const MSG_COLOR: Color = Color::Indexed(251);

pub struct TeditRender {
    pub scene: Scene,
    pub escfile: String,
    pub init: bool,
}

impl TeditRender {
    pub fn new(fpath: &str) -> Self {
        let mut scene = Scene::new();

        // Color box... (position will be set in do_init)
        let mut l = Sprite::new(0, 0, (COLORW + 2) as u16, (COLORH + 2) as u16);
        l.set_border(
            Borders::ALL,
            BorderType::Rounded,
            Style::default().fg(Color::DarkGray).bg(Color::Reset),
        );
        l.content.set_str(
            7,
            0,
            "BgColor",
            Style::default().fg(TITLE_COLOR).bg(Color::Indexed(0)),
        );
        for i in 0..270 {
            let blk = cellsym(160);
            let color = COLOR_PATTERN[i as usize];
            let tc;
            let display_char: &str;
            if color == 256 {
                display_char = "R";
                tc = Color::Indexed(243);
            } else {
                display_char = &blk;
                tc = Color::Indexed(color as u8);
            }
            l.content.set_str(
                i % COLORW + 1,
                i / COLORW + 1,
                display_char,
                Style::default().fg(tc),
            );
        }
        scene.add_sprite(l, "COLOR");

        // Symbol box...
        let mut cl = Sprite::new(0, 0, (SYMW + 2) as u16, (SYMH + 2) as u16);
        cl.set_border(
            Borders::ALL,
            BorderType::Rounded,
            Style::default().fg(Color::DarkGray).bg(Color::Reset),
        );
        cl.content.set_str(
            6,
            0,
            "Symbols",
            Style::default().fg(TITLE_COLOR).bg(Color::Indexed(0)),
        );
        for i in 0..SYMH as usize - 2 {
            for j in 0..SYMW as usize {
                let sidx = SYMBOL_SDL_LOW[i * SYMW as usize + j];
                cl.content.set_str_tex(
                    1 + j as u16,
                    i as u16 + 1,
                    cellsym(sidx as u8),
                    Style::default().fg(Color::White).bg(Color::Indexed(0)),
                    0,
                );
            }
        }
        cl.content.set_str(
            14,
            SYMH,
            "NEXT>",
            Style::default().fg(Color::LightGreen).bg(Color::Indexed(0)),
        );
        scene.add_sprite(cl, "SYMBOL");

        // Edit box... (position will be set in do_init)
        let mut elb = Sprite::new(0, 0, (EDITW + 2) as u16, (EDITH + 2) as u16);
        elb.set_border(
            Borders::ALL,
            BorderType::Rounded,
            Style::default().fg(Color::DarkGray).bg(Color::Reset),
        );
        elb.content.set_str(
            EDITW / 2 - 2,
            0,
            "Editor",
            Style::default().fg(TITLE_COLOR).bg(Color::Indexed(0)),
        );
        scene.add_sprite(elb, "EDIT-BORDER");

        // EDIT content sprite (position will be set in do_init)
        let el = Sprite::new(0, 0, EDITW as u16, EDITH as u16);
        scene.add_sprite(el, "EDIT");

        // MSG1 sprite (position will be set in do_init)
        let mut msg1 = Sprite::new(0, 0, (SYMW + 2) as u16, 1u16);
        msg1.content.set_str(
            0,
            0,
            "PEN",
            Style::default().fg(MENUFG_COLOR).bg(MENUBG_COLOR),
        );
        msg1.content.set_str(
            3,
            0,
            "",
            Style::default().bg(Color::Indexed(0)).fg(MENUBG_COLOR),
        );
        scene.add_sprite(msg1, "MSG1");

        // MSG3 sprite (position will be set in do_init)
        let mut msg3 = Sprite::new(0, 0, (EDITW + 2) as u16, 1u16);
        msg3.content.set_str(
            0,
            0,
            "FILE",
            Style::default().fg(MENUFG_COLOR).bg(MENUBG_COLOR),
        );
        msg3.content.set_str(
            4,
            0,
            "",
            Style::default().bg(Color::Indexed(0)).fg(MENUBG_COLOR),
        );
        msg3.content
            .set_str(6, 0, fpath, Style::default().fg(MSG_COLOR));
        msg3.content.set_str(
            EDITW - 4,
            0,
            "SAVE>",
            Style::default().fg(Color::LightGreen).bg(Color::Indexed(0)),
        );
        scene.add_sprite(msg3, "MSG3");

        event_register("Tedit.RedrawEdit", "draw_edit");
        event_register("Tedit.RedrawPen", "draw_pen");
        event_register("Tedit.Save", "save");

        timer_register("Tedit.HelpTimer", 6.0, "help_timer");
        timer_fire("Tedit.HelpTimer", 0u8);

        Self {
            scene,
            escfile: String::from(fpath),
            init: false,
        }
    }

    /// Initialize sprite positions after adapter is initialized
    /// Uses set_cell_pos which auto-converts cell coordinates to pixel coordinates
    fn do_init(&mut self, _ctx: &mut Context) {
        if self.init {
            return;
        }

        // COLOR box: cell position (0, SYMH + 2)
        let color_sprite = self.scene.get_sprite("COLOR");
        color_sprite.set_cell_pos(0, SYMH + 2);

        // SYMBOL box: cell position (0, 0)
        let symbol_sprite = self.scene.get_sprite("SYMBOL");
        symbol_sprite.set_cell_pos(0, 0);

        // EDIT-BORDER: cell position (SYMW + 2, 0)
        let edit_border = self.scene.get_sprite("EDIT-BORDER");
        edit_border.set_cell_pos(SYMW + 2, 0);

        // EDIT content: cell position (SYMW + 3, 1)
        let edit_sprite = self.scene.get_sprite("EDIT");
        edit_sprite.set_cell_pos(SYMW + 3, 1);

        // MSG1: cell position (0, EDITH + 2)
        let msg1 = self.scene.get_sprite("MSG1");
        msg1.set_cell_pos(0, EDITH + 2);

        // MSG3: cell position (SYMW + 2, EDITH + 2)
        let msg3 = self.scene.get_sprite("MSG3");
        msg3.set_cell_pos(SYMW + 2, EDITH + 2);

        self.init = true;
    }

    pub fn save(&mut self, ctx: &mut Context, _model: &mut TeditModel) {
        let el: &mut Sprite = self.scene.get_sprite("EDIT");
        if let Some(ast) = ctx.asset_manager.get(&self.escfile) {
            match ast.get_state() {
                AssetState::Ready => {
                    ast.save(&el.content);
                    info!("rawdata..{:?}", ast.get_base().raw_data);
                    fs::write(&self.escfile, &ast.get_base().raw_data).unwrap();
                }
                _ => {}
            }
        }
    }

    pub fn draw_pen(&mut self, _context: &mut Context, d: &mut TeditModel) {
        let cb = self.scene.get_sprite("COLOR");

        if d.color_tab_idx == 0 {
            cb.content.set_str(
                7,
                0,
                "FgColor",
                Style::default().fg(TITLE_COLOR).bg(Color::Indexed(0)),
            );
        } else {
            cb.content.set_str(
                7,
                0,
                "BgColor",
                Style::default().fg(TITLE_COLOR).bg(Color::Indexed(0)),
            );
        }

        let sb = self.scene.get_sprite("SYMBOL");
        for i in 0..SYMH as usize - 2 {
            for j in 0..SYMW as usize {
                let (sidx, fc) = if d.sym_tab_idx == 0 {
                    (SYMBOL_SDL_LOW[i * SYMW as usize + j], Color::White)
                } else if d.sym_tab_idx == 1 {
                    (SYMBOL_SDL[i * SYMW as usize + j], Color::White)
                } else {
                    if j == 0 || j == 17 {
                        (32, Color::Reset)
                    } else {
                        ((i * (SYMW - 2) as usize + j - 1) as u8, Color::White)
                    }
                };
                sb.content.set_str_tex(
                    1 + j as u16,
                    i as u16 + 1,
                    cellsym(sidx as u8),
                    Style::default().fg(fc).bg(Color::Reset),
                    d.sym_tab_idx,
                );
            }
        }
        sb.content.set_str(
            14,
            SYMH,
            "NEXT>",
            Style::default().fg(Color::LightGreen).bg(Color::Indexed(0)),
        );

        let m1: &mut Sprite = self.scene.get_sprite("MSG1");
        match d.curpen {
            TeditPen::SYMBOL(idx) => {
                m1.content.set_str(
                    5,
                    0,
                    format!("symbol {}             ", idx),
                    Style::default().fg(MSG_COLOR).bg(Color::Indexed(0)),
                );
                m1.content.set_str(
                    17,
                    0,
                    format!("{}", cellsym(idx as u8)),
                    Style::default()
                        .fg(MSG_COLOR)
                        .bg(Color::Indexed(d.sym_tab_idx)),
                );
            }
            TeditPen::FORE(idx) | TeditPen::BACK(idx) => {
                let color = COLOR_PATTERN[idx as usize];
                let tc;
                let cmsg;
                if color == 256 {
                    tc = Color::Indexed(243);
                    if d.curpen == TeditPen::FORE(idx) {
                        cmsg = format!("fg:Reset   ");
                    } else {
                        cmsg = format!("bg:Reset   ");
                    }
                } else {
                    tc = Color::Indexed(color as u8);
                    if d.curpen == TeditPen::FORE(idx) {
                        cmsg = format!("fg:{}      ", COLOR_PATTERN[idx as usize]);
                    } else {
                        cmsg = format!("bg:{}      ", COLOR_PATTERN[idx as usize]);
                    }
                }
                m1.content
                    .set_str(5, 0, cmsg, Style::default().fg(MSG_COLOR));
                m1.content.set_str(17, 0, "♥", Style::default().fg(tc));
            }
        }
    }

    pub fn draw_edit(&mut self, _context: &mut Context, d: &mut TeditModel) {
        let si = d.cury * EDITW + d.curx;
        let elb: &mut Sprite = self.scene.get_sprite("EDIT");
        match d.curpen {
            TeditPen::SYMBOL(idx) => {
                elb.content.content[si as usize].set_symbol(&cellsym(idx as u8));
                elb.content.content[si as usize].set_fg(Color::White);
                elb.content.content[si as usize].set_bg(Color::Indexed(d.sym_tab_idx));
            }
            TeditPen::FORE(idx) => {
                let tc;
                let color = COLOR_PATTERN[idx as usize];
                if color == 256 {
                    tc = Color::Reset;
                } else {
                    tc = Color::Indexed(color as u8);
                }
                elb.content.content[si as usize].set_fg(tc);
            }
            TeditPen::BACK(idx) => {
                let tc;
                let color = COLOR_PATTERN[idx as usize];
                if color == 256 {
                    tc = Color::Reset;
                } else {
                    tc = Color::Indexed(color as u8);
                }
                elb.content.content[si as usize].set_bg(tc);
            }
        }
    }
}

impl Render for TeditRender {
    type Model = TeditModel;

    fn init(&mut self, context: &mut Context, _data: &mut Self::Model) {
        context.adapter.init(
            SYMW + 2 + EDITW + 2,
            EDITH + 3,
            1.0,
            1.0,
            "tedit".to_string(),
        );
        self.scene.init(context);
        let l = self.scene.get_sprite("EDIT");
        l.set_content_by_asset(
            &mut context.asset_manager,
            AssetType::ImgPix,
            &self.escfile,
            0,
            0,
            0,
        );
    }

    fn handle_event(&mut self, context: &mut Context, model: &mut Self::Model, _dt: f32) {
        if event_check("Tedit.RedrawEdit", "draw_edit") {
            self.draw_edit(context, model);
        }

        if event_check("Tedit.RedrawPen", "draw_pen") {
            self.draw_pen(context, model);
        }

        if event_check("Tedit.Save", "save") {
            self.save(context, model);
        }
    }

    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {
        if event_check("Tedit.HelpTimer", "help_timer") {
            timer_fire("Tedit.HelpTimer", 0u8);
        }
    }

    fn draw(&mut self, context: &mut Context, _model: &mut Self::Model, _dt: f32) {
        self.do_init(context);
        if let Err(e) = self.scene.draw(context) {
            info!("draw error:{}", e);
        }
    }
}
