//! RustPixel Image/Sprite Editor
//!
//! A tool for creating and editing pixel art, sprites, and texture files.
//! Supports both terminal-based and graphical editing modes.

pub mod model;
#[cfg(not(graphics_mode))]
pub mod render_terminal;
#[cfg(graphics_mode)]
pub mod render_graphics;

#[cfg(not(graphics_mode))]
pub use render_terminal::TeditRender;
#[cfg(graphics_mode)]
pub use render_graphics::TeditRender;

pub use model::TeditModel;

use rust_pixel::game::Game;
use rust_pixel::util::get_project_path;
use log::info;

/// Run the editor with a specific file path
pub fn run_with_file(work_dir: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize assets based on mode
    #[cfg(all(graphics_mode, not(target_arch = "wasm32")))]
    {
        let _ = rust_pixel::init_pixel_assets("pixel_edit", work_dir, false, false);
    }

    #[cfg(not(graphics_mode))]
    {
        rust_pixel::init_game_config("pixel_edit", work_dir, false, false);
    }

    let m = TeditModel::new();
    let r = TeditRender::new(file_path);
    let mut g = Game::new(m, r);
    info!("pixel_edit(rust_pixel) start...");

    g.init();
    g.run()?;
    g.render.scene.reset(&mut g.context);

    Ok(())
}

/// Run the editor with default file
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let pp = get_project_path();
    #[cfg(not(graphics_mode))]
    let default_file = "assets/tmp/tedit.txt";
    #[cfg(graphics_mode)]
    let default_file = "assets/tmp/tedit.pix";

    run_with_file(&pp, default_file)
}
