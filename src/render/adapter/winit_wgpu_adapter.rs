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
//! ## Letterboxing Configuration
//! Set `ENABLE_LETTERBOXING` to `false` to disable aspect ratio preservation
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
            apply_cursor_to_window, check_border_area, input_events_from_winit,
            load_custom_cursor, winit_init_common, winit_move_win, BorderArea, Drag,
            WindowInitParams,
        },
        Adapter, AdapterBase, 
    },
    buffer::Buffer,
    graph::{UnifiedColor, UnifiedTransform, ENABLE_LETTERBOXING},
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

// Note: BorderArea enum is now defined in winit_common.rs

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
                                BorderArea::TopBar | BorderArea::Other => {
                                    // Start dragging when left button is pressed in border area
                                    adapter.drag.draging = true;
                                    adapter.drag.mouse_x = self.cursor_position.0;
                                    adapter.drag.mouse_y = self.cursor_position.1;
                                }
                                BorderArea::Close => {
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
            .with_resizable(true);

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
    /// Uses shared implementation from winit_common.
    fn set_mouse_cursor(&mut self) {
        if let (Some(window), Some(event_loop)) = (&self.window, &self.event_loop) {
            if let Some(cursor) = load_custom_cursor(event_loop) {
                self.custom_cursor = Some(cursor.clone());
                apply_cursor_to_window(window, &cursor);
            }
        }
    }

    /// Check if mouse position is in border area
    ///
    /// Uses shared implementation from winit_common.
    fn in_border(&self, x: f64, y: f64) -> BorderArea {
        check_border_area(x, y, &self.base)
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
    ///
    /// This method now delegates to `present_wgpu` to avoid code duplication.
    /// Supports aspect ratio preservation with letterboxing when window is resized.
    pub fn draw_render_textures_to_screen_wgpu(&mut self) -> Result<(), String> {
        use crate::render::graph::{RtComposite, UnifiedTransform};
        use crate::util::ARect;

        let rx = self.base.gr.ratio_x;
        let ry = self.base.gr.ratio_y;

        // Get canvas size (logical content size)
        let (cw, ch) = if let Some(pr) = &self.wgpu_pixel_renderer {
            (pr.canvas_width, pr.canvas_height)
        } else {
            return Err("wgpu_pixel_renderer not initialized".to_string());
        };

        // Get physical window size
        let (phys_w, phys_h) = if let Some(window) = &self.window {
            let size = window.inner_size();
            (size.width as f32, size.height as f32)
        } else {
            (cw as f32, ch as f32)
        };

        // Calculate aspect ratio preservation transform for RT2 (if enabled)
        let content_w = cw as f32;
        let content_h = ch as f32;

        let (scale_x, scale_y) = if ENABLE_LETTERBOXING {
            // 等比缩放模式：保持宽高比，留黑边
            let content_aspect = content_w / content_h;
            let window_aspect = phys_w / phys_h;

            if window_aspect > content_aspect {
                // Window is wider - scale by height, letterbox horizontally
                let scale = phys_h / content_h;
                let scaled_w = content_w * scale;
                (scaled_w / phys_w, 1.0)
            } else {
                // Window is taller - scale by width, letterbox vertically
                let scale = phys_w / content_w;
                let scaled_h = content_h * scale;
                (1.0, scaled_h / phys_h)
            }
        } else {
            // 拉伸模式：直接填充整个窗口
            (1.0, 1.0)
        };

        // Create transform for aspect ratio preservation
        let mut rt2_transform = UnifiedTransform::new();
        rt2_transform.scale(scale_x, scale_y);

        // Calculate game area dimensions for RT3
        let pw = (cw as f32 / rx) as u32;
        let ph = (ch as f32 / ry) as u32;

        // Build composites: RT2 with aspect ratio transform + RT3 game area
        let composites = vec![
            RtComposite::fullscreen(2).transform(rt2_transform.clone()),
            RtComposite::with_viewport(3, ARect { x: 0, y: 0, w: pw, h: ph })
                .transform(rt2_transform),
        ];

        self.present_wgpu(&composites)
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
                    let vp_x = vp.x as f32;
                    let vp_y = vp.y as f32;
                    let pw = vp.w as f32;
                    let ph = vp.h as f32;

                    // Get content size for texture sampling (original size before scaling)
                    // If not set, fall back to viewport dimensions
                    let (content_w, content_h) = composite.content_size
                        .map(|(w, h)| (w as f32, h as f32))
                        .unwrap_or((pw, ph));

                    // area controls TEXTURE SAMPLING - sample the content portion of the RT
                    // Uses content_size (original dimensions) for sampling, NOT viewport (scaled)
                    // WGPU uses Vulkan-style coordinates where texture (0,0) is top-left
                    // So for WGPU, we sample from y=0 (top) to y=content_h/pch
                    let area = [0.0, 0.0, content_w / pcw, content_h / pch];

                    // transform controls SCREEN POSITION via scaling and translation
                    // Uses viewport (possibly scaled) for display positioning
                    let mut base_transform = UnifiedTransform::new();
                    base_transform.scale(pw / pcw, ph / pch);

                    // NDC coordinates: -1 to 1, center is (0, 0)
                    // Convert viewport position to NDC offset:
                    let tx = (2.0 * vp_x + pw - pcw) / pcw;
                    let ty = (pch - 2.0 * vp_y - ph) / pch;
                    base_transform.translate(tx, ty);

                    // Step 2: Apply composite transform if specified
                    let final_transform = if let Some(ref user_transform) = composite.transform {
                        // Compose: first base_transform, then user_transform
                        base_transform.compose(user_transform)
                    } else {
                        base_transform
                    };

                    (area, final_transform)
                } else {
                    // Fullscreen - use user transform or identity
                    let area = [0.0, 0.0, 1.0, 1.0];
                    let transform = composite.transform.unwrap_or_else(UnifiedTransform::new);
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
        // Make destination RT visible (matching glow adapter behavior)
        if let Some(pr) = &mut self.wgpu_pixel_renderer {
            pr.set_render_texture_hidden(dst_texture, false);
        }
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
