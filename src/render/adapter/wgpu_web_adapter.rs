// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # WGPU Web Adapter
//!
//! WGPU-based browser rendering adapter using WebGPU with WebGL2 fallback.
//! This adapter uses the same WGPU rendering pipeline as the desktop adapter
//! but initializes from a canvas element instead of a winit window.

use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
use crate::render::{
    adapter::{
        wgpu::{WgpuRenderCore, WgpuRenderCoreBuilder},
        Adapter, AdapterBase, RtComposite, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
    },
    buffer::Buffer,
    sprite::Layer,
};
use log::info;
use std::any::Any;
use std::time::Duration;
use wasm_bindgen::JsCast;

/// WGPU Web Adapter - uses WGPU with WebGPU/WebGL2 fallback for browser rendering
pub struct WgpuWebAdapter {
    pub base: AdapterBase,

    /// WGPU instance
    pub wgpu_instance: Option<wgpu::Instance>,
    /// Canvas surface for rendering
    pub wgpu_surface: Option<wgpu::Surface<'static>>,
    /// Surface configuration
    pub wgpu_surface_config: Option<wgpu::SurfaceConfiguration>,
    /// Shared render core (contains device, queue, pixel_renderer)
    pub render_core: Option<WgpuRenderCore>,
}

impl WgpuWebAdapter {
    pub fn new() -> Self {
        Self {
            base: AdapterBase::new(),
            wgpu_instance: None,
            wgpu_surface: None,
            wgpu_surface_config: None,
            render_core: None,
        }
    }

    /// Initialize WGPU from cached texture data (called after wasm_init_pixel_assets)
    ///
    /// This performs async WGPU initialization using wasm-bindgen-futures.
    /// Must be called after the adapter.init() to set up the WebGPU/WebGL2 context.
    ///
    /// Note: This is an async function that must be awaited from JavaScript.
    pub async fn init_wgpu_from_cache_async(&mut self) {
        let tex_data = crate::get_pixel_texture_data();

        self.base.gr.set_pixel_size(self.base.cell_w, self.base.cell_h);

        web_sys::console::log_1(
            &format!(
                "RUST: cell_w={}, cell_h={}, pixel_w={}, pixel_h={}, ratio_x={}, ratio_y={}",
                self.base.cell_w, self.base.cell_h,
                self.base.gr.pixel_w, self.base.gr.pixel_h,
                self.base.gr.ratio_x, self.base.gr.ratio_y
            )
            .into(),
        );

        // Get canvas element
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        // Create WGPU instance with WebGL fallback
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..Default::default()
        });

        // Create surface from canvas
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .expect("Failed to create surface from canvas");

        // Use .await for async adapter request (wasm-bindgen-futures handles this)
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find a suitable GPU adapter");

        info!("WGPU Adapter: {:?}", adapter.get_info());

        // Request device (wasm wgpu only takes 1 argument)
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("RustPixel Web Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: Default::default(),
                trace: Default::default(),
            },
        )
        .await
        .expect("Failed to create device");

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.base.gr.pixel_w as u32,
            height: self.base.gr.pixel_h as u32,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Build render core using the shared builder
        let render_core = WgpuRenderCoreBuilder::new(
            self.base.gr.pixel_w as u32,
            self.base.gr.pixel_h as u32,
            surface_format,
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

        // Store everything
        self.wgpu_instance = Some(instance);
        self.wgpu_surface = Some(surface);
        self.wgpu_surface_config = Some(config);
        self.render_core = Some(render_core);

        info!("WGPU Web initialized from cache: {}x{}", tex_data.width, tex_data.height);
    }
}

