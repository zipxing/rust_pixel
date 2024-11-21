mod model;
mod render;
use std::env;
use std::error::Error;
use log::info;
use rust_pixel::game::Game;
use crate::{model::TeditModel, render::TeditRender};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let escfile: &str;
    let apath: &str;
    match args.len() {
        3 => {
            apath = &args[1];
            escfile = &args[2];
        }
        2 => {
            apath = &args[1];
            #[cfg(not(feature = "sdl"))]
            {
                escfile = "assets/tmp/tedit.txt";
            }
            #[cfg(feature = "sdl")]
            {
                escfile = "assets/tmp/tedit.pix";
            }
        }
        _ => {
            println!("Usage: pixel_edit asset_file_path <image_file_path>");
            return Ok(());
        }
    }

    let m = TeditModel::new();
    let r = TeditRender::new(escfile);
    let mut g = Game::new(m, r, "pixel_edit", apath);
    info!("pixel_edit(rust_pixel) start...{:?}", args);

    g.init();
    g.run()?;
    g.render.panel.reset(&mut g.context);

    Ok(())
}
