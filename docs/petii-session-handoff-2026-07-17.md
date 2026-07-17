# PETII 图片转换会话交接（2026-07-17）

> 本文取代 `docs/petii-equals-noise-handoff-2026-07-16.md` 中关于当前实现状态的描述。旧文档保留为实验历史，不应再作为下一步编码基线。

## 1. 当前目标

将 `petii` 收敛为稳定、通用的固定字符集图片转换器，为后续“一句话生成幻想机像素风格游戏”提供美术资产生成能力。

核心约束：

- 最终资产必须仍是 PETSCII glyph + 前景色 + 背景色组成的 tile grid。
- 参考图片只用于指导转换，不能变成任意 RGBA 自定义 tile。
- 优先修复通用转换问题，避免针对狮子、月亮、云、门洞等单张测试图写特判。
- AI 迭代默认以 mode 2 为确定性转换基础。

## 2. 模式语义

- mode 0：普通图片最近似 PETSCII glyph，每个 cell 使用场景背景和局部前景色。
- mode 1：提取本来已经严格按 PETSCII cell 创作的图片，保留 cell 前景/背景颜色。
- mode 2：普通图片转换，但限制字符词汇，排除字母、数字以及目前确认会产生文本噪声的 `!`、`%`、`&` 正反 glyph。

mode 2 的过滤使用 `glyph % 128`，因此正相与反相版本会同时排除：

```text
!  33 / 161
%  37 / 165
&  38 / 166
```

## 3. 本轮关键结论

### 3.1 黑色“等号”的根因是颜色错误

此前 mode 0/2 将普通非纯色 cell 的背景固定为 palette 0（黑色）。一旦最近似字符是横杠或竖杠，未覆盖像素就会变成突兀的黑色；邻近 cell 又容易选择相似字符，于是组合成大量黑色平行线。

现在 mode 2 使用局部双颜色：

- cell 内包含场景主背景时，可以使用场景背景色；
- cell 内不包含场景背景时，从 cell 自身颜色中选择前景/背景；
- 因此深色门洞和遗迹内部不再被强制填入天空蓝。

### 3.2 用户标注的蓝色 U 形是单字符

标注图还原出的 8×8 bitmap 与 glyph 161 完全一致，即反相 `!`，不是多字符拼接。由此加入了 mode 2 的 `! % &` 过滤策略。

注意：逐个扩大标点黑名单可能让匹配退化到相似标点，例如反相分号 glyph 187。下一步若仍有大量文本形噪声，应评估“图形字符白名单”，不要继续无限追加孤立 glyph 特判。

### 3.3 等号专项后处理已经删除

下列实验已从源码中完整删除，而不只是从运行路径禁用：

- 整图水平/竖直平行线扫描；
- `ThinRun` 与等号目标集合；
- 等号坐标下降修复；
- 为每个候选反复重绘整图；
- bitmap fragmentation 专项评分；
- 等号、实心色带、checker 专用测试。

这些处理成本高、泛化弱，而且是在修复颜色症状。删除后：

- 60×60 约 16 秒；
- 80×80 约 23 秒；
- 黑色等号没有反弹。

## 4. 当前确定性转换大流程

```text
输入图片
  |
  v
1. 统一预处理
  |
  v
2. 每个 cell 生成有界候选
  |
  v
3. 强边缘 cell 的跨 tile 连续性优化
  |
  v
4. 组装、截断并验证 PetsciiGrid
  |
  v
.pix + PNG
```

### 4.1 统一预处理

- 校验 `ConversionConfig`；
- 应用 contrast；
- Lanczos3 缩放到 `grid_width × 8`、`grid_height × 8`；
- 转灰度；
- 检测场景主背景色；
- mode 0/2 生成整图 Sobel map，并删除没有连接到强轮廓的弱小连通域；
- mode 1 跳过不需要的 Sobel 计算。

### 4.2 cell 候选生成

- 与场景背景接近的纯色 cell：直接使用 space；
- 其他纯色 cell：直接使用 solid block；
- 内部有变化的 cell：在固定 PETSCII charset 中生成 Top-K；
- mode 1 对已知 PETSCII 输入做二值化；
- mode 0/2 保留灰度结构；
- mode 2 使用局部双颜色；
- 强边缘 cell 使用 fill-side mask + glyph edge overlap 距离。

### 4.3 通用边缘连续性优化

只对强边缘 cell 的有界候选集进行多轮重排，评分包含：

- 当前 cell 的图像匹配距离；
- 上下左右相邻边界的颜色/笔画连续性；
- 单字符单边悬空毛刺；
- 3×3 cell 邻域内终止的细小分支。

