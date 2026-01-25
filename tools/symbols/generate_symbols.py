#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
统一的符号纹理生成工具

输入:
  - c64*.png: Sprite 源图像
  - tui.txt: TUI 字符和 Emoji 定义

输出:
  - symbols.png: 4096x4096 纹理图
  - symbol_map.json: 符号映射配置

4096x4096 纹理布局（Block-Based）：
┌────────────────────────────────────────────────────────────┐
│ Sprite 区域（y=0-2559, 2560px 高）                         │
│ - 10 rows × 16 blocks/row = 160 blocks                     │
│ - 每 block: 256×256px (16×16 chars, 16×16px each)          │
│ - Block 0-159: 40,960 sprites                              │
├────────────────────────────────────────────────────────────┤
│ TUI + Emoji 区域（y=2560-3071, 512px 高）                  │
│                                                            │
│ TUI 区域（x=0-2559）:                                      │
│ - 10 blocks (Block 160-169)                                │
│ - 每 block: 256×512px (16×16 chars, 16×32px each)          │
│ - 2560 TUI 字符                                            │
│                                                            │
│ Emoji 区域（x=2560-4095）:                                 │
│ - 6 blocks (Block 170-175)                                 │
│ - 每 block: 256×512px (8×16 emojis, 32×32px each)          │
│ - 768 Emoji                                                │
├────────────────────────────────────────────────────────────┤
│ CJK 区域（y=3072-4095, 1024px 高）                         │
│ - 128×32 grid of 32×32px chars                             │
│ - 4096 CJK 字符                                            │
└────────────────────────────────────────────────────────────┘

线性索引:
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

# 纹理参数
TEXTURE_SIZE = 4096
BLOCKS_PER_ROW = 16

# Sprite 区域参数
SPRITE_BLOCK_SIZE = 256
SPRITE_CHAR_SIZE = 16
SPRITE_CHARS_PER_BLOCK = 256
SPRITE_BLOCKS = 160
SPRITE_ROWS = 10
SPRITE_AREA_HEIGHT = 2560

# TUI 区域参数
TUI_BLOCK_WIDTH = 256
TUI_BLOCK_HEIGHT = 512
TUI_CHAR_WIDTH = 16
TUI_CHAR_HEIGHT = 32
TUI_CHARS_PER_BLOCK = 256
TUI_BLOCKS_START = 160
TUI_BLOCKS_COUNT = 10
TUI_AREA_START_Y = 2560

# TUI 渲染参数
TUI_RENDER_WIDTH = 40
TUI_RENDER_HEIGHT = 80
TUI_FONT_NAME = "DroidSansMono Nerd Font"
TUI_FONT_SIZE = 64

# Emoji 区域参数
EMOJI_BLOCK_WIDTH = 256
EMOJI_BLOCK_HEIGHT = 512
EMOJI_CHAR_SIZE = 32
EMOJI_CHARS_PER_BLOCK = 128
EMOJI_BLOCKS_START = 170
EMOJI_BLOCKS_COUNT = 6
EMOJI_AREA_START_X = 2560
EMOJI_AREA_START_Y = 2560

# Emoji 渲染参数
EMOJI_RENDER_SIZE = 64
EMOJI_FONT_NAME = "Apple Color Emoji"
EMOJI_FONT_SIZE = 64

# CJK 区域参数
CJK_CHAR_SIZE = 32
CJK_AREA_START_Y = 3072
CJK_GRID_COLS = 128
CJK_GRID_ROWS = 32

# CJK 渲染参数
CJK_RENDER_SIZE = 64
CJK_FONT_NAME = "DroidSansMono Nerd Font"
# CJK_FONT_NAME = "PingFang SC"  # macOS 内置中文字体
CJK_FONT_SIZE = 56

