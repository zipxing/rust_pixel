# SDF Looked Worse Than Bitmaps — How We Fixed Text Rendering in a Pixel Engine
# SDF 竟然不如位图好看——我们怎么修好了像素引擎的文字渲染

---

*[RustPixel](https://github.com/zipxing/rust_pixel) is a Rust 2D game engine that renders pixel sprites, TUI text, CJK characters, and emoji in a single instanced draw call. When we needed crisp text at fullscreen 4K, we reached for SDF — the "correct" solution. It made things worse.*

*[RustPixel](https://github.com/zipxing/rust_pixel) 是一个 Rust 2D 游戏引擎，用单次实例化 draw call 渲染像素精灵、TUI 文字、中文和 Emoji。当我们需要全屏 4K 下的清晰文字时，选了 SDF——"正确"的方案。结果更糟了。*

---

## The Setup / 背景

RustPixel's rendering is simple by design: one texture atlas, one instanced draw call, all symbols. Pixel sprites (C64-style 16×16), box-drawing characters, Nerd Font icons, PingFang SC Chinese text, Apple Color Emoji — all packed into one atlas, all rendered together.

RustPixel 的渲染设计上就简单：一张纹理图集，一次实例化 draw call，所有符号。像素精灵（C64 风格 16×16）、制表符、Nerd Font 图标、苹方中文、Apple Color Emoji——全部打包进一张图集，一起渲染。

Then we built MDPT — a fullscreen Markdown presentation tool. A TUI cell at 32×64 pixels gets blown up 4–8× on a 4K display. Bitmap text goes blurry. We needed a solution.

然后我们做了 MDPT——一个全屏 Markdown 演示工具。32×64 像素的 TUI 字符格子在 4K 显示器上放大 4-8 倍，位图文字就糊了。我们需要一个解决方案。

---

## SDF: The "Right" Answer That Wasn't / SDF："正确"的方案，但并不正确

Every text rendering tutorial will tell you: use Signed Distance Fields. Resolution-independent. Crisp at any scale. So we did:

每篇文字渲染教程都会告诉你：用签名距离场。分辨率无关，任何缩放都清晰。于是我们照做了：

- **TUI characters**: True MSDF via `msdfgen` — 3-channel distance fields with sharp corners
- **CJK (PingFang SC)**: Bitmap→SDF via `scipy.ndimage.distance_transform_edt` (Apple locks the glyph outlines in `.ttc`)
- **Sprites & Emoji**: Regular bitmaps

- **TUI 字符**：用 `msdfgen` 生成真 MSDF——三通道距离场，保留尖角
- **CJK（苹方）**：位图→SDF 转换，用 `scipy.ndimage.distance_transform_edt`（苹果把字形轮廓锁在 `.ttc` 里）
- **精灵 & Emoji**：普通位图

Everything in one 8192×8192 atlas, fragment shader branching per-fragment:

所有东西放进一张 8192×8192 的图集，片段着色器逐片段分支：

```wgsl
if (v_msdf > 0.5) {
    let d = median3(texColor.r, texColor.g, texColor.b);
    let w = max(fwidth(d), 0.03);
    let alpha = smoothstep(0.5 - w, 0.5 + w, d);
    output.color = vec4(fg_color.rgb, alpha);
} else {
    output.color = texColor * instance_color;
}
```

### The Problem: SDF Looked Bad / 问题：SDF 看起来更差

Here's the part nobody tells you in the tutorials: **the SDF anti-aliasing path actually produced worse results than direct bitmap rendering.**

教程里没人告诉你的部分来了：**SDF 的抗锯齿路径实际产生的效果比直接位图渲染更差。**

The `median3() + smoothstep()` pipeline introduced two artifacts at scale boundaries:

`median3() + smoothstep()` 管线在缩放边界处引入了两种瑕疵：

1. **Blur** — The anti-aliasing smoothing, when applied across scaled-up distance field texels, created a soft haze around character edges instead of clean lines. At 4–8× magnification, this was obvious.
2. **Fringing** — At certain scale transitions, the multi-channel reconstruction produced color fringing artifacts at corners and thin strokes.

1. **模糊** — 抗锯齿平滑作用在放大后的距离场纹素上，在字符边缘产生一层柔和的雾气，而不是干净的线条。在 4-8 倍放大下非常明显。
2. **毛刺** — 在某些缩放过渡处，多通道重建在拐角和细笔画处产生颜色毛刺。

The irony: we adopted SDF specifically for better quality at large scales. **It delivered worse quality than the bitmaps we were trying to improve.**

讽刺的是：我们采用 SDF 就是为了大缩放下更好的质量。**结果它比我们想改善的位图质量更差。**

### The CJK Double Whammy / CJK 的双重打击

It got worse for Chinese text. Real MSDF requires vector glyph outlines, but PingFang SC hides its outlines inside Apple's `.ttc` format. Our fallback — `bitmap → EDT → SDF` — lost edge sharpness in the distance transform step. So CJK text had SDF artifacts *plus* reduced source quality. One atlas, three quality tiers (MSDF → okay, bitmap-SDF → poor, bitmap → at least honest). Not a good look.

中文更糟。真正的 MSDF 需要矢量字形轮廓，但苹方把轮廓藏在苹果的 `.ttc` 格式里。我们的退路——`位图 → EDT → SDF`——在距离变换步骤中丢失了边缘锐度。所以中文字有 SDF 的瑕疵 *加上* 源质量降低。一张图集，三个质量档次（MSDF → 还行、位图-SDF → 差、位图 → 至少诚实）。

### And It Wasted 256MB / 而且浪费了 256MB

The 8192×8192 atlas (256MB uncompressed RGBA) loaded fully into GPU memory even if an app only used a handful of sprite blocks + TUI characters. Most RustPixel apps use maybe 5% of the symbols in the atlas.

8192×8192 的图集（256MB 未压缩 RGBA）全量加载进 GPU 内存，即使一个 app 只用了几组精灵 + TUI 字符。大部分 RustPixel app 大概只用到图集里 5% 的符号。

---

## The Fix: Just Use Better Bitmaps / 修复：直接用更好的位图

We overthought this. The actual solution is embarrassingly simple: **if bitmaps look better than SDF, just use higher-resolution bitmaps.**

我们想多了。实际方案简单到令人尴尬：**如果位图比 SDF 好看，那就用更高分辨率的位图。**

### 3-Level Mipmap + Texture2DArray / 三级 Mipmap + Texture2DArray

Pre-render every symbol at 3 resolutions. Let the engine pick the right one:

每个符号预渲染 3 个分辨率。让引擎选对的那个：

| Symbol Type / 符号类型 | High (mip0) | Mid (mip1) | Low (mip2) |
|------------------------|-------------|-------------|------------|
| Sprite | 64×64 | 32×32 | 16×16 |
| TUI / Braille | 64×128 | 32×64 | 16×32 |
| Emoji | 128×128 | 64×64 | 32×32 |
| CJK | 128×128 | 64×64 | 32×32 |

Selection is dead simple — based on actual pixel size per cell:

选择逻辑极其简单——基于每个格子的实际像素尺寸：

| Render Size / 渲染尺寸 | Mip Level | Scene / 场景 |
|------------------------|-----------|-------------|
| ≥ 48 px/unit | mip0 | Fullscreen 4K / 全屏 4K |
| ≥ 24 px/unit | mip1 | Normal window / 普通窗口 |
| < 24 px/unit | mip2 | Small window / 小窗口 |

Pack them into a `Texture2DArray` of multiple 4096×4096 layers instead of one massive 8192×8192 atlas:

打包进多张 4096×4096 层的 `Texture2DArray`，取代一张巨大的 8192×8192 图集：

```rust
pub struct Tile {
    pub cell_w: u8,       // 1=normal, 2=wide (CJK/Emoji)
    pub cell_h: u8,       // 1=single, 2=tall
    pub is_emoji: bool,   // Color emoji — no tint
    pub mips: [MipUV; 3], // UV coords for each mip level
}
```

### The Shader Got Stupid Simple / 着色器变得傻瓜式简单

Before (SDF era):
之前（SDF 时代）：

```wgsl
// 12 lines of branching, median3, smoothstep, fwidth...
if (v_msdf > 0.5) {
    let d = median3(texColor.r, texColor.g, texColor.b);
    let w = max(fwidth(d), 0.03);
    let alpha = smoothstep(0.5 - w, 0.5 + w, d);
    // ... bold path A ...
} else {
    // ... bold path B ...
}
```

After (mipmap era):
之后（mipmap 时代）：

```wgsl
let texColor = textureSample(t_texture, s_sampler, uv, layer);
output.color = texColor * instance_color;
```

Same path for sprites, TUI, CJK, emoji. No branching. One draw call.

精灵、TUI、CJK、Emoji 走同一条路径。无分支。一次 draw call。

### Memory: 48–80MB vs 256MB / 内存：48-80MB vs 256MB

A typical app loads 3–5 layers = 48–80MB, vs the old 256MB monolithic atlas. Apps that only use sprites + TUI can skip the CJK/emoji layers entirely.

典型 app 加载 3-5 层 = 48-80MB，对比旧的 256MB 单体图集。只用精灵 + TUI 的 app 可以完全跳过 CJK/Emoji 层。

### Pure Rust Toolchain / 纯 Rust 工具链

The old pipeline: Python 3 + scipy + Pillow + `msdfgen` C++ binary. 800+ lines of Python orchestrating external tools.

旧管线：Python 3 + scipy + Pillow + `msdfgen` C++ 二进制。800 多行 Python 调度外部工具。

The new pipeline:

新管线：

```bash
cargo pixel symbols -o assets/pix
```

Pure Rust. macOS CoreText for font rendering. DP shelf-packing for optimal layer utilization. Outputs `layers/layer_N.png` + `layered_symbol_map.json` (~375KB).

纯 Rust。macOS CoreText 渲染字体。DP 货架打包最优化层利用率。输出 `layers/layer_N.png` + `layered_symbol_map.json`（~375KB）。

---

## Side by Side / 对比

| | SDF/MSDF | Mipmap Bitmap |
|---|---|---|
| **Text quality at 4-8×** | Blur + fringing | Crisp (native rendering) |
| **CJK quality** | Poor (bitmap→EDT→SDF) | Good (CoreText direct) |
| **Max texture** | 8192×8192 (256MB) | 4096×4096 × N layers |
| **Typical memory** | 256MB (all or nothing) | 48–80MB (load what you need) |
| **Fragment shader** | Dual path + median3 | Single path |
| **Bold** | 2 implementations | 1 implementation |
| **Toolchain** | Python + scipy + msdfgen | `cargo pixel symbols` |
| **Draw calls** | 1 | 1 |
| **Scaling** | Infinite (math) | 3 discrete levels (sufficient) |
| **Rendering bugs since switch** | — | 0 |

---

## The Pixel Art Factor / 像素美术的因素

In a pixel art engine, SDF's core value proposition — smooth edges at any scale — actually **conflicts with the aesthetic**. Pixel sprites are *supposed* to look chunky when scaled up. You end up needing a dual-path shader specifically to *prevent* SDF from smoothing your intentionally pixelated content.

在像素引擎里，SDF 的核心价值——任何缩放下的平滑边缘——实际上**与美学冲突**。像素精灵在放大时*就应该*看起来粗粝。你需要双路径着色器专门来*阻止* SDF 平滑你故意做成像素风的内容。

With mipmaps, the intent is encoded in the content itself: pixel sprites use the same low-res bitmap at all mip levels (staying chunky), while text symbols have progressively higher-resolution bitmaps (staying crisp). Same texture array, same shader, same draw call.

用 mipmap，意图编码在内容本身：像素精灵在所有 mip 级别用相同的低分辨率位图（保持粗粝），文字符号有逐级更高分辨率的位图（保持清晰）。同一个纹理数组、同一个着色器、同一次 draw call。

---

## What We Actually Lost / 我们实际失去了什么

**Infinite scalability.** SDF gives mathematically perfect edges at any scale. Our 3 mip levels cover 720p through 4K. If someone runs MDPT on an 8K display, mip0 still looks good, but it's not mathematically perfect. We're okay with that.

**无限可伸缩性。** SDF 在任何缩放下给出数学上完美的边缘。我们的 3 级 mip 覆盖 720p 到 4K。如果有人在 8K 显示器上跑 MDPT，mip0 仍然好看，但不是数学上完美。我们接受这个。

**The cool factor.** "We use multi-channel signed distance fields" sounds impressive at conferences. "We pre-render bitmaps at 3 sizes" does not.

**炫酷因素。** "我们用多通道签名距离场"在会议上听着很帅。"我们把位图预渲染成 3 个尺寸"就不行。

---

## Takeaways / 总结

1. **Test before you trust.** SDF is mathematically elegant. But `median3() + smoothstep()` on scaled-up distance field texels can produce worse results than a properly-sized bitmap. We should have compared actual render quality before committing to the SDF pipeline.

2. **"Best practice" isn't always best.** Every tutorial recommends SDF for resolution-independent text. For our case — a pixel engine with known display ranges and mixed content types — discrete mipmaps produced better visual quality with simpler code.

3. **Memory matters.** 256MB GPU allocation for a texture atlas is a lot, especially when most apps use 5% of it. Texture2DArray lets you load only what you need.

4. **Toolchain is architecture.** Replacing Python + scipy + C++ with `cargo pixel symbols` wasn't just convenience — it removed a class of build complexity and contributor friction.

---

1. **先测试再信任。** SDF 数学上很优雅。但 `median3() + smoothstep()` 作用在放大的距离场纹素上，可能比正确尺寸的位图效果更差。我们应该在投入 SDF 管线之前对比实际渲染质量。

2. **"最佳实践"不总是最佳的。** 每篇教程都推荐 SDF 做分辨率无关文字。对于我们的场景——一个有已知显示范围和混合内容类型的像素引擎——离散 mipmap 用更简单的代码产生了更好的视觉质量。

3. **内存很重要。** 256MB GPU 分配给一张纹理图集很多，尤其当大部分 app 只用其中 5%。Texture2DArray 让你只加载需要的。

4. **工具链就是架构。** 用 `cargo pixel symbols` 替换 Python + scipy + C++ 不只是方便——它消除了一整类构建复杂度和贡献者摩擦。

---

*RustPixel is open source: [github.com/zipxing/rust_pixel](https://github.com/zipxing/rust_pixel)*

*RustPixel 是开源项目：[github.com/zipxing/rust_pixel](https://github.com/zipxing/rust_pixel)*
