//! # 图形模式渲染模块
//!
//! 本模块包含了图形模式渲染相关的数据结构、常量和函数。
//! 抽取自adapter.rs，专门处理像素级渲染、纹理管理、精灵渲染等功能。

use crate::{
    render::{AdapterBase, buffer::Buffer, sprite::Sprites, style::Color},
    util::{ARect, PointF32, PointI32, PointU16, Rand},
    LOGO_FRAME,
};
use std::sync::OnceLock;

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
use crate::render::adapter::gl::pixel::GlPixel;

/// 符号纹理文件路径
///
/// 符号纹理包含8x8块，每块包含16x16符号，总共128×128符号。
/// 这个纹理作为渲染文本和符号的字符图集。
///
/// 布局:
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                   Symbols Texture Layout                    │
/// │                                                             │
/// │  ┌─────────┬─────────┬─────────┬─────────┬─────────┐       │
/// │  │Block 0,0│Block 1,0│Block 2,0│Block 3,0│Block 4,0│ ...   │
/// │  │16x16    │16x16    │16x16    │16x16    │16x16    │       │
/// │  │Symbols  │Symbols  │Symbols  │Symbols  │Symbols  │       │
/// │  ├─────────┼─────────┼─────────┼─────────┼─────────┤       │
/// │  │Block 0,1│Block 1,1│Block 2,1│Block 3,1│Block 4,1│ ...   │
/// │  │16x16    │16x16    │16x16    │16x16    │16x16    │       │
/// │  │Symbols  │Symbols  │Symbols  │Symbols  │Symbols  │       │
/// │  └─────────┴─────────┴─────────┴─────────┴─────────┘       │
/// │                           ...                               │
/// └─────────────────────────────────────────────────────────────┘
/// ```
pub const PIXEL_TEXTURE_FILE: &str = "assets/pix/symbols.png";

/// 符号宽度静态变量（懒初始化）
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();

/// 符号高度静态变量（懒初始化）
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();

/// 根据纹理宽度计算符号宽度
///
/// # 参数
/// - `width`: 纹理总宽度
///
/// # 返回值
/// 单个符号的宽度
pub fn init_sym_width(width: u32) -> f32 {
    width as f32 / (16.0 * 8.0)
}

/// 根据纹理高度计算符号高度
///
/// # 参数
/// - `height`: 纹理总高度
///
/// # 返回值
/// 单个符号的高度
pub fn init_sym_height(height: u32) -> f32 {
    height as f32 / (16.0 * 8.0)
}

/// Logo显示宽度（字符数）
pub const PIXEL_LOGO_WIDTH: usize = 27;

/// Logo显示高度（字符数）
///
/// Logo在启动时显示，用于展示项目标识。
/// 使用RGB格式存储，每个像素3个字节。
pub const PIXEL_LOGO_HEIGHT: usize = 12;

