# Why I Replaced MSDF With Pre-Baked CoreText Mipmaps (and stopped fighting my own text renderer)

I tried MSDF for text in my engine. It worked… until I actually used the app like a normal human: resizing windows, switching between laptop screen and 4K, etc.

Turns out the core lesson is pretty boring but very real:

- **SDF/MSDF is great at scaling up.**
- **SDF/MSDF is not great at scaling down.**
- And TUI-style apps do both all the time.

---

## What is rust_pixel?

[rust_pixel](https://github.com/zipxing/rust_pixel) is my Rust 2D pixel / TUI engine. One render path for everything: sprites + terminal glyphs + CJK + emoji. It's "tile-first", GPU-backed, and I'm also building MDPT (a Markdown presentation tool) on top of it.

---

## The problem: MSDF looks nice zoomed in, less nice zoomed out

MSDF shines when you magnify text. Edges stay crisp and you don't get chunky pixels. But once you start **minifying** (small window, dense layout, smaller cell size), you're compressing detail into fewer pixels — and the usual MSDF reconstruction + filtering tends to produce:

- slightly mushy strokes
- occasional halo / fringing
- inconsistent stroke weight at tiny sizes

This isn't "MSDF is bad". It's just the wrong tradeoff for what I'm doing. In a map editor with infinite zoom? Totally worth it. In a terminal-like UI where sizes are bounded? I'd rather not.

---

## The fix: pre-bake a few sizes with CoreText and pick the closest one

In my use case, text sizes are basically bounded (roughly laptop → 4K fullscreen). That's not "infinite zoom". It's a small set of practical cell sizes.

So instead of continuously scaling an MSDF, I pre-render each glyph at a few target sizes using CoreText, pack them into a `Texture2DArray`, and at runtime I just choose the closest mip level based on the current cell pixel size.

| Cell size (px) | Mip | Pre-baked size |
|---|---|---|
| ≥ 48 px/cell | mip0 | large |
| ≥ 24 px/cell | mip1 | medium |
| < 24 px/cell | mip2 | small |

**Key point: the engine does zero scaling for glyphs.** No MSDF reconstruction. No `median3()`. No `smoothstep()`. No special "text shader". Just sample the right layer:

```wgsl
let texColor = textureSample(t_texture, s_sampler, uv, layer);
output.color = texColor * instance_color;
```

---

## Why this works so well for TUI

- Better small-size clarity (where MSDF often goes soft)
- More consistent stroke weight across typical window sizes
- Simpler shader (same path as sprites/UI)
- Tradeoff: more atlas space + a prebake step (fine for bounded sizes)

I'll add a screenshot comparison below — it's one of those "oh… yeah" moments.

---

## When I'd still use SDF/MSDF

If your app needs unbounded / unpredictable zoom (maps, vector editors, zoomable UIs), MSDF is still a great tool. But if your text sizes live in a small bounded range, pre-baked mipmaps are often the "boring solution" that just looks better.

---

**Repo:** <https://github.com/zipxing/rust_pixel>

**Previous rust_pixel demos** (if you want more context / visuals):

- [Demo #1 (MDPT)](https://www.reddit.com/r/rust/comments/1r1ontx/mdpt_markdown_tui_slides_with_gpu_rendering_not/) — Markdown TUI slides with GPU rendering
- [Demo #2 (TUI Tetris + bot)](https://www.reddit.com/r/rust/comments/1rceshu/tui_tetris_can_you_beat_the_bot_built_on_rust/) — Can you beat the bot?
- [Demo #3 (petview screensaver)](https://www.reddit.com/r/rust/comments/1rf7v47/rust_pixel_demo_3_petview_a_petscii_art/) — PETSCII art viewer
