mod model;
mod render;
use std::env;
use std::error::Error;
use log::info;
use rust_pixel::game::Game;
use rust_pixel::log::init_log;
use crate::{model::TeditModel, render::TeditRender};

fn main() -> Result<(), Box<dyn Error>> {
    init_log(log::LevelFilter::Info, "log/tedit.log");

    let args: Vec<String> = env::args().collect();
    info!("Tedit(pixel.rs) start...{:?}", args);

    let escfile: &str;
    match args.len() {
        2 => {
            escfile = &args[1];
        }
        1 => {
            #[cfg(not(feature = "sdl"))]
            {
                escfile = "assets/tmp/tedit.out";
            }
            #[cfg(feature = "sdl")]
            {
                escfile = "assets/tmp/tedit.sdl";
            }
        }
        _ => {
            println!("Usage: tedit <esc file path>");
            return Ok(());
        }
    }

    let m = TeditModel::new();
    let r = TeditRender::new(escfile);
    let mut g = Game::new(m, r);

    g.init();
    g.run()?;
    g.render.panel.reset(&mut g.context);

    Ok(())
}
