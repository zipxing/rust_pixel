mod model;
mod render;
use std::env;
use std::error::Error;
use log::info;
use rust_pixel::game::Game;
use crate::{model::TeditModel, render::TeditRender};

fn print_edit_usage() {
    eprintln!("RustPixel Image/Sprite Editor");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    edit <WORK_DIR> [FILE_PATH]");
    eprintln!("    cargo pixel edit <MODE> <WORK_DIR> [FILE_PATH]");
    eprintln!("    cargo pixel e <MODE> <WORK_DIR> [FILE_PATH]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <WORK_DIR>   Working directory (usually '.')");
    eprintln!("    [FILE_PATH]  File path to edit (optional, uses default if not specified)");
    eprintln!();
    eprintln!("MODES (when used via cargo-pixel):");
    eprintln!("    t, term    Terminal mode (text-based editing)");
    eprintln!("    s, sdl     SDL2 mode (graphics with OpenGL)");
    eprintln!("    w, web     Web mode (browser)");
    eprintln!("    g, winit   Winit mode (native window with OpenGL)");
    eprintln!("    wg, wgpu   WGPU mode (native window with modern GPU API)");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Launches the RustPixel image and sprite editor for creating and editing");
    eprintln!("    pixel art, sprites, and texture files. Supports both terminal-based and");
    eprintln!("    graphical editing modes depending on the build features.");
    eprintln!();
    eprintln!("DEFAULT FILES:");
    eprintln!("    Terminal mode: assets/tmp/tedit.txt");
    eprintln!("    Graphics mode: assets/tmp/tedit.pix");
    eprintln!();
    eprintln!("FEATURES:");
    eprintln!("    - Multi-format support (TXT, PIX files)");
    eprintln!("    - Real-time pixel art editing");
    eprintln!("    - Cross-platform rendering");
    eprintln!("    - Asset path management");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    edit .                               # Edit default file in current directory");
    eprintln!("    edit . my_image.pix                  # Edit specific PIX file");
    eprintln!("    cargo pixel edit t .                 # Terminal mode via cargo-pixel");
    eprintln!("    cargo pixel edit wg . sprite.pix     # WGPU mode with specific file");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    When used via cargo-pixel, equivalent to: cargo pixel r edit <MODE> -r <WORK_DIR> [FILE_PATH]");
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    // Check for help argument
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_edit_usage();
        return Ok(());
    }

    let escfile: &str;
    let apath: &str;
    match args.len() {
        3 => {
            apath = &args[1];
            escfile = &args[2];
        }
        2 => {
            apath = &args[1];
            #[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32")))]
            {
                escfile = "assets/tmp/tedit.txt";
            }
            #[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
            {
                escfile = "assets/tmp/tedit.pix";
            }
        }
        _ => {
            print_edit_usage();
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
