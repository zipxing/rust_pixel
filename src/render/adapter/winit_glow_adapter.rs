// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # Winit Adapter Implementation
//!
//! Cross-platform rendering adapter based on winit + glutin + glow technology stack.
//!
//! ## Technology Stack
//! - **winit**: Cross-platform window management and event handling
//! - **glutin**: OpenGL context management
//! - **glow**: Modern OpenGL bindings
//!
//! ## Features
//! - Cross-platform window management (Windows, macOS, Linux)
//! - High DPI/Retina display support
//! - Custom mouse cursor
//! - Window drag functionality
//! - Keyboard and mouse event handling
//! - OpenGL hardware-accelerated rendering
//!
//! ## Architecture Design
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │             WinitAdapter                    │
//! ├─────────────────────────────────────────────┤
//! │  Window Management  │  OpenGL Context       │
//! │  - winit::Window    │  - glutin::Context    │
//! │  - Event handling   │  - glutin::Surface    │
//! │  - Cursor support   │  - glow::Context      │
//! └─────────────────────────────────────────────┘
//! ```

use crate::event::Event;
use crate::render::{
    adapter::{
        winit_common::{
            input_events_from_winit, winit_init_common, winit_move_win, Drag, WindowInitParams,
        },
        Adapter, AdapterBase,
    },
    buffer::Buffer,
    sprite::Sprites,
};

// OpenGL backend imports (glow + glutin)
use crate::render::adapter::gl::pixel::GlPixelRenderer;

use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, Version},
    display::{GetGlDisplay, GlDisplay},
    prelude::GlSurface,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};

use glutin_winit::DisplayBuilder;

// Import HasWindowHandle trait for window_handle() method
use winit::raw_window_handle::HasWindowHandle;

// WGPU backend imports - only when wgpu is enabled
#[cfg(feature = "wgpu")]
use crate::render::adapter::wgpu::{pixel::WgpuPixelRender, WgpuRender};
use log::info;
// Removed unused import
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
pub use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Cursor, CustomCursor, Window},
};

/// Border area enumeration
///
/// Defines mouse click area types for determining whether drag operation should be triggered.
pub enum WinitBorderArea {
    /// Invalid area
    NOPE,
    /// Close button area
    CLOSE,
    /// Top title bar area (draggable)
    TOPBAR,
    /// Other border areas (draggable)
    OTHER,
}

/// Winit adapter main structure
///
/// Encapsulates all components of winit window management and modern rendering backends.
/// Supports two rendering backends: OpenGL (glow) and modern GPU API (wgpu).
/// Implements the same interface as SDL adapter for seamless replacement.

/// Winit + Glow OpenGL adapter
///
/// Specifically handles cross-platform adaptation for winit window management and OpenGL rendering.
/// Separated from WinitWgpuAdapter to avoid complex conditional compilation.
pub struct WinitGlowAdapter {
    /// Base adapter data
    pub base: AdapterBase,

    /// Window instance
    pub window: Option<Arc<Window>>,

    /// Event loop (for pump events mode)
    pub event_loop: Option<EventLoop<()>>,

    /// Drag state management
    pub drag: Drag,

    /// Whether cursor has been set
    pub cursor_set: bool,

    /// Window initialization parameters
    pub window_init_params: Option<WindowInitParams>,

    // OpenGL backend objects
    /// OpenGL display context
    pub gl_display: Option<glutin::display::Display>,
    /// OpenGL rendering context
    pub gl_context: Option<glutin::context::PossiblyCurrentContext>,
    /// OpenGL rendering surface
    pub gl_surface: Option<Surface<WindowSurface>>,
    /// Direct OpenGL pixel renderer
    pub gl_pixel_renderer: Option<GlPixelRenderer>,

    /// Whether to exit the program
    pub should_exit: bool,

    /// Event handler (for pump events mode)
    pub app_handler: Option<WinitGlowAppHandler>,

    /// Custom mouse cursor
    pub custom_cursor: Option<CustomCursor>,
}

/// Winit + Glow application event handler
///
/// Implements winit's ApplicationHandler trait to handle window events and user input.
/// Specifically designed for OpenGL adapter.
pub struct WinitGlowAppHandler {
    /// Pending pixel event queue
    pub pending_events: Vec<Event>,
    /// Current mouse position
    pub cursor_position: (f64, f64),
    /// X-axis scaling coefficient
    pub ratio_x: f32,
    /// Y-axis scaling coefficient
    pub ratio_y: f32,
    /// Whether to use TUI character height (32px) for mouse coordinate calculation
    pub use_tui_height: bool,
    /// Whether to exit
    pub should_exit: bool,

