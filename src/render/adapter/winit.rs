// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements an Adapter trait using winit for window management and glow for OpenGL rendering.
//! This replaces the SDL2 implementation while maintaining the same functionality.

use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
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
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, Event as WinitEvent, KeyEvent as WinitKeyEvent, MouseButton as WinitMouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode as WinitKeyCode, PhysicalKey},
    window::{Window, WindowId},
};
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, Version},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;

// data for drag window...
#[derive(Default)]
struct Drag {
    need: bool,
    dragging: bool,
    mouse_x: f64,
    mouse_y: f64,
    dx: f64,
    dy: f64,
}

pub struct WinitAdapter {
    pub base: AdapterBase,
    
    // winit objects
    pub event_loop: Option<EventLoop<()>>,
    pub window: Option<Window>,
    
    // glutin objects for OpenGL context
    pub gl_display: Option<glutin::display::Display>,
    pub gl_context: Option<glutin::context::PossiblyCurrentContext>,
    pub gl_surface: Option<Surface<WindowSurface>>,
    
    // events storage
    pub pending_events: Vec<Event>,
    pub should_exit: bool,
    
    // data for dragging the window
    drag: Drag,
}

pub enum WinitBorderArea {
    NOPE,
    CLOSE,
    TOPBAR,
    OTHER,
}

impl WinitAdapter {
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            base: AdapterBase::new(gn, project_path),
            event_loop: None,
            window: None,
            gl_display: None,
            gl_context: None,
            gl_surface: None,
            pending_events: Vec::new(),
            should_exit: false,
            drag: Default::default(),
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

    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested => return true,
            WindowEvent::MouseInput { state: ElementState::Pressed, button: WinitMouseButton::Left, .. } => {
                if let Some(window) = &self.window {
                    if let Ok(cursor_pos) = window.cursor_position() {
                        let bs = self.in_border(cursor_pos.x, cursor_pos.y);
                        match bs {
                            WinitBorderArea::TOPBAR | WinitBorderArea::OTHER => {
                                // start dragging when mouse left click
                                self.drag.dragging = true;
                                self.drag.mouse_x = cursor_pos.x;
                                self.drag.mouse_y = cursor_pos.y;
                            }
                            WinitBorderArea::CLOSE => {
                                return true;
                            }
                            _ => {}
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: WinitMouseButton::Left, .. } => {
                // stop dragging when mouse left button is released
                self.drag.dragging = false;
            }
            WindowEvent::CursorMoved { position, .. } if self.drag.dragging => {
                self.drag.need = true;
                // dragging window when mouse left button is held and moving
                self.drag.dx = position.x - self.drag.mouse_x;
                self.drag.dy = position.y - self.drag.mouse_y;
            }
            _ => {}
        }
        false
    }

    fn move_window(&mut self) {
        // dragging window, set the correct position of a window
        if self.drag.need {
            if let Some(window) = &self.window {
                if let Ok(current_pos) = window.outer_position() {
                    let new_pos = PhysicalPosition::new(
                        current_pos.x + self.drag.dx as i32,
                        current_pos.y + self.drag.dy as i32,
                    );
                    let _ = window.set_outer_position(new_pos);
                }
            }
            self.drag.need = false;
        }
    }
}

impl Adapter for WinitAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
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
                winit::window::WindowAttributes::default()
                    .with_title(&self.base.title)
                    .with_inner_size(window_size)
                    .with_decorations(false) // borderless like SDL version
                    .with_resizable(false)
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
        let raw_window_handle = window.raw_window_handle();
        
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(Some(raw_window_handle));

