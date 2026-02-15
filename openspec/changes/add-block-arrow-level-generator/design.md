## Context

block_arrow 是一个方块箭头谜题游戏。关卡由像素画自动生成：
1. 输入：小分辨率像素画（最大 16×16），像素值 0=背景（透明），1～15 表示最多 15 种颜色
2. 覆盖：用 polyomino（俄罗斯方块形状）无重叠覆盖所有非零像素
3. **颜色约束**：每个方块只能覆盖同一种颜色的像素，拼出来的图与 bitmap 颜色完全一致
4. 箭头：每个方块上标注一个方向箭头
5. 玩法：点击箭头，该方向无遮挡则方块飞走，全部飞走即通关

## Goals / Non-Goals

- Goals: 自动关卡生成算法、可解性保证、terminal 可玩
- Non-Goals: 图形模式（后续迭代）、多人、联网、计分排行

## 1. 数据结构

```rust
/// 方块形状：相对坐标集合
#[derive(Clone, Debug)]
pub struct Shape {
    pub cells: Vec<(i8, i8)>,   // 相对坐标 (dx, dy)
    pub name: &'static str,      // 形状名称，如 "T", "I4", "L3"
}

/// 方向
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Up, Down, Left, Right,
}

/// 放置在棋盘上的方块
#[derive(Clone, Debug)]
pub struct PlacedBlock {
    pub id: usize,
    pub shape_idx: usize,        // 使用的形状在 SHAPES 中的索引
    pub cells: Vec<(usize, usize)>,  // 棋盘上的绝对坐标
    pub color: u8,               // 位图颜色 (1-15)，来自 bitmap 像素值
    pub arrow: Direction,        // 箭头方向
}

/// 关卡数据
pub struct Level {
    pub width: usize,            // 棋盘宽
    pub height: usize,           // 棋盘高
    pub bitmap: Vec<Vec<u8>>,   // 像素画：0=背景，1-15=颜色
    pub blocks: Vec<PlacedBlock>,// 放置的方块
    pub solution: Vec<usize>,    // 解序列：按此顺序移除方块 ID
}

/// 棋盘状态（游戏运行时）
pub struct Board {
    pub grid: Vec<Vec<Option<usize>>>,  // 每格存方块 ID (None=空)
    pub blocks: Vec<PlacedBlock>,
    pub removed: Vec<bool>,      // 已移除标记
}
```

## 2. 形状库

参考 README 要求和 colorblk 的 shape.rs，定义以下形状（含所有旋转/翻转变体）：

| 类别 | 名称 | 基础形状 (dx,dy) | 面积 | 变体数 |
|------|------|------------------|------|--------|
| monomino | O1 | (0,0) | 1 | 1 |
| domino | I2 | (0,0)(1,0) | 2 | 2 |
| triomino | I3 | (0,0)(1,0)(2,0) | 3 | 2 |
| triomino | L3 | (0,0)(1,0)(0,1) | 3 | 4 |
| tetromino | I4 | (0,0)(1,0)(2,0)(3,0) | 4 | 2 |
| tetromino | O4 | (0,0)(1,0)(0,1)(1,1) | 4 | 1 |
| tetromino | T | (0,0)(1,0)(2,0)(1,1) | 4 | 4 |
| tetromino | S | (1,0)(2,0)(0,1)(1,1) | 4 | 2 |
| tetromino | Z | (0,0)(1,0)(1,1)(2,1) | 4 | 2 |
| tetromino | J | (0,0)(0,1)(1,1)(2,1) | 4 | 4 |
| tetromino | L | (2,0)(0,1)(1,1)(2,1) | 4 | 4 |

旋转生成：对 (dx,dy)，90° 旋转为 (-dy,dx)，平移使最小坐标为 0。
去重：归一化后的坐标集合相同则为同一变体。

## 3. 覆盖算法（回溯搜索）

核心约束：**每个方块只能覆盖同一颜色的像素**。因此按颜色分区独立求解。

