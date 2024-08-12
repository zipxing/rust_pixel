#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::PaletteState::*;
use crate::model::*;
use log::info;
use num_traits::FromPrimitive;
use palette_lib::COLORS_WITH_NAME;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::{Color, ColorData, ColorPro, ColorSpace::*, Style},
    util::Rect,
};
use std::cell::Cell;

pub struct PaletteRender {
    pub panel: Panel,
}

impl PaletteRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // creat main layer
        panel.add_layer("main");

        // background
        let gb = Sprite::new(0, 0, PALETTEW, PALETTEH);
        panel.add_layer_sprite(gb, "main", "back");

        // top menu
        let mb = Sprite::new(MENUX, MENUY, MENUW, 1);
        panel.add_layer_sprite(mb, "main", "menu");

        // main color
        let pl = Sprite::new(4, 24, 12, 6);
        panel.add_layer_sprite(pl, "main", "main_color");

        // 3 similar colors
        for i in 0..3 {
            let pl = Sprite::new(61, 25 + i * 2, C_WIDTH - 2, 1);
            panel.add_layer_sprite(pl, "main", &format!("simi{}", i));
        }

        let pl = Sprite::new(2, 22, 40, 1);
        panel.add_layer_sprite(pl, "main", "main_color_str");

        for i in 0..MAIN_COLOR_MSG_Y {
            for j in 0..MAIN_COLOR_MSG_X {
                let pl = Sprite::new(j * 20 + 22, 23 + i, 40, 1);
                panel.add_layer_sprite(
                    pl,
                    "main",
                    &format!("main_color_str{}", i * MAIN_COLOR_MSG_X + j),
                );
            }
        }

        // creat 7 state layers
        let help_msg = [
            "← ↑ → ↓ : select named colors",
            "← ↑ → ↓ : select named colors",
            "h : hsv picker     r : rgb picker     tab ← ↑ → ↓ : change value",
            "h : hsv picker     r : rgb picker     tab ← ↑ → ↓ : change value",
            "← ↑ → ↓ : select random colors",
            "a : add input color  d : delete input color  tab ← ↑ → ↓ : change value",
            "← ↑ → ↓ : select PHI(golden ratio) colors",
        ];
        for i in 0..7 {
            let ls = format!("{}", i);
            panel.add_layer(&ls);
            let mut pl = Sprite::new(ADJX + 1, ADJY + 30, C_WIDTH * 4, 1);
            pl.set_color_str(0, 0, help_msg[i], Color::Gray, Color::Reset);
            panel.add_layer_sprite(pl, &ls, "help_msg");
            if i != 0 {
                panel.deactive_layer(&ls);
            }
        }

        // named colors in layer0 or layer1
        for i in 0..2 {
            for row in 0..ROW_COUNT {
                for col in 0..COL_COUNT {
                    let pl = Sprite::new(ADJX + col * C_WIDTH + 1, ADJY + row, C_WIDTH - 1, 1);
                    let idx = (ROW_COUNT * COL_COUNT * i + row * COL_COUNT + col) as usize;
                    if idx >= COLORS_WITH_NAME.len() {
                        break;
                    }
                    panel.add_layer_sprite(pl, &format!("{}", i), &format!("{}", idx));
                }
            }
        }

        // color picker in layer2
        for y in 0..PICKER_COUNT_Y {
            for x in 0..PICKER_COUNT_X {
                let pl = Sprite::new(ADJX + x, ADJY + y, 1, 1);
                panel.add_layer_sprite(pl, "2", &format!("pick{}", y * PICKER_COUNT_X + x));
            }
        }
        for i in 0..PICKER_COUNT_X {
            let pl = Sprite::new(2 + i, 20, 1, 1);
            panel.add_layer_sprite(pl, "2", &format!("hsv_pick{}", i));
        }

        // color picker in layer3
        let pcs = [Color::Red, Color::Green, Color::Blue];
        for y in 0..3 {
            for x in 0..PICKER_COUNT_X {
                let mut pl = Sprite::new(2 + x, 9 + y * 2, 1, 1);
                pl.set_color_str(0, 0, " ", Color::Reset, pcs[y as usize]);
                panel.add_layer_sprite(pl, "3", &format!("rgb_pick_{}_{}", y, x));
            }
        }

        // color random in layer 4
        for y in 0..RANDOM_Y {
            for x in 0..RANDOM_X {
                let pl = Sprite::new(ADJX + x * RANDOM_W, ADJY + y, RANDOM_W - 1, 1);
                panel.add_layer_sprite(pl, "4", &format!("random{}", y * RANDOM_X + x));
            }
        }

        // color golden in layer 6
        for y in 0..RANDOM_Y {
            for x in 0..RANDOM_X {
                let pl = Sprite::new(ADJX + x * RANDOM_W, ADJY + y, RANDOM_W - 1, 1);
                panel.add_layer_sprite(pl, "6", &format!("random{}", y * RANDOM_X + x));
            }
        }

        // color picker in layer 5
        for y in 0..PICKER_COUNT_Y {
            for x in 0..PICKER_COUNT_X_GRADIENT {
                let pl = Sprite::new(ADJX + x, ADJY + y, 1, 1);
                panel.add_layer_sprite(pl, "5", &format!("pick{}", y * PICKER_COUNT_X + x));
            }
        }
        for i in 0..PICKER_COUNT_X_GRADIENT {
            let pl = Sprite::new(2 + i, 20, 1, 1);
            panel.add_layer_sprite(pl, "5", &format!("hsv_pick{}", i));
        }
        for i in 0..GRADIENT_INPUT_COUNT {
            let pl = Sprite::new(60, i as u16 + ADJY, 8, 1);
            panel.add_layer_sprite(pl, "5", &format!("gi_input{}", i));
        }
        for y in 0..GRADIENT_Y {
            for x in 0..GRADIENT_X {
                let pl = Sprite::new(67 + ADJX, y as u16 + ADJY, 9, 1);
                panel.add_layer_sprite(pl, "5", &format!("gi{}", y * GRADIENT_X + x));
            }
        }

        // creat select cursor layer
        panel.add_layer("select");
        for i in 0..5 {
            let pl = Sprite::new(0, 0, 1, 1);
            panel.add_layer_sprite(pl, "select", &format!("cursor{}", i));
        }
        event_register("Palette.RedrawSelect", "draw_select");
        event_register("Palette.RedrawMenu", "draw_menu");
        event_register("Palette.RedrawPanel", "draw_panel");
        event_register("Palette.RedrawMainColor", "draw_main_color");
        event_register("Palette.RedrawPicker", "draw_picker");
        event_register("Palette.RedrawGradient", "draw_gradient");
        event_register("Palette.RedrawRandom", "draw_random");

        Self { panel }
    }

    pub fn draw_select<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        let state = PaletteState::from_usize(ctx.state as usize).unwrap();
        match state {
            NameA | NameB => {
                let pl = self.panel.get_layer_sprite("select", "cursor0");
                let idx = d.select.cur().y * COL_COUNT as usize
                    + d.select.cur().x
                    + ctx.state as usize * 76;
                let cr = d.named_colors[idx].1;
                pl.set_color_str(0, 0, "", Color::Green, Color::Black);
                pl.set_pos(
                    2 + d.select.cur().x as u16 * C_WIDTH,
                    2 + d.select.cur().y as u16,
                );
                for i in 1..5 {
                    let pl = self
                        .panel
                        .get_layer_sprite("select", &format!("cursor{}", i));
                    pl.set_hidden(true);
                }
            }
            Random | Golden => {
                let pl = self.panel.get_layer_sprite("select", "cursor0");
                let idx = d.select.cur().y * RANDOM_X as usize + d.select.cur().x;
                let cr = d.random_colors[idx];
                pl.set_color_str(0, 0, "", Color::Green, Color::Black);
                pl.set_pos(
                    1 + d.select.cur().x as u16 * RANDOM_W,
                    2 + d.select.cur().y as u16,
                );
                for i in 1..5 {
                    let pl = self
                        .panel
                        .get_layer_sprite("select", &format!("cursor{}", i));
                    pl.set_hidden(true);
                }
            }
            PickerB => {
                let pl = self.panel.get_layer_sprite("select", "cursor0");
                let idx = d.select.area;
                pl.set_color_str(0, 0, "", Color::Green, Color::Black);
                pl.set_pos(1, 9 + idx as u16 * 2);
                let bcs = [Color::Red, Color::Green, Color::Blue];
                for i in 0..3 {
                    let pl = self.panel.get_layer_sprite("select", &format!("cursor{}", i+1));
                    pl.set_color_str(0, 0, "∙", Color::White, bcs[i]);
                    let x = d.select.ranges[i].x;
                    pl.set_pos((x as f64 / 256.0 * PICKER_COUNT_X as f64) as u16 + 2, 9 + i as u16 * 2);
                    pl.set_hidden(false);
                }
            }
            PickerA | Gradient => {
                let pl = self.panel.get_layer_sprite("select", "cursor0");
                let idx = d.select.area;
                pl.set_color_str(0, 0, "", Color::Green, Color::Black);
                if state == PickerA {
                    pl.set_pos(1, idx as u16 * 18 + 2);
                } else {
                    if idx < 3 {
                        pl.set_pos(
                            1 + idx as u16 / 2 * (PICKER_COUNT_X_GRADIENT + 1),
                            idx as u16 % 2 * 18 + 2,
                        );
                    } else {
                        pl.set_pos(PICKER_COUNT_X_GRADIENT + 11, (idx - 1) as u16 % 2 * 18 + 2);
                    }
                }
                if state == Gradient {
                    let pl = self.panel.get_layer_sprite("select", "cursor3");
                    if d.gradient_input_colors.len() != 0 {
                        let cr = d.gradient_input_colors[d.select.ranges[2].y];
                        pl.set_color_str(
                            0,
                            0,
                            "∙",
                            if cr.is_dark() {
                                Color::White
                            } else {
                                Color::Black
                            },
                            Color::Professional(cr),
                        );
                        pl.set_pos(
                            d.select.ranges[2].x as u16 + 3 + PICKER_COUNT_X_GRADIENT,
                            d.select.ranges[2].y as u16 + 2,
                        );
                        pl.set_hidden(false);
                    } else {
                        pl.set_hidden(true);
                    }
                    let pl = self.panel.get_layer_sprite("select", "cursor4");
                    if d.gradient_colors.len() != 0 {
                        let cr = d.gradient_colors[d.select.ranges[3].y];
                        pl.set_color_str(
                            0,
                            0,
                            "∙",
                            if cr.is_dark() {
                                Color::White
                            } else {
                                Color::Black
                            },
                            Color::Professional(cr),
                        );
                        pl.set_pos(
                            d.select.ranges[3].x as u16 + 12 + PICKER_COUNT_X_GRADIENT,
                            d.select.ranges[3].y as u16 + 2,
                        );
                        pl.set_hidden(false);
                    } else {
                        pl.set_hidden(true);
                    }
                } else {
                    for i in 3..5 {
                        let pl = self
                            .panel
                            .get_layer_sprite("select", &format!("cursor{}", i));
                        pl.set_hidden(true);
                    }
                }
                let pl = self.panel.get_layer_sprite("select", "cursor1");
                let cr = get_pick_color(
                    if ctx.state == 2 {
                        PICKER_COUNT_X as usize
                    } else {
                        PICKER_COUNT_X_GRADIENT as usize
                    },
                    d.select.ranges[0].x,
                    d.select.ranges[0].y,
                    d.select.ranges[1].x,
                    0,
                );
                pl.set_color_str(
                    0,
                    0,
                    "∙",
                    if cr.is_dark() {
                        Color::White
                    } else {
                        Color::Black
                    },
                    Color::Professional(cr),
                );
                pl.set_pos(
                    d.select.ranges[0].x as u16 + 2,
                    d.select.ranges[0].y as u16 + 2,
                );
                pl.set_hidden(false);
                let pl = self.panel.get_layer_sprite("select", "cursor2");
                let cr = get_pick_color(
                    if ctx.state == 2 {
                        PICKER_COUNT_X as usize
                    } else {
                        PICKER_COUNT_X_GRADIENT as usize
                    },
                    d.select.ranges[0].x,
                    d.select.ranges[0].y,
                    d.select.ranges[1].x,
                    1,
                );
                pl.set_color_str(
                    0,
                    0,
                    "∙",
                    if cr.is_dark() {
                        Color::White
                    } else {
                        Color::Black
                    },
                    Color::Professional(cr),
                );
                pl.set_pos((d.select.ranges[1].x / 4) as u16 + 2, 20);
                pl.set_hidden(false);
            }
        }
    }

    pub fn draw_panel<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        info!("draw_panel_clear....");
        for i in 0..7 {
            if i != ctx.state as usize {
                self.panel.deactive_layer(&format!("{}", i));
            } else {
                self.panel.active_layer(&format!("{}", i));
            }
        }
    }

    pub fn draw_gradient<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        if ctx.state != Gradient as u8 {
            return;
        }
        for i in 0..GRADIENT_INPUT_COUNT {
            let pl = self.panel.get_layer_sprite("5", &format!("gi_input{}", i));
            if i < d.gradient_input_colors.len() as u16 {
                pl.set_hidden(false);
                pl.set_color_str(
                    0,
                    0,
                    "████████",
                    Color::Professional(d.gradient_input_colors[i as usize]),
                    Color::Reset,
                );
            } else {
                pl.set_hidden(true);
            }
        }
        for y in 0..GRADIENT_Y {
            for x in 0..GRADIENT_X {
                let idx = y * GRADIENT_X + x;
                let pl = self.panel.get_layer_sprite("5", &format!("gi{}", idx));
                if idx < d.gradient_colors.len() as u16 {
                    pl.set_hidden(false);
                    pl.set_color_str(
                        0,
                        0,
                        "            ",
                        Color::White,
                        Color::Professional(d.gradient_colors[idx as usize]),
                    );
                } else {
                    pl.set_hidden(true);
                }
            }
        }
    }

    pub fn draw_random<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        if ctx.state != 4 && ctx.state != 6 {
            return;
        }
        let ls = format!("{}", ctx.state);
        for y in 0..RANDOM_Y {
            for x in 0..RANDOM_X {
                let i = y * RANDOM_X + x;
                let pl = self.panel.get_layer_sprite(&ls, &format!("random{}", i));
                pl.set_color_str(
                    0,
                    0,
                    &format!(" {:width$}", " ", width = C_WIDTH as usize - 1),
                    Color::Reset,
                    Color::Professional(d.random_colors[i as usize]),
                );
            }
        }
    }

    pub fn draw_picker<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        let state = PaletteState::from_usize(ctx.state as usize).unwrap();
        match state {
            PickerA | Gradient => {
                let w = if ctx.state == 2 {
                    PICKER_COUNT_X
                } else {
                    PICKER_COUNT_X_GRADIENT
                };
                for y in 0..PICKER_COUNT_Y {
                    for x in 0..w {
                        let pl = self.panel.get_layer_sprite(
                            &format!("{}", ctx.state),
                            &format!("pick{}", y * PICKER_COUNT_X + x),
                        );

                        let cr = get_pick_color(
                            w as usize,
                            x as usize,
                            y as usize,
                            d.select.ranges[1].x,
                            0,
                        );

                        let color = Color::Professional(cr);
                        pl.set_color_str(0, 0, "  ", color, color);
                        pl.set_color_str(0, 0, "  ", color, color);
                    }
                }
                for i in 0..w {
                    let pl = self
                        .panel
                        .get_layer_sprite(&format!("{}", ctx.state), &format!("hsv_pick{}", i));
                    let cr = ColorPro::from_space_f64(
                        HSVA,
                        i as f64 * (360.0 / w as f64),
                        1.0,
                        1.0,
                        1.0,
                    );
                    let color = Color::Professional(cr);
                    pl.set_color_str(0, 0, " ", color, color);
                }
            }
            PickerB => {}
            _ => {}
        }
    }

    pub fn draw_named_colors<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();

        for i in 0..2 {
            for row in 0..ROW_COUNT {
                for col in 0..COL_COUNT {
                    let idx = (ROW_COUNT * COL_COUNT * i + row * COL_COUNT + col) as usize;
                    if idx >= COLORS_WITH_NAME.len() {
                        break;
                    }
                    let pl = self
                        .panel
                        .get_layer_sprite(&format!("{}", i), &format!("{}", idx));
                    let s = d.named_colors[idx].0;
                    let cr = d.named_colors[idx].1;
                    let color = Color::Professional(cr);
                    pl.set_color_str(
                        0,
                        0,
                        &format!("{:width$}", s, width = C_WIDTH as usize - 1),
                        if cr.is_dark() {
                            Color::White
                        } else {
                            Color::Black
                        },
                        color,
                    );
                }
            }
        }
    }

    pub fn draw_main_color<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        let pl = self.panel.get_layer_sprite("main", "main_color");
        for i in 0..6 {
            pl.set_color_str(
                0,
                i,
                "            ",
                Color::White,
                Color::Professional(d.main_color),
            );
        }

        let pl = self.panel.get_layer_sprite("main", "main_color_str");
        let rgb = d.main_color.get_srgba_u8();
        pl.set_color_str(
            0,
            0,
            &format!("{:width$}", &get_color_info(d.main_color, 0), width = 18),
            Color::DarkGray,
            Color::Black,
        );

        for i in 0..MAIN_COLOR_MSG_Y {
            for j in 0..MAIN_COLOR_MSG_X {
                let pl = self.panel.get_layer_sprite(
                    "main",
                    &format!("main_color_str{}", i * MAIN_COLOR_MSG_X + j),
                );
                pl.set_color_str(
                    0,
                    0,
                    &format!(
                        "{:width$}",
                        &get_color_info(d.main_color, i * MAIN_COLOR_MSG_X + j + 1),
                        width = 18
                    ),
                    if i % 2 == 0 {
                        Color::Rgba(125, 130, 130, 255)
                    } else {
                        Color::Rgba(132, 132, 127, 255)
                    },
                    Color::Black,
                );
            }
        }

        let mut ids: Vec<usize> = vec![];
        ids.push(d.main_color_similar.0);
        ids.push(d.main_color_similar.1);
        ids.push(d.main_color_similar.2);

        for i in 0..3 {
            let pl = self.panel.get_layer_sprite("main", &format!("simi{}", i));

            let s = COLORS_WITH_NAME[ids[i]].0;
            let cr = COLORS_WITH_NAME[ids[i]].1;
            let color = Color::Professional(cr);

            pl.set_color_str(
                0,
                0,
                &format!("{:width$}", s, width = C_WIDTH as usize),
                if cr.is_dark() {
                    Color::White
                } else {
                    Color::Black
                },
                color,
            );
        }
    }

    pub fn draw_menu<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        let state = PaletteState::from_usize(ctx.state as usize).unwrap();
        let mb = self.panel.get_layer_sprite("main", "menu");
        let cst = match state {
            NameA | NameB => 0,
            PickerA | PickerB => 1,
            Random => 2,
            Gradient => 3,
            Golden => 4,
        };
        let mut xoff = 0u16;
        let mcolor = [240, 120, 235, 0, 7, 120];
        if cst == 0 {
            mb.set_color_str(
                xoff,
                0,
                "",
                Color::Indexed(mcolor[0]),
                Color::Indexed(mcolor[1]),
            );
        } else {
            mb.set_color_str(
                xoff,
                0,
                "",
                Color::Indexed(mcolor[2]),
                Color::Indexed(mcolor[0]),
            );
        }
        let menu_title = [
            " 1 Named ",
            " 2 Picker ",
            " 3 Random ",
            " 4 Gradient ",
            " 5 Golden ",
        ];
        for i in 0..5 {
            let fg = if cst == i {
                Color::Indexed(mcolor[3])
            } else {
                Color::Indexed(mcolor[4])
            };
            let bg = if cst == i {
                Color::Indexed(mcolor[1])
            } else {
                Color::Indexed(mcolor[0])
            };
            let menu_str = &menu_title[i as usize];
            if cst == i {
                mb.set_color_str(xoff, 0, "", Color::Indexed(mcolor[0]), bg);
            }
            xoff += 1;
            mb.set_color_str(xoff, 0, menu_str, fg, bg);
            xoff += menu_str.len() as u16;
            if cst == i {
                mb.set_color_str(xoff, 0, "", bg, Color::Indexed(mcolor[0]));
            } else {
                mb.set_color_str(xoff, 0, "", Color::Indexed(mcolor[2]), bg);
            }
        }
    }
}

