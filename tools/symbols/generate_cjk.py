#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
CJK 多尺寸纹理生成工具

从 SUBTLEX-CH-CHR.csv 读取汉字频率表，生成两张纹理文件：

输出:
  - cjk.png:   4096x4096 纹理 (Layer 1)
    - y=0-383:     16px 区域 (24行 × 256列, 容量6144, 放5935字)
    - y=384-1887:  32px 区域 (47行 × 128列, 容量6016, 放5935字)
    - y=1888-4063: 64px 前段  (34行 × 64列,  容量2176, 放前2176字)

  - cjk64.png: 4096x4096 纹理 (Layer 2)
    - y=0-3775:    64px 后段  (59行 × 64列,  容量3776, 放剩余3759字)

字符来源: SUBTLEX-CH-CHR.csv (按词频降序排列)
"""

import os
import sys
import csv
import argparse
from PIL import Image

# 尝试导入 macOS 渲染库
try:
    import Quartz
    import CoreText
    HAS_QUARTZ = True
except ImportError:
    HAS_QUARTZ = False
    print("警告: Quartz/CoreText 不可用，将使用预渲染的字符图像")

# ---------------- 配置 ----------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# 输入文件: SUBTLEX-CH-CHR.csv (位于项目根目录)
PROJECT_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, "..", ".."))
SUBTLEX_CSV = os.path.join(PROJECT_ROOT, "SUBTLEX-CH-CHR.csv")

# 输出文件
OUTPUT_CJK = os.path.join(SCRIPT_DIR, "cjk.png")
OUTPUT_CJK64 = os.path.join(SCRIPT_DIR, "cjk64.png")

# 纹理参数
TEXTURE_SIZE = 4096

# CJK 字体
CJK_FONT_NAME = "DroidSansMono Nerd Font"

# 三种尺寸配置
# 尺寸 -> (cell_size, render_size, font_size, cols_per_row)
SIZE_CONFIG = {
    16: {"cell": 16, "render": 32, "font": 28, "cols": 256},
    32: {"cell": 32, "render": 64, "font": 56, "cols": 128},
    64: {"cell": 64, "render": 128, "font": 112, "cols": 64},
}
# ------------------------------------------


def render_char_quartz(char, width, height, font_name, font_size):
    """
    使用 macOS Quartz 渲染单个字符为 PIL Image

    Returns:
        PIL.Image: RGBA 图像
    """
    import tempfile

    # 创建位图上下文
    color_space = Quartz.CGColorSpaceCreateDeviceRGB()
    context = Quartz.CGBitmapContextCreate(
        None,
        width, height,
        8,
        width * 4,
        color_space,
        Quartz.kCGImageAlphaPremultipliedLast
    )

    if context is None:
        return None

    # 清空背景（透明）
    Quartz.CGContextClearRect(context, Quartz.CGRectMake(0, 0, width, height))

    # 设置文本绘制模式和颜色（白色）
    Quartz.CGContextSetTextDrawingMode(context, Quartz.kCGTextFill)
    Quartz.CGContextSetRGBFillColor(context, 1.0, 1.0, 1.0, 1.0)

    # 创建字体
    font = CoreText.CTFontCreateWithName(font_name, font_size, None)

    # 创建属性字符串
    attributes = {
        CoreText.kCTFontAttributeName: font,
        CoreText.kCTForegroundColorFromContextAttributeName: True
    }
    attr_string = CoreText.CFAttributedStringCreate(None, char, attributes)

    # 创建 CTLine
    line = CoreText.CTLineCreateWithAttributedString(attr_string)

    # 获取字体度量信息用于居中
    ascent = CoreText.CTFontGetAscent(font)
    descent = CoreText.CTFontGetDescent(font)
    leading = CoreText.CTFontGetLeading(font)
    bounds = CoreText.CTLineGetBoundsWithOptions(line, 0)

    # 计算居中位置
    x = (width - bounds.size.width) / 2.0
    font_height = ascent + descent + leading
    baseline_y = (height - font_height) / 2.0 + descent

    # 绘制文本
    Quartz.CGContextSetTextPosition(context, x, baseline_y)
    CoreText.CTLineDraw(line, context)

    # 创建 CGImage
    cg_image = Quartz.CGBitmapContextCreateImage(context)
    if cg_image is None:
        return None

    # 通过临时文件转换为 PIL Image
    with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as tmp:
        tmp_path = tmp.name

    try:
        url = Quartz.CFURLCreateFromFileSystemRepresentation(
            None, tmp_path.encode('utf-8'), len(tmp_path.encode('utf-8')), False
        )
        dest = Quartz.CGImageDestinationCreateWithURL(url, "public.png", 1, None)
        if dest is None:
            return None
        Quartz.CGImageDestinationAddImage(dest, cg_image, None)
        Quartz.CGImageDestinationFinalize(dest)

        # 读取 PNG 并转换为 PIL Image
        img = Image.open(tmp_path).convert("RGBA")
        img = img.copy()
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)

    return img


def parse_subtlex_csv(filepath):
    """
    解析 SUBTLEX-CH-CHR.csv 频率表

    CSV 格式:
      行1: "Total character count: 46,841,097",,,,,,
      行2: "Context number: 6,243",,,,,,
      行3: Character,CHRCount,CHR/million,...
      行4+: 汉字数据

    Returns:
        list of str: 按频率降序排列的汉字列表
    """
    if not os.path.exists(filepath):
        print(f"  错误: 找不到 SUBTLEX 文件: {filepath}")
        return []

    cjk_chars = []
    with open(filepath, 'r', encoding='utf-8') as f:
        reader = csv.reader(f)
        for i, row in enumerate(reader):
            # 跳过前2行元数据和第3行表头
            if i < 3:
                continue
            if row and len(row) >= 1:
                char = row[0].strip()
                if char and len(char) == 1:
                    cjk_chars.append(char)

    print(f"  解析到 {len(cjk_chars)} 个汉字 (按频率降序)")
    return cjk_chars


def render_chars_at_size(chars, cell_size, render_size, font_size, label=""):
    """
    渲染一批汉字到指定尺寸

    Args:
        chars: 汉字列表
        cell_size: 最终单元格尺寸 (如 16, 32, 64)
        render_size: 渲染尺寸 (通常为 cell_size 的 2 倍以获得更好质量)
        font_size: 字体大小
        label: 用于日志的标签

    Returns:
        list of PIL.Image: 每个汉字的 cell_size × cell_size 图像
    """
    images = []
    total = len(chars)

    for i, char in enumerate(chars):
        img = None

        if HAS_QUARTZ:
            rendered = render_char_quartz(
                char, render_size, render_size,
                CJK_FONT_NAME, font_size
            )
            if rendered:
                img = rendered

        if img is None:
            img = Image.new("RGBA", (cell_size, cell_size), (0, 0, 0, 0))

        # 缩放到目标尺寸
        if img.size != (cell_size, cell_size):
            img = img.resize((cell_size, cell_size), Image.LANCZOS)

        images.append(img)

        if (i + 1) % 1000 == 0 or (i + 1) == total:
            print(f"    {label} {cell_size}px: {i + 1}/{total}")

    return images


def paste_images_to_texture(texture, images, cols, cell_size, start_y):
    """
    将图像列表按网格粘贴到纹理

    Args:
        texture: PIL.Image 纹理
        images: 图像列表
        cols: 每行列数
        cell_size: 单元格尺寸
        start_y: 起始 y 坐标

    Returns:
        int: 下一个可用的 y 坐标
    """
    for i, img in enumerate(images):
        col = i % cols
        row = i // cols
        x = col * cell_size
        y = start_y + row * cell_size
        texture.paste(img, (x, y))

    rows_used = (len(images) + cols - 1) // cols
    return start_y + rows_used * cell_size


def main():
    parser = argparse.ArgumentParser(description='生成多尺寸 CJK 纹理 (cjk.png + cjk64.png)')
    parser.add_argument('--output-cjk', default=OUTPUT_CJK,
                        help=f'输出 cjk.png 路径 (默认: {OUTPUT_CJK})')
    parser.add_argument('--output-cjk64', default=OUTPUT_CJK64,
                        help=f'输出 cjk64.png 路径 (默认: {OUTPUT_CJK64})')
    parser.add_argument('--csv', default=SUBTLEX_CSV,
                        help=f'SUBTLEX CSV 路径 (默认: {SUBTLEX_CSV})')
    args = parser.parse_args()

    print("=" * 70)
    print("生成多尺寸 CJK 纹理")
    print("  Layer 1: cjk.png   (16px + 32px + 64px前段)")
    print("  Layer 2: cjk64.png (64px后段)")
    print("=" * 70)

    # 1. 解析汉字
    print(f"\n解析 {os.path.basename(args.csv)}...")
    cjk_chars = parse_subtlex_csv(args.csv)
    total_chars = len(cjk_chars)

    if total_chars == 0:
        print("错误: 未找到任何汉字")
        sys.exit(1)

    # 2. 计算布局
    cfg16 = SIZE_CONFIG[16]
    cfg32 = SIZE_CONFIG[32]
    cfg64 = SIZE_CONFIG[64]

    rows_16 = (total_chars + cfg16["cols"] - 1) // cfg16["cols"]
    height_16 = rows_16 * cfg16["cell"]

    rows_32 = (total_chars + cfg32["cols"] - 1) // cfg32["cols"]
    height_32 = rows_32 * cfg32["cell"]

    y_start_16 = 0
    y_start_32 = height_16
    y_start_64 = height_16 + height_32

    remaining_height = TEXTURE_SIZE - y_start_64
    rows_64_layer1 = remaining_height // cfg64["cell"]
    chars_64_layer1 = rows_64_layer1 * cfg64["cols"]
    if chars_64_layer1 > total_chars:
        chars_64_layer1 = total_chars
        rows_64_layer1 = (chars_64_layer1 + cfg64["cols"] - 1) // cfg64["cols"]

    chars_64_layer2 = total_chars - chars_64_layer1
    rows_64_layer2 = (chars_64_layer2 + cfg64["cols"] - 1) // cfg64["cols"] if chars_64_layer2 > 0 else 0

    print(f"\n布局计算 ({total_chars} 个汉字):")
    print(f"  cjk.png (Layer 1, {TEXTURE_SIZE}×{TEXTURE_SIZE}):")
    print(f"    16px: y={y_start_16}-{y_start_16 + height_16 - 1}, {rows_16}行×{cfg16['cols']}列, {total_chars}字")
    print(f"    32px: y={y_start_32}-{y_start_32 + height_32 - 1}, {rows_32}行×{cfg32['cols']}列, {total_chars}字")
    print(f"    64px: y={y_start_64}-{y_start_64 + rows_64_layer1 * 64 - 1}, {rows_64_layer1}行×{cfg64['cols']}列, {chars_64_layer1}字")
    print(f"    总高度: {y_start_64 + rows_64_layer1 * 64}px")
    if chars_64_layer2 > 0:
        print(f"  cjk64.png (Layer 2, {TEXTURE_SIZE}×{TEXTURE_SIZE}):")
        print(f"    64px: y=0-{rows_64_layer2 * 64 - 1}, {rows_64_layer2}行×{cfg64['cols']}列, {chars_64_layer2}字")
    else:
        print(f"  cjk64.png: 不需要 (所有64px字符已在Layer 1)")

    # 3. 渲染所有尺寸
    print("\n渲染汉字...")

    if not HAS_QUARTZ:
        print("  错误: Quartz 不可用，无法渲染汉字")
        sys.exit(1)

    print("  渲染 16px 尺寸...")
    imgs_16 = render_chars_at_size(
        cjk_chars, cfg16["cell"], cfg16["render"], cfg16["font"], "Layer1"
    )

    print("  渲染 32px 尺寸...")
    imgs_32 = render_chars_at_size(
        cjk_chars, cfg32["cell"], cfg32["render"], cfg32["font"], "Layer1"
    )

    print("  渲染 64px 尺寸...")
    imgs_64 = render_chars_at_size(
        cjk_chars, cfg64["cell"], cfg64["render"], cfg64["font"], "Layer1+2"
    )

    # 4. 生成 cjk.png
    print(f"\n生成 cjk.png ({TEXTURE_SIZE}×{TEXTURE_SIZE})...")
    tex_cjk = Image.new("RGBA", (TEXTURE_SIZE, TEXTURE_SIZE), (0, 0, 0, 0))

    print("  写入 16px 区域...")
    paste_images_to_texture(tex_cjk, imgs_16, cfg16["cols"], cfg16["cell"], y_start_16)

    print("  写入 32px 区域...")
    paste_images_to_texture(tex_cjk, imgs_32, cfg32["cols"], cfg32["cell"], y_start_32)

    print(f"  写入 64px 前段 ({chars_64_layer1} 字)...")
    paste_images_to_texture(tex_cjk, imgs_64[:chars_64_layer1], cfg64["cols"], cfg64["cell"], y_start_64)

    tex_cjk.save(args.output_cjk, "PNG")
    cjk_size = os.path.getsize(args.output_cjk) / 1024 / 1024
    print(f"  保存: {args.output_cjk} ({cjk_size:.2f} MB)")

    # 5. 生成 cjk64.png
    if chars_64_layer2 > 0:
        print(f"\n生成 cjk64.png ({TEXTURE_SIZE}×{TEXTURE_SIZE})...")
        tex_cjk64 = Image.new("RGBA", (TEXTURE_SIZE, TEXTURE_SIZE), (0, 0, 0, 0))

        print(f"  写入 64px 后段 ({chars_64_layer2} 字)...")
        paste_images_to_texture(tex_cjk64, imgs_64[chars_64_layer1:], cfg64["cols"], cfg64["cell"], 0)

        tex_cjk64.save(args.output_cjk64, "PNG")
        cjk64_size = os.path.getsize(args.output_cjk64) / 1024 / 1024
        print(f"  保存: {args.output_cjk64} ({cjk64_size:.2f} MB)")
    else:
        print("\n不需要生成 cjk64.png")

    # 6. 统计
    print("\n" + "=" * 70)
    print("完成!")
    print("=" * 70)
    print(f"汉字总数: {total_chars} (来源: SUBTLEX-CH-CHR.csv)")
    print(f"\ncjk.png (Texture Array Layer 1):")
    print(f"  16px: {total_chars}字, y={y_start_16}-{y_start_16 + height_16 - 1}")
    print(f"  32px: {total_chars}字, y={y_start_32}-{y_start_32 + height_32 - 1}")
    print(f"  64px: {chars_64_layer1}字, y={y_start_64}-{y_start_64 + rows_64_layer1 * 64 - 1}")
    if chars_64_layer2 > 0:
        print(f"\ncjk64.png (Texture Array Layer 2):")
        print(f"  64px: {chars_64_layer2}字, y=0-{rows_64_layer2 * 64 - 1}")
    print(f"\n覆盖率: 16px=100%, 32px=100%, 64px=100%")


if __name__ == "__main__":
    main()
