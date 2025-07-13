// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # Winit适配器实现
//!
//! 基于winit + glutin + glow技术栈的跨平台渲染适配器。
//!
//! ## 技术栈
//! - **winit**: 跨平台窗口管理和事件处理
//! - **glutin**: OpenGL上下文管理
//! - **glow**: 现代OpenGL绑定
//!
//! ## 功能特性
//! - 跨平台窗口管理（Windows、macOS、Linux）
//! - 高DPI/Retina显示支持
//! - 自定义鼠标光标
//! - 窗口拖拽功能
//! - 键盘和鼠标事件处理
//! - OpenGL硬件加速渲染
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │             WinitAdapter                    │
//! ├─────────────────────────────────────────────┤
//! │  Window Management  │  OpenGL Context      │
//! │  - winit::Window    │  - glutin::Context   │
//! │  - Event handling   │  - glutin::Surface   │
//! │  - Cursor support   │  - glow::Context     │
//! └─────────────────────────────────────────────┘
//! ```

use crate::event::Event;
use crate::render::{
    adapter::{
        init_sym_height, init_sym_width, Adapter, AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
        PIXEL_TEXTURE_FILE,
    },
    buffer::Buffer,
    sprite::Sprites,
};

// OpenGL backend imports (glow + glutin) - only when wgpu is disabled
#[cfg(not(feature = "wgpu"))]
use crate::render::adapter::gl::pixel::GlPixel;

#[cfg(not(feature = "wgpu"))]
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, Version},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};

#[cfg(not(feature = "wgpu"))]
use glutin_winit::DisplayBuilder;

// Import HasWindowHandle trait for window_handle() method
#[cfg(not(feature = "wgpu"))]
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

/// 窗口拖拽状态管理
///
/// 记录窗口拖拽的相关状态，支持通过鼠标拖拽移动窗口位置。
/// 类似于SDL版本的实现，提供相同的用户体验。
#[derive(Default)]
struct Drag {
    /// 是否需要执行拖拽操作
    need: bool,
    /// 是否正在拖拽中
    draging: bool,
    /// 拖拽起始鼠标X坐标
    mouse_x: f64,
    /// 拖拽起始鼠标Y坐标
    mouse_y: f64,
    /// X轴拖拽偏移量
    dx: f64,
    /// Y轴拖拽偏移量
    dy: f64,
}

/// 边框区域枚举
///
/// 定义鼠标点击区域的类型，用于确定是否应该触发拖拽操作。
pub enum WinitBorderArea {
    /// 无效区域
    NOPE,
    /// 关闭按钮区域
    CLOSE,
    /// 顶部标题栏区域（可拖拽）
    TOPBAR,
    /// 其他边框区域（可拖拽）
    OTHER,
}

/// Winit适配器主结构
///
/// 封装了winit窗口管理和现代渲染后端的所有组件。
/// 支持两种渲染后端：OpenGL (glow) 和现代GPU API (wgpu)。
/// 实现了与SDL适配器相同的接口，可以无缝替换。
/// 窗口初始化参数
#[derive(Debug, Clone)]
pub struct WindowInitParams {
    pub width: u16,
    pub height: u16,
    pub ratio_x: f32,
    pub ratio_y: f32,
    pub title: String,
    pub texture_path: String,
}

pub struct WinitAdapter {
    /// 基础适配器数据
    pub base: AdapterBase,

    // Winit相关对象
    /// 窗口实例
    pub window: Option<Arc<Window>>,
    /// 事件循环
    pub event_loop: Option<EventLoop<()>>,
    /// 窗口初始化参数（用于在resumed中创建窗口）
    pub window_init_params: Option<WindowInitParams>,

    // OpenGL backend objects (only when wgpu is disabled)
    #[cfg(not(feature = "wgpu"))]
    /// OpenGL显示上下文
    pub gl_display: Option<glutin::display::Display>,
    #[cfg(not(feature = "wgpu"))]
    /// OpenGL渲染上下文
    pub gl_context: Option<glutin::context::PossiblyCurrentContext>,
    #[cfg(not(feature = "wgpu"))]
    /// OpenGL渲染表面
    pub gl_surface: Option<Surface<WindowSurface>>,

