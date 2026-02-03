// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # Visual Effects System
//!
//! This module provides a unified system for visual effects, supporting both:
//! - **CPU Effects (BufferEffect)**: Cell-level effects on Buffer
//! - **GPU Effects (GpuTransition)**: Pixel-level shader effects on RenderTextures
//!
//! ## Architecture
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     Visual Effects System                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  CPU Effects (BufferEffect trait)                                    │
//! │    - 操作对象: Buffer (字符级别)                                      │
//! │    - 执行位置: CPU                                                    │
//! │    - apply(&self, src: &Buffer, dst: &mut Buffer, params)            │
//! │                                                                      │
//! │  Built-in CPU Effects:                                               │
//! │    - WaveEffect      (水平波浪扭曲)                                   │
//! │    - RippleEffect    (中心涟漪扩散)                                   │
//! │    - SwirlEffect     (漩涡旋转)                                       │
//! │    - NoiseEffect     (随机噪点)                                       │
//! │    - FadeEffect      (透明度渐变)                                     │
//! │    - PixelateEffect  (马赛克像素化)                                   │
//! │    - BlurEffect      (简单模糊)                                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  GPU Effects (GpuTransition enum)                                    │
//! │    - 操作对象: RenderTexture (像素级别)                               │
//! │    - 执行位置: GPU (via GLSL shaders)                                 │
//! │    - 通过 Adapter::blend_rts() 调用                                   │
//! │                                                                      │
//! │  Built-in GPU Transitions (7种):                                     │
//! │    - Squares     (方格渐变)                                           │
//! │    - Heart       (心形展开)                                           │
//! │    - Noise       (噪点过渡)                                           │
//! │    - RotateZoom  (旋转缩放)                                           │
//! │    - Bounce      (弹跳波浪)                                           │
//! │    - Dispersion  (色散分离)                                           │
//! │    - Ripple      (涟漪扩散)                                           │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  EffectChain: 组合多个CPU特效                                         │
//! │  GpuBlendEffect: GPU混合特效描述                                      │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage Example
//! ```ignore
//! use rust_pixel::render::effect::*;
//!
//! // === CPU Effects (Buffer级别) ===
//! let wave = WaveEffect::new(0.03, 15.0);
//! let ripple = RippleEffect::new(0.05, 10.0);
//! let mut chain = EffectChain::new();
//! chain.add(Box::new(wave));
//! chain.add(Box::new(ripple));
//! let params = EffectParams::new(0.5, stage);
//! chain.apply(&src_buffer, &mut dst_buffer, &params);
//!
//! // === GPU Effects (RenderTexture级别) ===
//! let gpu_effect = GpuBlendEffect::ripple(0.5);
//! ctx.adapter.blend_rts(
//!     0, 1, 2,                    // src1, src2, dst RT索引
//!     gpu_effect.effect_type(),   // 特效类型
//!     gpu_effect.progress         // 进度
//! );
//! ```

use crate::render::buffer::Buffer;
use crate::render::cell::cellsym;
use crate::render::style::Color;

// ============================================================================
// GPU Transition Effects (Shader-based, 像素级别)
// ============================================================================

