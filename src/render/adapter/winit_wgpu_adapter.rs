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

use crate::event::Event;
use crate::render::{
    adapter::{
        wgpu::{WgpuRenderCore, WgpuRenderCoreBuilder},
        winit_common::{
            apply_cursor_to_window, check_border_area, input_events_from_winit,
            load_custom_cursor, winit_init_common, winit_move_win, BorderArea, Drag,
            WindowInitParams,
        },
        Adapter, AdapterBase, RtComposite,
    },
    buffer::Buffer,
    graph::{UnifiedTransform, is_letterboxing_enabled, set_letterboxing_override},
    sprite::Layer,
};

use log::info;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
pub use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Cursor, CustomCursor, Fullscreen, Window},
};

/// Winit + WGPU adapter main structure
///
/// Encapsulates all components of winit window management and the WGPU modern rendering backend.
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
    /// Window surface for rendering
    pub wgpu_surface: Option<wgpu::Surface<'static>>,
    /// Surface configuration
    pub wgpu_surface_config: Option<wgpu::SurfaceConfiguration>,
    /// Shared render core (contains device, queue, pixel_renderer)
    pub render_core: Option<WgpuRenderCore>,

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

    /// Original ratio values (saved before maximize override)
    original_ratio_x: f32,
    original_ratio_y: f32,
    /// Whether ratio was overridden by maximize
    ratio_overridden: bool,
}

/// Winit + WGPU application event handler
///
/// Implements the winit ApplicationHandler trait, handling window events and user input.
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
    pub adapter_ref: *mut WinitWgpuAdapter,
}

