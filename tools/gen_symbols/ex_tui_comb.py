#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
从 data/source.png 提取 TUI 字符，使用与 extract_tui.py 相同的矩形逻辑
然后组合成最终的 symbols.png
"""

from PIL import Image, ImageDraw
import numpy as np
import os
import math

# ============ 配置 ============
BASE_DIR = "./data"
SOURCE_IMG = f"{BASE_DIR}/source.png"
OUTPUT_SYMBOLS = f"{BASE_DIR}/symbols.png"
TUI_TXT = "tui.txt"

# 列检测参数（与 extract_tui.py 一致）
BAR_PERCENTILE = 1.0
MERGE_GAP = 2
LEFT_EDGE_AS_BAR = True
RIGHT_EDGE_AS_BAR = True

# 固定行规则（与 extract_tui.py 一致）
XADJ = 14
ADJ = 10
TOP0 = 11 - ADJ          # 第一行 top = 3
ROW_HEIGHT = 70 + ADJ    # 每行高度 = 70
ROW_GAP = 10 - ADJ       # 行间距 = 13
NUM_ROWS = 16            # 总共 16 行

# 目标纹理配置
TEXTURE_SIZE = 1024
CELL_WIDTH_SPRITE = 8
CELL_HEIGHT_SPRITE = 8
CELL_WIDTH_TUI = 8
CELL_HEIGHT_TUI = 16
CELL_WIDTH_EMOJI = 16
CELL_HEIGHT_EMOJI = 16

# TUI region: rows 96-127, cols 0-79
TUI_START_ROW = 96
TUI_START_COL = 0
TUI_COLS = 80
TUI_ROWS = 32

# Emoji region: rows 96-127, cols 80-127
EMOJI_START_ROW = 96
EMOJI_START_COL = 80
EMOJI_COLS = 48
EMOJI_ROWS = 32
# ==============================


def parse_tui_txt(filename):
    """解析 tui.txt，返回 ASCII 字符列表"""
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
    
    # Section 1: ASCII/TUI characters
    ascii_chars = []
    if len(sections) > 0:
        for line in sections[0]:
            ascii_chars.extend(list(line))
    
    return ascii_chars


def detect_columns(img):
    """检测列分隔线（与 extract_tui.py 逻辑一致）"""
    W, H = img.size
    
    # 计算亮度
    gray = np.array(img.convert("L"))
    darkness = (255 - gray).sum(axis=0).astype(float)
    darkness = (darkness - darkness.min()) / (darkness.max() - darkness.min() + 1e-6)
    
    # 找候选竖线
    th = np.percentile(darkness, BAR_PERCENTILE)
    cand = darkness <= th
    
    # 合并裂开的细竖线段
    merged = cand.copy()
    gap = 0
    for i in range(W):
        if not cand[i]:
            gap += 1
            if gap <= MERGE_GAP and i > 0 and i < W-1 and cand[i-1] and cand[i+1]:
                merged[i] = True
            elif gap > MERGE_GAP:
                gap = 0
        else:
            gap = 0
    
    # 收集竖线中心位置
    bars = []
    i = 0
    while i < W:
        if merged[i]:
            s = i
            while i < W and merged[i]:
                i += 1
            e = i - 1
            bars.append((s + e) // 2)
        else:
            i += 1
    
    # 确保左右边界
    if LEFT_EDGE_AS_BAR and (len(bars) == 0 or bars[0] > 0):
        bars = [0] + bars
    if RIGHT_EDGE_AS_BAR and (len(bars) == 0 or bars[-1] < W - 1):
        bars = bars + [W - 1]
    
    # 转成列区间
    columns = []
    for b0, b1 in zip(bars[:-1], bars[1:]):
        L = b0 + 1
        R = b1 - 1
        if R >= L:
            columns.append((L, R))
    
    return columns


def get_row_bounds(n):
    """第 n 行（从 1 开始）的 top/bottom"""
    top = TOP0 + (n - 1) * (ROW_HEIGHT + ROW_GAP)
    bottom = top + ROW_HEIGHT - 1
    return top, bottom


def extract_characters(img, columns, ascii_chars):
    """
    从 source.png 提取字符
    返回: list of (char, PIL.Image)
    """
    W, H = img.size
    extracted = []
    
    print(f"Extracting {len(ascii_chars)} characters from {len(columns)} columns x {NUM_ROWS} rows...")
    
    char_idx = 0
    for row in range(1, NUM_ROWS + 1):
        for col_idx, (L, R) in enumerate(columns):
            if char_idx >= len(ascii_chars):
                break
            
            char = ascii_chars[char_idx]
            
            t, b = get_row_bounds(row)
            if b < 0 or t >= H:
                continue
            
            # clamp
            t = max(0, t)
            b = min(H - 1, b)
            
            # 应用 XADJ 调整
            left = L + XADJ
            right = R - XADJ
            
            # 提取字符图像
            char_img = img.crop((left, t, right + 1, b + 1))
            
            extracted.append((char, char_img))
            char_idx += 1
            
            if char_idx % 50 == 0:
                print(f"  Extracted {char_idx}/{len(ascii_chars)}...")
        
        if char_idx >= len(ascii_chars):
            break
    
    print(f"✓ Extracted {len(extracted)} characters")
    return extracted


def create_tui_texture(char_images, target_w, target_h, cols, rows):
    """
    创建对齐的 TUI 纹理
    char_images: list of (char, PIL.Image)
    """
    width = cols * target_w
    height = rows * target_h
    
    texture = Image.new('RGBA', (width, height), (0, 0, 0, 0))
    
    print(f"Creating TUI texture: {width}x{height} ({cols}x{rows} grid, cell={target_w}x{target_h})")
    
    for idx, (char, char_img) in enumerate(char_images):
        if idx >= cols * rows:
            break
        
        col = idx % cols
        row = idx // cols
        
        x = col * target_w
        y = row * target_h
        
        # 缩放到目标尺寸
        resized = char_img.resize((target_w, target_h), Image.Resampling.LANCZOS)
        
        texture.paste(resized, (x, y), resized)
    
    return texture


def combine_textures(sprite_path, tui_texture, output_path):
    """
    组合 Sprite 和 TUI 纹理到最终的 symbols.png
    """
    final = Image.new('RGBA', (TEXTURE_SIZE, TEXTURE_SIZE), (0, 0, 0, 255))
    
    # 加载现有的 sprite 纹理（rows 0-95）
    if os.path.exists(sprite_path):
        sprite = Image.open(sprite_path).convert('RGBA')
        # 粘贴 sprite 区域（rows 0-95, all 128 columns）
        sprite_region = sprite.crop((0, 0, TEXTURE_SIZE, 768))
        final.paste(sprite_region, (0, 0), sprite_region)
        print(f"✓ Loaded existing sprite texture from: {sprite_path}")
    else:
        print(f"Warning: Sprite texture not found: {sprite_path}")
        print("  Creating black placeholder for sprite region")
    
    # 粘贴 TUI 区域（rows 96-127, cols 0-79）
    tui_x = TUI_START_COL * CELL_WIDTH_SPRITE
    tui_y = TUI_START_ROW * CELL_HEIGHT_SPRITE
    final.paste(tui_texture, (tui_x, tui_y), tui_texture)
    print(f"✓ Pasted TUI texture at ({tui_x}, {tui_y})")
    
    final.save(output_path)
    print(f"✓ Saved final texture: {output_path}")
    return final


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    
    # 解析 tui.txt
    tui_txt_path = os.path.join(script_dir, TUI_TXT)
    print(f"Parsing {tui_txt_path}...")
    ascii_chars = parse_tui_txt(tui_txt_path)
    print(f"  Found {len(ascii_chars)} ASCII/TUI characters")
    
    # 加载源图像
    source_path = os.path.join(script_dir, SOURCE_IMG)
    print(f"\nLoading source image: {source_path}...")
    img = Image.open(source_path).convert("RGBA")
    print(f"  Image size: {img.size}")
    
    # 检测列
    print(f"\nDetecting columns...")
    columns = detect_columns(img)
    print(f"  Found {len(columns)} columns")
    
    # 提取字符
    print(f"\nExtracting characters...")
    char_images = extract_characters(img, columns, ascii_chars)
    
    # 创建对齐的 TUI 纹理
    print(f"\nCreating aligned TUI texture...")
    tui_texture = create_tui_texture(
        char_images,
        CELL_WIDTH_TUI,
        CELL_HEIGHT_TUI,
        TUI_COLS,
        TUI_ROWS
    )
    
    # 保存中间结果
    tui_output = os.path.join(script_dir, BASE_DIR, 'tui_extracted.png')
    os.makedirs(os.path.dirname(tui_output), exist_ok=True)
    tui_texture.save(tui_output)
    print(f"✓ Saved TUI texture: {tui_output}")
    
    # 组合到最终纹理
    print(f"\nCombining textures...")
    sprite_path = os.path.join(script_dir, BASE_DIR, 'symbols_sprite.png')
    final_output = os.path.join(script_dir, OUTPUT_SYMBOLS)
    combine_textures(sprite_path, tui_texture, final_output)
    
    print(f"\n{'='*60}")
    print("✓ All done!")
    print(f"{'='*60}")
    print(f"\nGenerated files:")
    print(f"  - {tui_output} (TUI texture)")
    print(f"  - {final_output} (Final symbols.png)")
    print(f"\nNext steps:")
    print(f"  1. Review {final_output}")
    print(f"  2. Copy to ../../assets/pix/symbols.png")
    print(f"  3. Test with: cargo pixel r ui_demo wg -r")


if __name__ == "__main__":
    main()

