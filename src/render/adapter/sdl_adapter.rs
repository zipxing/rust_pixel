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
        gl::pixel::GlPixelRenderer, init_sym_height, init_sym_width, Adapter, AdapterBase,
        PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
    },
    buffer::Buffer,
    sprite::Sprites,
};
use log::info;
use sdl2::{
    event::Event as SEvent,
    image::LoadSurface,
    keyboard::Keycode as SKeycode,
    mouse::*,
    surface::Surface,
    video::{Window, WindowPos::Positioned},
    EventPump, Sdl,
};
use std::any::Any;
use std::time::Duration;

/// DEPRECATED: Window dragging state for custom borderless windows
///
/// This struct is no longer used as RustPixel now uses OS window decoration
/// with native window dragging. Kept for backward compatibility.
#[allow(dead_code)]
#[derive(Default)]
struct Drag {
    need: bool,
    draging: bool,
    mouse_x: i32,
    mouse_y: i32,
    dx: i32,
    dy: i32,
}

pub struct SdlAdapter {
    pub base: AdapterBase,

    // sdl object
    pub sdl_context: Sdl,
    pub sdl_window: Option<Window>,
    pub event_pump: Option<EventPump>,

    // gl object
    pub gl_context: Option<sdl2::video::GLContext>,

    // Direct OpenGL pixel renderer - no more trait objects
    pub gl_pixel_renderer: Option<GlPixelRenderer>,

    // custom cursor
    pub cursor: Option<Cursor>,

    // DEPRECATED: Window dragging state (no longer used with OS decoration)
    drag: Drag,
}

/// DEPRECATED: Border area detection for custom window borders
///
/// This enum is no longer used as RustPixel now uses OS window decoration.
/// Kept for backward compatibility.
#[allow(dead_code)]
pub enum SdlBorderArea {
    NOPE,
    CLOSE,
    TOPBAR,
    OTHER,
}

impl SdlAdapter {
    pub fn new() -> Self {
        Self {
            base: AdapterBase::new(),
            sdl_context: sdl2::init().unwrap(),
            event_pump: None,
            cursor: None,
            sdl_window: None,
            gl_context: None,
            gl_pixel_renderer: None,
            drag: Default::default(),
        }
    }

    fn set_mouse_cursor(&mut self, s: &Surface) {
        self.cursor = Some(
            Cursor::from_surface(s, 0, 0)
                .map_err(|err| format!("failed to load cursor: {}", err))
                .unwrap(),
        );
        if let Some(cursor) = &self.cursor {
            cursor.set();
        }
        self.sdl_context.mouse().show_cursor(true);
    }

    /// DEPRECATED: Check if mouse position is in custom border area
    ///
    /// This method is no longer used as RustPixel now uses OS window decoration
    /// instead of custom borders. Kept for backward compatibility.
    #[allow(dead_code)]
    fn in_border(&self, x: i32, y: i32) -> SdlBorderArea {
        let w = self.base.gr.cell_width();
        let h = self.base.gr.cell_height();
        let sw = self.base.cell_w + 2;
        if y >= 0 && y < h as i32 {
            if x >= 0 && x <= ((sw - 1) as f32 * w) as i32 {
                return SdlBorderArea::TOPBAR;
            }
            if x > ((sw - 1) as f32 * w) as i32 && x <= (sw as f32 * w) as i32 {
                return SdlBorderArea::CLOSE;
            }
        } else if x > w as i32 && x <= ((sw - 1) as f32 * w) as i32 {
            return SdlBorderArea::NOPE;
        }
        SdlBorderArea::OTHER
    }

