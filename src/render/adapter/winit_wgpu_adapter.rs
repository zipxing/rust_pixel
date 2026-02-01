// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # Winit + WGPU Adapter Implementation
//!
//! Modern cross-platform rendering adapter based on winit + wgpu technology stack.
//!
//! ## Technology Stack
//! - **winit**: Cross-platform window management and event handling
//! - **wgpu**: Modern GPU API abstraction layer (based on Vulkan, Metal, D3D12, WebGPU)
//!
//! ## Features
//! - Cross-platform window management (Windows, macOS, Linux)
//! - High DPI/Retina display support
//! - Custom mouse cursor
//! - Window drag functionality
//! - Keyboard and mouse event handling
//! - Modern GPU hardware-accelerated rendering
//! - Command buffer and asynchronous rendering
//!
//! ## Architecture Design
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │            WinitWgpuAdapter                 │
//! ├─────────────────────────────────────────────┤
//! │  Window Management  │  WGPU Resources       │
//! │  - winit::Window    │  - wgpu::Device       │
//! │  - Event handling   │  - wgpu::Queue        │
//! │  - Cursor support   │  - wgpu::Surface      │
//! └─────────────────────────────────────────────┘
//! ```

use crate::event::Event;
use crate::render::{
    adapter::{
        winit_common::{
            input_events_from_winit, winit_init_common, winit_move_win, Drag, WindowInitParams,
        },
        Adapter, AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
    },
    buffer::Buffer,
    graph::{UnifiedColor, UnifiedTransform},
    sprite::Layer,
};

// WGPU backend imports
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
/// Defines the type of mouse click area to determine if a drag operation should be triggered.
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
/// Encapsulates all components of winit window management and modern rendering backend.
/// Supports two rendering backends: OpenGL (glow) and modern GPU API (wgpu).
/// Implements the same interface as the SDL adapter, allowing seamless replacement.

/// Winit + WGPU adapter main structure
///
/// Encapsulates all components of winit window management and the WGPU modern rendering backend.
/// Specifically designed for the WGPU API, separated from WinitGlowAdapter.
pub struct WinitWgpuAdapter {
    /// Base adapter data
    pub base: AdapterBase,

    // Winit related objects
    /// Window instance
    pub window: Option<Arc<Window>>,
    /// Event loop
    pub event_loop: Option<EventLoop<()>>,
    /// Window initialization parameters (for creating window in resumed)
    pub window_init_params: Option<WindowInitParams>,

    // WGPU backend objects
    /// WGPU instance for creating devices and surfaces
    pub wgpu_instance: Option<wgpu::Instance>,
    /// WGPU device for creating resources
    pub wgpu_device: Option<wgpu::Device>,
    /// WGPU queue for submitting commands
    pub wgpu_queue: Option<wgpu::Queue>,
    /// Window surface for rendering
    pub wgpu_surface: Option<wgpu::Surface<'static>>,
    /// Surface configuration
    pub wgpu_surface_config: Option<wgpu::SurfaceConfiguration>,
    /// Main pixel renderer
    pub wgpu_pixel_renderer: Option<WgpuPixelRender>,

    /// Whether the program should exit
    pub should_exit: bool,

    /// Event handler (for pump_events mode)
    pub app_handler: Option<WinitWgpuAppHandler>,

    /// Custom mouse cursor
    pub custom_cursor: Option<CustomCursor>,

    /// Whether cursor has been set (delayed set flag)
    pub cursor_set: bool,

    /// Window drag data
    drag: Drag,
}

/// Winit + WGPU application event handler
///
/// Implements the winit ApplicationHandler trait, handling window events and user input.
/// Specifically designed for the WGPU adapter.
pub struct WinitWgpuAppHandler {
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
    /// It must be ensured that the safety is maintained when used.
    pub adapter_ref: *mut WinitWgpuAdapter,
}

