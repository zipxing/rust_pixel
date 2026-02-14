#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
统一的符号纹理生成工具

输入:
  - c64*.png: Sprite 源图像
  - tui.txt: TUI 字符和 Emoji 定义

输出:
  - symbols.png: 4096x4096 或 8192x8192 纹理图
  - symbol_map.json: 符号映射配置

支持两种纹理尺寸:
  - 4096x4096 (默认): 16×16px 基础符号
  - 8192x8192 (--size 8192): 32×32px 基础符号，更高清晰度

纹理布局（Block-Based，网格数量不变，像素尺寸按比例缩放）：
┌────────────────────────────────────────────────────────────┐
│ Sprite 区域                                                │
│ - 10 rows × 16 blocks/row = 160 blocks                     │
│ - 每 block: 256×256 grid (16×16 chars)                     │
│ - Block 0-159: 40,960 sprites                              │
├────────────────────────────────────────────────────────────┤
│ TUI + Emoji 区域                                           │
│                                                            │
│ TUI 区域:                                                  │
│ - 10 blocks (Block 160-169)                                │
│ - 每 block: 16×16 chars (1:2 宽高比)                       │
│ - 2560 TUI 字符                                            │
│                                                            │
│ Emoji 区域:                                                │
│ - 6 blocks (Block 170-175)                                 │
│ - 每 block: 8×16 emojis (2x 宽高)                          │
│ - 768 Emoji                                                │
├────────────────────────────────────────────────────────────┤
│ CJK 区域                                                   │
│ - 128×32 grid (2x 宽高)                                    │
│ - 4096 CJK 字符                                            │
└────────────────────────────────────────────────────────────┘

线性索引 (与纹理尺寸无关):
  - Sprite: [0, 40959] = 160 blocks × 256
  - TUI:    [40960, 43519] = 10 blocks × 256
  - Emoji:  [43520, 44287] = 6 blocks × 128
  - CJK:    [44288, 48383] = 128 cols × 32 rows
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

# 缓存目录（可选，用于调试）
TUI_CACHE_DIR = os.path.join(SCRIPT_DIR, "tui_chars")
EMOJI_CACHE_DIR = os.path.join(SCRIPT_DIR, "tui_emojis")
TUI_FIX_DIR = os.path.join(SCRIPT_DIR, "tui_fix")

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


# 字体名称（Quartz 系统字体名，用于位图渲染 fallback）
TUI_FONT_NAME = "DroidSansMono Nerd Font"
EMOJI_FONT_NAME = "Apple Color Emoji"
CJK_FONT_NAME = "DroidSansMono Nerd Font"

# MSDF 默认字体文件路径（TUI 和 CJK 需要不同字体）
# TUI: Nerd Font 包含 Powerline、box-drawing、ASCII 等符号
# CJK: Arial Unicode 包含完整 CJK 汉字
MSDF_TUI_FONT_DEFAULT = os.path.expanduser("~/Library/Fonts/DroidSansMNerdFontMono-Regular.otf")
MSDF_CJK_FONT_DEFAULT = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf"

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


# 字体度量缓存（每个字体文件只读一次）
_font_metrics_cache = {}

def get_font_metrics(font_path):
    """
    读取字体度量信息，用于 MSDF 一致性定位

    Returns:
        dict: {ascent, descent, advance} 均为 EM 归一化值
    """
    if font_path in _font_metrics_cache:
        return _font_metrics_cache[font_path]

    defaults = {'ascent': 0.8, 'descent': -0.2, 'advance': 0.6}
    try:
        from fontTools.ttLib import TTFont
        font = TTFont(font_path)
        upm = font['head'].unitsPerEm
        hhea = font['hhea']
        ascent = hhea.ascent / upm
        descent = hhea.descent / upm  # 负值
        # 从 hmtx 取等宽字体的 advance width
        hmtx = font['hmtx']
        # 用 'A' 或 'space' 作为参考
        for ref_glyph in ['A', 'space', '.notdef']:
            if ref_glyph in hmtx.metrics:
                advance = hmtx.metrics[ref_glyph][0] / upm
                break
        else:
            advance = 0.6
        font.close()
        metrics = {'ascent': ascent, 'descent': descent, 'advance': advance}
    except Exception as e:
        print(f"    ⚠ 无法读取字体度量 ({e})，使用默认值")
        metrics = defaults

    _font_metrics_cache[font_path] = metrics
    print(f"    字体度量: ascent={metrics['ascent']:.3f}, descent={metrics['descent']:.3f}, advance={metrics['advance']:.3f}")
    return metrics