    /// Adapter reference (for drag handling)
    ///
    /// Note: This uses a raw pointer to avoid borrow checker limitations,
    /// and the adapter state needs to be modified during event processing.
    /// It must be ensured that it is safe to use.
    pub adapter_ref: *mut WinitGlowAdapter,
}

impl ApplicationHandler for WinitGlowAppHandler {
    /// Callback when application resumes
    ///
    /// Creates OpenGL window and rendering resources in resumed event.
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Create window and rendering context here
        if let Some(adapter) = unsafe { self.adapter_ref.as_mut() } {
            if adapter.window.is_none() {
                adapter.create_glow_window_and_context(event_loop);
            }

            // Delay cursor setting - after window is fully initialized
            if !adapter.cursor_set {
                // Clear screen before setting cursor - may help with transparency issues
                adapter.initial_clear_screen();
                adapter.set_mouse_cursor();
                adapter.cursor_set = true;
            }
        }
    }

    /// Handle window events
    ///
    /// This is the core method for event handling, processing all window events including:
    /// - Window close request
    /// - Keyboard input (supports Q key exit)
    /// - Mouse movement and clicks
    /// - Window drag logic
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
                // Handle Q key exit (consistent with SDL version)
                if key_event.state == winit::event::ElementState::Pressed {
                    if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key {
                        if keycode == winit::keyboard::KeyCode::KeyQ {
                            self.should_exit = true;
                            event_loop.exit();
                            return;
                        }
                    }
                }

                // Convert keyboard events to pixel events
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
                    self.use_tui_height,
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
                        self.cursor_position =
                            (position.x / scale_factor, position.y / scale_factor);
                    } else {
                        self.cursor_position = (position.x, position.y);
                    }
                }

                // Handle window drag
                unsafe {
                    let adapter = &mut *self.adapter_ref;
                    if adapter.drag.draging {
                        adapter.drag.need = true;
                        adapter.drag.dx = position.x - adapter.drag.mouse_x;
                        adapter.drag.dy = position.y - adapter.drag.mouse_y;
                    }
                }

                // Only convert to pixel events when not dragging
                // Use logical position to ensure consistent coordinate system
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
                            self.use_tui_height,
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
                                    // Start dragging when left mouse button is pressed in border area
                                    adapter.drag.draging = true;
                                    adapter.drag.mouse_x = self.cursor_position.0;
                                    adapter.drag.mouse_y = self.cursor_position.1;
                                }
                                WinitBorderArea::CLOSE => {
                                    // Exit program when clicking close button area
                                    self.should_exit = true;
                                    event_loop.exit();
                                }
                                _ => {
                                    // Non-dragging area, pass event to game logic
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
                                        self.use_tui_height,
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

                            // Only pass mouse release event to game when not dragging
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
                                    self.use_tui_height,
                                    &mut self.cursor_position,
                                ) {
                                    self.pending_events.push(pixel_event);
                                }
                            }
                        }
                    }
                    _ => {
                        // Convert other mouse input events
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
                            self.use_tui_height,
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
                    self.use_tui_height,
                    &mut self.cursor_position,
                ) {
                    self.pending_events.push(pixel_event);
                }
            }
        }
    }
}

impl WinitGlowAdapter {
    /// Create a new Winit + Glow adapter instance
    ///
    /// # Parameters
    /// - `gn`: Game name identifier
    /// - `project_path`: Project root path for resource loading
    ///
    /// # Returns
    /// Returns the initialized WinitGlowAdapter instance, all OpenGL related components are None,
    /// and need to be used normally after calling the init() method.
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            base: AdapterBase::new(gn, project_path),
            window: None,
            event_loop: None,
            drag: Drag::default(),
            cursor_set: false,
            window_init_params: None,

            // OpenGL backend fields
            gl_display: None,
            gl_context: None,
            gl_surface: None,
            gl_pixel_renderer: None,

