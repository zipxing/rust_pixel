#!/usr/bin/env python
# coding=utf-8
#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
CoreText-only glyph render debug (160x320)
- Load font strictly from file path (CTFontDescriptor from URL)
- Solve font size by CoreText metrics (bisection on lineHeight)
- Apply width constraint using glyph path bounds to avoid clipping
- Render single character via CTLineDraw into CGBitmapContext
"""

import os
import math
import tempfile
import Quartz
import CoreText
import numpy as np
from PIL import Image


# ========= CONFIG =========
# FONT_PATH = os.path.expanduser("~/Library/Fonts/DejaVuSansMNerdFont-Regular.ttf")
FONT_PATH = os.path.expanduser("~/Library/Fonts/DroidSansMNerdFontMono-Regular.otf")
OUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "debug_coretext_160x320")

WIDTH = 160
HEIGHT = 320

PADDING = 0.92          # target line height = HEIGHT * PADDING
WIDTH_MARGIN = 0.98     # max ink width <= WIDTH * WIDTH_MARGIN
BOUNDS_OPT = CoreText.kCTLineBoundsUseGlyphPathBounds  # best for "ink" bounds
# ==========================


def cfurl_from_path(path: str):
    b = path.encode("utf-8")
    return Quartz.CFURLCreateFromFileSystemRepresentation(None, b, len(b), False)


def ctfont_from_file(font_path: str, size: float):
    """Load a CTFont from an exact font file path (avoid name matching/fallback)."""
    url = cfurl_from_path(font_path)
    descs = CoreText.CTFontManagerCreateFontDescriptorsFromURL(url)
    if not descs or len(descs) == 0:
        raise RuntimeError(f"Failed to create font descriptors from: {font_path}")
    desc = descs[0]
    return CoreText.CTFontCreateWithFontDescriptor(desc, float(size), None)


def ct_line_for_char(ctfont, ch: str):
    attrs = {
        CoreText.kCTFontAttributeName: ctfont,
        CoreText.kCTForegroundColorFromContextAttributeName: True,
    }
    s = CoreText.CFAttributedStringCreate(None, ch, attrs)
    return CoreText.CTLineCreateWithAttributedString(s)


def ct_line_typo_width(line):
    tb = CoreText.CTLineGetTypographicBounds(line, None, None, None)
    # PyObjC often returns (width, ascent, descent, leading)
    if isinstance(tb, tuple):
        return float(tb[0])
    return float(tb)


def ct_line_ink_bounds(line):
    r = CoreText.CTLineGetBoundsWithOptions(line, BOUNDS_OPT)
    # r is CGRect with origin/size
    return r


def solve_font_size_for_height(font_path: str, target_h: float, padding: float = 0.92):
    """Bisection: find size such that ascent+descent+leading ~= target_h*padding"""
    target = target_h * padding
    lo, hi = 1.0, 512.0

    for _ in range(32):
        mid = (lo + hi) / 2.0
        f = ctfont_from_file(font_path, mid)
        h = float(CoreText.CTFontGetAscent(f) + CoreText.CTFontGetDescent(f) + CoreText.CTFontGetLeading(f))
        if h < target:
            lo = mid
        else:
            hi = mid

    return (lo + hi) / 2.0


def apply_width_constraint(font_path: str, size: float, cell_w: float, margin: float = 0.98):
    """Scale size down if ink bounds exceed width for worst-case glyphs."""
    f = ctfont_from_file(font_path, size)
    worst = 0.0
    worst_ch = None

    # Wide candidates for mono fonts; add any you care about
    test_chars = "W@M#%&QG"
    for ch in test_chars:
        line = ct_line_for_char(f, ch)
        r = ct_line_ink_bounds(line)
        w = float(r.size.width)
        if w > worst:
            worst = w
            worst_ch = ch

    limit = cell_w * margin
    if worst <= limit:
        return size, (worst_ch, worst, limit)

    scaled = size * (limit / worst)
    return scaled, (worst_ch, worst, limit)


def compute_positions(ctfont, ch: str):
    """
    Compute x, baseline_y using ink bounds (glyph path bounds) to center,
    and font ascent/descent to vertically center line box.
    """
    ascent = float(CoreText.CTFontGetAscent(ctfont))
    descent = float(CoreText.CTFontGetDescent(ctfont))
    leading = float(CoreText.CTFontGetLeading(ctfont))

    line = ct_line_for_char(ctfont, ch)
    ink = ct_line_ink_bounds(line)

    # Center ink bounds horizontally: (WIDTH - ink_w)/2 - ink_origin_x
    x = (WIDTH - float(ink.size.width)) / 2.0 - float(ink.origin.x)

    # Vertically center using (ascent+descent) box (stable baseline):
    baseline_y = (HEIGHT - (ascent + descent)) / 2.0 + descent

    # Pixel snap (important for small-ish sizes; harmless here)
    x = round(x)
    baseline_y = round(baseline_y)

    return line, x, baseline_y, (ascent, descent, leading), ink


def render_char_to_image(ctfont, ch: str):
    """Render one char into an RGBA PIL image."""
    color_space = Quartz.CGColorSpaceCreateDeviceRGB()
    ctx = Quartz.CGBitmapContextCreate(
        None, WIDTH, HEIGHT, 8, WIDTH * 4, color_space,
        Quartz.kCGImageAlphaPremultipliedLast
    )

    Quartz.CGContextClearRect(ctx, Quartz.CGRectMake(0, 0, WIDTH, HEIGHT))
    Quartz.CGContextSetTextDrawingMode(ctx, Quartz.kCGTextFill)
    Quartz.CGContextSetRGBFillColor(ctx, 1.0, 1.0, 1.0, 1.0)

    line, x, baseline_y, (ascent, descent, leading), ink = compute_positions(ctfont, ch)

    Quartz.CGContextSetTextPosition(ctx, x, baseline_y)
    CoreText.CTLineDraw(line, ctx)

    cg_image = Quartz.CGBitmapContextCreateImage(ctx)

    # Write to temp PNG then load via PIL (simple & reliable)
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        tmp_path = tmp.name

    url = Quartz.CFURLCreateFromFileSystemRepresentation(
        None, tmp_path.encode("utf-8"), len(tmp_path.encode("utf-8")), False
    )
    dest = Quartz.CGImageDestinationCreateWithURL(url, "public.png", 1, None)
    Quartz.CGImageDestinationAddImage(dest, cg_image, None)
    Quartz.CGImageDestinationFinalize(dest)

    img = Image.open(tmp_path).convert("RGBA").copy()
    os.unlink(tmp_path)

    # Analyze clipping
    arr = np.array(img)
    alpha = arr[:, :, 3]
    cols = np.any(alpha > 0, axis=0)
    rows = np.any(alpha > 0, axis=1)

    clip_info = None
    if np.any(cols) and np.any(rows):
        x_min = int(np.where(cols)[0][0])
        x_max = int(np.where(cols)[0][-1])
        y_min = int(np.where(rows)[0][0])
        y_max = int(np.where(rows)[0][-1])
        clipped = []
        if x_min == 0: clipped.append("L")
        if x_max == WIDTH - 1: clipped.append("R")
        if y_min == 0: clipped.append("T")
        if y_max == HEIGHT - 1: clipped.append("B")
        clip_info = (x_min, x_max, y_min, y_max, clipped)

    return img, (x, baseline_y, ascent, descent, leading, ink), clip_info


def safe_name(ch: str):
    if ch.isalnum():
        return ch
    return f"U{ord(ch):04X}"


def main():
    os.makedirs(OUT_DIR, exist_ok=True)

    if not os.path.exists(FONT_PATH):
        raise FileNotFoundError(f"FONT_PATH not found: {FONT_PATH}")

    print("=== CoreText Font Load ===")
    print("FONT_PATH:", FONT_PATH)
    print(f"Canvas: {WIDTH}x{HEIGHT}")

    # 1) size by height (CoreText metrics)
    size_h = solve_font_size_for_height(FONT_PATH, HEIGHT, PADDING)
    # 2) width constraint
    size, (worst_ch, worst_w, limit_w) = apply_width_constraint(FONT_PATH, size_h, WIDTH, WIDTH_MARGIN)

    font = ctfont_from_file(FONT_PATH, size)

    a = float(CoreText.CTFontGetAscent(font))
    d = float(CoreText.CTFontGetDescent(font))
    l = float(CoreText.CTFontGetLeading(font))
    print("\n=== Solved font size ===")
    print(f"size_by_height: {size_h:.3f}")
    print(f"worst_ink_char: '{worst_ch}' ink_w={worst_w:.2f}, limit={limit_w:.2f}")
    print(f"final_size:     {size:.3f}")
    print(f"metrics: ascent={a:.2f}, descent={d:.2f}, leading={l:.2f}, lineH={a+d+l:.2f}, target={HEIGHT*PADDING:.2f}")

    # Test set (extend as needed)
    test_chars = ["G", "Q", "W", "M", "@", "f", "t", "i", "A", "g", "#", "%", "&"]
    for ch in test_chars:
        img, (x, by, ascent, descent, leading, ink), clip = render_char_to_image(font, ch)
        name = safe_name(ch)
        out_path = os.path.join(OUT_DIR, f"{name}.png")
        img.save(out_path)

        line = ct_line_for_char(font, ch)
        tw = ct_line_typo_width(line)

        print(f"\n[{ch}]")
        print(f"  pos: x={x}, baseline_y={by}")
        print(f"  CT metrics: ascent={ascent:.2f}, descent={descent:.2f}, leading={leading:.2f}")
        print(f"  typo_width={tw:.2f}")
        print(f"  ink: origin=({ink.origin.x:.2f},{ink.origin.y:.2f}) size=({ink.size.width:.2f},{ink.size.height:.2f})")
        if clip is None:
            print(f"  clip: (empty?)")
        else:
            x_min, x_max, y_min, y_max, sides = clip
            print(f"  pixels: x=[{x_min},{x_max}] y=[{y_min},{y_max}] clip={','.join(sides) if sides else 'OK'}")

    print("\nSaved to:", OUT_DIR)


if __name__ == "__main__":
    main()