这一步不包含场景名称、对象类型或具体形状的特判。

### 4.4 结果组装

- 将最终选中候选放到 alternatives 第一位；
- alternatives 截断到配置的 Top-K；
- 构造并验证 typed `PetsciiGrid`；
- 上层保存 `.pix`、PNG、manifest 和候选产物。

## 5. 当前代码结构

主要文件：

```text
tools/petii/src/converter.rs
```

重要入口与职责：

- `convert_image`：四阶段编排；
- `prepare_reference`：统一预处理；
- `CandidateGenerator::generate`：单 cell 候选生成；
- `select_cell_colors`：mode 相关颜色策略；
- `rank_glyphs`：固定字符集 Top-K；
- `EdgeTarget`：fill mask + edge overlap 距离；
- `clean_edge_image`：弱边缘连通域清理；
- `refine_edge_continuity`：跨 tile 连续性重排；
- `neighborhood_artifact_penalty`：通用邻域毛刺抑制；
- `rendered_color_index`：统一 glyph bitmap 到 palette index 的取色逻辑。

本轮将 `converter.rs` 从约 1576 行降到约 1156 行。输出行为不变。

## 6. 最新视觉回归产物

固定输入：

```text
tmp/lion-image2/reference.png
```

重构后输出：

```text
tmp/lion-mode2-refactored-60x60/final.png
tmp/lion-mode2-refactored-60x60/final.pix
tmp/lion-mode2-refactored-80x80/final.png
tmp/lion-mode2-refactored-80x80/final.pix
```

结果：

```text
60×60 score=0.009914, time≈16.06s
80×80 score=0.008272, time≈22.74s
```

重构后的 60/80 `.pix` 和 PNG 与重构前的 `lion-mode2-local-colors-*` 逐字节一致。

复现命令：

```bash
cargo run -p petii -- ai "月光下守卫废墟的狮子" \
  --input tmp/lion-image2/reference.png \
  --direct --mode 2 --width 60 --height 60 \
  --output-dir tmp/lion-mode2-refactored-60x60

cargo run -p petii -- ai "月光下守卫废墟的狮子" \
  --input tmp/lion-image2/reference.png \
  --direct --mode 2 --width 80 --height 80 \
  --output-dir tmp/lion-mode2-refactored-80x80
```

使用 `--input` + `--direct` 不需要 API key。

## 7. 当前验证状态

已通过：

```bash
cargo test -p petii
cargo check -p petii
openspec validate add-ai-petscii-generation-loop --strict
git diff --check
```

结果：

```text
31 library tests passed
5 CLI tests passed
OpenSpec strict validation passed
```

`cargo clippy -p petii --all-targets -- -D warnings` 会被 `src/render/symbol_map.rs` 中两个已有 warning 阻挡：

- `needless_range_loop`；
- `unnecessary_map_or`。

它们不在本轮 petii 修改范围内。

## 8. 当前 Git 状态

基线：

```text
branch: main
base commit: 7da0a43c update petii
```

尚未提交的文件：

```text
tools/petii/src/converter.rs
tools/petii/README.md
openspec/changes/add-ai-petscii-generation-loop/tasks.md
openspec/changes/add-ai-petscii-generation-loop/specs/petscii-generation/spec.md
docs/petii-session-handoff-2026-07-17.md
```

`tmp/lion-*` 为本地视觉产物，通常不会进入 Git。换电脑后若需要肉眼比较，应重新生成，或显式将选定样例加入版本化 benchmark 目录。

## 9. 换电脑后的恢复步骤

在当前电脑提交并推送后，另一台电脑执行：

```bash
git pull
git status --short
cargo test -p petii
openspec validate add-ai-petscii-generation-loop --strict
```

然后确认参考图是否存在：

```bash
test -f tmp/lion-image2/reference.png
```

如果 `tmp` 没有提交，这是正常的。需要把参考图单独复制过去，或选择一个版本库内的 benchmark reference 再运行转换。

恢复工作时优先阅读：

```text
docs/petii-session-handoff-2026-07-17.md
tools/petii/README.md
tools/petii/src/converter.rs
openspec/changes/add-ai-petscii-generation-loop/tasks.md
```

## 10. 推荐的下一步