    // WGPU backend objects (only when wgpu is enabled)
    #[cfg(feature = "wgpu")]
    /// WGPU instance for creating devices and surfaces
    pub wgpu_instance: Option<wgpu::Instance>,
    #[cfg(feature = "wgpu")]
    /// WGPU device for creating resources
    pub wgpu_device: Option<wgpu::Device>,
    #[cfg(feature = "wgpu")]
    /// WGPU queue for submitting commands
    pub wgpu_queue: Option<wgpu::Queue>,
    #[cfg(feature = "wgpu")]
    /// Window surface for rendering
    pub wgpu_surface: Option<wgpu::Surface<'static>>,
    #[cfg(feature = "wgpu")]
    /// Surface configuration
    pub wgpu_surface_config: Option<wgpu::SurfaceConfiguration>,
    #[cfg(feature = "wgpu")]
    /// Main pixel renderer
    pub wgpu_pixel_renderer: Option<WgpuPixelRender>,

    /// 是否应该退出程序
    pub should_exit: bool,

    /// 事件处理器（用于pump events模式）
    pub app_handler: Option<WinitAppHandler>,

    /// 自定义鼠标光标
    pub custom_cursor: Option<CustomCursor>,

    /// 窗口拖拽数据
    drag: Drag,
}

/// Winit应用程序事件处理器
///
/// 实现winit的ApplicationHandler trait，处理窗口事件和用户输入。
/// 使用不安全指针引用适配器实例以支持拖拽功能。
pub struct WinitAppHandler {
    /// 待处理的像素事件队列
    pub pending_events: Vec<Event>,
    /// 当前鼠标位置
    pub cursor_position: (f64, f64),
    /// X轴比例调整系数
    pub ratio_x: f32,
    /// Y轴比例调整系数
    pub ratio_y: f32,
    /// 是否应该退出
    pub should_exit: bool,

    /// 适配器引用（用于拖拽处理）
    ///
    /// 注意：这里使用原始指针是为了避免借用检查器的限制，
    /// 在事件处理期间需要修改适配器状态。使用时必须确保安全性。
    pub adapter_ref: *mut WinitAdapter,
}