/// RustPixel Logo数据
///
/// 预定义的Logo图像数据，RGB格式，每个像素3字节。
/// 在游戏启动阶段显示，提供品牌识别。
///
/// 数据格式：[R, G, B, R, G, B, ...]
/// 尺寸：27 × 12 像素
pub const PIXEL_LOGO: [u8; PIXEL_LOGO_WIDTH * PIXEL_LOGO_HEIGHT * 3] = [
    32, 15, 1, 32, 202, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 239, 1, 32, 15, 1, 100, 239, 1, 32,
    239, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 165, 1, 32,
    165, 0, 32, 87, 1, 32, 15, 1, 18, 202, 1, 21, 202, 1, 19, 202, 1, 20, 202, 1, 32, 15, 1, 47,
    239, 1, 47, 239, 1, 116, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 87, 1, 32, 165, 0, 32, 165, 1, 32, 240, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 100, 239, 1, 81, 49, 1, 47, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15,
    1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15, 0, 32, 87, 1, 32, 15, 0, 32, 165, 0,
    47, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 238, 1, 47,
    238, 1, 47, 238, 1, 47, 239, 1, 100, 239, 1, 46, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1,
    104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1, 84,
    239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160,
    49, 1, 81, 49, 1, 32, 15, 1, 160, 86, 1, 32, 15, 1, 160, 49, 1, 47, 236, 1, 47, 236, 1, 46,
    234, 1, 160, 49, 1, 47, 239, 1, 81, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 47,
    239, 1, 160, 49, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 87, 1, 160, 45,
    1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 160, 45, 1, 32, 15, 1, 160, 45, 1, 32, 235, 1, 116, 235, 1,
    160, 45, 1, 47, 236, 1, 160, 45, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 46, 234, 1, 32, 15, 1,
    46, 234, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 1,
    32, 15, 0, 32, 197, 1, 160, 147, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 160, 147, 1, 32, 15,
    1, 160, 147, 1, 32, 235, 1, 116, 235, 1, 46, 235, 1, 81, 147, 1, 47, 239, 1, 47, 239, 1, 100,
    239, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 47, 239, 1, 32, 15, 1, 160, 147, 1,
    32, 239, 1, 84, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 32, 239, 1, 160, 147, 1, 47,
    239, 1, 104, 239, 1, 104, 240, 1, 160, 147, 1, 32, 15, 1, 160, 147, 1, 32, 15, 1, 116, 235, 1,
    160, 147, 1, 47, 239, 1, 160, 147, 1, 47, 239, 1, 47, 239, 1, 160, 147, 1, 104, 238, 1, 104,
    238, 1, 104, 238, 1, 104, 238, 1, 47, 242, 1, 160, 147, 1, 47, 239, 1, 104, 239, 1, 104, 239,
    1, 104, 239, 1, 47, 239, 1, 84, 239, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 81,
    214, 1, 47, 239, 1, 81, 214, 1, 47, 239, 1, 160, 214, 1, 47, 239, 1, 32, 0, 1, 46, 235, 1, 160,
    214, 1, 47, 236, 1, 81, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 47, 242, 1,
    81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 47, 239, 1, 32, 165, 1, 160, 214,
    1, 103, 239, 1, 32, 242, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 0, 1,
    32, 0, 1, 32, 87, 1, 32, 87, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 165, 0, 32,
    165, 0, 160, 214, 1, 103, 239, 1, 32, 242, 1, 32, 97, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97,
    0, 32, 165, 0, 32, 15, 1, 90, 214, 1, 47, 239, 1, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32,
    0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32,
    15, 1, 32, 15, 1, 32, 15, 1,
];

/// GPU渲染单元结构
///
/// RenderCell作为游戏缓冲区和GPU渲染管线之间的中间数据格式。
/// 这种设计提供了以下优势：
///
/// ## 设计优点
/// - **GPU优化**: 数据预格式化以便高效上传到GPU
/// - **批处理**: 多个单元可以在单次绘制调用中渲染
/// - **灵活渲染**: 支持旋转、缩放和复杂效果
/// - **内存高效**: 大型场景的紧凑表示
///
/// ## 渲染管线集成
/// ```text
/// ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
/// │   Buffer    │───►│ RenderCell  │───►│ OpenGL/GPU  │
/// │(Characters) │    │   Array     │    │  Rendering  │
/// └─────────────┘    └─────────────┘    └─────────────┘
/// ```
///
/// 每个RenderCell包含渲染一个字符或精灵所需的所有信息，
/// 包括颜色、位置、旋转和纹理坐标。
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RenderCell {
    /// 前景色RGBA分量（0.0-1.0范围）
    ///
    /// 用于字符/符号渲染。Alpha分量控制透明度和混合操作。
    pub fcolor: (f32, f32, f32, f32),

    /// 可选背景色RGBA分量
    ///
    /// 存在时，在符号后面渲染彩色背景。
    /// 如果为None，背景透明。
    pub bcolor: Option<(f32, f32, f32, f32)>,

    /// 纹理和符号索引打包值
    ///
    /// - 高位：纹理索引（使用哪个纹理）
    /// - 低位：符号索引（纹理中的哪个字符/符号）
    pub texsym: usize,

    /// 屏幕坐标X位置
    pub x: f32,

    /// 屏幕坐标Y位置
    pub y: f32,

    /// 像素宽度
    pub w: u32,

    /// 像素高度
    pub h: u32,

    /// 旋转角度（弧度）
    ///
    /// 用于精灵旋转效果。0.0表示无旋转。
    pub angle: f32,

    /// 旋转中心X坐标
    ///
    /// 定义旋转发生的轴心点。
    pub cx: f32,

    /// 旋转中心Y坐标
    ///
    /// 定义旋转发生的轴心点。
    pub cy: f32,
}

