// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements an Adapter trait using winit for window management and glow for OpenGL rendering.
//! This replaces the SDL2 implementation while maintaining the same functionality.

use crate::event::Event;
use crate::render::{
    adapter::{
        gl::pixel::GlPixel, init_sym_height, init_sym_width, Adapter, AdapterBase,
        PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
    },
    buffer::Buffer,
    sprite::Sprites,
};

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, Version},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use log::info;
use raw_window_handle::HasWindowHandle;
use std::any::Any;
use std::time::Duration;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
pub use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Cursor, CustomCursor, Window},
};

// Window dragging support structures (similar to SDL)
#[derive(Default)]
struct Drag {
    need: bool,
    draging: bool,
    mouse_x: f64,
    mouse_y: f64,
    dx: f64,
    dy: f64,
}

pub enum WinitBorderArea {
    NOPE,
    CLOSE,
    TOPBAR,
    OTHER,
}

pub struct WinitAdapter {
    pub base: AdapterBase,

    // winit objects
    pub window: Option<Window>,
    pub event_loop: Option<EventLoop<()>>,

    // glutin objects for OpenGL context
    pub gl_display: Option<glutin::display::Display>,
    pub gl_context: Option<glutin::context::PossiblyCurrentContext>,
    pub gl_surface: Option<Surface<WindowSurface>>,

    pub should_exit: bool,

    // Event handling - for pump events mode
    pub app_handler: Option<WinitAppHandler>,

    // custom cursor
    pub custom_cursor: Option<CustomCursor>,

    // data for dragging the window
    drag: Drag,
}

// ApplicationHandler for winit pump events
pub struct WinitAppHandler {
    pub pending_events: Vec<Event>,
    pub cursor_position: (f64, f64),
    pub ratio_x: f32,
    pub ratio_y: f32,
    pub should_exit: bool,

    // Reference to adapter for drag handling
    pub adapter_ref: *mut WinitAdapter,
}

impl ApplicationHandler for WinitAppHandler {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Window should already be created
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.should_exit = true;
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Handle Q key for exit (similar to SDL version)
                if key_event.state == winit::event::ElementState::Pressed {
                    if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key {
                        if keycode == winit::keyboard::KeyCode::KeyQ {
                            self.should_exit = true;
                            event_loop.exit();
                            return;
                        }
                    }
                }

