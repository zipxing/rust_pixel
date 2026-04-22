## Context

rust_pixel 当前围绕 2D tile/TUI 渲染构建，核心热路径是 `Scene > Layer > Sprite > Buffer > Cell`，图形模式通过 WGPU 纹理数组渲染 tile。该架构不适合直接承载块体素世界，因为 3D 渲染的主数据结构应是 block/chunk/mesh，而不是逐 cell 组合。

同时，项目已经具备复用价值很高的公共基础设施：

- `event` / `timer` / `context` / `game loop`
- WGPU 后端与纹理数组上传
- LayeredSymbolMap 与 tile UV/layer 表达
- PUA 编码体系与资产工具链

目标是在不破坏 2D/TUI 心智模型的前提下，新增一个独立的 3D 模式，并复用上述公共基础设施。

## Goals / Non-Goals

- Goals:
  - 增加独立的 `3d` 运行模式
  - 支持 Minecraft 风格的块体素展示与相机移动
  - 复用事件、消息、Timer、Context、资产和 WGPU 基础设施
  - 建立 `cube face -> PUA -> symbol -> tile` 的资产映射链
  - 保持现有 2D/TUI 模式兼容

- Non-Goals:
  - 不在第一阶段实现 2D/3D 混合渲染
  - 不在第一阶段实现 Web 3D
  - 不在第一阶段实现体素物理、流体、骨骼、体积光等复杂效果
  - 不把 3D 数据强塞进 `Scene/Layer/Sprite/Buffer/Cell` 热路径

## Decisions

- Decision: 3D 模式采用独立渲染管线，而不是修改现有 2D 管线
  - Why: 2D 管线以 cell/tile 为中心，3D 管线需要 chunk mesh、深度测试、相机矩阵和面剔除
  - Alternatives considered:
    - 复用 `Scene/Layer/Sprite` 承载 3D 面片：实现复杂且会污染 2D 抽象
    - 在现有 `draw_all_graph()` 内分支出 3D 逻辑：短期可做，但长期可维护性差

- Decision: 3D 模式作为独立运行模式 `cargo pixel r <app> 3d`
  - Why: 降低对现有 app 和 CLI 语义的扰动
  - Alternatives considered:
    - 复用 `g` 模式并在 app 内切换：会混淆 2D 与 3D 运行时契约

- Decision: 运行时复用公共模块，渲染数据结构分离
  - Why: 事件/消息/Timer/Context 已经成熟，重复造轮子没有收益
  - Reuse boundary:
    - Reuse: `event`, `timer`, `context`, `game loop`, 资产加载框架, WGPU 设备/队列/纹理数组
    - Separate: `VoxelWorld`, `Chunk`, `VoxelMesh`, `Camera`, `VoxelRenderer`

- Decision: 体素贴图使用 `cube face -> PUA -> symbol -> tile` 设计
  - Why: 与现有 PUA/tile 体系一致，同时保留 symbol 的可读性和编辑器友好性
  - Runtime rule:
    - 配置与编辑器可以经由 `PUA <-> symbol` 进行解析与显示
    - 热路径渲染应缓存为 face material/tile 引用，避免每帧字符串查找

- Decision: PUA 统一使用 Supplementary PUA-A 的 sprite 编码域
  - Why: 与现有 sprite 语义一致，便于序列化与工具链共享
  - Constraint:
    - 仅真实 sprite/voxel 面材质占用 PUA sprite block
    - TUI/emoji/CJK 的逻辑区间不得直接混用为 voxel 面 PUA

- Decision: 材质定义支持 `all`、`top/bottom/side`、six-face 三种粒度
  - Why: 满足石头、草方块、箱子/机器等常见体素方块需求

## Module Layout

### Engine-side modules

```text
src/render/
├── adapter.rs
├── adapter/
│   └── ... existing adapters ...
└── voxel/
    ├── mod.rs
    ├── world.rs
    ├── mesh.rs
    ├── camera.rs
    ├── material.rs
    ├── atlas.rs
    └── renderer.rs
```

