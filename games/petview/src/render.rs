#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PetviewModel, PETH, PETW};
use log::info;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::adapter::web::WebAdapter,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};
use std::fmt::Write;
use std::io::Cursor;

const PIXW: u16 = 40;
const PIXH: u16 = 25;

// fn debug_img(img: &[u8], w: usize, h: usize) {
//     let mut idx = 0;
//     for i in 0..h {
//         let mut line = " ".to_string();
//         for j in 0..w {
//             write!(
//                 line,
//                 " {}.{}.{}.{} ",
//                 img[idx + 0],
//                 img[idx + 1],
//                 img[idx + 2],
//                 img[idx + 3]
//             )
//             .unwrap();
//             idx += 4;
//         }
//         info!("{:?}", line);
//     }
// }

pub struct PetviewRender {
    pub panel: Panel,
    pub progress: f32,
    pub tex_ready: bool,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // background...
        let mut gb = Sprite::new(0, 0, PETW, PETH);
        gb.set_alpha(230);
        panel.add_sprite(gb, "back");

        let mut p1 = Sprite::new(0, 0, PIXW, PIXH);
        p1.set_hidden(true);
        panel.add_pixel_sprite(p1, "petimg1");

        let mut p2 = Sprite::new(0, 0, PIXW, PIXH);
        p2.set_hidden(true);
        panel.add_pixel_sprite(p2, "petimg2");

        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self {
            panel,
            progress: 0.0,
            tex_ready: false,
        }
    }
}

impl Render for PetviewRender {
    type Model = PetviewModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(PETW + 2, PETH, 1.0, 1.0, "petview".to_string());
        self.panel.init(ctx);

        let sa = ctx.adapter.as_any().downcast_mut::<WebAdapter>().unwrap();

        let p1 = self.panel.get_pixel_sprite("petimg1");
        asset2sprite!(p1, ctx, "12.pix");

        let p2 = self.panel.get_pixel_sprite("petimg2");
        asset2sprite!(p2, ctx, "8.pix");
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("Template.RedrawTile", "draw_tile") {}
    }

    fn handle_timer(&mut self, ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        let sa = ctx.adapter.as_any().downcast_mut::<WebAdapter>().unwrap();
        if !self.tex_ready && ctx.stage > 200 {
            let p1 = self.panel.get_pixel_sprite("petimg1");
            asset2sprite!(p1, ctx, "12.pix");
            info!("EEEEE....{:?}", &p1.content);
            sa.render_buffer_to_texture(&p1.content, 0);
            let p2 = self.panel.get_pixel_sprite("petimg2");
            asset2sprite!(p2, ctx, "8.pix");
            sa.render_buffer_to_texture(&p2.content, 1);
            self.tex_ready = true;
            info!("tex_ready.........");
        }
        if event_check("PetView.Timer", "pet_timer") {
            // info!("timer......{}", ctx.stage);
            // let p1 = self.panel.get_pixel_sprite("petimg2");
            if let (Some(pix), Some(gl)) = (&mut sa.gl_pix, &mut sa.gl) {
                pix.bind_render_texture(gl, 3);
                pix.clear(gl);
                pix.render_trans_frame(&gl, 40 * 16, 25 * 16, self.progress);
                // sa.sdl_window.as_ref().unwrap().gl_swap_window();
                self.progress += 0.03;
                if self.progress >= 1.0 {
                    self.progress = 0.0;
                }
            }
            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
