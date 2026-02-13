## Why

rust_pixel 目前拥有完善的渲染引擎、UI 框架和游戏开发工具链，但缺乏一个能够展示其全部能力的 **旗舰级应用**。同时，LLM 竞技类游戏是当前 AI 领域的热点，将 LLM 与像素风格策略游戏结合，可以：

1. **展示 rust_pixel 能力**：大地图、多实体、战斗动画、TUI 仪表盘、回放系统
2. **创造独特卖点**：让不同大模型在同一规则下竞技，谁活到最后谁赢
3. **形成内容生态**：可复盘、可联赛、可直播，产生持续的观看价值
4. **验证引擎性能**：大量单位、多层渲染、实时战斗对引擎是极好的压力测试

## What Changes

### 新增应用 `apps/llm_arena`

- **ADDED** `core_sim/` — 纯逻辑核心仿真，deterministic，无渲染依赖
  - 世界状态管理（大地图、势力、军团、资源点）
  - 军团移动、相遇检测、战斗实例结算
  - 反龟缩机制（安全区收缩、资源衰减、维护递增）
  - seed + PRNG 确保可复现

- **ADDED** `ai_protocol/` — LLM 通信协议层
  - Observation JSON schema（限视距 + 情报衰减）
  - Action JSON schema（8-10 类动作 + 校验）
  - Replay log 格式定义

- **ADDED** `agents/` — AI 代理实现
  - LLM Agent（调用外部 API）
  - Rule Agent（规则 AI，baseline）
  - Random Agent（随机策略，测试用）

- **ADDED** `viewer/` — rust_pixel UI 层
  - 大地图渲染（军团色块、战斗锁定圈、摘要标签）
  - 战斗窗口（80×45 tiles，弹幕式动画）
  - 资源面板、排行榜、事件流
  - 回放/倍速/暂停控制

- **ADDED** `league_cli/`（可选）— 批量跑局与联赛统计

### 核心设计原则

- **可观战**：信息密度高、战斗清晰、可倍速/暂停/回放
- **可竞技**：信息对称、限视距、统一调用预算、可复盘可仲裁
- **低美术成本**：颜色=阵营，几何=兵种，特效只做点/线/环
- **高性能**：大地图抽象、局部战斗高密度；采用限额与抽样渲染
- **可迭代**：core_sim 与 UI 解耦，能批量跑局做联赛

## Impact

- **新增 app**: `apps/llm_arena/`（独立应用，不影响现有代码）
- **可选依赖**: `reqwest`（LLM API 调用）、`serde_json`（协议序列化）
- **复用现有**: Scene/Layer/Sprite、Buffer/Cell、Timer/Event 系统
- **不破坏现有 API**: 纯增量开发

## Success Criteria

1. MVP-0：2 势力对战，战斗可观看，seed 可复现
2. MVP-1：4-8 势力，多模型接入，统一预算，回放导出
3. MVP-2：联赛化，批量跑局，积分系统，战报生成