1. 先提交当前“局部颜色修复 + 流程精简”作为稳定基线。
2. 用多张不同题材图片验证局部颜色策略的泛化性，不再只看 lion reference。
3. 若仍有文本形噪声，统计实际 glyph 分布，再评估 mode 2 图形字符白名单；避免逐个追加标点黑名单。
4. 将少量固定 reference 和期望统计沉淀到版本化 benchmark，减少依赖 `tmp` 和人工截图。
5. 等确定性转换稳定后，再继续 AI critic/repair loop，避免 AI 层掩盖底层转换问题。

## 11. 轮廓语法与保守去噪进展（同日续）

在提交 `965e1e1` 之后，mode 2 已加入独立于单字符近似的轮廓语法层：

- 从参考边缘构建 cell 级 contour graph，区分 open chain、closed loop 和 junction；
- 为字符建立四边 edge-port topology，并按人工作品统计采用 ±1 像素端口容差；
- 通过链动态规划、junction 协调、连续性细化和 pair repair 优化跨 cell 轮廓；
- 全局参考误差最多允许比 Top-1 基线增加 5%；
- 输出 `edge-metrics.json` 和 `edge-debug.png`，用于同时检查外观、断裂、意外端点、spur 和 orphan excursion。

当前进一步加入保守 orphan cleanup：只允许用 Blank/Solid 候选删除孤立笔画，不允许把一种噪音换成另一种标点；删除候选与当前字符的 reference distance 差必须不超过 `0.04`。若 5% 预算已经耗尽，清理器通过回滚别处价值最低的轮廓编辑腾出预算，而不是重新裁剪整幅图。

固定输入 `tmp/lion.png` 的当前审核候选：

```text
tmp/lion-mode2-edge-v26-conservative-denoise-60x60/final.png
tmp/lion-mode2-edge-v26-conservative-denoise-60x60/final.pix
```

v26 相对 Top-1 基线的主要指标：

```text
shared_port_break_rate: 0.4874 -> 0.3838
unexpected_endpoint_rate: 0.4190 -> 0.3209
spur_cell_count: 83 -> 57
orphan_excursion_count: 338 -> 269
reference_loss: 0.16622 -> 0.17406（+4.72%，在 5% 预算内）
```

这版已删除狮子口鼻处和背景中的多处高置信短笔画，但断裂率比纯轮廓版约高 4.5 个百分点。下一步应以人工审核决定是否接受这一视觉/连续性权衡；不要再放宽删除阈值。若继续优化，应研究局部“替换为正确 corner/straight glyph”的平滑策略，而不是扩大 Blank/Solid 删除范围。

## 12. 参考图约束的最终重涂色（同日续）

v26 暴露出颜色与 glyph 决策次序的问题：cell 的前景/背景色在候选生成时确定，但轮廓优化和去噪可能随后换掉 glyph。最终 glyph 的前景/背景像素归属改变后，继续沿用旧颜色会产生视觉缺口或突兀色块。

v27 在 Mode 2 的所有 glyph 决策结束后增加 `reference-repaint`：

1. 固定最终 glyph，不再修改字符与拓扑；
2. 按 glyph bitmap 将每格分成 foreground/background 两个实际区域；
3. 分别从参考图重新拟合调色板颜色；
4. 用两遍确定性局部协调抑制邻格突色；
5. 只有当输出边界色差明显大于参考图同位置色差时才施加连续性惩罚，因此原图已有的硬边仍保留。

审核产物：

```text
tmp/lion-mode2-edge-v27-reference-repaint-60x60/final.png
tmp/lion-mode2-edge-v27-reference-repaint-60x60/final.pix
```

相对 v26，3600 个 glyph 全部保持不变；769 格前景色和 1915 格背景色被重新拟合。release 模式的最终重建 score 从 `0.009748` 降至 `0.007457`，改善约 23.5%；重涂阶段耗时约 `0.065s`。

随后人工审核指出 v27 的主背景偏灰紫。根因是 RGB 欧氏距离把 `[95,95,135]` 视为接近低饱和蓝灰，但人眼会明显感到色相漂移。最终 v30 删除自定义 HSV 实验，直接复用 RustPixel `color_distance_rgb`（Lab + CIEDE2000）对调色板候选排序：

```text
tmp/lion-mode2-edge-v30-ciede2000-repaint-60x60/final.png
tmp/lion-mode2-edge-v30-ciede2000-repaint-60x60/final.pix
```

v30 保持 3600 个 glyph 不变，相对 v26 仅调整 563 格前景色和 344 格背景色；天空恢复为明确蓝色，近黑主体不再被色相模型污染。最终 score 为 `0.007625`，相对 v26 改善约 21.8%。失败的 v28/v29 仅是本地实验产物，不应作为恢复基线。
