## Why

rust_pixel 现有能力集中在 tile-first 的 2D/TUI 渲染，但项目已经具备跨平台事件、计时器、上下文、资产管理与 WGPU 纹理数组等基础设施，适合增量扩展一个独立的 3D 体素模式。新增体素模式可以：

1. 扩展引擎能力边界，在不破坏 2D/TUI 模型的前提下支持 Minecraft 风格展示与交互
2. 复用现有公共模块（事件、消息、Timer、Context、Adapter 调度）而不是重新搭建运行时
3. 复用现有 tile / atlas / PUA 资产体系，降低 3D 资产管线成本
4. 为后续 3D 编辑器、体素游戏和 2D/3D 混合工具提供基础设施

## What Changes

### 新增独立的 `3d` 运行模式

- **ADDED** `cargo pixel r <app> 3d` 模式，作为独立于现有 `term` / `g` / `w` 的新运行入口
- **ADDED** 3D 专用渲染子系统，采用 Minecraft 风格的块体素展示精度
- **ADDED** 3D 模式专用应用接口与适配器扩展，保持与现有 Game/Event/Timer/Context 运行时兼容

### 新增体素渲染与世界表达

- **ADDED** 体素世界数据结构（Block、Chunk、ChunkMesh、Camera）
- **ADDED** 仅渲染可见面的块体素网格构建流程
- **ADDED** 面剔除、深度测试、基础相机控制与块体素材质系统

### 明确第一阶段模块落点

- **ADDED** `src/render/voxel/`
  - `mod.rs`：3D 渲染模块导出
  - `world.rs`：`BlockId`、`VoxelWorld`、`ChunkCoord`、`Chunk`
  - `mesh.rs`：可见面提取、面顶点结构、ChunkMesh 构建
  - `camera.rs`：第一人称/自由相机、view/projection 计算
  - `material.rs`：`VoxelMaterial`、面材质粒度（all / tbs / six-face）
  - `atlas.rs`：`PUA -> symbol -> tile` 运行时索引与缓存
  - `renderer.rs`：WGPU voxel pipeline、depth buffer、draw flow
- **ADDED** `src/render/adapter/` 扩展点
  - 保留现有 2D adapter 入口
  - 为 `3d` 模式增加 voxel renderer 初始化与 per-frame 渲染入口
- **ADDED** `apps/voxel_demo/`
  - 用作 Minecraft 风格 MVP 验证 app
  - 只承担演示与回归，不影响现有 app

### 明确 CLI 与运行时接入点

- **ADDED** `tools/cargo-pixel/` 中的 `3d` build/run 解析
- **ADDED** 应用宏或运行时初始化层中的 3D 模式选择分支
- **ADDED** 3D 模式对现有消息、事件、Timer、Context 的复用约束

### 新增基于 PUA 的体素贴图资产链路

- **ADDED** `cube face -> PUA -> symbol -> tile` 的映射设计
- **ADDED** 使用 Supplementary PUA-A 作为体素面贴图的统一编码域
- **ADDED** 体素材质/方块定义，支持 `all`、`top/bottom/side` 与六面独立贴图
- **ADDED** 运行时将 PUA 解析为 atlas tile，配置和编辑器层保留 `symbol` 作为可读别名

### 明确复用与隔离边界

- **ADDED** 复用 `event`、`timer`、`context`、`game loop`、资产加载框架与 WGPU 纹理数组
- **ADDED** 3D 渲染数据结构与现有 `Scene/Layer/Sprite/Buffer/Cell` 并行存在，而非强行复用 2D Cell 渲染管线
- **ADDED** 保持现有 2D/TUI API 兼容，不修改既有应用默认行为

## Impact

- Affected specs: `voxel-rendering`
- Affected code:
  - `src/render/`（新增 3D/voxel 子系统）
  - `src/context.rs` / `src/game.rs` / `src/render/adapter.rs`（运行时与模式接入）
  - `tools/cargo-pixel/`（CLI 新模式）
  - `apps/voxel_demo/`（MVP 验证应用）
  - 资产与符号映射工具链（PUA/tile/material 映射）
- Compatibility:
  - 对现有 `term` / `g` / `w` 模式保持增量兼容
  - 不要求现有 2D 应用迁移到 3D 模式
