#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
生成 2048x2048 的 symbols.png
整合 Sprite、TUI 和 Emoji 符号

布局（基于 design.md，尺寸扩大 2 倍）：
2048x2048 纹理布局（Block-Based）：
┌────────────────────────────────────────┐
│ Sprite 区域（行 0-1535）                │ 1536px 高
│ - 6 rows × 8 blocks/row = 48 blocks   │
│ - 每 block: 16×16 chars, 16×16px each  │
│ - Block 0-47: 12,288 sprites           │
│ - 线性索引：0-12287                     │
├────────────────────────────────────────┤
│ TUI + Emoji 区域（行 1536-2047）        │ 512px 高
│ - 8 blocks horizontally                │
│ - Block 48-51: TUI active (1024 chars) │
│ - Block 52: TUI reserved (256 chars)   │
│ - Block 53-54: Emoji active (256)      │
│ - Block 55: Emoji reserved (128)       │
│ - TUI 线性索引：12288-13567             │
│ - Emoji 线性索引：13568-13951           │
└────────────────────────────────────────┘

Block 规格（扩大 2 倍）：
- Sprite blocks (0-47):  16×16 chars/block, 16×16px each, 256 chars/block
- TUI blocks (48-52):    16×16 chars/block, 16×32px each, 256 chars/block
- Emoji blocks (53-55):  8×16 chars/block, 32×32px each, 128 chars/block
"""

import os
import sys
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

# 输出文件
OUTPUT_PNG = os.path.join(SCRIPT_DIR, "symbols.png")

# 纹理参数
TEXTURE_SIZE = 2048
GRID_SIZE = 16  # 基础网格单元大小（16x16 像素）

# Block 参数
BLOCK_WIDTH = 256  # 每个 block 宽度（16 * 16px）
BLOCK_HEIGHT = 256  # 每个 block 高度（16 * 16px）
BLOCKS_PER_ROW = 8  # 每行 8 个 block

# Sprite 参数（16x16 像素）
SPRITE_SIZE = 16
SPRITE_BLOCKS = 48  # Block 0-47（6 rows × 8 blocks/row）
SPRITE_PER_BLOCK = 16 * 16  # 每个 block 256 个 sprite
TOTAL_SPRITES = SPRITE_BLOCKS * SPRITE_PER_BLOCK  # 12,288 个

# TUI 参数（16x32 像素）
TUI_WIDTH = 16
TUI_HEIGHT = 32
TUI_BLOCKS_START = 48  # Block 48-52
TUI_BLOCKS_ACTIVE = 4  # Block 48-51（active）
TUI_PER_BLOCK = 16 * 16  # 每个 block 256 个字符
TUI_COUNT = TUI_BLOCKS_ACTIVE * TUI_PER_BLOCK  # 1024 个

# Emoji 参数（32x32 像素）
EMOJI_SIZE = 32
EMOJI_BLOCKS_START = 53  # Block 53-54
EMOJI_BLOCKS_ACTIVE = 2  # Block 53-54（active）
EMOJI_PER_BLOCK = 8 * 16  # 每个 block 128 个 emoji
EMOJI_COUNT = EMOJI_BLOCKS_ACTIVE * EMOJI_PER_BLOCK  # 256 个
# ------------------------------------------


def load_c64_block(source_path):
    """
    加载一个 C64 源文件（16x16 个符号，每个 16x16px，间隔 1px）
    
    Returns:
        list of PIL.Image: 256 个符号图像
    """
    img = Image.open(source_path).convert("RGBA")
    symbols = []
    
    # C64 源文件格式：16x16 个符号，行列间空白 1px
    for row in range(16):
        for col in range(16):
            # 计算符号位置（考虑 1px 间隔）
            x = col * (SPRITE_SIZE + 1)
            y = row * (SPRITE_SIZE + 1)
            
            # 提取符号
            symbol = img.crop((x, y, x + SPRITE_SIZE, y + SPRITE_SIZE))
            symbols.append(symbol)
    
    return symbols


def load_tui_chars(tui_dir, count):
    """
    加载 TUI 字符（16x32px）
    
    Returns:
        list of PIL.Image: TUI 字符图像
    """
    symbols = []
    
    for i in range(count):
        # 查找文件：0000_*.png
        files = [f for f in os.listdir(tui_dir) if f.startswith(f"{i:04d}_")]
        
        if not files:
            # 创建空白符号
            symbol = Image.new("RGBA", (TUI_WIDTH, TUI_HEIGHT), (0, 0, 0, 0))
            symbols.append(symbol)
            print(f"  警告: TUI 字符 {i} 未找到，使用空白")
            continue
        
        # 加载并缩放到 16x32
        img_path = os.path.join(tui_dir, files[0])
        img = Image.open(img_path).convert("RGBA")
        
        # 如果尺寸不对，缩放
        if img.size != (TUI_WIDTH, TUI_HEIGHT):
            img = img.resize((TUI_WIDTH, TUI_HEIGHT), Image.LANCZOS)
        
        symbols.append(img)
    
    return symbols


def load_emojis(emoji_dir, count):
    """
    加载 Emoji（32x32px）
    
    Returns:
        list of PIL.Image: Emoji 图像
    """
    symbols = []
    
    for i in range(count):
        # 查找文件：0000_*.png
        files = [f for f in os.listdir(emoji_dir) if f.startswith(f"{i:04d}_")]
        
        if not files:
            # 创建空白符号
            symbol = Image.new("RGBA", (EMOJI_SIZE, EMOJI_SIZE), (0, 0, 0, 0))
            symbols.append(symbol)
            print(f"  警告: Emoji {i} 未找到，使用空白")
            continue
        
        # 加载并缩放到 32x32
        img_path = os.path.join(emoji_dir, files[0])
        img = Image.open(img_path).convert("RGBA")
        
        # 如果尺寸不对，缩放
        if img.size != (EMOJI_SIZE, EMOJI_SIZE):
            img = img.resize((EMOJI_SIZE, EMOJI_SIZE), Image.LANCZOS)
        
        symbols.append(img)
    
    return symbols


def main():
    print("="*60)
    print("生成 2048x2048 symbols.png")
    print("="*60)
    
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
    print(f"\n创建 {TEXTURE_SIZE}x{TEXTURE_SIZE} 纹理...")
    texture = Image.new("RGBA", (TEXTURE_SIZE, TEXTURE_SIZE), (0, 0, 0, 0))
    
    # 加载 Sprite 符号
    print("\n加载 Sprite 符号...")
    all_sprites = []
    for i, src in enumerate(C64_SOURCES):
        print(f"  加载 {os.path.basename(src)}...")
        sprites = load_c64_block(src)
        all_sprites.extend(sprites)
        print(f"    ✓ 加载了 {len(sprites)} 个符号")
    
    print(f"  总共 {len(all_sprites)} 个 Sprite 符号")
    
    # 加载 TUI 字符
    print("\n加载 TUI 字符...")
    tui_chars = load_tui_chars(TUI_CHARS_DIR, TUI_COUNT)
    print(f"  ✓ 加载了 {len(tui_chars)} 个 TUI 字符")
    
    # 加载 Emoji
    print("\n加载 Emoji...")
    emojis = load_emojis(TUI_EMOJIS_DIR, EMOJI_COUNT)
    print(f"  ✓ 加载了 {len(emojis)} 个 Emoji")
    
    # 绘制 Sprite 区域（Block 0-47）
    print("\n绘制 Sprite 区域（Block 0-47）...")
    sprite_idx = 0
    
    for block_idx in range(SPRITE_BLOCKS):
        if sprite_idx >= len(all_sprites):
            break
        
        # 计算 block 位置
        block_row = block_idx // BLOCKS_PER_ROW
        block_col = block_idx % BLOCKS_PER_ROW
        block_x = block_col * BLOCK_WIDTH
        block_y = block_row * BLOCK_HEIGHT
        
        # 在 block 内绘制 16x16 个 sprite（每个 16x16px）
        for row in range(16):
            for col in range(16):
                if sprite_idx >= len(all_sprites):
                    break
                
                x = block_x + col * SPRITE_SIZE
                y = block_y + row * SPRITE_SIZE
                
                texture.paste(all_sprites[sprite_idx], (x, y))
                sprite_idx += 1
        
        if (block_idx + 1) % 8 == 0:
            print(f"  ✓ 已绘制 {block_idx + 1}/{SPRITE_BLOCKS} blocks ({sprite_idx} sprites)")
    
    print(f"  ✓ 绘制了 {sprite_idx} 个 Sprite 符号")
    
    # 绘制 TUI 区域（Block 48-51）
    print(f"\n绘制 TUI 区域（Block {TUI_BLOCKS_START}-{TUI_BLOCKS_START + TUI_BLOCKS_ACTIVE - 1}）...")
    tui_idx = 0
    
    for block_idx in range(TUI_BLOCKS_ACTIVE):
        if tui_idx >= len(tui_chars):
            break
        
        # 计算 block 位置
        actual_block = TUI_BLOCKS_START + block_idx
        block_row = actual_block // BLOCKS_PER_ROW
        block_col = actual_block % BLOCKS_PER_ROW
        block_x = block_col * BLOCK_WIDTH
        block_y = block_row * BLOCK_HEIGHT
        
        # 在 block 内绘制 16x16 个 TUI 字符（每个 16x32px）
        # 注意：TUI 字符高度是 32px，占 2 个网格单元
        for row in range(16):
            for col in range(16):
                if tui_idx >= len(tui_chars):
                    break
                
                x = block_x + col * TUI_WIDTH
                y = block_y + row * TUI_HEIGHT
                
                texture.paste(tui_chars[tui_idx], (x, y))
                tui_idx += 1
        
        print(f"  ✓ 已绘制 Block {actual_block} ({tui_idx}/{TUI_COUNT} 字符)")
    
    print(f"  ✓ 绘制了 {tui_idx} 个 TUI 字符")
    
    # 绘制 Emoji 区域（Block 53-54）
    print(f"\n绘制 Emoji 区域（Block {EMOJI_BLOCKS_START}-{EMOJI_BLOCKS_START + EMOJI_BLOCKS_ACTIVE - 1}）...")
    emoji_idx = 0
    
    for block_idx in range(EMOJI_BLOCKS_ACTIVE):
        if emoji_idx >= len(emojis):
            break
        
        # 计算 block 位置
        actual_block = EMOJI_BLOCKS_START + block_idx
        block_row = actual_block // BLOCKS_PER_ROW
        block_col = actual_block % BLOCKS_PER_ROW
        block_x = block_col * BLOCK_WIDTH
        block_y = block_row * BLOCK_HEIGHT
        
        # 在 block 内绘制 8x16 个 Emoji（每个 32x32px）
        # 注意：Emoji 是 32x32，占 2x2 个网格单元
        for row in range(16):
            for col in range(8):
                if emoji_idx >= len(emojis):
                    break
                
                x = block_x + col * EMOJI_SIZE
                y = block_y + row * EMOJI_SIZE
                
                texture.paste(emojis[emoji_idx], (x, y))
                emoji_idx += 1
        
        print(f"  ✓ 已绘制 Block {actual_block} ({emoji_idx}/{EMOJI_COUNT} Emoji)")
    
    print(f"  ✓ 绘制了 {emoji_idx} 个 Emoji")
    
    # 保存纹理
    print(f"\n保存纹理到 {OUTPUT_PNG}...")
    texture.save(OUTPUT_PNG, "PNG")
    
    # 统计
    print("\n" + "="*60)
    print("完成！")
    print("="*60)
    print(f"纹理尺寸: {TEXTURE_SIZE}x{TEXTURE_SIZE}")
    print(f"\nBlock 布局:")
    print(f"  - Sprite: Block 0-47 ({sprite_idx} 个，索引 0-{sprite_idx-1})")
    print(f"  - TUI:    Block 48-51 ({tui_idx} 个，索引 {TOTAL_SPRITES}-{TOTAL_SPRITES+tui_idx-1})")
    print(f"  - Emoji:  Block 53-54 ({emoji_idx} 个，索引 {TOTAL_SPRITES + TUI_BLOCKS_ACTIVE * TUI_PER_BLOCK + TUI_PER_BLOCK}-{TOTAL_SPRITES + TUI_BLOCKS_ACTIVE * TUI_PER_BLOCK + TUI_PER_BLOCK + emoji_idx-1})")
    print(f"\n输出文件: {OUTPUT_PNG}")
    print(f"文件大小: {os.path.getsize(OUTPUT_PNG) / 1024 / 1024:.2f} MB")


if __name__ == "__main__":
    main()

