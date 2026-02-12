// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Context encapsulates several public variables
//! including stage, state, input events, etc.
//! For simplicity, state is set to u8 type, you can create your own states using enums in your games.
//! Context also integrates an RNG for user's convenience.
//! A render adapter is also provided
//! to make it compatible with web, WGPU, or terminal modes.
//! Finally, an asset_manager is included as well.
//!
//! Note: `game_name` and `project_path` are now stored in the global `GAME_CONFIG`.
//! Use `rust_pixel::get_game_config()` to access them from anywhere.

use crate::{asset::AssetManager, event::Event, render::adapter::Adapter, util::Rand};

#[cfg(cross_backend)]
use crate::render::adapter::cross_adapter::CrosstermAdapter;

#[cfg(wgpu_backend)]
use crate::render::adapter::winit_wgpu_adapter::WinitWgpuAdapter;

#[cfg(wasm)]
use crate::render::adapter::web_adapter::WebAdapter;

pub struct Context {
    pub stage: u32,
    pub state: u8,
    pub rand: Rand,
    pub asset_manager: AssetManager,
    pub input_events: Vec<Event>,
    pub adapter: Box<dyn Adapter>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            stage: 0,
            state: 0,
            rand: Rand::new(),
            asset_manager: AssetManager::new(),
            input_events: vec![],
            #[cfg(wasm)]
            adapter: Box::new(WebAdapter::new()),
            #[cfg(wgpu_backend)]
            adapter: Box::new(WinitWgpuAdapter::new()),
            #[cfg(cross_backend)]
            adapter: Box::new(CrosstermAdapter::new()),
        }
    }

    pub fn cell_width(&mut self) -> f32 {
        #[cfg(graphics_mode)]
        let ret = self.adapter.get_base().gr.cell_width();
        #[cfg(not(graphics_mode))]
        let ret = 0.0f32;
        ret
    }

    pub fn cell_height(&mut self) -> f32 {
        #[cfg(graphics_mode)]
        let ret = self.adapter.get_base().gr.cell_height();
        #[cfg(not(graphics_mode))]
        let ret = 0.0f32;
        ret
    }

    // ========================================================================
    // Viewport Helper Methods (Graphics Mode Only)
    // ========================================================================
    //
    // These methods simplify viewport creation by automatically fetching
    // all required parameters (sym_w, sym_h, ratio, canvas_size) from Context.
    //
    // Usage:
    //   let vp = ctx.centered_viewport(40, 25);
    //   ctx.adapter.present(&[
    //       RtComposite::fullscreen(2),
    //       RtComposite::with_viewport(3, vp),
    //   ]);
    //
    // Or with the shorthand:
    //   ctx.adapter.present(&[
    //       RtComposite::fullscreen(2),
    //       ctx.centered_rt(3, 40, 25),
    //   ]);

    /// Calculate centered viewport from cell dimensions
    ///
    /// Returns an ARect positioned to center the given cell area on the canvas.
    /// This is a convenience method that fetches all rendering parameters automatically.
    ///
    /// # Parameters
    /// - `cell_w`, `cell_h`: Size in cells (e.g., 40x25)
    ///
    /// # Example
    /// ```ignore
    /// let vp = ctx.centered_viewport(40, 25);
    /// ctx.adapter.present(&[
    ///     RtComposite::fullscreen(2),
    ///     RtComposite::with_viewport(3, vp),
    /// ]);
    /// ```
    #[cfg(graphics_mode)]
    pub fn centered_viewport(&mut self, cell_w: u16, cell_h: u16) -> crate::util::ARect {
        use crate::render::adapter::{PIXEL_SYM_WIDTH, PIXEL_SYM_HEIGHT};

        let sym_w = *PIXEL_SYM_WIDTH.get().expect("PIXEL_SYM_WIDTH not initialized");
        let sym_h = *PIXEL_SYM_HEIGHT.get().expect("PIXEL_SYM_HEIGHT not initialized");
        let rx = self.adapter.get_base().gr.ratio_x;
        let ry = self.adapter.get_base().gr.ratio_y;
        let canvas_w = self.adapter.get_base().gr.pixel_w as u32;
        let canvas_h = self.adapter.get_base().gr.pixel_h as u32;

        let vp_w = (cell_w as f32 * sym_w / rx) as u32;
        let vp_h = (cell_h as f32 * sym_h / ry) as u32;
        let x = ((canvas_w.saturating_sub(vp_w)) / 2) as i32;
        let y = ((canvas_h.saturating_sub(vp_h)) / 2) as i32;

        crate::util::ARect { x, y, w: vp_w, h: vp_h }
    }

    /// Create a centered RtComposite from cell dimensions
    ///
    /// Combines RT index with centered viewport calculation in one call.
    /// This is the most convenient method for creating centered RT presentations.
    ///
    /// # Parameters
    /// - `rt`: Render texture index (0-3)
    /// - `cell_w`, `cell_h`: Size in cells (e.g., 40x25)
    ///
    /// # Example
    /// ```ignore
    /// ctx.adapter.present(&[
    ///     RtComposite::fullscreen(2),
    ///     ctx.centered_rt(3, 40, 25),
    /// ]);
    /// ```
    #[cfg(graphics_mode)]
    pub fn centered_rt(&mut self, rt: usize, cell_w: u16, cell_h: u16) -> crate::render::adapter::RtComposite {
        use crate::render::adapter::RtComposite;

        let vp = self.centered_viewport(cell_w, cell_h);
        RtComposite::with_viewport(rt, vp)
    }

    /// Get current canvas size in pixels
    ///
    /// Returns (width, height) of the rendering canvas.
    #[cfg(graphics_mode)]
    pub fn canvas_size(&mut self) -> (u32, u32) {
        let gr = &self.adapter.get_base().gr;
        (gr.pixel_w as u32, gr.pixel_h as u32)
    }

    /// Get current DPI scaling ratios
    ///
    /// Returns (ratio_x, ratio_y) for high-DPI display support.
    #[cfg(graphics_mode)]
    pub fn ratio(&mut self) -> (f32, f32) {
        let gr = &self.adapter.get_base().gr;
        (gr.ratio_x, gr.ratio_y)
    }
}
