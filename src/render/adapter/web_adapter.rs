// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Implements an Adapter trait. Moreover, all SDL related processing is handled here.
//! Includes resizing of height and width, init settings.
//! Use opengl and glow mod for rendering.
use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
use crate::render::{
    adapter::{
        gl::pixel::GlPixelRenderer,
        Adapter, AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
    },
    buffer::Buffer,
    sprite::Layer,
};
use log::info;
use std::any::Any;
use std::time::Duration;

pub struct WebAdapter {
    pub base: AdapterBase,
    
    // Direct OpenGL pixel renderer - no more trait objects
    pub gl_pixel_renderer: Option<GlPixelRenderer>,
    
    // OpenGL context for web (needed for init_glpix)
    pub gl: Option<glow::Context>,
}

impl WebAdapter {
    pub fn new() -> Self {
        Self {
            base: AdapterBase::new(),
            gl_pixel_renderer: None,
            gl: None,
        }
    }

    /// Initialize WebGL pixel renderer using pre-cached texture data
    ///
    /// This method uses the texture data that was cached during wasm_init_pixel_assets().
    /// Call this instead of upload_imgdata() when using the new unified asset loading.
    pub fn init_glpix_from_cache(&mut self) {
        let tex_data = crate::get_pixel_texture_data();

        self.base.gr.set_pixel_size(self.base.cell_w, self.base.cell_h);

        // Create direct OpenGL pixel renderer using cached texture data
        if let Some(gl) = self.gl.take() {
            let gl_pixel_renderer = GlPixelRenderer::new(
                gl,
                "#version 300 es",
                self.base.gr.pixel_w as i32,
                self.base.gr.pixel_h as i32,
                tex_data.width as i32,
                tex_data.height as i32,
                &tex_data.data,
            );
            self.gl_pixel_renderer = Some(gl_pixel_renderer);
        }
        info!("web glpix init from cache ok: {}x{}", tex_data.width, tex_data.height);
    }
}

impl Adapter for WebAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String) {
        self.set_size(w, h).set_title(s);
        self.base.gr.set_ratiox(rx);
        self.base.gr.set_ratioy(ry);

        use wasm_bindgen::JsCast;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let webgl2_context = canvas
            .get_context("webgl2")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .unwrap();
        let gl = glow::Context::from_webgl2_context(webgl2_context);

        // Store the OpenGL context
        self.gl = Some(gl);
        info!("Window & gl init ok...");
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn poll_event(&mut self, _timeout: Duration, _es: &mut Vec<Event>) -> bool {
        false
    }



    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        _p: &Buffer,
        pixel_sprites: &mut Vec<Layer>,
        stage: u32,
    ) -> Result<(), String> {
        self.draw_all_graph(current_buffer, _p, pixel_sprites, stage);
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok((0, 0))
    }

    /// Render buffer to RT (Web implementation)
    fn rbuf2rt(&mut self, rbuf: &[crate::render::adapter::RenderCell], rtidx: usize, debug: bool) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            let ratio_x = self.base.gr.ratio_x;
            let ratio_y = self.base.gr.ratio_y;

            if let Err(e) = gl_pixel_renderer.render_buffer_to_texture_self_contained(rbuf, rtidx, debug, ratio_x, ratio_y) {
                eprintln!("WebAdapter: rbuf2rt error: {}", e);
            }
        } else {
            eprintln!("WebAdapter: gl_pixel_renderer not initialized");
        }
    }

    fn post_draw(&mut self) {
        // For WebGL, buffer swapping is handled automatically by the browser
        // No explicit action needed here
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    /// Set RT visibility (Web implementation)
    fn set_rt_visible(&mut self, texture_index: usize, visible: bool) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.get_gl_pixel_mut().set_render_texture_hidden(texture_index, !visible);
        }
    }

    /// Blend two RTs with transition effect (Web implementation)
    fn blend_rts(&mut self, src1: usize, src2: usize, target: usize, effect: usize, progress: f32) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.render_gl_transition(src1, src2, target, effect, progress);
        }
    }

    /// Setup buffer transition (Web implementation)
    fn setup_buffer_transition(&mut self, target_texture: usize) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.setup_transbuf_rendering(target_texture);
        }
    }

    /// Copy one RT to another (Web implementation)
    fn copy_rt(&mut self, src_index: usize, dst_index: usize) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.copy_rt(src_index, dst_index);
        }
    }

    /// Present render textures to screen using RtComposite chain
    ///
    /// This is the new unified API for presenting RTs to the screen.
    /// Each RtComposite specifies which RT to draw, viewport, blend mode, and alpha.
    fn present(&mut self, composites: &[crate::render::adapter::RtComposite]) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            // Bind screen framebuffer
            gl_pixel_renderer.gl_pixel.bind_screen(&gl_pixel_renderer.gl);

            // Clear screen
            use glow::HasContext;
            let gl = gl_pixel_renderer.get_gl();
            unsafe {
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }

            // Use the new present() method
            gl_pixel_renderer.present(composites);
        } else {
            // Web console logging for debugging
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&"WebAdapter: gl_pixel_renderer not initialized for present".into());
        }

        // For WebGL, buffer swapping is handled automatically by the browser
        self.post_draw();
    }

    /// Present with default settings (RT2 fullscreen, RT3 with game area viewport)
    ///
    /// Uses the original working logic with float precision.
    /// Web needs explicit viewport setup using pixel_w/pixel_h.
    fn present_default(&mut self) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            let rx = self.base.gr.ratio_x;
            let ry = self.base.gr.ratio_y;
            let (canvas_w, canvas_h) = gl_pixel_renderer.get_gl_pixel().get_canvas_size();
            info!(
                "WebAdapter::present_default - pixel_w: {}, pixel_h: {}, canvas_w: {}, canvas_h: {}, ratio: ({}, {})",
                self.base.gr.pixel_w, self.base.gr.pixel_h, canvas_w, canvas_h, rx, ry
            );
            // Web mode: use pixel_w/pixel_h for viewport (same as before)
            gl_pixel_renderer.present_default_with_physical_size(
                rx,
                ry,
                Some((self.base.gr.pixel_w, self.base.gr.pixel_h)),
            );
        }
    }
}