def compute_msdf_layout(metrics, cell_width, cell_height, pxrange):
    """
    根据字体度量计算统一的 MSDF scale 和 translate

    所有字符共享同一套参数，确保大小一致、基线对齐。

    Returns:
        (scale, translate_x, translate_y)
    """
    ascent = metrics['ascent']
    descent = metrics['descent']  # 负值
    advance = metrics['advance']
    total_height = ascent - descent  # 如 0.8 - (-0.2) = 1.0

    # 留出 pxrange 像素的边距（距离场需要过渡空间）
    margin = pxrange
    effective_height = cell_height - 2 * margin

    # scale: 每 EM 多少像素
    scale = effective_height / total_height

    # translate_y: 让 descent 线对齐到下边距
    # 像素坐标: pixel_y = (shape_y + ty) * scale
    # descent 对齐到 margin: margin = (descent + ty) * scale
    translate_y = margin / scale - descent

    # translate_x: 水平居中
    # 单元格中心 = cell_width / 2 像素 = (advance/2 + tx) * scale
    translate_x = cell_width / (2 * scale) - advance / 2

    return scale, translate_x, translate_y


def render_char_msdf(char, width, height, font_path, pxrange=4,
                     msdf_layout=None, msdfgen_path='/tmp/msdfgen'):
    """
    使用 msdfgen 生成单个字符的 MSDF 图像

    Args:
        char: 要渲染的字符
        width: 输出宽度（像素）
        height: 输出高度（像素）
        font_path: TTF/OTF 字体文件路径
        pxrange: MSDF 距离场像素范围
        msdf_layout: (scale, tx, ty) 统一布局参数，None 则用 autoframe
        msdfgen_path: msdfgen 可执行文件路径

    Returns:
        PIL.Image: RGBA 图像（RGB 通道存 MSDF 距离场数据），失败返回 None
    """
    import subprocess
    import tempfile

    codepoint = ord(char)

    with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as tmp:
        tmp_path = tmp.name

    try:
        cmd = [
            msdfgen_path, 'msdf',
            '-font', font_path, str(codepoint),
            '-dimensions', str(width), str(height),
            '-pxrange', str(pxrange),
        ]
        if msdf_layout:
            scale, tx, ty = msdf_layout
            cmd += ['-emnormalize', '-scale', str(scale), '-translate', str(tx), str(ty)]
        else:
            cmd += ['-autoframe']
        cmd += ['-o', tmp_path]

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            return None

        img = Image.open(tmp_path).convert("RGBA")
        img = img.copy()
        return img
    except Exception:
        return None
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


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
        # 复制一份以避免文件被删除后引用问题
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