pub struct Graph {
    /// Physical window width in pixels
    pub pixel_w: u32,

    /// Physical window height in pixels
    pub pixel_h: u32,

    /// Horizontal scaling ratio for different DPI displays
    ///
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_x: f32,

    /// Vertical scaling ratio for different DPI displays
    ///
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_y: f32,

    /// Render flag controlling immediate vs buffered rendering
    ///
    /// - true: Direct rendering to screen (normal mode)
    /// - false: Buffered rendering for external access (used for FFI/WASM)
    pub rflag: bool,

    /// Render buffer storing RenderCell array for buffered mode
    ///
    /// When rflag is false, rendered data is stored rbuf instead of
    /// being directly drawn to screen. Used for external access to
    /// rendering data (e.g., Python FFI, WASM exports).
    pub rbuf: Vec<RenderCell>,

    /// OpenGL context handle
    ///
    /// Provides access to OpenGL functions for rendering operations.
    /// Uses the glow crate for cross-platform OpenGL abstraction.
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    pub gl: Option<glow::Context>,

    /// OpenGL pixel renderer instance
    ///
    /// High-level OpenGL rendering interface that manages shaders,
    /// textures, and render targets for the pixel-based rendering pipeline.
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    pub gl_pixel: Option<GlPixel>,
}

impl Graph {
    /// 创建新的图形渲染上下文
    ///
    /// 初始化所有图形模式相关的数据结构和渲染状态。
    /// 渲染标志默认为true（直接渲染到屏幕）。
    pub fn new() -> Self {
        Self {
            pixel_w: 0,
            pixel_h: 0,
            ratio_x: 1.0,
            ratio_y: 1.0,
            rflag: true,
            rbuf: Vec::new(),
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            gl: None,
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            gl_pixel: None,
        }
    }

    /// 设置X轴缩放比例
    ///
    /// 用于处理不同DPI显示器的缩放适配。
    /// 该值会影响像素宽度计算和渲染坐标转换。
    ///
    /// # 参数
    /// - `rx`: X轴缩放比例（1.0为标准比例）
    pub fn set_ratiox(&mut self, rx: f32) {
        self.ratio_x = rx;
    }

    /// 设置Y轴缩放比例
    ///
    /// 用于处理不同DPI显示器的缩放适配。
    /// 该值会影响像素高度计算和渲染坐标转换。
    ///
    /// # 参数
    /// - `ry`: Y轴缩放比例（1.0为标准比例）
    pub fn set_ratioy(&mut self, ry: f32) {
        self.ratio_y = ry;
    }

    /// 根据当前设置计算并设置像素尺寸
    ///
    /// 基于单元格数量、符号尺寸和缩放比例计算实际的像素宽度和高度。
    /// 这是图形模式窗口大小计算的核心方法。
    ///
    /// # 参数
    /// - `cell_w`: 游戏区域宽度（字符单元格数）
    /// - `cell_h`: 游戏区域高度（字符单元格数）
    ///
    /// # 计算公式
    /// ```text
    /// pixel_w = (cell_w + 2) * symbol_width / ratio_x
    /// pixel_h = (cell_h + 2) * symbol_height / ratio_y
    /// ```
    /// 其中 +2 是为了边框预留空间
    pub fn set_pixel_size(&mut self, cell_w: u16, cell_h: u16) {
        self.pixel_w = ((cell_w + 2) as f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init")
            / self.ratio_x) as u32;
        self.pixel_h = ((cell_h + 2) as f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init")
            / self.ratio_y) as u32;
    }

    /// 获取单个字符单元格的宽度（像素）
    ///
    /// 基于符号纹理尺寸和当前X轴缩放比例计算单个字符单元格的实际像素宽度。
    /// 这个值用于精确的位置计算和渲染布局。
    ///
    /// # 返回值
    /// 单个字符单元格的像素宽度
    pub fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH.get().expect("lazylock init") / self.ratio_x
    }

    /// 获取单个字符单元格的高度（像素）
    ///
    /// 基于符号纹理尺寸和当前Y轴缩放比例计算单个字符单元格的实际像素高度。
    /// 这个值用于精确的位置计算和渲染布局。
    ///
    /// # 返回值
    /// 单个字符单元格的像素高度
    pub fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT.get().expect("lazylock init") / self.ratio_y
    }
}