impl Adapter for WgpuWebAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String) {
        self.set_size(w, h).set_title(s);
        self.base.gr.set_ratiox(rx);
        self.base.gr.set_ratioy(ry);
        info!("WgpuWebAdapter::init - size: {}x{}, ratio: ({}, {})", w, h, rx, ry);
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn get_canvas_size(&self) -> (u32, u32) {
        (self.base.gr.pixel_w, self.base.gr.pixel_h)
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

    fn rbuf2rt(&mut self, rbuf: &[crate::render::adapter::RenderCell], rtidx: usize, debug: bool) {
        if let Some(core) = &mut self.render_core {
            core.rbuf2rt(rbuf, rtidx, debug);
        } else {
            web_sys::console::error_1(&"WgpuWebAdapter: render core not initialized".into());
        }
    }

    fn post_draw(&mut self) {
        // For web, present the surface - handled in present() method
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn set_rt_visible(&mut self, texture_index: usize, visible: bool) {
        if let Some(core) = &mut self.render_core {
            core.set_rt_visible(texture_index, visible);
        }
    }

    fn blend_rts(&mut self, src1: usize, src2: usize, target: usize, effect: usize, progress: f32) {
        if let Some(core) = &mut self.render_core {
            core.blend_rts(src1, src2, target, effect, progress);
        }
    }

    fn setup_buffer_transition(&mut self, _target_texture: usize) {
        // WGPU doesn't need special setup for transitions
    }

    fn copy_rt(&mut self, src_index: usize, dst_index: usize) {
        if let Some(core) = &mut self.render_core {
            core.copy_rt(src_index, dst_index);
        }
    }

    fn present(&mut self, composites: &[RtComposite]) {
        if let (Some(surface), Some(core)) = (&self.wgpu_surface, &mut self.render_core) {
            // Get current surface texture
            let output = match surface.get_current_texture() {
                Ok(output) => output,
                Err(e) => {
                    web_sys::console::error_1(&format!("Surface error: {:?}", e).into());
                    return;
                }
            };
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Use render core to present
            core.present(&view, composites);

            // Present the frame
            output.present();
        }
    }

    fn present_default(&mut self) {
        self.present(&[RtComposite::fullscreen(2)]);
    }

    fn set_sharpness(&mut self, sharpness: f32) {
        if let Some(core) = &mut self.render_core {
            core.set_sharpness(sharpness);
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
/// # Parameters
/// - `t`: Event type identifier
/// - `e`: Web event reference (KeyboardEvent, MouseEvent, etc.)
/// - `pixel_h`: Window pixel height
/// - `ratiox`: X-axis scaling ratio
/// - `ratioy`: Y-axis scaling ratio
/// - `use_tui_height`: If true, uses TUI character height (32px) for mouse coordinate conversion
pub fn input_events_from_web(t: u8, e: web_sys::Event, pixel_h: u32, ratiox: f32, ratioy: f32, use_tui_height: bool) -> Option<Event> {
    let sym_width = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as f32;
    let sym_height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as f32;
    let mut mcte: Option<MouseEvent> = None;

    if let Some(key_e) = wasm_bindgen::JsCast::dyn_ref::<web_sys::KeyboardEvent>(&e) {
        assert!(t == 0);
        let key = key_e.key();
        let key_code = match key.as_str() {
            "ArrowLeft" => Some(KeyCode::Left),
            "ArrowRight" => Some(KeyCode::Right),
            "ArrowUp" => Some(KeyCode::Up),
            "ArrowDown" => Some(KeyCode::Down),
            "PageUp" => Some(KeyCode::PageUp),
            "PageDown" => Some(KeyCode::PageDown),
            "Home" => Some(KeyCode::Home),
            "End" => Some(KeyCode::End),
            "Escape" => Some(KeyCode::Esc),
            "Enter" => Some(KeyCode::Enter),
            "Backspace" => Some(KeyCode::Backspace),
            "Tab" => Some(KeyCode::Tab),
            " " => Some(KeyCode::Char(' ')),
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
        let cell_height = if use_tui_height {
            sym_height * 2.0
        } else {
            sym_height
        };
        mc.column /= (sym_width / ratiox) as u16;
        // Canvas size now matches pixel_h exactly (set dynamically by JS)
        // No offset needed since canvas top is at y=0
        mc.row /= (cell_height / ratioy) as u16;
        return Some(Event::Mouse(mc));
    }
    None
}
