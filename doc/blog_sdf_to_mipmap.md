# Why System Font Renderers Beat SDF — Lessons From a Pixel Engine
# 为什么系统字体渲染器比 SDF 更强——来自像素引擎的实战经验

---

*[RustPixel](https://github.com/zipxing/rust_pixel) is a Rust 2D game engine that renders pixel sprites, TUI text, CJK characters, and emoji in a single instanced draw call. When we needed crisp text at fullscreen 4K, we reached for SDF — the "correct" solution. Then we realized: for bounded display sizes, industrial-grade system font renderers like CoreText already produce pixel-perfect output. Why approximate with math what the OS can render perfectly?*

*[RustPixel](https://github.com/zipxing/rust_pixel) 是一个 Rust 2D 游戏引擎，用单次实例化 draw call 渲染像素精灵、TUI 文字、中文和 Emoji。当我们需要全屏 4K 下的清晰文字时，选了 SDF——"正确"的方案。后来我们意识到：对于有限的显示尺寸范围，CoreText 这种工业级系统字体渲染器已经能输出像素级完美的结果。既然操作系统能渲染得完美，为什么要用数学去近似？*

---

## The Setup / 背景

RustPixel renders everything — pixel sprites, TUI characters, CJK text, emoji — from a single texture in one instanced draw call. When we built MDPT (a fullscreen Markdown presentation tool), a TUI cell at 32×64 pixels gets blown up 4–8× on a 4K display. Bitmap text goes blurry. We needed a solution.

RustPixel 用单张纹理、一次实例化 draw call 渲染所有内容——像素精灵、TUI 字符、中文、Emoji。当我们做 MDPT（全屏 Markdown 演示工具）时，32×64 像素的 TUI 字符格子在 4K 显示器上放大 4-8 倍，位图文字糊了。我们需要一个方案。

---

## SDF: Great in Theory / SDF：理论上很完美

SDF (Signed Distance Fields) is the textbook answer for resolution-independent rendering. The math is elegant: encode distance-to-edge, reconstruct with `smoothstep`, get crisp edges at any scale. We implemented it:

SDF（签名距离场）是分辨率无关渲染的教科书方案。数学很优雅：编码到边缘的距离，用 `smoothstep` 重建，在任何缩放下获得清晰边缘。我们实现了它：

- **TUI characters**: True MSDF via `msdfgen` — sharp corners preserved
- **CJK (PingFang SC)**: Bitmap→SDF via `scipy.ndimage.distance_transform_edt` (Apple locks glyph outlines in `.ttc`)
- **Sprites & Emoji**: Regular bitmaps

```wgsl
// SDF fragment shader path
let d = median3(texColor.r, texColor.g, texColor.b);
let w = max(fwidth(d), 0.03);
let alpha = smoothstep(0.5 - w, 0.5 + w, d);
```

**SDF's strength is infinite scalability** — one texture, mathematically perfect edges at *any* magnification. For a map engine or vector graphics tool that needs to zoom from 0.1× to 1000×, SDF is brilliant.

**SDF 的优势是无限可伸缩性**——一张纹理，在*任何*放大倍数下都有数学上完美的边缘。对于需要从 0.1 倍缩放到 1000 倍的地图引擎或矢量图形工具，SDF 很出色。

**But a TUI application doesn't need infinite scalability.** Text sizes are bounded: from small windows (~720p) to fullscreen 4K. That's maybe a 3–4× range. And for that bounded range, SDF's mathematical approximation actually performed **worse** than what we had.

**但 TUI 应用不需要无限可伸缩性。** 文字尺寸是有限的：从小窗口（~720p）到全屏 4K，大概 3-4 倍的范围。而对于这个有限的范围，SDF 的数学近似实际表现**比我们原来的更差**。

---

## What Went Wrong / 哪里出了问题

### 1. Math < Industry / 数学 < 工业

Here's the uncomfortable realization: **`median3() + smoothstep()` is a mathematical approximation of text edge reconstruction.** It's trying to reconstruct what a font renderer would produce — but from a lossy intermediate representation (the distance field).

一个不太舒服的领悟：**`median3() + smoothstep()` 是文字边缘重建的数学近似。** 它试图重建字体渲染器本该产生的结果——但是从一个有损的中间表示（距离场）来重建。

Meanwhile, macOS CoreText / Quartz is a **battle-tested, decades-old font rendering engine** with sub-pixel anti-aliasing, hinting, and kerning optimized for every Apple display. When you ask CoreText to render "Hello" at 64×128 pixels, the output is pixel-perfect — because it was designed to be.

而 macOS CoreText / Quartz 是一个**久经考验、有几十年历史的字体渲染引擎**，拥有针对每种 Apple 显示器优化的亚像素抗锯齿、字体微调和字距调整。当你让 CoreText 在 64×128 像素下渲染 "Hello"，输出就是像素级完美的——因为它就是为此设计的。

**Why approximate with a shader what the OS renders perfectly?**

**既然操作系统能完美渲染，为什么要用着色器去近似？**

### 2. SDF Artifacts at TUI Scales / SDF 在 TUI 尺寸下的瑕疵

At 4–8× magnification, the `median3() + smoothstep()` pipeline introduced:

在 4-8 倍放大下，`median3() + smoothstep()` 管线引入了：

- **Blur** — anti-aliasing smoothing across scaled-up distance field texels created a soft haze around edges
- **Fringing** — multi-channel reconstruction produced color artifacts at corners and thin strokes

- **模糊** — 抗锯齿平滑作用在放大的距离场纹素上，在边缘产生柔和的雾气
- **毛刺** — 多通道重建在拐角和细笔画处产生颜色伪影

These aren't SDF bugs — they're inherent limitations of reconstructing edges from a finite-resolution distance field. At infinite resolution they vanish; at TUI scales they're visible.

这不是 SDF 的 bug——而是从有限分辨率的距离场重建边缘的固有局限。在无限分辨率下它们会消失；在 TUI 尺寸下它们可见。

### 3. The CJK Problem / CJK 的问题

Real MSDF requires vector glyph outlines. PingFang SC — the best CJK font on macOS — hides its outlines inside Apple's `.ttc` format. Our fallback was `bitmap → EDT → SDF`: we used CoreText to render the bitmap, then *threw away* that perfect rendering to convert it into a distance field, then reconstructed it with `smoothstep`. We literally degraded CoreText's output to make it "resolution-independent."

真正的 MSDF 需要矢量字形轮廓。苹方——macOS 上最好的中文字体——把轮廓藏在 `.ttc` 格式里。我们的退路是 `位图 → EDT → SDF`：用 CoreText 渲染位图，然后*丢掉*那个完美的渲染结果去转换成距离场，再用 `smoothstep` 重建。我们字面上降级了 CoreText 的输出来让它"分辨率无关"。

**That moment — realizing we were degrading the OS's font output to feed it into a shader that produced worse results — was when we knew SDF was wrong for us.**

**那个瞬间——意识到我们在降级操作系统的字体输出，然后喂给一个产生更差结果的着色器——就是我们知道 SDF 对我们来说是错的时候。**

---

## The Mipmap Approach: Let CoreText Do What It Does Best / Mipmap 方案：让 CoreText 做它最擅长的事

The insight was simple: **if the display size range is known (720p to 4K), pre-render every symbol at 3 resolutions using the system font renderer, and let the engine pick the right one at render time — no scaling, no approximation.**

洞察很简单：**如果显示尺寸范围是已知的（720p 到 4K），用系统字体渲染器把每个符号预渲染成 3 个分辨率，引擎在渲染时选对的那个——不缩放、不近似。**

### Pre-baked by CoreText / CoreText 预烘焙

```bash
cargo pixel symbols -o assets/pix    # Pure Rust, uses CoreText directly
```

Each symbol is rendered at 3 sizes by macOS CoreText/Quartz — the same engine that renders every character on your Mac's screen:

每个符号由 macOS CoreText/Quartz 渲染成 3 个尺寸——跟你 Mac 屏幕上渲染每个字符的是同一个引擎：

| Symbol Type / 符号类型 | High (mip0) | Mid (mip1) | Low (mip2) |
|------------------------|-------------|-------------|------------|
| Sprite | 64×64 | 32×32 | 16×16 |
| TUI / Braille | 64×128 | 32×64 | 16×32 |
| Emoji | 128×128 | 64×64 | 32×32 |
| CJK | 128×128 | 64×64 | 32×32 |

These aren't "bitmaps" in the cheap sense. Each one is a **pixel-perfect rendering by an industrial-grade font engine** at the exact target resolution. Sub-pixel anti-aliasing, hinting, the works.

这些不是简陋意义上的"位图"。每一张都是**工业级字体引擎在精确目标分辨率下的像素级完美渲染**。亚像素抗锯齿、字体微调，应有尽有。

### The Engine Does Zero Scaling / 引擎做零缩放

This is the key: **the engine never scales text.** It determines the current display size per cell, picks the matching mip level, and renders the pre-baked texture directly — 1:1 or close to it.

这是关键：**引擎从不缩放文字。** 它判断当前每个格子的显示尺寸，选择匹配的 mip 级别，直接渲染预烘焙的纹理——1:1 或接近 1:1。

| Render Size / 渲染尺寸 | Mip Level | What Happens / 发生了什么 |
|------------------------|-----------|--------------------------|
| ≥ 48 px/unit | mip0 | Use high-res pre-bake / 用高分辨率预烘焙 |
| ≥ 24 px/unit | mip1 | Use mid-res pre-bake / 用中分辨率预烘焙 |
| < 24 px/unit | mip2 | Use low-res pre-bake / 用低分辨率预烘焙 |

The fragment shader is trivial:

片段着色器简单到极致：

```wgsl
let texColor = textureSample(t_texture, s_sampler, uv, layer);
output.color = texColor * instance_color;
```

**Same code path for sprites, TUI, CJK, emoji. No branching. One draw call.**

**精灵、TUI、CJK、Emoji 走同一条代码路径。无分支。一次 draw call。**

### Texture2DArray — Load Only What You Need / 只加载需要的

Instead of one 8192×8192 atlas (256MB), symbols are packed into multiple 4096×4096 layers using DP shelf-packing:

不再用一张 8192×8192 图集（256MB），符号用 DP 货架打包算法装进多张 4096×4096 层：

```rust
pub struct Tile {
    pub cell_w: u8,       // 1=normal, 2=wide (CJK/Emoji)
    pub cell_h: u8,       // 1=single, 2=tall
    pub is_emoji: bool,   // Color emoji — no tint
    pub mips: [MipUV; 3], // Pre-baked UV coords per mip level
}
```

A typical app loads 3–5 layers (48–80MB). Apps using only sprites + TUI skip the CJK/emoji layers entirely.

典型 app 加载 3-5 层（48-80MB）。只用精灵 + TUI 的 app 完全跳过 CJK/Emoji 层。

---

## Side by Side / 对比

| | SDF/MSDF | Mipmap (CoreText pre-bake) |
|---|---|---|
| **Text quality** | Math approximation (blur + fringing) | Pixel-perfect (system renderer) |
| **CJK quality** | Poor (bitmap→EDT→SDF degraded CoreText output) | Excellent (CoreText direct) |
| **Scaling approach** | Engine scales via shader math | Engine picks pre-baked size, no scaling |
| **Max texture** | 8192×8192 (256MB, all-or-nothing) | 4096×4096 × N layers (load what you need) |
| **Typical memory** | 256MB | 48–80MB |
| **Fragment shader** | Dual path + median3 + smoothstep | Single path, texture sample only |
| **Bold** | 2 implementations (SDF threshold / bitmap neighbor) | 1 implementation |
| **Toolchain** | Python + scipy + msdfgen | `cargo pixel symbols` (Pure Rust) |
| **Draw calls** | 1 | 1 |
| **Infinite zoom** | ✅ (SDF's real strength) | ❌ 3 discrete levels (sufficient for TUI) |

---

## The Pixel Art Factor / 像素美术的因素

In a pixel art engine, SDF's "smooth at any scale" philosophy actively **fights the aesthetic**. Pixel sprites *should* look chunky when scaled up — that's the whole point. With SDF you need a dual-path shader to prevent it from smoothing your intentionally pixelated content.

在像素引擎里，SDF 的"任何缩放都平滑"理念**与美学主动对抗**。像素精灵放大时*就应该*看起来粗粝——这是全部要义。用 SDF 你需要双路径着色器来阻止它平滑你故意做成像素风的内容。

With mipmaps, intent is in the content: pixel sprites have the same low-res bitmap at all mip levels (chunky by design), text symbols have high-res CoreText renders (crisp by design). Same texture array, same shader, same draw call.

用 mipmap，意图在内容里：像素精灵在所有 mip 级别用相同的低分辨率位图（设计上就粗粝），文字符号有高分辨率的 CoreText 渲染（设计上就清晰）。同一个纹理数组、同一个着色器、同一次 draw call。

---

## When SDF IS the Right Choice / 什么时候 SDF 才是正确选择

To be fair, SDF is genuinely superior when:

公平地说，SDF 在以下场景确实更优：

- **Infinite zoom range** — map engines, vector editors, CAD tools
- **Dynamic text generation** — chat bubbles, procedural UI where you can't pre-bake
- **Memory-constrained + huge scale range** — one small SDF texture covering 0.1× to 100×
- **Glow / outline / shadow effects** — distance field makes these trivial in the shader

- **无限缩放范围** — 地图引擎、矢量编辑器、CAD 工具
- **动态文字生成** — 聊天气泡、程序化 UI，无法预烘焙的场景
- **内存受限 + 巨大缩放范围** — 一张小 SDF 纹理覆盖 0.1× 到 100×
- **发光/描边/阴影效果** — 距离场让这些在着色器里变得很简单

Our case was none of these. We had bounded display sizes, known symbol sets, and a platform with an excellent system font renderer. Pre-baking was the obvious answer — we just took a detour through SDF to realize it.

我们的场景不属于以上任何一种。我们有有限的显示尺寸、已知的符号集，和一个拥有优秀系统字体渲染器的平台。预烘焙是显而易见的答案——我们只是绕了一圈 SDF 才意识到。

---

## Takeaways / 总结

1. **Know your scale range.** SDF shines at infinite scalability. For bounded ranges (720p–4K), pre-baked bitmaps at discrete sizes are simpler, faster, and look better.

2. **Don't out-engineer the OS.** CoreText/DirectWrite/FreeType are industrial-grade font renderers with decades of optimization. A `smoothstep()` in a fragment shader won't beat them at their own game. If you can pre-render at the target size, do it.

3. **"No scaling" beats "smart scaling."** Our engine doesn't scale text — it picks the right pre-baked size. Zero approximation, zero artifacts, zero shader complexity.

4. **Toolchain is architecture.** `cargo pixel symbols` (Rust + CoreText) replaced Python + scipy + `msdfgen` (C++). Fewer moving parts, easier builds, happier contributors.

---

1. **了解你的缩放范围。** SDF 在无限可伸缩性上大放异彩。对于有限范围（720p–4K），离散尺寸的预烘焙位图更简单、更快、更好看。

2. **别试图超越操作系统。** CoreText/DirectWrite/FreeType 是有几十年优化的工业级字体渲染器。片段着色器里的 `smoothstep()` 不会在字体渲染这件事上赢过它们。如果你能在目标尺寸预渲染，就直接做。

3. **"不缩放"胜过"智能缩放"。** 我们的引擎不缩放文字——它选择正确的预烘焙尺寸。零近似、零瑕疵、零着色器复杂度。

4. **工具链就是架构。** `cargo pixel symbols`（Rust + CoreText）替换了 Python + scipy + `msdfgen`（C++）。更少的活动部件，更简单的构建，更开心的贡献者。

---

*RustPixel is open source: [github.com/zipxing/rust_pixel](https://github.com/zipxing/rust_pixel)*

*RustPixel 是开源项目：[github.com/zipxing/rust_pixel](https://github.com/zipxing/rust_pixel)*
