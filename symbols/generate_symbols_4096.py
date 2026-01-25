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

    # ========== CJK 区域预留 ==========
    print(f"\nCJK 区域预留（y={CJK_AREA_START_Y}-{TEXTURE_SIZE-1}）...")
    print(f"  预留 {CJK_GRID_COLS}×{CJK_GRID_ROWS} = {CJK_GRID_COLS * CJK_GRID_ROWS} 个 CJK 字符位置")

    # ========== 保存纹理 ==========
    print(f"\n保存纹理到 {OUTPUT_PNG}...")
    texture.save(OUTPUT_PNG, "PNG")

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
    print(f"    - 预留: {CJK_GRID_COLS * CJK_GRID_ROWS} chars")
    print(f"\n输出文件: {OUTPUT_PNG}")
    file_size = os.path.getsize(OUTPUT_PNG)
    print(f"文件大小: {file_size / 1024 / 1024:.2f} MB")


if __name__ == "__main__":
    main()
