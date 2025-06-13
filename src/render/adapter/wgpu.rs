// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements an Adapter trait using winit + wgpu for rendering.
//! Similar to SDL adapter but uses modern graphics API.

use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
use crate::render::{
    adapter::{
        init_sym_height, init_sym_width, Adapter, AdapterBase,
        PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
    },
    buffer::Buffer,
    sprite::Sprites,
};
use log::info;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "wgpu")]
use {
    winit::{
        application::ApplicationHandler,
        event::{Event as WinitEvent, WindowEvent, KeyEvent as WinitKeyEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        keyboard::{KeyCode as WinitKeyCode, PhysicalKey},
        window::{Window, WindowId},
    },
    wgpu::{
        Device, Instance, Queue, Surface, SurfaceConfiguration, Adapter as WgpuAdapterTrait,
    },
};

// Data for drag window (similar to SDL version)
#[derive(Default)]
struct Drag {
    need: bool,
    draging: bool,
    mouse_x: f64,
    mouse_y: f64,
    dx: f64,
    dy: f64,
}

pub struct WgpuAdapter {
    pub base: AdapterBase,
    
    // Event handling
    events: Vec<Event>,
    should_exit: bool,
    
    // Window drag data
    drag: Drag,
    
    #[cfg(feature = "wgpu")]
    // Winit objects
    pub event_loop: Option<EventLoop<()>>,
    pub window: Option<Arc<Window>>,
    
    #[cfg(feature = "wgpu")]
    // WGPU objects
    pub instance: Option<Instance>,
    pub surface: Option<Surface<'static>>,
    pub adapter: Option<wgpu::Adapter>,
    pub device: Option<Device>,
    pub queue: Option<Queue>,
    pub config: Option<SurfaceConfiguration>,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl WgpuAdapter {
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            base: AdapterBase::new(gn, project_path),
            events: Vec::new(),
            should_exit: false,
            drag: Default::default(),
            
            #[cfg(feature = "wgpu")]
            event_loop: None,
            #[cfg(feature = "wgpu")]
            window: None,
            #[cfg(feature = "wgpu")]
            instance: None,
            #[cfg(feature = "wgpu")]
            surface: None,
            #[cfg(feature = "wgpu")]
            adapter: None,
            #[cfg(feature = "wgpu")]
            device: None,
            #[cfg(feature = "wgpu")]
            queue: None,
            #[cfg(feature = "wgpu")]
            config: None,
            #[cfg(feature = "wgpu")]
            size: winit::dpi::PhysicalSize::new(800, 600),
        }
    }

    #[cfg(feature = "wgpu")]
    async fn init_wgpu_context(&mut self, window: Arc<Window>) -> Result<(), String> {
        // Create wgpu instance
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("Failed to create surface: {}", e))?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to find an appropriate adapter: {}", e))?;

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::default(),
                },
            )
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.size.width,
            height: self.size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Store everything
        self.window = Some(window);
        self.instance = Some(instance);
        self.surface = Some(surface);
        self.adapter = Some(adapter);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);

        info!("WGPU context initialized successfully");
        Ok(())
    }

    #[cfg(feature = "wgpu")]
    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested => {
                self.should_exit = true;
                true
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // Convert winit mouse events to our Event system
                // TODO: Implement mouse event conversion
                false
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Handle cursor movement for dragging
                // TODO: Implement cursor movement handling
                false
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Convert keyboard events
                if let Some(pixel_event) = self.convert_key_event(event) {
                    self.events.push(pixel_event);
                }
                false
            }
            WindowEvent::Resized(physical_size) => {
                self.size = *physical_size;
                if let (Some(surface), Some(device), Some(config)) = 
                    (&self.surface, &self.device, &mut self.config) {
                    config.width = physical_size.width;
                    config.height = physical_size.height;
                    surface.configure(device, config);
                }
                false
            }
            _ => false,
        }
    }

    #[cfg(feature = "wgpu")]
    fn convert_key_event(&self, event: &WinitKeyEvent) -> Option<Event> {
        if !event.state.is_pressed() {
            return None;
        }

        let key_char = match event.physical_key {
            PhysicalKey::Code(WinitKeyCode::Space) => ' ',
            PhysicalKey::Code(WinitKeyCode::KeyA) => 'a',
            PhysicalKey::Code(WinitKeyCode::KeyB) => 'b',
            PhysicalKey::Code(WinitKeyCode::KeyC) => 'c',
            PhysicalKey::Code(WinitKeyCode::KeyD) => 'd',
            PhysicalKey::Code(WinitKeyCode::KeyE) => 'e',
            PhysicalKey::Code(WinitKeyCode::KeyF) => 'f',
            PhysicalKey::Code(WinitKeyCode::KeyG) => 'g',
            PhysicalKey::Code(WinitKeyCode::KeyH) => 'h',
            PhysicalKey::Code(WinitKeyCode::KeyI) => 'i',
            PhysicalKey::Code(WinitKeyCode::KeyJ) => 'j',
            PhysicalKey::Code(WinitKeyCode::KeyK) => 'k',
            PhysicalKey::Code(WinitKeyCode::KeyL) => 'l',
            PhysicalKey::Code(WinitKeyCode::KeyM) => 'm',
            PhysicalKey::Code(WinitKeyCode::KeyN) => 'n',
            PhysicalKey::Code(WinitKeyCode::KeyO) => 'o',
            PhysicalKey::Code(WinitKeyCode::KeyP) => 'p',
            PhysicalKey::Code(WinitKeyCode::KeyQ) => 'q',
            PhysicalKey::Code(WinitKeyCode::KeyR) => 'r',
            PhysicalKey::Code(WinitKeyCode::KeyS) => 's',
            PhysicalKey::Code(WinitKeyCode::KeyT) => 't',
            PhysicalKey::Code(WinitKeyCode::KeyU) => 'u',
            PhysicalKey::Code(WinitKeyCode::KeyV) => 'v',
            PhysicalKey::Code(WinitKeyCode::KeyW) => 'w',
            PhysicalKey::Code(WinitKeyCode::KeyX) => 'x',
            PhysicalKey::Code(WinitKeyCode::KeyY) => 'y',
            PhysicalKey::Code(WinitKeyCode::KeyZ) => 'z',
            _ => return None,
        };

        let key_event = KeyEvent::new(KeyCode::Char(key_char), KeyModifiers::NONE);
        Some(Event::Key(key_event))
    }
}