/// Convert game data to RenderCell format with texture coordinate calculation
///
/// This function converts individual game elements (characters, sprites, etc.) into
/// GPU-ready RenderCell format. It handles texture coordinate calculation, color
/// conversion, and transformation parameters.
///
/// ## Conversion Process
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Data Transformation                        │
/// │                                                             │
/// │  Game Data Input:                                           │
/// │  ├── Colors (u8 RGBA) ────────► Normalized (f32 RGBA)       │
/// │  ├── Texture & Symbol Index ──► Packed texsym value         │
/// │  ├── Screen Rectangle ─────────► Position & dimensions      │
/// │  ├── Rotation angle ───────────► Angle + center point       │
/// │  └── Background color ─────────► Optional background        │
/// │                                                             │
/// │                       ▼                                     │
/// │               ┌─────────────────────┐                       │
/// │               │    RenderCell       │                       │
/// │               │   (GPU-ready)       │                       │
/// │               └─────────────────────┘                       │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// # Parameters
/// - `rbuf`: Target RenderCell vector to append to
/// - `fc`: Foreground color as (R,G,B,A) in 0-255 range
/// - `bgc`: Optional background color
/// - `texidx`: Texture index in the texture atlas
/// - `symidx`: Symbol index within the texture
/// - `s`: Screen rectangle (position and size)
/// - `angle`: Rotation angle in radians
/// - `ccp`: Center point for rotation
pub fn push_render_buffer(
    rbuf: &mut Vec<RenderCell>,
    fc: &(u8, u8, u8, u8),
    bgc: &Option<(u8, u8, u8, u8)>,
    texidx: usize,
    symidx: usize,
    s: ARect,
    angle: f64,
    ccp: &PointI32,
) {
    let mut wc = RenderCell {
        fcolor: (
            fc.0 as f32 / 255.0,
            fc.1 as f32 / 255.0,
            fc.2 as f32 / 255.0,
            fc.3 as f32 / 255.0,
        ),
        ..Default::default()
    };
    if let Some(bc) = bgc {
        wc.bcolor = Some((
            bc.0 as f32 / 255.0,
            bc.1 as f32 / 255.0,
            bc.2 as f32 / 255.0,
            bc.3 as f32 / 255.0,
        ));
    } else {
        wc.bcolor = None;
    }
    let x = symidx as u32 % 16u32 + (texidx as u32 % 8u32) * 16u32;
    let y = symidx as u32 / 16u32 + (texidx as u32 / 8u32) * 16u32;
    wc.texsym = (y * 16u32 * 8u32 + x) as usize;
    wc.x = s.x as f32 + PIXEL_SYM_WIDTH.get().expect("lazylock init");
    wc.y = s.y as f32 + PIXEL_SYM_HEIGHT.get().expect("lazylock init");
    wc.w = s.w;
    wc.h = s.h;
    if angle == 0.0 {
        wc.angle = angle as f32;
    } else {
        let mut aa = (1.0 - angle / 180.0) * std::f64::consts::PI;
        let pi2 = std::f64::consts::PI * 2.0;
        while aa < 0.0 {
            aa += pi2;
        }
        while aa > pi2 {
            aa -= pi2;
        }
        wc.angle = aa as f32;
    }
    wc.cx = ccp.x as f32;
    wc.cy = ccp.y as f32;
    rbuf.push(wc);
}

