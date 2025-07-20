// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # Winit + WGPU 适配器实现
//!
//! 基于winit + wgpu技术栈的现代跨平台渲染适配器。
//!
//! ## 技术栈
//! - **winit**: 跨平台窗口管理和事件处理
//! - **wgpu**: 现代GPU API抽象层（基于Vulkan、Metal、D3D12、WebGPU）
//!
//! ## 功能特性
//! - 跨平台窗口管理（Windows、macOS、Linux）
//! - 高DPI/Retina显示支持
//! - 自定义鼠标光标
//! - 窗口拖拽功能
//! - 键盘和鼠标事件处理
//! - 现代GPU硬件加速渲染
//! - 命令缓冲区和异步渲染
//!
//! ## 架构设计
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
    sprite::Sprites,
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

/// Winit + WGPU适配器主结构
///
/// 封装了winit窗口管理和WGPU现代渲染后端的所有组件。
/// 专门针对WGPU API设计，与WinitGlowAdapter分离。
pub struct WinitWgpuAdapter {
    /// 基础适配器数据
    pub base: AdapterBase,

    // Winit相关对象
    /// 窗口实例
    pub window: Option<Arc<Window>>,
    /// 事件循环
    pub event_loop: Option<EventLoop<()>>,
    /// 窗口初始化参数（用于在resumed中创建窗口）
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

    /// 是否应该退出程序
    pub should_exit: bool,

    /// 事件处理器（用于pump events模式）
    pub app_handler: Option<WinitWgpuAppHandler>,

    /// 自定义鼠标光标
    pub custom_cursor: Option<CustomCursor>,

    /// 光标是否已经设置（延迟设置标志）
    pub cursor_set: bool,

    /// 窗口拖拽数据
    drag: Drag,
}

/// Winit + WGPU应用程序事件处理器
///
/// 实现winit的ApplicationHandler trait，处理窗口事件和用户输入。
/// 专门针对WGPU适配器设计。
pub struct WinitWgpuAppHandler {
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
    pub adapter_ref: *mut WinitWgpuAdapter,
}

