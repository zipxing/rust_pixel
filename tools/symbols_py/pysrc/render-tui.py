#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
渲染 tui.txt 中的 TUI 字符和 Emoji
- TUI 字符: 8x16 像素，使用 DroidSansMono Nerd Font
- Emoji: 16x16 像素，使用 Apple Color Emoji
"""

import os
import sys
import Quartz
import CoreText

# ---------------- 配置 ----------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# 输入文件
TUI_TXT = os.path.join(SCRIPT_DIR, "tui.txt")

# 输出目录
TUI_OUTPUT_DIR = os.path.join(SCRIPT_DIR, "tui_chars")
EMOJI_OUTPUT_DIR = os.path.join(SCRIPT_DIR, "tui_emojis")

# 渲染参数
TUI_WIDTH = 40
TUI_HEIGHT = 80
TUI_FONT_NAME = "DroidSansMono Nerd Font"
TUI_FONT_SIZE = 64  # 调整以适应 8x16

EMOJI_SIZE = 64
EMOJI_FONT_NAME = "Apple Color Emoji"
EMOJI_FONT_SIZE = 64  # 调整以适应 16x16
# ------------------------------------------


def parse_tui_txt(filepath):
    """
    解析 tui.txt
    
    Returns:
        (tui_chars, emojis)
        - tui_chars: list of TUI characters (ASCII + special chars)
        - emojis: list of emoji strings
    """
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    # 跳过开头的空行
    start_idx = 0
    while start_idx < len(lines) and lines[start_idx].strip() == '':
        start_idx += 1
    
    # 从第一个非空行开始，找到分隔 TUI 和 Emoji 的空行
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
    
    # 解析 TUI 字符
    # 之前用 | 分隔，现在紧密排列
    tui_chars = []
    
    # 强制在第一个位置添加空格 (Index 0)
    tui_chars.append(' ')
    
    for line in tui_lines:
        line = line.strip()
        if line:
            # 直接按字符解析
            for char in line:
                tui_chars.append(char)
                
    print(f"  ✓ 解析到 {len(tui_chars)} 个 TUI 字符")
    
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
    
    print(f"  ✓ 解析到 {len(emojis)} 个 Emoji")
    
    return tui_chars, emojis


def render_char_to_png(char, output_path, width, height, font_name, font_size):
    """
    渲染单个字符为 PNG 图片
    
    Args:
        char: 字符
        output_path: 输出 PNG 文件路径
        width: 图片宽度
        height: 图片高度
        font_name: 字体名称
        font_size: 字体大小
    """
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
        print(f"错误: 无法创建位图上下文")
        return False
    
    # 清空背景（透明）
    Quartz.CGContextClearRect(context, Quartz.CGRectMake(0, 0, width, height))
    
    # 设置文本绘制模式
    Quartz.CGContextSetTextDrawingMode(context, Quartz.kCGTextFill)
    
    # 设置文本颜色为白色
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
    
    # 获取字形边界以居中
    bounds = CoreText.CTLineGetBoundsWithOptions(line, 0)
    text_width = bounds.size.width
    text_height = bounds.size.height
    
    # 获取字体度量信息
    ascent = CoreText.CTFontGetAscent(font)
    descent = CoreText.CTFontGetDescent(font)
    leading = CoreText.CTFontGetLeading(font)
    
    # 计算居中位置
    # 水平居中
    x = (width - text_width) / 2.0
    
    # 垂直居中：使用字体度量
    font_height = ascent + descent + leading
    baseline_y = (height - font_height) / 2.0 + descent
    
    # 设置文本位置
    Quartz.CGContextSetTextPosition(context, x, baseline_y)
    
    # 绘制文本
    CoreText.CTLineDraw(line, context)
    
    # 创建 CGImage
    image = Quartz.CGBitmapContextCreateImage(context)
    
    if image is None:
        print(f"错误: 无法创建图像")
        return False
    
    # 保存为 PNG
    url = Quartz.CFURLCreateFromFileSystemRepresentation(
        None,
        output_path.encode('utf-8'),
        len(output_path.encode('utf-8')),
        False
    )
    
    dest = Quartz.CGImageDestinationCreateWithURL(url, "public.png", 1, None)
    
    if dest is None:
        print(f"错误: 无法创建图像目标")
        return False
    
    Quartz.CGImageDestinationAddImage(dest, image, None)
    success = Quartz.CGImageDestinationFinalize(dest)
    
    return success


def main():
    print("="*60)
    print("渲染 tui.txt 中的 TUI 字符和 Emoji")
    print("="*60)
    
    # 检查文件
    if not os.path.exists(TUI_TXT):
        print(f"错误: 找不到 {TUI_TXT}")
        sys.exit(1)
    
    # 解析 tui.txt
    print(f"\n解析 {TUI_TXT}...")
    tui_chars, emojis = parse_tui_txt(TUI_TXT)
    
    if len(tui_chars) == 0 and len(emojis) == 0:
        print("错误: 未找到任何字符")
        sys.exit(1)
    
    # 创建输出目录
    import shutil
    
    if os.path.exists(TUI_OUTPUT_DIR):
        shutil.rmtree(TUI_OUTPUT_DIR)
    os.makedirs(TUI_OUTPUT_DIR)
    
    if os.path.exists(EMOJI_OUTPUT_DIR):
        shutil.rmtree(EMOJI_OUTPUT_DIR)
    os.makedirs(EMOJI_OUTPUT_DIR)
    
    print(f"\n创建输出目录:")
    print(f"  - {TUI_OUTPUT_DIR}")
    print(f"  - {EMOJI_OUTPUT_DIR}")
    
    # 渲染 TUI 字符
    print(f"\n渲染 {len(tui_chars)} 个 TUI 字符 ({TUI_WIDTH}x{TUI_HEIGHT})...")
    tui_success = 0
    
    for idx, char in enumerate(tui_chars):
        # 文件名：索引_字符.png
        # 对于特殊字符，使用 Unicode 码点
        if char.isprintable() and char not in ['/', '\\', ':', '*', '?', '"', '<', '>', '|']:
            char_name = char
        else:
            char_name = f"U{ord(char):04X}"
        
        output_file = os.path.join(TUI_OUTPUT_DIR, f"{idx:04d}_{char_name}.png")
        
        if render_char_to_png(char, output_file, TUI_WIDTH, TUI_HEIGHT, 
                              TUI_FONT_NAME, TUI_FONT_SIZE):
            tui_success += 1
            if (idx + 1) % 50 == 0:
                print(f"  ✓ 已渲染 {idx + 1}/{len(tui_chars)}")
        else:
            print(f"  ✗ 渲染失败: {idx:4d} {char}")
    
    # 渲染 Emoji
    print(f"\n渲染 {len(emojis)} 个 Emoji ({EMOJI_SIZE}x{EMOJI_SIZE})...")
    emoji_success = 0
    
    for idx, emoji in enumerate(emojis):
        # 文件名：索引_emoji.png
        emoji_clean = emoji.replace('\uFE0F', '').replace('\u200D', '_')
        output_file = os.path.join(EMOJI_OUTPUT_DIR, f"{idx:04d}_{emoji_clean}.png")
        
        if render_char_to_png(emoji, output_file, EMOJI_SIZE, EMOJI_SIZE,
                              EMOJI_FONT_NAME, EMOJI_FONT_SIZE):
            emoji_success += 1
            if (idx + 1) % 50 == 0:
                print(f"  ✓ 已渲染 {idx + 1}/{len(emojis)}")
        else:
            print(f"  ✗ 渲染失败: {idx:4d} {emoji}")
    
    # 统计
    print("\n" + "="*60)
    print("完成！")
    print("="*60)
    print(f"TUI 字符: {tui_success}/{len(tui_chars)}")
    print(f"Emoji:    {emoji_success}/{len(emojis)}")
    print(f"\n输出目录:")
    print(f"  - {TUI_OUTPUT_DIR}")
    print(f"  - {EMOJI_OUTPUT_DIR}")


if __name__ == "__main__":
    main()