/// GPU过渡特效类型
///
/// 这些特效在GPU上通过GLSL着色器执行，用于两个RenderTexture之间的混合过渡。
/// 对应 `shader_source.rs` 中的 `TRANS_FS` 着色器数组。
///
/// # 工作原理
/// ```text
/// ┌─────────────┐     ┌─────────────┐
/// │   RT1 (源1) │     │   RT2 (源2) │
/// └──────┬──────┘     └──────┬──────┘
///        │                   │
///        └─────────┬─────────┘
///                  ▼
///        ┌─────────────────────┐
///        │   GPU Transition    │  ← progress (0.0~1.0)
///        │   (GLSL Shader)     │
///        └──────────┬──────────┘
///                   ▼
///           ┌─────────────┐
///           │   RT (目标)  │
///           └─────────────┘
/// ```
///
/// # 使用方式
/// ```ignore
/// let transition = GpuTransition::Ripple;
/// ctx.adapter.blend_rts(
///     src_rt1,           // 源RT1索引 (过渡起始画面)
///     src_rt2,           // 源RT2索引 (过渡目标画面)
///     dst_rt,            // 目标RT索引 (混合结果)
///     transition.into(), // 特效类型转为usize
///     progress,          // 进度 0.0(全src1) ~ 1.0(全src2)
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum GpuTransition {
    /// 方格渐变过渡 (Squares Grid)
    ///
    /// 画面被分割成动态大小的方格，方格内容逐渐从源图像过渡到目标图像。
    /// 方格大小随进度变化，产生像素化的过渡效果。
    ///
    /// 适合场景：复古风格场景切换、像素艺术游戏
    Squares = 0,

    /// 心形展开过渡 (Heart Shape)
    ///
    /// 从画面中心以心形向外展开，逐渐显示目标图像。
    /// 使用数学公式生成心形轮廓。
    ///
    /// 适合场景：恋爱游戏、节日活动、特殊奖励
    Heart = 1,

    /// 噪点过渡 (Static Noise)
    ///
    /// 通过随机噪点实现电视雪花般的过渡效果。
    /// 在过渡中间阶段显示纯噪点。
    ///
    /// 适合场景：信号干扰、故障艺术、恐怖氛围
    Noise = 2,

    /// 旋转缩放过渡 (Rotate & Zoom)
    ///
    /// 画面同时进行旋转和缩放变换，产生眩晕般的过渡效果。
    /// 前半段旋转放大，后半段旋转缩小恢复。
    ///
    /// 适合场景：时空穿越、回忆闪回、梦境切换
    RotateZoom = 3,

    /// 弹跳波浪过渡 (Bounce Wave)
    ///
    /// 源图像像弹跳的球一样从底部弹起消失，逐渐显示目标图像。
    /// 使用正弦函数实现弹跳曲线。
    ///
    /// 适合场景：轻快的场景切换、游戏关卡过渡
    Bounce = 4,

    /// 色散分离过渡 (Color Dispersion)
    ///
    /// RGB三通道产生不同程度的位移，产生色差/色散效果。
    /// 同时添加波浪扭曲增强视觉冲击。
    ///
    /// 适合场景：冲击波、能量释放、赛博朋克风格
    Dispersion = 5,

    /// 涟漪扩散过渡 (Ripple Wave)
    ///
    /// 从中心向外扩散的同心圆水波纹效果。
    /// 波纹内部显示混合后的画面，外部保持原画面。
    ///
    /// 适合场景：水面效果、能量场、传送门
    Ripple = 6,
}

impl GpuTransition {
    /// 获取所有GPU过渡类型的数组引用
    pub fn all() -> &'static [GpuTransition] {
        &[
            GpuTransition::Squares,
            GpuTransition::Heart,
            GpuTransition::Noise,
            GpuTransition::RotateZoom,
            GpuTransition::Bounce,
            GpuTransition::Dispersion,
            GpuTransition::Ripple,
        ]
    }

    /// 获取特效英文名称
    pub fn name(&self) -> &'static str {
        match self {
            GpuTransition::Squares => "Squares",
            GpuTransition::Heart => "Heart",
            GpuTransition::Noise => "Noise",
            GpuTransition::RotateZoom => "RotateZoom",
            GpuTransition::Bounce => "Bounce",
            GpuTransition::Dispersion => "Dispersion",
            GpuTransition::Ripple => "Ripple",
        }
    }

    /// 获取特效中文名称
    pub fn name_cn(&self) -> &'static str {
        match self {
            GpuTransition::Squares => "方格渐变",
            GpuTransition::Heart => "心形展开",
            GpuTransition::Noise => "噪点过渡",
            GpuTransition::RotateZoom => "旋转缩放",
            GpuTransition::Bounce => "弹跳波浪",
            GpuTransition::Dispersion => "色散分离",
            GpuTransition::Ripple => "涟漪扩散",
        }
    }

    /// 获取特效详细描述
    pub fn description(&self) -> &'static str {
        match self {
            GpuTransition::Squares => "Grid squares transition with dynamic sizing",
            GpuTransition::Heart => "Heart-shaped reveal from center",
            GpuTransition::Noise => "TV static noise transition",
            GpuTransition::RotateZoom => "Rotation with zoom in/out effect",
            GpuTransition::Bounce => "Bouncing wave wipe transition",
            GpuTransition::Dispersion => "RGB chromatic aberration effect",
            GpuTransition::Ripple => "Circular ripple wave from center",
        }
    }

    /// 获取GPU过渡特效总数
    pub const fn count() -> usize {
        7
    }

    /// 从索引创建 (循环取模，确保总是有效)
    pub fn from_index(index: usize) -> Self {
        match index % Self::count() {
            0 => GpuTransition::Squares,
            1 => GpuTransition::Heart,
            2 => GpuTransition::Noise,
            3 => GpuTransition::RotateZoom,
            4 => GpuTransition::Bounce,
            5 => GpuTransition::Dispersion,
            _ => GpuTransition::Ripple,
        }
    }

    /// 获取下一个过渡类型 (循环)
    pub fn next(&self) -> Self {
        Self::from_index(*self as usize + 1)
    }

    /// 获取上一个过渡类型 (循环)
    pub fn prev(&self) -> Self {
        let idx = *self as usize;
        Self::from_index(if idx == 0 { Self::count() - 1 } else { idx - 1 })
    }
}