```
generate_cover(bitmap) -> Option<Vec<PlacedBlock>>:
    all_blocks = []

    // 按颜色分区独立覆盖
    for color in 1..=15:
        color_cells = [(x,y) | bitmap[y][x] == color]
        if color_cells is empty: continue

        // 对该颜色的连通区域求解覆盖
        blocks = cover_region(color_cells, color)
        if blocks is None: return None
        all_blocks.extend(blocks)

    return Some(all_blocks)

cover_region(uncovered, color) -> Option<Vec<PlacedBlock>>:
    if uncovered is empty:
        return Some([])

    // 选择最左上角的未覆盖格
    target = uncovered.first()

    // 尝试每种形状变体（优先大块）
    for shape_variant in all_variants (shuffled, large-first):
        placement = try_place(variant, target, uncovered)
        // 约束：所有格子都必须在 uncovered 中（同颜色区域内）
        if placement is valid:
            place(placement)
            result = cover_region(uncovered - placement, color)
            if result is Some:
                return Some(placement + result)
            unplace(placement)

    return None
```

优化策略：
- **按颜色分区**：每种颜色独立覆盖，大幅减少搜索空间
- 优先尝试大面积方块（tetromino > triomino > domino > monomino）
- 随机打乱尝试顺序，多次运行取最优
- 设置超时，超时后降级允许 monomino 填补

## 4. 箭头分配 + 可解性验证

```
assign_arrows(blocks) -> Option<(Vec<Direction>, Vec<usize>)>:
    // 回溯搜索：为每个方块分配箭头方向，同时构建移除顺序
    // 约束：一个方块能被移除 iff 箭头方向上无其他方块遮挡

    // 贪心+回溯：
    // 1. 找当前可自由飞走的方块（某方向无遮挡）
    // 2. 选择一个，标记箭头方向，移除
    // 3. 重复直到全部移除或无解

    remaining = all blocks
    solution = []

    while remaining is not empty:
        found = false
        for block in remaining (shuffled):
            for dir in [Up, Down, Left, Right] (shuffled):
                if can_fly(block, dir, remaining):
                    block.arrow = dir
                    solution.push(block.id)
                    remaining.remove(block)
                    found = true
                    break
            if found: break

        if not found:
            return None  // 当前覆盖方案无法生成可解关卡

    // solution 逆序即为玩家的解序列（最后移除的最先可见）
    return Some(arrows, solution.reverse())
```

`can_fly(block, dir, remaining)`: 检查 block 沿 dir 方向的路径上是否有其他 remaining 方块的格子遮挡。

## 5. 终端渲染布局

```
┌────────────────────────────────────┐
│  Block Arrow  Level 1    9x9      │
├────────────────────────────────────┤
│                                    │
│   ┌──┬──┐┌──┐                     │
│   │▲ │▲ ││◀ │                     │
│   ├──┼──┤├──┤┌──┬──┐              │
│   │  │  ││  ││▶ │▶ │              │
│   └──┴──┘└──┘├──┼──┤              │
│              │  │  │              │
│   ┌──┐      └──┴──┘              │
│   │▼ │                            │
│   └──┘                            │
│                                    │
├────────────────────────────────────┤
│  Arrows: ▲▼◀▶  [SPACE] to fly    │
│  Cursor: ←↑↓→  Blocks left: 5    │
└────────────────────────────────────┘
```

每个棋盘格 3 字符宽 × 2 行高（terminal 模式），不同方块用不同颜色。
箭头符号：▲(Up) ▼(Down) ◀(Left) ▶(Right)

## 6. 文件结构

```
apps/block_arrow/
├── lib/src/lib.rs          # 核心算法：Shape, Level, 覆盖、箭头分配、可解性
├── src/
│   ├── model.rs            # 游戏状态：Board, 输入处理, 胜利判定
│   ├── render_terminal.rs  # 终端渲染：网格、方块颜色、箭头、光标
│   └── render_graphics.rs  # (暂不修改，后续迭代)
└── assets/
    └── levels/             # 预生成关卡 JSON (可选)
```

## 7. 关键参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| BOARD_W / BOARD_H | 最大 16 | 棋盘尺寸（由 bitmap 决定，最大 16×16） |
| CELL_W (terminal) | 3 | 终端模式每格字符宽 |
| CELL_H (terminal) | 2 | 终端模式每格字符高 |
| MAX_COVER_ATTEMPTS | 100 | 覆盖算法最大尝试次数 |
| PREFER_LARGE_BLOCKS | true | 优先使用大面积方块 |
| ALLOW_MONOMINO | true | 允许单格方块填补 |
