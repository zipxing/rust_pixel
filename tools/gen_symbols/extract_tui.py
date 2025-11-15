# -*- coding: utf-8 -*-
"""
在原图上绘制按列 & 按固定行规则切割出的字符红框，
用于调试列分割和行分割是否正确。
"""

from PIL import Image, ImageDraw
import numpy as np
import os

# ---------------- 配置（只需改这里） ----------------
BASE_DIR = "./"
IMG_PATH = f"{BASE_DIR}/tui.png"      # 原始整张字符表
OUTPUT_PATH = f"{BASE_DIR}/tui_debug_boxes.png"

# 列检测参数（亮度最亮的竖线作为分隔）
BAR_PERCENTILE = 1.0
MERGE_GAP = 2
LEFT_EDGE_AS_BAR = True
RIGHT_EDGE_AS_BAR = True

# 固定行规则（你提供的数据）
XADJ = 14
ADJ = 10
TOP0 = 11 - ADJ          # 第一行 top = 3
ROW_HEIGHT = 70 + ADJ   # 每行高度 = 70
ROW_GAP = 10 - ADJ      # 行间距 = 13
NUM_ROWS = 16     # 总共 16 行
# ----------------------------------------------------


# === 加载图像 ===
img = Image.open(IMG_PATH).convert("RGBA")
W, H = img.size

# === 计算亮度并识别列竖线 ===
gray = np.array(img.convert("L"))
darkness = (255 - gray).sum(axis=0).astype(float)
darkness = (darkness - darkness.min()) / (darkness.max() - darkness.min() + 1e-6)

# 根据亮度最亮列找候选竖线
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

# 确保左右边界有分隔线
if LEFT_EDGE_AS_BAR and (len(bars) == 0 or bars[0] > 0):
    bars = [0] + bars
if RIGHT_EDGE_AS_BAR and (len(bars) == 0 or bars[-1] < W - 1):
    bars = bars + [W - 1]

# 把列分隔线转成列区间
columns = []
for b0, b1 in zip(bars[:-1], bars[1:]):
    L = b0 + 1
    R = b1 - 1
    if R >= L:
        columns.append((L, R))


# === 行区间函数 ===
def get_row_bounds(n):
    """
    第 n 行（从 1 开始）的 top/bottom
    """
    top = TOP0 + (n - 1) * (ROW_HEIGHT + ROW_GAP)
    bottom = top + ROW_HEIGHT - 1
    return top, bottom


# === 在原图上绘制红框 ===
draw = ImageDraw.Draw(img)

for (L, R) in columns:
    for row in range(1, NUM_ROWS + 1):
        t, b = get_row_bounds(row)
        if b < 0 or t >= H:
            continue
        # clamp
        t = max(0, t)
        b = min(H - 1, b)
        print([L + XADJ, t, R - XADJ, b])
        draw.rectangle([L + XADJ, t, R - XADJ, b], outline="red", width=1)
        # break

# === 保存输出 ===
os.makedirs(BASE_DIR, exist_ok=True)
img.save(OUTPUT_PATH)

print("调试图已生成：", OUTPUT_PATH)