impl ApplicationHandler for WinitAppHandler {
    /// 应用程序恢复时的回调
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        unsafe {
            let adapter = &mut *self.adapter_ref;
            // 如果是WGPU模式且窗口尚未创建，则创建窗口和资源
            #[cfg(feature = "wgpu")]
            if adapter.window.is_none() && adapter.window_init_params.is_some() {
                adapter.create_wgpu_window_and_resources(event_loop);
            }
        }
    }

    /// 处理窗口事件
    ///
    /// 这是事件处理的核心方法，处理所有的窗口事件包括：
    /// - 窗口关闭请求
    /// - 键盘输入（支持Q键退出）
    /// - 鼠标移动和点击
    /// - 窗口拖拽逻辑
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
                // 处理Q键退出（与SDL版本保持一致）
                if key_event.state == winit::event::ElementState::Pressed {
                    if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key {
                        if keycode == winit::keyboard::KeyCode::KeyQ {
                            self.should_exit = true;
                            event_loop.exit();
                            return;
                        }
                    }
                }

                // 将键盘事件转换为像素事件
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
                // 为Retina显示器转换物理坐标到逻辑坐标
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

                // 处理窗口拖拽
                unsafe {
                    let adapter = &mut *self.adapter_ref;
                    if adapter.drag.draging {
                        adapter.drag.need = true;
                        adapter.drag.dx = position.x - adapter.drag.mouse_x;
                        adapter.drag.dy = position.y - adapter.drag.mouse_y;
                    }
                }

                // 只有在非拖拽状态时才转换为像素事件
                // 使用逻辑位置确保坐标系统一致
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
                                    // 在边框区域按下左键时开始拖拽
                                    adapter.drag.draging = true;
                                    adapter.drag.mouse_x = self.cursor_position.0;
                                    adapter.drag.mouse_y = self.cursor_position.1;
                                }
                                WinitBorderArea::CLOSE => {
                                    // 点击关闭按钮区域时退出程序
                                    self.should_exit = true;
                                    event_loop.exit();
                                }
                                _ => {
                                    // 非拖拽区域，将事件传递给游戏逻辑
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

                            // 只有在非拖拽状态时才将鼠标释放事件传递给游戏
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
                        // 转换其他鼠标输入事件
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
                // 将其他winit事件转换为RustPixel事件
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
    /// 创建新的Winit适配器实例
    ///
    /// # 参数
    /// - `gn`: 游戏名称
    /// - `project_path`: 项目路径（用于资源加载）
    ///
    /// # 返回值
    /// 返回初始化的WinitAdapter实例，所有OpenGL相关组件都为None，
    /// 需要在调用init()方法后才能正常使用。
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            base: AdapterBase::new(gn, project_path),
            window: None,
            event_loop: None,
            window_init_params: None,

            // OpenGL backend fields (only when wgpu is disabled)
            #[cfg(not(feature = "wgpu"))]
            gl_display: None,
            #[cfg(not(feature = "wgpu"))]
            gl_context: None,
            #[cfg(not(feature = "wgpu"))]
            gl_surface: None,

            // WGPU backend fields (only when wgpu is enabled)
            #[cfg(feature = "wgpu")]
            wgpu_instance: None,
            #[cfg(feature = "wgpu")]
            wgpu_device: None,
            #[cfg(feature = "wgpu")]
            wgpu_queue: None,
            #[cfg(feature = "wgpu")]
            wgpu_surface: None,
            #[cfg(feature = "wgpu")]
            wgpu_surface_config: None,
            #[cfg(feature = "wgpu")]
            wgpu_pixel_renderer: None,

            should_exit: false,
            app_handler: None,
            custom_cursor: None,
            drag: Default::default(),
        }
    }

    #[cfg(not(feature = "wgpu"))]
    fn init_glow(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing GLOW adapter...");

        // 1. 加载纹理文件和设置符号尺寸（与 OpenGL 版本相同）
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
        self.set_size(w, h).set_title(title);
        self.base.gr.set_ratiox(rx);
        self.base.gr.set_ratioy(ry);
        self.base.gr.set_pixel_size(self.base.cell_w, self.base.cell_h);

        info!(
            "pixel_w={} pixel_h={}",
            self.base.gr.pixel_w, self.base.gr.pixel_h
        );

        // Create event loop
        let event_loop = EventLoop::new().unwrap();

        // For Retina displays, we need to adjust window logical size so its physical size matches our render area
        // First create a temporary window to get the scale factor
        let temp_window_size = LogicalSize::new(self.base.gr.pixel_w, self.base.gr.pixel_h);
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
        let adjusted_logical_w = self.base.gr.pixel_w;
        let adjusted_logical_h = self.base.gr.pixel_h;
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
            self.base.gr.pixel_w, self.base.gr.pixel_h
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
        self.base.gr.gl = Some(gl);
        self.window = Some(Arc::new(window));
        self.gl_display = Some(gl_display);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(surface);

        // Create GlPixel with logical dimensions for consistent coordinate system
        // The framebuffer is still high-res (physical size), but GlPixel will handle scaling
        self.base.gr.gl_pixel = Some(GlPixel::new(
            self.base.gr.gl.as_ref().unwrap(),
            "#version 330 core",
            self.base.gr.pixel_w as i32, // Use logical size for coordinate system
            self.base.gr.pixel_h as i32, // Use logical size for coordinate system
            texwidth as i32,
            texheight as i32,
            &teximg,
        ));

        // Ratio remains the same, but OpenGL will render at higher resolution on Retina
        info!(
            "Using standard ratio: {}x{}, OpenGL framebuffer: {}x{} (2x on Retina)",
            self.base.gr.ratio_x, self.base.gr.ratio_y, physical_size.width, physical_size.height
        );

        self.app_handler = Some(WinitAppHandler {
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
            ratio_x: self.base.gr.ratio_x, // Standard ratio - OpenGL handles scaling automatically
            ratio_y: self.base.gr.ratio_y, // Standard ratio - OpenGL handles scaling automatically
            should_exit: false,
            adapter_ref: self as *mut WinitAdapter,
        });

        // Store event loop for later use
        self.event_loop = Some(event_loop);

        // Set custom mouse cursor (similar to SDL version)
        self.set_mouse_cursor();

        // Ensure cursor is visible (similar to SDL version)
        self.show_cursor().unwrap();

        // Perform initial clear to prevent white flash on window creation
        // This is important for winit mode because the window is immediately visible
        // after creation, unlike SDL mode
        if let (Some(gl), Some(gl_pixel)) = (&self.base.gr.gl, &mut self.base.gr.gl_pixel) {
            use glow::HasContext;

            unsafe {
                // Bind the screen framebuffer
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);

                // Get actual window physical size for viewport
                if let Some(window) = &self.window {
                    let physical_size = window.inner_size();
                    gl.viewport(
                        0,
                        0,
                        physical_size.width as i32,
                        physical_size.height as i32,
                    );
                }

                // Set black clear color and clear the screen
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }

            // Swap buffers to display the cleared screen immediately
            if let (Some(surface), Some(context)) = (&self.gl_surface, &self.gl_context) {
                surface.swap_buffers(context).unwrap();
            }
        }

        info!("Winit window & OpenGL context initialized successfully");
    }

    /// 初始化现代 WGPU 渲染后端
    ///
    /// 使用 wgpu 技术栈初始化现代 GPU 渲染环境。
    /// 提供跨平台的现代 GPU API 支持，包括 Vulkan、Metal、D3D12 等。
    ///
    /// # 参数
    /// - `w`: 逻辑宽度（字符数）
    /// - `h`: 逻辑高度（字符数）
    /// - `rx`: X轴缩放比例
    /// - `ry`: Y轴缩放比例
    /// - `title`: 窗口标题
    ///
    /// # 初始化流程
    /// 1. 加载纹理资源并设置符号尺寸
    /// 2. 创建事件循环和窗口
    /// 3. 初始化 WGPU 实例、适配器、设备
    /// 4. 创建窗口表面和配置
    /// 5. 初始化 WGPU 像素渲染器
    /// 6. 设置自定义光标和事件处理器
    #[cfg(feature = "wgpu")]
    fn init_wgpu(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing WGPU adapter...");

        // 1. 加载纹理文件和设置符号尺寸（与 OpenGL 版本相同）
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

        info!("WGPU load texture...{}", texture_path);
        info!(
            "symbol_w={} symbol_h={}",
            PIXEL_SYM_WIDTH.get().expect("lazylock init"),
            PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
        );

        // 设置基础参数
        self.set_size(w, h).set_title(title.clone());
        self.base.gr.set_ratiox(rx);
        self.base.gr.set_ratioy(ry);
        self.base.gr.set_pixel_size(self.base.cell_w, self.base.cell_h);

        info!(
            "pixel_w={} pixel_h={}",
            self.base.gr.pixel_w, self.base.gr.pixel_h
        );

        // 2. 创建事件循环，但延迟创建窗口到resumed方法中
        let event_loop = EventLoop::new().unwrap();

        // 存储窗口初始化参数，稍后在resumed中使用
        self.window_init_params = Some(WindowInitParams {
            width: w,
            height: h,
            ratio_x: rx,
            ratio_y: ry,
            title,
            texture_path,
        });

        // 3. 存储事件循环，窗口将在resumed方法中创建
        self.event_loop = Some(event_loop);

        // 4. 配置事件处理器
        self.app_handler = Some(WinitAppHandler {
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
            ratio_x: self.base.gr.ratio_x,
            ratio_y: self.base.gr.ratio_y,
            should_exit: false,
            adapter_ref: self as *mut WinitAdapter,
        });

        info!("WGPU adapter initialization prepared, window will be created in resumed()");
    }

    /// 在resumed方法中创建WGPU窗口和相关资源
    #[cfg(feature = "wgpu")]
    fn create_wgpu_window_and_resources(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let params = self.window_init_params.as_ref().unwrap().clone();

        // 计算窗口大小（处理 Retina 显示器）
        let window_size = LogicalSize::new(self.base.gr.pixel_w, self.base.gr.pixel_h);

        let window_attributes = winit::window::Window::default_attributes()
            .with_title(&params.title)
            .with_inner_size(window_size)
            .with_decorations(false) // 无边框，与 SDL 版本一致
            .with_resizable(false);

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        // 获取实际物理尺寸
        let physical_size = window.inner_size();
        info!(
            "Window created - logical: {}x{}, physical: {}x{}",
            self.base.gr.pixel_w, self.base.gr.pixel_h, physical_size.width, physical_size.height
        );

        // 初始化 WGPU 核心组件
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // 先存储窗口，稍后创建surface
        self.window = Some(window.clone());

        // 创建窗口表面（使用 unsafe 方式避免生命周期问题）
        let wgpu_surface = unsafe {
            wgpu_instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(&**self.window.as_ref().unwrap())
                        .unwrap(),
                )
                .expect("Failed to create surface")
        };

        // 异步获取适配器和设备（使用 pollster 简化）
        let (wgpu_device, wgpu_queue, wgpu_surface_config) = pollster::block_on(async {
            // 请求适配器
            let adapter = wgpu_instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&wgpu_surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("Failed to find suitable WGPU adapter");

            info!("WGPU adapter found: {:?}", adapter.get_info());

            // 请求设备和队列
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("RustPixel WGPU Device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                    },
                    None,
                )
                .await
                .expect("Failed to create WGPU device");

            // 配置表面
            let surface_caps = wgpu_surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb()) // 优先选择线性格式，匹配GL模式
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

        // 创建并初始化 WGPU 像素渲染器
        // 使用逻辑尺寸创建渲染器以避免缓冲区过大，坐标转换在着色器中处理
        let mut wgpu_pixel_renderer = WgpuPixelRender::new_with_format(
            self.base.gr.pixel_w,       // 使用逻辑尺寸避免缓冲区过大
            self.base.gr.pixel_h,       // 使用逻辑尺寸避免缓冲区过大
            wgpu_surface_config.format, // Use actual surface format
        );

        // 加载纹理
        if let Err(e) =
            wgpu_pixel_renderer.load_symbol_texture(&wgpu_device, &wgpu_queue, &params.texture_path)
        {
            panic!("Failed to load symbol texture: {}", e);
        }

        // 创建shader和渲染管线
        wgpu_pixel_renderer.create_shader(&wgpu_device);

        // 创建buffers
        wgpu_pixel_renderer.create_buffer(&wgpu_device);

        // 创建bind group
        wgpu_pixel_renderer.create_bind_group(&wgpu_device);

        // 初始化render textures（4个离屏渲染目标）
        if let Err(e) = wgpu_pixel_renderer.init_render_textures(&wgpu_device) {
            panic!("Failed to initialize render textures: {}", e);
        }

        // 初始化General2D渲染器（用于转场效果）
        wgpu_pixel_renderer.init_general2d_renderer(&wgpu_device);

        // 初始化Transition渲染器（用于转场效果）
        wgpu_pixel_renderer.init_transition_renderer(&wgpu_device);

        // 设置ratio参数以匹配OpenGL版本的坐标变换
        wgpu_pixel_renderer.set_ratio(self.base.gr.ratio_x, self.base.gr.ratio_y);

        // 存储所有 WGPU 对象
        self.wgpu_instance = Some(wgpu_instance);
        self.wgpu_device = Some(wgpu_device);
        self.wgpu_queue = Some(wgpu_queue);
        self.wgpu_surface = Some(wgpu_surface);
        self.wgpu_surface_config = Some(wgpu_surface_config);
        self.wgpu_pixel_renderer = Some(wgpu_pixel_renderer);

        // 设置自定义光标
        self.set_mouse_cursor();

        info!("WGPU window & context initialized successfully");
    }

    /// 设置自定义鼠标光标
    ///
    /// 加载并设置自定义的鼠标光标图像。光标图像从assets/pix/cursor.png加载，
    /// 如果加载失败则使用系统默认光标。
    ///
    /// # 实现细节
    /// - 支持PNG格式的光标图像
    /// - 自动转换为RGBA8格式
    /// - 热点位置设置为(0, 0)
    fn set_mouse_cursor(&mut self) {
        // 构建光标图像文件路径
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
                    window.as_ref().set_cursor(custom_cursor);
                    // Ensure cursor is visible after setting custom cursor
                    window.as_ref().set_cursor_visible(true);
                }
            }
        } else {
            // If custom cursor fails to load, ensure standard cursor is visible
            if let Some(window) = &self.window {
                window.as_ref().set_cursor_visible(true);
            }
        }
    }

    /// 检查鼠标位置是否在边框区域
    ///
    /// 用于确定鼠标点击位置的区域类型，决定是否触发拖拽操作。
    ///
    /// # 参数
    /// - `x`: 鼠标X坐标
    /// - `y`: 鼠标Y坐标
    ///
    /// # 返回值
    /// 返回对应的边框区域类型
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

    /// WGPU版本的转场渲染到纹理（为petview等应用提供高级API）
    #[cfg(feature = "wgpu")]
    pub fn render_transition_to_texture_wgpu(
        &mut self,
        target_texture_idx: usize,
        shader_idx: usize,
        progress: f32,
    ) -> Result<(), String> {
        if let (Some(device), Some(queue), Some(pixel_renderer)) = (
            &self.wgpu_device,
            &self.wgpu_queue,
            &mut self.wgpu_pixel_renderer,
        ) {
            // 创建命令编码器
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Transition Render Encoder"),
            });

            // 使用新的render_trans_frame_to_texture方法，避免借用冲突
            pixel_renderer.render_trans_frame_to_texture(
                device,
                queue,
                &mut encoder,
                target_texture_idx,
                shader_idx,
                progress,
            )?;

            // 提交命令
            queue.submit(std::iter::once(encoder.finish()));
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }

    /// WGPU版本的render buffer到纹理渲染方法
    ///
    /// 这个方法实现了与OpenGL版本相同的接口，将RenderCell数据渲染到指定的render texture中。
    /// 用于统一两种渲染后端的接口。
    ///
    /// # 参数
    /// - `rbuf`: RenderCell数据数组
    /// - `rtidx`: 目标render texture索引
    /// - `debug`: 是否启用调试模式
    #[cfg(feature = "wgpu")]
    pub fn draw_render_buffer_to_texture_wgpu(
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
            // 准备渲染数据
            pixel_renderer.prepare_draw(device, queue);
            pixel_renderer.prepare_draw_with_render_cells(device, queue, rbuf);

            // 创建命令编码器用于渲染到纹理
            let mut rt_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(&format!("Render to RT{} Encoder", rtidx)),
            });

            // 渲染到指定的render texture
            {
                let render_pass_result =
                    pixel_renderer.begin_render_to_texture(&mut rt_encoder, rtidx);
                if let Ok(mut render_pass) = render_pass_result {
                    // begin_render_to_texture已经设置好了pipeline、buffers和bind groups
                    render_pass.draw_indexed(0..6, 0, 0..pixel_renderer.get_instance_count());
                }
                // render_pass自动drop
            }

            // 提交渲染到纹理的命令
            queue.submit(std::iter::once(rt_encoder.finish()));
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }

    /// WGPU版本的render texture到屏幕渲染（内部实现）
    ///
    /// 这是WGPU版本的draw_render_textures_to_screen方法的内部实现，与OpenGL版本对应。
    /// 负责将render texture中的内容最终合成到屏幕上，支持转场效果。
    ///
    /// 正确的WGPU渲染流程：
    /// 1. 将RenderCell数据渲染到render texture 2（主缓冲区）
    /// 2. 将render texture 2合成到屏幕（如果不隐藏）
    /// 3. 将render texture 3合成到屏幕（如果不隐藏，用于转场效果）
    #[cfg(feature = "wgpu")]
    pub fn draw_render_textures_to_screen_wgpu(&mut self) -> Result<(), String> {
        if let (Some(device), Some(queue), Some(surface), Some(pixel_renderer)) = (
            &self.wgpu_device,
            &self.wgpu_queue,
            &self.wgpu_surface,
            &mut self.wgpu_pixel_renderer,
        ) {
            // 获取当前表面纹理
            let output = surface
                .get_current_texture()
                .map_err(|e| format!("Failed to acquire next swap chain texture: {}", e))?;

            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // 创建命令编码器用于屏幕合成
            let mut screen_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Screen Composition Encoder"),
                });

            // 清空屏幕
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
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                // clear_pass自动drop
            }

            // 绘制render texture 2（主缓冲区）到屏幕
            if !pixel_renderer.get_render_texture_hidden(2) {
                use crate::render::adapter::wgpu::color::WgpuColor;
                use crate::render::adapter::wgpu::transform::WgpuTransform;

                let transform = WgpuTransform::new();
                let color = WgpuColor::new(1.0, 1.0, 1.0, 1.0);

                pixel_renderer.draw_general2d(
                    device,
                    queue,
                    &mut screen_encoder,
                    &view,
                    2,                    // render texture 2
                    [0.0, 0.0, 1.0, 1.0], // 全屏区域
                    &transform,
                    &color,
                )?;
            }

            // 绘制render texture 3（转场效果）到屏幕
            if !pixel_renderer.get_render_texture_hidden(3) {
                use crate::render::adapter::wgpu::color::WgpuColor;
                use crate::render::adapter::wgpu::transform::WgpuTransform;

                let pcw = pixel_renderer.canvas_width as f32;
                let pch = pixel_renderer.canvas_height as f32;
                let rx = self.base.gr.ratio_x;
                let ry = self.base.gr.ratio_y;

                // 使用实际的游戏区域尺寸（匹配OpenGL版本）
                let pw =
                    self.base.cell_w as f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                let ph =
                    self.base.cell_h as f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

                let mut transform = WgpuTransform::new();
                transform.scale(pw / pcw, ph / pch);
                let color = WgpuColor::new(1.0, 1.0, 1.0, 1.0);

                pixel_renderer.draw_general2d(
                    device,
                    queue,
                    &mut screen_encoder,
                    &view,
                    3,                                                 // render texture 3
                    [0.0 / pcw, (pch - ph) / pch, pw / pcw, ph / pch], // 游戏区域
                    &transform,
                    &color,
                )?;
            }

            // 提交屏幕合成命令并呈现帧
            queue.submit(std::iter::once(screen_encoder.finish()));
            output.present();
        } else {
            return Err("WGPU components not initialized".to_string());
        }

        Ok(())
    }
}