def render_tui_chars(tui_chars, use_cache=False, use_msdf=False, msdf_font=None, msdf_pxrange=4):
    """
    渲染 TUI 字符

    Args:
        tui_chars: TUI 字符列表
        use_cache: 是否使用缓存目录中的图像
        use_msdf: 是否使用 MSDF 渲染
        msdf_font: MSDF 字体文件路径
        msdf_pxrange: MSDF 距离场像素范围

    Returns:
        list of PIL.Image: TUI 字符图像
    """
    symbols = []
    total = cfg.TUI_BLOCKS_COUNT * cfg.TUI_CHARS_PER_BLOCK  # 2560

    # MSDF 模式：预计算统一布局参数（所有字符共享）
    msdf_layout = None
    if use_msdf and msdf_font:
        metrics = get_font_metrics(msdf_font)
        msdf_layout = compute_msdf_layout(
            metrics, cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT, msdf_pxrange
        )
        s, tx, ty = msdf_layout
        print(f"    TUI MSDF 布局: scale={s:.2f}, translate=({tx:.3f}, {ty:.3f})")

    msdf_fail_count = 0
    for i in range(total):
        symbol = None

        if use_msdf and msdf_font:
            # === MSDF 模式 ===
            if i < len(tui_chars):
                char = tui_chars[i]
                rendered = render_char_msdf(
                    char, cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT,
                    msdf_font, msdf_pxrange, msdf_layout=msdf_layout
                )
                if rendered:
                    symbol = rendered
                else:
                    msdf_fail_count += 1
            # MSDF 模式下不 fallback 到 Quartz（位图数据会被 shader 误解为距离场）
            # 失败时创建空白 MSDF（全黑 RGB = 距离场外部 = 不可见）
            if symbol is None:
                symbol = Image.new("RGBA", (cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT), (0, 0, 0, 255))
        else:
            # === 位图模式 ===
            # 首先尝试从修复目录加载
            if os.path.exists(TUI_FIX_DIR):
                files = [f for f in os.listdir(TUI_FIX_DIR) if f.startswith(f"{i:04d}_")]
                if files:
                    img_path = os.path.join(TUI_FIX_DIR, files[0])
                    symbol = Image.open(img_path).convert("RGBA")

            # 如果使用缓存，从缓存目录加载
            if symbol is None and use_cache and os.path.exists(TUI_CACHE_DIR):
                files = [f for f in os.listdir(TUI_CACHE_DIR) if f.startswith(f"{i:04d}_")]
                if files:
                    img_path = os.path.join(TUI_CACHE_DIR, files[0])
                    symbol = Image.open(img_path).convert("RGBA")

            # 如果有字符定义，Quartz 渲染
            if symbol is None and i < len(tui_chars) and HAS_QUARTZ:
                char = tui_chars[i]
                rendered = render_char_quartz(
                    char, cfg.TUI_RENDER_WIDTH, cfg.TUI_RENDER_HEIGHT,
                    TUI_FONT_NAME, cfg.TUI_FONT_SIZE
                )
                if rendered:
                    symbol = rendered

            # 如果都没有，创建空白
            if symbol is None:
                symbol = Image.new("RGBA", (cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT), (0, 0, 0, 0))

        # 缩放到目标尺寸
        if symbol.size != (cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT):
            symbol = symbol.resize((cfg.TUI_CHAR_WIDTH, cfg.TUI_CHAR_HEIGHT), Image.LANCZOS)

        symbols.append(symbol)

        if (i + 1) % 256 == 0:
            print(f"    渲染 TUI: {i + 1}/{total}")

    if use_msdf and msdf_fail_count > 0:
        print(f"    ⚠ MSDF 渲染失败 {msdf_fail_count} 个字符（已填充空白距离场）")
    return symbols


def render_emojis(emojis, use_cache=False):
    """
    渲染 Emoji

    Args:
        emojis: Emoji 列表
        use_cache: 是否使用缓存目录中的图像

    Returns:
        list of PIL.Image: Emoji 图像
    """
    symbols = []
    total = cfg.EMOJI_BLOCKS_COUNT * cfg.EMOJI_CHARS_PER_BLOCK  # 768

    for i in range(total):
        symbol = None

        # 如果使用缓存，从缓存目录加载
        if use_cache and os.path.exists(EMOJI_CACHE_DIR):
            files = [f for f in os.listdir(EMOJI_CACHE_DIR) if f.startswith(f"{i:04d}_")]
            if files:
                img_path = os.path.join(EMOJI_CACHE_DIR, files[0])
                symbol = Image.open(img_path).convert("RGBA")

        # 如果有 Emoji 定义，直接渲染
        if symbol is None and i < len(emojis) and HAS_QUARTZ:
            emoji = emojis[i]
            rendered = render_char_quartz(
                emoji, cfg.EMOJI_RENDER_SIZE, cfg.EMOJI_RENDER_SIZE,
                EMOJI_FONT_NAME, cfg.EMOJI_FONT_SIZE
            )
            if rendered:
                symbol = rendered

        # 如果都没有，创建空白
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