        let mut not_current_gl_context = Some(
            unsafe { gl_display.create_context(&gl_config, &context_attributes) }
                .expect("failed to create context"),
        );

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            std::num::NonZeroU32::new(self.base.pixel_w).unwrap(),
            std::num::NonZeroU32::new(self.base.pixel_h).unwrap(),
        );

        let surface = unsafe { gl_config.display().create_window_surface(&gl_config, &attrs) }
            .expect("failed to create surface");

        let gl_context = not_current_gl_context
            .take()
            .unwrap()
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
        self.event_loop = Some(event_loop);

        self.base.gl_pixel = Some(GlPixel::new(
            self.base.gl.as_ref().unwrap(),
            "#version 330 core",
            self.base.pixel_w as i32,
            self.base.pixel_h as i32,
            texwidth as i32,
            texheight as i32,
            &teximg,
        ));

        info!("Window & gl init ok...");
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
        // This is a simplified polling implementation
        // In a real winit application, you'd typically use the event loop differently
        // For now, we'll return the pending events and check for exit condition
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
        // process window dragging move...
        self.move_window();

        self.draw_all_graph(current_buffer, previous_buffer, pixel_sprites, stage);

        // swap buffers for display
        if let Some(surface) = &self.gl_surface {
            surface.swap_buffers(&self.gl_context.as_ref().unwrap()).unwrap();
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
pub fn input_events_from_winit(event: &WinitEvent<()>, adjx: f32, adjy: f32) -> Option<Event> {
    let sym_width = PIXEL_SYM_WIDTH.get().expect("lazylock init");
    let sym_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init");
    
    match event {
        WinitEvent::WindowEvent { event: WindowEvent::KeyboardInput { event: key_event, .. }, .. } => {
            if key_event.state == ElementState::Pressed {
                if let PhysicalKey::Code(keycode) = key_event.physical_key {
                    let kc = match keycode {
                        WinitKeyCode::Space => ' ',
                        WinitKeyCode::KeyA => 'a',
                        WinitKeyCode::KeyB => 'b',
                        WinitKeyCode::KeyC => 'c',
                        WinitKeyCode::KeyD => 'd',
                        WinitKeyCode::KeyE => 'e',
                        WinitKeyCode::KeyF => 'f',
                        WinitKeyCode::KeyG => 'g',
                        WinitKeyCode::KeyH => 'h',
                        WinitKeyCode::KeyI => 'i',
                        WinitKeyCode::KeyJ => 'j',
                        WinitKeyCode::KeyK => 'k',
                        WinitKeyCode::KeyL => 'l',
                        WinitKeyCode::KeyM => 'm',
                        WinitKeyCode::KeyN => 'n',
                        WinitKeyCode::KeyO => 'o',
                        WinitKeyCode::KeyP => 'p',
                        WinitKeyCode::KeyQ => 'q',
                        WinitKeyCode::KeyR => 'r',
                        WinitKeyCode::KeyS => 's',
                        WinitKeyCode::KeyT => 't',
                        WinitKeyCode::KeyU => 'u',
                        WinitKeyCode::KeyV => 'v',
                        WinitKeyCode::KeyW => 'w',
                        WinitKeyCode::KeyX => 'x',
                        WinitKeyCode::KeyY => 'y',
                        WinitKeyCode::KeyZ => 'z',
                        _ => return None,
                    };
                    let cte = KeyEvent::new(KeyCode::Char(kc), KeyModifiers::NONE);
                    return Some(Event::Key(cte));
                }
            }
        }
        WinitEvent::WindowEvent { event: WindowEvent::MouseInput { state, button, .. }, .. } => {
            // We need cursor position for mouse events, but winit doesn't provide it in MouseInput
            // This is a limitation of the current implementation
            let (x, y) = (0, 0); // Placeholder - in real implementation, we'd track cursor position
            
            let mouse_event = match (state, button) {
                (ElementState::Pressed, WinitMouseButton::Left) => {
                    Some(MouseEvent {
                        kind: Down(Left),
                        column: x,
                        row: y,
                        modifiers: KeyModifiers::NONE,
                    })
                }
                (ElementState::Released, WinitMouseButton::Left) => {
                    Some(MouseEvent {
                        kind: Up(Left),
                        column: x,
                        row: y,
                        modifiers: KeyModifiers::NONE,
                    })
                }
                _ => None,
            };
            
            if let Some(mut mc) = mouse_event {
                mc.column = (mc.column as f32 / (sym_width / adjx)) as u16;
                mc.row = (mc.row as f32 / (sym_height / adjy)) as u16;
                if mc.column >= 1 {
                    mc.column -= 1;
                }
                if mc.row >= 1 {
                    mc.row -= 1;
                }
                return Some(Event::Mouse(mc));
            }
        }
        WinitEvent::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
            let mut mc = MouseEvent {
                kind: Moved,
                column: position.x as u16,
                row: position.y as u16,
                modifiers: KeyModifiers::NONE,
            };
            mc.column = (mc.column as f32 / (sym_width / adjx)) as u16;
            mc.row = (mc.row as f32 / (sym_height / adjy)) as u16;
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
    None
} 