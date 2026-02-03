//! Terminal mode renderer for the edit tool
//!
//! Uses Scene architecture for terminal-based sprite editing.

use crate::model::{TeditModel, TeditPen, COLORH, COLORW, EDITH, EDITW, SYMH, SYMW};
use log::info;
use rust_pixel::{
    asset::{AssetState, AssetType},
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::Render,
    render::{
        scene::Scene,
        sprite::{BorderType, Borders, Sprite},
        style::{Color, Style},
    },
};
use std::fs;
use unicode_segmentation::UnicodeSegmentation;

pub const SYMBOL_ASCII: [[&str; 16]; 3] = [
    [
        " !#$%&()*+,-./0123",
        "456789:;\"'<=>?@[\\]",
        "^_`{|}~⌐¬½¼¡«»∙·※⦿",
        "ABCDEFGHIJKLMNOPQR",
        "STUVWXYZabcdefghij",
        "klmnopqrstuvwxyz",
        "▀▄äàåçêëèïîìÄÅÉæÆô",
        "öòûùÿÖÜ¢£¥₧ƒáíóúñÑ",
        "ªº¿αßΓπΣσµτΦΘΩδ∞φε",
        "∩≡±≥≤⌠⌡÷≈‾√ⁿ²♠♣♥♦░",
        "▒▓▙▟▛▜⚆⚇⚈⚉◐◑◓◒▴▾",
        "◂▸←↑→↓⭠⭡⭢⭣⠁⠂⠄⠈⠐⠠⡀⢀",
        "█▉▊▋▌▍▎▏█▇▆▅▄▃▂▁│║",
        "┃─═━┐╮╗┓┌╭╔┏┘╯╝┛└╰",
        "╚┗┤╣┫├╠┣┬╦┳┴╩┻┼╬╋≋",
        "                  ",
    ],
    [
        "😀😃😆😅😂😇😍😎😜",
        "🥺😢😟😤😭😱😡😵🤮",
        "🌼🍉🎃🍄🌹🌻🌸🪴🌷",
        "🌵🌲🌳🌴🎄🌿🍀🌱🪷",
        "🌞🌛⭐️⚡️🌈💦💧☔️❄️ ",
        "🍎🍋🍑🍌🍇🍓🥝🥭🍒",
        "🥬🍆🥕🥚🧅🍞🧄🍗🌶️ ",
        "🍖🦴🍔🍟🍕🥦🍚🥟🍜",
        "🍺🍻🥂🍷🍸🍹🎂🧁🍰",
        "🏀⚽️🏈🥎🏐🎱🏓⛳️🏒",
        "🏹🥊🪂🎣🥇🥈🥉🎲🏆",
        "🚗🚑🚌🚀🚁⛵️⚓️🛬🛩️ ",
        "⏰💰💣🧨💈🎁🎈🎉🔑",
        "👉👆👈👇👍👏👎👊👌",
        "👩🧑👨👵👷👮🥷🙏✌️ ",
        "                  ",
    ],
    [
        "🐶🐱🐭🐹🐰🦊🐻🐼🐨",
        "🐯🦁🐮🐷🐸🐵🐒🐥🦋",
        "🐬🐳🦀🐠🦈🐴🦂🦕🐙",
        "🐏🦒🦓🐆🐫🦌🐘🦛🦏",
        "🦚🦜🐓🦢🐇🐝🐞🐍🐢",
        "🎹🥁🎸🪗🎻🎺🎷🪕🪘",
        "🗿🗽🗼🏰🏯🎡🎢⛲️⛰️",
        "🎠⛱️🏖️🏝️🏜️🌋🏠🏡🏘️",
        "🏚️🏭🏥🏢🏬⛺️🏕️🛖🕌",
        "📱🎙️📺📞🖥️💻⌛️🛠️⚙️ ",
        "🧸🪣📎🔗📒📅🔐✏️ 🧲",
        "💕💝✅❎❌🆘🚫💤🚸",
        "🔴🟠🟡🟢🔵🟣⚫️⚪️🟤",
        "🟥🟧🟨🟩🟦🟪⬛️⬜️🟫",
        "🏧🛃🛅🛄🚹🚺🚼🔆❤️ ",
        "                  ",
    ],
];

