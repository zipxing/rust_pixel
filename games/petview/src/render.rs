#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::model::{PetviewModel, PETH, PETW};
use log::info;
use rust_pixel::render::adapter::Adapter;
use rust_pixel::{
    asset::AssetType,
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    // render::adapter::sdl::SdlAdapter,
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};

use std::fmt::Write;
use std::io::Cursor;

const PIXW: u16 = 40;
const PIXH: u16 = 25;

pub struct PetviewRender {
    pub panel: Panel,
}

impl PetviewRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        let mut p1 = Sprite::new(0, 0, PIXW, PIXH);
        p1.set_hidden(true);
        panel.add_pixel_sprite(p1, "petimg1");

        let mut p2 = Sprite::new(0, 0, PIXW, PIXH);
        p2.set_hidden(true);
        panel.add_pixel_sprite(p2, "petimg2");

        let mut p3 = Sprite::new(160u16, 450u16, PIXW, 1u16);
        p3.set_color_str(
            1,
            0,
            "RustPixel - x.com/PETSCIIWORLD",
            Color::Rgba(0, 205, 0, 255),
            Color::Reset,
        );
        panel.add_pixel_sprite(p3, "pet-msg");
        timer_register("PetView.Timer", 0.1, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self { panel }
    }
}

impl Render for PetviewRender {
    type Model = PetviewModel;

    fn init(&mut self, ctx: &mut Context, _data: &mut Self::Model) {
        ctx.adapter
            .init(PETW + 2, PETH, 1.0, 1.0, "petview".to_string());
        self.panel.init(ctx);

        let p1 = self.panel.get_pixel_sprite("petimg1");
        asset2sprite!(p1, ctx, "1.pix");

        let p2 = self.panel.get_pixel_sprite("petimg2");
        asset2sprite!(p2, ctx, "2.pix");
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        if !model.tex_ready {
            let p1 = self.panel.get_pixel_sprite("petimg1");
            asset2sprite!(p1, ctx, &format!("{}.pix", model.img_cur + 1));
            let l1 = p1.check_asset_request(&mut ctx.asset_manager);
            if l1 {
                #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                ctx.adapter.render_buffer_to_texture(&p1.content, 0);
            }
            let p2 = self.panel.get_pixel_sprite("petimg2");
            asset2sprite!(p2, ctx, &format!("{}.pix", model.img_next + 1));
            let l2 = p2.check_asset_request(&mut ctx.asset_manager);
            if l2 {
                #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
                ctx.adapter.render_buffer_to_texture(&p2.content, 1);
            }
            if l1 && l2 {
                model.tex_ready = true;
                info!("tex_ready.........");
            }
        }
        if event_check("PetView.Timer", "pet_timer") {
            // info!("timer......{}", ctx.stage);
            // let p1 = self.panel.get_pixel_sprite("petimg2");
            
            let sa = ctx.adapter.get_base();
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            if let (Some(pix), Some(gl)) = (&mut sa.gl_pixel, &mut sa.gl) {
                pix.bind_target(gl, 3);
                if ctx.state == 0 {
                    pix.render_trans_frame(&gl, 0, 864, 512, 1.0);
                } else {
                    pix.render_trans_frame(
                        &gl,
                        model.trans_effect,
                        864,
                        512,
                        model.progress,
                    );
                }
            }
            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