impl From<GpuTransition> for usize {
    fn from(t: GpuTransition) -> Self {
        t as usize
    }
}

impl From<usize> for GpuTransition {
    fn from(v: usize) -> Self {
        GpuTransition::from_index(v)
    }
}

impl Default for GpuTransition {
    fn default() -> Self {
        GpuTransition::Squares
    }
}

/// GPU混合特效描述
///
/// 封装了GPU过渡特效的完整参数，便于传递和复用。
/// 提供便捷的工厂方法创建各种过渡效果。
///
/// # Example
/// ```ignore
/// // 方式1: 使用new
/// let effect = GpuBlendEffect::new(GpuTransition::Ripple, 0.5);
///
/// // 方式2: 使用工厂方法
/// let effect = GpuBlendEffect::ripple(0.5);
/// let effect = GpuBlendEffect::dispersion(0.3);
///
/// // 调用blend_rts
/// ctx.adapter.blend_rts(0, 1, 2, effect.effect_type(), effect.progress);
/// ```
#[derive(Clone, Debug)]
pub struct GpuBlendEffect {
    /// 过渡特效类型
    pub transition: GpuTransition,
    /// 过渡进度 (0.0 = 完全显示src1, 1.0 = 完全显示src2)
    pub progress: f32,
}

impl GpuBlendEffect {
    /// 创建新的GPU混合特效
    ///
    /// # Parameters
    /// - `transition`: 过渡类型
    /// - `progress`: 过渡进度 (0.0 ~ 1.0)
    pub fn new(transition: GpuTransition, progress: f32) -> Self {
        Self {
            transition,
            progress: progress.clamp(0.0, 1.0),
        }
    }

    /// 获取特效类型索引 (用于 Adapter::blend_rts)
    #[inline]
    pub fn effect_type(&self) -> usize {
        self.transition as usize
    }

    /// 设置进度值
    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// 更新进度值 (可变引用版本)
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    // === 工厂方法 (Factory Methods) ===

    /// 创建方格过渡特效
    pub fn squares(progress: f32) -> Self {
        Self::new(GpuTransition::Squares, progress)
    }

    /// 创建心形过渡特效
    pub fn heart(progress: f32) -> Self {
        Self::new(GpuTransition::Heart, progress)
    }

    /// 创建噪点过渡特效
    pub fn noise(progress: f32) -> Self {
        Self::new(GpuTransition::Noise, progress)
    }

    /// 创建旋转缩放过渡特效
    pub fn rotate_zoom(progress: f32) -> Self {
        Self::new(GpuTransition::RotateZoom, progress)
    }

    /// 创建弹跳波浪过渡特效
    pub fn bounce(progress: f32) -> Self {
        Self::new(GpuTransition::Bounce, progress)
    }