impl Adapter for WgpuAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        // Load texture file (use c64.png for testing)
        let texture_path = format!(
            "{}{}{}",
            self.base.project_path,
            std::path::MAIN_SEPARATOR,
            "assets/pix/c64.png"
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
            
        info!("wgpu_pixel load texture...{}", texture_path);
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

        #[cfg(feature = "wgpu")]
        {
            self.size = winit::dpi::PhysicalSize::new(
                self.base.pixel_w,
                self.base.pixel_h,
            );
        }

        info!("WgpuAdapter init completed");
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {
        self.events.clear();
        self.should_exit = false;
    }

    fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH.get().expect("lazylock init") / self.base.ratio_x
    }

    fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT.get().expect("lazylock init") / self.base.ratio_y
    }

    fn poll_event(&mut self, timeout: Duration, es: &mut Vec<Event>) -> bool {
        // For now, just return collected events
        // The actual event loop will be handled differently in winit 0.30
        es.extend(self.events.drain(..));
        
        if self.should_exit {
            return true;
        }

        std::thread::sleep(timeout);
        false
    }

    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        _previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        // TODO: Implement WGPU rendering
        // For now, just use the base rendering logic
        #[cfg(feature = "wgpu")]
        {
            // Basic clear screen for now
            if let (Some(surface), Some(device), Some(queue)) = 
                (&self.surface, &self.device, &self.queue) {
                
                let output = surface.get_current_texture()
                    .map_err(|e| format!("Failed to get surface texture: {}", e))?;
                
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                {
                    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                }

                queue.submit(std::iter::once(encoder.finish()));
                output.present();
            }
        }
        
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), String> {
        #[cfg(feature = "wgpu")]
        if let Some(window) = &self.window {
            window.set_cursor_visible(false);
        }
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), String> {
        #[cfg(feature = "wgpu")]
        if let Some(window) = &self.window {
            window.set_cursor_visible(true);
        }
        Ok(())
    }

    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        // TODO: Implement cursor positioning
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        // TODO: Implement cursor position retrieval
        Ok((0, 0))
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

// Application handler for winit 0.30
#[cfg(feature = "wgpu")]
pub struct WgpuApp {
    adapter: Option<WgpuAdapter>,
}

#[cfg(feature = "wgpu")]
impl WgpuApp {
    pub fn new(mut adapter: WgpuAdapter) -> Self {
        Self {
            adapter: Some(adapter),
        }
    }
}

#[cfg(feature = "wgpu")]
impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(adapter) = &mut self.adapter {
            if adapter.window.is_none() {
                let window_attributes = Window::default_attributes()
                    .with_title(&adapter.base.title)
                    .with_inner_size(adapter.size)
                    .with_decorations(false); // Borderless like SDL version

                match event_loop.create_window(window_attributes) {
                    Ok(window) => {
                        let window = Arc::new(window);
                        
                        // Initialize WGPU context
                        let window_clone = window.clone();
                        let init_result = pollster::block_on(async {
                            adapter.init_wgpu_context(window_clone).await
                        });
                        
                        if let Err(e) = init_result {
                            log::error!("Failed to initialize WGPU: {}", e);
                            event_loop.exit();
                        } else {
                            info!("Window and WGPU context created successfully");
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create window: {}", e);
                        event_loop.exit();
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(adapter) = &mut self.adapter {
            if adapter.handle_window_event(&event) {
                event_loop.exit();
            }
        }
    }
} 