## Context

当前 mode 2 流程已经具备 Sobel 边缘图、弱连通域清理、单 cell Top-K、局部双颜色、fill-side mask、glyph edge overlap，以及有限的邻域连续性重排。这些能力解决了明显的黑色平行线和局部毛刺，但优化单位仍以 cell 和小邻域为主，没有显式表示一条轮廓如何穿过多个 cell。

PETSCII 字符集中存在大量为图形构图而设计的字符：水平/竖直笔画、不同高度或宽度的半块、对角线、角、楔形、交叉、条纹和反相版本。人工作者不是逐 cell 独立选择这些字符，而是把它们串成具有入口、出口、方向和填充侧的笔画序列。

对 `apps/petview/assets` 全部 2099 张 40×25 人工作品的初步观察也显示：人工画通常以有限颜色组织语义区域，并在轮廓和纹理处大量使用图形 glyph；当前 60×60 lion 基线则大量退化为纯色/实心 cell。此 change 只解决“边缘语法”，颜色区域化和纹理风格化可在后续独立 change 中处理。

## Goals / Non-Goals

### Goals

- 将 PETSCII glyph 解释为可组合的边缘拓扑原语。
- 从参考图建立跨 cell 的轮廓链，而不是只为每个 cell 保存独立边缘强度。
- 在基线候选附近进行有界、确定性的规则优化。
- 显著减少跨 cell 边缘断裂、错误连接、悬空毛刺和锯齿方向跳变。
- 通过可解释指标、覆盖图和 benchmark 验证改进。
- 从人工 PETSCII 语料提取通用统计，但保持运行时无语料依赖、无场景特判。

### Non-Goals

- 改变 PETSCII 字符集、生成自定义 RGBA tile 或修改 `.pix` 格式。
- 在本 change 中解决全图 palette 量化、色块分割、语义识别或 AI critic/repair loop。
- 逐像素复制人工作品，或训练端到端生成模型。
- 保证所有弱纹理都连续；优化目标优先保护强主体轮廓。
- 强迫所有轮廓闭合；图像边界、遮挡和真实开放笔画允许合法端点。

## Decisions

### Decision: 建立几何派生的 glyph 拓扑目录

对每个允许 glyph 的实际 8×8 bitmap（包括反相语义）预计算 `GlyphTopology`：

- 四条 cell 边界上的一个或多个笔画/填充端口及其位置区间；
- 局部主方向和进入/离开切线；
- bitmap 内部连接分量、端点和分支点数量；
- 前景填充侧、覆盖率和质心；
- 角色标签：blank、solid、straight、corner、diagonal、wedge、junction、texture、text-like。

核心几何数据必须从 bitmap 派生，避免维护脆弱的逐 glyph 特判。少量人工审核的角色覆盖表可以修正几何上歧义的字符，但必须按字符角色说明原因，并同时覆盖正相/反相。

### Decision: 将参考边缘转换为跨 cell 轮廓图

在现有清理后 Sobel map 上进行细化/追踪，得到有序轮廓片段。每个轮廓 cell 记录：

- 轮廓从哪条边进入和离开；
- 端口在边界上的近似位置；
- cell 内主方向与曲率；
- 前景位于轮廓哪一侧；
- 是否为图像边界、遮挡候选、合法端点或 junction。

短小弱分量继续在进入优化器前清理。轮廓图不携带“狮子”“门洞”等对象标签。

### Decision: 联合优化候选格，Top-1 基线不可丢失

优化器的真正输入是逐 cell Top-K 候选格及其局部误差，而不是先固化一张 Top-1 图再做修补。Top-1 转换结果始终作为候选零、质量对照和最终 fallback。对强轮廓 cell，优化器可以从现有 Top-K 及拓扑兼容的图形字符中构造一个有上限的候选集；平坦背景、平坦前景和轮廓带之外的 cell 默认冻结。

候选必须满足：

- 使用允许字符集和 palette；
- 颜色来自该 cell 的基线颜色角色或经验证的前后景交换；
- 与目标边缘方向、填充侧和密度处于配置阈值内；
- 候选数量受固定上限约束。

第一版优先更换 glyph，不进行任意局部重新量化，以便隔离边缘优化的效果。

### Decision: 使用 PETSCII 边缘语法能量函数

优化目标由可解释项组成：

- `reference_loss`：候选 8×8 渲染与参考 cell 的像素/灰度误差；
- `port_mismatch`：相邻 glyph 在共享边界上的端口缺失、错位或多余连接；
- `tangent_discontinuity`：轮廓方向在 cell 边界处发生非参考支持的突变；
- `fill_side_flip`：前景填充侧沿同一轮廓无依据翻转；
- `endpoint_penalty`：非图像边界、非遮挡位置出现悬空端点；
- `spur_penalty`：短分支、往返折线和 1-cell 毛刺；
- `junction_penalty`：参考图无 junction 时出现三向/四向连接；
- `curvature_penalty`：连续轮廓选用的 glyph 序列产生不必要的锯齿或曲率跳变；
- `edit_penalty`：在收益不足时保持基线，限制过度重写。

共享边界比较必须允许小范围端口位置容差，但不能只比较两侧是否“有像素”；方向和填充侧必须参与判定。

### Decision: 先链级动态规划，再做有界局部修复