impl Render for PaletteRender {
    fn init<G: Model>(&mut self, context: &mut Context, data: &mut G) {
        context
            .adapter
            .init(PALETTEW + 2, PALETTEH, 1.0, 1.0, "palette".to_string());
        self.panel.init(context);
        self.draw_menu(context, data);

        let d = data.as_any().downcast_mut::<PaletteModel>().unwrap();
        let gb = self.panel.get_layer_sprite("main", "back");
        asset2sprite!(gb, context, "back.txt");

        // for co in 0..CCOUNT {
        //     let gb = self.panel.get_sprite(&format!("COLOR{}", co));
        //     let (r, g, b, a) = d.gradient_colors[co].get_srgba_u8();
        //     let cr = Color::Rgba(r, g, b, 255);
        //     gb.set_color_str(0, 0, &format!("{:10}", " "), Color::White, cr);
        // }
        //

        self.draw_named_colors(context, data);
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Palette.RedrawSelect", "draw_select") {
            self.draw_select(context, data);
        }
        if event_check("Palette.RedrawMenu", "draw_menu") {
            self.draw_menu(context, data);
        }
        if event_check("Palette.RedrawPanel", "draw_panel") {
            self.draw_panel(context, data);
        }
        if event_check("Palette.RedrawMainColor", "draw_main_color") {
            self.draw_main_color(context, data);
        }
        if event_check("Palette.RedrawPicker", "draw_picker") {
            self.draw_picker(context, data);
        }
        if event_check("Palette.RedrawGradient", "draw_gradient") {
            self.draw_gradient(context, data);
        }
        if event_check("Palette.RedrawRandom", "draw_random") {
            self.draw_random(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
