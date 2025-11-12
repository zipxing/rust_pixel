#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
macOS-only (PyObjC) emoji spritesheet generator using CoreText + Quartz.
- Renders with the system text stack (like iTerm2), so Apple Color Emoji is fully colored.
- Outputs a combined spritesheet (default 16x16 cells -> 256 total) with RGBA pixels.
- Default cell size: 32x32 (configurable).
- You can pass a custom emoji list file (one emoji per line), otherwise a reasonable default set is used.

Usage:
  pip install pyobjc
  python make_emoji_sheet_coretext.py \
    --out ./emoji_sheet_32.png \
    --cell 32 \
    --cols 16 \
    --font "AppleColorEmoji" \
    --render_px 256

Notes:
- This script requires macOS with PyObjC. It uses CoreText/Quartz directly (no Pillow).
- For best clarity, we render at --render_px then downscale into 32x32 (or your --cell) with high-quality interpolation.
"""

import argparse
import math
import sys

# PyObjC imports
from Foundation import NSURL, NSMutableAttributedString, NSDictionary
from Quartz import (
    CGColorSpaceCreateDeviceRGB,
    CGBitmapContextCreate,
    CGBitmapContextCreateImage,
    CGImageDestinationCreateWithURL,
    CGImageDestinationAddImage,
    CGImageDestinationFinalize,
    CGDataProviderCreateWithCFData,
    CGDataConsumerCreateWithURL,
    CGImageCreate,
    CGContextDrawImage,
    CGContextScaleCTM,
    CGContextTranslateCTM,
    CGContextSetInterpolationQuality,
    kCGInterpolationHigh,
    CGRectMake,
    kCGImageAlphaPremultipliedLast,
)
import Quartz.CoreGraphics as CG
from CoreText import (
    CTFontCreateWithName,
    CTLineCreateWithAttributedString,
    CTLineDraw,
    CTLineGetBoundsWithOptions,
    kCTFontAttributeName,
    kCTFontOptionsPreventAutoActivation,
    kCTLineBoundsIncludeLanguageExtents,
)
from CoreFoundation import CFDataCreate, kCFAllocatorDefault

# Fallback UTI (older macOS)
try:
    from Quartz import kUTTypePNG
except Exception:
    # UTType from UniformTypeIdentifiers (macOS 11+), but CGImageDestination accepts kUTTypePNG if present.
    # We define it here as a string if missing.
    kUTTypePNG = "public.png"

# -----------------------------
# Default emoji set (fits 256 when cols*rows == 256).
# You can replace or extend this list; empty strings are allowed (render transparent cells).
# -----------------------------
DEFAULT_EMOJIS = [
    # A compact, clear set. You can replace with your 256-set if desired.
    "😀","😃","😄","😁","😆","😅","😂","🤣","😉","😊","🙂","🙃","😋","😍","🥰","😘",
    "😗","😚","😙","😜","😝","😛","🥲","🤗","🤔","🤨","😐","😑","🙄","😶","😏","😒",
    "😔","😞","😟","😕","☹️","😣","😖","😫","😩","🥺","😢","😭","😤","😠","😡","🤯",
    "😱","😳","✅","❌","⚠️","❗","❓","➕","➖","➗","♻️","🔺","🔻","🔸","🔹","⭐",
    "🌟","✨","💫","🎯","🚀","⚡","💡","🔔","📌","📍","🔗","🔒","⬆️","⬇️","⬅️","➡️",
    "↗️","↘️","↙️","↖️","🔼","🔽","⏫","⏬","⤴️","⤵️","🔁","🔄","🔃","▶️","⏸️","🍎",
    "🍊","🍋","🍌","🍉","🍇","🍓","🍒","🍍","🥭","🍑","🥥","🍕","🍔","🍟","🍿","🍩",
    "🍪","🍰","☕","☀️","🌤️","⛅","🌧️","⛄","🌈","🌸","🌺","🌻","🌼","🌲","🌳","🌵",
    "🍀","🍂","🍁","🐶","🐱","🐭","🐻","📁","📂","📄","📊","📈","📉","🗂️","🗃️","🔍","🔧",
    "🔨","⚙️","🖥️","💻","⌨️","🖱️","💾","🔋","🔌","⚽","🏀","🏈","⚾","🎾","🏐","🏓",
    "🏸","🎯","🎳","🎮","🎲","🎨","🎭","🎪",
] + [""] * (256 - 160)  # pad to 256 with transparent cells


def make_bitmap_context(width, height):
    color_space = CGColorSpaceCreateDeviceRGB()
    bytes_per_row = width * 4
    ctx = CGBitmapContextCreate(
        None, width, height, 8, bytes_per_row, color_space, kCGImageAlphaPremultipliedLast
    )
    return ctx

def draw_emoji_rgba(emoji: str, font_name: str, render_px: int):
    """Render a single emoji to an RGBA CGImage at render_px x render_px using CoreText/Quartz."""
    ctx = make_bitmap_context(render_px, render_px)
    # Flip to CoreGraphics canvas coords (origin bottom-left)
    CGContextTranslateCTM(ctx, 0, render_px)
    CGContextScaleCTM(ctx, 1.0, -1.0)

    if not emoji:
        # Transparent image
        return CGBitmapContextCreateImage(ctx)

    # Create font & attributed string
    font = CTFontCreateWithName(font_name, render_px * 0.8, None)  # 80% of canvas as a heuristic
    attrs = { kCTFontAttributeName: font }
    astr = NSMutableAttributedString.alloc().initWithString_attributes_(emoji, attrs)
    line = CTLineCreateWithAttributedString(astr)

    # Measure and center
    bounds = CTLineGetBoundsWithOptions(line, kCTLineBoundsIncludeLanguageExtents)
    bw = bounds.size.width
    bh = bounds.size.height
    bx = bounds.origin.x
    by = bounds.origin.y

    tx = (render_px - bw) / 2.0 - bx
    ty = (render_px - bh) / 2.0 - by

    CG.CGContextSetTextDrawingMode(ctx, CG.kCGTextFill)
    CG.CGContextSetRGBFillColor(ctx, 1, 1, 1, 1)  # color is ignored for color emoji layers

    CG.CGContextSetShouldAntialias(ctx, True)
    CG.CGContextSetAllowsAntialiasing(ctx, True)

    CG.CGContextSaveGState(ctx)
    CG.CGContextTranslateCTM(ctx, tx, ty)
    CTLineDraw(line, ctx)
    CG.CGContextRestoreGState(ctx)

    img = CGBitmapContextCreateImage(ctx)
    return img

def paste_scaled(dest_ctx, src_img, dx, dy, dw, dh):
    """Draw src_img scaled into dest_ctx at (dx,dy) with size dw x dh."""
    CGContextSetInterpolationQuality(dest_ctx, kCGInterpolationHigh)
    rect = CGRectMake(dx, dy, dw, dh)
    # Quartz expects bottom-left origin for images; our dest_ctx is already flipped to bottom-left after we flip it
    # We'll draw with current CTM (assumed bottom-left origin). To ensure correct placement, don't flip here.
    CGContextDrawImage(dest_ctx, rect, src_img)

def main():
    parser = argparse.ArgumentParser(description="Generate a 32x32 RGBA emoji spritesheet using CoreText/Quartz (macOS).")
    parser.add_argument("--out", type=str, default="./emoji_sheet_32.png", help="Output PNG path.")
    parser.add_argument("--cell", type=int, default=32, help="Cell size in pixels (default 32).")
    parser.add_argument("--cols", type=int, default=16, help="Columns in the sheet (default 16).")
    parser.add_argument("--font", type=str, default="AppleColorEmoji", help="Font PostScript name (e.g., AppleColorEmoji).")
    parser.add_argument("--render_px", type=int, default=256, help="Supersample render size before downscale (default 256).")
    parser.add_argument("--emoji_file", type=str, default="", help="Optional: path to a text file with one emoji per line.")
    args = parser.parse_args()

    # Build emoji list
    emojis = []
    if args.emoji_file:
        with open(args.emoji_file, "r", encoding="utf-8") as f:
            for line in f:
                s = line.strip()
                emojis.append(s)
    else:
        emojis = list(DEFAULT_EMOJIS)

    # Limit or pad to fill rows*cols
    total = args.cols * args.cols  # default 16*16
    if len(emojis) < total:
        emojis = emojis + [""] * (total - len(emojis))
    elif len(emojis) > total:
        emojis = emojis[:total]

    rows = math.ceil(len(emojis) / args.cols)

    # Create destination context (final spritesheet)
    W = args.cols * args.cell
    H = rows * args.cell
    dest = make_bitmap_context(W, H)

    # Flip to bottom-left origin
    CGContextTranslateCTM(dest, 0, H)
    CGContextScaleCTM(dest, 1.0, -1.0)

    # Draw each emoji
    for idx, ch in enumerate(emojis):
        gx = idx % args.cols
        gy = idx // args.cols
        dx = gx * args.cell
        dy = (rows - 1 - gy) * args.cell  # because we flipped the CTM

        cgimg = draw_emoji_rgba(ch, args.font, args.render_px)
        paste_scaled(dest, cgimg, dx, dy, args.cell, args.cell)

    # Export PNG
    out_url = NSURL.fileURLWithPath_(args.out)
    dest_img = CGBitmapContextCreateImage(dest)
    image_dest = CGImageDestinationCreateWithURL(out_url, kUTTypePNG, 1, None)
    CGImageDestinationAddImage(image_dest, dest_img, None)
    ok = CGImageDestinationFinalize(image_dest)
    if not ok:
        print("Failed to write PNG.", file=sys.stderr)
        sys.exit(2)

    print(f"[✓] Saved: {args.out}")
    print(f"[i] Size:  {W}x{H}  (cells: {args.cols}x{rows}, cell={args.cell})")
    print(f"[i] Font:  {args.font}  render_px={args.render_px}")

if __name__ == "__main__":
    main()