# 线性索引基址
LINEAR_SPRITE_BASE = 0
LINEAR_TUI_BASE = 40960
LINEAR_EMOJI_BASE = 43520
LINEAR_CJK_BASE = 44288
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
    加载一个 C64 源文件（16×16 个符号，每个 16×16px，间隔 1px）

    Returns:
        list of PIL.Image: 256 个符号图像
    """
    img = Image.open(source_path).convert("RGBA")
    symbols = []

    for row in range(16):
        for col in range(16):
            x = col * (SPRITE_CHAR_SIZE + 1)
            y = row * (SPRITE_CHAR_SIZE + 1)
            symbol = img.crop((x, y, x + SPRITE_CHAR_SIZE, y + SPRITE_CHAR_SIZE))
            symbols.append(symbol)

    return symbols


def render_tui_chars(tui_chars, use_cache=False):
    """
    渲染 TUI 字符

    Args:
        tui_chars: TUI 字符列表
        use_cache: 是否使用缓存目录中的图像

    Returns:
        list of PIL.Image: TUI 字符图像（16×32px）
    """
    symbols = []
    total = TUI_BLOCKS_COUNT * TUI_CHARS_PER_BLOCK  # 2560

    for i in range(total):
        symbol = None

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

        # 如果有字符定义，直接渲染
        if symbol is None and i < len(tui_chars) and HAS_QUARTZ:
            char = tui_chars[i]
            rendered = render_char_quartz(
                char, TUI_RENDER_WIDTH, TUI_RENDER_HEIGHT,
                TUI_FONT_NAME, TUI_FONT_SIZE
            )
            if rendered:
                symbol = rendered

        # 如果都没有，创建空白
        if symbol is None:
            symbol = Image.new("RGBA", (TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT), (0, 0, 0, 0))

        # 缩放到目标尺寸
        if symbol.size != (TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT):
            symbol = symbol.resize((TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT), Image.LANCZOS)

        symbols.append(symbol)

        if (i + 1) % 256 == 0:
            print(f"    渲染 TUI: {i + 1}/{total}")

    return symbols


def render_emojis(emojis, use_cache=False):
    """
    渲染 Emoji

    Args:
        emojis: Emoji 列表
        use_cache: 是否使用缓存目录中的图像

    Returns:
        list of PIL.Image: Emoji 图像（32×32px）
    """
    symbols = []
    total = EMOJI_BLOCKS_COUNT * EMOJI_CHARS_PER_BLOCK  # 768

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
                emoji, EMOJI_RENDER_SIZE, EMOJI_RENDER_SIZE,
                EMOJI_FONT_NAME, EMOJI_FONT_SIZE
            )
            if rendered:
                symbol = rendered

        # 如果都没有，创建空白
        if symbol is None:
            symbol = Image.new("RGBA", (EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE), (0, 0, 0, 0))

        # 缩放到目标尺寸
        if symbol.size != (EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE):
            symbol = symbol.resize((EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE), Image.LANCZOS)

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


def render_cjk_chars(cjk_chars, use_cache=False):
    """
    渲染 CJK 汉字

    Args:
        cjk_chars: 汉字列表
        use_cache: 是否使用缓存目录中的图像

    Returns:
        list of PIL.Image: CJK 汉字图像（32×32px）
    """
    symbols = []
    total = CJK_GRID_COLS * CJK_GRID_ROWS  # 4096

    cjk_cache_dir = os.path.join(SCRIPT_DIR, "cjk_chars")

    for i in range(total):
        symbol = None

        # 如果使用缓存，从缓存目录加载
        if use_cache and os.path.exists(cjk_cache_dir):
            files = [f for f in os.listdir(cjk_cache_dir) if f.startswith(f"{i:04d}_")]
            if files:
                img_path = os.path.join(cjk_cache_dir, files[0])
                symbol = Image.open(img_path).convert("RGBA")

        # 如果有汉字定义，直接渲染
        if symbol is None and i < len(cjk_chars) and HAS_QUARTZ:
            char = cjk_chars[i]
            rendered = render_char_quartz(
                char, CJK_RENDER_SIZE, CJK_RENDER_SIZE,
                CJK_FONT_NAME, CJK_FONT_SIZE
            )
            if rendered:
                symbol = rendered

        # 如果都没有，创建空白
        if symbol is None:
            symbol = Image.new("RGBA", (CJK_CHAR_SIZE, CJK_CHAR_SIZE), (0, 0, 0, 0))

        # 缩放到目标尺寸
        if symbol.size != (CJK_CHAR_SIZE, CJK_CHAR_SIZE):
            symbol = symbol.resize((CJK_CHAR_SIZE, CJK_CHAR_SIZE), Image.LANCZOS)

        symbols.append(symbol)

        if (i + 1) % 512 == 0:
            print(f"    渲染 CJK: {i + 1}/{total}")

    return symbols


def build_cjk_mappings(cjk_chars):
    """
    构建 CJK 汉字映射表

    每个汉字映射到其在网格中的 (col, row) 位置
    网格为 128 列 × 32 行

    Returns:
        dict: {字符: [col, row], ...}
    """
    mappings = {}
    for i, char in enumerate(cjk_chars):
        col = i % CJK_GRID_COLS
        row = i // CJK_GRID_COLS
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

    symbol_map = {
        "version": 1,
        "texture_size": TEXTURE_SIZE,
        "regions": {
            "sprite": {
                "type": "block",
                "block_range": [0, SPRITE_BLOCKS - 1],
                "char_size": [SPRITE_CHAR_SIZE, SPRITE_CHAR_SIZE],
                "chars_per_block": SPRITE_CHARS_PER_BLOCK,
                "symbols": sprite_symbols,
                "extras": sprite_extras
            },
            "tui": {
                "type": "block",
                "block_range": [TUI_BLOCKS_START, TUI_BLOCKS_START + TUI_BLOCKS_COUNT - 1],
                "char_size": [TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT],
                "chars_per_block": TUI_CHARS_PER_BLOCK,
                "symbols": tui_symbols
            },
            "emoji": {
                "type": "block",
                "block_range": [EMOJI_BLOCKS_START, EMOJI_BLOCKS_START + EMOJI_BLOCKS_COUNT - 1],
                "char_size": [EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE],
                "chars_per_block": EMOJI_CHARS_PER_BLOCK,
                "symbols": emojis
            },
            "cjk": {
                "type": "grid",
                "pixel_region": [0, CJK_AREA_START_Y, TEXTURE_SIZE, TEXTURE_SIZE - CJK_AREA_START_Y],
                "char_size": [CJK_CHAR_SIZE, CJK_CHAR_SIZE],
                "grid_cols": CJK_GRID_COLS,
                "mappings": build_cjk_mappings(cjk_chars) if cjk_chars else {}
            }
        },
        "linear_index": {
            "sprite_base": LINEAR_SPRITE_BASE,
            "sprite_total": SPRITE_BLOCKS * SPRITE_CHARS_PER_BLOCK,
            "tui_base": LINEAR_TUI_BASE,
            "tui_total": TUI_BLOCKS_COUNT * TUI_CHARS_PER_BLOCK,
            "emoji_base": LINEAR_EMOJI_BASE,
            "emoji_total": EMOJI_BLOCKS_COUNT * EMOJI_CHARS_PER_BLOCK,
            "cjk_base": LINEAR_CJK_BASE,
            "cjk_total": CJK_GRID_COLS * CJK_GRID_ROWS
        }
    }

    return symbol_map


def main():
    parser = argparse.ArgumentParser(description='生成 symbols.png 和 symbol_map.json')
    parser.add_argument('--use-cache', action='store_true',
                        help='使用缓存的字符图像而不是重新渲染')
    parser.add_argument('--output-png', default=OUTPUT_PNG,
                        help=f'输出 PNG 文件路径 (默认: {OUTPUT_PNG})')
    parser.add_argument('--output-json', default=OUTPUT_JSON,
                        help=f'输出 JSON 文件路径 (默认: {OUTPUT_JSON})')
    args = parser.parse_args()

    print("=" * 70)
    print("生成 4096x4096 symbols.png 和 symbol_map.json")
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
    print(f"\n创建 {TEXTURE_SIZE}×{TEXTURE_SIZE} 纹理...")
    texture = Image.new("RGBA", (TEXTURE_SIZE, TEXTURE_SIZE), (0, 0, 0, 0))

    # ========== 加载 Sprite 符号 ==========
    print("\n加载 Sprite 符号...")
    all_sprites = []
    for i, src in enumerate(C64_SOURCES):
        print(f"  加载 {os.path.basename(src)}...")
        sprites = load_c64_block(src)
        all_sprites.extend(sprites)
    print(f"  总共 {len(all_sprites)} 个 Sprite 符号")

    # ========== 渲染 TUI 字符 ==========
    print("\n渲染 TUI 字符...")
    if not HAS_QUARTZ and not args.use_cache:
        print("  警告: Quartz 不可用，强制使用缓存")
        args.use_cache = True
    tui_images = render_tui_chars(tui_chars, args.use_cache)
    print(f"  生成 {len(tui_images)} 个 TUI 字符图像")

    # ========== 渲染 Emoji ==========
    print("\n渲染 Emoji...")
    emoji_images = render_emojis(emojis, args.use_cache)
    print(f"  生成 {len(emoji_images)} 个 Emoji 图像")

    # ========== 渲染 CJK 汉字 ==========
    print("\n渲染 CJK 汉字...")
    cjk_images = render_cjk_chars(cjk_chars, args.use_cache)
    print(f"  生成 {len(cjk_images)} 个 CJK 图像")

    # ========== 绘制 Sprite 区域 ==========
    print(f"\n绘制 Sprite 区域 (Block 0-{SPRITE_BLOCKS-1})...")
    sprite_idx = 0

    for block_idx in range(SPRITE_BLOCKS):
        if sprite_idx >= len(all_sprites):
            break

        block_row = block_idx // BLOCKS_PER_ROW
        block_col = block_idx % BLOCKS_PER_ROW
        block_x = block_col * SPRITE_BLOCK_SIZE
        block_y = block_row * SPRITE_BLOCK_SIZE

        for row in range(16):
            for col in range(16):
                if sprite_idx >= len(all_sprites):
                    break

                x = block_x + col * SPRITE_CHAR_SIZE
                y = block_y + row * SPRITE_CHAR_SIZE

                texture.paste(all_sprites[sprite_idx], (x, y))
                sprite_idx += 1

        if (block_idx + 1) % 16 == 0:
            print(f"  已绘制 {block_idx + 1}/{SPRITE_BLOCKS} blocks")

    print(f"  绘制了 {sprite_idx} 个 Sprite")

    # ========== 绘制 TUI 区域 ==========
    print(f"\n绘制 TUI 区域 (Block {TUI_BLOCKS_START}-{TUI_BLOCKS_START + TUI_BLOCKS_COUNT - 1})...")
    tui_idx = 0

    for block_idx in range(TUI_BLOCKS_COUNT):
        if tui_idx >= len(tui_images):
            break

        block_x = block_idx * TUI_BLOCK_WIDTH
        block_y = TUI_AREA_START_Y

        for row in range(16):
            for col in range(16):
                if tui_idx >= len(tui_images):
                    break

                x = block_x + col * TUI_CHAR_WIDTH
                y = block_y + row * TUI_CHAR_HEIGHT

                texture.paste(tui_images[tui_idx], (x, y))
                tui_idx += 1

        print(f"  已绘制 Block {TUI_BLOCKS_START + block_idx}")

    print(f"  绘制了 {tui_idx} 个 TUI 字符")

    # ========== 绘制 Emoji 区域 ==========
    print(f"\n绘制 Emoji 区域 (Block {EMOJI_BLOCKS_START}-{EMOJI_BLOCKS_START + EMOJI_BLOCKS_COUNT - 1})...")
    emoji_idx = 0

    for block_idx in range(EMOJI_BLOCKS_COUNT):
        if emoji_idx >= len(emoji_images):
            break

        block_x = EMOJI_AREA_START_X + block_idx * EMOJI_BLOCK_WIDTH
        block_y = EMOJI_AREA_START_Y

        for row in range(16):
            for col in range(8):
                if emoji_idx >= len(emoji_images):
                    break

                x = block_x + col * EMOJI_CHAR_SIZE
                y = block_y + row * EMOJI_CHAR_SIZE

                texture.paste(emoji_images[emoji_idx], (x, y))
                emoji_idx += 1

        print(f"  已绘制 Block {EMOJI_BLOCKS_START + block_idx}")

    print(f"  绘制了 {emoji_idx} 个 Emoji")

    # ========== 绘制 CJK 区域 ==========
    print(f"\n绘制 CJK 区域 (y={CJK_AREA_START_Y}-{TEXTURE_SIZE - 1})...")
    cjk_idx = 0

    for i, img in enumerate(cjk_images):
        if img is None:
            continue

        col = i % CJK_GRID_COLS
        row = i // CJK_GRID_COLS

        x = col * CJK_CHAR_SIZE
        y = CJK_AREA_START_Y + row * CJK_CHAR_SIZE

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
    print(f"纹理尺寸: {TEXTURE_SIZE}×{TEXTURE_SIZE}")
    print(f"\n区域布局:")
    print(f"  Sprite (Block 0-{SPRITE_BLOCKS-1}): {sprite_idx} 个")
    print(f"  TUI (Block {TUI_BLOCKS_START}-{TUI_BLOCKS_START + TUI_BLOCKS_COUNT - 1}): {tui_idx} 个")
    print(f"  Emoji (Block {EMOJI_BLOCKS_START}-{EMOJI_BLOCKS_START + EMOJI_BLOCKS_COUNT - 1}): {emoji_idx} 个")
    print(f"  CJK (y={CJK_AREA_START_Y}-{TEXTURE_SIZE-1}): {cjk_idx} 个 (容量 {CJK_GRID_COLS * CJK_GRID_ROWS})")
    print(f"\n线性索引范围:")
    print(f"  Sprite: [{LINEAR_SPRITE_BASE}, {LINEAR_TUI_BASE - 1}]")
    print(f"  TUI:    [{LINEAR_TUI_BASE}, {LINEAR_EMOJI_BASE - 1}]")
    print(f"  Emoji:  [{LINEAR_EMOJI_BASE}, {LINEAR_CJK_BASE - 1}]")
    print(f"  CJK:    [{LINEAR_CJK_BASE}, {LINEAR_CJK_BASE + CJK_GRID_COLS * CJK_GRID_ROWS - 1}]")
    print(f"\n输出文件:")
    print(f"  {args.output_png} ({os.path.getsize(args.output_png) / 1024 / 1024:.2f} MB)")
    print(f"  {args.output_json}")


if __name__ == "__main__":
    main()