                // Convert keyboard event to pixel event
                let winit_event = WinitEvent::WindowEvent {
                    window_id: _window_id,
                    event: WindowEvent::KeyboardInput {
                        device_id: winit::event::DeviceId::dummy(),
                        event: key_event,
                        is_synthetic: false,
                    },
                };
                if let Some(pixel_event) = input_events_from_winit(
                    &winit_event,
                    self.ratio_x,
                    self.ratio_y,
                    &mut self.cursor_position,
                ) {
                    self.pending_events.push(pixel_event);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Convert physical coordinates to logical coordinates for Retina displays
                unsafe {
                    let adapter = &*self.adapter_ref;
                    if let Some(window) = &adapter.window {
                        let scale_factor = window.scale_factor();
                        self.cursor_position = (position.x / scale_factor, position.y / scale_factor);
                    } else {
                        self.cursor_position = (position.x, position.y);
                    }
                }

                // Handle window dragging
                unsafe {
                    let adapter = &mut *self.adapter_ref;
                    if adapter.drag.draging {
                        adapter.drag.need = true;
                        adapter.drag.dx = position.x - adapter.drag.mouse_x;
                        adapter.drag.dy = position.y - adapter.drag.mouse_y;
                    }
                }

                // Convert to pixel event only if not dragging
                // Use logical position for consistent coordinate system
                let logical_position = winit::dpi::PhysicalPosition::new(
                    self.cursor_position.0,
                    self.cursor_position.1,
                );
                let winit_event = WinitEvent::WindowEvent {
                    window_id: _window_id,
                    event: WindowEvent::CursorMoved {
                        device_id: winit::event::DeviceId::dummy(),
                        position: logical_position,
                    },
                };

                unsafe {
                    let adapter = &*self.adapter_ref;
                    if !adapter.drag.draging {
                        if let Some(pixel_event) = input_events_from_winit(
                            &winit_event,
                            self.ratio_x,
                            self.ratio_y,
                            &mut self.cursor_position,
                        ) {
                            self.pending_events.push(pixel_event);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match (state, button) {
                    (winit::event::ElementState::Pressed, winit::event::MouseButton::Left) => {
                        unsafe {
                            let adapter = &mut *self.adapter_ref;
                            let bs =
                                adapter.in_border(self.cursor_position.0, self.cursor_position.1);
                            match bs {
                                WinitBorderArea::TOPBAR | WinitBorderArea::OTHER => {
                                    // start dragging when mouse left click on border
                                    adapter.drag.draging = true;
                                    adapter.drag.mouse_x = self.cursor_position.0;
                                    adapter.drag.mouse_y = self.cursor_position.1;
                                }
                                WinitBorderArea::CLOSE => {
                                    self.should_exit = true;
                                    event_loop.exit();
                                }
                                _ => {
                                    // Not dragging, pass event to game
                                    let winit_event = WinitEvent::WindowEvent {
                                        window_id: _window_id,
                                        event: WindowEvent::MouseInput {
                                            device_id: winit::event::DeviceId::dummy(),
                                            state,
                                            button,
                                        },
                                    };
                                    if let Some(pixel_event) = input_events_from_winit(
                                        &winit_event,
                                        self.ratio_x,
                                        self.ratio_y,
                                        &mut self.cursor_position,
                                    ) {
                                        self.pending_events.push(pixel_event);
                                    }
                                }
                            }
                        }
                    }
                    (winit::event::ElementState::Released, winit::event::MouseButton::Left) => {
                        unsafe {
                            let adapter = &mut *self.adapter_ref;
                            let was_dragging = adapter.drag.draging;
                            adapter.drag.draging = false;

                            // Only pass mouse release to game if we weren't dragging
                            if !was_dragging {
                                let winit_event = WinitEvent::WindowEvent {
                                    window_id: _window_id,
                                    event: WindowEvent::MouseInput {
                                        device_id: winit::event::DeviceId::dummy(),
                                        state,
                                        button,
                                    },
                                };
                                if let Some(pixel_event) = input_events_from_winit(
                                    &winit_event,
                                    self.ratio_x,
                                    self.ratio_y,
                                    &mut self.cursor_position,
                                ) {
                                    self.pending_events.push(pixel_event);
                                }
                            }
                        }
                    }
                    _ => {
                        // Convert other mouse inputs
                        let winit_event = WinitEvent::WindowEvent {
                            window_id: _window_id,
                            event: WindowEvent::MouseInput {
                                device_id: winit::event::DeviceId::dummy(),
                                state,
                                button,
                            },
                        };
                        if let Some(pixel_event) = input_events_from_winit(
                            &winit_event,
                            self.ratio_x,
                            self.ratio_y,
                            &mut self.cursor_position,
                        ) {
                            self.pending_events.push(pixel_event);
                        }
                    }
                }
            }
            _ => {
                // Convert other winit events to RustPixel events
                let winit_event = WinitEvent::WindowEvent {
                    window_id: _window_id,
                    event,
                };
                if let Some(pixel_event) = input_events_from_winit(
                    &winit_event,
                    self.ratio_x,
                    self.ratio_y,
                    &mut self.cursor_position,
                ) {
                    self.pending_events.push(pixel_event);
                }
            }
        }
    }
}

impl WinitAdapter {
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            base: AdapterBase::new(gn, project_path),
            window: None,
            event_loop: None,
            gl_display: None,
            gl_context: None,
            gl_surface: None,
            should_exit: false,
            app_handler: None,
            custom_cursor: None,
            drag: Default::default(),
        }
    }

    fn set_mouse_cursor(&mut self) {
        // Load custom cursor image
        let cursor_path = format!(
            "{}{}{}",
            self.base.project_path,
            std::path::MAIN_SEPARATOR,
            "assets/pix/cursor.png"
        );

        if let Ok(cursor_img) = image::open(&cursor_path) {
            let cursor_rgba = cursor_img.to_rgba8();
            let (width, height) = cursor_rgba.dimensions();
            let cursor_data = cursor_rgba.into_raw();

            // Create CustomCursor source from image data
            if let Ok(cursor_source) =
                CustomCursor::from_rgba(cursor_data, width as u16, height as u16, 0, 0)
            {
                // Need to create the actual cursor from the source using event_loop
                if let (Some(window), Some(event_loop)) = (&self.window, &self.event_loop) {
                    let custom_cursor = event_loop.create_custom_cursor(cursor_source);
                    self.custom_cursor = Some(custom_cursor.clone());
                    window.set_cursor(custom_cursor);
                    // Ensure cursor is visible after setting custom cursor
                    window.set_cursor_visible(true);
                }
            }
        } else {
            // If custom cursor fails to load, ensure standard cursor is visible
            if let Some(window) = &self.window {
                window.set_cursor_visible(true);
            }
        }
    }

    fn in_border(&self, x: f64, y: f64) -> WinitBorderArea {
        let w = self.cell_width();
        let h = self.cell_height();
        let sw = self.base.cell_w + 2;
        if y >= 0.0 && y < h as f64 {
            if x >= 0.0 && x <= ((sw - 1) as f32 * w) as f64 {
                return WinitBorderArea::TOPBAR;
            }
            if x > ((sw - 1) as f32 * w) as f64 && x <= (sw as f32 * w) as f64 {
                return WinitBorderArea::CLOSE;
            }
        } else if x > w as f64 && x <= ((sw - 1) as f32 * w) as f64 {
            return WinitBorderArea::NOPE;
        }
        WinitBorderArea::OTHER
    }
}