    /// DEPRECATED: Handle custom window dragging for borderless windows
    ///
    /// This method is no longer used as RustPixel now uses OS window decoration
    /// with native window dragging. Kept for backward compatibility.
    #[allow(dead_code)]
    fn drag_window(&mut self, event: &SEvent) -> bool {
        match *event {
            SEvent::Quit { .. }
            | SEvent::KeyDown {
                keycode: Some(SKeycode::Q),
                ..
            } => return true,
            SEvent::MouseButtonDown {
                mouse_btn: sdl2::mouse::MouseButton::Left,
                x,
                y,
                ..
            } => {
                let bs = self.in_border(x, y);
                match bs {
                    SdlBorderArea::TOPBAR | SdlBorderArea::OTHER => {
                        // start dragging when mouse left click
                        self.drag.draging = true;
                        self.drag.mouse_x = x;
                        self.drag.mouse_y = y;
                    }
                    SdlBorderArea::CLOSE => {
                        return true;
                    }
                    _ => {}
                }
            }
            SEvent::MouseButtonUp {
                mouse_btn: sdl2::mouse::MouseButton::Left,
                ..
            } => {
                // stop dragging when mouse left button is release
                self.drag.draging = false;
            }
            SEvent::MouseMotion { x, y, .. } if self.drag.draging => {
                self.drag.need = true;
                // dragging window when mouse left button is hold and moving
                self.drag.dx = x - self.drag.mouse_x;
                self.drag.dy = y - self.drag.mouse_y;
            }
            _ => {}
        }
        false
    }
}

