#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::gl::GlTransition;
use crate::model::{PetviewModel, PETH, PETW};
use log::info;
use std::fmt::Write;
use std::io::Cursor;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::adapter::sdl::SdlAdapter,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};

const PIXW: u16 = 40;
const PIXH: u16 = 25;

fn debug_img(img: &[u8], w: usize, h: usize) {
    let mut idx = 0;
    for i in 0..h {
        let mut line = " ".to_string();
        for j in 0..w {
            write!(line, " {}.{}.{}.{} ", img[idx + 0], img[idx + 1], img[idx + 2], img[idx + 3]).unwrap();
            idx += 4;
        }
        info!("{:?}", line);
    }
}

pub struct PetviewRender {
    pub panel: Panel,
    pub glt: Option<GlTransition>,
    pub progress: f32,
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
        panel.add_pixel_sprite(p2, "petimg2");

        let glt = None;

        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self {
            panel,
            glt,
            progress: 0.0,
        }
    }
}

impl Render for PetviewRender {
    type Model = PetviewModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(PETW + 2, PETH, 1.0, 1.0, "petview".to_string());
        self.panel.init(ctx);

        let sa = ctx.adapter.as_any().downcast_mut::<SdlAdapter>().unwrap();

        let p1 = self.panel.get_pixel_sprite("petimg1");
        asset2sprite!(p1, ctx, "1.pix");
        let img1 = p1.content.get_rgba_image();

        let p2 = self.panel.get_pixel_sprite("petimg2");
        asset2sprite!(p2, ctx, "2.pix");
        let img2 = p2.content.get_rgba_image();

        debug_img(&img1, PIXW as usize, PIXH as usize);
        info!("........");
        debug_img(&img2, PIXW as usize, PIXH as usize);

        // let img1: Vec<u8> = vec![
        //     201, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 
        //     202, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255,
        //     203, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 
        //     204, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255,
        // ];
        // let img2: Vec<u8> = vec![
        //     0, 201, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 
        //     0, 202, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255,
        //     0, 203, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 
        //     0, 204, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255,
        // ];

        if let Some(gl) = &sa.gl {
            let mut glt = GlTransition::new(&gl, PIXW as u32, PIXH as u32);
            glt.set_texture(&gl, &img1, &img2);
            self.glt = Some(glt);
        }
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, _model: &mut Self::Model, _dt: f32) {
        let sa = ctx.adapter.as_any().downcast_mut::<SdlAdapter>().unwrap();
        if event_check("PetView.Timer", "pet_timer") {
            let p1 = self.panel.get_pixel_sprite("petimg2");
            if let Some(gl) = &sa.gl {
                if let Some(glt) = &mut self.glt {
                    glt.render_frame(&gl, self.progress);
                    info!("p..{} pixels...{}", self.progress, glt.pixels.len());
                    debug_img(&glt.pixels, PIXW as usize, PIXH as usize);
                    p1.content.set_rgba_image(&glt.pixels, PIXW, PIXH);
                    self.progress += 0.03;
                    if self.progress >= 1.0 {
                        self.progress = 0.0;
                    }
                }
            }
            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
