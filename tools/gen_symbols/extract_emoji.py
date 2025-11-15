# -*- coding: utf-8 -*-
"""
在 emoji 图片上绘制红框，用于调试网格分割是否正确。
每张图片包含 16 行 9 列的 emoji 符号。

使用类似 extract_tui.py 的参数化方式，便于手工调试。
"""

from PIL import Image, ImageDraw
import numpy as np
import os

# ---------------- 配置（调整这些参数来对齐红框） ----------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# Emoji 图片路径
EMOJI1_PATH = os.path.join(SCRIPT_DIR, "emoji1.png")
EMOJI2_PATH = os.path.join(SCRIPT_DIR, "emoji2.png")

# 输出路径
OUTPUT1_PATH = os.path.join(SCRIPT_DIR, "emoji1_debug_boxes.png")
OUTPUT2_PATH = os.path.join(SCRIPT_DIR, "emoji2_debug_boxes.png")

# ============ Emoji1 参数 ============
EMOJI1_CONFIG = {
    'NUM_ROWS': 16,           # 总行数
    'NUM_COLS': 9,            # 总列数
    'START_X': 0,             # 左上角起始 X 坐标
    'START_Y': 0,             # 左上角起始 Y 坐标
    'CELL_WIDTH': 42,         # 单元格宽度
    'CELL_HEIGHT': 41,        # 单元格高度
    'COL_GAP': 0,             # 列间距
    'ROW_GAP': 0,             # 行间距
    'INNER_PADDING': 0,       # 内边距（红框内缩）
}

# ============ Emoji2 参数 ============
EMOJI2_CONFIG = {
    'NUM_ROWS': 16,
    'NUM_COLS': 9,
    'START_X': 3,
    'START_Y': 2,
    'CELL_WIDTH': 42,
    'CELL_HEIGHT': 41,
    'COL_GAP': 0,
    'ROW_GAP': 0,
    'INNER_PADDING': 0,
}
# ------------------------------------------------------------------


def get_cell_bounds(row, col, config):
    """
    获取指定行列的单元格边界
    
    Args:
        row: 行号（从 0 开始）
        col: 列号（从 0 开始）
        config: 配置字典
    
    Returns:
        (left, top, right, bottom) 元组
    """
    start_x = config['START_X']
    start_y = config['START_Y']
    cell_width = config['CELL_WIDTH']
    cell_height = config['CELL_HEIGHT']
    col_gap = config['COL_GAP']
    row_gap = config['ROW_GAP']
    padding = config['INNER_PADDING']
    
    # 计算单元格外边界
    left = start_x + col * (cell_width + col_gap)
    top = start_y + row * (cell_height + row_gap)
    right = left + cell_width - 1
    bottom = top + cell_height - 1
    
    # 应用内边距（确保不会导致 right < left 或 bottom < top）
    padding = min(padding, cell_width // 2, cell_height // 2)
    left += padding
    top += padding
    right -= padding
    bottom -= padding
    
    # 确保边界有效
    if right < left:
        right = left
    if bottom < top:
        bottom = top
    
    return left, top, right, bottom


def detect_grid_and_draw_boxes(img_path, output_path, config):
    """
    使用参数化配置绘制红框网格
    
    Args:
        img_path: 输入图片路径
        output_path: 输出图片路径
        config: 配置字典
    """
    print(f"\n处理图片: {os.path.basename(img_path)}")
    
    # 加载图像
    img = Image.open(img_path).convert("RGBA")
    W, H = img.size
    print(f"  图片尺寸: {W}x{H}")
    
    # 显示配置
    print(f"  配置参数:")
    print(f"    网格: {config['NUM_COLS']}列 x {config['NUM_ROWS']}行")
    print(f"    起始点: ({config['START_X']}, {config['START_Y']})")
    print(f"    单元格: {config['CELL_WIDTH']}x{config['CELL_HEIGHT']}")
    print(f"    间距: 列={config['COL_GAP']}, 行={config['ROW_GAP']}")
    print(f"    内边距: {config['INNER_PADDING']}")
    
    # 创建绘图对象
    draw = ImageDraw.Draw(img)
    
    # 绘制每个单元格的红框
    for row in range(config['NUM_ROWS']):
        for col in range(config['NUM_COLS']):
            left, top, right, bottom = get_cell_bounds(row, col, config)
            
            # clamp 到图片边界
            left = max(0, left)
            top = max(0, top)
            right = min(W - 1, right)
            bottom = min(H - 1, bottom)
            
            # 确保边界有效（clamp 后可能导致问题）
            if right < left or bottom < top:
                print(f"  警告: 跳过无效单元格 ({row}, {col}): [{left}, {top}, {right}, {bottom}]")
                continue
            
            # 绘制红框
            draw.rectangle([left, top, right, bottom], outline="red", width=1)
            print([left, top, right, bottom])
    
    # 保存输出
    img.save(output_path)
    print(f"  ✓ 已保存: {output_path}")
    
    return config['CELL_WIDTH'], config['CELL_HEIGHT']


def main():
    print("="*60)
    print("Emoji 图片网格标注工具")
    print("="*60)
    
    # 检查文件是否存在
    if not os.path.exists(EMOJI1_PATH):
        print(f"错误: 找不到文件 {EMOJI1_PATH}")
        return
    
    if not os.path.exists(EMOJI2_PATH):
        print(f"错误: 找不到文件 {EMOJI2_PATH}")
        return
    
    # 处理 emoji1.png
    cell_w1, cell_h1 = detect_grid_and_draw_boxes(
        EMOJI1_PATH, 
        OUTPUT1_PATH, 
        EMOJI1_CONFIG
    )
    
    # 处理 emoji2.png
    cell_w2, cell_h2 = detect_grid_and_draw_boxes(
        EMOJI2_PATH, 
        OUTPUT2_PATH, 
        EMOJI2_CONFIG
    )
    
    print("\n" + "="*60)
    print("✓ 完成！")
    print("="*60)
    print(f"\n生成的文件:")
    print(f"  - {OUTPUT1_PATH}")
    print(f"  - {OUTPUT2_PATH}")
    print(f"\n单元格尺寸:")
    print(f"  emoji1.png: {cell_w1} x {cell_h1}")
    print(f"  emoji2.png: {cell_w2} x {cell_h2}")
    print(f"\n请检查红框是否正确对齐 emoji 符号。")
    print(f"如果不对齐，请调整脚本顶部的配置参数：")
    print(f"  - START_X, START_Y: 左上角起始坐标")
    print(f"  - CELL_WIDTH, CELL_HEIGHT: 单元格尺寸")
    print(f"  - COL_GAP, ROW_GAP: 列间距、行间距")
    print(f"  - INNER_PADDING: 内边距（红框内缩）")
    print(f"\n对齐正确后，可以使用这些参数来提取 emoji。")


if __name__ == "__main__":
    main()

