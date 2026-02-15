先实现核心算法库（lib），再实现 model 和 terminal 渲染。确保每一步可编译可测试。

## Phase 0: 方块形状定义 ✨

- [ ] 0.1 在 `lib/src/lib.rs` 定义 `Shape` 结构体和 `Direction` 枚举
- [ ] 0.2 定义所有基础形状坐标（monomino, domino, triomino, tetromino）
- [ ] 0.3 实现旋转函数 `rotate90(cells)` 和归一化函数 `normalize(cells)`
- [ ] 0.4 生成所有形状的所有唯一旋转变体 `generate_all_variants()`
- [ ] 0.5 简单单测验证变体数量正确（如 T 有 4 个变体，O4 有 1 个）

## Phase 1: 覆盖算法

- [ ] 1.1 定义 `Level` 和 `PlacedBlock` 数据结构
- [ ] 1.2 实现内置测试位图（9×9 实例数据，最大支持 16×16）
- [ ] 1.3 实现回溯覆盖算法 `solve_cover(bitmap) -> Option<Vec<PlacedBlock>>`
  - 选择最左上角未覆盖格
  - 尝试每个形状变体的每种放置方式
  - 优先大面积块，monomino 作为最后手段
- [ ] 1.4 覆盖算法优化：随机化 + 多次尝试 + 超时降级
- [ ] 1.5 验证：打印覆盖结果到控制台，确认位图可被完整覆盖

## Phase 2: 箭头分配与可解性

- [ ] 2.1 实现 `can_fly(block, dir, remaining_blocks)` 碰撞检测
- [ ] 2.2 实现贪心箭头分配 `assign_arrows(blocks) -> Option<solution>`
  - 每轮找一个可飞走的方块，分配箭头并移除
  - 重复直到全部移除或死锁
- [ ] 2.3 集成：`generate_level(bitmap) -> Option<Level>` 完整流程
  - 先覆盖，再分配箭头，失败则重试覆盖
- [ ] 2.4 验证：打印生成的关卡（方块 + 箭头 + 解序列）

## Phase 3: 游戏模型

- [ ] 3.1 重写 `model.rs` — `BlockArrowModel` 结构体
  - `Board`（棋盘网格）, `blocks`, `cursor`, `game_state`
- [ ] 3.2 实现 `init()` — 调用关卡生成器创建关卡
- [ ] 3.3 实现 `handle_input()` — 方向键移动光标，空格/回车触发飞走
- [ ] 3.4 实现 `try_fly(block_id)` — 检查选中方块能否飞走，执行移除
- [ ] 3.5 实现胜利判定和关卡推进

## Phase 4: 终端渲染

- [ ] 4.1 重写 `render_terminal.rs` — `BlockArrowRender` 结构体
- [ ] 4.2 绘制棋盘网格（每格 3×2 字符）
- [ ] 4.3 绘制方块颜色和箭头符号（▲▼◀▶）
- [ ] 4.4 绘制光标高亮和状态栏（剩余方块数、关卡号）
- [ ] 4.5 集成测试：`cargo pixel r block_arrow t` 可玩

## Phase 5: 打磨（可选）

- [ ] 5.1 多关卡支持：内置多个像素画位图
- [ ] 5.2 飞走动画（terminal 模式简单闪烁/消失效果）
- [ ] 5.3 难度控制：调整允许的方块类型影响难度