impl ApplicationHandler for WinitWgpuAppHandler {
    /// Callback when the application resumes
    ///
    /// Creates a window and rendering resources in the resumed event.
    /// This is a unified lifecycle management approach, applicable to both rendering backends.
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Create window and rendering context here
        if let Some(adapter) = unsafe { self.adapter_ref.as_mut() } {
            if adapter.window.is_none() {
                adapter.create_wgpu_window_and_resources(event_loop);
            }

            // Delay cursor setting - after window is fully initialized
            if !adapter.cursor_set {
                // Clear screen before setting cursor - may help with transparency issues
                adapter.clear_screen_wgpu();

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

                // Convert keyboard events to RustPixel events
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
                                    // Start dragging when left button is pressed in border area
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

impl WinitWgpuAdapter {
    /// Create a new Winit adapter instance
    ///
    /// # Returns
    /// Returns the initialized WinitAdapter instance, all GPU components are None,
    /// and need to be used normally after calling the init() method.
    /// 返回初始化的 WinitAdapter 实例，所有 GPU 组件都是 None，
    /// 需要调用 init() 方法后才能正常使用。
    pub fn new() -> Self {
        Self {
            base: AdapterBase::new(),
            window: None,
            event_loop: None,
            window_init_params: None,

            // WGPU backend fields (only when wgpu is enabled)
            wgpu_instance: None,
            wgpu_device: None,
            wgpu_queue: None,
            wgpu_surface: None,
            wgpu_surface_config: None,
            wgpu_pixel_renderer: None,

            should_exit: false,
            app_handler: None,
            custom_cursor: None,
            cursor_set: false,
            drag: Default::default(),
        }
    }

    /// Common initialization method - handles all public logic
    ///
    /// This method handles initialization steps required by both rendering backends:
    /// 1. Load texture files and set symbol dimensions
    /// 2. Set base parameters (size, ratio, pixel size)
    /// 3. Create event loop
    /// 4. Set application handler
    /// 5. Store window initialization parameters
    ///
    /// # Parameters
    /// - `w`: Logical width (characters)
    /// - `h`: Logical height (characters)
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

        // Set WGPU specific application handler
        self.app_handler = Some(WinitWgpuAppHandler {
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
            ratio_x: self.base.gr.ratio_x,
            ratio_y: self.base.gr.ratio_y,
            use_tui_height: self.base.gr.use_tui_height,
            should_exit: false,
            adapter_ref: self as *mut WinitWgpuAdapter,
        });

        texture_path
    }

    /// WGPU backend initialization
    ///
    /// Uses unified lifecycle management, window creation deferred to resumed event.
    fn init_wgpu(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing Winit adapter with WGPU backend...");
        let _texture_path = self.init_common(w, h, rx, ry, title);
        // Window creation will be completed in the resumed event
    }

    /// Create WGPU window and related resources in the resumed event
    fn create_wgpu_window_and_resources(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let params = self.window_init_params.as_ref().unwrap().clone();

        info!("Creating WGPU window and resources...");

        // Calculate window size (handle Retina displays)
        let window_size = LogicalSize::new(self.base.gr.pixel_w, self.base.gr.pixel_h);

        let window_attributes = winit::window::Window::default_attributes()
            .with_title(&params.title)
            .with_inner_size(window_size)
            .with_decorations(true) // Use OS window decoration (title bar, etc.)
            .with_resizable(false);

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        let physical_size = window.inner_size();
        info!(
            "Window created - logical: {}x{}, physical: {}x{}",
            self.base.gr.pixel_w, self.base.gr.pixel_h, physical_size.width, physical_size.height
        );

        // Initialize WGPU core components
        let wgpu_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        self.window = Some(window.clone());

        // Create window surface
        let wgpu_surface = unsafe {
            wgpu_instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(&**self.window.as_ref().unwrap())
                        .unwrap(),
                )
                .expect("Failed to create surface")
        };

        // Asynchronously get adapter and device
        let (wgpu_device, wgpu_queue, wgpu_surface_config) = pollster::block_on(async {
            let adapter = wgpu_instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&wgpu_surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("Failed to find suitable WGPU adapter");

            info!("WGPU adapter found: {:?}", adapter.get_info());

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("RustPixel WGPU Device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: wgpu::MemoryHints::Performance,
                        ..Default::default()
                    },
                )
                .await
                .expect("Failed to create WGPU device");

            let surface_caps = wgpu_surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(surface_caps.formats[0]);

            let surface_config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: physical_size.width,
                height: physical_size.height,
                present_mode: surface_caps.present_modes[0],
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            wgpu_surface.configure(&device, &surface_config);

            info!(
                "WGPU surface configured: {}x{}, format: {:?}",
                surface_config.width, surface_config.height, surface_config.format
            );

            (device, queue, surface_config)
        });

        // Create and initialize WGPU pixel renderer
        let mut wgpu_pixel_renderer = WgpuPixelRender::new_with_format(
            self.base.gr.pixel_w,
            self.base.gr.pixel_h,
            wgpu_surface_config.format,
        );

        // Initialize all WGPU components using pre-loaded texture data
        let tex_data = crate::get_pixel_texture_data();
        if let Err(e) = wgpu_pixel_renderer.load_symbol_texture_from_data(
            &wgpu_device,
            &wgpu_queue,
            tex_data.width,
            tex_data.height,
            &tex_data.data,
        ) {
            panic!("Failed to load symbol texture: {}", e);
        }

        wgpu_pixel_renderer.create_shader(&wgpu_device);
        wgpu_pixel_renderer.create_buffer(&wgpu_device);
        wgpu_pixel_renderer.create_bind_group(&wgpu_device);

        if let Err(e) = wgpu_pixel_renderer.init_render_textures(&wgpu_device) {
            panic!("Failed to initialize render textures: {}", e);
        }

        wgpu_pixel_renderer.init_general2d_renderer(&wgpu_device);
        wgpu_pixel_renderer.init_transition_renderer(&wgpu_device);
        wgpu_pixel_renderer.set_ratio(self.base.gr.ratio_x, self.base.gr.ratio_y);

        // Store all WGPU objects
        self.wgpu_instance = Some(wgpu_instance);
        self.wgpu_device = Some(wgpu_device);
        self.wgpu_queue = Some(wgpu_queue);
        self.wgpu_surface = Some(wgpu_surface);
        self.wgpu_surface_config = Some(wgpu_surface_config);
        self.wgpu_pixel_renderer = Some(wgpu_pixel_renderer);

        info!("WGPU window & context initialized successfully");
    }