    /// 创建色散过渡特效
    pub fn dispersion(progress: f32) -> Self {
        Self::new(GpuTransition::Dispersion, progress)
    }

    /// 创建涟漪过渡特效
    pub fn ripple(progress: f32) -> Self {
        Self::new(GpuTransition::Ripple, progress)
    }

    /// 从索引创建特效 (循环)
    pub fn from_index(index: usize, progress: f32) -> Self {
        Self::new(GpuTransition::from_index(index), progress)
    }
}

impl Default for GpuBlendEffect {
    fn default() -> Self {
        Self::new(GpuTransition::Squares, 0.0)
    }
}

// ============================================================================
// CPU Buffer Effects (字符级别)
// ============================================================================

/// Effect parameters passed to each effect
#[derive(Clone, Debug)]
pub struct EffectParams {
    /// Progress/time value (0.0 - 1.0 for transitions, or absolute time)
    pub time: f32,
    /// Stage/frame counter for animation
    pub stage: usize,
    /// Random seed for reproducible effects
    pub seed: u32,
    /// Effect intensity multiplier (0.0 - 1.0)
    pub intensity: f32,
}

impl EffectParams {
    /// Create new effect params with time and stage
    pub fn new(time: f32, stage: usize) -> Self {
        Self {
            time,
            stage,
            seed: 0,
            intensity: 1.0,
        }
    }

    /// Set random seed
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// Set intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }
}

impl Default for EffectParams {
    fn default() -> Self {
        Self::new(0.0, 0)
    }
}

/// Trait for CPU-based buffer effects
///
/// Implement this trait to create custom visual effects that operate
/// on Buffer contents at the cell level.
pub trait BufferEffect: Send + Sync {
    /// Apply the effect from source buffer to destination buffer
    ///
    /// # Parameters
    /// - `src`: Source buffer (read-only)
    /// - `dst`: Destination buffer (will be modified)
    /// - `params`: Effect parameters (time, stage, intensity, etc.)
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams);

    /// Get effect name for debugging
    fn name(&self) -> &'static str;
}

// ============================================================================
// Distortion Effects
// ============================================================================

/// Apply a distortion function to remap buffer coordinates
///
/// This is a helper function used by distortion-based effects.
/// The distortion_fn takes normalized UV coordinates (0-1) and returns
/// new UV coordinates for sampling.
pub fn apply_distortion<F>(src: &Buffer, dst: &mut Buffer, distortion_fn: F)
where
    F: Fn(f32, f32) -> (f32, f32),
{
    let width = src.area.width as i32;
    let height = src.area.height as i32;

    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;

            let (du, dv) = distortion_fn(u, v);

            let src_x = (du * width as f32).round() as i32;
            let src_y = (dv * height as f32).round() as i32;

            let src_x = src_x.clamp(0, width - 1);
            let src_y = src_y.clamp(0, height - 1);

            let src_index = (src_y * width + src_x) as usize;
            let dest_index = (y * width + x) as usize;

            if let (Some(src_cell), Some(dest_cell)) = (
                src.content.get(src_index),
                dst.content.get_mut(dest_index),
            ) {
                *dest_cell = src_cell.clone();
            }
        }
    }
}

/// Wave distortion effect - creates horizontal wave patterns
///
/// ```text
/// Before:          After:
/// ┌──────────┐    ┌──────────┐
/// │ AAAAAAAA │    │  AAAAAAAA│
/// │ BBBBBBBB │    │BBBBBBBB  │
/// │ CCCCCCCC │    │  CCCCCCCC│
/// │ DDDDDDDD │    │DDDDDDDD  │
/// └──────────┘    └──────────┘
/// ```
#[derive(Clone, Debug)]
pub struct WaveEffect {
    /// Wave amplitude (0.0 - 0.5, typical: 0.03)
    pub amplitude: f32,
    /// Wave frequency (typical: 10.0 - 20.0)
    pub frequency: f32,
}

impl WaveEffect {
    pub fn new(amplitude: f32, frequency: f32) -> Self {
        Self { amplitude, frequency }
    }
}

impl BufferEffect for WaveEffect {
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        let time = params.time;
        let amp = self.amplitude * params.intensity;
        let freq = self.frequency;