pub fn render_helper(
    cell_w: u16,
    r: PointF32,
    i: usize,
    sh: &(u8, u8, Color, Color),
    p: PointU16,
    is_border: bool,
) -> (ARect, ARect, ARect, usize, usize) {
    let w = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as i32;
    let h = *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as i32;
    let dstx = i as u16 % cell_w;
    let dsty = i as u16 / cell_w;
    let tex_count = 64;
    let tx = if sh.1 < tex_count { sh.1 as usize } else { 1 };
    let srcy = sh.0 as u32 / w as u32 + (tx as u32 / 2u32) * w as u32;
    let srcx = sh.0 as u32 % w as u32 + (tx as u32 % 2u32) * w as u32;
    let bsrcy = 160u32 / w as u32;
    let bsrcx = 160u32 % w as u32 + w as u32;

    (
        // background sym rect in texture(sym=160 tex=1)
        ARect {
            x: w * bsrcx as i32,
            y: h * bsrcy as i32,
            w: w as u32,
            h: h as u32,
        },
        // sym rect in texture
        ARect {
            x: w * srcx as i32,
            y: h * srcy as i32,
            w: w as u32,
            h: h as u32,
        },
        // dst rect in render texture
        ARect {
            x: (dstx + if is_border { 0 } else { 1 }) as i32 * (w as f32 / r.x) as i32 + p.x as i32,
            y: (dsty + if is_border { 0 } else { 1 }) as i32 * (h as f32 / r.y) as i32 + p.y as i32,
            w: (w as f32 / r.x) as u32,
            h: (h as f32 / r.y) as u32,
        },
        // texture id
        tx,
        // sym id
        sh.0 as usize,
    )
}

/// Render pixel sprites with rotation and transformation support
///
/// This function processes individual sprite objects and converts them to renderable
/// format. It supports advanced features like rotation, scaling, and complex
/// transformations while maintaining efficient rendering performance.
///
/// ## Sprite Rendering Pipeline
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                   Sprite Processing                         │
/// │                                                             │
/// │  ┌─────────────┐                                            │
/// │  │   Sprite    │                                            │
/// │  │   Object    │                                            │
/// │  │  ┌───────┐  │  ┌─────────────────────────────────────┐  │
/// │  │  │Pixels │  │  │        Transformation               │  │
/// │  │  │Array  │  │  │  ┌─────────────────────────────────┐ │  │
/// │  │  └───────┘  │  │  │  1. Position calculation        │ │  │
/// │  │     │       │  │  │  2. Rotation matrix applied     │ │  │
/// │  │     ▼       │  │  │  3. Scaling based on rx/ry     │ │  │
/// │  │  ┌───────┐  │  │  │  4. Color & texture mapping    │ │  │
/// │  │  │Colors │  │  │  └─────────────────────────────────┘ │  │
/// │  │  │&Flags │  │  └─────────────────────────────────────┘  │
/// │  │  └───────┘  │                     │                     │
/// │  └─────────────┘                     ▼                     │
/// │                        ┌─────────────────────┐              │
/// │                        │  Callback Function  │              │
/// │                        │ (push_render_buffer) │              │
/// │                        └─────────────────────┘              │
/// │                                 │                           │
/// │                                 ▼                           │
/// │                        ┌─────────────────────┐              │
/// │                        │    RenderCell       │              │
/// │                        │      Array          │              │
/// │                        └─────────────────────┘              │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Features Supported
/// - **Rotation**: Full 360-degree rotation around sprite center
/// - **Scaling**: Display ratio compensation for different screen densities
/// - **Transparency**: Alpha blending and background color support
/// - **Instanced Rendering**: Efficient batch processing for multiple sprites
///
/// # Parameters
/// - `pixel_spt`: Sprite object containing pixel data and transformation info
/// - `rx`: Horizontal scaling ratio for display compensation
/// - `ry`: Vertical scaling ratio for display compensation
/// - `f`: Callback function to process each sprite pixel
pub fn render_pixel_sprites<F>(pixel_spt: &mut Sprites, rx: f32, ry: f32, mut f: F)
where
    // Callback signature: (fg_color, bg_color, bg_rect, sym_rect, dst_rect, tex_idx, sym_idx, angle, center_point)
    F: FnMut(
        &(u8, u8, u8, u8),
        &Option<(u8, u8, u8, u8)>,
        ARect,
        ARect,
        ARect,
        usize,
        usize,
        f64,
        PointI32,
    ),
{
    // sort by render_weight...
    pixel_spt.update_render_index();
    for si in &pixel_spt.render_index {
        let s = &pixel_spt.sprites[si.0];
        if s.is_hidden() {
            continue;
        }
        let px = s.content.area.x;
        let py = s.content.area.y;
        let pw = s.content.area.width;
        let ph = s.content.area.height;

        for (i, cell) in s.content.content.iter().enumerate() {
            let sh = &cell.get_cell_info();
            let (s0, s1, s2, texidx, symidx) = render_helper(
                pw,
                PointF32 { x: rx, y: ry },
                i,
                sh,
                PointU16 { x: px, y: py },
                false,
            );
            let x = i % pw as usize;
            let y = i / pw as usize;
            // center point ...
            let ccp = PointI32 {
                x: ((pw as f32 / 2.0 - x as f32) * PIXEL_SYM_WIDTH.get().expect("lazylock init")
                    / rx) as i32,
                y: ((ph as f32 / 2.0 - y as f32) * PIXEL_SYM_HEIGHT.get().expect("lazylock init")
                    / ry) as i32,
            };
            let mut fc = sh.2.get_rgba();
            fc.3 = s.alpha;
            let bc;
            if sh.3 != Color::Reset {
                let mut brgba = sh.3.get_rgba();
                brgba.3 = s.alpha;
                bc = Some(brgba);
            } else {
                bc = None;
            }
            f(&fc, &bc, s0, s1, s2, texidx, symidx, s.angle, ccp);
        }
    }
}

