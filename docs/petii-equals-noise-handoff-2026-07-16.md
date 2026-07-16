# PETII “等号”噪声处理交接（2026-07-16）

## 当前目标

继续改善 `petii` 的 mode 2 图片转换质量，消除成片出现的水平或竖直平行短线。这里的“等号噪声”不是单根线太粗，而是两个相邻或跨 tile 的平行笔画组合后看起来像 `=` 或旋转后的 `=`。

模式语义已经明确：

- mode 0：普通图片的最近似 PETSCII 字符匹配。
- mode 1：提取输入中原本已经精确按 PETSCII tile 创作的画。
- mode 2：mode 0 去掉字母和数字；AI 迭代应以 mode 2 为基础。

## 测试素材与命令

固定输入：

```text
tmp/lion-image2/reference.png
```

复现命令：

```bash
cargo run -p petii -- ai "月光下守卫废墟的狮子" \
  --input tmp/lion-image2/reference.png \
  --direct \
  --mode 2 \
  --output-dir tmp/lion-mode2-no-equals-length
```

用户标注图（红箭头指出等号噪声）：

```text
/var/folders/v7/r502vq1j2n53tkfjr8fsxxkm0000gn/T/codex-clipboard-34f26c04-b312-42fe-8def-4d2e97aa5914.png
```

## 已经得到认可的基线

目前肉眼效果较好、可安全回退的版本是：

```text
tmp/lion-mode2-continuity-v7/final.png
tmp/lion-mode2-continuity-v7/final.pix
```

该版本 score 约为 `0.013409`。它已经包含：

- 纯色背景强制使用空格字符。
- 非背景纯色块强制使用实心字符。
- 整图 Sobel 边缘检测及弱边缘连通域清理。
- edge target 的遮罩和轮廓联合距离。
- top-K 候选的跨 tile 边界连续性选择。
- 单 tile 悬空毛刺惩罚。
- 3×3 邻域细小悬空分支惩罚。

用户对这一阶段的评价是“好很多”，但指出画面仍有大量水平、竖直的 `=` 状噪声。

## 本轮“等号噪声”实验

相关实现集中在：

```text
tools/petii/src/converter.rs
```

新增或正在实验的主要函数：

- `equals_noise_penalty`
- `collect_thin_runs`
- `parallel_run_pair_penalty`
- `global_equals_targets`
- `collect_global_parallel_pairs`
- `refine_equals_noise`
- `render_selected_colors`
- `paint_candidate`
- `bitmap_fragmentation_penalty`

### 实验 1：3×3 邻域等号检测

能够识别严格同色、同长度的两条细线，但对真实图中的跨 tile 组合基本无效。真实噪声往往颜色略有不同、长度不齐，或者由多个字符边缘拼成。

### 实验 2：整图二值“线对数量”坐标下降

结果目录：

```text
tmp/lion-mode2-no-equals-global
```

只消除了极少数格子。根因是目标函数只统计线对数量：即使替换让一段长线缩短，只要剩余长度仍超过检测阈值，线对数量就不变，因此替换会被拒绝。

### 实验 3：整图“平行线重叠长度”坐标下降

当前工作区代码处于这一实验状态。变化包括：

- 全局目标由线对个数改为平行线重叠长度。
- 不再只允许修改 Sobel edge cell；所有拥有多个候选的 tile 都可以参与修复。
- 候选与当前字符的距离退化上限为 `0.1`。
- 不允许候选的 bitmap fragmentation 比当前字符更差。
- 只有全局平行线目标下降时才接受替换。

结果目录：

```text
tmp/lion-mode2-no-equals-length
```

命令输出：

```text
grid=40x40, score=0.013444, iterations=0
```

定向测试结果：

```text
15 passed; 0 failed
```

运行命令：

```bash
cargo test -p petii converter::tests::
```

注意：这一实验还不能视为合格结果。Codex 图片预览曾把该目录的 `final.png` 显示成月亮和大量区域消失，但 `final.pix` 与 v7 的 diff 表明月亮相关 tile 没有被修改，且之前也发生过类似预览误导。回家继续时应先用独立渲染/像素 diff 验证 `final.pix` 与 `final.png` 是否一致，再做视觉判断。

## 当前判断

整图逐像素寻找严格线段仍然不是最合适的抽象。最明显的噪声通常来自 tile 级选择：相邻 tile 分别选中带横杠或竖杠的 glyph，拼接后才形成视觉上的 `=`。下一步应优先做 tile 级的笔画签名，而不是继续放宽整图同色线段阈值。

建议方案：

1. 为每个 glyph bitmap 计算水平/竖直笔画签名，例如每行、每列的连续覆盖长度和位置。
2. 在最终 tile 网格上识别相邻行或列中重复出现、间隔较近、方向相同的长笔画组合。
3. 修复时比较候选的局部 3×3 或 5×5 tile 笔画目标，允许逐步缩短噪声，但保护闭合轮廓和实心区域。
4. 对圆形等真实轮廓增加保护：两条线之间若主要是同一前景色实心填充，不应判定为 `=` 噪声。
5. 先只做一轮保守替换，并输出“修复前/后命中数量、被替换坐标和 glyph”，方便针对标注图核验。

还需要特别检查 mode 2 是否应该直接降低某些 glyph 的优先级，例如 bitmap 本身具有两个分离平行笔画的字符；不要简单删除整个 PETSCII 图形子集，以免破坏真正有用的边框字符。

## 性能情况

当前 `refine_equals_noise` 对每个目标 tile 的每个候选都会重新扫描整张 320×320 像素图。40×40 输出一次约多花几十秒，明显过慢。确认算法有效后需要改成局部增量评分，至少只重算候选 tile 周围受影响的行、列或 tile 邻域。

## 测试覆盖

`converter.rs` 当前包含以下相关测试：

- 水平和竖直平行线可以被检测。
- 实心色带不会被误认为平行线噪声。
- checker/碎片字符的 fragmentation 高于连续规则字符。
- 原有边缘清理、连续性、毛刺、纯色块、mode 2 字符过滤测试。

本轮只重新跑了 converter 定向测试。提交前若时间允许，还应执行：

```bash
cargo test -p petii
cargo check -p petii
openspec validate add-ai-petscii-generation-loop --strict
git diff --check
```

## 工作区提醒

当前尚未提交。除 `converter.rs` 外，工作区还包含此前连续开发形成的修改：

```text
openspec/changes/add-ai-petscii-generation-loop/specs/petscii-generation/spec.md
openspec/changes/add-ai-petscii-generation-loop/tasks.md
tools/petii/README.md
tools/petii/src/ai_cli.rs
tools/petii/src/c64.rs
tools/petii/src/main.rs
tools/petii/src/types.rs
```

`tmp/lion-*` 是本地生成结果，通常不会进入 Git。临时的 `tools/petii/examples/inspect_glyphs.rs` 已删除，不要提交调试 example。

如果希望先保留一个质量稳定的提交，建议将当前 `refine_equals_noise` 整图实验单独提交或暂时禁用，把 v7 的边缘连续性版本作为可工作的基线；随后再用独立提交推进 tile 级等号检测。
