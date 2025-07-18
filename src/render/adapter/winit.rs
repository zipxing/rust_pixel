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
    pixel_renderer::{PixelRenderer, RenderContext, UnifiedColor, UnifiedTransform},
    sprite::Sprites,
};

// OpenGL backend imports (glow + glutin) - only when wgpu is disabled
#[cfg(not(feature = "wgpu"))]
use crate::render::adapter::gl::pixel::GlPixelRenderer;

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

    /// 光标是否已经设置（延迟设置标志）
    pub cursor_set: bool,

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
    ///
    /// 在resumed事件中创建窗口和渲染资源。
    /// 这是统一的生命周期管理方式，适用于两种渲染后端。
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // 在这里创建窗口和渲染上下文
        if let Some(adapter) = unsafe { self.adapter_ref.as_mut() } {
            if adapter.window.is_none() {
                #[cfg(not(feature = "wgpu"))]
                {
                    adapter.create_glow_window_and_context(event_loop);
                }

                #[cfg(feature = "wgpu")]
                {
                    adapter.create_wgpu_window_and_resources(event_loop);
                }
            }

            // 延迟设置光标 - 在窗口完全初始化后进行
            if !adapter.cursor_set {
                // 在设置光标前先清屏 - 可能有助于解决透明度问题
                #[cfg(not(feature = "wgpu"))]
                {
                    adapter.initial_clear_screen();
                }

                #[cfg(feature = "wgpu")]
                {
                    adapter.clear_screen_wgpu();
                }

                adapter.set_mouse_cursor();
                adapter.cursor_set = true;
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
            cursor_set: false,
            drag: Default::default(),
        }
    }

    /// 通用初始化方法 - 处理所有公共逻辑
    ///
    /// 这个方法处理两种渲染后端都需要的初始化步骤：
    /// 1. 加载纹理文件和设置符号尺寸
    /// 2. 设置基础参数（尺寸、比例、像素大小）
    /// 3. 创建事件循环
    /// 4. 设置应用处理器
    /// 5. 存储窗口初始化参数
    ///
    /// # 参数
    /// - `w`: 逻辑宽度（字符数）
    /// - `h`: 逻辑高度（字符数）
    /// - `rx`: X轴缩放比例
    /// - `ry`: Y轴缩放比例
    /// - `title`: 窗口标题
    ///
    /// # 返回值
    /// 返回纹理路径，用于后续的渲染器初始化
    fn init_common(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) -> String {
        info!("Initializing Winit adapter common components...");

        // 1. 加载纹理文件和设置符号尺寸
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

        info!("Loaded texture: {}", texture_path);
        info!(
            "Symbol dimensions: {}x{}",
            PIXEL_SYM_WIDTH.get().expect("lazylock init"),
            PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
        );

        // 2. 设置基础参数
        self.set_size(w, h).set_title(title.clone());
        self.base.gr.set_ratiox(rx);
        self.base.gr.set_ratioy(ry);
        self.base
            .gr
            .set_pixel_size(self.base.cell_w, self.base.cell_h);

        info!(
            "Window pixel size: {}x{}",
            self.base.gr.pixel_w, self.base.gr.pixel_h
        );

        // 3. 创建事件循环
        let event_loop = EventLoop::new().unwrap();
        self.event_loop = Some(event_loop);

        // 4. 存储窗口初始化参数，用于在resumed中创建窗口
        self.window_init_params = Some(WindowInitParams {
            width: w,
            height: h,
            ratio_x: rx,
            ratio_y: ry,
            title,
            texture_path: texture_path.clone(),
        });

        // 5. 设置应用处理器
        self.app_handler = Some(WinitAppHandler {
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
            ratio_x: self.base.gr.ratio_x,
            ratio_y: self.base.gr.ratio_y,
            should_exit: false,
            adapter_ref: self as *mut WinitAdapter,
        });

        info!("Common initialization completed, window will be created in resumed()");
        texture_path
    }

    /// OpenGL (glow) 后端初始化
    ///
    /// 使用统一的生命周期管理，窗口创建推迟到resumed事件中。
    #[cfg(not(feature = "wgpu"))]
    fn init_glow(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing Winit adapter with OpenGL backend...");
        let _texture_path = self.init_common(w, h, rx, ry, title);
        // 窗口创建将在resumed事件中完成
    }

    /// WGPU 后端初始化
    ///
    /// 使用统一的生命周期管理，窗口创建推迟到resumed事件中。
    #[cfg(feature = "wgpu")]
    fn init_wgpu(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing Winit adapter with WGPU backend...");
        let _texture_path = self.init_common(w, h, rx, ry, title);
        // 窗口创建将在resumed事件中完成
    }

    /// 在resumed事件中创建OpenGL窗口和上下文
    #[cfg(not(feature = "wgpu"))]
    fn create_glow_window_and_context(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let params = self.window_init_params.as_ref().unwrap().clone();

        info!("Creating OpenGL window and context...");

        // 计算窗口大小（处理 Retina 显示器）
        let window_size = LogicalSize::new(self.base.gr.pixel_w, self.base.gr.pixel_h);

        let template = ConfigTemplateBuilder::new();
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(
            winit::window::Window::default_attributes()
                .with_title(&params.title)
                .with_inner_size(window_size)
                .with_decorations(false) // 无边框，与SDL版本一致
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

        // 创建OpenGL上下文
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

        // 创建glow上下文
        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s)
                    .expect("failed to construct C string from string for gl proc address");
                gl_display.get_proc_address(s.as_c_str())
            })
        };

        // 加载纹理并创建GlPixelRenderer
        let teximg = image::open(&params.texture_path)
            .map_err(|e| e.to_string())
            .unwrap()
            .to_rgba8();
        let texwidth = teximg.width();
        let texheight = teximg.height();

        // 创建统一的像素渲染器，拥有OpenGL上下文
        let gl_pixel_renderer = GlPixelRenderer::new(
            gl,
            "#version 330 core",
            self.base.gr.pixel_w as i32,
            self.base.gr.pixel_h as i32,
            texwidth as i32,
            texheight as i32,
            &teximg,
        );

        // 存储统一渲染器和其他OpenGL对象
        self.base.gr.pixel_renderer = Some(Box::new(gl_pixel_renderer));
        self.window = Some(window);
        self.gl_display = Some(gl_display);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(surface);

        // 设置光标和执行初始清屏
        self.show_cursor().unwrap();
        self.initial_clear_screen();

        info!("OpenGL window & context initialized successfully");
    }

    /// 在resumed事件中创建WGPU窗口和相关资源
    #[cfg(feature = "wgpu")]
    fn create_wgpu_window_and_resources(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let params = self.window_init_params.as_ref().unwrap().clone();

        info!("Creating WGPU window and resources...");

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

        self.window = Some(window.clone());

        // 创建窗口表面
        let wgpu_surface = unsafe {
            wgpu_instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(&**self.window.as_ref().unwrap())
                        .unwrap(),
                )
                .expect("Failed to create surface")
        };

        // 异步获取适配器和设备
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
                    },
                    None,
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

        // 创建并初始化 WGPU 像素渲染器
        let mut wgpu_pixel_renderer = WgpuPixelRender::new_with_format(
            self.base.gr.pixel_w,
            self.base.gr.pixel_h,
            wgpu_surface_config.format,
        );

        // 初始化所有WGPU组件
        if let Err(e) =
            wgpu_pixel_renderer.load_symbol_texture(&wgpu_device, &wgpu_queue, &params.texture_path)
        {
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

        // 存储所有 WGPU 对象
        self.wgpu_instance = Some(wgpu_instance);
        self.wgpu_device = Some(wgpu_device);
        self.wgpu_queue = Some(wgpu_queue);
        self.wgpu_surface = Some(wgpu_surface);
        self.wgpu_surface_config = Some(wgpu_surface_config);
        self.wgpu_pixel_renderer = Some(wgpu_pixel_renderer);

        info!("WGPU window & context initialized successfully");
    }

    /// 执行初始清屏操作
    ///
    /// 防止窗口创建时的白屏闪烁，立即清空屏幕并显示黑色背景。
    ///
    /// WGPU 模式不需要 initial_clear_screen，原因如下：
    /// - 更好的资源管理：WGPU 的 Surface 配置机制确保了良好的初始状态
    /// - 延迟渲染：命令缓冲区模式让我们可以在 present 前准备好所有内容
    /// - 默认 Clear 行为：RenderPass 默认就有 clear 操作
    /// - 原子性操作：整个渲染过程是原子的，要么全部完成要么不显示
    ///
    /// 这就是现代图形 API（Vulkan、Metal、D3D12）相比传统 OpenGL 的优势之一：更好的资源管理和更少的意外行为。
    #[cfg(not(feature = "wgpu"))]
    fn initial_clear_screen(&mut self) {
        if let Some(pixel_renderer) = &mut self.base.gr.pixel_renderer {
            // Use GlPixelRenderer specific methods for OpenGL operations
            use crate::render::adapter::gl::pixel::GlPixelRenderer;
            if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                let gl = gl_pixel_renderer.get_gl();
                use glow::HasContext;

                unsafe {
                    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                    gl.clear_color(0.0, 0.0, 0.0, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                if let Some(gl_surface) = &self.gl_surface {
                    if let Err(e) = gl_surface.swap_buffers(&self.gl_context.as_ref().unwrap()) {
                        eprintln!("Failed to swap buffers during initial clear: {:?}", e);
                    }
                }
            }
        }
    }

    /// WGPU模式的清屏操作
    ///
    /// 在设置光标前执行清屏，可能有助于解决光标透明度问题
    #[cfg(feature = "wgpu")]
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
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    // clear_pass自动drop
                }

                queue.submit(std::iter::once(encoder.finish()));
                output.present();
            }
        }
    }

    /// 设置自定义鼠标光标
    ///
    /// 从assets/pix/cursor.png加载光标图像，并设置为窗口的自定义光标。
    /// - 自动转换为RGBA8格式
    /// - 热点位置设置为(0, 0)
    /// - 处理透明度和预乘alpha
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
            let mut cursor_data = cursor_rgba.into_raw();

            // 预乘alpha处理 - 这是解决光标透明度问题的常见方法
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
            // 使用统一的WgpuPixelRender封装，匹配OpenGL版本的接口
            let rx = self.base.gr.ratio_x;
            let ry = self.base.gr.ratio_y;

            // 绑定目标render texture
            pixel_renderer.bind_target(rtidx);

            // 设置清除颜色
            if debug {
                // 调试模式使用红色背景
                pixel_renderer.set_clear_color(
                    UnifiedColor::new(1.0, 0.0, 0.0, 1.0),
                );
            } else {
                // 正常模式使用黑色背景
                pixel_renderer.set_clear_color(
                    UnifiedColor::new(0.0, 0.0, 0.0, 1.0),
                );
            }

            // 清除目标
            pixel_renderer.clear();

            // 渲染RenderCell数据到当前绑定的目标
            pixel_renderer.render_rbuf(device, queue, rbuf, rx, ry);

            // 创建命令编码器用于渲染到纹理
            let mut rt_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(&format!("Render to RT{} Encoder", rtidx)),
            });

            // 执行渲染到当前绑定的目标
            pixel_renderer.render_to_current_target(&mut rt_encoder, None)?;

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

            // 使用统一的WgpuPixelRender封装，匹配OpenGL版本的接口
            // 绑定屏幕作为渲染目标
            pixel_renderer.bind_screen();

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

            // 绘制render texture 2（主缓冲区）到屏幕 - 使用统一接口
            if !pixel_renderer.get_render_texture_hidden(2) {
                let unified_transform = UnifiedTransform::new();
                let unified_color = UnifiedColor::white();
                let mut context = RenderContext::Wgpu {
                    device,
                    queue,
                    encoder: &mut screen_encoder,
                    view: Some(&view),
                };

                PixelRenderer::render_texture_to_screen(
                    pixel_renderer,
                    &mut context,
                    2,                    // render texture 2
                    [0.0, 0.0, 1.0, 1.0], // 全屏区域
                    &unified_transform,
                    &unified_color,
                )?;
            }

            // 绘制render texture 3（转场效果）到屏幕 - 使用统一接口
            if !pixel_renderer.get_render_texture_hidden(3) {
                let pcw = pixel_renderer.canvas_width as f32;
                let pch = pixel_renderer.canvas_height as f32;
                let rx = self.base.gr.ratio_x;
                let ry = self.base.gr.ratio_y;

                // 使用实际的游戏区域尺寸（匹配OpenGL版本）
                let pw = 40.0f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                let ph = 25.0f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

                let mut unified_transform = UnifiedTransform::new();
                unified_transform.scale(pw / pcw, ph / pch);
                let unified_color = UnifiedColor::white();
                let mut context = RenderContext::Wgpu {
                    device,
                    queue,
                    encoder: &mut screen_encoder,
                    view: Some(&view),
                };

                PixelRenderer::render_texture_to_screen(
                    pixel_renderer,
                    &mut context,
                    3,                                          // render texture 3
                    [0.0 / pcw, 0.0 / pch, pw / pcw, ph / pch], // 游戏区域，WGPU Y轴从顶部开始
                    &unified_transform,
                    &unified_color,
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

    /// 调试方法：保存render texture为PNG图片文件
    ///
    /// 这个方法将指定的render texture保存为PNG文件，用于调试渲染问题
    #[cfg(feature = "wgpu")]
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

            // 获取render texture
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

            // 创建staging buffer用于读取texture数据
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Render Texture Staging Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            // 创建命令编码器
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Texture Copy Encoder"),
            });

            // 复制texture到buffer
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &render_texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &staging_buffer,
                    layout: wgpu::ImageDataLayout {
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

            // 提交命令
            queue.submit(std::iter::once(encoder.finish()));

            // 映射buffer并读取数据（异步操作）
            let buffer_slice = staging_buffer.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });

            // 等待映射完成
            device.poll(wgpu::Maintain::Wait);

            match receiver.recv() {
                Ok(Ok(())) => {
                    // 读取数据
                    let data = buffer_slice.get_mapped_range();
                    let mut rgba_data = vec![0u8; (texture_width * texture_height * 4) as usize];

                    // 复制数据（处理padding）
                    for y in 0..texture_height {
                        let src_start = (y * bytes_per_row) as usize;
                        let dst_start = (y * texture_width * 4) as usize;
                        let row_size = (texture_width * 4) as usize;

                        // 只复制实际像素数据，跳过padding
                        rgba_data[dst_start..dst_start + row_size]
                            .copy_from_slice(&data[src_start..src_start + row_size]);
                    }

                    // 解除映射
                    drop(data);
                    staging_buffer.unmap();

                    // 保存为PNG文件
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

    /// 调试方法：打印当前渲染状态信息
    #[cfg(feature = "wgpu")]
    pub fn debug_print_render_info(&self) {
        info!("=== WGPU渲染状态信息 ===");

        // 基础参数
        info!("基础参数:");
        info!("  Cell数量: {}x{}", self.base.cell_w, self.base.cell_h);
        info!(
            "  窗口像素尺寸: {}x{}",
            self.base.gr.pixel_w, self.base.gr.pixel_h
        );
        info!(
            "  比例: {:.3}x{:.3}",
            self.base.gr.ratio_x, self.base.gr.ratio_y
        );

        // 符号尺寸
        if let (Some(sym_w), Some(sym_h)) = (PIXEL_SYM_WIDTH.get(), PIXEL_SYM_HEIGHT.get()) {
            info!("  符号尺寸: {}x{}", sym_w, sym_h);

            // 计算游戏区域
            let game_area_w = self.base.cell_w as f32 * sym_w / self.base.gr.ratio_x;
            let game_area_h = self.base.cell_h as f32 * sym_h / self.base.gr.ratio_y;
            info!("  游戏区域: {:.2}x{:.2}", game_area_w, game_area_h);
        }

        // WGPU状态
        if let Some(pixel_renderer) = &self.wgpu_pixel_renderer {
            info!("WGPU状态:");
            info!(
                "  Canvas尺寸: {}x{}",
                pixel_renderer.canvas_width, pixel_renderer.canvas_height
            );

            // Render texture状态
            for i in 0..4 {
                let hidden = pixel_renderer.get_render_texture_hidden(i);
                info!(
                    "  RenderTexture{}: {}",
                    i,
                    if hidden { "隐藏" } else { "显示" }
                );
            }
        }

        info!("========================");
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
        self.post_draw();
        Ok(())
    }

    fn post_draw(&mut self) {
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

        // OpenGL mode - fixed implementation to avoid binding conflicts
        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        {
            use glow::HasContext;

            // Get window physical size first to avoid borrowing conflicts
            let physical_size = if let Some(window) = &self.window {
                Some(window.inner_size())
            } else {
                None
            };

            let bs = self.get_base();

            if let Some(pixel_renderer) = &mut bs.gr.pixel_renderer {
                // Use GlPixelRenderer specific methods for OpenGL operations
                use crate::render::adapter::gl::pixel::GlPixelRenderer;
                if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                    // Step 1: Bind screen and set viewport for Retina displays
                    if let Some(physical_size) = physical_size {
                        gl_pixel_renderer.bind_screen_with_viewport(
                            physical_size.width as i32,
                            physical_size.height as i32,
                        );
                    } else {
                        // Fallback: use logical canvas size
                        let (width, height) = gl_pixel_renderer.get_canvas_size();
                        gl_pixel_renderer.bind_screen_with_viewport(width as i32, height as i32);
                    }

                    // Step 2: Clear screen
                    let gl = gl_pixel_renderer.get_gl();
                    unsafe {
                        gl.clear_color(0.0, 0.0, 0.0, 1.0);
                        gl.clear(glow::COLOR_BUFFER_BIT);
                    }

                    // Step 3: Render textures without rebinding (avoid state conflicts)
                    let rx = bs.gr.ratio_x;
                    let ry = bs.gr.ratio_y;
                    if let Err(e) = gl_pixel_renderer.render_textures_to_screen_no_bind(rx, ry) {
                        eprintln!("Failed to render textures to screen: {}", e);
                    }

                    // Step 4: Swap buffers to display the result (done in post_draw)
                    // Note: We don't swap buffers here to avoid double swapping
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