fn get_nosdl_sym(sym_tab_idx: u8, idx: u16) -> &'static str {
    let codey = (idx / SYMW) as usize;
    let mut codex = (idx % SYMW) as usize;
    if sym_tab_idx != 0 {
        codex /= 2;
    }
    let graphemes = UnicodeSegmentation::graphemes(SYMBOL_ASCII[sym_tab_idx as usize][codey], true)
        .collect::<Vec<&str>>();
    graphemes[codex]
}

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
pub const MENUBG_COLOR: Color = Color::Indexed(236);
pub const MSG_COLOR: Color = Color::Indexed(251);

pub struct TeditRender {
    pub scene: Scene,
    pub escfile: String,
}

impl TeditRender {
    pub fn new(fpath: &str) -> Self {
        let mut scene = Scene::new();

        // Color box...
        let mut l = Sprite::new(0, SYMH + 2, (COLORW + 2) as u16, (COLORH + 2) as u16);
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
            let blk = "█";
            let color = COLOR_PATTERN[i as usize];
            let tc;
            let display_char;
            if color == 256 {
                display_char = "R";
                tc = Color::Indexed(243);
            } else {
                display_char = blk;
                tc = Color::Indexed(color as u8);
            }
            l.content.set_str(
                i % COLORW + 1,
                i / COLORW + 1,
                display_char,
                Style::default().fg(tc),
            );
        }
        l.content.set_str(
            14,
            COLORH,
            "FGBG>",
            Style::default().fg(Color::LightGreen).bg(Color::Indexed(0)),
        );
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
        for i in 0..SYMH - 3 {
            cl.content.set_str(
                1,
                i as u16 + 1,
                SYMBOL_ASCII[0][i as usize],
                Style::default(),
            );
        }
        cl.content.set_str(
            14,
            SYMH,
            "NEXT>",
            Style::default().fg(Color::LightGreen).bg(Color::Indexed(0)),
        );
        scene.add_sprite(cl, "SYMBOL");

        // Edit box...
        let mut elb = Sprite::new((SYMW + 2) as u16, 0, (EDITW + 2) as u16, (EDITH + 2) as u16);
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

        let el = Sprite::new((SYMW + 3) as u16, 1, EDITW as u16, EDITH as u16);
        scene.add_sprite(el, "EDIT");

        let mut msg1 = Sprite::new(0, (EDITH + 2) as u16, (SYMW + 2) as u16, 1u16);
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

        let mut msg3 = Sprite::new(
            (SYMW + 2) as u16,
            (EDITH + 2) as u16,
            (EDITW + 2) as u16,
            1u16,
        );
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
        }
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
        for i in 0..SYMH - 3 {
            sb.content.set_str(
                1,
                i as u16 + 1,
                SYMBOL_ASCII[d.sym_tab_idx as usize][i as usize],
                Style::default(),
            );
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
                let s = get_nosdl_sym(d.sym_tab_idx, idx);
                m1.content.set_str(
                    5,
                    0,
                    format!("symbol {}             ", s),
                    Style::default().fg(MSG_COLOR),
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
                let s = get_nosdl_sym(d.sym_tab_idx, idx);
                elb.content.content[si as usize].set_symbol(s);
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
            0.5,
            0.5,
            "tedit".to_string(),
        );
        self.scene.init(context);
        let l = self.scene.get_sprite("EDIT");
        l.set_content_by_asset(
            &mut context.asset_manager,
            AssetType::ImgEsc,
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
        if let Err(e) = self.scene.draw(context) {
            info!("draw error:{}", e);
        }
    }
}