/// Main buffer rendering with character-to-pixel conversion
///
/// This function processes the main game buffer containing character data and
/// converts it to renderable format. It follows the principle.md design where
/// characters are the fundamental rendering unit, with each character mapped
/// to symbols in the texture atlas.
///
/// ## Buffer Rendering Process
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                   Main Buffer Processing                    │
/// │                                                             │
/// │  ┌─────────────────────┐                                   │
/// │  │      Buffer         │                                   │
/// │  │   ┌─────────────┐   │                                   │
/// │  │   │ Character   │   │    ┌─────────────────────────────┐│
/// │  │   │   Grid      │   │    │   Per-Character Process    ││
/// │  │   │             │   │    │                             ││
/// │  │   │ ┌─┬─┬─┬─┐   │   │    │ 1. Read character data      ││
/// │  │   │ │A│B│C│D│   │   │    │ 2. Extract colors & symbol  ││
/// │  │   │ ├─┼─┼─┼─┤   │───────► │ 3. Calculate screen pos     ││
/// │  │   │ │E│F│G│H│   │   │    │ 4. Map to texture coords    ││
/// │  │   │ ├─┼─┼─┼─┤   │   │    │ 5. Call render callback     ││
/// │  │   │ │I│J│K│L│   │   │    │                             ││
/// │  │   │ └─┴─┴─┴─┘   │   │    └─────────────────────────────┘│
/// │  │   └─────────────┘   │                     │             │
/// │  └─────────────────────┘                     ▼             │
/// │                                ┌─────────────────────┐     │
/// │                                │   RenderCell Array  │     │
/// │                                │   (GPU-ready data)  │     │
/// │                                └─────────────────────┘     │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Character Data Structure
/// Each character in the buffer contains:
/// - **Symbol Index**: Which character/symbol to display
/// - **Texture Index**: Which texture sheet to use
/// - **Foreground Color**: Primary character color
/// - **Background Color**: Optional background fill color
/// - **Style Flags**: Bold, italic, underline, etc.
///
/// # Parameters
/// - `buf`: Game buffer containing character grid data
/// - `width`: Buffer width in characters
/// - `rx`: Horizontal scaling ratio for display adaptation
/// - `ry`: Vertical scaling ratio for display adaptation
/// - `border`: Include border rendering (for windowed modes)
/// - `f`: Callback function to process each character
pub fn render_main_buffer<F>(buf: &Buffer, width: u16, rx: f32, ry: f32, border: bool, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    for (i, cell) in buf.content.iter().enumerate() {
        // symidx, texidx, fg, bg
        let sh = cell.get_cell_info();
        let (s0, s1, s2, texidx, symidx) = render_helper(
            width,
            PointF32 { x: rx, y: ry },
            i,
            &sh,
            PointU16 { x: 0, y: 0 },
            border,
        );
        let fc = sh.2.get_rgba();
        let bc = if sh.3 != Color::Reset {
            Some(sh.3.get_rgba())
        } else {
            None
        };
        f(&fc, &bc, s0, s1, s2, texidx, symidx);
    }
}

