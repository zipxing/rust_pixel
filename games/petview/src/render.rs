#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::gl::GlTransition;
use crate::model::{PetviewModel, PETH, PETW};
use log::info;
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

        let p1 = Sprite::new(0, 0, 40, 25);
        panel.add_pixel_sprite(p1, "petimg1");
        let p2 = Sprite::new(0, 0, 40, 25);
        panel.add_pixel_sprite(p2, "petimg2");

        // let glt = GlTransition::new(40, 25);
        let glt = None;

        timer_register("PetView.Timer", 0.2, "pet_timer");
        timer_fire("PetView.Timer", 1);

        Self {
            panel,
            glt,
            progress: 0.0,
        }
    }
}

impl Render for PetviewRender {
    fn init<G: Model>(&mut self, ctx: &mut Context, _data: &mut G) {
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
            let mut glt = GlTransition::new(&gl, 40, 25);
            glt.set_texture(&gl, &img1, &img2);
            self.glt = Some(glt);
        }
    }

    fn handle_event<G: Model>(&mut self, ctx: &mut Context, data: &mut G, _dt: f32) {}

    fn handle_timer<G: Model>(&mut self, ctx: &mut Context, _model: &mut G, _dt: f32) {
        let sa = ctx.adapter.as_any().downcast_mut::<SdlAdapter>().unwrap();
        if event_check("PetView.Timer", "pet_timer") {
            let p1 = self.panel.get_pixel_sprite("petimg2");
            if let Some(gl) = &sa.gl {
                if let Some(glt) = &mut self.glt {
                    glt.render_frame(&gl, self.progress);
                    // // glt.render_frame(&gl, 1.0);
                    // info!("p..{} pixels...{}", self.progress, glt.pixels.len());
                    // for i in 0..4 {
                    //     info!("{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                    //         glt.pixels[i * 24 + 0], glt.pixels[i * 24 + 1], glt.pixels[i * 24 + 2], glt.pixels[i * 24 + 3], 
                    //         glt.pixels[i * 24 + 4], glt.pixels[i * 24 + 5], glt.pixels[i * 24 + 6], glt.pixels[i * 24 + 7], 
                    //         glt.pixels[i * 24 + 8], glt.pixels[i * 24 + 9], glt.pixels[i * 24 + 10], glt.pixels[i * 24 + 11], 
                    //         glt.pixels[i * 24 + 12], glt.pixels[i * 24 + 13], glt.pixels[i * 24 + 14], glt.pixels[i * 24 + 15], 
                    //         glt.pixels[i * 24 + 16], glt.pixels[i * 24 + 17], glt.pixels[i * 24 + 18], glt.pixels[i * 24 + 19], 
                    //         glt.pixels[i * 24 + 20], glt.pixels[i * 24 + 21], glt.pixels[i * 24 + 22], glt.pixels[i * 24 + 23], 
                    //     );
                    // }
                    p1.content.set_rgba_image(&glt.pixels, 40, 25);
                    self.progress += 0.03;
                    if self.progress >= 1.0 {
                        self.progress = 0.0;
                    }
                }
            }
            // asset2sprite!(p1, ctx, &format!("{}.pix", (ctx.stage / 13 % 20) + 1));
            timer_fire("PetView.Timer", 1);
        }
    }

    fn draw<G: Model>(&mut self, ctx: &mut Context, data: &mut G, dt: f32) {
        self.panel.draw(ctx).unwrap();
    }
}