impl Adapter for WinitAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing Winit adapter...");

        // load texture file...
        let texture_path = format!(
            "{}{}{}",
            self.base.project_path,
            std::path::MAIN_SEPARATOR,
            PIXEL_TEXTURE_FILE
        );
        let teximg = image::open(&texture_path)
            .map_err(|e| e.to_string())
            .unwrap()
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
            "symbol_w={} symbol_h={}",
            PIXEL_SYM_WIDTH.get().expect("lazylock init"),
            PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
        );
        self.set_size(w, h)
            .set_ratiox(rx)
            .set_ratioy(ry)
            .set_pixel_size()
            .set_title(title);

        info!(
            "pixel_w={} pixel_h={}",
            self.base.pixel_w, self.base.pixel_h
        );

        // Create event loop
        let event_loop = EventLoop::new().unwrap();

        // For Retina displays, we need to adjust window logical size so its physical size matches our render area
        // First create a temporary window to get the scale factor
        let temp_window_size = LogicalSize::new(self.base.pixel_w, self.base.pixel_h);
        let temp_display_builder = DisplayBuilder::new().with_window_attributes(Some(
            winit::window::WindowAttributes::default()
                .with_title(&self.base.title)
                .with_inner_size(temp_window_size)
                .with_decorations(false)
                .with_resizable(false),
        ));
        let (temp_window, temp_gl_config) = temp_display_builder
            .build(&event_loop, ConfigTemplateBuilder::new(), |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();
        let temp_window = temp_window.unwrap();

                // Get scale factor and calculate proper window size to match SDL behavior
        let scale_factor = temp_window.scale_factor();
        
        // For consistency with SDL: on Retina displays, we want the window to be 2x larger
        // So logical size = original size (not divided by scale factor)
        let adjusted_logical_w = self.base.pixel_w;
        let adjusted_logical_h = self.base.pixel_h;
        let window_size = LogicalSize::new(adjusted_logical_w, adjusted_logical_h);
        
        info!(
            "Scale factor: {}, Window logical size: {}x{} (same as SDL)",
            scale_factor, adjusted_logical_w, adjusted_logical_h
        );
        info!(
            "Expected physical size: {}x{} (2x render size on Retina)",
            (adjusted_logical_w as f64 * scale_factor) as u32, 
            (adjusted_logical_h as f64 * scale_factor) as u32
        );

        let template = ConfigTemplateBuilder::new();

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(
            winit::window::WindowAttributes::default()
                .with_title(&self.base.title)
                .with_inner_size(window_size)
                .with_decorations(false) // borderless like SDL version
                .with_resizable(false),
        ));

        let (window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = window.unwrap();

        // Get actual physical size - should be 2x render size on Retina
        let physical_size = window.inner_size();
        info!(
            "Actual window physical size: {}x{} (2x on Retina)",
            physical_size.width, physical_size.height
        );
        info!(
            "Render area size: {}x{}",
            self.base.pixel_w, self.base.pixel_h
        );

        let gl_display = gl_config.display();
        let raw_window_handle = window.window_handle().unwrap().as_raw();

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(Some(raw_window_handle));

        let not_current_gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .expect("failed to create context")
        };

        // Use physical size for surface to match actual framebuffer (2x on Retina)
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            std::num::NonZeroU32::new(physical_size.width).unwrap(),
            std::num::NonZeroU32::new(physical_size.height).unwrap(),
        );

        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .expect("failed to create surface")
        };

        let gl_context = not_current_gl_context
            .make_current(&surface)
            .expect("failed to make context current");

        // Create the OpenGL context using glow
        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s)
                    .expect("failed to construct C string from string for gl proc address");

                gl_display.get_proc_address(s.as_c_str())
            })
        };

        // Store the OpenGL context and objects
        self.base.gl = Some(gl);
        self.window = Some(window);
        self.gl_display = Some(gl_display);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(surface);

        // Create GlPixel with logical dimensions for consistent coordinate system
        // The framebuffer is still high-res (physical size), but GlPixel will handle scaling
        self.base.gl_pixel = Some(GlPixel::new(
            self.base.gl.as_ref().unwrap(),
            "#version 330 core",
            self.base.pixel_w as i32,    // Use logical size for coordinate system
            self.base.pixel_h as i32,    // Use logical size for coordinate system  
            texwidth as i32,
            texheight as i32,
            &teximg,
        ));

        // Ratio remains the same, but OpenGL will render at higher resolution on Retina
        info!(
            "Using standard ratio: {}x{}, OpenGL framebuffer: {}x{} (2x on Retina)",
            self.base.ratio_x, self.base.ratio_y,
            physical_size.width, physical_size.height
        );

        self.app_handler = Some(WinitAppHandler {
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
            ratio_x: self.base.ratio_x, // Standard ratio - OpenGL handles scaling automatically
            ratio_y: self.base.ratio_y, // Standard ratio - OpenGL handles scaling automatically
            should_exit: false,
            adapter_ref: self as *mut WinitAdapter,
        });

        // Store event loop for later use
        self.event_loop = Some(event_loop);

        // Set custom mouse cursor (similar to SDL version)
        self.set_mouse_cursor();

        // Ensure cursor is visible (similar to SDL version)
        self.show_cursor().unwrap();

        info!("Winit window & OpenGL context initialized successfully");
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH.get().expect("lazylock init") / self.base.ratio_x
    }

    fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT.get().expect("lazylock init") / self.base.ratio_y
    }

    fn poll_event(&mut self, timeout: Duration, es: &mut Vec<Event>) -> bool {
        if let (Some(event_loop), Some(app_handler)) =
            (self.event_loop.as_mut(), self.app_handler.as_mut())
        {
            // Use pump_app_events for non-blocking event polling
            let pump_timeout = Some(timeout);
            let status = event_loop.pump_app_events(pump_timeout, app_handler);

            // Collect events from app handler, but filter out dragging events
            for event in app_handler.pending_events.drain(..) {
                // Don't pass mouse events to the game when dragging window
                if !self.drag.draging {
                    es.push(event);
                }
            }

            // Check if we should exit
            if app_handler.should_exit {
                return true;
            }

            // Check pump status
            if let PumpStatus::Exit(_) = status {
                return true;
            }
        }

        self.should_exit
    }

    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        // process window dragging move...
        winit_move_win(
            &mut self.drag.need,
            self.window.as_ref(),
            self.drag.dx,
            self.drag.dy,
        );

        self.draw_all_graph(current_buffer, previous_buffer, pixel_sprites, stage);

        // swap buffers for display
        if let Some(surface) = &self.gl_surface {
            if let Some(context) = &self.gl_context {
                surface.swap_buffers(context).unwrap();
            }
        }

        // Request redraw
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), String> {
        // For GUI applications, we don't want to hide the mouse cursor
        // This is similar to SDL behavior - let the mouse cursor remain visible
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), String> {
        if let Some(window) = &self.window {
            window.set_cursor_visible(true);
        }
        Ok(())
    }

    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok((0, 0))
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
    
    // Override draw_render_textures_to_screen to handle Retina scaling
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]  
    fn draw_render_textures_to_screen(&mut self) {
        use crate::render::adapter::gl::color::GlColor;
        use crate::render::adapter::gl::transform::GlTransform;
        use glow::HasContext;
        
        // Get window physical size first to avoid borrowing conflicts
        let physical_size = if let Some(window) = &self.window {
            Some(window.inner_size())
        } else {
            None
        };
        
        let bs = self.get_base();

        if let (Some(pix), Some(gl)) = (&mut bs.gl_pixel, &mut bs.gl) {
            // First bind screen with GlPixel's logical viewport
            pix.bind_screen(gl);
            
            // Then manually set the correct viewport for Retina displays
            if let Some(physical_size) = physical_size {
                unsafe {
                    gl.viewport(0, 0, physical_size.width as i32, physical_size.height as i32);
                }
            }
            
            let c = GlColor::new(1.0, 1.0, 1.0, 1.0);

            // draw render_texture 2 ( main buffer )
            if !pix.get_render_texture_hidden(2) {
                let t = GlTransform::new();
                pix.draw_general2d(gl, 2, [0.0, 0.0, 1.0, 1.0], &t, &c);
            }

            // draw render_texture 3 ( gl transition )
            if !pix.get_render_texture_hidden(3) {
                let pcw = pix.canvas_width as f32;
                let pch = pix.canvas_height as f32;
                let rx = bs.ratio_x;
                let ry = bs.ratio_y;
                let pw = 40.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                let ph = 25.0 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

                let mut t2 = GlTransform::new();
                t2.scale(pw / pcw, ph / pch);
                pix.draw_general2d(
                    gl,
                    3,
                    [0.0 / pcw, (pch - ph) / pch, pw / pcw, ph / pch],
                    &t2,
                    &c,
                );
            }
        }
    }
}