macro_rules! web_event {
    ($ek:expr, $ei:expr, $($btn:expr)* ) => {
        Some(MouseEvent {
            kind: $ek$(($btn))*,
            column: $ei.5 as u16,
            row: $ei.6 as u16,
            modifiers: KeyModifiers::NONE,
        })
    };
}

/// Convert Web I/O events to RustPixel event for unified event processing
///
/// For keyboard and mouse event handling examples, refer to the handle_input method in game/unblock/model.rs
///
/// # Parameters
/// - `t`: Event type identifier
/// - `e`: Web event reference (KeyboardEvent, MouseEvent, etc.)
/// - `pixel_h`: Window pixel height
/// - `ratiox`: X-axis scaling ratio
/// - `ratioy`: Y-axis scaling ratio
/// - `use_tui_height`: If true, uses TUI character height (32px) for mouse coordinate conversion;
///                     if false, uses Sprite character height (16px)
///
/// # Mouse Coordinate Conversion
/// Mouse pixel coordinates are converted to character cell coordinates.
/// The conversion accounts for TUI double-height mode to ensure accurate click detection.
pub fn input_events_from_web(t: u8, e: web_sys::Event, pixel_h: u32, ratiox: f32, ratioy: f32, use_tui_height: bool) -> Option<Event> {
    let sym_width = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as f32;
    let sym_height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as f32;
    let mut mcte: Option<MouseEvent> = None;

    if let Some(key_e) = wasm_bindgen::JsCast::dyn_ref::<web_sys::KeyboardEvent>(&e) {
        assert!(t == 0);
        let key = key_e.key();
        let key_code = match key.as_str() {
            // Arrow keys
            "ArrowLeft" => Some(KeyCode::Left),
            "ArrowRight" => Some(KeyCode::Right),
            "ArrowUp" => Some(KeyCode::Up),
            "ArrowDown" => Some(KeyCode::Down),
            // Navigation keys
            "PageUp" => Some(KeyCode::PageUp),
            "PageDown" => Some(KeyCode::PageDown),
            "Home" => Some(KeyCode::Home),
            "End" => Some(KeyCode::End),
            "Escape" => Some(KeyCode::Esc),
            "Enter" => Some(KeyCode::Enter),
            "Backspace" => Some(KeyCode::Backspace),
            "Tab" => Some(KeyCode::Tab),
            " " => Some(KeyCode::Char(' ')),
            // Single character keys (letters, digits, symbols)
            s if s.len() == 1 => {
                let ch = s.chars().next().unwrap();
                Some(KeyCode::Char(ch))
            }
            _ => None,
        };
        if let Some(kc) = key_code {
            let cte = KeyEvent::new(kc, KeyModifiers::NONE);
            return Some(Event::Key(cte));
        }
        return None;
    }

    if let Some(mouse_e) = wasm_bindgen::JsCast::dyn_ref::<web_sys::MouseEvent>(&e) {
        let medat = (
            mouse_e.buttons(),
            mouse_e.screen_x(),
            mouse_e.screen_y(),
            mouse_e.client_x(),
            mouse_e.client_y(),
            mouse_e.x(),
            mouse_e.y(),
        );
        match t {
            1 => {
                mcte = web_event!(Up, medat, Left);
            }
            2 => {
                mcte = web_event!(Down, medat, Left);
            }
            3 => {
                if medat.0 == 1 {
                    mcte = web_event!(Drag, medat, Left);
                } else {
                    mcte = web_event!(Moved, medat,);
                }
            }
            _ => {}
        }
    }
    if let Some(mut mc) = mcte {
        // Convert pixel coordinates to cell coordinates
        // No border offset needed (using OS window decoration)
        // Account for TUI mode: double height (32px) vs sprite height (16px)
        let cell_height = if use_tui_height {
            sym_height * 2.0
        } else {
            sym_height
        };
        mc.column /= (sym_width / ratiox) as u16;
        // adjust row by upper space
        mc.row -= 800 - pixel_h as u16;
        mc.row /= (cell_height / ratioy) as u16;
        return Some(Event::Mouse(mc));
    }
    None
} 