impl ApplicationHandler for WinitWgpuAppHandler {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("[DEBUG resumed] called");
        if let Some(adapter) = unsafe { self.adapter_ref.as_mut() } {
            if adapter.window.is_none() {
                log::info!("[DEBUG resumed] creating window and resources...");
                adapter.create_wgpu_window_and_resources(event_loop);
                log::info!("[DEBUG resumed] window created");
            }

            if !adapter.cursor_set {
                log::info!("[DEBUG resumed] calling clear_screen_wgpu...");
                adapter.clear_screen_wgpu();
                log::info!("[DEBUG resumed] clear_screen_wgpu done, setting cursor...");
                adapter.set_mouse_cursor();
                adapter.cursor_set = true;
            }
        }
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
                if key_event.state == winit::event::ElementState::Pressed {
                    if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key {
                        if keycode == winit::keyboard::KeyCode::KeyQ {
                            self.should_exit = true;
                            event_loop.exit();
                            return;
                        }
                    }
                }

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
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    unsafe {
                        let adapter = &mut *self.adapter_ref;

                        let is_max = adapter.window.as_ref()
                            .map(|w| w.is_maximized()).unwrap_or(false);
                        let is_fullscreen = adapter.window.as_ref()
                            .and_then(|w| w.fullscreen()).is_some();

                        // Reconfigure surface to match new window size
                        if let (Some(surface), Some(config), Some(core)) = (
                            &adapter.wgpu_surface,
                            adapter.wgpu_surface_config.as_mut(),
                            &adapter.render_core,
                        ) {
                            config.width = new_size.width;
                            config.height = new_size.height;
                            surface.configure(&core.device, config);
                        }

                        // On maximize/fullscreen: force ratio=1.0, enable letterboxing,
                        // and rebuild render core for crisp rendering
                        // On restore: restore original ratio and disable letterboxing
                        let should_override = (is_max || is_fullscreen)
                            && !adapter.ratio_overridden
                            && adapter.base.gr.ratio_x > 1.0;

                        if should_override {
                            adapter.original_ratio_x = adapter.base.gr.ratio_x;
                            adapter.original_ratio_y = adapter.base.gr.ratio_y;
                            adapter.base.gr.ratio_x = 1.0;
                            adapter.base.gr.ratio_y = 1.0;
                            adapter.base.gr.set_pixel_size(
                                adapter.base.cell_w, adapter.base.cell_h,
                            );
                            set_letterboxing_override(true);
                            adapter.rebuild_render_core();
                            adapter.ratio_overridden = true;
                        } else if !is_max && !is_fullscreen && adapter.ratio_overridden {
                            adapter.base.gr.ratio_x = adapter.original_ratio_x;
                            adapter.base.gr.ratio_y = adapter.original_ratio_y;
                            adapter.base.gr.set_pixel_size(
                                adapter.base.cell_w, adapter.base.cell_h,
                            );
                            set_letterboxing_override(false);
                            adapter.rebuild_render_core();
                            adapter.ratio_overridden = false;
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                unsafe {
                    let adapter = &*self.adapter_ref;
                    if let Some(window) = &adapter.window {
                        let actual_size = window.inner_size();
                        let expected_w = adapter.base.gr.pixel_w as f64;
                        let expected_h = adapter.base.gr.pixel_h as f64;
                        let phys_w = actual_size.width as f64;
                        let phys_h = actual_size.height as f64;

                        if is_letterboxing_enabled() {
                            // Content is centered with black borders; adjust cursor mapping
                            let content_aspect = expected_w / expected_h;
                            let window_aspect = phys_w / phys_h;

                            if window_aspect > content_aspect {
                                // Black bars on left/right
                                let content_phys_w = phys_h * content_aspect;
                                let offset_x = (phys_w - content_phys_w) / 2.0;
                                self.cursor_position = (
                                    (position.x - offset_x) / content_phys_w * expected_w,
                                    position.y / phys_h * expected_h,
                                );
                            } else {
                                // Black bars on top/bottom
                                let content_phys_h = phys_w / content_aspect;
                                let offset_y = (phys_h - content_phys_h) / 2.0;
                                self.cursor_position = (
                                    position.x / phys_w * expected_w,
                                    (position.y - offset_y) / content_phys_h * expected_h,
                                );
                            }
                        } else {
                            // No letterboxing: content stretches to fill window
                            self.cursor_position = (
                                position.x / phys_w * expected_w,
                                position.y / phys_h * expected_h,
                            );
                        }
                    } else {
                        self.cursor_position = (position.x, position.y);
                    }
                }

                unsafe {
                    let adapter = &mut *self.adapter_ref;
                    if adapter.drag.draging {
                        adapter.drag.need = true;
                        adapter.drag.dx = position.x - adapter.drag.mouse_x;
                        adapter.drag.dy = position.y - adapter.drag.mouse_y;
                    }
                }

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
                    (winit::event::ElementState::Pressed, winit::event::MouseButton::Left) |
                    (winit::event::ElementState::Released, winit::event::MouseButton::Left) => {
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
                    _ => {
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
    pub fn new() -> Self {
        Self {
            base: AdapterBase::new(),
            window: None,
            event_loop: None,
            window_init_params: None,

            wgpu_instance: None,
            wgpu_surface: None,
            wgpu_surface_config: None,
            render_core: None,

            should_exit: false,
            app_handler: None,
            custom_cursor: None,
            cursor_set: false,
            drag: Default::default(),
            original_ratio_x: 1.0,
            original_ratio_y: 1.0,
            ratio_overridden: false,
        }
    }

    fn init_common(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) -> String {
        let (event_loop, window_init_params, texture_path) =
            winit_init_common(self, w, h, rx, ry, title);

        self.event_loop = Some(event_loop);
        self.window_init_params = Some(window_init_params);

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

    fn init_wgpu(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing Winit adapter with WGPU backend...");
        let _texture_path = self.init_common(w, h, rx, ry, title);
    }

    fn create_wgpu_window_and_resources(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let params = self.window_init_params.as_ref().unwrap().clone();

        info!("Creating WGPU window and resources...");

        let window_size = LogicalSize::new(self.base.gr.pixel_w, self.base.gr.pixel_h);

        // Check if fullscreen mode is requested from GameConfig
        let game_config = crate::get_game_config();
        let fullscreen = if game_config.fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };

        let window_attributes = winit::window::Window::default_attributes()
            .with_title(&params.title)
            .with_inner_size(window_size)
            .with_decorations(true)
            .with_resizable(true)
            .with_fullscreen(fullscreen.clone());

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        let physical_size = window.inner_size();

        let wgpu_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        self.window = Some(window.clone());

        let wgpu_surface = unsafe {
            wgpu_instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(&**self.window.as_ref().unwrap())
                        .unwrap(),
                )
                .expect("Failed to create surface")
        };

        let (device, queue, wgpu_surface_config) = pollster::block_on(async {
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

        // Build render core using the shared builder
        let tex_data = crate::get_pixel_texture_data();
        let render_core = WgpuRenderCoreBuilder::new(
            self.base.gr.pixel_w,
            self.base.gr.pixel_h,
            wgpu_surface_config.format,
        )
        .with_ratio(self.base.gr.ratio_x, self.base.gr.ratio_y)
        .build(
            device,
            queue,
            tex_data.width,
            tex_data.height,
            &tex_data.data,
        )
        .expect("Failed to build render core");

        self.wgpu_instance = Some(wgpu_instance);
        self.wgpu_surface = Some(wgpu_surface);
        self.wgpu_surface_config = Some(wgpu_surface_config);
        self.render_core = Some(render_core);
    }

    fn clear_screen_wgpu(&mut self) {
        if let (Some(surface), Some(core)) = (&self.wgpu_surface, &self.render_core) {
            if let Ok(output) = surface.get_current_texture() {
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = core.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Cursor Clear Screen Encoder"),
                });

                {
                    let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Cursor Clear Screen Pass"),
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

                core.queue.submit(std::iter::once(encoder.finish()));
                output.present();
            }
        }
    }

    fn set_mouse_cursor(&mut self) {
        if let (Some(window), Some(event_loop)) = (&self.window, &self.event_loop) {
            if let Some(cursor) = load_custom_cursor(event_loop) {
                self.custom_cursor = Some(cursor.clone());
                apply_cursor_to_window(window, &cursor);
            }
        }
    }

    /// Rebuild render core with current pixel_w/pixel_h and ratio settings.
    /// Extracts device/queue from old core, builds new core, reconfigures surface.
    fn rebuild_render_core(&mut self) {
        if let Some(old_core) = self.render_core.take() {
            let device = old_core.device;
            let queue = old_core.queue;
            let format = self.wgpu_surface_config.as_ref().unwrap().format;

            let tex_data = crate::get_pixel_texture_data();
            let new_core = WgpuRenderCoreBuilder::new(
                self.base.gr.pixel_w,
                self.base.gr.pixel_h,
                format,
            )
            .with_ratio(self.base.gr.ratio_x, self.base.gr.ratio_y)
            .build(device, queue, tex_data.width, tex_data.height, &tex_data.data)
            .expect("Failed to rebuild render core");

            info!(
                "Render core rebuilt: {}x{}, ratio: ({}, {})",
                self.base.gr.pixel_w, self.base.gr.pixel_h,
                self.base.gr.ratio_x, self.base.gr.ratio_y
            );

            self.render_core = Some(new_core);
        }
    }

    #[allow(dead_code)]
    fn in_border(&self, x: f64, y: f64) -> BorderArea {
        check_border_area(x, y, &self.base)
    }

    /// Present render textures to screen with letterboxing support
    fn draw_render_textures_to_screen_wgpu(&mut self) -> Result<(), String> {
        use crate::util::ARect;

        let rx = self.base.gr.ratio_x;
        let ry = self.base.gr.ratio_y;

        let (cw, ch) = if let Some(core) = &self.render_core {
            core.canvas_size()
        } else {
            return Err("render_core not initialized".to_string());
        };

        let (phys_w, phys_h) = if let Some(window) = &self.window {
            let size = window.inner_size();
            (size.width as f32, size.height as f32)
        } else {
            (cw as f32, ch as f32)
        };

        let content_w = cw as f32;
        let content_h = ch as f32;

        let (scale_x, scale_y) = if is_letterboxing_enabled() {
            let content_aspect = content_w / content_h;
            let window_aspect = phys_w / phys_h;

            if window_aspect > content_aspect {
                let scale = phys_h / content_h;
                let scaled_w = content_w * scale;
                (scaled_w / phys_w, 1.0)
            } else {
                let scale = phys_w / content_w;
                let scaled_h = content_h * scale;
                (1.0, scaled_h / phys_h)
            }
        } else {
            (1.0, 1.0)
        };

        let mut rt2_transform = UnifiedTransform::new();
        rt2_transform.scale(scale_x, scale_y);

        let pw = (cw as f32 / rx) as u32;
        let ph = (ch as f32 / ry) as u32;

        // Build composites list - only include RT3 if it's not hidden
        // This fixes the Windows startup issue where RT3 might contain garbage
        // and its hidden state isn't being properly respected
        let mut composites = vec![
            RtComposite::fullscreen(2).transform(rt2_transform.clone()),
        ];

        // Check RT3 hidden state before adding it to composites
        let rt3_hidden = if let Some(core) = &self.render_core {
            core.pixel_renderer.get_render_texture_hidden(3)
        } else {
            true
        };

        if !rt3_hidden {
            composites.push(
                RtComposite::with_viewport(3, ARect { x: 0, y: 0, w: pw, h: ph })
                    .transform(rt2_transform),
            );
        }

        self.present_wgpu(&composites)
    }

    fn present_wgpu(&mut self, composites: &[RtComposite]) -> Result<(), String> {
        if let (Some(surface), Some(core)) = (&self.wgpu_surface, &mut self.render_core) {
            let output = surface
                .get_current_texture()
                .map_err(|e| format!("Failed to acquire next swap chain texture: {}", e))?;

            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            core.present(&view, composites);

            output.present();
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }
}

impl Adapter for WinitWgpuAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing WinitWgpu adapter with WGPU backend...");
        self.init_wgpu(w, h, rx, ry, title);
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn poll_event(&mut self, timeout: Duration, es: &mut Vec<Event>) -> bool {
        if let (Some(event_loop), Some(app_handler)) =
            (self.event_loop.as_mut(), self.app_handler.as_mut())
        {
            let pump_timeout = Some(timeout);
            let status = event_loop.pump_app_events(pump_timeout, app_handler);

            for event in app_handler.pending_events.drain(..) {
                if !self.drag.draging {
                    es.push(event);
                }
            }

            if app_handler.should_exit {
                return true;
            }

            if let PumpStatus::Exit(_) = status {
                return true;
            }
        }

        std::thread::sleep(timeout);

        self.should_exit
    }

    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Layer>,
        stage: u32,
    ) -> Result<(), String> {
        winit_move_win(
            &mut self.drag.need,
            self.window.as_ref().map(|v| &**v),
            self.drag.dx,
            self.drag.dy,
        );

        self.draw_all_graph(current_buffer, previous_buffer, pixel_sprites, stage);

        self.post_draw();
        Ok(())
    }

    fn post_draw(&mut self) {
        if let Some(window) = &self.window {
            window.as_ref().request_redraw();
        }
    }

    fn hide_cursor(&mut self) -> Result<(), String> {
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

    fn rbuf2rt(
        &mut self,
        rbuf: &[crate::render::adapter::RenderCell],
        rtidx: usize,
        debug: bool,
    ) where
        Self: Sized,
    {
        if let Some(core) = &mut self.render_core {
            core.rbuf2rt(rbuf, rtidx, debug);
        } else {
            eprintln!("WinitWgpuAdapter: render core not initialized");
        }
    }

    fn set_rt_visible(&mut self, texture_index: usize, visible: bool) {
        if let Some(core) = &mut self.render_core {
            core.set_rt_visible(texture_index, visible);
        }
    }

    fn blend_rts(
        &mut self,
        src_texture1: usize,
        src_texture2: usize,
        dst_texture: usize,
        effect_type: usize,
        progress: f32,
    ) {
        if let Some(core) = &mut self.render_core {
            core.blend_rts(src_texture1, src_texture2, dst_texture, effect_type, progress);
        }
    }

    fn setup_buffer_transition(&mut self, target_texture: usize) {
        if let Some(core) = &mut self.render_core {
            core.set_rt_visible(target_texture, false);
        }
    }

    fn copy_rt(&mut self, src_index: usize, dst_index: usize) {
        if let Some(core) = &mut self.render_core {
            core.copy_rt(src_index, dst_index);
        }
    }

    fn present(&mut self, composites: &[RtComposite]) {
        if let Err(e) = self.present_wgpu(composites) {
            eprintln!("WinitWgpuAdapter: Failed to present: {}", e);
        }
    }

    fn present_default(&mut self) {
        if let Err(e) = self.draw_render_textures_to_screen_wgpu() {
            eprintln!("WinitWgpuAdapter: Failed to present_default: {}", e);
        }
    }

    fn set_sharpness(&mut self, sharpness: f32) {
        if let Some(core) = &mut self.render_core {
            core.set_sharpness(sharpness);
        }
    }

    fn set_msdf_enabled(&mut self, enabled: bool) {
        if let Some(core) = &mut self.render_core {
            core.set_msdf_enabled(enabled);
        }
    }
}