impl Adapter for SdlAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        // load texture file using global GAME_CONFIG
        // 使用全局 GAME_CONFIG 加载纹理文件
        let project_path = &crate::get_game_config().project_path;
        let texture_path = format!(
            "{}{}{}",
            project_path,
            std::path::MAIN_SEPARATOR,
            PIXEL_TEXTURE_FILE
        );
        let teximg = image::open(&texture_path)
            .map_err(|e| e.to_string())
            .expect(&format!("open file:{:?}", &texture_path))
            .to_rgba8();
        let texwidth = teximg.width();
        let texheight = teximg.height();
        PIXEL_SYM_WIDTH
            .set(init_sym_width(texwidth))
            .expect("lazylock init");
        PIXEL_SYM_HEIGHT
            .set(init_sym_height(texheight))
            .expect("lazylock init");

        info!("gl_pixel load texture...{}", texture_path);
        info!(
            "symbol_w={} symbol_h={} (Sprite: 8x8, TUI: 8x16)",
            PIXEL_SYM_WIDTH.get().expect("lazylock init"),
            PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
        );

        // Symbol map is lazy-loaded from app's assets directory via get_symbol_map()
        // when first accessed (after init_game_config() sets project_path)

        self.set_size(w, h).set_title(title);
        self.base.gr.set_ratiox(rx);
        self.base.gr.set_ratioy(ry);
        self.base
            .gr
            .set_pixel_size(self.base.cell_w, self.base.cell_h);
        info!(
            "pixel_w={} pixel_h={}",
            self.base.gr.pixel_w, self.base.gr.pixel_h
        );

        // init video subsystem...
        let video_subsystem = self.sdl_context.video().unwrap();

        // Set OpenGL attributes
        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);

        let window = video_subsystem
            .window(&self.base.title, self.base.gr.pixel_w, self.base.gr.pixel_h)
            .opengl()
            .position_centered()
            // Use OS window decoration (title bar, etc.) instead of borderless
            // .borderless()
            // .fullscreen()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let gl_context = window.gl_create_context().unwrap();
        self.gl_context = Some(gl_context);

        // Create the OpenGL context using glow
        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                video_subsystem.gl_get_proc_address(s) as *const _
            })
        };

        // Create direct OpenGL pixel renderer
        let gl_pixel_renderer = GlPixelRenderer::new(
            gl,
            "#version 330 core",
            self.base.gr.pixel_w as i32,
            self.base.gr.pixel_h as i32,
            texwidth as i32,
            texheight as i32,
            &teximg,
        );

        // Store the direct renderer - no more trait objects!
        self.gl_pixel_renderer = Some(gl_pixel_renderer);
        self.sdl_window = Some(window);

        info!("Window & gl init ok...");

        // custom mouse cursor image using global GAME_CONFIG
        // 使用全局 GAME_CONFIG 加载自定义鼠标光标图像
        let cursor_path = format!(
            "{}{}{}",
            project_path,
            std::path::MAIN_SEPARATOR,
            "assets/pix/cursor.png"
        );
        let surface = Surface::from_file(&cursor_path)
            .map_err(|err| format!("failed to load cursor image: {}", err))
            .unwrap();
        self.set_mouse_cursor(&surface);

        // init event_pump
        self.event_pump = Some(self.sdl_context.event_pump().unwrap());
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn poll_event(&mut self, timeout: Duration, es: &mut Vec<Event>) -> bool {
        let mut ses: Vec<SEvent> = vec![];
        if let Some(ref mut ep) = self.event_pump {
            for event in ep.poll_iter() {
                ses.push(event.clone());
                // convert sdl events to pixel events, providing a unified processing interfaces
                if let Some(et) =
                    input_events_from_sdl(&event, self.base.gr.ratio_x, self.base.gr.ratio_y, self.base.gr.use_tui_height)
                {
                    if !self.drag.draging {
                        es.push(et);
                    }
                }
            }
            for event in ses {
                // Using OS window decoration, no need for custom drag/close handling
                // Just check for quit events
                match event {
                    SEvent::Quit { .. } => return true,
                    SEvent::KeyDown {
                        keycode: Some(SKeycode::Q),
                        ..
                    } => return true,
                    _ => {}
                }
            }
            ::std::thread::sleep(timeout);
        }
        false
    }

    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        _p: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        // No custom window dragging needed (using OS window decoration)
        // sdl_move_win(
        //     &mut self.drag.need,
        //     self.sdl_window.as_mut().unwrap(),
        //     self.drag.dx,
        //     self.drag.dy,
        // );

        self.draw_all_graph(current_buffer, _p, pixel_sprites, stage);
        self.post_draw();

        Ok(())
    }

    fn post_draw(&mut self) {
        // swap window for display
        self.sdl_window.as_ref().unwrap().gl_swap_window();
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

    /// Direct implementation of draw_render_buffer_to_texture for SDL
    fn draw_render_buffer_to_texture(
        &mut self,
        rbuf: &[crate::render::adapter::RenderCell],
        rtidx: usize,
        debug: bool,
    ) where
        Self: Sized,
    {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            let ratio_x = self.base.gr.ratio_x;
            let ratio_y = self.base.gr.ratio_y;

            // Use direct method call - no more trait objects!
            if let Err(e) = gl_pixel_renderer
                .render_buffer_to_texture_self_contained(rbuf, rtidx, debug, ratio_x, ratio_y)
            {
                eprintln!("SdlAdapter: render_buffer_to_texture error: {}", e);
            }
        } else {
            eprintln!("SdlAdapter: gl_pixel_renderer not initialized");
        }
    }

    /// Direct implementation of draw_render_textures_to_screen for SDL
    fn draw_render_textures_to_screen(&mut self)
    where
        Self: Sized,
    {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            let ratio_x = self.base.gr.ratio_x;
            let ratio_y = self.base.gr.ratio_y;

            // Bind to screen framebuffer and render textures
            gl_pixel_renderer.bind_screen_with_viewport(
                self.base.gr.pixel_w as i32,
                self.base.gr.pixel_h as i32,
            );

            // Use direct method call - no more trait objects!
            if let Err(e) = gl_pixel_renderer.render_textures_to_screen_no_bind(ratio_x, ratio_y) {
                eprintln!("SdlAdapter: render_textures_to_screen error: {}", e);
            }
        } else {
            eprintln!("SdlAdapter: gl_pixel_renderer not initialized for texture rendering");
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    /// SDL adapter implementation of render texture visibility control
    fn set_render_texture_visible(&mut self, texture_index: usize, visible: bool) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer
                .get_gl_pixel_mut()
                .set_render_texture_hidden(texture_index, !visible);
        }
    }

    /// SDL adapter implementation of simple transition rendering
    fn render_simple_transition(&mut self, target_texture: usize) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.render_normal_transition(target_texture);
        }
    }

    /// SDL adapter implementation of advanced transition rendering
    fn render_advanced_transition(
        &mut self,
        target_texture: usize,
        effect_type: usize,
        progress: f32,
    ) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.render_gl_transition(target_texture, effect_type, progress);
        }
    }

    /// SDL adapter implementation of buffer transition setup
    fn setup_buffer_transition(&mut self, target_texture: usize) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.setup_transbuf_rendering(target_texture);
        }
    }
}

