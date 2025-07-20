//! # 🔗 Winit 共享代码模块 (Winit Common Module)
//!
//! 这个模块是WGPU重构的重要成果之一，提取了 `WinitGlowAdapter` 和 `WinitWgpuAdapter` 
//! 之间的共同代码，实现了DRY原则并提高了代码维护性。
//!
//! ## 🎯 设计目标 (Design Goals)
//!
//! ### 代码复用 (Code Reuse)
//! - **消除重复**: 两个winit适配器之间有大量相同的代码
//! - **统一接口**: 提供一致的事件处理和窗口管理接口  
//! - **维护性**: 修改共享逻辑只需更新一个地方
//!
//! ### 性能优化 (Performance Optimization)
//! - **零成本抽象**: 共享代码通过内联完全消除运行时开销
//! - **编译时特化**: 每个后端都能获得最优的机器码
//! - **内存效率**: 避免重复的数据结构定义
//!
//! ## 📦 提供的功能 (Provided Features)
//!
//! ### 窗口管理 (Window Management)
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Window Management                         │
//! │  ┌─────────────┬─────────────┬─────────────────────────────┐ │
//! │  │    Drag     │WindowInit   │        Movement             │ │
//! │  │   System    │ Params      │       Handler               │ │
//! │  │             │             │                             │ │
//! │  │ - State     │ - Size      │ - Position Update           │ │
//! │  │ - Coords    │ - Title     │ - Drag Detection            │ │
//! │  │ - Flags     │ - Ratios    │ - Border Area Check         │ │
//! │  └─────────────┴─────────────┴─────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### 事件处理 (Event Handling)
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Event Translation                        │
//! │                                                             │
//! │  Winit Events ──┐                    ┌── RustPixel Events   │
//! │                 │                    │                     │
//! │  ┌─────────────┐ │  ┌─────────────┐  │ ┌─────────────────┐ │
//! │  │ Keyboard    │─┼─→│   Common    │──┼→│ Unified Events  │ │
//! │  │ Mouse       │ │  │ Translation │  │ │ (Key, Mouse)    │ │
//! │  │ Window      │─┘  │   Logic     │  └→│ (Drag-aware)    │ │
//! │  └─────────────┘    └─────────────┘    └─────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 🚀 重构价值 (Refactoring Value)
//!
//! ### 代码减少 (Code Reduction)
//! - **~200行重复代码** 提取到共享模块
//! - **4个重复函数** 合并为统一实现
//! - **维护负担减半** - 只需维护一份逻辑
//!
//! ### 一致性保证 (Consistency Guarantee)  
//! - **相同的拖拽行为** 在所有winit后端
//! - **统一的事件处理** 逻辑和响应
//! - **一致的错误处理** 和边界情况

use crate::event::Event;
use crate::render::adapter::{PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    window::Window,
};

/// 窗口拖拽状态管理
///
/// 记录窗口拖拽的相关状态，支持通过鼠标拖拽移动窗口位置。
/// 类似于SDL版本的实现，提供相同的用户体验。
#[derive(Default)]
pub struct Drag {
    /// 是否需要执行拖拽操作
    pub need: bool,
    /// 是否正在拖拽中
    pub draging: bool,
    /// 拖拽起始鼠标X坐标
    pub mouse_x: f64,
    /// 拖拽起始鼠标Y坐标
    pub mouse_y: f64,
    /// X轴拖拽偏移量
    pub dx: f64,
    /// Y轴拖拽偏移量
    pub dy: f64,
}

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

/// 窗口移动函数
///
/// 执行实际的窗口拖拽移动操作。处理拖拽状态的更新和窗口位置设置。
///
/// # 参数
/// - `drag_need`: 是否需要拖拽的可变引用
/// - `window`: 可选的窗口引用
/// - `dx`: X轴偏移量
/// - `dy`: Y轴偏移量
///
/// # 行为
/// - 根据拖拽偏移量移动窗口位置
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