    /// Execute initial clear screen operation
    ///
    /// Prevents white screen flicker when creating a window, immediately clears the screen and displays a black background.
    ///
    /// WGPU mode does not require initial_clear_screen because:
    /// - Better resource management: WGPU's Surface configuration mechanism ensures a good initial state
    /// - Deferred rendering: Command buffer mode allows us to prepare everything before present
    /// - Default Clear behavior: RenderPass has a clear operation by default
    /// - Atomic operations: The entire rendering process is atomic, either fully completed or not displayed
    ///
    /// This is one of the advantages of modern graphics APIs (Vulkan, Metal, D3D12) compared to traditional OpenGL: better resource management and fewer unexpected behaviors.
    fn clear_screen_wgpu(&mut self) {
        if let (Some(device), Some(queue), Some(surface)) =
            (&self.wgpu_device, &self.wgpu_queue, &self.wgpu_surface)
        {
            if let Ok(output) = surface.get_current_texture() {
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Cursor Clear Screen Encoder"),
                });

                {
                    let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Cursor Clear Screen Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    // clear_pass automatically drops
                }

                queue.submit(std::iter::once(encoder.finish()));
                output.present();
            }
        }
    }

    /// Set custom mouse cursor
    ///
    /// Loads the cursor image from assets/pix/cursor.png and sets it as the window's custom cursor.
    /// - Automatically converted to RGBA8 format
    /// - Hotspot position set to (0, 0)
    /// - Handles transparency and pre-multiplied alpha
    fn set_mouse_cursor(&mut self) {
        // Build cursor image file path using global GAME_CONFIG
        // 使用全局 GAME_CONFIG 构建光标图像文件路径
        let project_path = &crate::get_game_config().project_path;
        let cursor_path = format!(
            "{}{}{}",
            project_path,
            std::path::MAIN_SEPARATOR,
            "assets/pix/cursor.png"
        );

        if let Ok(cursor_img) = image::open(&cursor_path) {
            let cursor_rgba = cursor_img.to_rgba8();
            let (width, height) = cursor_rgba.dimensions();
            let mut cursor_data = cursor_rgba.into_raw();

            // Pre-multiply alpha - this is a common method to solve cursor transparency issues
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
    /// Used to determine the type of border area for the mouse click position, deciding whether to trigger a drag operation.
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

    /// WGPU version of transition rendering to texture (provides advanced API for petview etc.)
    pub fn render_transition_to_texture_wgpu(
        &mut self,
        src_texture1_idx: usize,
        src_texture2_idx: usize,
        target_texture_idx: usize,
        shader_idx: usize,
        progress: f32,
    ) -> Result<(), String> {
        if let (Some(device), Some(queue), Some(pixel_renderer)) = (
            &self.wgpu_device,
            &self.wgpu_queue,
            &mut self.wgpu_pixel_renderer,
        ) {
            // Create command encoder
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Transition Render Encoder"),
            });

            // Use the new render_trans_frame_to_texture method to avoid borrowing conflicts
            pixel_renderer.render_trans_frame_to_texture(
                device,
                queue,
                &mut encoder,
                src_texture1_idx,
                src_texture2_idx,
                target_texture_idx,
                shader_idx,
                progress,
            )?;

            // Submit commands
            queue.submit(std::iter::once(encoder.finish()));
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }

    /// WGPU version of render buffer to texture rendering method
    ///
    /// This method implements the same interface as the OpenGL version, rendering RenderCell data to the specified render texture.
    /// Used to unify the interfaces of the two rendering backends.
    ///
    /// # Parameters
    /// - `rbuf`: RenderCell data array
    /// - `rtidx`: Target render texture index
    /// - `debug`: Whether to enable debug mode
    pub fn rbuf2rt_wgpu(
        &mut self,
        rbuf: &[crate::render::adapter::RenderCell],
        rtidx: usize,
        debug: bool,
    ) -> Result<(), String> {
        if let (Some(device), Some(queue), Some(pixel_renderer)) = (
            &self.wgpu_device,
            &self.wgpu_queue,
            &mut self.wgpu_pixel_renderer,
        ) {
            // Use unified WgpuPixelRender wrapper to match OpenGL version interface
            let rx = self.base.gr.ratio_x;
            let ry = self.base.gr.ratio_y;

            // Bind target render texture
            pixel_renderer.bind_target(rtidx);

            // Set clear color
            if debug {
                // Debug mode uses red background
                pixel_renderer.set_clear_color(UnifiedColor::new(1.0, 0.0, 0.0, 1.0));
            } else {
                // Normal mode uses black background
                pixel_renderer.set_clear_color(UnifiedColor::new(0.0, 0.0, 0.0, 1.0));
            }

            // Clear target
            pixel_renderer.clear();

            // Render RenderCell data to the currently bound target
            pixel_renderer.render_rbuf(device, queue, rbuf, rx, ry);

            // Create command encoder for rendering to texture
            let mut rt_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(&format!("Render to RT{} Encoder", rtidx)),
            });

            // Execute rendering to the currently bound target
            pixel_renderer.render_to_current_target(&mut rt_encoder, None)?;

            // Submit rendering to texture commands
            queue.submit(std::iter::once(rt_encoder.finish()));
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }

    /// WGPU version of render texture to screen rendering (internal implementation)
    ///
    /// This is the internal implementation of the WGPU version of draw_render_textures_to_screen, corresponding to the OpenGL version.
    /// Responsible for final composition of content from render textures onto the screen, supporting transition effects.
    ///
    /// Correct WGPU rendering process:
    /// 1. Render RenderCell data to render texture 2 (main buffer)
    /// 2. Synthesize render texture 2 onto the screen (if not hidden)
    /// 3. Synthesize render texture 3 onto the screen (if not hidden, for transition effects)
    pub fn draw_render_textures_to_screen_wgpu(&mut self) -> Result<(), String> {
        if let (Some(device), Some(queue), Some(surface), Some(pixel_renderer)) = (
            &self.wgpu_device,
            &self.wgpu_queue,
            &self.wgpu_surface,
            &mut self.wgpu_pixel_renderer,
        ) {
            // Get current surface texture
            let output = surface
                .get_current_texture()
                .map_err(|e| format!("Failed to acquire next swap chain texture: {}", e))?;

            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Use unified WgpuPixelRender wrapper to match OpenGL version interface
            // Bind screen as render target
            pixel_renderer.bind_screen();

            // Create command encoder for screen composition
            let mut screen_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Screen Composition Encoder"),
                });

            // Clear screen
            {
                let _clear_pass = screen_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Clear Screen Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                // clear_pass automatically drops
            }

            // Draw render texture 2 (main buffer) to screen - using low-level WGPU method
            if !pixel_renderer.get_render_texture_hidden(2) {
                let unified_transform = UnifiedTransform::new();
                let unified_color = UnifiedColor::white();

                pixel_renderer.render_texture_to_screen_impl(
                    device,
                    queue,
                    &mut screen_encoder,
                    &view,
                    2,                    // render texture 2
                    [0.0, 0.0, 1.0, 1.0], // Full screen area
                    &unified_transform,
                    &unified_color,
                )?;
            }

            // Draw render texture 3 (transition effect) to screen - using unified interface
            if !pixel_renderer.get_render_texture_hidden(3) {
                let pcw = pixel_renderer.canvas_width as f32;
                let pch = pixel_renderer.canvas_height as f32;
                let rx = self.base.gr.ratio_x;
                let ry = self.base.gr.ratio_y;

                // Use actual game area dimensions (matching OpenGL version)
                let pw = 40.0f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                let ph = 25.0f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

                let mut unified_transform = UnifiedTransform::new();
                unified_transform.scale(pw / pcw, ph / pch);
                let unified_color = UnifiedColor::white();

                pixel_renderer.render_texture_to_screen_impl(
                    device,
                    queue,
                    &mut screen_encoder,
                    &view,
                    3,                                          // render texture 3
                    [0.0 / pcw, 0.0 / pch, pw / pcw, ph / pch], // Game area, WGPU Y-axis starts from top
                    &unified_transform,
                    &unified_color,
                )?;
            }

            // Submit screen composition commands and present frame
            queue.submit(std::iter::once(screen_encoder.finish()));
            output.present();
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }

    /// Present render textures to screen using RtComposite chain (WGPU implementation)
    ///
    /// This is the new unified API for presenting RTs to the screen.
    /// Each RtComposite specifies which RT to draw, viewport, blend mode, and alpha.
    pub fn present_wgpu(
        &mut self,
        composites: &[crate::render::adapter::RtComposite],
    ) -> Result<(), String> {
        // Get ratio values before borrowing self.wgpu_* fields
        let rx = self.base.gr.ratio_x;
        let ry = self.base.gr.ratio_y;

        if let (Some(device), Some(queue), Some(surface), Some(pixel_renderer)) = (
            &self.wgpu_device,
            &self.wgpu_queue,
            &self.wgpu_surface,
            &mut self.wgpu_pixel_renderer,
        ) {
            // Get current surface texture
            let output = surface
                .get_current_texture()
                .map_err(|e| format!("Failed to acquire next swap chain texture: {}", e))?;

            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Bind screen as render target
            pixel_renderer.bind_screen();

            // Create command encoder for screen composition
            let mut screen_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Present Encoder"),
                });

            // Clear screen
            {
                let _clear_pass = screen_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Clear Screen Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
            }

            let pcw = pixel_renderer.canvas_width as f32;
            let pch = pixel_renderer.canvas_height as f32;

            // Render each composite in order
            for composite in composites {
                let rtidx = composite.rt;

                // Skip hidden RTs
                if pixel_renderer.get_render_texture_hidden(rtidx) {
                    continue;
                }

                // Calculate area and transform based on viewport
                let (area, transform) = if let Some(ref vp) = composite.viewport {
                    // Use viewport values from caller (calculated with current ratio)
                    // Note: Using ARect fields (w, h) instead of Rect (width, height)
                    let pw = vp.w as f32;
                    let ph = vp.h as f32;

                    // area controls TEXTURE SAMPLING (WGPU uses top-left origin)
                    let area = [0.0, 0.0, pw / pcw, ph / pch];

                    let mut transform = UnifiedTransform::new();
                    transform.scale(pw / pcw, ph / pch);

                    (area, transform)
                } else {
                    // Fullscreen - sample entire texture, identity transform
                    let area = [0.0, 0.0, 1.0, 1.0];
                    let transform = UnifiedTransform::new();
                    (area, transform)
                };

                // Create color with alpha
                let alpha_f = composite.alpha as f32 / 255.0;
                let color = UnifiedColor::new(1.0, 1.0, 1.0, alpha_f);

                // Render this RT to screen
                pixel_renderer.render_texture_to_screen_impl(
                    device,
                    queue,
                    &mut screen_encoder,
                    &view,
                    rtidx,
                    area,
                    &transform,
                    &color,
                )?;
            }

            // Submit screen composition commands and present frame
            queue.submit(std::iter::once(screen_encoder.finish()));
            output.present();
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }

    /// Debug method: save render texture as PNG image file
    ///
    /// This method saves the specified render texture as a PNG file for debugging rendering issues
    pub fn debug_save_render_texture_to_file(
        &mut self,
        rt_index: usize,
        filename: &str,
    ) -> Result<(), String> {
        if let (Some(device), Some(queue), Some(pixel_renderer)) = (
            &self.wgpu_device,
            &self.wgpu_queue,
            &mut self.wgpu_pixel_renderer,
        ) {
            info!("Saving render texture {} to {}", rt_index, filename);

            // Get render texture
            let render_texture = pixel_renderer
                .get_render_texture(rt_index)
                .ok_or_else(|| format!("Render texture {} not found", rt_index))?;

            let texture_width = render_texture.width;
            let texture_height = render_texture.height;
            let bytes_per_pixel = 4; // RGBA
            let unpadded_bytes_per_row = texture_width * bytes_per_pixel;

            // WGPU requires bytes_per_row to be a multiple of COPY_BYTES_PER_ROW_ALIGNMENT (256)
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
            let buffer_size = (bytes_per_row * texture_height) as u64;

            info!(
                "Texture copy info: {}x{}, unpadded: {}, aligned: {}, buffer_size: {}",
                texture_width, texture_height, unpadded_bytes_per_row, bytes_per_row, buffer_size
            );

            // Create staging buffer for reading texture data
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Render Texture Staging Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            // Create command encoder
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Texture Copy Encoder"),
            });

            // Copy texture to buffer
            encoder.copy_texture_to_buffer(
                render_texture.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &staging_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(texture_height),
                    },
                },
                wgpu::Extent3d {
                    width: texture_width,
                    height: texture_height,
                    depth_or_array_layers: 1,
                },
            );

            // Submit commands
            queue.submit(std::iter::once(encoder.finish()));

            // Map buffer and read data (asynchronous operation)
            let buffer_slice = staging_buffer.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });

            // Wait for mapping to complete
            // device.poll() is no longer needed in newer wgpu versions

            match receiver.recv() {
                Ok(Ok(())) => {
                    // Read data
                    let data = buffer_slice.get_mapped_range();
                    let mut rgba_data = vec![0u8; (texture_width * texture_height * 4) as usize];

                    // Copy data (handling padding)
                    for y in 0..texture_height {
                        let src_start = (y * bytes_per_row) as usize;
                        let dst_start = (y * texture_width * 4) as usize;
                        let row_size = (texture_width * 4) as usize;

                        // Only copy actual pixel data, skipping padding
                        rgba_data[dst_start..dst_start + row_size]
                            .copy_from_slice(&data[src_start..src_start + row_size]);
                    }

                    // Unmap
                    drop(data);
                    staging_buffer.unmap();

                    // Save as PNG file
                    match image::save_buffer(
                        filename,
                        &rgba_data,
                        texture_width,
                        texture_height,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(()) => {
                            info!(
                                "Successfully saved render texture {} to {}",
                                rt_index, filename
                            );
                            Ok(())
                        }
                        Err(e) => Err(format!("Failed to save image: {}", e)),
                    }
                }
                Ok(Err(e)) => Err(format!("Failed to map buffer: {:?}", e)),
                Err(e) => Err(format!("Failed to receive mapping result: {}", e)),
            }
        } else {
            Err("WGPU components not initialized".to_string())
        }
    }

    /// Debug method: print current render state information
    pub fn debug_print_render_info(&self) {
        info!("=== WGPU Render State Information ===");

        // Base parameters
        info!("Base Parameters:");
        info!("  Cell count: {}x{}", self.base.cell_w, self.base.cell_h);
        info!(
            "  Window pixel size: {}x{}",
            self.base.gr.pixel_w, self.base.gr.pixel_h
        );
        info!(
            "  Ratio: {:.3}x{:.3}",
            self.base.gr.ratio_x, self.base.gr.ratio_y
        );

        // Symbol dimensions
        if let (Some(sym_w), Some(sym_h)) = (PIXEL_SYM_WIDTH.get(), PIXEL_SYM_HEIGHT.get()) {
            info!("  Symbol dimensions: {}x{}", sym_w, sym_h);

            // Calculate game area
            let game_area_w = self.base.cell_w as f32 * sym_w / self.base.gr.ratio_x;
            let game_area_h = self.base.cell_h as f32 * sym_h / self.base.gr.ratio_y;
            info!("  Game area: {:.2}x{:.2}", game_area_w, game_area_h);
        }

        // WGPU state
        if let Some(pixel_renderer) = &self.wgpu_pixel_renderer {
            info!("WGPU State:");
            info!(
                "  Canvas size: {}x{}",
                pixel_renderer.canvas_width, pixel_renderer.canvas_height
            );

            // Render texture state
            for i in 0..4 {
                let hidden = pixel_renderer.get_render_texture_hidden(i);
                info!(
                    "  RenderTexture{}: {}",
                    i,
                    if hidden { "Hidden" } else { "Visible" }
                );
            }
        }

        info!("================================");
    }
}