pub fn sdl_move_win(drag_need: &mut bool, win: &mut Window, dx: i32, dy: i32) {
    // dragging window, set the correct position of a window
    if *drag_need {
        let (win_x, win_y) = win.position();
        win.set_position(Positioned(win_x + dx), Positioned(win_y + dy));
        *drag_need = false;
    }
}

// avoid repeated code by defining a marco
macro_rules! sdl_event {
    ($ek:expr, $x:expr, $y:expr, $($btn:expr)* ) => {
        Some(MouseEvent {
            kind: $ek$(($btn))*,
            column: $x as u16,
            row: $y as u16,
            modifiers: KeyModifiers::NONE,
        })
    };
}

/// Convert SDL input events to RustPixel event for unified event processing
///
/// For keyboard and mouse event handling examples, refer to the handle_input method in game/unblock/model.rs
///
/// # Parameters
/// - `e`: SDL event reference
/// - `adjx`: X-axis adjustment factor (for high DPI displays)
/// - `adjy`: Y-axis adjustment factor (for high DPI displays)
/// - `use_tui_height`: If true, uses TUI character height (32px) for mouse coordinate conversion;
///                     if false, uses Sprite character height (16px)
///
/// # Mouse Coordinate Conversion
/// Mouse pixel coordinates are converted to character cell coordinates.
/// The conversion accounts for TUI double-height mode to ensure accurate click detection.
pub fn input_events_from_sdl(e: &SEvent, adjx: f32, adjy: f32, use_tui_height: bool) -> Option<Event> {
    let sym_width = PIXEL_SYM_WIDTH.get().expect("lazylock init");
    let sym_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init");
    let mut mcte: Option<MouseEvent> = None;
    match e {
        SEvent::KeyDown { keycode, .. } => {
            let kc = match keycode {
                Some(SKeycode::Space) => ' ',
                Some(SKeycode::A) => 'a',
                Some(SKeycode::B) => 'b',
                Some(SKeycode::C) => 'c',
                Some(SKeycode::D) => 'd',
                Some(SKeycode::E) => 'e',
                Some(SKeycode::F) => 'f',
                Some(SKeycode::G) => 'g',
                Some(SKeycode::H) => 'h',
                Some(SKeycode::I) => 'i',
                Some(SKeycode::J) => 'j',
                Some(SKeycode::K) => 'k',
                Some(SKeycode::L) => 'l',
                Some(SKeycode::M) => 'm',
                Some(SKeycode::N) => 'n',
                Some(SKeycode::O) => 'o',
                Some(SKeycode::P) => 'p',
                Some(SKeycode::Q) => 'q',
                Some(SKeycode::R) => 'r',
                Some(SKeycode::S) => 's',
                Some(SKeycode::T) => 't',
                Some(SKeycode::U) => 'u',
                Some(SKeycode::V) => 'v',
                Some(SKeycode::W) => 'w',
                Some(SKeycode::X) => 'x',
                Some(SKeycode::Y) => 'y',
                Some(SKeycode::Z) => 'z',
                _ => {
                    return None;
                }
            };
            let cte = KeyEvent::new(KeyCode::Char(kc), KeyModifiers::NONE);
            return Some(Event::Key(cte));
        }
        SEvent::MouseButtonUp { x, y, .. } => {
            mcte = sdl_event!(Up, *x, *y, Left);
        }
        SEvent::MouseButtonDown { x, y, .. } => {
            mcte = sdl_event!(Down, *x, *y, Left);
        }
        SEvent::MouseMotion {
            x, y, mousestate, ..
        } => {
            if mousestate.left() {
                mcte = sdl_event!(Drag, *x, *y, Left);
            } else {
                mcte = sdl_event!(Moved, *x, *y,);
            }
        }
        _ => {}
    }
    if let Some(mut mc) = mcte {
        // Convert pixel coordinates to cell coordinates
        // No border offset needed (using OS window decoration)
        // Account for TUI mode: double height (32px) vs sprite height (16px)
        let cell_height = if use_tui_height {
            *sym_height * 2.0
        } else {
            *sym_height
        };
        mc.column /= (sym_width / adjx) as u16;
        mc.row /= (cell_height / adjy) as u16;
        return Some(Event::Mouse(mc));
    }
    None
}