        apply_distortion(src, dst, |u, v| {
            let offset_x = u + amp * (freq * v + time).sin();
            (offset_x, v)
        });
    }

    fn name(&self) -> &'static str {
        "WaveEffect"
    }
}

/// Ripple distortion effect - creates circular ripple from center
///
/// ```text
/// Before:          After:
/// ┌──────────┐    ┌──────────┐
/// │ XXXXXXXX │    │ XX    XX │
/// │ XXXXXXXX │    │X  XXXX  X│
/// │ XXXXXXXX │    │X  XXXX  X│
/// │ XXXXXXXX │    │ XX    XX │
/// └──────────┘    └──────────┘
/// ```
#[derive(Clone, Debug)]
pub struct RippleEffect {
    /// Ripple amplitude (0.0 - 0.2, typical: 0.05)
    pub amplitude: f32,
    /// Ripple frequency (typical: 5.0 - 15.0)
    pub frequency: f32,
    /// Center X (0.0 - 1.0, default: 0.5)
    pub center_x: f32,
    /// Center Y (0.0 - 1.0, default: 0.5)
    pub center_y: f32,
}

impl RippleEffect {
    pub fn new(amplitude: f32, frequency: f32) -> Self {
        Self {
            amplitude,
            frequency,
            center_x: 0.5,
            center_y: 0.5,
        }
    }

    pub fn with_center(mut self, cx: f32, cy: f32) -> Self {
        self.center_x = cx;
        self.center_y = cy;
        self
    }
}

impl BufferEffect for RippleEffect {
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        let time = params.time;
        let amp = self.amplitude * params.intensity;
        let freq = self.frequency;
        let cx = self.center_x;
        let cy = self.center_y;

        apply_distortion(src, dst, |u, v| {
            let dx = u - cx;
            let dy = v - cy;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < 0.001 {
                return (u, v);
            }

            let offset = amp * (freq * distance - time).sin();
            let du = u + (dx / distance) * offset;
            let dv = v + (dy / distance) * offset;
            (du, dv)
        });
    }

    fn name(&self) -> &'static str {
        "RippleEffect"
    }
}

/// Swirl distortion effect - rotates pixels around center
#[derive(Clone, Debug)]
pub struct SwirlEffect {
    /// Swirl strength (radians, typical: 0.5 - 2.0)
    pub strength: f32,
    /// Swirl radius (0.0 - 1.0, default: 0.5)
    pub radius: f32,
    /// Center X (0.0 - 1.0, default: 0.5)
    pub center_x: f32,
    /// Center Y (0.0 - 1.0, default: 0.5)
    pub center_y: f32,
}

impl SwirlEffect {
    pub fn new(strength: f32, radius: f32) -> Self {
        Self {
            strength,
            radius,
            center_x: 0.5,
            center_y: 0.5,
        }
    }
}

impl BufferEffect for SwirlEffect {
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        let strength = self.strength * params.intensity;
        let radius = self.radius;
        let cx = self.center_x;
        let cy = self.center_y;

        apply_distortion(src, dst, |u, v| {
            let dx = u - cx;
            let dy = v - cy;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance > radius || distance < 0.001 {
                return (u, v);
            }

            let factor = 1.0 - distance / radius;
            let angle = strength * factor * factor;

            let cos_a = angle.cos();
            let sin_a = angle.sin();

            let new_dx = dx * cos_a - dy * sin_a;
            let new_dy = dx * sin_a + dy * cos_a;

            (cx + new_dx, cy + new_dy)
        });
    }

    fn name(&self) -> &'static str {
        "SwirlEffect"
    }
}

// ============================================================================
// Noise and Overlay Effects
// ============================================================================

/// Random noise effect - adds random symbols to buffer
#[derive(Clone, Debug)]
pub struct NoiseEffect {
    /// Noise density (0.0 - 1.0, percentage of cells affected)
    pub density: f32,
    /// Noise color (None = use random grayscale)
    pub color: Option<Color>,
}

