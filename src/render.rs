// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # 渲染模块
//!
//! 支持两种渲染模式：文本模式和图形模式。
//!
//! ## 子模块
//! - `adapter`: 渲染适配器接口（crossterm、sdl、web、winit）
//! - `cell`: 基础绘制单元，即一个字符
//! - `buffer`: 由cells组成的向量，管理屏幕缓冲区
//! - `sprite`: 基础绘制组件，进一步封装buffer
//! - `style`: 定义绘制属性，如前景色和背景色
//! - `panel`: 绘制面板，兼容文本模式和图形模式
//! - `graph`: 图形渲染相关的数据结构和函数
//! - `image`: 图像处理功能
//! - `symbols`: 符号和字符处理

pub mod adapter;
pub mod buffer;
pub mod cell;
pub mod graph;
pub mod image;
pub mod panel;
pub mod sprite;
pub mod style;
pub mod symbols;

// 重新导出常用类型和函数
pub use adapter::{Adapter, AdapterBase};
pub use buffer::Buffer;
pub use cell::Cell;
pub use graph::{
    init_sym_height, init_sym_width, push_render_buffer, render_border, render_logo,
    render_main_buffer, render_pixel_sprites, RenderCell, PIXEL_LOGO, PIXEL_LOGO_HEIGHT,
    PIXEL_LOGO_WIDTH, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
};
pub use panel::Panel;
pub use sprite::Sprites;
pub use style::{Color, Style};