impl Adapter for WinitAdapter {
    /// 初始化Winit适配器
    ///
    /// 这是适配器的主要初始化方法，根据编译特性标志选择不同的渲染后端：
    /// - 当 wgpu 特性启用时，使用现代 WGPU 渲染管线
    /// - 当 wgpu 特性禁用时，使用传统 OpenGL + Glutin 管线
    ///
    /// # 参数
    /// - `w`: 逻辑宽度（字符数）
    /// - `h`: 逻辑高度（字符数）
    /// - `rx`: X轴缩放比例
    /// - `ry`: Y轴缩放比例
    /// - `title`: 窗口标题
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        #[cfg(feature = "wgpu")]
        {
            info!("Initializing Winit adapter with WGPU backend...");
            self.init_wgpu(w, h, rx, ry, title);
        }

        #[cfg(not(feature = "wgpu"))]
        {
            info!("Initializing Winit adapter with OpenGL backend...");
            self.init_glow(w, h, rx, ry, title);
        }
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH.get().expect("lazylock init") / self.base.gr.ratio_x
    }

    fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT.get().expect("lazylock init") / self.base.gr.ratio_y
    }

    /// 轮询事件
    ///
    /// 处理窗口事件并将其转换为RustPixel事件。使用pump_events模式
    /// 避免阻塞主线程，确保渲染性能。
    ///
    /// # 参数
    /// - `timeout`: 事件轮询超时时间（未使用）
    /// - `es`: 输出事件向量
    ///
    /// # 返回值
    /// 如果应该退出程序则返回true
    ///
    /// # 特殊处理
    /// - 窗口拖拽：检测并执行窗口移动
    /// - Q键退出：与SDL版本保持一致
    /// - Retina显示：正确处理高DPI坐标转换
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

    /// 渲染一帧到屏幕
    ///
    /// 根据编译特性选择不同的渲染后端：
    /// - WGPU 版本：使用现代 GPU 渲染管线
    /// - OpenGL 版本：使用传统 OpenGL 渲染管线
    ///
    /// # 参数
    /// - `current_buffer`: 当前帧缓冲区
    /// - `previous_buffer`: 前一帧缓冲区
    /// - `pixel_sprites`: 像素精灵列表
    /// - `stage`: 渲染阶段
    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        winit_move_win(
            &mut self.drag.need,
            self.window.as_ref().map(|v| &**v),
            self.drag.dx,
            self.drag.dy,
        );

        self.draw_all_graph(current_buffer, previous_buffer, pixel_sprites, stage);

        #[cfg(not(feature = "wgpu"))]
        {
            // OpenGL模式：交换缓冲区,Wgpu模式是不需要的
            if let Some(surface) = &self.gl_surface {
                if let Some(context) = &self.gl_context {
                    surface.swap_buffers(context).unwrap();
                }
            }
        }

        if let Some(window) = &self.window {
            window.as_ref().request_redraw();
        }

        Ok(())
    }

    /// 隐藏光标
    ///
    /// 在图形应用程序中，我们不希望隐藏鼠标光标。
    /// 这与SDL版本的行为相似 - 让鼠标光标保持可见。
    ///
    /// # 设计考虑
    /// 保持与SDL适配器的一致性，实际上不执行隐藏操作。
    fn hide_cursor(&mut self) -> Result<(), String> {
        // 对于GUI应用程序，我们不希望隐藏鼠标光标
        // 这与SDL行为相似 - 让鼠标光标保持可见
        Ok(())
    }

    /// 显示光标
    ///
    /// 确保鼠标光标可见。如果窗口存在，则显式设置光标可见性。
    fn show_cursor(&mut self) -> Result<(), String> {
        if let Some(window) = &self.window {
            window.set_cursor_visible(true);
        }
        Ok(())
    }

    /// 设置光标位置
    ///
    /// 在Winit中，光标位置通常由系统管理，此方法为兼容性而保留。
    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        Ok(())
    }

    /// 获取光标位置
    ///
    /// 返回当前光标位置。在Winit实现中，返回固定值以保持接口兼容性。
    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok((0, 0))
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    /// 重写渲染纹理到屏幕的方法以处理Retina缩放
    ///
    /// 这是专门为Winit适配器优化的渲染方法，解决了在Retina显示器上
    /// 的viewport设置问题。
    ///
    /// # Retina显示问题
    /// 在Retina显示器上：
    /// - 逻辑分辨率与物理分辨率不同（通常是2倍关系）
    /// - GlPixel使用逻辑尺寸设置viewport
    /// - 但framebuffer实际是物理尺寸
    /// - 导致显示区域只占屏幕的1/4
    ///
    /// # 解决方案
    /// 1. 让GlPixel先绑定屏幕framebuffer
    /// 2. 手动重设viewport为物理尺寸
    /// 3. 确保渲染覆盖整个屏幕
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_render_textures_to_screen(&mut self) {
        // Check if we're in WGPU mode first
        #[cfg(feature = "wgpu")]
        {
            if self.wgpu_pixel_renderer.is_some() {
                // WGPU mode - call the WGPU implementation
                if let Err(e) = self.draw_render_textures_to_screen_wgpu() {
                    eprintln!("WGPU draw_render_textures_to_screen error: {}", e);
                }
                return;
            }
        }

        // OpenGL mode - original implementation
        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        {
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

            if let (Some(pix), Some(gl)) = (&mut bs.gr.gl_pixel, &mut bs.gr.gl) {
                // First bind screen with GlPixel's logical viewport
                pix.bind_screen(gl);

                // Then manually set the correct viewport for Retina displays
                if let Some(physical_size) = physical_size {
                    unsafe {
                        gl.viewport(
                            0,
                            0,
                            physical_size.width as i32,
                            physical_size.height as i32,
                        );
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
                    let rx = bs.gr.ratio_x;
                    let ry = bs.gr.ratio_y;
                    // Use actual game area dimensions instead of hardcoded 40x25
                    let pw = 40.0f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                    let ph = 25.0f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

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
}

/// 将Winit输入事件转换为RustPixel事件
///
/// 为了统一事件处理，将winit的事件系统转换为RustPixel的事件格式。
/// 这样可以确保游戏逻辑与具体的窗口库解耦。
///
/// # 参数
/// - `event`: Winit原始事件
/// - `adjx`: X轴坐标调整系数
/// - `adjy`: Y轴坐标调整系数  
/// - `cursor_pos`: 当前鼠标位置（可变引用）
///
/// # 支持的事件类型
/// - 键盘输入：字母键、方向键
/// - 鼠标输入：左键按下/释放
/// - 鼠标移动：更新光标位置
///
/// # 返回值
/// 如果事件可以转换则返回Some(Event)，否则返回None
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

/// 移动Winit窗口位置
///
/// 根据拖拽偏移量移动窗口到新位置。这个函数实现了与SDL版本相同的
/// 窗口拖拽功能。
///
/// # 参数
/// - `drag_need`: 是否需要拖拽的标志（会被重置为false）
/// - `window`: 窗口实例的可选引用
/// - `dx`: X轴拖拽偏移量
/// - `dy`: Y轴拖拽偏移量
///
/// # 实现细节
/// - 获取当前窗口位置
/// - 计算新位置（当前位置 + 偏移量）
/// - 调用set_outer_position移动窗口
/// - 重置拖拽标志
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
