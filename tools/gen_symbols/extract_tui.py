# -*- coding: utf-8 -*-
"""
从 tuinew.png 提取 TUI 字符
使用固定网格参数（16行18列）
"""

from PIL import Image, ImageDraw
import os

# ---------------- 配置（调整这些参数来对齐红框） ----------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# TUI 图片路径
TUI_IMG_PATH = os.path.join(SCRIPT_DIR, "tuinew.png")

# 输出路径
OUTPUT_DEBUG = os.path.join(SCRIPT_DIR, "tui_debug_boxes.png")
OUTPUT_DIR = os.path.join(SCRIPT_DIR, "tui")

# ============ TUI 网格参数 ============
TUI_CONFIG = {
    'NUM_ROWS': 16,           # 总行数
    'NUM_COLS': 18,           # 总列数
    'START_X': 11,            # 左上角起始 X 坐标
    'START_Y': 5,             # 左上角起始 Y 坐标
    'CELL_WIDTH': 42,         # 单元格宽度
    'CELL_HEIGHT': 82,        # 单元格高度
    'COL_GAP': 0,             # 列间距
    'ROW_GAP': 0,             # 行间距
    'INNER_PADDING': 0,       # 内边距（红框内缩）
}

# 高度过滤阈值
MIN_HEIGHT = 30
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


def extract_and_draw(img_path, output_debug, output_dir, config):
    """
    提取字符并绘制调试框
    
    Args:
        img_path: 输入图片路径
        output_debug: 调试图输出路径
        output_dir: 字符输出目录
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
    
    # 创建输出目录
    os.makedirs(output_dir, exist_ok=True)
    
    # 创建绘图对象
    draw = ImageDraw.Draw(img)
    
    char_id = 0
    skipped_count = 0
    
    # 先行后列：第一行为 0～17，第二行 18～35 ...
    for row in range(config['NUM_ROWS']):
        for col in range(config['NUM_COLS']):
            left, top, right, bottom = get_cell_bounds(row, col, config)
            
            # clamp 到图片边界
            left = max(0, left)
            top = max(0, top)
            right = min(W - 1, right)
            bottom = min(H - 1, bottom)
            
            # 计算高度
            height = bottom - top + 1
            
            # 确保边界有效（clamp 后可能导致问题）
            if right < left or bottom < top:
                print(f"  警告: 跳过无效单元格 ({row}, {col}): [{left}, {top}, {right}, {bottom}]")
                skipped_count += 1
                continue
            
            # 过滤高度太小的字符
            if height < MIN_HEIGHT:
                print(f"[跳过] 行={row+1}, 列={col+1}, 高度 {height} < {MIN_HEIGHT}")
                skipped_count += 1
                # 用红色标注跳过的
                draw.rectangle([left, top, right, bottom], outline="red", width=1)
                continue
            
            print(f"[{char_id:04d}] 行={row+1}, 列={col+1}, 高度={height}, 边界=[{left}, {top}, {right}, {bottom}]")
            
            # 提取这个矩形区域的图像
            char_img = img.crop((left, top, right + 1, bottom + 1))
            
            # 保存为单独的文件
            char_output = os.path.join(output_dir, f"{char_id:04d}.png")
            char_img.save(char_output)
            
            # 绘制绿框（表示有效）
            draw.rectangle([left, top, right, bottom], outline="green", width=1)
            
            char_id += 1
    
    # 保存调试图
    img.save(output_debug)
    print(f"\n✓ 调试图已保存: {output_debug}")
    print(f"✓ 提取了 {char_id} 个有效字符到: {output_dir}/")
    print(f"✓ 跳过了 {skipped_count} 个字符")
    
    return char_id, skipped_count


def main():
    print("="*60)
    print("TUI 字符提取工具（固定网格）")
    print("="*60)
    
    # 检查文件
    if not os.path.exists(TUI_IMG_PATH):
        print(f"错误: 找不到 {TUI_IMG_PATH}")
        return
    
    # 提取字符并绘制调试框
    char_count, skipped = extract_and_draw(
        TUI_IMG_PATH,
        OUTPUT_DEBUG,
        OUTPUT_DIR,
        TUI_CONFIG
    )
    
    print("\n" + "="*60)
    print("✓ 完成！")
    print("="*60)
    print(f"\n生成的文件:")
    print(f"  - {OUTPUT_DEBUG} (调试图)")
    print(f"  - {OUTPUT_DIR}/ (提取的字符)")
    print(f"\n统计:")
    print(f"  - 有效字符: {char_count}")
    print(f"  - 跳过字符: {skipped}")
    print(f"\n如果红框不对齐，请调整脚本顶部的配置参数：")
    print(f"  - START_X, START_Y: 左上角起始坐标")
    print(f"  - CELL_WIDTH, CELL_HEIGHT: 单元格尺寸")
    print(f"  - COL_GAP, ROW_GAP: 列间距、行间距")
    print(f"  - INNER_PADDING: 内边距（红框内缩）")


if __name__ == "__main__":
    main()
