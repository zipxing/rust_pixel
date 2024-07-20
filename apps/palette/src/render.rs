#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PaletteModel, CCOUNT, PALETTEH, PALETTEW};
use palette_lib::COLORS_WITH_NAME;
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
        let adjy = 6;

        for row in 0..37 {
            for col in 0..4 {
                let mut pl = Sprite::new(adjx + col * 20, adjy + row, 20, 1);
                let idx = (row * 10 + col) as usize;
                if idx >= COLORS_WITH_NAME.len() {
                    break;
                }
                let s = COLORS_WITH_NAME[idx].0;
                let r = COLORS_WITH_NAME[idx].1;
                let g = COLORS_WITH_NAME[idx].2;
                let b = COLORS_WITH_NAME[idx].3;
                let mut cr = ColorPro::from_space_data(
                    SRGBA,
                    ColorData {
                        v: [r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0],
                    },
                );
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
            let pl = Sprite::new(adjx + co * 2, adjy - 3, 2, 1);
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
            gb.set_color_str(
                0,
                0,
                &format!("{:10}", " "),
                Color::White,
                cr,
            );
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