impl NoiseEffect {
    pub fn new(density: f32) -> Self {
        Self {
            density: density.clamp(0.0, 1.0),
            color: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl BufferEffect for NoiseEffect {
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        // First copy source to destination
        dst.content.clone_from(&src.content);

        let density = self.density * params.intensity;
        let cell_count = dst.content.len();
        let noise_count = (cell_count as f32 * density) as usize;

        // Simple LCG random generator using seed
        let mut rng = params.seed.wrapping_add(params.stage as u32);
        let lcg_next = |r: &mut u32| -> u32 {
            *r = r.wrapping_mul(1103515245).wrapping_add(12345);
            *r
        };

        for _ in 0..noise_count {
            let idx = lcg_next(&mut rng) as usize % cell_count;
            let sym = (lcg_next(&mut rng) % 255) as u8;

            let color = self.color.unwrap_or_else(|| {
                let gray = 100 + (lcg_next(&mut rng) % 100) as u8;
                Color::Rgba(gray, gray, gray, 200)
            });

            if let Some(cell) = dst.content.get_mut(idx) {
                cell.set_symbol(&cellsym(sym)).set_fg(color);
            }
        }
    }

    fn name(&self) -> &'static str {
        "NoiseEffect"
    }
}

/// Fade effect - adjusts alpha of all cells
#[derive(Clone, Debug)]
pub struct FadeEffect {
    /// Target alpha (0 = transparent, 255 = opaque)
    pub target_alpha: u8,
}

impl FadeEffect {
    pub fn new(alpha: u8) -> Self {
        Self { target_alpha: alpha }
    }

    /// Create fade-in effect (transparent to opaque based on params.time)
    pub fn fade_in() -> Self {
        Self { target_alpha: 255 }
    }

    /// Create fade-out effect (opaque to transparent based on params.time)
    pub fn fade_out() -> Self {
        Self { target_alpha: 0 }
    }
}

impl BufferEffect for FadeEffect {
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        dst.content.clone_from(&src.content);

        let alpha = ((self.target_alpha as f32) * params.intensity) as u8;

        for cell in dst.content.iter_mut() {
            // Modify foreground color alpha
            if let Color::Rgba(r, g, b, _) = cell.fg {
                cell.fg = Color::Rgba(r, g, b, alpha);
            }
        }
    }

    fn name(&self) -> &'static str {
        "FadeEffect"
    }
}

// ============================================================================
// Pixelation and Blur Effects
// ============================================================================

/// Pixelate effect - creates mosaic/blocky appearance
#[derive(Clone, Debug)]
pub struct PixelateEffect {
    /// Block size (1 = no effect, larger = more pixelated)
    pub block_size: u16,
}

impl PixelateEffect {
    pub fn new(block_size: u16) -> Self {
        Self {
            block_size: block_size.max(1),
        }
    }
}

impl BufferEffect for PixelateEffect {
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        let width = src.area.width as usize;
        let height = src.area.height as usize;
        let block = (self.block_size as f32 * params.intensity).max(1.0) as usize;

        dst.content.clone_from(&src.content);

