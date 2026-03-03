# Crisp Text at Any Scale in a Pixel Art Engine: From SDF to Mipmaps
# 像素引擎里的清晰文字：从 SDF 到 Mipmap 的实战之路

---

*[RustPixel](https://github.com/zipxing/rust_pixel) is a Rust 2D game engine with a unique twist: it renders everything — pixel art sprites, TUI text, CJK characters, emoji — through a single instanced draw call. One of its flagship apps is MDPT, a fullscreen Markdown presentation tool that blends pixel aesthetics with readable text. This is the story of how we solved its hardest rendering problem.*

*[RustPixel](https://github.com/zipxing/rust_pixel) 是一个 Rust 2D 游戏引擎，有个独特之处：它用单次实例化 draw call 渲染所有内容——像素精灵、TUI 文字、中文字符、Emoji。其旗舰应用 MDPT 是一个全屏 Markdown 演示工具，把像素美学和可读文字融为一体。这是我们解决其中最难的渲染问题的故事。*

---

## The Challenge: Pixel Art + Readable Text in One Draw Call / 挑战：一次 Draw Call 搞定像素美术 + 可读文字

Here's what makes RustPixel unusual: a single frame might contain C64-style 16×16 pixel sprites, box-drawing characters, Nerd Font icons, Chinese text rendered in PingFang SC, and color emoji — all from one texture, all in one draw call.

RustPixel 的独特之处在于：一帧画面里可能同时有 C64 风格 16×16 像素精灵、制表符、Nerd Font 图标、苹方渲染的中文、彩色 Emoji——全部来自一张纹理，一次 draw call。

In a normal pixel art game, you'd just render everything as bitmaps. But MDPT changes the equation: it's a **fullscreen presentation tool**. When you go fullscreen on a 4K display, a TUI cell originally designed at 32×64 pixels gets magnified 4–8×. Pixel art sprites look intentionally chunky at that scale — that's the aesthetic. But text? Text just looks blurry and broken.

在普通像素游戏里，所有东西都用位图渲染就行。但 MDPT 改变了这个等式：它是一个**全屏演示工具**。当你在 4K 显示器上全屏时，原本 32×64 像素的 TUI 字符格子要放大 4-8 倍。像素精灵在这个缩放下看起来是故意的粗粝感——那是美学。但文字？文字就是糊了。

**The core tension: pixel sprites should stay pixelated, but text should stay crisp. Same texture. Same shader. Same draw call.**

**核心矛盾：像素精灵应该保持像素感，但文字应该保持清晰。同一张纹理，同一个着色器，同一次 draw call。**

---

## Attempt 1: SDF/MSDF (v2.1 – v2.3) / 方案一：SDF/MSDF（v2.1 – v2.3）

The textbook answer for resolution-independent text is Signed Distance Fields. We implemented a hybrid approach:

教科书式的分辨率无关文字方案是签名距离场。我们实现了混合方案：

- **TUI characters**: True MSDF via `msdfgen` — sharp corners preserved
- **CJK (PingFang SC)**: Bitmap-to-SDF via `scipy` — Apple locks the glyph outlines
- **Sprites & Emoji**: Regular bitmaps — pixel art should stay pixelated

- **TUI 字符**：用 `msdfgen` 生成真 MSDF——保留尖角
- **CJK（苹方）**：位图转 SDF，用 `scipy`——苹果锁了字形轮廓
- **精灵 & Emoji**：普通位图——像素美术就该有像素感

We packed everything into an 8192×8192 atlas with four rendering regions, and the fragment shader branched per-fragment:

所有东西打包进 8192×8192 的图集，四个渲染区域，片段着色器逐片段分支：

```wgsl
if (v_msdf > 0.5) {
    // Distance field path: math-based edge reconstruction
    let d = median3(texColor.r, texColor.g, texColor.b);
    let alpha = smoothstep(0.5 - fwidth(d), 0.5 + fwidth(d), d);
    output.color = vec4(fg_color.rgb, alpha);
} else {
    // Bitmap path: direct texture sampling
    output.color = texColor * instance_color;
}
```

The MSDF flag was cleverly encoded in the sign bit of `origin_y` — zero extra memory per instance.

MSDF 标记巧妙地编码在 `origin_y` 的符号位里——每个实例零额外内存。

**It worked.** Text was crisp at any zoom. But after several months in production...

**它工作了。** 文字在任何缩放下都清晰。但在生产环境跑了几个月后……

---

## Why SDF Didn't Work For Us / 为什么 SDF 对我们不适用

### The dual-path problem / 双路径问题

Every rendering feature needed two implementations. Bold text? MSDF bold lowers the distance threshold from 0.5 to 0.45. Bitmap bold samples neighboring texels. Two completely different algorithms for the same visual feature. Bugs loved this.

每个渲染特性都需要两套实现。粗体？MSDF 粗体把距离阈值从 0.5 降到 0.45，位图粗体采样相邻纹素。同一个视觉效果的两种完全不同的算法。Bug 最喜欢这种了。

### The CJK compromise / CJK 的妥协

Real MSDF needs vector glyph outlines. PingFang SC — the best CJK font on macOS — hides its outlines inside Apple's `.ttc` format. Our fallback was `bitmap → distance_transform_edt → SDF`, which looked *okay* but lost the sharp edge quality that justified using SDF in the first place. One atlas, three quality tiers. Not great.

真正的 MSDF 需要矢量字形轮廓。苹方——macOS 上最好的中文字体——把轮廓藏在苹果的 `.ttc` 格式里。我们的退路是 `位图 → distance_transform_edt → SDF`，效果*还行*但丢失了用 SDF 的初衷——清晰的边缘质量。一张图集，三个质量档次。不太行。

### The toolchain friction / 工具链摩擦

Generating the atlas required Python 3 + scipy + Pillow + the `msdfgen` C++ binary. For a Rust engine, asking contributors to install a Python scientific computing stack plus a C++ tool felt wrong.

生成图集需要 Python 3 + scipy + Pillow + `msdfgen` C++ 二进制。对于一个 Rust 引擎，让贡献者安装 Python 科学计算栈加 C++ 工具，这不对。

### The 8192×8192 limit / 8192×8192 的限制

To fit ~48,000 symbols with SDF padding, we needed an 8192×8192 atlas (256MB uncompressed). Some mobile GPUs and older WebGL implementations choke on textures this large.

为了放下约 48000 个符号加 SDF 边距，我们需要 8192×8192 的图集（未压缩 256MB）。部分移动端 GPU 和旧版 WebGL 处理不了这么大的纹理。

---

## The Insight / 顿悟

We stepped back and looked at what MDPT actually does. The screenshots tell the story: pixel sprites, formatted text, code blocks, CJK, emoji — all coexisting on screen. The display size range is known: from small windows (~720p) to fullscreen 4K. We don't need infinite mathematical scalability. We need "good enough at 3-4 sizes."

我们退后一步，看了看 MDPT 实际在做什么。截图说明一切：像素精灵、格式化文字、代码块、中文、Emoji——全部共存在屏幕上。显示尺寸范围是已知的：从小窗口（~720p）到全屏 4K。我们不需要无限的数学可伸缩性，我们需要的是"在 3-4 个尺寸下足够好"。

**The simplest solution: pre-render every symbol at multiple resolutions and pick the right one at render time.**

**最简单的方案：把每个符号预渲染成多个分辨率，渲染时选对的那个。**

---

## The Mipmap Solution (v2.4+) / Mipmap 方案（v2.4+）

### Texture2DArray with 3-level mipmaps / 三级 Mipmap 的 Texture2DArray

Instead of one giant 8192×8192 atlas, we use a `Texture2DArray` of multiple 4096×4096 layers. Each symbol is pre-rendered at 3 resolutions:

不再用一张巨大的 8192×8192 图集，而是用多个 4096×4096 层的 `Texture2DArray`。每个符号预渲染 3 个分辨率：

| Render Size / 渲染尺寸 | Mip Level | Scene / 场景 |
|------------------------|-----------|-------------|
| ≥ 48 px/unit | mip0 (high) | Fullscreen 4K / 全屏 4K |
| ≥ 24 px/unit | mip1 (mid) | Normal window / 普通窗口 |
| < 24 px/unit | mip2 (low) | Small window, thumbnails / 小窗口 |

```rust
pub struct Tile {
    pub cell_w: u8,       // 1=normal, 2=wide (CJK/Emoji)
    pub cell_h: u8,       // 1=single, 2=tall (TUI/CJK)
    pub is_emoji: bool,   // Color emoji — no tint
    pub mips: [MipUV; 3], // UV coords for each mip level
}
```

The engine selects the mip level based on actual pixel size per cell. No shader branching. No distance field math. Just pick the right pre-rendered bitmap:

引擎根据每个格子的实际像素尺寸选择 mip 级别。没有着色器分支，没有距离场数学，只是选对预渲染好的位图：

```wgsl
// That's the entire fragment shader for ALL symbol types
let texColor = textureSample(t_texture, s_sampler, uv, layer);
output.color = texColor * instance_color;
```

**Sprites, TUI glyphs, CJK, emoji — all the same code path. One draw call. No branching.**

**精灵、TUI 字形、CJK、Emoji——全部走同一条代码路径。一次 draw call，无分支。**

### Pure Rust toolchain / 纯 Rust 工具链

The texture generator is now a Rust tool using macOS CoreText bindings directly:

纹理生成器现在是 Rust 工具，直接用 macOS CoreText 绑定：

```bash
cargo pixel symbols -o assets/pix
```

No Python. No scipy. No external binaries. It uses DP shelf-packing to optimally fit symbols into 4096×4096 layers, and outputs a compact JSON symbol map (~375KB) mapping every symbol to its 3-level mip UV coordinates.

没有 Python，没有 scipy，没有外部二进制。用 DP 货架打包算法最优地把符号装进 4096×4096 的层，输出一个紧凑的 JSON 符号映射（~375KB），每个符号对应三级 mip 的 UV 坐标。

---

## What Changed — Side by Side / 前后对比

| | SDF/MSDF (v2.3) | Mipmap (v2.4+) |
|---|---|---|
| **Max texture** | 8192×8192 (single) | 4096×4096 × ~15 layers |
| **Fragment shader** | Dual path (SDF + bitmap) | Single path (bitmap only) |
| **CJK quality** | Acceptable (bitmap→SDF) | Good (native CoreText) |
| **Scaling** | Infinite (math) | 3 discrete levels |
| **Toolchain** | Python + scipy + msdfgen | Pure Rust (`cargo pixel symbols`) |
| **Draw calls** | 1 | 1 |
| **Bold** | 2 implementations | 1 implementation |
| **WebGL compat** | ⚠️ 8K texture limit | ✅ 4K layers |
| **Rendering bugs since switch** | — | 0 |

---

## The Pixel Art Factor / 像素美术的因素

Here's something specific to pixel art engines that general-purpose text rendering articles don't cover:

有个像素美术引擎特有的点，通用文字渲染文章不会提到：

**Pixel sprites WANT to be pixelated.** That's the whole aesthetic. When a C64-style character is magnified 8×, those chunky pixels are a feature, not a bug. SDF's "smooth at any scale" philosophy actively fights against this — you end up needing the dual-path shader specifically to *prevent* SDF from "improving" your pixel art.

**像素精灵就是要有像素感的。** 这就是美学。当 C64 风格的角色放大 8 倍时，那些粗粝的像素是特性而非缺陷。SDF 的"任何缩放下都平滑"的理念恰好与此对立——你需要双路径着色器，专门来*阻止* SDF "改善"你的像素美术。

With mipmaps, the solution is natural: pixel sprites have the same bitmap at all 3 mip levels (or just mip2). They stay chunky. Text symbols have progressively higher-resolution bitmaps. They stay crisp. Same texture array, same shader, same draw call — but the content itself encodes the intent.

用 mipmap，解决方案很自然：像素精灵在 3 个 mip 级别用相同的位图（或只用 mip2），保持粗粝。文字符号有逐级更高分辨率的位图，保持清晰。同一个纹理数组、同一个着色器、同一次 draw call——但内容本身编码了意图。

---

## Honest Tradeoffs / 诚实的取舍

**What we gave up:**

**我们放弃了什么：**

- **Infinite scalability.** If someone runs MDPT on an 8K display, mip0 at 48+ px/unit is still good, but not mathematically perfect. We're okay with that — our target is 720p to 4K.
- **Memory efficiency at extreme scales.** We store 3 copies of every symbol. In practice, total memory is similar because SDF needed larger cells for distance field padding, and our DP packing is tighter.
- **The cool factor.** "We use multi-channel signed distance fields" sounds way more impressive at conferences than "we pre-render at 3 sizes."

- **无限可伸缩性。** 如果有人在 8K 显示器上跑 MDPT，mip0 在 48+ px/unit 下仍然很好，但不是数学上完美的。我们接受——目标是 720p 到 4K。
- **极端缩放下的内存效率。** 我们存了每个符号的 3 个副本。但实际总内存差不多，因为 SDF 需要更大的格子放距离场边距，而我们的 DP 打包更紧凑。
- **炫酷因素。** "我们用多通道签名距离场"在会议上听着比"我们预渲染 3 个尺寸"帅多了。

**What we gained:**

**我们获得了什么：**

- Zero rendering bugs since the switch
- A fragment shader anyone can understand in 5 seconds
- A Rust-native toolchain that `cargo` can build
- Better CJK quality (native font rendering > mathematical approximation)
- Smaller max texture size (4K vs 8K layers)
- One rendering path for everything: sprites, text, CJK, emoji

- 切换以来零渲染 Bug
- 任何人 5 秒就能看懂的片段着色器
- `cargo` 就能构建的 Rust 原生工具链
- 更好的 CJK 质量（原生字体渲染 > 数学近似）
- 更小的最大纹理尺寸（4K vs 8K 层）
- 所有内容走一条渲染路径：精灵、文字、CJK、Emoji

---

## Takeaways / 总结

1. **Know your actual scale range.** "Works at any scale" is seductive, but if your actual range is 720p–4K, discrete mipmaps are simpler and sufficient.

2. **In pixel art engines, SDF fights your aesthetic.** You need a dual-path shader just to prevent SDF from smoothing your intentionally pixelated content. Mipmaps let the content itself carry the intent — pixel art stays chunky, text stays crisp.

3. **Toolchain is part of the architecture.** Replacing Python + scipy + C++ with `cargo pixel symbols` wasn't just convenience — it removed a class of contributor friction and CI complexity.

4. **Boring engineering wins.** Pre-rendering at 3 sizes isn't clever. It's reliable. We haven't touched the rendering pipeline since the switch.

---

1. **了解你的实际缩放范围。** "任何缩放都能用"很诱人，但如果实际范围是 720p–4K，离散 mipmap 更简单且足够。

2. **在像素引擎里，SDF 与你的美学对抗。** 你需要双路径着色器只是为了阻止 SDF 平滑你故意做成像素风的内容。Mipmap 让内容本身承载意图——像素美术保持粗粝，文字保持清晰。

3. **工具链是架构的一部分。** 用 `cargo pixel symbols` 替换 Python + scipy + C++ 不只是方便——它消除了一整类贡献者摩擦和 CI 复杂度。

4. **无聊的工程赢了。** 预渲染 3 个尺寸不聪明，但可靠。切换以来我们没碰过渲染管线。

---

*RustPixel is open source: [github.com/zipxing/rust_pixel](https://github.com/zipxing/rust_pixel). MDPT and other apps are in the `apps/` directory.*

*RustPixel 是开源项目：[github.com/zipxing/rust_pixel](https://github.com/zipxing/rust_pixel)。MDPT 等应用在 `apps/` 目录。*