impl ApplicationHandler for WinitWgpuAppHandler {
    /// 应用程序恢复时的回调
    ///
    /// 在resumed事件中创建窗口和渲染资源。
    /// 这是统一的生命周期管理方式，适用于两种渲染后端。
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // 在这里创建窗口和渲染上下文
        if let Some(adapter) = unsafe { self.adapter_ref.as_mut() } {
            if adapter.window.is_none() {
                adapter.create_wgpu_window_and_resources(event_loop);
            }

            // 延迟设置光标 - 在窗口完全初始化后进行
            if !adapter.cursor_set {
                // 在设置光标前先清屏 - 可能有助于解决透明度问题
                adapter.clear_screen_wgpu();

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

impl WinitWgpuAdapter {
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
        // 使用统一的winit共享初始化逻辑
        let (event_loop, window_init_params, texture_path) =
            winit_init_common(self, w, h, rx, ry, title);

        // 存储共享初始化结果
        self.event_loop = Some(event_loop);
        self.window_init_params = Some(window_init_params);

        // 设置WGPU特定的应用处理器
        self.app_handler = Some(WinitWgpuAppHandler {
            pending_events: Vec::new(),
            cursor_position: (0.0, 0.0),
            ratio_x: self.base.gr.ratio_x,
            ratio_y: self.base.gr.ratio_y,
            should_exit: false,
            adapter_ref: self as *mut WinitWgpuAdapter,
        });

        texture_path
    }

    /// WGPU 后端初始化
    ///
    /// 使用统一的生命周期管理，窗口创建推迟到resumed事件中。
    fn init_wgpu(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing Winit adapter with WGPU backend...");
        let _texture_path = self.init_common(w, h, rx, ry, title);
        // 窗口创建将在resumed事件中完成
    }

    /// 在resumed事件中创建WGPU窗口和相关资源
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
                pixel_renderer.set_clear_color(UnifiedColor::new(1.0, 0.0, 0.0, 1.0));
            } else {
                // 正常模式使用黑色背景
                pixel_renderer.set_clear_color(UnifiedColor::new(0.0, 0.0, 0.0, 1.0));
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

            // 绘制render texture 2（主缓冲区）到屏幕 - 使用底层WGPU方法
            if !pixel_renderer.get_render_texture_hidden(2) {
                let unified_transform = UnifiedTransform::new();
                let unified_color = UnifiedColor::white();

                pixel_renderer.render_texture_to_screen_impl(
                    device,
                    queue,
                    &mut screen_encoder,
                    &view,
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

                pixel_renderer.render_texture_to_screen_impl(
                    device,
                    queue,
                    &mut screen_encoder,
                    &view,
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

impl Adapter for WinitWgpuAdapter {
    /// 初始化Winit + WGPU适配器
    ///
    /// 这是适配器的主要初始化方法，专门使用WGPU现代渲染管线。
    ///
    /// # 参数
    /// - `w`: 逻辑宽度（字符数）
    /// - `h`: 逻辑高度（字符数）
    /// - `rx`: X轴缩放比例
    /// - `ry`: Y轴缩放比例
    /// - `title`: 窗口标题
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String) {
        info!("Initializing WinitWgpu adapter with WGPU backend...");
        self.init_wgpu(w, h, rx, ry, title);
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
    /// 使用WGPU现代渲染管线绘制当前帧。
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
        // 处理窗口拖拽移动
        winit_move_win(
            &mut self.drag.need,
            self.window.as_ref().map(|v| &**v),
            self.drag.dx,
            self.drag.dy,
        );

        // 使用统一的图形渲染流程 - 与SdlAdapter和WinitGlowAdapter保持一致
        self.draw_all_graph(current_buffer, previous_buffer, pixel_sprites, stage);

        self.post_draw();
        Ok(())
    }

    fn post_draw(&mut self) {
        // WGPU模式不需要显式的缓冲区交换，在draw_render_textures_to_screen_wgpu中已经调用了present()
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

    /// 重写渲染缓冲区到纹理的方法，直接使用我们的WGPU渲染器
    ///
    /// 这个方法专门为WinitWgpuAdapter实现，不依赖统一的pixel_renderer抽象
    fn draw_render_buffer_to_texture(
        &mut self,
        rbuf: &[crate::render::adapter::RenderCell],
        rtidx: usize,
        debug: bool,
    ) where
        Self: Sized,
    {
        // 直接调用我们的WGPU渲染方法
        if let Err(e) = self.draw_render_buffer_to_texture_wgpu(rbuf, rtidx, debug) {
            eprintln!(
                "WinitWgpuAdapter: Failed to render buffer to texture {}: {}",
                rtidx, e
            );
        }
    }

    /// 重写渲染纹理到屏幕的方法，直接使用我们的WGPU渲染器
    ///
    /// 这个方法专门为WinitWgpuAdapter实现，处理转场效果的最终合成
    fn draw_render_textures_to_screen(&mut self)
    where
        Self: Sized,
    {
        // 直接调用我们的WGPU渲染方法
        if let Err(e) = self.draw_render_textures_to_screen_wgpu() {
            eprintln!(
                "WinitWgpuAdapter: Failed to render textures to screen: {}",
                e
            );
        }
    }

    /// WinitWgpu adapter implementation of render texture visibility control
    fn set_render_texture_visible(&mut self, texture_index: usize, visible: bool) {
        if let Some(wgpu_pixel_renderer) = &mut self.wgpu_pixel_renderer {
            wgpu_pixel_renderer.set_render_texture_hidden(texture_index, !visible);
        }
    }

    /// WinitWgpu adapter implementation of simple transition rendering
    fn render_simple_transition(&mut self, target_texture: usize) {
        // WGPU使用标准的转场渲染，参数为(target, shader=0, progress=1.0)
        if let Err(e) = self.render_transition_to_texture_wgpu(target_texture, 0, 1.0) {
            eprintln!("WinitWgpuAdapter: Simple transition error: {}", e);
        }
    }

    /// WinitWgpu adapter implementation of advanced transition rendering
    fn render_advanced_transition(
        &mut self,
        target_texture: usize,
        effect_type: usize,
        progress: f32,
    ) {
        // WGPU使用完整的转场渲染API
        if let Err(e) =
            self.render_transition_to_texture_wgpu(target_texture, effect_type, progress)
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
}