/// Window border rendering for windowed display modes
///
/// This function renders decorative borders around the game area for SDL and Winit
/// modes. The border provides visual separation between the game content and the
/// desktop environment, creating a more polished windowed gaming experience.
///
/// ## Border Layout
/// ```text
/// ┌───────────────────────────────────────────────────────┐
/// │                      Window Border                    │
/// │  ┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  ├─┤                 Game Content Area           ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                     80 x 40                 ├─┤  │
/// │  ├─┤                  Character Grid             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  └─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘  │
/// └───────────────────────────────────────────────────────┘
/// ```
///
/// The border consists of:
/// - **Top/Bottom Edges**: Horizontal line characters
/// - **Left/Right Edges**: Vertical line characters
/// - **Corners**: Corner junction characters
/// - **Consistent Styling**: Matches the game's visual theme
///
/// # Parameters
/// - `cell_w`: Game area width in characters
/// - `cell_h`: Game area height in characters
/// - `rx`: Horizontal scaling ratio
/// - `ry`: Vertical scaling ratio
/// - `f`: Callback function to render each border character
pub fn render_border<F>(cell_w: u16, cell_h: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    let sh_top = (102u8, 1u8, Color::Indexed(7), Color::Reset);
    let sh_other = (24u8, 2u8, Color::Indexed(7), Color::Reset);
    let sh_close = (214u8, 1u8, Color::Indexed(7), Color::Reset);

    for n in 0..cell_h as usize + 2 {
        for m in 0..cell_w as usize + 2 {
            if n != 0 && n != cell_h as usize + 1 && m != 0 && m != cell_w as usize + 1 {
                continue;
            }
            let rsh;
            if n == 0 {
                if m as u16 <= cell_w {
                    rsh = &sh_top;
                } else {
                    rsh = &sh_close;
                }
            } else {
                rsh = &sh_other;
            }
            let (s0, s1, s2, texidx, symidx) = render_helper(
                cell_w + 2,
                PointF32 { x: rx, y: ry },
                n * (cell_w as usize + 2) + m,
                rsh,
                PointU16 { x: 0, y: 0 },
                true,
            );
            let fc = rsh.2.get_rgba();
            let bc = None;
            f(&fc, &bc, s0, s1, s2, texidx, symidx);
        }
    }
}