def render_cjk_chars(cjk_chars, use_cache=False, use_msdf=False, msdf_font=None, msdf_pxrange=4):
    """
    渲染 CJK 汉字

    Args:
        cjk_chars: 汉字列表
        use_cache: 是否使用缓存目录中的图像
        use_msdf: 是否使用 MSDF 渲染
        msdf_font: MSDF 字体文件路径
        msdf_pxrange: MSDF 距离场像素范围

    Returns:
        list of PIL.Image: CJK 汉字图像
    """
    symbols = []
    total = cfg.CJK_GRID_COLS * cfg.CJK_GRID_ROWS  # 4096

    cjk_cache_dir = os.path.join(SCRIPT_DIR, "cjk_chars")

    # MSDF 模式：预计算统一布局参数
    msdf_layout = None
    if use_msdf and msdf_font:
        metrics = get_font_metrics(msdf_font)
        msdf_layout = compute_msdf_layout(
            metrics, cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE, msdf_pxrange
        )
        s, tx, ty = msdf_layout
        print(f"    CJK MSDF 布局: scale={s:.2f}, translate=({tx:.3f}, {ty:.3f})")

    msdf_fail_count = 0

    for i in range(total):
        symbol = None

        if use_msdf and msdf_font:
            # === MSDF 模式 ===
            if i < len(cjk_chars):
                char = cjk_chars[i]
                rendered = render_char_msdf(
                    char, cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE,
                    msdf_font, msdf_pxrange, msdf_layout=msdf_layout
                )
                if rendered:
                    symbol = rendered
                else:
                    msdf_fail_count += 1
            # MSDF 模式下不 fallback 到 Quartz
            if symbol is None:
                symbol = Image.new("RGBA", (cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE), (0, 0, 0, 255))
        else:
            # === 位图模式 ===
            if use_cache and os.path.exists(cjk_cache_dir):
                files = [f for f in os.listdir(cjk_cache_dir) if f.startswith(f"{i:04d}_")]
                if files:
                    img_path = os.path.join(cjk_cache_dir, files[0])
                    symbol = Image.open(img_path).convert("RGBA")

            # 如果有汉字定义，Quartz 渲染
            if symbol is None and i < len(cjk_chars) and HAS_QUARTZ:
                char = cjk_chars[i]
                rendered = render_char_quartz(
                    char, cfg.CJK_RENDER_SIZE, cfg.CJK_RENDER_SIZE,
                    CJK_FONT_NAME, cfg.CJK_FONT_SIZE
                )
                if rendered:
                    symbol = rendered

            # 如果都没有，创建空白
            if symbol is None:
                symbol = Image.new("RGBA", (cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE), (0, 0, 0, 0))

        # 缩放到目标尺寸
        if symbol.size != (cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE):
            symbol = symbol.resize((cfg.CJK_CHAR_SIZE, cfg.CJK_CHAR_SIZE), Image.LANCZOS)

        symbols.append(symbol)

        if (i + 1) % 512 == 0:
            print(f"    渲染 CJK: {i + 1}/{total}")

    if use_msdf and msdf_fail_count > 0:
        print(f"    ⚠ MSDF 渲染失败 {msdf_fail_count} 个字符（已填充空白距离场）")
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
    sprite_extras = {
        "▇": [0, 209],
        "▒": [0, 94],
        "∙": [0, 122],
        "│": [0, 93],
        "┐": [0, 110],
        "╮": [0, 73],
        "┌": [0, 112],
        "╭": [0, 85],
        "└": [0, 109],
        "╰": [0, 74],
        "┘": [0, 125],
        "╯": [0, 75]
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

    parser = argparse.ArgumentParser(description='生成 symbols.png 和 symbol_map.json')
    parser.add_argument('--size', type=int, default=4096, choices=[4096, 8192],
                        help='纹理尺寸: 4096 (默认) 或 8192')
    parser.add_argument('--use-cache', action='store_true',
                        help='使用缓存的字符图像而不是重新渲染')
    parser.add_argument('--output-png', default=None,
                        help=f'输出 PNG 文件路径 (默认: symbols.png 或 symbols_8192.png)')
    parser.add_argument('--output-json', default=None,
                        help=f'输出 JSON 文件路径 (默认: symbol_map.json 或 symbol_map_8192.json)')
    parser.add_argument('--msdf', action='store_true',
                        help='使用 MSDF 渲染 TUI 和 CJK 字符（Sprite 和 Emoji 不变）')
    parser.add_argument('--msdf-font', default=None,
                        help=f'TUI MSDF 字体文件路径 (默认: {MSDF_TUI_FONT_DEFAULT})')
    parser.add_argument('--msdf-cjk-font', default=None,
                        help=f'CJK MSDF 字体文件路径 (默认: {MSDF_CJK_FONT_DEFAULT})')
    parser.add_argument('--msdf-pxrange', type=int, default=4,
                        help='MSDF 距离场像素范围 (默认: 4)')
    args = parser.parse_args()

    # MSDF 模式：使用默认字体路径
    if args.msdf:
        if not args.msdf_font:
            args.msdf_font = MSDF_TUI_FONT_DEFAULT
        if not args.msdf_cjk_font:
            args.msdf_cjk_font = MSDF_CJK_FONT_DEFAULT
        # 校验字体文件存在
        if not os.path.exists(args.msdf_font):
            print(f"错误: TUI MSDF 字体不存在: {args.msdf_font}")
            sys.exit(1)
        if not os.path.exists(args.msdf_cjk_font):
            print(f"错误: CJK MSDF 字体不存在: {args.msdf_cjk_font}")
            sys.exit(1)
        print(f"  MSDF TUI 字体: {args.msdf_font}")
        print(f"  MSDF CJK 字体: {args.msdf_cjk_font}")

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

    print("=" * 70)
    print(f"生成 {cfg.size}x{cfg.size} symbols.png 和 symbol_map.json")
    if cfg.scale > 1:
        print(f"  缩放因子: {cfg.scale}x (基础符号: {cfg.SPRITE_CHAR_SIZE}x{cfg.SPRITE_CHAR_SIZE}px)")
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
    if args.msdf:
        print(f"\n渲染 TUI 字符 (MSDF, pxrange={args.msdf_pxrange})...")
    else:
        print("\n渲染 TUI 字符...")
    if not args.msdf and not HAS_QUARTZ and not args.use_cache:
        print("  警告: Quartz 不可用，强制使用缓存")
        args.use_cache = True
    tui_images = render_tui_chars(tui_chars, args.use_cache,
                                   use_msdf=args.msdf,
                                   msdf_font=args.msdf_font,
                                   msdf_pxrange=args.msdf_pxrange)
    print(f"  生成 {len(tui_images)} 个 TUI 字符图像")

    # ========== 渲染 Emoji ==========
    print("\n渲染 Emoji (位图)...")
    emoji_images = render_emojis(emojis, args.use_cache)
    print(f"  生成 {len(emoji_images)} 个 Emoji 图像")

    # ========== 渲染 CJK 汉字 ==========
    if args.msdf:
        print(f"\n渲染 CJK 汉字 (MSDF, pxrange={args.msdf_pxrange})...")
    else:
        print("\n渲染 CJK 汉字...")
    cjk_msdf_font = args.msdf_cjk_font if args.msdf else None
    cjk_images = render_cjk_chars(cjk_chars, args.use_cache,
                                    use_msdf=args.msdf,
                                    msdf_font=cjk_msdf_font,
                                    msdf_pxrange=args.msdf_pxrange)
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
