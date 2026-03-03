#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
符号纹理生成工具（SDF 模式）

使用 macOS Quartz/CoreText 渲染字符，转换为 SDF（Signed Distance Field）纹理。
适用于 RustPixel 引擎的 GPU 渲染模式。

输入:
  - c64*.png: Sprite 源图像（PETSCII 字符集）
  - tui.txt: TUI 字符和 Emoji 定义
  - 3500C.txt: CJK 常用汉字（可选）

输出:
  - symbols.png: 4096x4096 或 8192x8192 纹理图
  - symbol_map.json: 符号映射配置

用法:
  python3 generate_symbols.py                    # 默认 8192x8192
  python3 generate_symbols.py --size 4096        # 4096x4096
  python3 generate_symbols.py --save-bitmap      # 保存渲染前的位图（调试用）

纹理布局:
  Sprite: Block 0-159 (40,960 个, 16×16px 或 32×32px)
  TUI:    Block 160-169 (2,560 个, 16×32px 或 32×64px)
  Emoji:  Block 170-175 (768 个, 32×32px 或 64×64px, 位图模式)
  CJK:    128×32 grid (4,096 个, 32×32px 或 64×64px)
"""

import os
import sys
import json
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

# 输入文件
C64_SOURCES = [
    os.path.join(SCRIPT_DIR, "c64l.png"),
    os.path.join(SCRIPT_DIR, "c64u.png"),
    os.path.join(SCRIPT_DIR, "c64e1.png"),
    os.path.join(SCRIPT_DIR, "c64e2.png"),
]
TUI_TXT = os.path.join(SCRIPT_DIR, "tui.txt")
CJK_TXT = os.path.join(SCRIPT_DIR, "3500C.txt")

# 输出文件
OUTPUT_PNG = os.path.join(SCRIPT_DIR, "symbols.png")
OUTPUT_JSON = os.path.join(SCRIPT_DIR, "symbol_map.json")


# ------------------------------------------
# 纹理尺寸配置类
# ------------------------------------------
class TextureConfig:
    """纹理配置，支持 4096 和 8192 两种尺寸"""

    def __init__(self, size=4096):
        if size not in (4096, 8192):
            raise ValueError(f"不支持的纹理尺寸: {size}，只支持 4096 或 8192")

        self.size = size
        self.scale = size // 4096  # 1 for 4096, 2 for 8192

        # 网格常量（不随尺寸变化）
        self.BLOCKS_PER_ROW = 16
        self.SPRITE_CHARS_PER_BLOCK = 256
        self.SPRITE_BLOCKS = 160
        self.SPRITE_ROWS = 10
        self.TUI_CHARS_PER_BLOCK = 256
        self.TUI_BLOCKS_START = 160
        self.TUI_BLOCKS_COUNT = 10
        self.EMOJI_CHARS_PER_BLOCK = 128
        self.EMOJI_BLOCKS_START = 170
        self.EMOJI_BLOCKS_COUNT = 6
        self.CJK_GRID_COLS = 128
        self.CJK_GRID_ROWS = 32

        # 线性索引基址（不随尺寸变化）
        self.LINEAR_SPRITE_BASE = 0
        self.LINEAR_TUI_BASE = 40960
        self.LINEAR_EMOJI_BASE = 43520
        self.LINEAR_CJK_BASE = 44288

        # 像素尺寸（随 scale 缩放）
        self._init_pixel_sizes()

    def _init_pixel_sizes(self):
        """初始化像素尺寸（基于 scale 缩放）"""
        s = self.scale

        # Sprite 区域参数
        self.SPRITE_BLOCK_SIZE = 256 * s
        self.SPRITE_CHAR_SIZE = 16 * s
        self.SPRITE_AREA_HEIGHT = 2560 * s

        # TUI 区域参数
        self.TUI_BLOCK_WIDTH = 256 * s
        self.TUI_BLOCK_HEIGHT = 512 * s
        self.TUI_CHAR_WIDTH = 16 * s
        self.TUI_CHAR_HEIGHT = 32 * s
        self.TUI_AREA_START_Y = 2560 * s

        # Emoji 区域参数
        self.EMOJI_BLOCK_WIDTH = 256 * s
        self.EMOJI_BLOCK_HEIGHT = 512 * s
        self.EMOJI_CHAR_SIZE = 32 * s
        self.EMOJI_AREA_START_X = 2560 * s
        self.EMOJI_AREA_START_Y = 2560 * s

        # CJK 区域参数
        self.CJK_CHAR_SIZE = 32 * s
        self.CJK_AREA_START_Y = 3072 * s

        # 渲染参数（渲染尺寸可以更大，然后缩放到目标尺寸）
        # 对于 8192，使用更大的渲染尺寸以获得更好的质量
        self.TUI_RENDER_WIDTH = 40 * s
        self.TUI_RENDER_HEIGHT = 80 * s
        self.TUI_FONT_SIZE = 64 * s

        self.EMOJI_RENDER_SIZE = 64 * s
        self.EMOJI_FONT_SIZE = 64 * s

        self.CJK_RENDER_SIZE = 64 * s
        self.CJK_FONT_SIZE = 56 * s

    def __repr__(self):
        return f"TextureConfig(size={self.size}, scale={self.scale})"


# 字体配置
EMOJI_FONT_NAME = "Apple Color Emoji"
CJK_FONT_NAME = "PingFang SC"

# TUI 字体文件路径（用于 SDF 渲染）
# TUI_FONT_PATH = os.path.expanduser("~/Library/Fonts/DejaVuSansMNerdFont-Regular.ttf")
TUI_FONT_PATH = os.path.expanduser("~/Library/Fonts/DroidSansMNerdFontMono-Regular.otf")

# 全局配置实例（在 main() 中初始化）
cfg = None
# ------------------------------------------


def parse_tui_txt(filepath):
    """
    解析 tui.txt

    Returns:
        (tui_chars, emojis)
        - tui_chars: list of TUI characters
        - emojis: list of emoji strings
    """
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # 跳过开头的空行
    start_idx = 0
    while start_idx < len(lines) and lines[start_idx].strip() == '':
        start_idx += 1

    # 找到分隔 TUI 和 Emoji 的空行
    separator_idx = -1
    for i in range(start_idx, len(lines)):
        if lines[i].strip() == '':
            separator_idx = i
            break

    if separator_idx == -1:
        print("错误: 未找到空行分隔符")
        return [], []

    tui_lines = lines[start_idx:separator_idx]
    emoji_lines = lines[separator_idx + 1:]

    # 解析 TUI 字符（第一个位置强制为空格）
    tui_chars = [' ']
    for line in tui_lines:
        line = line.strip()
        if line:
            for char in line:
                tui_chars.append(char)

    print(f"  解析到 {len(tui_chars)} 个 TUI 字符")

    # 解析 Emoji
    emojis = []
    for line in emoji_lines:
        line = line.strip()
        if not line:
            continue

        i = 0
        while i < len(line):
            char = line[i]
            code = ord(char)

            # 检查是否是 emoji 起始字符
            is_emoji_start = (
                (0x1F000 <= code <= 0x1FFFF) or
                (0x2600 <= code <= 0x27BF) or
                (0x2300 <= code <= 0x23FF) or
                (0x2B00 <= code <= 0x2BFF) or
                char in '⭐⚡☔⛳⛵⚓⛱⛰⛲⏰✏✅✌❤❎❌⚫⚪⬛⬜'
            )

            if is_emoji_start:
                emoji = char
                # 检查变体选择器 U+FE0F
                if i + 1 < len(line) and ord(line[i + 1]) == 0xFE0F:
                    emoji += line[i + 1]
                    i += 2
                else:
                    i += 1
                emojis.append(emoji)
            else:
                i += 1

    print(f"  解析到 {len(emojis)} 个 Emoji")

    return tui_chars, emojis


def bitmap_to_sdf(bitmap_img, spread=6):
    """
    将位图 RGBA 图像转换为 SDF（Signed Distance Field）

    从 alpha 通道提取形状，计算每个像素到最近边缘的有符号距离，
    归一化后存入 RGB 通道。Shader 用 median3(r,g,b) 解码时，
    三通道相同 → 等价于单通道 SDF，效果接近 MSDF。

    使用 scipy.ndimage.distance_transform_edt（C 实现），极快。

    Args:
        bitmap_img: PIL.Image RGBA 位图
        spread: 距离场扩展范围（像素），对应 shader 的 pxrange

    Returns:
        PIL.Image: RGBA 图像（RGB 存 SDF，A=255）
    """
    import numpy as np
    from scipy.ndimage import distance_transform_edt

    arr = np.array(bitmap_img)
    alpha = arr[:, :, 3].astype(np.float32) / 255.0
    h, w = alpha.shape

    # 二值化：alpha > 0.5 为 "内部"
    inside = alpha > 0.5

    # scipy EDT：输入 True 的位置距离为 0，False 的位置计算到最近 True 的距离
    dist_outside = distance_transform_edt(~inside)  # 外部像素到最近内部像素的距离
    dist_inside = distance_transform_edt(inside)     # 内部像素到最近外部像素的距离

    # 有符号距离：内部为正，外部为负
    sdf = np.where(inside, dist_inside, -dist_outside)

    # 归一化到 [0, 1]：0.5 = 边缘，1.0 = spread 像素内部，0.0 = spread 像素外部
    sdf = sdf / spread * 0.5 + 0.5
    sdf = np.clip(sdf, 0.0, 1.0)

    # 存入 RGB（三通道相同 = 单通道 SDF，median3 返回同一值）
    sdf_u8 = (sdf * 255).astype(np.uint8)
    result = np.zeros((h, w, 4), dtype=np.uint8)
    result[:, :, 0] = sdf_u8
    result[:, :, 1] = sdf_u8
    result[:, :, 2] = sdf_u8
    result[:, :, 3] = 255

    return Image.fromarray(result, 'RGBA')


def is_graphic_char(ch):
    """
    判断字符是否为图形字符（需要填满格子以正确拼接）

    包括: Box Drawing, Block Elements, Braille, Powerline/NerdFont
    """
    cp = ord(ch)
    return (
        (0x2500 <= cp <= 0x257F) or  # Box Drawing
        (0x2580 <= cp <= 0x259F) or  # Block Elements
        (0x2800 <= cp <= 0x28FF) or  # Braille Patterns
        (cp >= 0xE000)               # Private Use / NerdFont / Powerline
    )


# ========== CoreText 辅助函数 ==========

def cfurl_from_path(path):
    """创建 CFURL"""
    b = path.encode("utf-8")
    return Quartz.CFURLCreateFromFileSystemRepresentation(None, b, len(b), False)


def ctfont_from_file(font_path, size, font_name=None):
    """
    从字体文件路径加载 CTFont

    Args:
        font_path: 字体文件路径（.ttf/.otf/.ttc）
        size: 字体大小
        font_name: 字体名称（用于 .ttc 文件选择或回退）
    """
    url = cfurl_from_path(font_path)
    descs = CoreText.CTFontManagerCreateFontDescriptorsFromURL(url)

    if descs and len(descs) > 0:
        # 对于 .ttc 文件，尝试找到匹配的字体
        if font_name and len(descs) > 1:
            for desc in descs:
                name = CoreText.CTFontDescriptorCopyAttribute(desc, CoreText.kCTFontDisplayNameAttribute)
                if name and font_name in str(name):
                    return CoreText.CTFontCreateWithFontDescriptor(desc, float(size), None)
        # 默认使用第一个
        return CoreText.CTFontCreateWithFontDescriptor(descs[0], float(size), None)

    # 回退到名称加载
    if font_name:
        return CoreText.CTFontCreateWithName(font_name, float(size), None)

    raise RuntimeError(f"Failed to create font from: {font_path}")


def ct_line_for_char(ctfont, ch):
    """创建单字符的 CTLine"""
    attrs = {
        CoreText.kCTFontAttributeName: ctfont,
        CoreText.kCTForegroundColorFromContextAttributeName: True,
    }
    s = CoreText.CFAttributedStringCreate(None, ch, attrs)
    return CoreText.CTLineCreateWithAttributedString(s)


def ct_line_ink_bounds(line):
    """获取字形的 ink bounds（实际绘制边界）"""
    return CoreText.CTLineGetBoundsWithOptions(line, CoreText.kCTLineBoundsUseGlyphPathBounds)


def solve_font_size_for_height(font_path, target_h, padding=0.92):
    """二分法：找到 font_size 使 ascent+descent+leading ≈ target_h * padding"""
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


def apply_width_constraint(font_path, size, cell_w, margin=0.98):
    """检查最宽字符，如果超出宽度限制就缩小 font_size"""
    f = ctfont_from_file(font_path, size)
    worst = 0.0

    # 等宽字体中最宽的字符
    test_chars = "W@M#%&QG"
    for ch in test_chars:
        line = ct_line_for_char(f, ch)
        r = ct_line_ink_bounds(line)
        w = float(r.size.width)
        if w > worst:
            worst = w

    limit = cell_w * margin
    if worst <= limit:
        return size

    return size * (limit / worst)


# 缓存已计算的 font_size（避免重复二分）
_font_size_cache = {}


def get_quartz_font_size(font_path, width, height, padding):
    """获取适合目标尺寸的 font_size（带缓存）"""
    cache_key = (font_path, width, height, padding)
    if cache_key in _font_size_cache:
        return _font_size_cache[cache_key]

    # 1. 基于高度计算
    size_h = solve_font_size_for_height(font_path, height, padding)
    # 2. 宽度约束
    size = apply_width_constraint(font_path, size_h, width)

    _font_size_cache[cache_key] = size
    return size


def render_char_quartz(char, width, height, font_name, font_size, fill_cell=False, text_padding=0.92, font_path=None):
    """
    使用 macOS Quartz 渲染单个字符为 PIL Image

    两种模式：
    1. 有 font_path：使用 dgpt.py 的逻辑（二分法 + 宽度约束 + ink bounds 居中）
    2. 无 font_path：使用旧逻辑（font_name + 简单居中）

    Args:
        char: 要渲染的字符
        width, height: 输出尺寸
        font_name: 字体名称（无 font_path 时使用）
        font_size: 基础字体大小（无 font_path 时使用）
        fill_cell: 是否填满格子
        text_padding: 文本字符的缩放系数
        font_path: 字体文件路径（如果提供，使用新逻辑）

    Returns:
        PIL.Image: RGBA 图像
    """
    import tempfile

    # 计算实际 padding
    padding = 1.0 if fill_cell else text_padding

    # 根据是否有 font_path 选择不同的逻辑
    if font_path and os.path.exists(font_path):
        if fill_cell:
            # 图形字符：只基于高度计算 font_size，不应用宽度约束
            # 这样可以确保方块、制表符等尽可能大
            actual_font_size = solve_font_size_for_height(font_path, height, padding)
        else:
            # 普通文本字符：二分法 + 宽度约束
            actual_font_size = get_quartz_font_size(font_path, width, height, padding)
        font = ctfont_from_file(font_path, actual_font_size)
    else:
        # 旧逻辑：使用传入的 font_size，用名称加载字体
        actual_font_size = font_size * padding
        font = CoreText.CTFontCreateWithName(font_name, actual_font_size, None)

    ascent = float(CoreText.CTFontGetAscent(font))
    descent = float(CoreText.CTFontGetDescent(font))

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

    # 创建 CTLine
    line = ct_line_for_char(font, char)

    # 根据是否有 font_path 和 fill_cell 选择居中方式
    if fill_cell:
        # 图形字符（制表符、方块等）：使用 typographic bounds 居中
        # 这些字符需要固定位置以便正确拼接
        typo_width = CoreText.CTLineGetTypographicBounds(line, None, None, None)
        if isinstance(typo_width, tuple):
            typo_width = typo_width[0]
        x = (width - typo_width) / 2.0
    elif font_path and os.path.exists(font_path):
        # 普通文本字符：使用 ink bounds 居中（视觉居中）
        ink = ct_line_ink_bounds(line)
        x = (width - float(ink.size.width)) / 2.0 - float(ink.origin.x)
    else:
        # 旧逻辑：使用 typographic bounds 居中
        typo_width = CoreText.CTLineGetTypographicBounds(line, None, None, None)
        if isinstance(typo_width, tuple):
            typo_width = typo_width[0]
        x = (width - typo_width) / 2.0

    # 垂直居中
    baseline_y = (height - (ascent + descent)) / 2.0 + descent

    # 像素对齐
    x = round(x)
    baseline_y = round(baseline_y)

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


def load_c64_block(source_path):
    """
    加载一个 C64 源文件（16×16 个符号，源文件固定为 16×16px 间隔 1px）

    Returns:
        list of PIL.Image: 256 个符号图像（缩放到目标尺寸）
    """
    img = Image.open(source_path).convert("RGBA")
    symbols = []

    # 源文件固定为 16×16px + 1px 间隔
    SRC_CHAR_SIZE = 16

    for row in range(16):
        for col in range(16):
            x = col * (SRC_CHAR_SIZE + 1)
            y = row * (SRC_CHAR_SIZE + 1)
            symbol = img.crop((x, y, x + SRC_CHAR_SIZE, y + SRC_CHAR_SIZE))

            # 如果目标尺寸不同，进行缩放
            if cfg.SPRITE_CHAR_SIZE != SRC_CHAR_SIZE:
                symbol = symbol.resize(
                    (cfg.SPRITE_CHAR_SIZE, cfg.SPRITE_CHAR_SIZE),
                    Image.LANCZOS
                )

            symbols.append(symbol)

    return symbols


def render_tui_chars(tui_chars, pxrange=4, text_padding=0.92, save_bitmap=False):
    """
    渲染 TUI 字符（使用 Quartz bitmap → SDF）

    Args:
        tui_chars: TUI 字符列表
        pxrange: SDF 距离场像素范围
        text_padding: 文本字符缩放系数（图形字符始终为 1.0）
        save_bitmap: 是否保存渲染前的位图（用于调试）

    Returns:
        list of PIL.Image: TUI 字符图像
    """
    if not HAS_QUARTZ:
        print("错误: SDF 渲染需要 Quartz（仅 macOS）")
        return []

    symbols = []
    total = cfg.TUI_BLOCKS_COUNT * cfg.TUI_CHARS_PER_BLOCK  # 2560

    # 超采样倍率（2x 足够获得良好的 SDF 质量）
    sdf_scale = 2

    # 创建位图缓存目录
    bitmap_cache_dir = os.path.join(SCRIPT_DIR, "tui_bitmap_cache")
    if save_bitmap:
        os.makedirs(bitmap_cache_dir, exist_ok=True)
        print(f"    位图缓存目录: {bitmap_cache_dir}")

    for i in range(total):
        symbol = None

        if i < len(tui_chars):
            char = tui_chars[i]
            cp = ord(char)

            # 渲染字符
            render_w = cfg.TUI_RENDER_WIDTH * sdf_scale
            render_h = cfg.TUI_RENDER_HEIGHT * sdf_scale

            # 图形字符需要填满格子
            fill_cell = is_graphic_char(char)
            rendered = render_char_quartz(
                char, render_w, render_h,
                font_name=None, font_size=None,
                fill_cell=fill_cell, text_padding=text_padding,
                font_path=TUI_FONT_PATH
            )

            if rendered:
                # 保存位图到缓存目录
                if save_bitmap:
                    safe_char = char if char.isprintable() and char not in '/\\:*?"<>|' else f"U{cp:04X}"
                    bitmap_path = os.path.join(bitmap_cache_dir, f"{i:04d}_{safe_char}.png")
                    rendered.save(bitmap_path)

                # 转换为 SDF
                sdf_img = bitmap_to_sdf(rendered, spread=pxrange * sdf_scale)
                symbol = sdf_img.resize(
                    (cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT), Image.LANCZOS
                )

        # 空白填充（SDF 模式使用黑色背景）
        if symbol is None:
            symbol = Image.new("RGBA", (cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT), (0, 0, 0, 255))

        if symbol.size != (cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT):
            symbol = symbol.resize((cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT), Image.LANCZOS)

        symbols.append(symbol)

        if (i + 1) % 256 == 0:
            print(f"    渲染 TUI: {i + 1}/{total}")

    return symbols


def render_emojis(emojis):
    """
    渲染 Emoji（位图模式，彩色 Emoji 不适合 SDF）

    Args:
        emojis: Emoji 列表

    Returns:
        list of PIL.Image: Emoji 图像
    """
    if not HAS_QUARTZ:
        print("警告: Quartz 不可用，无法渲染 Emoji")
        return []

    symbols = []
    total = cfg.EMOJI_BLOCKS_COUNT * cfg.EMOJI_CHARS_PER_BLOCK  # 768

    for i in range(total):
        symbol = None

        if i < len(emojis):
            emoji = emojis[i]
            rendered = render_char_quartz(
                emoji, cfg.EMOJI_RENDER_SIZE, cfg.EMOJI_RENDER_SIZE,
                EMOJI_FONT_NAME, cfg.EMOJI_FONT_SIZE
            )
            if rendered:
                symbol = rendered

        # 如果没有，创建空白
        if symbol is None:
            symbol = Image.new("RGBA", (cfg.EMOJI_CHAR_SIZE, cfg.EMOJI_CHAR_SIZE), (0, 0, 0, 0))

        # 缩放到目标尺寸
        if symbol.size != (cfg.EMOJI_CHAR_SIZE, cfg.EMOJI_CHAR_SIZE):
            symbol = symbol.resize((cfg.EMOJI_CHAR_SIZE, cfg.EMOJI_CHAR_SIZE), Image.LANCZOS)

        symbols.append(symbol)

        if (i + 1) % 128 == 0:
            print(f"    渲染 Emoji: {i + 1}/{total}")

    return symbols


def parse_cjk_txt(filepath):
    """
    解析 CJK 汉字文件 (3500C.txt)

    Returns:
        list of str: 汉字列表
    """
    if not os.path.exists(filepath):
        print(f"  警告: CJK 文件不存在: {filepath}")
        return []

    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    cjk_chars = []
    for line in lines:
        line = line.strip()
        if line:
            # 每行一个汉字
            cjk_chars.append(line)

    print(f"  解析到 {len(cjk_chars)} 个 CJK 汉字")
    return cjk_chars


def render_cjk_chars(cjk_chars, pxrange=4, text_padding=0.92):
    """
    渲染 CJK 汉字（使用 Quartz bitmap → SDF）

    Args:
        cjk_chars: 汉字列表
        pxrange: SDF 距离场像素范围
        text_padding: 文本字符缩放系数

    Returns:
        list of PIL.Image: CJK 汉字图像
    """
    if not HAS_QUARTZ:
        print("错误: SDF 渲染需要 Quartz（仅 macOS）")
        return []

    symbols = []
    total = cfg.CJK_GRID_COLS * cfg.CJK_GRID_ROWS  # 4096

    # 超采样倍率
    sdf_scale = 2

    for i in range(total):
        symbol = None

        if i < len(cjk_chars):
            char = cjk_chars[i]

            # 渲染字符
            render_size = cfg.CJK_RENDER_SIZE * sdf_scale
            font_size = cfg.CJK_FONT_SIZE * sdf_scale
            rendered = render_char_quartz(
                char, render_size, render_size,
                CJK_FONT_NAME, font_size,
                text_padding=text_padding
            )

            if rendered:
                # 转换为 SDF
                sdf_img = bitmap_to_sdf(rendered, spread=pxrange * sdf_scale)
                symbol = sdf_img.resize(
                    (cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE), Image.LANCZOS
                )

        # 空白填充（SDF 模式使用黑色背景）
        if symbol is None:
            symbol = Image.new("RGBA", (cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE), (0, 0, 0, 255))

        if symbol.size != (cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE):
            symbol = symbol.resize((cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE), Image.LANCZOS)

        symbols.append(symbol)

        if (i + 1) % 512 == 0:
            print(f"    渲染 CJK: {i + 1}/{total}")

    return symbols


def build_cjk_mappings(cjk_chars):
    """
    构建 CJK 汉字映射表

    每个汉字映射到其在网格中的 (col, row) 位置
    网格为 128 列 × 32 行（与纹理尺寸无关）

    Returns:
        dict: {字符: [col, row], ...}
    """
    mappings = {}
    for i, char in enumerate(cjk_chars):
        col = i % cfg.CJK_GRID_COLS
        row = i // cfg.CJK_GRID_COLS
        mappings[char] = [col, row]
    return mappings


def build_symbol_map(tui_chars, emojis, cjk_chars=None):
    """
    构建 symbol_map.json 内容

    Returns:
        dict: symbol_map 配置
    """
    # 构建 TUI symbols 字符串（过滤掉 Powerline 私有区域字符）
    tui_symbols = ""
    for char in tui_chars:
        # 保留所有字符，包括 Powerline 符号
        tui_symbols += char

    # 构建 Sprite extras（特殊字符映射）
    # 格式: "字符": [block, idx] 其中 block 0-159，idx 0-255
    sprite_extras = {
        "▇": [1, 209],
        "▒": [1, 94],
        "∙": [1, 122],
        "│": [1, 93],
        "┐": [1, 110],
        "╮": [1, 73],
        "┌": [1, 112],
        "╭": [1, 85],
        "└": [1, 109],
        "╰": [1, 74],
        "┘": [1, 125],
        "╯": [1, 75],
        "_": [2, 30]   # 下划线在 block 2 的第 30 个位置
    }

    # 构建 Sprite symbols（C64 字符集）
    sprite_symbols = "@abcdefghijklmnopqrstuvwxyz[£]↑← !\"#$%&'()*+,-./0123456789:;<=>?─ABCDEFGHIJKLMNOPQRSTUVWXYZ┼"

    # Note: texture_size and char_size are NOT included because they are
    # computed dynamically in Rust based on actual loaded texture dimensions.
    # See src/render/symbol_map.rs layout module for the calculation logic:
    #   - sprite: 1x1 base units
    #   - tui: 1x2 base units (double height)
    #   - emoji: 2x2 base units
    #   - cjk: 2x2 base units
    symbol_map = {
        "version": 1,
        "regions": {
            "sprite": {
                "type": "block",
                "block_range": [0, cfg.SPRITE_BLOCKS - 1],
                "chars_per_block": cfg.SPRITE_CHARS_PER_BLOCK,
                "symbols": sprite_symbols,
                "extras": sprite_extras
            },
            "tui": {
                "type": "block",
                "block_range": [cfg.TUI_BLOCKS_START, cfg.TUI_BLOCKS_START + cfg.TUI_BLOCKS_COUNT - 1],
                "chars_per_block": cfg.TUI_CHARS_PER_BLOCK,
                "symbols": tui_symbols
            },
            "emoji": {
                "type": "block",
                "block_range": [cfg.EMOJI_BLOCKS_START, cfg.EMOJI_BLOCKS_START + cfg.EMOJI_BLOCKS_COUNT - 1],
                "chars_per_block": cfg.EMOJI_CHARS_PER_BLOCK,
                "symbols": emojis
            },
            "cjk": {
                "type": "grid",
                "grid_cols": cfg.CJK_GRID_COLS,
                "mappings": build_cjk_mappings(cjk_chars) if cjk_chars else {}
            }
        },
        "linear_index": {
            "sprite_base": cfg.LINEAR_SPRITE_BASE,
            "sprite_total": cfg.SPRITE_BLOCKS * cfg.SPRITE_CHARS_PER_BLOCK,
            "tui_base": cfg.LINEAR_TUI_BASE,
            "tui_total": cfg.TUI_BLOCKS_COUNT * cfg.TUI_CHARS_PER_BLOCK,
            "emoji_base": cfg.LINEAR_EMOJI_BASE,
            "emoji_total": cfg.EMOJI_BLOCKS_COUNT * cfg.EMOJI_CHARS_PER_BLOCK,
            "cjk_base": cfg.LINEAR_CJK_BASE,
            "cjk_total": cfg.CJK_GRID_COLS * cfg.CJK_GRID_ROWS
        }
    }

    return symbol_map


def main():
    global cfg

    parser = argparse.ArgumentParser(description='生成 symbols.png 和 symbol_map.json（SDF 模式）')
    parser.add_argument('--size', type=int, default=8192, choices=[4096, 8192],
                        help='纹理尺寸: 4096 或 8192 (默认)')
    parser.add_argument('--output-png', default=None,
                        help='输出 PNG 文件路径 (默认: symbols.png 或 symbols_8192.png)')
    parser.add_argument('--output-json', default=None,
                        help='输出 JSON 文件路径 (默认: symbol_map.json 或 symbol_map_8192.json)')
    parser.add_argument('--pxrange', type=int, default=4,
                        help='SDF 距离场像素范围 (默认: 4)')
    parser.add_argument('--text-padding', type=float, default=0.92,
                        help='文本字符缩放系数 (0~1, 默认: 0.92, 图形字符始终为 1.0)')
    parser.add_argument('--save-bitmap', action='store_true',
                        help='保存 SDF 渲染前的位图到缓存目录（用于调试）')
    args = parser.parse_args()

    # 检查 Quartz 是否可用
    if not HAS_QUARTZ:
        print("错误: SDF 渲染需要 Quartz（仅 macOS）")
        sys.exit(1)

    # 初始化配置
    cfg = TextureConfig(args.size)

    # 设置默认输出文件名
    if args.output_png is None:
        if args.size == 8192:
            args.output_png = os.path.join(SCRIPT_DIR, "symbols_8192.png")
        else:
            args.output_png = OUTPUT_PNG

    if args.output_json is None:
        if args.size == 8192:
            args.output_json = os.path.join(SCRIPT_DIR, "symbol_map_8192.json")
        else:
            args.output_json = OUTPUT_JSON

    # 显示配置信息
    print("\n准备 SDF 渲染...")
    print(f"  TUI 字体: {os.path.basename(TUI_FONT_PATH)}")
    print(f"  CJK 字体: {CJK_FONT_NAME}")
    print(f"  pxrange: {args.pxrange}")

    print("\n" + "=" * 70)
    print(f"生成 {cfg.size}x{cfg.size} symbols.png 和 symbol_map.json")
    if cfg.scale > 1:
        print(f"  缩放因子: {cfg.scale}x (基础符号: {cfg.SPRITE_CHAR_SIZE}x{cfg.SPRITE_CHAR_SIZE}px)")
    print(f"  模式: SDF (Quartz bitmap-to-SDF)")
    print("=" * 70)

    # 检查输入文件
    print("\n检查输入文件...")
    for src in C64_SOURCES:
        if not os.path.exists(src):
            print(f"错误: 找不到 {src}")
            sys.exit(1)
        print(f"  ✓ {os.path.basename(src)}")

    if not os.path.exists(TUI_TXT):
        print(f"错误: 找不到 {TUI_TXT}")
        sys.exit(1)
    print(f"  ✓ {os.path.basename(TUI_TXT)}")

    if os.path.exists(CJK_TXT):
        print(f"  ✓ {os.path.basename(CJK_TXT)}")
    else:
        print(f"  ⚠ {os.path.basename(CJK_TXT)} (可选，未找到)")

    # 解析 tui.txt
    print(f"\n解析 {os.path.basename(TUI_TXT)}...")
    tui_chars, emojis = parse_tui_txt(TUI_TXT)

    if len(tui_chars) == 0 and len(emojis) == 0:
        print("错误: 未找到任何字符")
        sys.exit(1)

    # 解析 CJK 汉字
    print(f"\n解析 {os.path.basename(CJK_TXT)}...")
    cjk_chars = parse_cjk_txt(CJK_TXT)

    # 创建空白纹理
    print(f"\n创建 {cfg.size}×{cfg.size} 纹理...")
    texture = Image.new("RGBA", (cfg.size, cfg.size), (0, 0, 0, 0))

    # ========== 加载 Sprite 符号 ==========
    print("\n加载 Sprite 符号...")
    all_sprites = []
    for i, src in enumerate(C64_SOURCES):
        print(f"  加载 {os.path.basename(src)}...")
        sprites = load_c64_block(src)
        all_sprites.extend(sprites)
    print(f"  总共 {len(all_sprites)} 个 Sprite 符号")

    # ========== 渲染 TUI 字符 ==========
    print(f"\n渲染 TUI 字符 (SDF, pxrange={args.pxrange})...")
    tui_images = render_tui_chars(
        tui_chars,
        pxrange=args.pxrange,
        text_padding=args.text_padding,
        save_bitmap=args.save_bitmap
    )
    print(f"  生成 {len(tui_images)} 个 TUI 字符图像")

    # ========== 渲染 Emoji ==========
    print("\n渲染 Emoji (位图)...")
    emoji_images = render_emojis(emojis)
    print(f"  生成 {len(emoji_images)} 个 Emoji 图像")

    # ========== 渲染 CJK 汉字 ==========
    print(f"\n渲染 CJK 汉字 (SDF, pxrange={args.pxrange})...")
    cjk_images = render_cjk_chars(
        cjk_chars,
        pxrange=args.pxrange,
        text_padding=args.text_padding
    )
    print(f"  生成 {len(cjk_images)} 个 CJK 图像")

    # ========== 绘制 Sprite 区域 ==========
    print(f"\n绘制 Sprite 区域 (Block 0-{cfg.SPRITE_BLOCKS-1})...")
    sprite_idx = 0

    for block_idx in range(cfg.SPRITE_BLOCKS):
        if sprite_idx >= len(all_sprites):
            break

        block_row = block_idx // cfg.BLOCKS_PER_ROW
        block_col = block_idx % cfg.BLOCKS_PER_ROW
        block_x = block_col * cfg.SPRITE_BLOCK_SIZE
        block_y = block_row * cfg.SPRITE_BLOCK_SIZE

        for row in range(16):
            for col in range(16):
                if sprite_idx >= len(all_sprites):
                    break

                x = block_x + col * cfg.SPRITE_CHAR_SIZE
                y = block_y + row * cfg.SPRITE_CHAR_SIZE

                texture.paste(all_sprites[sprite_idx], (x, y))
                sprite_idx += 1

        if (block_idx + 1) % 16 == 0:
            print(f"  已绘制 {block_idx + 1}/{cfg.SPRITE_BLOCKS} blocks")

    print(f"  绘制了 {sprite_idx} 个 Sprite")

    # ========== 绘制 TUI 区域 ==========
    print(f"\n绘制 TUI 区域 (Block {cfg.TUI_BLOCKS_START}-{cfg.TUI_BLOCKS_START + cfg.TUI_BLOCKS_COUNT - 1})...")
    tui_idx = 0

    for block_idx in range(cfg.TUI_BLOCKS_COUNT):
        if tui_idx >= len(tui_images):
            break

        block_x = block_idx * cfg.TUI_BLOCK_WIDTH
        block_y = cfg.TUI_AREA_START_Y

        for row in range(16):
            for col in range(16):
                if tui_idx >= len(tui_images):
                    break

                x = block_x + col * cfg.TUI_CHAR_WIDTH
                y = block_y + row * cfg.TUI_CHAR_HEIGHT

                texture.paste(tui_images[tui_idx], (x, y))
                tui_idx += 1

        print(f"  已绘制 Block {cfg.TUI_BLOCKS_START + block_idx}")

    print(f"  绘制了 {tui_idx} 个 TUI 字符")

    # ========== 绘制 Emoji 区域 ==========
    print(f"\n绘制 Emoji 区域 (Block {cfg.EMOJI_BLOCKS_START}-{cfg.EMOJI_BLOCKS_START + cfg.EMOJI_BLOCKS_COUNT - 1})...")
    emoji_idx = 0

    for block_idx in range(cfg.EMOJI_BLOCKS_COUNT):
        if emoji_idx >= len(emoji_images):
            break

        block_x = cfg.EMOJI_AREA_START_X + block_idx * cfg.EMOJI_BLOCK_WIDTH
        block_y = cfg.EMOJI_AREA_START_Y

        for row in range(16):
            for col in range(8):
                if emoji_idx >= len(emoji_images):
                    break

                x = block_x + col * cfg.EMOJI_CHAR_SIZE
                y = block_y + row * cfg.EMOJI_CHAR_SIZE

                texture.paste(emoji_images[emoji_idx], (x, y))
                emoji_idx += 1

        print(f"  已绘制 Block {cfg.EMOJI_BLOCKS_START + block_idx}")

    print(f"  绘制了 {emoji_idx} 个 Emoji")

    # ========== 绘制 CJK 区域 ==========
    print(f"\n绘制 CJK 区域 (y={cfg.CJK_AREA_START_Y}-{cfg.size - 1})...")
    cjk_idx = 0

    for i, img in enumerate(cjk_images):
        if img is None:
            continue

        col = i % cfg.CJK_GRID_COLS
        row = i // cfg.CJK_GRID_COLS

        x = col * cfg.CJK_CHAR_SIZE
        y = cfg.CJK_AREA_START_Y + row * cfg.CJK_CHAR_SIZE

        texture.paste(img, (x, y))
        cjk_idx += 1

        if (i + 1) % 512 == 0:
            print(f"  已绘制 {i + 1}/{len(cjk_images)}")

    print(f"  绘制了 {cjk_idx} 个 CJK 字符")

    # ========== 保存纹理 ==========
    print(f"\n保存纹理到 {args.output_png}...")
    texture.save(args.output_png, "PNG")

    # ========== 生成 symbol_map.json ==========
    print(f"\n生成 {args.output_json}...")
    symbol_map = build_symbol_map(tui_chars, emojis, cjk_chars)

    with open(args.output_json, 'w', encoding='utf-8') as f:
        json.dump(symbol_map, f, ensure_ascii=False, indent=2)

    # ========== 统计 ==========
    print("\n" + "=" * 70)
    print("完成!")
    print("=" * 70)
    print(f"纹理尺寸: {cfg.size}×{cfg.size} (缩放因子: {cfg.scale}x)")
    print(f"基础符号尺寸: {cfg.SPRITE_CHAR_SIZE}×{cfg.SPRITE_CHAR_SIZE}px")
    print(f"\n区域布局:")
    print(f"  Sprite (Block 0-{cfg.SPRITE_BLOCKS-1}): {sprite_idx} 个 ({cfg.SPRITE_CHAR_SIZE}×{cfg.SPRITE_CHAR_SIZE}px)")
    print(f"  TUI (Block {cfg.TUI_BLOCKS_START}-{cfg.TUI_BLOCKS_START + cfg.TUI_BLOCKS_COUNT - 1}): {tui_idx} 个 ({cfg.TUI_CHAR_WIDTH}×{cfg.TUI_CHAR_HEIGHT}px)")
    print(f"  Emoji (Block {cfg.EMOJI_BLOCKS_START}-{cfg.EMOJI_BLOCKS_START + cfg.EMOJI_BLOCKS_COUNT - 1}): {emoji_idx} 个 ({cfg.EMOJI_CHAR_SIZE}×{cfg.EMOJI_CHAR_SIZE}px)")
    print(f"  CJK (y={cfg.CJK_AREA_START_Y}-{cfg.size-1}): {cjk_idx} 个 ({cfg.CJK_CHAR_SIZE}×{cfg.CJK_CHAR_SIZE}px, 容量 {cfg.CJK_GRID_COLS * cfg.CJK_GRID_ROWS})")
    print(f"\n线性索引范围 (与尺寸无关):")
    print(f"  Sprite: [{cfg.LINEAR_SPRITE_BASE}, {cfg.LINEAR_TUI_BASE - 1}]")
    print(f"  TUI:    [{cfg.LINEAR_TUI_BASE}, {cfg.LINEAR_EMOJI_BASE - 1}]")
    print(f"  Emoji:  [{cfg.LINEAR_EMOJI_BASE}, {cfg.LINEAR_CJK_BASE - 1}]")
    print(f"  CJK:    [{cfg.LINEAR_CJK_BASE}, {cfg.LINEAR_CJK_BASE + cfg.CJK_GRID_COLS * cfg.CJK_GRID_ROWS - 1}]")
    print(f"\n输出文件:")
    print(f"  {args.output_png} ({os.path.getsize(args.output_png) / 1024 / 1024:.2f} MB)")
    print(f"  {args.output_json}")


if __name__ == "__main__":
    main()
