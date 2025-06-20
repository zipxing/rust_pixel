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
use log::info;
use std::any::Any;
use std::time::Duration;
pub use winit::{
    dpi::LogicalSize,
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, Version},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;

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
    
    // Event handling
    pub pending_events: Vec<Event>,
    pub cursor_position: (f64, f64),
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
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
        }
    }

    pub fn run_event_loop(&mut self) -> Result<(), String> {
        if let Some(event_loop) = self.event_loop.take() {
            event_loop.run(move |event, window_target| {
                window_target.set_control_flow(ControlFlow::Poll);
                
                match event {
                    WinitEvent::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                        window_target.exit();
                    }
                    WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                        // Trigger redraw
                        if let Some(surface) = &self.gl_surface {
                            if let Some(context) = &self.gl_context {
                                surface.swap_buffers(context).unwrap();
                            }
                        }
                    }
                    _ => {}
                }
            }).map_err(|e| e.to_string())?;
        }
        Ok(())
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
        
        // Create window with OpenGL context
        let window_size = LogicalSize::new(self.base.pixel_w, self.base.pixel_h);
        
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(false);

        let display_builder = DisplayBuilder::new()
            .with_window_attributes(
                Some(winit::window::WindowAttributes::default()
                    .with_title(&self.base.title)
                    .with_inner_size(window_size)
                    .with_decorations(false) // borderless like SDL version
                    .with_resizable(false))
            );

        let (window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        let transparency_check = config.supports_transparency().unwrap_or(false)
                            & !accum.supports_transparency().unwrap_or(false);

                        if transparency_check || config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = window.unwrap();
        
        let gl_display = gl_config.display();
        let raw_window_handle = window.window_handle().unwrap().as_raw();
        
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(Some(raw_window_handle));

        let not_current_gl_context = unsafe { 
            gl_display.create_context(&gl_config, &context_attributes)
                .expect("failed to create context")
        };

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            std::num::NonZeroU32::new(self.base.pixel_w).unwrap(),
            std::num::NonZeroU32::new(self.base.pixel_h).unwrap(),
        );

        let surface = unsafe { 
            gl_config.display().create_window_surface(&gl_config, &attrs)
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

        self.base.gl_pixel = Some(GlPixel::new(
            self.base.gl.as_ref().unwrap(),
            "#version 330 core",
            self.base.pixel_w as i32,
            self.base.pixel_h as i32,
            texwidth as i32,
            texheight as i32,
            &teximg,
        ));

        // Store event loop for later use
        self.event_loop = Some(event_loop);

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
        // Return any pending events that were collected in the event loop
        es.extend(self.pending_events.drain(..));
        
        if self.should_exit {
            return true;
        }
        
        // Sleep for the timeout duration
        std::thread::sleep(timeout);
        false
    }

    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
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
        if let Some(window) = &self.window {
            window.set_cursor_visible(false);
        }
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
}

/// Convert winit input events to RustPixel event, for the sake of unified event processing
pub fn input_events_from_winit(event: &WinitEvent<()>, adjx: f32, adjy: f32, cursor_pos: &mut (f64, f64)) -> Option<Event> {
    use crate::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*};
    
    let sym_width = PIXEL_SYM_WIDTH.get().expect("lazylock init");
    let sym_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init");
    
    match event {
        WinitEvent::WindowEvent { event: window_event, .. } => {
            match window_event {
                WindowEvent::KeyboardInput { event: key_event, .. } => {
                    if key_event.state == winit::event::ElementState::Pressed {
                        if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key {
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
                                winit::keyboard::KeyCode::ArrowUp => return Some(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))),
                                winit::keyboard::KeyCode::ArrowDown => return Some(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))),
                                winit::keyboard::KeyCode::ArrowLeft => return Some(Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))),
                                winit::keyboard::KeyCode::ArrowRight => return Some(Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))),
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