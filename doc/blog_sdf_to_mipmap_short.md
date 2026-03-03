# Why We Replaced SDF With Pre-Baked CoreText Mipmaps
# 为什么我们用 CoreText 预烘焙 Mipmap 替代了 SDF

---

**TL;DR:** SDF excels at infinite scaling. But for TUI apps where text sizes are bounded, pre-baking bitmaps with a system font renderer like CoreText produces better results — because you skip the approximation entirely. The engine never scales text; it just picks the right pre-baked size.

**一句话版本：** SDF 擅长无限缩放。但对于文字尺寸有限的 TUI 应用，用 CoreText 这种系统字体渲染器预烘焙位图效果更好——因为你完全跳过了近似这一步。引擎从不缩放文字，只是选择正确的预烘焙尺寸。

---

## Context / 背景

[RustPixel](https://github.com/zipxing/rust_pixel) is a Rust 2D pixel engine. One draw call renders everything: sprites, TUI characters, CJK text, emoji. Our app MDPT (Markdown presentation tool) goes fullscreen on 4K — text cells get magnified 4–8×.

[RustPixel](https://github.com/zipxing/rust_pixel) 是一个 Rust 2D 像素引擎，一次 draw call 渲染所有内容：精灵、TUI 字符、中文、Emoji。我们的应用 MDPT（Markdown 演示工具）会在 4K 下全屏——文字格子被放大 4-8 倍。

We implemented SDF/MSDF. It worked, but the rendered text had **blur and fringing artifacts** from the `median3() + smoothstep()` reconstruction. Especially CJK — PingFang SC doesn't expose glyph outlines, so we had to go `CoreText render → bitmap → EDT → SDF → smoothstep reconstruction`. We were literally **degrading CoreText's pixel-perfect output** to feed it into a shader that produced worse results.

我们实现了 SDF/MSDF。能用，但渲染出的文字有 `median3() + smoothstep()` 重建带来的**模糊和毛刺**。中文尤其严重——苹方不暴露字形轮廓，所以我们走的路是 `CoreText 渲染 → 位图 → EDT → SDF → smoothstep 重建`。我们在**降级 CoreText 已经像素级完美的输出**，喂给一个效果更差的着色器。

---

## The Key Insight / 核心洞察

SDF solves a real problem: **rendering at unpredictable scales.** Map engines zooming 0.1×–1000×? SDF. Vector editors? SDF. Procedural UI with unknown layout sizes? SDF.

SDF 解决的是一个真实问题：**在不可预测的缩放下渲染。** 地图引擎 0.1×–1000× 缩放？SDF。矢量编辑器？SDF。布局尺寸未知的程序化 UI？SDF。

**But TUI applications have bounded text sizes.** A terminal-style app runs between ~720p and 4K. That's 3–4 discrete sizes, not infinite. And for known sizes, you have a much better tool: **the operating system's own font renderer.**

**但 TUI 应用的文字尺寸是有限的。** 终端风格的应用运行在 ~720p 到 4K 之间。那是 3-4 个离散的尺寸，不是无限。而对于已知的尺寸，你有个好得多的工具：**操作系统自己的字体渲染器。**

CoreText (macOS), DirectWrite (Windows), FreeType (Linux) — these are **industrial-grade font engines** with decades of optimization: sub-pixel anti-aliasing, hinting, kerning, CJK stroke optimization. A fragment shader doing `smoothstep(0.5 - fwidth(d), 0.5 + fwidth(d), d)` will not beat them at their own game.

CoreText（macOS）、DirectWrite（Windows）、FreeType（Linux）——这些是**工业级字体引擎**，有几十年的优化：亚像素抗锯齿、字体微调、字距调整、CJK 笔画优化。片段着色器里一个 `smoothstep(0.5 - fwidth(d), 0.5 + fwidth(d), d)` 不会在字体渲染这件事上赢过它们。

---

## Our Solution: 3-Level Mipmap Pre-Bake / 我们的方案：三级 Mipmap 预烘焙

Pre-render every symbol at 3 resolutions using CoreText, pack into a `Texture2DArray`:

用 CoreText 把每个符号预渲染成 3 个分辨率，打包进 `Texture2DArray`：

| Display Size / 显示尺寸 | Mip | What Renders / 渲染内容 |
|-------------------------|-----|------------------------|
| ≥ 48 px/cell | mip0 | CoreText @ 64–128px (4K fullscreen) |
| ≥ 24 px/cell | mip1 | CoreText @ 32–64px (normal window) |
| < 24 px/cell | mip2 | CoreText @ 16–32px (small window) |

**The engine does zero scaling.** It determines the cell's pixel size, picks the closest mip level, and renders the pre-baked texture directly. No `smoothstep`, no `median3`, no approximation.

**引擎做零缩放。** 它判断格子的像素尺寸，选最接近的 mip 级别，直接渲染预烘焙的纹理。没有 `smoothstep`，没有 `median3`，没有近似。

```wgsl
// Entire fragment shader — same path for sprites, TUI, CJK, emoji
let texColor = textureSample(t_texture, s_sampler, uv, layer);
output.color = texColor * instance_color;
```

The tool is pure Rust (`cargo pixel symbols`), calling CoreText directly. No Python, no scipy, no `msdfgen`.

工具是纯 Rust（`cargo pixel symbols`），直接调用 CoreText。没有 Python，没有 scipy，没有 `msdfgen`。

---

## SDF vs Mipmap Pre-Bake / 对比

| | SDF | Mipmap (CoreText pre-bake) |
|---|---|---|
| **Text quality** | Shader math approximation | System renderer, pixel-perfect |
| **Scaling** | Engine scales via shader | Engine picks size, no scaling |
| **Infinite zoom** | ✅ SDF's real strength | ❌ 3 levels (sufficient for TUI) |
| **Shader** | Dual path, branching | Single path, trivial |
| **CJK** | Degraded (bitmap→SDF→reconstruct) | Native CoreText rendering |

---

## When to Use What / 什么时候用什么

**Use SDF when** scale range is unbounded or unpredictable (maps, vector graphics, procedural UI).

**SDF 适用于**缩放范围无界或不可预测的场景（地图、矢量图形、程序化 UI）。

**Use pre-baked mipmaps when** scale range is bounded and you have access to a good font renderer (TUI apps, game engines with fixed display targets, presentation tools).

**预烘焙 mipmap 适用于**缩放范围有界、且你能用到好的字体渲染器的场景（TUI 应用、有固定显示目标的游戏引擎、演示工具）。

The question isn't "is SDF good?" — it's "does my use case need infinite scalability?" If yes, SDF. If no, let the OS font renderer do what it was built for, and just pick the right size.

问题不是"SDF 好不好"——而是"我的场景需要无限可伸缩性吗？"如果是，用 SDF。如果不是，让操作系统字体渲染器做它被造来做的事，你只需选对尺寸。

---

*RustPixel: [github.com/zipxing/rust_pixel](https://github.com/zipxing/rust_pixel)*