impl Adapter for WinitWgpuAdapter {
    /// Initialize Winit + WGPU adapter
    ///
    /// This is the main initialization method for the adapter, specifically using the WGPU modern rendering pipeline.
    ///
    /// # Parameters
    /// - `w`: Logical width (characters)
    /// - `h`: Logical height (characters)
    /// - `rx`: X-axis scaling ratio
    /// - `ry`: Y-axis scaling ratio
    /// - `title`: Window title
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing WinitWgpu adapter with WGPU backend...");
        self.init_wgpu(w, h, rx, ry, title);
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
    /// Returns true if the program should exit
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

        // Frame rate control - sleep for remaining time to avoid CPU busy loop
        // This matches SDL adapter behavior and reduces CPU usage significantly
        std::thread::sleep(timeout);

        // Return exit status
        self.should_exit
    }

    /// Draw a frame to the screen
    ///
    /// Draws the current frame using the WGPU modern rendering pipeline.
    ///
    /// # Parameters
    /// - `current_buffer`: Current frame buffer
    /// - `previous_buffer`: Previous frame buffer
    /// - `pixel_sprites`: List of pixel sprites
    /// - `stage`: Render stage
    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Layer>,
        stage: u32,
    ) -> Result<(), String> {
        // Handle window drag movement
        winit_move_win(
            &mut self.drag.need,
            self.window.as_ref().map(|v| &**v),
            self.drag.dx,
            self.drag.dy,
        );

        // Use unified graphics rendering process - consistent with SdlAdapter and WinitGlowAdapter
        self.draw_all_graph(current_buffer, previous_buffer, pixel_sprites, stage);

        self.post_draw();
        Ok(())
    }

    fn post_draw(&mut self) {
        // WGPU mode does not require explicit buffer swap, draw_render_textures_to_screen_wgpu already called present()
        if let Some(window) = &self.window {
            window.as_ref().request_redraw();
        }
    }

    /// Hide cursor
    ///
    /// In a graphical application, we do not want to hide the mouse cursor.
    /// This is similar to the SDL version - keep the mouse cursor visible.
    ///
    /// # Design considerations
    /// Maintain consistency with SDL adapter, in fact, no hiding operation is performed.
    fn hide_cursor(&mut self) -> Result<(), String> {
        // For GUI applications, we do not want to hide the mouse cursor
        // This is similar to SDL behavior - keep the mouse cursor visible
        Ok(())
    }

    /// Show cursor
    ///
    /// Ensure the mouse cursor is visible. If the window exists, explicitly set cursor visibility.
    fn show_cursor(&mut self) -> Result<(), String> {
        if let Some(window) = &self.window {
            window.set_cursor_visible(true);
        }
        Ok(())
    }

    /// Set cursor position
    ///
    /// In Winit, cursor position is usually managed by the system, this method is kept for compatibility.
    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        Ok(())
    }

    /// Get cursor position
    ///
    /// Returns the current cursor position. In the Winit implementation, a fixed value is returned for compatibility.
    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok((0, 0))
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    /// Override render buffer to texture method, directly use our WGPU renderer
    ///
    /// This method is specifically implemented for WinitWgpuAdapter, does not rely on the unified pixel_renderer abstraction
    fn rbuf2rt(
        &mut self,
        rbuf: &[crate::render::adapter::RenderCell],
        rtidx: usize,
        debug: bool,
    ) where
        Self: Sized,
    {
        // Directly call our WGPU rendering method
        if let Err(e) = self.rbuf2rt_wgpu(rbuf, rtidx, debug) {
            eprintln!(
                "WinitWgpuAdapter: Failed to render buffer to texture {}: {}",
                rtidx, e
            );
        }
    }

    /// WinitWgpu adapter implementation of render texture visibility control
    fn set_rt_visible(&mut self, texture_index: usize, visible: bool) {
        if let Some(wgpu_pixel_renderer) = &mut self.wgpu_pixel_renderer {
            wgpu_pixel_renderer.set_render_texture_hidden(texture_index, !visible);
        }
    }

    /// WinitWgpu adapter implementation of advanced transition rendering
    fn blend_rts(
        &mut self,
        src_texture1: usize,
        src_texture2: usize,
        dst_texture: usize,
        effect_type: usize,
        progress: f32,
    ) {
        // WGPU uses full transition rendering API
        if let Err(e) =
            self.render_transition_to_texture_wgpu(src_texture1, src_texture2, dst_texture, effect_type, progress)
        {
            eprintln!("WinitWgpuAdapter: Advanced transition error: {}", e);
        }
    }

    /// WinitWgpu adapter implementation of buffer transition setup
    fn setup_buffer_transition(&mut self, target_texture: usize) {
        if let Some(wgpu_pixel_renderer) = &mut self.wgpu_pixel_renderer {
            // WGPU uses texture visibility to setup buffer transitions
            wgpu_pixel_renderer.set_render_texture_hidden(target_texture, true);
        }
    }

    /// WinitWgpu adapter implementation of render texture copy
    fn copy_rt(&mut self, src_index: usize, dst_index: usize) {
        if let (Some(wgpu_pixel_renderer), Some(device), Some(queue)) = (
            &mut self.wgpu_pixel_renderer,
            &self.wgpu_device,
            &self.wgpu_queue,
        ) {
            wgpu_pixel_renderer.copy_rt(device, queue, src_index, dst_index);
        }
    }

    /// Present render textures to screen using RtComposite chain
    ///
    /// This is the new unified API for presenting RTs to the screen.
    /// Each RtComposite specifies which RT to draw, viewport, blend mode, and alpha.
    fn present(&mut self, composites: &[crate::render::adapter::RtComposite]) {
        if let Err(e) = self.present_wgpu(composites) {
            eprintln!("WinitWgpuAdapter: Failed to present: {}", e);
        }
    }

    /// Present with default settings (RT2 fullscreen, RT3 with game area viewport)
    ///
    /// Uses the original working logic with proper viewport calculation.
    fn present_default(&mut self) {
        // Call the existing working implementation
        if let Err(e) = self.draw_render_textures_to_screen_wgpu() {
            eprintln!("WinitWgpuAdapter: Failed to present_default: {}", e);
        }
    }
}