/// RustPixel Logo animation rendering with dynamic effects
///
/// This function renders the animated RustPixel logo during the startup sequence.
/// It provides a visually appealing introduction to the framework with dynamic
/// effects and proper centering across different screen resolutions.
///
/// ## Logo Animation Sequence
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Logo Animation Timeline                  │
/// │                                                             │
/// │  Stage 0 ────────────────────────────────► LOGO_FRAME       │
/// │    │                                            │           │
/// │    ▼                                            ▼           │
/// │  ┌─────────────────┐                    ┌─────────────────┐ │
/// │  │  Logo Display   │                    │  Start Game     │ │
/// │  │                 │                    │   Rendering     │ │
/// │  │  ┌───────────┐  │                    │                 │ │
/// │  │  │ ██████    │  │   Dynamic Effects: │                 │ │
/// │  │  │ ██  ██    │  │   - Random colors  │                 │ │
/// │  │  │ ██████    │  │   - Centered pos   │                 │ │
/// │  │  │ ██  ██    │  │   - Smooth trans   │                 │ │
/// │  │  │ ██  ██    │  │   - Frame timing   │                 │ │
/// │  │  └───────────┘  │                    │                 │ │
/// │  └─────────────────┘                    └─────────────────┘ │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Rendering Features
/// - **Centered Positioning**: Automatically centers on any screen size
/// - **Dynamic Colors**: Randomly generated color effects for visual appeal
/// - **Smooth Animation**: Frame-based timing for consistent display
/// - **High-DPI Support**: Proper scaling for different display densities
/// - **Cross-platform**: Works consistently across SDL, Winit, and Web modes
///
/// ## Logo Data Processing
/// The function processes the PIXEL_LOGO constant array where each character
/// is represented by 3 bytes: [symbol_id, texture_id, flags]. The logo is
/// dynamically positioned and colored based on the current animation stage.
///
/// # Parameters
/// - `srx`: Screen horizontal scaling ratio
/// - `sry`: Screen vertical scaling ratio
/// - `spw`: Screen physical width in pixels
/// - `sph`: Screen physical height in pixels
/// - `rd`: Random number generator for color effects
/// - `stage`: Current animation stage (0 to LOGO_FRAME)
/// - `f`: Callback function to render each logo character
pub fn render_logo<F>(srx: f32, sry: f32, spw: u32, sph: u32, rd: &mut Rand, stage: u32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, ARect, usize, usize),
{
    let rx = srx * 1.0;
    let ry = sry * 1.0;
    for y in 0usize..PIXEL_LOGO_HEIGHT {
        for x in 0usize..PIXEL_LOGO_WIDTH {
            let sci = y * PIXEL_LOGO_WIDTH + x;
            let symw = PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
            let symh = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

            let (_s0, s1, mut s2, texidx, symidx) = render_helper(
                PIXEL_LOGO_WIDTH as u16,
                PointF32 { x: rx, y: ry },
                sci,
                &(
                    PIXEL_LOGO[sci * 3],
                    PIXEL_LOGO[sci * 3 + 2],
                    Color::Indexed(PIXEL_LOGO[sci * 3 + 1]),
                    Color::Reset,
                ),
                PointU16 {
                    x: spw as u16 / 2 - (PIXEL_LOGO_WIDTH as f32 / 2.0 * symw) as u16,
                    y: sph as u16 / 2 - (PIXEL_LOGO_HEIGHT as f32 / 2.0 * symh) as u16,
                },
                false,
            );
            let fc = Color::Indexed(PIXEL_LOGO[sci * 3 + 1]).get_rgba();

            let randadj = 12 - (rd.rand() % 24) as i32;
            let sg = LOGO_FRAME as u8 / 3;
            let r: u8;
            let g: u8;
            let b: u8;
            let a: u8;
            if stage <= sg as u32 {
                r = (stage as u8).saturating_mul(10);
                g = (stage as u8).saturating_mul(10);
                b = (stage as u8).saturating_mul(10);
                a = 255;
                s2.x += randadj;
            } else if stage <= sg as u32 * 2 {
                r = fc.0;
                g = fc.1;
                b = fc.2;
                a = 255;
            } else {
                let cc = (stage as u8 - sg * 2).saturating_mul(10);
                r = fc.0.saturating_sub(cc);
                g = fc.1.saturating_sub(cc);
                b = fc.2.saturating_sub(cc);
                a = 255;
            }
            f(&(r, g, b, a), s1, s2, texidx, symidx);
        }
    }
}

    // merge main buffer & pixel sprites to render buffer...
pub fn generate_render_buffer(
        cb: &Buffer,
        _pb: &Buffer,
        ps: &mut Vec<Sprites>,
        stage: u32,
        base: &mut AdapterBase,
    ) -> Vec<RenderCell> {
        let mut rbuf = vec![];
        let width = cb.area.width;
        let pz = PointI32 { x: 0, y: 0 };

        // render logo...
        if stage <= LOGO_FRAME {
            render_logo(
                base.gr.ratio_x,
                base.gr.ratio_y,
                base.gr.pixel_w,
                base.gr.pixel_h,
                &mut base.rd,
                stage,
                |fc, _s1, s2, texidx, symidx| {
                    push_render_buffer(&mut rbuf, fc, &None, texidx, symidx, s2, 0.0, &pz);
                },
            );
            return rbuf;
        }

        let cw = base.cell_w;
        let ch = base.cell_h;
        let rx = base.gr.ratio_x;
        let ry = base.gr.ratio_y;
        let mut rfunc = |fc: &(u8, u8, u8, u8),
                         bc: &Option<(u8, u8, u8, u8)>,
                         _s0: ARect,
                         _s1: ARect,
                         s2: ARect,
                         texidx: usize,
                         symidx: usize| {
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };

        // render windows border, for sdl, winit and wgpu mode
        // #[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu"))]
        render_border(cw, ch, rx, ry, &mut rfunc);

        // render main buffer...
        if stage > LOGO_FRAME {
            render_main_buffer(cb, width, rx, ry, false, &mut rfunc);
        }

        // render pixel_sprites...
        if stage > LOGO_FRAME {
            for item in ps {
                if item.is_pixel && !item.is_hidden {
                    render_pixel_sprites(
                        item,
                        rx,
                        ry,
                        |fc, bc, _s0, _s1, s2, texidx, symidx, angle, ccp| {
                            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, angle, &ccp);
                        },
                    );
                }
            }
        }
        rbuf
    }