            should_exit: false,
            app_handler: None,
            custom_cursor: None,
        }
    }

    /// General initialization method - handles all common logic
    ///
    /// This method handles initialization steps required by both rendering backends:
    /// 1. Load texture files and set symbol dimensions
    /// 2. Set basic parameters (size, ratio, pixel size)
    /// 3. Create event loop
    /// 4. Set application handler
    /// 5. Store window initialization parameters
    ///
    /// # Parameters
    /// - `w`: Logical width (number of characters)
    /// - `h`: Logical height (number of characters)
    /// - `rx`: X-axis scaling ratio
    /// - `ry`: Y-axis scaling ratio
    /// - `title`: Window title
    ///
    /// # Returns
    /// Returns the texture path for subsequent renderer initialization
    fn init_common(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) -> String {
        // Use unified winit shared initialization logic
        let (event_loop, window_init_params, texture_path) =
            winit_init_common(self, w, h, rx, ry, title);

        // Store shared initialization results
        self.event_loop = Some(event_loop);
        self.window_init_params = Some(window_init_params);

        // Set Glow specific application handler
        self.app_handler = Some(WinitGlowAppHandler {
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
            ratio_x: self.base.gr.ratio_x,
            ratio_y: self.base.gr.ratio_y,
            use_tui_height: self.base.gr.use_tui_height,
            should_exit: false,
            adapter_ref: self as *mut WinitGlowAdapter,
        });

        texture_path
    }

    /// OpenGL backend initialization
    ///
    /// Uses unified lifecycle management, window creation deferred to resumed event.
    fn init_glow(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing WinitGlow adapter with OpenGL backend...");
        let _texture_path = self.init_common(w, h, rx, ry, title);
        // Window creation will be completed in resumed event
    }

    /// Create OpenGL window and context in resumed event
    fn create_glow_window_and_context(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let params = self.window_init_params.as_ref().unwrap().clone();

        info!("Creating OpenGL window and context...");

        // Calculate window size (handle Retina displays)
        let window_size = LogicalSize::new(self.base.gr.pixel_w, self.base.gr.pixel_h);

        let template = ConfigTemplateBuilder::new();
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(
            winit::window::Window::default_attributes()
                .with_title(&params.title)
                .with_inner_size(window_size)
                .with_decorations(true) // Use OS window decoration (title bar, etc.)
                .with_resizable(false),
        ));

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
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

        let window = Arc::new(window.unwrap());
        let physical_size = window.inner_size();

        info!(
            "Window created - logical: {}x{}, physical: {}x{}",
            self.base.gr.pixel_w, self.base.gr.pixel_h, physical_size.width, physical_size.height
        );

        // Create OpenGL context
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

        let gl_surface = unsafe {
            let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
                window.window_handle().unwrap().as_raw(),
                std::num::NonZeroU32::new(physical_size.width).unwrap(),
                std::num::NonZeroU32::new(physical_size.height).unwrap(),
            );

            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

        // Create OpenGL renderer
        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s)
                    .expect("failed to construct C string from string for gl proc address");
                gl_display.get_proc_address(&s)
            })
        };

        // Create and initialize GlPixelRenderer
        let teximg = image::open(&params.texture_path)
            .map_err(|e| e.to_string())
            .unwrap()
            .to_rgba8();
        let texwidth = teximg.width();
        let texheight = teximg.height();

        let gl_pixel_renderer = GlPixelRenderer::new(
            gl,
            "#version 330 core",
            self.base.gr.pixel_w as i32,
            self.base.gr.pixel_h as i32,
            texwidth as i32,
            texheight as i32,
            &teximg,
        );

        // Store all OpenGL objects
        self.window = Some(window);
        self.gl_display = Some(gl_display);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl_pixel_renderer = Some(gl_pixel_renderer);

        info!("OpenGL window & context initialized successfully");
    }

    /// Execute initial clear screen operation
    ///
    /// Prevents white screen flicker during window creation, immediately clears the screen and displays a black background.
    /// In OpenGL mode, this ensures that the window is displayed correctly immediately after creation.
    fn initial_clear_screen(&mut self) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            use glow::HasContext;
            let gl = gl_pixel_renderer.get_gl();

            unsafe {
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }

            if let Some(gl_surface) = &self.gl_surface {
                if let Err(e) = gl_surface.swap_buffers(self.gl_context.as_ref().unwrap()) {
                    eprintln!("Failed to swap buffers during initial clear: {:?}", e);
                }
            }
        }
    }

    /// Set custom mouse cursor
    ///
    /// Loads cursor image from assets/pix/cursor.png and sets it as the window's custom cursor.
    /// - Automatically converts to RGBA8 format
    /// - Hotspot position set to (0, 0)
    /// - Handles transparency and pre-multiplied alpha
    fn set_mouse_cursor(&mut self) {
        // Build cursor image file path
        let cursor_path = format!(
            "{}{}{}",
            self.base.project_path,
            std::path::MAIN_SEPARATOR,
            "assets/pix/cursor.png"
        );

        if let Ok(cursor_img) = image::open(&cursor_path) {
            let cursor_rgba = cursor_img.to_rgba8();
            let (width, height) = cursor_rgba.dimensions();
            let mut cursor_data = cursor_rgba.into_raw();

            // Pre-multiplied alpha handling - this is a common method to solve cursor transparency issues
            for chunk in cursor_data.chunks_exact_mut(4) {
                let alpha = chunk[3] as f32 / 255.0;
                chunk[0] = (chunk[0] as f32 * alpha) as u8; // R * alpha
                chunk[1] = (chunk[1] as f32 * alpha) as u8; // G * alpha
                chunk[2] = (chunk[2] as f32 * alpha) as u8; // B * alpha
            }

            // Create CustomCursor source from image data
            let cursor_source =
                CustomCursor::from_rgba(cursor_data, width as u16, height as u16, 0, 0).unwrap();

            // Need to create the actual cursor from the source using event_loop
            if let (Some(window), Some(event_loop)) = (&self.window, &self.event_loop) {
                let custom_cursor = event_loop.create_custom_cursor(cursor_source);
                self.custom_cursor = Some(custom_cursor.clone());
                window.as_ref().set_cursor(custom_cursor);
                // Ensure cursor is visible after setting custom cursor
                window.as_ref().set_cursor_visible(true);
            }
        }
    }

    /// Check if mouse position is in border area
    ///
    /// Determines the type of border area for the mouse click position, decides whether to trigger drag operation.
    ///
    /// # Parameters
    /// - `x`: Mouse X coordinate
    /// - `y`: Mouse Y coordinate
    ///
    /// # Returns
    /// Returns the corresponding border area type
    fn in_border(&self, x: f64, y: f64) -> WinitBorderArea {
        let w = self.base.gr.cell_width();
        let h = self.base.gr.cell_height();
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

impl Adapter for WinitGlowAdapter {
    /// Initialize Winit + Glow adapter
    ///
    /// This is the main initialization method for the adapter, specifically using OpenGL + Glutin rendering pipeline.
    ///
    /// # Parameters
    /// - `w`: Logical width (number of characters)
    /// - `h`: Logical height (number of characters)
    /// - `rx`: X-axis scaling ratio
    /// - `ry`: Y-axis scaling ratio
    /// - `title`: Window title
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing WinitGlow adapter with OpenGL backend...");
        self.init_glow(w, h, rx, ry, title);
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    /// Poll events
    ///
    /// Handles window events and converts them to RustPixel events. Uses pump_events mode
    /// to avoid blocking the main thread and ensure rendering performance.
    ///
    /// # Parameters
    /// - `timeout`: Event polling timeout (unused)
    /// - `es`: Output event vector
    ///
    /// # Returns
    /// Returns true if program should exit
    ///
    /// # Special handling
    /// - Window drag: Detect and execute window movement
    /// - Q key exit: Consistent with SDL version
    /// - Retina display: Correctly handle high DPI coordinate conversion
    fn poll_event(&mut self, timeout: Duration, es: &mut Vec<Event>) -> bool {
        // Poll event logic - debug output removed

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

        // Return exit status
        self.should_exit
    }

    /// Render a frame to the screen
    ///
    /// Selects different rendering backends based on compilation features:
    /// - WGPU version: Uses modern GPU rendering pipeline
    /// - OpenGL version: Uses traditional OpenGL rendering pipeline
    ///
    /// # Parameters
    /// - `current_buffer`: Current frame buffer
    /// - `previous_buffer`: Previous frame buffer
    /// - `pixel_sprites`: List of pixel sprites
    /// - `stage`: Rendering stage
    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        // Handle window drag movement
        winit_move_win(
            &mut self.drag.need,
            self.window.as_ref().map(|v| &**v),
            self.drag.dx,
            self.drag.dy,
        );

        // Use unified graphics rendering process - consistent with SdlAdapter
        self.draw_all_graph(current_buffer, previous_buffer, pixel_sprites, stage);
        self.post_draw();
        Ok(())
    }

    fn post_draw(&mut self) {
        // OpenGL mode: swap buffers to display rendering results
        if let (Some(gl_surface), Some(gl_context)) = (&self.gl_surface, &self.gl_context) {
            if let Err(e) = gl_surface.swap_buffers(gl_context) {
                eprintln!("Failed to swap buffers: {:?}", e);
            }
        }

        if let Some(window) = &self.window {
            window.as_ref().request_redraw();
        }
    }

    /// Hide cursor
    ///
    /// In graphical applications, we do not want to hide the mouse cursor.
    /// This is similar to the behavior of the SDL version - keep the mouse cursor visible.
    fn hide_cursor(&mut self) -> Result<(), String> {
        // For GUI applications, we do not want to hide the mouse cursor
        Ok(())
    }

    /// Show cursor
    fn show_cursor(&mut self) -> Result<(), String> {
        if let Some(window) = &self.window {
            window.set_cursor_visible(true);
        }
        Ok(())
    }

    /// Set cursor position
    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        Ok(())
    }

    /// Get cursor position
    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok((0, 0))
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    /// Override render buffer to texture method, directly use our OpenGL renderer
    ///
    /// This method is specifically implemented for WinitGlowAdapter, does not rely on the unified pixel_renderer abstraction
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

            // Directly call our GlPixelRenderer method
            if let Err(e) = gl_pixel_renderer
                .render_buffer_to_texture_self_contained(rbuf, rtidx, debug, ratio_x, ratio_y)
            {
                eprintln!(
                    "WinitGlowAdapter: Failed to render buffer to texture {}: {}",
                    rtidx, e
                );
            }
        } else {
            eprintln!("WinitGlowAdapter: gl_pixel_renderer not initialized");
        }
    }

    /// Override render texture to screen method, directly use our OpenGL renderer
    ///
    /// This method is specifically implemented for WinitGlowAdapter, handles final composition of transition effects
    fn draw_render_textures_to_screen(&mut self)
    where
        Self: Sized,
    {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            let ratio_x = self.base.gr.ratio_x;
            let ratio_y = self.base.gr.ratio_y;

            // Get physical window size for Retina display support
            let physical_size = if let Some(window) = &self.window {
                Some(window.inner_size())
            } else {
                None
            };

            // Bind screen and set correct viewport
            if let Some(physical_size) = physical_size {
                gl_pixel_renderer.bind_screen_with_viewport(
                    physical_size.width as i32,
                    physical_size.height as i32,
                );
            } else {
                // Fallback: use standard binding
                gl_pixel_renderer
                    .gl_pixel
                    .bind_screen(&gl_pixel_renderer.gl);
            }

            // Clear screen
            use glow::HasContext;
            let gl = gl_pixel_renderer.get_gl();
            unsafe {
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }

            // Directly call our rendering method, no need to bind screen
            if let Err(e) = gl_pixel_renderer.render_textures_to_screen_no_bind(ratio_x, ratio_y) {
                eprintln!(
                    "WinitGlowAdapter: Failed to render textures to screen: {}",
                    e
                );
            }
        } else {
            eprintln!("WinitGlowAdapter: gl_pixel_renderer not initialized for texture rendering");
        }
    }

    /// WinitGlow adapter implementation of render texture visibility control
    fn set_render_texture_visible(&mut self, texture_index: usize, visible: bool) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer
                .get_gl_pixel_mut()
                .set_render_texture_hidden(texture_index, !visible);
        }
    }

    /// WinitGlow adapter implementation of simple transition rendering
    fn render_simple_transition(&mut self, target_texture: usize) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.render_normal_transition(target_texture);
        }
    }

    /// WinitGlow adapter implementation of advanced transition rendering
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

    /// WinitGlow adapter implementation of buffer transition setup
    fn setup_buffer_transition(&mut self, target_texture: usize) {
        if let Some(gl_pixel_renderer) = &mut self.gl_pixel_renderer {
            gl_pixel_renderer.setup_transbuf_rendering(target_texture);
        }
    }
}
