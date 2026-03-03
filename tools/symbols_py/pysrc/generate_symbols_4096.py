#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
生成 4096x4096 的 symbols.png
整合 Sprite、TUI、Emoji 和 CJK 符号

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

Block 规格：
- Sprite blocks (0-159):  16×16 chars/block, 16×16px each, 256 chars/block
- TUI blocks (160-169):   16×16 chars/block, 16×32px each, 256 chars/block
- Emoji blocks (170-175): 8×16 chars/block, 32×32px each, 128 chars/block
"""

import os
import sys
import json
from PIL import Image

# ---------------- 配置 ----------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# 输入文件
C64_SOURCES = [
    os.path.join(SCRIPT_DIR, "c64l.png"),
    os.path.join(SCRIPT_DIR, "c64u.png"),
    os.path.join(SCRIPT_DIR, "c64e1.png"),
    os.path.join(SCRIPT_DIR, "c64e2.png"),
]

TUI_CHARS_DIR = os.path.join(SCRIPT_DIR, "tui_chars")
TUI_EMOJIS_DIR = os.path.join(SCRIPT_DIR, "tui_emojis")
TUI_FIX_DIR = os.path.join(SCRIPT_DIR, "tui_fix")

# 输出文件
OUTPUT_PNG = os.path.join(SCRIPT_DIR, "symbols.png")
SYMBOL_MAP_JSON = os.path.join(SCRIPT_DIR, "symbol_map.json")

# 纹理参数
TEXTURE_SIZE = 4096

# Block 参数
SPRITE_BLOCK_SIZE = 256   # 每个 Sprite block: 256×256px
TUI_BLOCK_WIDTH = 256     # TUI block 宽度
TUI_BLOCK_HEIGHT = 512    # TUI block 高度（16 chars × 32px）
EMOJI_BLOCK_WIDTH = 256   # Emoji block 宽度
EMOJI_BLOCK_HEIGHT = 512  # Emoji block 高度（16 rows × 32px）

BLOCKS_PER_ROW = 16  # 每行 16 个 block

# Sprite 区域参数
SPRITE_CHAR_SIZE = 16     # 每个 sprite 16×16px
SPRITE_CHARS_PER_BLOCK = 256  # 16×16 = 256 chars per block
SPRITE_BLOCKS = 160       # Block 0-159
SPRITE_ROWS = 10          # 10 rows of blocks
SPRITE_AREA_HEIGHT = 2560 # 10 rows × 256px

# TUI 区域参数
TUI_CHAR_WIDTH = 16       # TUI 字符宽度
TUI_CHAR_HEIGHT = 32      # TUI 字符高度
TUI_CHARS_PER_BLOCK = 256 # 16×16 = 256 chars per block
TUI_BLOCKS_START = 160    # Block 160-169
TUI_BLOCKS_COUNT = 10     # 10 blocks
TUI_AREA_START_Y = 2560   # TUI 区域起始 Y

# Emoji 区域参数
EMOJI_CHAR_SIZE = 32      # Emoji 32×32px
EMOJI_CHARS_PER_BLOCK = 128  # 8×16 = 128 emojis per block
EMOJI_BLOCKS_START = 170  # Block 170-175
EMOJI_BLOCKS_COUNT = 6    # 6 blocks
EMOJI_AREA_START_X = 2560 # Emoji 区域起始 X
EMOJI_AREA_START_Y = 2560 # Emoji 区域起始 Y

# CJK 区域参数
CJK_CHAR_SIZE = 32        # CJK 字符 32×32px
CJK_AREA_START_Y = 3072   # CJK 区域起始 Y
CJK_GRID_COLS = 128       # 每行 128 个 CJK 字符
CJK_GRID_ROWS = 32        # 32 行

# CJK 渲染参数
CJK_TXT = os.path.join(SCRIPT_DIR, "3500C.txt")
CJK_CHARS_DIR = os.path.join(SCRIPT_DIR, "cjk_chars")
CJK_RENDER_SIZE = 64      # 渲染尺寸（会缩放到 32×32）
CJK_FONT_NAME = "PingFang SC"
CJK_FONT_SIZE = 56

# TUI 输入文件
TUI_TXT = os.path.join(SCRIPT_DIR, "tui.txt")
# ------------------------------------------


def load_c64_block(source_path):
    """
    加载一个 C64 源文件（16×16 个符号，每个 16×16px，间隔 1px）

    Returns:
        list of PIL.Image: 256 个符号图像
    """
    img = Image.open(source_path).convert("RGBA")
    symbols = []

    # C64 源文件格式：16×16 个符号，行列间空白 1px
    for row in range(16):
        for col in range(16):
            # 计算符号位置（考虑 1px 间隔）
            x = col * (SPRITE_CHAR_SIZE + 1)
            y = row * (SPRITE_CHAR_SIZE + 1)

            # 提取符号
            symbol = img.crop((x, y, x + SPRITE_CHAR_SIZE, y + SPRITE_CHAR_SIZE))
            symbols.append(symbol)

    return symbols


def load_tui_chars(tui_dir, count, fix_dir=None):
    """
    加载 TUI 字符（16×32px）

    Args:
        tui_dir: TUI 字符目录
        count: 要加载的字符数量
        fix_dir: 修复目录（可选，优先加载）

    Returns:
        list of PIL.Image: TUI 字符图像
    """
    symbols = []

    for i in range(count):
        symbol = None

        # 首先尝试从修复目录加载
        if fix_dir and os.path.exists(fix_dir):
            files = [f for f in os.listdir(fix_dir) if f.startswith(f"{i:04d}_")]
            if files:
                img_path = os.path.join(fix_dir, files[0])
                symbol = Image.open(img_path).convert("RGBA")

        # 如果修复目录没有，从主目录加载
        if symbol is None:
            files = [f for f in os.listdir(tui_dir) if f.startswith(f"{i:04d}_")]
            if files:
                img_path = os.path.join(tui_dir, files[0])
                symbol = Image.open(img_path).convert("RGBA")

        # 如果都没有找到，创建空白符号
        if symbol is None:
            symbol = Image.new("RGBA", (TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT), (0, 0, 0, 0))
            if i < 270:  # 只警告前面的字符
                print(f"  警告: TUI 字符 {i} 未找到，使用空白")

        # 如果尺寸不对，缩放
        if symbol.size != (TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT):
            symbol = symbol.resize((TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT), Image.LANCZOS)

        symbols.append(symbol)

    return symbols


def load_emojis(emoji_dir, count):
    """
    加载 Emoji（32×32px）

    Returns:
        list of PIL.Image: Emoji 图像
    """
    symbols = []

    for i in range(count):
        # 查找文件：0000_*.png
        files = [f for f in os.listdir(emoji_dir) if f.startswith(f"{i:04d}_")]

        if not files:
            # 创建空白符号
            symbol = Image.new("RGBA", (EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE), (0, 0, 0, 0))
            symbols.append(symbol)
            if i < 270:  # 只警告前面的 Emoji
                print(f"  警告: Emoji {i} 未找到，使用空白")
            continue

        # 加载并缩放到 32×32
        img_path = os.path.join(emoji_dir, files[0])
        img = Image.open(img_path).convert("RGBA")

        # 如果尺寸不对，缩放
        if img.size != (EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE):
            img = img.resize((EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE), Image.LANCZOS)

        symbols.append(img)

    return symbols


def parse_tui_txt(filepath):
    """
    解析 tui.txt 获取 TUI 字符和 Emoji 列表

    Returns:
        (tui_chars, emojis)
        - tui_chars: list of TUI characters
        - emojis: list of emoji strings
    """
    if not os.path.exists(filepath):
        print(f"  警告: TUI 文件不存在: {filepath}")
        return [], []

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
        print("  错误: 未找到空行分隔符")
        return [], []

    tui_lines = lines[start_idx:separator_idx]
    emoji_lines = lines[separator_idx + 1:]

    # 解析 TUI 字符（首位添加空格）
    tui_chars = [' ']
    for line in tui_lines:
        line = line.strip()
        if line:
            for char in line:
                tui_chars.append(char)

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

    print(f"  解析到 {len(tui_chars)} 个 TUI 字符, {len(emojis)} 个 Emoji")
    return tui_chars, emojis


def parse_cjk_txt(filepath):
    """
    解析 CJK 字符文件（每行一个汉字）

    Returns:
        list of str: CJK 字符列表
    """
    if not os.path.exists(filepath):
        print(f"  警告: CJK 文件不存在: {filepath}")
        return []

    with open(filepath, 'r', encoding='utf-8') as f:
        chars = [line.strip() for line in f if line.strip()]

    # 限制数量（最多 4096 个）
    max_chars = CJK_GRID_COLS * CJK_GRID_ROWS
    if len(chars) > max_chars:
        print(f"  警告: CJK 字符数量 {len(chars)} 超过上限 {max_chars}，将截断")
        chars = chars[:max_chars]

    return chars


def render_cjk_chars(cjk_chars, output_dir):
    """
    使用 Quartz 渲染 CJK 字符为 PNG 图像

    Args:
        cjk_chars: CJK 字符列表
        output_dir: 输出目录

    Returns:
        bool: 是否成功
    """
    try:
        import Quartz
        import CoreText
    except ImportError:
        print("  错误: 无法导入 Quartz/CoreText（需要 macOS）")
        return False

    import shutil

    # 清理并创建输出目录
    if os.path.exists(output_dir):
        shutil.rmtree(output_dir)
    os.makedirs(output_dir)

    print(f"  渲染 {len(cjk_chars)} 个 CJK 字符到 {output_dir}...")

    success_count = 0
    for idx, char in enumerate(cjk_chars):
        output_path = os.path.join(output_dir, f"{idx:04d}_{char}.png")

        # 创建位图上下文
        color_space = Quartz.CGColorSpaceCreateDeviceRGB()
        context = Quartz.CGBitmapContextCreate(
            None,
            CJK_RENDER_SIZE, CJK_RENDER_SIZE,
            8,
            CJK_RENDER_SIZE * 4,
            color_space,
            Quartz.kCGImageAlphaPremultipliedLast
        )

        if context is None:
            print(f"    错误: 无法创建上下文 {idx}: {char}")
            continue

        # 清空背景（透明）
        Quartz.CGContextClearRect(context, Quartz.CGRectMake(0, 0, CJK_RENDER_SIZE, CJK_RENDER_SIZE))

        # 设置文本绘制模式
        Quartz.CGContextSetTextDrawingMode(context, Quartz.kCGTextFill)
        Quartz.CGContextSetRGBFillColor(context, 1.0, 1.0, 1.0, 1.0)

        # 创建字体
        font = CoreText.CTFontCreateWithName(CJK_FONT_NAME, CJK_FONT_SIZE, None)

        # 创建属性字符串
        attributes = {
            CoreText.kCTFontAttributeName: font,
            CoreText.kCTForegroundColorFromContextAttributeName: True
        }
        attr_string = CoreText.CFAttributedStringCreate(None, char, attributes)

        # 创建 CTLine
        line = CoreText.CTLineCreateWithAttributedString(attr_string)

        # 获取字形边界以居中
        bounds = CoreText.CTLineGetBoundsWithOptions(line, 0)
        text_width = bounds.size.width
        text_height = bounds.size.height

        # 获取字体度量信息
        ascent = CoreText.CTFontGetAscent(font)
        descent = CoreText.CTFontGetDescent(font)
        leading = CoreText.CTFontGetLeading(font)

        # 计算居中位置
        x = (CJK_RENDER_SIZE - text_width) / 2.0
        font_height = ascent + descent + leading
        baseline_y = (CJK_RENDER_SIZE - font_height) / 2.0 + descent

        # 设置文本位置并绘制
        Quartz.CGContextSetTextPosition(context, x, baseline_y)
        CoreText.CTLineDraw(line, context)

        # 创建 CGImage
        image = Quartz.CGBitmapContextCreateImage(context)
        if image is None:
            print(f"    错误: 无法创建图像 {idx}: {char}")
            continue

        # 保存为 PNG
        url = Quartz.CFURLCreateFromFileSystemRepresentation(
            None,
            output_path.encode('utf-8'),
            len(output_path.encode('utf-8')),
            False
        )

        dest = Quartz.CGImageDestinationCreateWithURL(url, "public.png", 1, None)
        if dest is None:
            print(f"    错误: 无法创建图像目标 {idx}: {char}")
            continue

        Quartz.CGImageDestinationAddImage(dest, image, None)
        if Quartz.CGImageDestinationFinalize(dest):
            success_count += 1

        if (idx + 1) % 500 == 0:
            print(f"    已渲染 {idx + 1}/{len(cjk_chars)}")

    print(f"  渲染完成: {success_count}/{len(cjk_chars)}")
    return success_count > 0


def load_cjk_chars(cjk_dir, cjk_chars):
    """
    加载渲染好的 CJK 字符图像

    Args:
        cjk_dir: CJK 字符目录
        cjk_chars: CJK 字符列表

    Returns:
        list of PIL.Image: CJK 字符图像
    """
    symbols = []

    for i, char in enumerate(cjk_chars):
        # 查找文件：0000_字.png
        img_path = os.path.join(cjk_dir, f"{i:04d}_{char}.png")

        if os.path.exists(img_path):
            img = Image.open(img_path).convert("RGBA")
            # 缩放到 32×32
            if img.size != (CJK_CHAR_SIZE, CJK_CHAR_SIZE):
                img = img.resize((CJK_CHAR_SIZE, CJK_CHAR_SIZE), Image.LANCZOS)
            symbols.append(img)
        else:
            # 创建空白符号
            symbols.append(Image.new("RGBA", (CJK_CHAR_SIZE, CJK_CHAR_SIZE), (0, 0, 0, 0)))
            if i < 100:  # 只警告前面的
                print(f"  警告: CJK 字符 {i} ({char}) 未找到")

    return symbols


def build_cjk_mappings(cjk_chars):
    """
    构建 CJK 字符到网格位置的映射

    Args:
        cjk_chars: CJK 字符列表

    Returns:
        dict: {char: [col, row], ...}
    """
    mappings = {}
    for i, char in enumerate(cjk_chars):
        col = i % CJK_GRID_COLS
        row = i // CJK_GRID_COLS
        mappings[char] = [col, row]
    return mappings


def main():
    print("="*70)
    print("生成 4096x4096 symbols.png")
    print("="*70)

    # 检查输入文件
    for src in C64_SOURCES:
        if not os.path.exists(src):
            print(f"错误: 找不到 {src}")
            sys.exit(1)

    if not os.path.exists(TUI_CHARS_DIR):
        print(f"错误: 找不到 {TUI_CHARS_DIR}")
        sys.exit(1)

    if not os.path.exists(TUI_EMOJIS_DIR):
        print(f"错误: 找不到 {TUI_EMOJIS_DIR}")
        sys.exit(1)

    # 解析 tui.txt 获取 TUI 字符和 Emoji（用于 symbol_map.json）
    print("\n解析 tui.txt...")
    tui_chars_list, emoji_list = parse_tui_txt(TUI_TXT)

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
        print(f"    加载了 {len(sprites)} 个符号")

    print(f"  总共 {len(all_sprites)} 个 Sprite 符号")

    # ========== 加载 TUI 字符 ==========
    print("\n加载 TUI 字符...")
    tui_total = TUI_BLOCKS_COUNT * TUI_CHARS_PER_BLOCK  # 10 blocks × 256 = 2560
    tui_chars = load_tui_chars(TUI_CHARS_DIR, tui_total, TUI_FIX_DIR)
    print(f"  加载了 {len(tui_chars)} 个 TUI 字符")

    # ========== 加载 Emoji ==========
    print("\n加载 Emoji...")
    emoji_total = EMOJI_BLOCKS_COUNT * EMOJI_CHARS_PER_BLOCK  # 6 blocks × 128 = 768
    emojis = load_emojis(TUI_EMOJIS_DIR, emoji_total)
    print(f"  加载了 {len(emojis)} 个 Emoji")

    # ========== 加载/渲染 CJK 字符 ==========
    print("\n处理 CJK 字符...")
    cjk_chars = parse_cjk_txt(CJK_TXT)
    print(f"  解析到 {len(cjk_chars)} 个 CJK 字符")

    cjk_images = []
    if cjk_chars:
        # 检查是否已有渲染好的字符
        if os.path.exists(CJK_CHARS_DIR):
            existing_files = len([f for f in os.listdir(CJK_CHARS_DIR) if f.endswith('.png')])
            if existing_files >= len(cjk_chars):
                print(f"  使用已渲染的 CJK 字符 ({existing_files} 个)")
            else:
                print(f"  已有 {existing_files} 个，需要重新渲染...")
                render_cjk_chars(cjk_chars, CJK_CHARS_DIR)
        else:
            render_cjk_chars(cjk_chars, CJK_CHARS_DIR)

        # 加载 CJK 字符图像
        cjk_images = load_cjk_chars(CJK_CHARS_DIR, cjk_chars)
        print(f"  加载了 {len(cjk_images)} 个 CJK 字符图像")

    # ========== 绘制 Sprite 区域（Block 0-159）==========
    print(f"\n绘制 Sprite 区域（Block 0-{SPRITE_BLOCKS-1}）...")
    sprite_idx = 0

    for block_idx in range(SPRITE_BLOCKS):
        if sprite_idx >= len(all_sprites):
            # 如果没有更多 sprite，使用空白填充
            break

        # 计算 block 位置（10 rows × 16 blocks/row）
        block_row = block_idx // BLOCKS_PER_ROW
        block_col = block_idx % BLOCKS_PER_ROW
        block_x = block_col * SPRITE_BLOCK_SIZE
        block_y = block_row * SPRITE_BLOCK_SIZE

        # 在 block 内绘制 16×16 个 sprite（每个 16×16px）
        for row in range(16):
            for col in range(16):
                if sprite_idx >= len(all_sprites):
                    break

                x = block_x + col * SPRITE_CHAR_SIZE
                y = block_y + row * SPRITE_CHAR_SIZE

                texture.paste(all_sprites[sprite_idx], (x, y))
                sprite_idx += 1

        if (block_idx + 1) % 16 == 0:
            print(f"  已绘制 {block_idx + 1}/{SPRITE_BLOCKS} blocks ({sprite_idx} sprites)")

    print(f"  绘制了 {sprite_idx} 个 Sprite 符号（占用 {(sprite_idx + 255) // 256} 个 block）")

    # ========== 绘制 TUI 区域（Block 160-169）==========
    print(f"\n绘制 TUI 区域（Block {TUI_BLOCKS_START}-{TUI_BLOCKS_START + TUI_BLOCKS_COUNT - 1}）...")
    tui_idx = 0

    for block_idx in range(TUI_BLOCKS_COUNT):
        if tui_idx >= len(tui_chars):
            break

        # 计算 block 位置（在 y=2560 开始，x=0-2559）
        block_x = block_idx * TUI_BLOCK_WIDTH
        block_y = TUI_AREA_START_Y

        # 在 block 内绘制 16×16 个 TUI 字符（每个 16×32px）
        for row in range(16):
            for col in range(16):
                if tui_idx >= len(tui_chars):
                    break

                x = block_x + col * TUI_CHAR_WIDTH
                y = block_y + row * TUI_CHAR_HEIGHT

                texture.paste(tui_chars[tui_idx], (x, y))
                tui_idx += 1

        print(f"  已绘制 Block {TUI_BLOCKS_START + block_idx} ({tui_idx} 字符)")

    print(f"  绘制了 {tui_idx} 个 TUI 字符")

    # ========== 绘制 Emoji 区域（Block 170-175）==========
    print(f"\n绘制 Emoji 区域（Block {EMOJI_BLOCKS_START}-{EMOJI_BLOCKS_START + EMOJI_BLOCKS_COUNT - 1}）...")
    emoji_idx = 0

    for block_idx in range(EMOJI_BLOCKS_COUNT):
        if emoji_idx >= len(emojis):
            break

        # 计算 block 位置（在 y=2560 开始，x=2560 开始）
        block_x = EMOJI_AREA_START_X + block_idx * EMOJI_BLOCK_WIDTH
        block_y = EMOJI_AREA_START_Y

        # 在 block 内绘制 8×16 个 Emoji（每个 32×32px）
        for row in range(16):
            for col in range(8):
                if emoji_idx >= len(emojis):
                    break

                x = block_x + col * EMOJI_CHAR_SIZE
                y = block_y + row * EMOJI_CHAR_SIZE

                texture.paste(emojis[emoji_idx], (x, y))
                emoji_idx += 1

        print(f"  已绘制 Block {EMOJI_BLOCKS_START + block_idx} ({emoji_idx} Emoji)")

    print(f"  绘制了 {emoji_idx} 个 Emoji")

    # ========== 绘制 CJK 区域 ==========
    print(f"\n绘制 CJK 区域（y={CJK_AREA_START_Y}-{TEXTURE_SIZE-1}）...")
    cjk_idx = 0

    for i, img in enumerate(cjk_images):
        col = i % CJK_GRID_COLS
        row = i // CJK_GRID_COLS

        x = col * CJK_CHAR_SIZE
        y = CJK_AREA_START_Y + row * CJK_CHAR_SIZE

        texture.paste(img, (x, y))
        cjk_idx += 1

        if (i + 1) % 500 == 0:
            print(f"  已绘制 {i + 1}/{len(cjk_images)}")

    print(f"  绘制了 {cjk_idx} 个 CJK 字符")

    # ========== 保存纹理 ==========
    print(f"\n保存纹理到 {OUTPUT_PNG}...")
    texture.save(OUTPUT_PNG, "PNG")

    # ========== 生成 symbol_map.json ==========
    print(f"\n生成 {SYMBOL_MAP_JSON}...")

    # 构建 CJK 映射
    cjk_mappings = build_cjk_mappings(cjk_chars) if cjk_chars else {}

    symbol_map = {
        "version": 1,
        "texture_size": TEXTURE_SIZE,
        "regions": {
            "sprite": {
                "type": "block",
                "block_range": [0, SPRITE_BLOCKS - 1],
                "char_size": [SPRITE_CHAR_SIZE, SPRITE_CHAR_SIZE],
                "chars_per_block": SPRITE_CHARS_PER_BLOCK,
                "symbols": "@abcdefghijklmnopqrstuvwxyz[£]↑← !\"#$%&'()*+,-./0123456789:;<=>?─ABCDEFGHIJKLMNOPQRSTUVWXYZ┼",
                "extras": {
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
            },
            "tui": {
                "type": "block",
                "block_range": [TUI_BLOCKS_START, TUI_BLOCKS_START + TUI_BLOCKS_COUNT - 1],
                "char_size": [TUI_CHAR_WIDTH, TUI_CHAR_HEIGHT],
                "chars_per_block": TUI_CHARS_PER_BLOCK,
                "symbols": ''.join(tui_chars_list)  # 从 tui.txt 动态读取
            },
            "emoji": {
                "type": "block",
                "block_range": [EMOJI_BLOCKS_START, EMOJI_BLOCKS_START + EMOJI_BLOCKS_COUNT - 1],
                "char_size": [EMOJI_CHAR_SIZE, EMOJI_CHAR_SIZE],
                "chars_per_block": EMOJI_CHARS_PER_BLOCK,
                "symbols": emoji_list  # 从 tui.txt 动态读取
            },
            "cjk": {
                "type": "grid",
                "pixel_region": [0, CJK_AREA_START_Y, TEXTURE_SIZE, TEXTURE_SIZE - CJK_AREA_START_Y],
                "char_size": [CJK_CHAR_SIZE, CJK_CHAR_SIZE],
                "grid_cols": CJK_GRID_COLS,
                "mappings": cjk_mappings
            }
        },
        "linear_index": {
            "sprite_base": 0,
            "sprite_total": SPRITE_BLOCKS * SPRITE_CHARS_PER_BLOCK,
            "tui_base": SPRITE_BLOCKS * SPRITE_CHARS_PER_BLOCK,
            "tui_total": TUI_BLOCKS_COUNT * TUI_CHARS_PER_BLOCK,
            "emoji_base": SPRITE_BLOCKS * SPRITE_CHARS_PER_BLOCK + TUI_BLOCKS_COUNT * TUI_CHARS_PER_BLOCK,
            "emoji_total": EMOJI_BLOCKS_COUNT * EMOJI_CHARS_PER_BLOCK,
            "cjk_base": SPRITE_BLOCKS * SPRITE_CHARS_PER_BLOCK + TUI_BLOCKS_COUNT * TUI_CHARS_PER_BLOCK + EMOJI_BLOCKS_COUNT * EMOJI_CHARS_PER_BLOCK,
            "cjk_total": len(cjk_chars) if cjk_chars else CJK_GRID_COLS * CJK_GRID_ROWS
        }
    }

    with open(SYMBOL_MAP_JSON, 'w', encoding='utf-8') as f:
        json.dump(symbol_map, f, ensure_ascii=False, indent=2)

    print(f"  已保存 symbol_map.json (包含 {len(cjk_mappings)} 个 CJK 映射)")

    # ========== 统计 ==========
    print("\n" + "="*70)
    print("完成！")
    print("="*70)
    print(f"纹理尺寸: {TEXTURE_SIZE}×{TEXTURE_SIZE}")
    print(f"\n区域布局:")
    print(f"  Sprite 区域 (y=0-{SPRITE_AREA_HEIGHT-1}):")
    print(f"    - Block 0-{SPRITE_BLOCKS-1} ({SPRITE_BLOCKS} blocks, {SPRITE_ROWS} rows × {BLOCKS_PER_ROW} cols)")
    print(f"    - 每 block: 256×256px (16×16 chars, 16×16px each)")
    print(f"    - 已绘制: {sprite_idx} sprites")
    print(f"  TUI 区域 (y={TUI_AREA_START_Y}-{TUI_AREA_START_Y + TUI_BLOCK_HEIGHT - 1}, x=0-{TUI_BLOCKS_COUNT * TUI_BLOCK_WIDTH - 1}):")
    print(f"    - Block {TUI_BLOCKS_START}-{TUI_BLOCKS_START + TUI_BLOCKS_COUNT - 1} ({TUI_BLOCKS_COUNT} blocks)")
    print(f"    - 每 block: 256×512px (16×16 chars, 16×32px each)")
    print(f"    - 已绘制: {tui_idx} chars")
    print(f"  Emoji 区域 (y={EMOJI_AREA_START_Y}-{EMOJI_AREA_START_Y + EMOJI_BLOCK_HEIGHT - 1}, x={EMOJI_AREA_START_X}-{TEXTURE_SIZE - 1}):")
    print(f"    - Block {EMOJI_BLOCKS_START}-{EMOJI_BLOCKS_START + EMOJI_BLOCKS_COUNT - 1} ({EMOJI_BLOCKS_COUNT} blocks)")
    print(f"    - 每 block: 256×512px (8×16 emojis, 32×32px each)")
    print(f"    - 已绘制: {emoji_idx} emojis")
    print(f"  CJK 区域 (y={CJK_AREA_START_Y}-{TEXTURE_SIZE-1}):")
    print(f"    - Grid: {CJK_GRID_COLS}×{CJK_GRID_ROWS} (32×32px each)")
    print(f"    - 已绘制: {cjk_idx} chars")
    print(f"\n输出文件:")
    print(f"  - {OUTPUT_PNG}")
    print(f"  - {SYMBOL_MAP_JSON}")
    file_size = os.path.getsize(OUTPUT_PNG)
    print(f"文件大小: {file_size / 1024 / 1024:.2f} MB")


if __name__ == "__main__":
    main()