/// Convert winit input events to RustPixel event, for the sake of unified event processing
pub fn input_events_from_winit(
    event: &WinitEvent<()>,
    adjx: f32,
    adjy: f32,
    cursor_pos: &mut (f64, f64),
) -> Option<Event> {
    use crate::event::{
        Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
    };

    let sym_width = PIXEL_SYM_WIDTH.get().expect("lazylock init");
    let sym_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init");

    match event {
        WinitEvent::WindowEvent {
            event: window_event,
            ..
        } => {
            match window_event {
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => {
                    if key_event.state == winit::event::ElementState::Pressed {
                        if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key
                        {
                            let kc = match keycode {
                                winit::keyboard::KeyCode::Space => ' ',
                                winit::keyboard::KeyCode::KeyA => 'a',
                                winit::keyboard::KeyCode::KeyB => 'b',
                                winit::keyboard::KeyCode::KeyC => 'c',
                                winit::keyboard::KeyCode::KeyD => 'd',
                                winit::keyboard::KeyCode::KeyE => 'e',
                                winit::keyboard::KeyCode::KeyF => 'f',
                                winit::keyboard::KeyCode::KeyG => 'g',
                                winit::keyboard::KeyCode::KeyH => 'h',
                                winit::keyboard::KeyCode::KeyI => 'i',
                                winit::keyboard::KeyCode::KeyJ => 'j',
                                winit::keyboard::KeyCode::KeyK => 'k',
                                winit::keyboard::KeyCode::KeyL => 'l',
                                winit::keyboard::KeyCode::KeyM => 'm',
                                winit::keyboard::KeyCode::KeyN => 'n',
                                winit::keyboard::KeyCode::KeyO => 'o',
                                winit::keyboard::KeyCode::KeyP => 'p',
                                winit::keyboard::KeyCode::KeyQ => 'q',
                                winit::keyboard::KeyCode::KeyR => 'r',
                                winit::keyboard::KeyCode::KeyS => 's',
                                winit::keyboard::KeyCode::KeyT => 't',
                                winit::keyboard::KeyCode::KeyU => 'u',
                                winit::keyboard::KeyCode::KeyV => 'v',
                                winit::keyboard::KeyCode::KeyW => 'w',
                                winit::keyboard::KeyCode::KeyX => 'x',
                                winit::keyboard::KeyCode::KeyY => 'y',
                                winit::keyboard::KeyCode::KeyZ => 'z',
                                winit::keyboard::KeyCode::ArrowUp => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Up,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::ArrowDown => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Down,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::ArrowLeft => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Left,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::ArrowRight => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Right,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                _ => return None,
                            };
                            let cte = KeyEvent::new(KeyCode::Char(kc), KeyModifiers::NONE);
                            return Some(Event::Key(cte));
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let mouse_event = match (state, button) {
                        (winit::event::ElementState::Pressed, winit::event::MouseButton::Left) => {
                            Some(MouseEvent {
                                kind: Down(Left),
                                column: (cursor_pos.0 / (sym_width / adjx) as f64) as u16,
                                row: (cursor_pos.1 / (sym_height / adjy) as f64) as u16,
                                modifiers: KeyModifiers::NONE,
                            })
                        }
                        (winit::event::ElementState::Released, winit::event::MouseButton::Left) => {
                            Some(MouseEvent {
                                kind: Up(Left),
                                column: (cursor_pos.0 / (sym_width / adjx) as f64) as u16,
                                row: (cursor_pos.1 / (sym_height / adjy) as f64) as u16,
                                modifiers: KeyModifiers::NONE,
                            })
                        }
                        _ => None,
                    };

                    if let Some(mut mc) = mouse_event {
                        if mc.column >= 1 {
                            mc.column -= 1;
                        }
                        if mc.row >= 1 {
                            mc.row -= 1;
                        }
                        return Some(Event::Mouse(mc));
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // Update cursor position
                    cursor_pos.0 = position.x;
                    cursor_pos.1 = position.y;

                    let mut mc = MouseEvent {
                        kind: Moved,
                        column: (position.x / (sym_width / adjx) as f64) as u16,
                        row: (position.y / (sym_height / adjy) as f64) as u16,
                        modifiers: KeyModifiers::NONE,
                    };
                    if mc.column >= 1 {
                        mc.column -= 1;
                    }
                    if mc.row >= 1 {
                        mc.row -= 1;
                    }
                    return Some(Event::Mouse(mc));
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

pub fn winit_move_win(drag_need: &mut bool, window: Option<&Window>, dx: f64, dy: f64) {
    // dragging window, set the correct position of a window
    if *drag_need {
        if let Some(win) = window {
            if let Ok(pos) = win.outer_position() {
                let new_x = pos.x + dx as i32;
                let new_y = pos.y + dy as i32;
                let _ = win.set_outer_position(winit::dpi::PhysicalPosition::new(new_x, new_y));
            }
        }
        *drag_need = false;
    }
}
