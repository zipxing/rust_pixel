#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{
    PaletteModel, PaletteState, GRADIENT_COUNT, MENUW, MENUX, MENUY, PALETTEH, PALETTEW,
};
use num_traits::FromPrimitive;
use palette_lib::COLORS_WITH_NAME;
use std::cell::Cell;
use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::{Style, Color, ColorData, ColorPro, ColorSpace::*},
};

pub struct PaletteRender {
    pub panel: Panel,
    pub panel_clear: Panel,
    pub panel_main: Panel,
    pub panels: Vec<Panel>,
}

impl PaletteRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();
        let mut panels = vec![];

        // background...
        let gb = Sprite::new(0, 0, PALETTEW, PALETTEH);
        panel.add_sprite(gb, "back");

        let mut panel_clear = Panel::new();
        let mut gb = Sprite::new(2, 2, PALETTEW - 4, PALETTEH - 21);
        for i in 0..PALETTEH - 21 {
            for j in 0..PALETTEW  - 4 {
                gb.content.set_str(j, i, " ", Style::default().bg(Color::Indexed(233)));
            }
        }
        panel_clear.add_sprite(gb, "clear");

        // tab panels...
        for i in 0..6 {
            let p = Panel::new();
            panels.push(p);
        }

        let adjx = 2;
        let adjy = 2;

        let col_count = 4;
        let row_count = 19;
        let c_width = 19;

        for i in 0..2 {
            for row in 0..row_count {
                for col in 0..col_count {
                    let pl = Sprite::new(adjx + col * c_width, adjy + row, c_width, 1);
                    let idx = (row_count * col_count * i + row * col_count + col) as usize;
                    if idx >= COLORS_WITH_NAME.len() {
                        break;
                    }
                    panels[i as usize].add_sprite(pl, &format!("{}", idx));
                }
            }
        }

        let mut panel_main = Panel::new();
        let pl = Sprite::new(4, 24, 12, 6);
        panel_main.add_sprite(pl, "MAIN_COLOR");

        for i in 0..3 {
            let pl = Sprite::new(61, 25 + i * 2, c_width - 2, 1);
            panel_main.add_sprite(pl, &format!("SIMI{}", i));
        }

        let width = 25;
        for y in 0..width {
            for x in 0..width {
                let pl = Sprite::new(adjx + x * 2, adjy + y, 2, 1);
                panels[2].add_sprite(pl, &format!("pick{}", y * width + x));
            }
        }

        for j in 0..2 {
            for i in 0..60 {
                let pl = Sprite::new(8 + i, 29 + j, 1, 1);
                panels[2].add_sprite(pl, &format!("hsv_pick{}", j * 60 + i));
            }
        }

        let mb = Sprite::new(MENUX, MENUY, MENUW, 1);
        panel.add_sprite(mb, "menu");
        // for co in 0..CCOUNT as u16 {
        //     let pl = Sprite::new(adjx + co * 2, adjy - 1, 2, 1);
        //     panel.add_sprite(pl, &format!("COLOR{}", co));
        // }

        event_register("Palette.RedrawMenu", "draw_menu");
        event_register("Palette.RedrawPanel", "draw_panel");
        event_register("Palette.RedrawMainColor", "draw_main_color");

        Self {
            panel,
            panel_clear,
            panel_main,
            panels,
        }
    }

    pub fn draw_panel<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        self.panel_clear.clear(ctx);
        self.panel_clear.draw(ctx).unwrap();
        self.panels[ctx.state as usize].clear(ctx);
    }

    pub fn draw_named_colors<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        let col_count = 4;
        let row_count = 19;
        let c_width = 19;

        for i in 0..2 {
            for row in 0..row_count {
                for col in 0..col_count {
                    let idx = (row_count * col_count * i + row * col_count + col) as usize;
                    if idx >= COLORS_WITH_NAME.len() {
                        break;
                    }
                    let pl = self.panels[i as usize].get_sprite(&format!("{}", idx));
                    let s = d.named_colors[idx].0;
                    let cr = d.named_colors[idx].1;
                    let color = Color::Professional(cr);
                    pl.set_color_str(
                        0,
                        0,
                        &format!("{:width$}", s, width = c_width as usize),
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

    // pub fn draw_tile<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {}
    pub fn draw_main_color<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();
        let pl = self.panel_main.get_sprite("MAIN_COLOR");
        for i in 0..6 {
            pl.set_color_str(
                0,
                i,
                "            ",
                Color::White,
                Color::Professional(d.main_color),
            );
        }

        let mut ids: Vec<usize> = vec![];
        ids.push(d.main_color_similar.0);
        ids.push(d.main_color_similar.1);
        ids.push(d.main_color_similar.2);

        for i in 0..3 {
            let pl = self.panel_main.get_sprite(&format!("SIMI{}", i));

            let s = COLORS_WITH_NAME[ids[i]].0;
            let cr = COLORS_WITH_NAME[ids[i]].1;
            let color = Color::Professional(cr);

            pl.set_color_str(
                0,
                0,
                &format!("{:width$}", s, width = 19usize),
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
        let mb = self.panel.get_sprite("menu");
        let cst = ctx.state as usize;
        let mut xoff = 0u16;
        if cst == 0 {
            mb.set_color_str(xoff, 0, "", Color::Indexed(236), Color::Indexed(120));
        } else {
            mb.set_color_str(xoff, 0, "", Color::Indexed(8), Color::Indexed(236));
        }
        for i in 0..6 {
            let fg = if cst == i {
                Color::Indexed(0)
            } else {
                Color::Indexed(7)
            };
            let bg = if cst == i {
                Color::Indexed(120)
            } else {
                Color::Indexed(236)
            };
            let menu_str = &format!(" {} {:?} ", i + 1, PaletteState::from_usize(i).unwrap());
            if cst == i {
                mb.set_color_str(xoff, 0, "", Color::Indexed(236), bg);
            }
            xoff += 1;
            mb.set_color_str(xoff, 0, menu_str, fg, bg);
            xoff += menu_str.len() as u16;
            if cst == i {
                mb.set_color_str(xoff, 0, "", bg, Color::Indexed(236));
            } else {
                mb.set_color_str(xoff, 0, "", Color::Indexed(8), bg);
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
        self.panel_main.init(context);
        for i in 0..6 {
            self.panels[i].init(context);
        }
        self.draw_menu(context, data);

        let d = data.as_any().downcast_mut::<PaletteModel>().unwrap();
        let gb = self.panel.get_sprite("back");
        asset2sprite!(gb, context, "back.txt");
        // for co in 0..CCOUNT {
        //     let gb = self.panel.get_sprite(&format!("COLOR{}", co));
        //     let (r, g, b, a) = d.gradient_colors[co].get_srgba_u8();
        //     let cr = Color::Rgba(r, g, b, 255);
        //     gb.set_color_str(0, 0, &format!("{:10}", " "), Color::White, cr);
        // }
        //
        let width = 25;

        for y in 0..width {
            for x in 0..width {
                let pl = self.panels[2].get_sprite(&format!("pick{}", y * width + x));
                let cr = d.picker_colors[y * width + x];
                let color = Color::Professional(cr);
                pl.set_color_str(0, 0, "  ", color, color);
            }
        }

        for j in 0..2 {
            for i in 0..60 {
                let pl = self.panels[2].get_sprite(&format!("hsv_pick{}", (j * 60 + i)));
                let cr = ColorPro::from_space_f64(HSVA, (j * 60 + i) as f64 * 3.0, 1.0, 1.0, 1.0);
                let color = Color::Professional(cr);
                pl.set_color_str(0, 0, " ", color, color);
            }
        }
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Palette.RedrawMenu", "draw_menu") {
            self.draw_menu(context, data);
        }
        if event_check("Palette.RedrawPanel", "draw_panel") {
            self.draw_panel(context, data);
        }
        if event_check("Palette.RedrawMainColor", "draw_main_color") {
            self.draw_main_color(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        self.panel.draw(ctx).unwrap();
        self.panel_main.draw(ctx).unwrap();
        self.panels[ctx.state as usize].draw(ctx).unwrap();
    }
}
