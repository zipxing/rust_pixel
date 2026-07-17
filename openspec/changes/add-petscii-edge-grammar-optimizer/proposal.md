## Why

`petii` 已能为每个 cell 生成局部近似 glyph，并对强边缘候选加入邻接评分，但结果仍容易呈现为彼此独立的色块或局部最优字符。高质量 PETSCII 艺术的关键差异是：作者把字符集中的水平线、竖线、斜线、角、楔形、半块和连接字符当作一套跨 cell 的边缘语言，使轮廓在多个 cell 间保持方向、填充侧和笔画连续。

需要在单字符近似基线之后增加一个独立、确定性、有界的“PETSCII 边缘语法优化”阶段，将局部候选组织成接近人工绘制的连续轮廓，同时保留固定字符集、调色板和 `.pix` 格式约束。

## What Changes

- 为固定 PETSCII 字符集生成可检查的 glyph 边缘拓扑目录，包括边界端口、方向、端点、连接分量、填充侧、密度和角色分类。
- 从参考图的清理后边缘图中提取跨 cell 轮廓链和每个 cell 的目标边缘拓扑。
- 将现有逐 cell Top-K 候选格作为局部图像证据，在轮廓带内扩展有界的图形 glyph 候选；Top-1 图只作为对照和回退基线。
- 以端口连接、方向延续、曲率、填充侧一致、分支/毛刺抑制和参考图误差组成规则化目标函数。
- 使用确定性、有预算的链级和局部邻域优化重排候选；只有通过质量门槛的结果才能替换基线。
- 增加边缘断裂、悬空端点、短毛刺、轮廓覆盖和参考重建损失等可解释指标与调试覆盖图。
- 使用 `apps/petview/assets` 的人工 PETSCII 作品统计 glyph 角色和合法邻接模式，用于规则校准、测试和评估，不在运行时复制具体作品或按场景特判。
- 建立包含不同题材、尺度和边缘方向的版本化 benchmark，并以确定性指标和盲测评价优化效果。

## Impact

- Affected specs: new `petscii-edge-optimization` capability.
- Affected code: `tools/petii/src/converter.rs`，并可能新增独立的 glyph 拓扑、轮廓链、优化器和指标模块。
- Affected assets/tests: 从 `apps/petview/assets` 选择版本化的统计/结构测试样本；新增普通图片 reference benchmark。
- No external service or model dependency.
- No change to the `.pix` format, PETSCII glyph bitmap, palette definition, or RustPixel rendering APIs.
- Existing direct conversion remains the baseline and fallback path.

## Success Criteria

1. 输出仍只包含允许的 PETSCII glyph 和 palette index，并通过 `PetsciiGrid` 验证。
2. 在版本化 benchmark 上，优化结果的强轮廓边界断裂率和非预期悬空端点率相对基线的中位数均至少降低 30%。
3. 优化结果的参考图重建损失相对基线不得恶化超过 5%，且任何单例超过门槛时保留基线。
4. 相同输入、配置和字符集产生逐字节一致的 `.pix`。
5. 在盲测中，至少 70% 的优化结果胜出或与逐 cell 基线持平。
6. 实现不包含狮子、月亮、人物、建筑等场景/对象特判，也不通过无限追加 glyph 黑名单实现质量提升。