无 junction 的轮廓链优先使用动态规划或等价的有界序列优化，因为相邻代价可解释且容易保证确定性。junction、相交轮廓和相邻链冲突再使用固定轮数、固定扫描顺序的局部邻域修复。

所有 tie-break 使用稳定的 glyph ID、候选原始排名和坐标顺序。禁止无界全图搜索或依赖随机哈希迭代顺序。

第一版开放链动态规划只使用每个 cell 的前 6 个候选，以限制相邻状态组合；完整 Top-16 候选仍保留给后续局部协调。共享端口允许最多 1 个 bitmap 像素的位置偏差。简单闭环通过枚举首 cell 的 Top-4 状态并计入首尾接缝代价求解；三向/四向 junction 使用固定两轮、正反稳定顺序的入射端口协调，然后进入通用局部修复。链、环和 junction 复用同一份 contour graph。

### Decision: 使用人工语料校准规则，不让语料成为运行时模板库

离线分析 `apps/petview/assets`：

- 各角色 glyph 的使用频率；
- 共享边界上的角色转移和端口匹配分布；
- 合法端点、corner、junction 和反相 glyph 的典型比例；
- 每幅作品的边缘断裂、短毛刺和方向变化基线。

这些统计用于选择默认权重、发现拓扑分类错误并构造测试。运行时优化不得按作品 ID、题材或邻接片段查找并复制具体画面。版本化产物保存聚合统计和选定测试 ID，避免复制整个图库。

### Decision: 以质量门控保证不会为连续性牺牲图像

优化完成后同时计算基线和优化结果：

- 强轮廓端口断裂率；
- 非预期端点率；
- 短毛刺/错误 junction 数；
- 轮廓覆盖率；
- reference reconstruction loss；
- 被修改 cell 数和比例。

只有综合分提高且 reference loss 未超过配置退化门槛时才接受优化结果，否则逐图或逐区域回退到基线。

## Proposed Pipeline

```text
reference image
      |
      v
existing preprocessing + cleaned Sobel map
      |
      +--------------------------+
      |                          |
      v                          v
per-cell Top-K lattice      contour tracing
      |                          |
      v                          v
bounded topology-compatible candidates
      |
      v
chain-level edge grammar optimization
      |
      v
junction/conflict local repair
      |
      v
quality gate against unchanged baseline
      |
      v
validated PetsciiGrid + metrics + debug overlays
```

## Proposed Internal Components

```text
tools/petii/src/
├── converter.rs             pipeline orchestration and fallback
├── glyph_topology.rs        bitmap-derived glyph edge descriptors
├── contour.rs               cleaned edge map to contour chains
├── edge_grammar.rs          candidate expansion and bounded optimizer
├── quality_metrics.rs       continuity and regression metrics
└── preview.rs               result and diagnostic rendering
```

Module boundaries may be combined if the implementation remains small, but glyph topology and quality metrics should stay independently testable.

## Test Strategy

- Glyph-level exhaustive tests over all allowed glyphs and their inverse forms.
- Synthetic line, diagonal, corner, loop, T-junction, occlusion and image-border fixtures.
- Property tests: valid output, bounded candidates, stable tie-break, no edit outside allowed contour band.
- Golden tests for deterministic `.pix`, metrics JSON and diagnostic overlay dimensions.
- Corpus calibration against aggregate `petview` statistics.
- Versioned ordinary-image benchmark comparing unchanged baseline and optimized result.
- Blinded A/B visual review for final quality gate.

## Risks / Trade-offs

- 8×8 bitmap 的边界像素不总能表达感知上的连接。Mitigation: 使用端口区间、切线和小范围容差，并允许审核覆盖。
- 强制连续可能把真实遮挡连接起来。Mitigation: 从参考边缘置信度、填充侧和遮挡候选识别合法端点，并通过 reference loss 门控。
- 独立链可能在 junction 处冲突。Mitigation: 链级优化后执行固定预算的 junction 协调，并允许区域回退。
- 候选扩展可能增加运行时间。Mitigation: 只处理强轮廓带，限制每 cell 候选数、链长度、修复轮数和总预算。
- 人工语料分布可能偏向特定年代或风格。Mitigation: 只用聚合统计校准通用几何规则，最终仍用多题材普通图片和人工盲测验收。

## Migration Plan

1. 在不接入转换路径的情况下实现 glyph 拓扑目录、可视化和测试。
2. 实现轮廓图与合成 fixture，验证端口/方向语义。
3. 以显式配置开关接入 mode 2 基线后的优化阶段。
4. 建立 benchmark 并调整默认权重。
5. 达到质量门槛后将其设为 AI/direct mode 2 的默认后处理，同时保留关闭开关和原基线 fallback。

Rollback consists of disabling the edge grammar stage; baseline candidate generation and current continuity refinement remain available.

## Open Questions

- 第一版是否需要区分双线宽/粗轮廓，还是统一为单个感知中心线加 fill-side。
- junction 协调使用小型动态规划、beam search，还是固定轮数坐标下降。
- 人工审核的 glyph 角色覆盖表应直接维护在 Rust 常量中，还是作为版本化数据文件生成代码。
- 质量门控按整图回退是否足够，还是首版就需要按轮廓链/区域回退。