        for by in (0..height).step_by(block) {
            for bx in (0..width).step_by(block) {
                // Sample from center of block
                let sample_x = (bx + block / 2).min(width - 1);
                let sample_y = (by + block / 2).min(height - 1);
                let sample_idx = sample_y * width + sample_x;

                if let Some(sample_cell) = src.content.get(sample_idx) {
                    // Fill block with sampled cell
                    for dy in 0..block {
                        for dx in 0..block {
                            let x = bx + dx;
                            let y = by + dy;
                            if x < width && y < height {
                                let idx = y * width + x;
                                if let Some(cell) = dst.content.get_mut(idx) {
                                    *cell = sample_cell.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "PixelateEffect"
    }
}

/// Simple box blur effect
#[derive(Clone, Debug)]
pub struct BlurEffect {
    /// Blur radius (1 = 3x3, 2 = 5x5, etc.)
    pub radius: u16,
}

impl BlurEffect {
    pub fn new(radius: u16) -> Self {
        Self {
            radius: radius.max(1),
        }
    }
}

impl BufferEffect for BlurEffect {
    fn apply(&self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        let width = src.area.width as i32;
        let height = src.area.height as i32;
        let radius = (self.radius as f32 * params.intensity).max(1.0) as i32;

        dst.content.clone_from(&src.content);

        for y in 0..height {
            for x in 0..width {
                // For character-based blur, we just sample from a random neighbor
                // within the radius (true blur would need color averaging)
                let offset_x = ((x + radius / 2) % (radius * 2 + 1)) - radius;
                let offset_y = ((y + radius / 2) % (radius * 2 + 1)) - radius;

                let sample_x = (x + offset_x).clamp(0, width - 1);
                let sample_y = (y + offset_y).clamp(0, height - 1);

                let src_idx = (sample_y * width + sample_x) as usize;
                let dst_idx = (y * width + x) as usize;

                if let (Some(src_cell), Some(dst_cell)) = (
                    src.content.get(src_idx),
                    dst.content.get_mut(dst_idx),
                ) {
                    *dst_cell = src_cell.clone();
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "BlurEffect"
    }
}

// ============================================================================
// Effect Chain
// ============================================================================

/// Chain multiple effects together
///
/// Effects are applied in order, with each effect's output
/// becoming the next effect's input.
pub struct EffectChain {
    effects: Vec<Box<dyn BufferEffect>>,
    /// Intermediate buffer for chaining (reused to avoid allocation)
    temp_buffer: Option<Buffer>,
}

impl EffectChain {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            temp_buffer: None,
        }
    }

    /// Add an effect to the chain
    pub fn add(&mut self, effect: Box<dyn BufferEffect>) -> &mut Self {
        self.effects.push(effect);
        self
    }

    /// Clear all effects
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Get number of effects in chain
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Apply all effects in sequence
    pub fn apply(&mut self, src: &Buffer, dst: &mut Buffer, params: &EffectParams) {
        if self.effects.is_empty() {
            dst.content.clone_from(&src.content);
            return;
        }

        if self.effects.len() == 1 {
            self.effects[0].apply(src, dst, params);
            return;
        }

        // Initialize temp buffer if needed
        if self.temp_buffer.is_none() || self.temp_buffer.as_ref().unwrap().area != src.area {
            self.temp_buffer = Some(src.clone());
        }

        let temp = self.temp_buffer.as_mut().unwrap();

        // Apply first effect: src -> temp
        self.effects[0].apply(src, temp, params);

        // Apply middle effects: temp -> dst -> temp (ping-pong)
        for i in 1..self.effects.len() - 1 {
            if i % 2 == 1 {
                self.effects[i].apply(temp, dst, params);
            } else {
                self.effects[i].apply(dst, temp, params);
            }
        }

        // Apply last effect to dst
        let last_idx = self.effects.len() - 1;
        if last_idx % 2 == 1 {
            // Last intermediate result is in temp
            self.effects[last_idx].apply(temp, dst, params);
        } else {
            // Last intermediate result is in dst, need to use temp
            temp.content.clone_from(&dst.content);
            self.effects[last_idx].apply(temp, dst, params);
        }
    }
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Preset Effect Chains
// ============================================================================

/// Create a dissolve transition effect chain
pub fn dissolve_chain(noise_density: f32) -> EffectChain {
    let mut chain = EffectChain::new();
    chain.add(Box::new(NoiseEffect::new(noise_density)));
    chain
}

/// Create a wave + ripple distortion effect chain
pub fn distortion_chain(wave_amp: f32, ripple_amp: f32) -> EffectChain {
    let mut chain = EffectChain::new();
    chain.add(Box::new(WaveEffect::new(wave_amp, 15.0)));
    chain.add(Box::new(RippleEffect::new(ripple_amp, 10.0)));
    chain
}

/// Create a glitch effect chain (pixelate + noise + wave)
pub fn glitch_chain(intensity: f32) -> EffectChain {
    let mut chain = EffectChain::new();
    chain.add(Box::new(PixelateEffect::new((intensity * 4.0) as u16 + 1)));
    chain.add(Box::new(WaveEffect::new(intensity * 0.1, 20.0)));
    chain.add(Box::new(NoiseEffect::new(intensity * 0.2)));
    chain
}
