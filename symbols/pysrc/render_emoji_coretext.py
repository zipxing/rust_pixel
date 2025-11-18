#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
使用 CoreText + Quartz 渲染 tui.txt 中的 emoji
生成高质量的 emoji spritesheet
"""

import os
import sys
import math

# PyObjC imports
from Foundation import NSURL, NSMutableAttributedString
from Quartz import (
    CGColorSpaceCreateDeviceRGB,
    CGBitmapContextCreate,
    CGBitmapContextCreateImage,
    CGImageDestinationCreateWithURL,
    CGImageDestinationAddImage,
    CGImageDestinationFinalize,
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
    kCTLineBoundsIncludeLanguageExtents,
)

# Fallback UTI
try:
    from Quartz import kUTTypePNG
except Exception:
    kUTTypePNG = "public.png"

# ---------------- 配置 ----------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
TUI_TXT = os.path.join(SCRIPT_DIR, "tui.txt")

# 输出配置
OUTPUT_FILE = os.path.join(SCRIPT_DIR, "emoji_rendered_coretext.png")
CELL_SIZE = 64          # 单元格大小（像素）
COLS = 9                # 列数（emoji 每行 9 个）
RENDER_PX = 256         # 超采样渲染尺寸
FONT_NAME = "AppleColorEmoji"
# ------------------------------------------


def parse_tui_txt(filename):
    """解析 tui.txt，提取 emoji"""
    with open(filename, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    sections = []
    current_section = []
    
    for line in lines:
        stripped = line.strip()
        if stripped:
            current_section.append(stripped)
        elif current_section:
            sections.append(current_section)
            current_section = []
    
    if current_section:
        sections.append(current_section)
    
    # 提取 emoji（section 2 和 3）
    def extract_emojis(section_lines):
        emojis = []
        for line in section_lines:
            i = 0
            while i < len(line):
                char = line[i]
                # Check for variation selector or ZWJ
                if i + 1 < len(line) and ord(line[i + 1]) in [0xFE0F, 0x200D]:
                    j = i + 1
                    while j < len(line) and (ord(line[j]) in [0xFE0F, 0x200D] or (j > i and ord(line[j]) >= 0x1F000)):
                        j += 1
                    emojis.append(line[i:j])
                    i = j
                elif char.strip():
                    emojis.append(char)
                    i += 1
                else:
                    i += 1
        return emojis
    
    emoji1 = extract_emojis(sections[1]) if len(sections) > 1 else []
    emoji2 = extract_emojis(sections[2]) if len(sections) > 2 else []
    
    return emoji1 + emoji2


def make_bitmap_context(width, height):
    """创建 RGBA 位图上下文"""
    color_space = CGColorSpaceCreateDeviceRGB()
    bytes_per_row = width * 4
    ctx = CGBitmapContextCreate(
        None, width, height, 8, bytes_per_row, color_space, kCGImageAlphaPremultipliedLast
    )
    return ctx


def draw_emoji_rgba(emoji: str, font_name: str, render_px: int):
    """
    使用 CoreText/Quartz 渲染单个 emoji
    
    Args:
        emoji: emoji 字符
        font_name: 字体名称
        render_px: 渲染尺寸
    
    Returns:
        CGImage
    """
    ctx = make_bitmap_context(render_px, render_px)
    
    # 翻转到 CoreGraphics 坐标系（原点在左下角）
    CGContextTranslateCTM(ctx, 0, render_px)
    CGContextScaleCTM(ctx, 1.0, -1.0)
    
    if not emoji or not emoji.strip():
        # 透明图像
        return CGBitmapContextCreateImage(ctx)
    
    # 创建字体和属性字符串
    font = CTFontCreateWithName(font_name, render_px * 0.8, None)
    attrs = {kCTFontAttributeName: font}
    astr = NSMutableAttributedString.alloc().initWithString_attributes_(emoji, attrs)
    line = CTLineCreateWithAttributedString(astr)
    
    # 测量并居中
    bounds = CTLineGetBoundsWithOptions(line, kCTLineBoundsIncludeLanguageExtents)
    bw = bounds.size.width
    bh = bounds.size.height
    bx = bounds.origin.x
    by = bounds.origin.y
    
    tx = (render_px - bw) / 2.0 - bx
    ty = (render_px - bh) / 2.0 - by
    
    CG.CGContextSetTextDrawingMode(ctx, CG.kCGTextFill)
    CG.CGContextSetRGBFillColor(ctx, 1, 1, 1, 1)
    
    CG.CGContextSetShouldAntialias(ctx, True)
    CG.CGContextSetAllowsAntialiasing(ctx, True)
    
    CG.CGContextSaveGState(ctx)
    CG.CGContextTranslateCTM(ctx, tx, ty)
    CTLineDraw(line, ctx)
    CG.CGContextRestoreGState(ctx)
    
    img = CGBitmapContextCreateImage(ctx)
    return img


def paste_scaled(dest_ctx, src_img, dx, dy, dw, dh):
    """将 src_img 缩放绘制到 dest_ctx"""
    CGContextSetInterpolationQuality(dest_ctx, kCGInterpolationHigh)
    rect = CGRectMake(dx, dy, dw, dh)
    CGContextDrawImage(dest_ctx, rect, src_img)


def main():
    print("="*60)
    print("CoreText Emoji 渲染工具")
    print("="*60)
    
    # 检查文件
    if not os.path.exists(TUI_TXT):
        print(f"错误: 找不到 {TUI_TXT}")
        sys.exit(1)
    
    # 解析 emoji
    print(f"\n解析 {TUI_TXT}...")
    emojis = parse_tui_txt(TUI_TXT)
    print(f"  找到 {len(emojis)} 个 emoji")
    
    if not emojis:
        print("错误: 没有找到 emoji")
        sys.exit(1)
    
    # 计算行数
    rows = math.ceil(len(emojis) / COLS)
    
    # 创建目标上下文
    W = COLS * CELL_SIZE
    H = rows * CELL_SIZE
    dest = make_bitmap_context(W, H)
    
    print(f"\n渲染配置:")
    print(f"  输出尺寸: {W}x{H}")
    print(f"  网格: {COLS}列 x {rows}行")
    print(f"  单元格: {CELL_SIZE}x{CELL_SIZE}")
    print(f"  渲染尺寸: {RENDER_PX}x{RENDER_PX}")
    print(f"  字体: {FONT_NAME}")
    
    # 翻转到左下角原点
    CGContextTranslateCTM(dest, 0, H)
    CGContextScaleCTM(dest, 1.0, -1.0)
    
    # 渲染每个 emoji
    print(f"\n开始渲染...")
    for idx, emoji in enumerate(emojis):
        gx = idx % COLS
        gy = idx // COLS
        dx = gx * CELL_SIZE
        dy = (rows - 1 - gy) * CELL_SIZE  # 因为翻转了 CTM
        
        cgimg = draw_emoji_rgba(emoji, FONT_NAME, RENDER_PX)
        paste_scaled(dest, cgimg, dx, dy, CELL_SIZE, CELL_SIZE)
        
        if (idx + 1) % 50 == 0:
            print(f"  已渲染 {idx + 1}/{len(emojis)}...")
    
    print(f"  ✓ 渲染完成: {len(emojis)} 个 emoji")
    
    # 导出 PNG
    print(f"\n保存到 {OUTPUT_FILE}...")
    out_url = NSURL.fileURLWithPath_(OUTPUT_FILE)
    dest_img = CGBitmapContextCreateImage(dest)
    image_dest = CGImageDestinationCreateWithURL(out_url, kUTTypePNG, 1, None)
    CGImageDestinationAddImage(image_dest, dest_img, None)
    ok = CGImageDestinationFinalize(image_dest)
    
    if not ok:
        print("错误: 保存 PNG 失败", file=sys.stderr)
        sys.exit(2)
    
    print("\n" + "="*60)
    print("✓ 完成！")
    print("="*60)
    print(f"\n生成的文件:")
    print(f"  - {OUTPUT_FILE}")
    print(f"  - 尺寸: {W}x{H}")
    print(f"  - 网格: {COLS}x{rows}")
    print(f"  - Emoji 数量: {len(emojis)}")


if __name__ == "__main__":
    main()