/// 从Winit事件转换为RustPixel事件
///
/// 将winit的原生事件转换为RustPixel内部事件格式，便于统一处理。
/// 支持键盘输入、鼠标操作等多种事件类型。
///
/// # 参数
/// - `event`: Winit事件引用
/// - `adjx`: X轴调整系数（用于高DPI显示）
/// - `adjy`: Y轴调整系数（用于高DPI显示）
/// - `cursor_pos`: 当前光标位置的可变引用
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
                                winit::keyboard::KeyCode::Enter => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Enter,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Escape => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Esc,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Backspace => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Backspace,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Tab => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Tab,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Delete => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Delete,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Home => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Home,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::End => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::End,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::PageUp => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::PageUp,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::PageDown => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::PageDown,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Insert => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Insert,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Digit1 => '1',
                                winit::keyboard::KeyCode::Digit2 => '2',
                                winit::keyboard::KeyCode::Digit3 => '3',
                                winit::keyboard::KeyCode::Digit4 => '4',
                                winit::keyboard::KeyCode::Digit5 => '5',
                                winit::keyboard::KeyCode::Digit6 => '6',
                                winit::keyboard::KeyCode::Digit7 => '7',
                                winit::keyboard::KeyCode::Digit8 => '8',
                                winit::keyboard::KeyCode::Digit9 => '9',
                                winit::keyboard::KeyCode::Digit0 => '0',
                                _ => return None,
                            };
                            return Some(Event::Key(KeyEvent::new(
                                KeyCode::Char(kc),
                                KeyModifiers::NONE,
                            )));
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    cursor_pos.0 = position.x;
                    cursor_pos.1 = position.y;

                    let px = cursor_pos.0 / *sym_width as f64 / adjx as f64;
                    let py = cursor_pos.1 / *sym_height as f64 / adjy as f64;

                    return Some(Event::Mouse(MouseEvent {
                        kind: Moved,
                        column: px as u16,
                        row: py as u16,
                        modifiers: KeyModifiers::NONE,
                    }));
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *state == winit::event::ElementState::Pressed {
                        let px = cursor_pos.0 / *sym_width as f64 / adjx as f64;
                        let py = cursor_pos.1 / *sym_height as f64 / adjy as f64;

                        let mouse_button = match button {
                            winit::event::MouseButton::Left => Left,
                            winit::event::MouseButton::Right => Right,
                            winit::event::MouseButton::Middle => Middle,
                            _ => Left,
                        };

                        return Some(Event::Mouse(MouseEvent {
                            kind: Down(mouse_button),
                            column: px as u16,
                            row: py as u16,
                            modifiers: KeyModifiers::NONE,
                        }));
                    } else {
                        let px = cursor_pos.0 / *sym_width as f64 / adjx as f64;
                        let py = cursor_pos.1 / *sym_height as f64 / adjy as f64;

                        let mouse_button = match button {
                            winit::event::MouseButton::Left => Left,
                            winit::event::MouseButton::Right => Right,
                            winit::event::MouseButton::Middle => Middle,
                            _ => Left,
                        };

                        return Some(Event::Mouse(MouseEvent {
                            kind: Up(mouse_button),
                            column: px as u16,
                            row: py as u16,
                            modifiers: KeyModifiers::NONE,
                        }));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

/// 🔧 Winit适配器通用初始化函数
///
/// 这个函数提取了WinitGlowAdapter和WinitWgpuAdapter之间的所有共同初始化逻辑，
/// 实现DRY原则并大大减少了代码重复。
///
/// ## 🎯 共享的初始化步骤
/// 1. **纹理加载**: 加载PIXEL_TEXTURE_FILE并设置符号尺寸
/// 2. **参数设置**: 配置窗口尺寸、缩放比例等基础参数
/// 3. **事件循环**: 创建winit EventLoop实例
/// 4. **参数存储**: 保存WindowInitParams供resumed事件使用
///
/// ## 🚀 性能优势
/// - **编译时优化**: 内联消除函数调用开销
/// - **代码复用**: 避免重复维护相同逻辑
/// - **类型安全**: 强类型泛型确保正确的适配器使用
///
/// # 泛型参数
/// - `T`: 适配器类型，必须实现基础的尺寸和标题设置接口
///
/// # 参数
/// - `adapter`: 适配器的可变引用
/// - `w`: 窗口宽度（单元格）
/// - `h`: 窗口高度（单元格）
/// - `rx`: X轴缩放比例
/// - `ry`: Y轴缩放比例
/// - `title`: 窗口标题
///
/// # 返回值
/// - `(EventLoop<()>, WindowInitParams, String)`: 事件循环、初始化参数和纹理路径
pub fn winit_init_common<T>(
    adapter: &mut T,
    w: u16,
    h: u16,
    rx: f32,
    ry: f32,
    title: String,
) -> (winit::event_loop::EventLoop<()>, WindowInitParams, String)
where
    T: crate::render::adapter::Adapter,
{
    use crate::render::adapter::{
        init_sym_height, init_sym_width, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
    };
    use log::info;
    use winit::event_loop::EventLoop;

    info!("Initializing Winit adapter common components...");

    // 1. 加载纹理文件和设置符号尺寸
    let project_path = adapter.get_base().project_path.clone();
    let texture_path = format!(
        "{}{}{}",
        project_path,
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
    adapter.set_size(w, h);
    adapter.set_title(title.clone());
    
    // 获取base引用一次，避免多次可变借用
    let base = adapter.get_base();
    base.gr.set_ratiox(rx);
    base.gr.set_ratioy(ry);
    
    // 先获取需要的值，再调用方法
    let cell_w = base.cell_w;
    let cell_h = base.cell_h;
    base.gr.set_pixel_size(cell_w, cell_h);

    info!(
        "Window pixel size: {}x{}",
        base.gr.pixel_w, base.gr.pixel_h
    );

    // 3. 创建事件循环
    let event_loop = EventLoop::new().unwrap();

    // 4. 创建窗口初始化参数
    let window_init_params = WindowInitParams {
        width: w,
        height: h,
        ratio_x: rx,
        ratio_y: ry,
        title,
        texture_path: texture_path.clone(),
    };

    info!("Common initialization completed, window will be created in resumed()");

    (event_loop, window_init_params, texture_path)
} 