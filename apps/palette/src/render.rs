#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PaletteModel, CCOUNT, PALETTEH, PALETTEW};
use palette_lib::COLORS_WITH_NAME;
use std::cell::Cell;
// use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::{Color, ColorData, ColorPro, ColorSpace::*},
};

pub struct PaletteRender {
    pub panel: Panel,
}

impl PaletteRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        let adjx = 1;
        let adjy = 3;

        let mut ncolors: Vec<(&'static str, Cell<ColorPro>)> = vec![];
        for c in COLORS_WITH_NAME {
            let cr = ColorPro::from_space_data_u8(SRGBA, c.1, c.2, c.3, 255);
            ncolors.push((c.0, Cell::new(cr)));
        }

        // ncolors.sort_by_key(|nc| (1000.0 - nc.1.get().brightness() * 1000.0) as i32);
        // ncolors.sort_by_key(|nc| (1000.0 - nc.1.get().hue() * 1000.0) as i32);
        ncolors.sort_by_key(|nc| (nc.1.get().chroma() * 1000.0) as i32);

        for row in 0..37 {
            for col in 0..4 {
                let mut pl = Sprite::new(adjx + col * 20, adjy + row, 20, 1);
                let idx = (row * 4 + col) as usize;
                if idx >= COLORS_WITH_NAME.len() {
                    break;
                }
                let s = ncolors[idx].0;
                let mut cr = ncolors[idx].1.get();
                let color = Color::Professional(cr);
                pl.set_color_str(
                    0,
                    0,
                    &format!("{:20}", s),
                    if cr.is_dark() {
                        Color::White
                    } else {
                        Color::Black
                    },
                    color,
                );
                panel.add_sprite(pl, &format!("{}", idx));
            }
        }

        for co in 0..CCOUNT as u16 {
            let pl = Sprite::new(adjx + co * 2, adjy - 1, 2, 1);
            panel.add_sprite(pl, &format!("COLOR{}", co));
        }

        // background...
        let mut gb = Sprite::new(0, 0, PALETTEW, PALETTEH);
        gb.set_alpha(30);
        panel.add_sprite(gb, "back");

        event_register("Palette.RedrawTile", "draw_tile");

        Self { panel }
    }

    pub fn draw_tile<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {}
}

impl Render for PaletteRender {
    fn init<G: Model>(&mut self, context: &mut Context, data: &mut G) {
        context
            .adapter
            .init(PALETTEW + 2, PALETTEH, 1.0, 1.0, "palette".to_string());
        self.panel.init(context);

        let d = data.as_any().downcast_mut::<PaletteModel>().unwrap();
        let gb = self.panel.get_sprite("back");
        asset2sprite!(gb, context, "back.txt");
        for co in 0..CCOUNT {
            let gb = self.panel.get_sprite(&format!("COLOR{}", co));
            let (r, g, b, a) = d.colors[co].get_srgba_u8();
            let cr = Color::Rgba(r, g, b, 255);
            gb.set_color_str(0, 0, &format!("{:10}", " "), Color::White, cr);
        }
    }

    fn handle_event<G: Model>(&mut self, context: &mut Context, data: &mut G, _dt: f32) {
        if event_check("Palette.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer<G: Model>(&mut self, _context: &mut Context, _model: &mut G, _dt: f32) {}

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