- `world.rs`
  - `BlockId`, `ChunkCoord`, `Chunk`, `VoxelWorld`
  - 负责方块存储、chunk 查询、面可见性所需邻接访问
- `mesh.rs`
  - `VoxelVertex`, `FaceDir`, `ChunkMesh`
  - 负责从 chunk 数据生成可见面网格
- `camera.rs`
  - 相机位置、朝向、投影矩阵、基础输入更新
- `material.rs`
  - `VoxelMaterialDef`, `VoxelMaterial`, `VoxelMaterialRegistry`
  - 支持 `all`、`top/bottom/side`、six-face
- `atlas.rs`
  - `VoxelFaceRef`, `VoxelAtlasResolver`
  - 管理 `PUA -> symbol -> tile` 解析与缓存
- `renderer.rs`
  - `VoxelRenderer`
  - 初始化 pipeline、depth buffer、uniform、chunk mesh draw flow

### Runtime integration points

- `src/render/mod.rs` 或等效导出层
  - 导出 `voxel` 模块
- `src/render/adapter.rs`
  - 为 `3d` 模式定义统一入口，不污染现有 2D draw hot path
- `src/context.rs`
  - 复用输入事件、Timer、AssetManager
  - 为 3D 模式暴露相机/世界更新所需共享上下文
- `src/game.rs`
  - 保持现有 game loop
  - 在 render 初始化与 draw 阶段允许 3D 模式分支

### Demo application

```text
apps/voxel_demo/
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── model.rs
│   └── render.rs
└── assets/
    └── ... voxel materials / textures ...
```

- `model.rs`
  - 相机状态、基础世界生成、输入驱动移动
- `render.rs`
  - 连接 voxel renderer 与 runtime
  - 不复用 `Scene/Layer/Sprite` 作为主渲染数据

## Implementation Phases

### Phase 1: Runtime Skeleton

- CLI 增加 `3d` 模式解析
- app 运行时增加 3D 模式初始化分支
- 新增 `src/render/voxel/mod.rs` 与空骨架模块

### Phase 2: Minimal Voxel MVP

- 单 chunk 或小型 chunk grid
- 单色或单贴图 block
- 相机移动、深度测试、可见面渲染
- `apps/voxel_demo` 可启动

### Phase 3: Material / PUA / Tile Integration

- 引入 `VoxelMaterialRegistry`
- 接入 `cube face -> PUA -> symbol -> tile`
- 缓存为 runtime tile/material 引用，避免热路径字符串查找

### Phase 4: Optimization and Tooling

- chunk dirty rebuild
- greedy meshing 或等价面合并
- 更明确的 block allocation / tooling validation
- 3D app template 与后续 Web 支持评估

## Risks / Trade-offs

- 风险: PUA block 分配与现有 sprite 区发生冲突
  - Mitigation: 在 proposal/spec 中要求明确保留 block 规划与工具链校验

- 风险: 运行时同时维护 `symbol_map`、`layered_symbol_map`、voxel material map，造成真相源分裂
  - Mitigation: 明确一个单一生成源，其他运行时索引均由该源派生

- 风险: 过度复用 2D 接口导致 3D 管线变形
  - Mitigation: 只复用公共运行时，不复用 2D cell 热路径

- 风险: 首阶段过早追求复杂效果，推迟 MVP 落地
  - Mitigation: 先以 Minecraft 风格块体素为目标，只做足够展示与交互精度

## Migration Plan

1. 先引入 spec 与 CLI 模式入口，不改现有 app 默认模式
2. 增加 3D 运行时最小骨架与 voxel renderer
3. 增加最小材质/PUA/tile 资产链路
4. 使用独立 demo app 或独立模式验证
5. 后续再迭代 chunk 优化、工具链、编辑器与 Web 支持

## Open Questions

- `3d` 模式是否需要一个单独 app 模板（如 `cargo pixel c my_voxel --3d`）
- 体素材质的单一真相源应放在 `symbol_map.json` 扩展字段，还是独立 `voxel_materials.json`
- block 区间规划是否要在第一阶段固定写死，还是交由工具链动态分配
