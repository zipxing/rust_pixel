# LLM Arena 技术设计文档

## 1. 架构概览

```
┌─────────────────────────────────────────────────────────────────────┐
│                          llm_arena                                   │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   core_sim   │  │  ai_protocol │  │    agents    │              │
│  │ ─────────────│  │ ─────────────│  │ ─────────────│              │
│  │ World        │  │ Observation  │  │ LLMAgent     │              │
│  │ Faction      │◄─┤ Action       │◄─┤ RuleAgent    │              │
│  │ Army         │  │ ReplayLog    │  │ RandomAgent  │              │
│  │ Battle       │  │ Validator    │  │              │              │
│  │ Zone         │  │              │  │              │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│         │                                                           │
│         ▼                                                           │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                          viewer                               │  │
│  │ ─────────────────────────────────────────────────────────────│  │
│  │  WorldMap │ BattleWindow │ ResourcePanel │ Leaderboard │ Feed │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                              │                                      │
│                              ▼                                      │
│                      rust_pixel Engine                              │
└─────────────────────────────────────────────────────────────────────┘
```

## 2. 核心数据结构

### 2.1 世界状态 (World)

```rust
pub struct World {
    pub seed: u64,
    pub tick: u32,
    pub map_size: (u16, u16),           // 256×256
    pub factions: Vec<Faction>,
    pub armies: Vec<Army>,
    pub battles: Vec<BattleInstance>,
    pub resource_points: Vec<ResourcePoint>,
    pub zone: SafeZone,
    pub rng: StdRng,                    // Seeded PRNG
}
```

### 2.2 势力 (Faction)

```rust
pub struct Faction {
    pub id: u8,
    pub name: String,
    pub color: Color,
    pub resources: Resources,
    pub bases: Vec<Base>,
    pub is_alive: bool,
    pub agent_type: AgentType,          // LLM / Rule / Random
}

pub struct Resources {
    pub food: f32,
    pub materials: f32,
    pub ammo: f32,
    pub med: f32,
    pub morale: f32,
    pub pop: u32,
}
```

### 2.3 军团 (Army)

```rust
pub struct Army {
    pub id: u32,
    pub faction_id: u8,
    pub position: (f32, f32),           // 浮点坐标
    pub troops: u32,
    pub supplies: f32,

    // 质量属性
    pub atk: f32,
    pub def: f32,
    pub range_type: RangeType,          // Melee / Ranged / Artillery
    pub speed: f32,
    pub discipline: f32,

    // 状态属性
    pub morale: f32,                    // 0-100
    pub fatigue: f32,                   // 0-100
    pub intent: Intent,                 // Gather / Defend / Attack / Retreat

    // 战术开关
    pub stance: Stance,                 // Aggressive / Balanced / Defensive
    pub focus: Focus,                   // FocusFire / Spread / TargetSupport
    pub retreat_threshold: f32,

    // 状态标记
    pub engaged_lock: u32,              // 遭遇锁定 tick
    pub current_battle: Option<u32>,
}

pub enum RangeType { Melee, Ranged, Artillery }
pub enum Intent { Gather, Defend, Attack, Retreat }
pub enum Stance { Aggressive, Balanced, Defensive }
pub enum Focus { FocusFire, Spread, TargetSupport }
```

### 2.4 战斗实例 (BattleInstance)

```rust
pub struct BattleInstance {
    pub id: u32,
    pub position: (f32, f32),
    pub radius: f32,                    // 危险区半径
    pub participants: Vec<BattleParticipant>,
    pub start_tick: u32,
    pub duration: u32,                  // 预计持续 tick
    pub phase: BattlePhase,
}

pub struct BattleParticipant {
    pub army_id: u32,
    pub faction_id: u8,
    pub initial_troops: u32,
    pub current_troops: u32,
    pub effective_power: f32,           // 有效战力 P
    pub casualties: u32,
}

pub enum BattlePhase { Engaging, Fighting, Resolving, Ended }
```

### 2.5 安全区 (SafeZone)

```rust
pub struct SafeZone {
    pub center: (f32, f32),
    pub radius: f32,
    pub shrink_rate: f32,               // 每 tick 缩小量
    pub next_shrink_tick: u32,
    pub damage_per_tick: f32,           // 圈外伤害
}
```

## 3. 战斗结算公式

### 3.1 有效战力计算

```rust
impl Army {
    pub fn effective_power(&self) -> f32 {
        let q = self.quality_factor();
        self.troops as f32 * q
    }

    fn quality_factor(&self) -> f32 {
        const K_ATK: f32 = 0.05;
        const K_DEF: f32 = 0.03;

        let base = 1.0 + self.atk * K_ATK - self.def * K_DEF;
        let morale_factor = 0.5 + self.morale / 200.0;
        let fatigue_factor = 1.0 - self.fatigue / 150.0;
        let supply_factor = (self.supplies / 100.0).clamp(0.6, 1.1);

        base * morale_factor * fatigue_factor * supply_factor
    }
}
```

### 3.2 战斗伤亡模型

```rust
impl BattleInstance {
    pub fn tick_combat(&mut self, rng: &mut StdRng) {
        let total_power: f32 = self.participants.iter()
            .map(|p| p.effective_power)
            .sum();

        const BASE_INTENSITY: f32 = 0.005;

        for i in 0..self.participants.len() {
            let attacker = &self.participants[i];
            let noise = 0.85 + rng.gen::<f32>() * 0.30;  // [0.85, 1.15]

            for j in 0..self.participants.len() {
                if i == j || self.participants[i].faction_id == self.participants[j].faction_id {
                    continue;
                }

                let loss = (BASE_INTENSITY * attacker.effective_power * noise).round() as u32;
                self.participants[j].current_troops =
                    self.participants[j].current_troops.saturating_sub(loss);
                self.participants[j].casualties += loss;
            }
        }
    }
}
```

### 3.3 维护消耗递增

```rust
impl World {
    pub fn maintenance_multiplier(&self) -> f32 {
        const ALPHA: f32 = 0.8;
        const T0: f32 = 6000.0;  // 10 分钟 = 6000 tick
        const BETA: f32 = 1.2;

        let t = self.tick as f32;
        1.0 + ALPHA * (t / T0).powf(BETA)
    }
}
```

## 4. AI 协议设计

### 4.1 Observation Schema

```json
{
  "meta": {
    "match_id": "string",
    "seed": 12345,
    "tick": 1000,
    "zone_radius": 100.0,
    "sim_speed": 1.0
  },
  "self": {
    "faction_id": 0,
    "resources": {
      "food": 500.0,
      "materials": 200.0,
      "ammo": 150.0,
      "med": 80.0,
      "morale": 75.0,
      "pop": 1000
    },
    "armies": [
      {
        "id": 1,
        "position": [120.5, 80.3],
        "troops": 500,
        "morale": 80.0,
        "intent": "attack"
      }
    ],
    "bases": [...]
  },
  "visible": {
    "enemies": [
      {
        "army_id": 5,
        "faction_id": 1,
        "position": [130.0, 85.0],
        "estimated_troops": 400,
        "last_seen_tick": 990
      }
    ],
    "resource_points": [...],
    "battles": [...]
  },
  "events": [
    {"tick": 980, "type": "battle_start", "data": {...}},
    {"tick": 995, "type": "zone_shrink", "data": {...}}
  ]
}
```

### 4.2 Action Schema

```json
{
  "actions": [
    {
      "type": "move_army",
      "army_id": 1,
      "target": [140.0, 90.0],
      "route_style": "safe"
    },
    {
      "type": "set_engagement_rules",
      "army_id": 1,
      "stance": "aggressive",
      "retreat_threshold": 0.3
    },
    {
      "type": "form_army",
      "base_id": 0,
      "troops": 200,
      "composition": {"melee": 0.6, "ranged": 0.4}
    }
  ]
}
```

## 5. 渲染设计

### 5.0 架构原则：TUI + Sprite 混合

**核心思路**：Widget 写 TUI 框架，Layer/Sprite 写游戏内容

```
┌─────────────────────────────────────────────────────────────────┐
│                        Scene                                     │
├─────────────────────────────────────────────────────────────────┤
│  Layer: "tui" (Widget 系统)                                      │
│  ├─ ResourcePanel (Widget)     ← rust_pixel UI 组件              │
│  ├─ Leaderboard (Widget)       ← 排行榜面板                      │
│  ├─ EventFeed (Widget)         ← 事件流列表                      │
│  └─ ControlBar (Widget)        ← 顶栏控制                        │
├─────────────────────────────────────────────────────────────────┤
│  Layer: "world_map" (Sprite 系统)                                │
│  ├─ Sprite: terrain_bg         ← PETSCII 地形背景                │
│  ├─ Sprite: resource_points[]  ← 资源点 (emoji: ⛏️💎🌾)          │
│  ├─ Sprite: armies[]           ← 军团 (2×2 PETSCII 色块)         │
│  ├─ Sprite: battle_circles[]   ← 战斗锁定圈 (PETSCII 边框)       │
│  └─ Sprite: zone_border        ← 安全区边界                      │
├─────────────────────────────────────────────────────────────────┤
│  Layer: "battle_window" (Sprite 系统，可弹出)                    │
│  ├─ Sprite: battle_bg          ← 战斗场地背景                    │
│  ├─ Sprite: units[]            ← 战斗单位 (PETSCII 点群)         │
│  └─ Sprite: effects[]          ← 特效 (子弹/爆炸)                │
└─────────────────────────────────────────────────────────────────┘
```

### 5.1 大地图渲染 (Layer: "world_map")

**使用 Sprite 系统 + PETSCII/Emoji**

```rust
// 军团 Sprite：2×2 tiles 的阵营色块
// 使用 PETSCII block 字符: █ ▓ ▒ ░
// 阵营颜色：红(1) 蓝(4) 绿(2) 黄(3) 紫(5) 青(6)
// 亮度映射：troops 高用 █，低用 ░

// 资源点 Sprite：使用 emoji
// 食物：🌾 或 🍖
// 矿石：⛏️ 或 💎
// 医疗：💊 或 ❤️
// 弹药：💣 或 🔥

// 战斗区域 Sprite：
// - 锁定圈：PETSCII 边框字符 ┌─┐│└─┘
// - 战斗中心：⚔️ 或 💥
// - 摘要用 TUI 覆盖层

// 安全区边界：
// - 使用 PETSCII 虚线 ┄ ┆
// - 颜色：警告红
```

### 5.2 战斗窗口渲染 (Layer: "battle_window")

**使用 Sprite 系统，弹幕式动画**

```rust
// 尺寸：80×45 tiles
// 窗口框架用 Widget，内容用 Sprite

// 单位 Sprite：PETSCII 字符
// 近战：● ○ ◆ ◇
// 远程：→ ← ↑ ↓
// 炮兵：★ ☆

// 特效 Sprite（严格限额）：
// 子弹：· (抽样 33%)
// 激光：─ │ (每阵营 ≤15)
// 爆炸：💥 或 PETSCII ✹ ✸ (同屏 ≤80)
// 命中：单位闪白 1 帧

// 采样渲染：
//   render_count = clamp(50, 400, troops / 5)
```

### 5.3 UI 面板 (Widget 系统)

**使用 rust_pixel Widget 组件**

```rust
use rust_pixel::ui::{Panel, Label, ProgressBar, List};

// ResourcePanel: 固定位置 Widget
pub struct ResourcePanel {
    panel: Panel,
    labels: Vec<Label>,        // Food: 500 (+10/m)
    bars: Vec<ProgressBar>,    // 资源条
}

// Leaderboard: 固定位置 Widget
pub struct Leaderboard {
    panel: Panel,
    list: List,                // 排名列表
}

// EventFeed: 滚动 Widget
pub struct EventFeed {
    panel: Panel,
    events: Vec<String>,       // 事件日志
    scroll_offset: usize,
}
```

### 5.4 UI 布局

```
┌──────────────────────────────────────────────────────────────────────┐
│ Seed: 12345 │ Tick: 1000 │ Alive: 4/8 │ Zone: 80% │ Speed: 2x │ ⏸ │
├──────────────────────────────────────────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────────────────────────────────────────┐ │
│ │ RESOURCES   │ │                                                 │ │
│ │ Food:   500 │ │              WORLD MAP                          │ │
│ │ Mats:   200 │ │             (256×256)                           │ │
│ │ Ammo:   150 │ │                                                 │ │
│ │ Med:     80 │ │        ██   ⚔   ██                              │ │
│ │ Morale:  75 │ │                                                 │ │
│ ├─────────────┤ │                                                 │ │
│ │ LEADERBOARD │ │                                                 │ │
│ │ 1. Red   42%│ │                                                 │ │
│ │ 2. Blue  35%│ │                                                 │ │
│ │ 3. Green 23%│ │                                                 │ │
│ ├─────────────┤ │                                                 │ │
│ │ EVENT FEED  │ │                                                 │ │
│ │ [980] Battle│ │                                                 │ │
│ │ [995] Zone  │ │                                                 │ │
│ └─────────────┘ └─────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

## 6. 文件结构

```
apps/llm_arena/
├── Cargo.toml
├── src/
│   ├── lib.rs                      # app! 宏
│   ├── main.rs                     # 入口
│   ├── model.rs                    # ArenaModel
│   ├── render_terminal.rs          # 终端渲染
│   └── render_graphics.rs          # 图形渲染
├── core_sim/
│   ├── mod.rs
│   ├── world.rs                    # World, tick()
│   ├── faction.rs                  # Faction, Resources
│   ├── army.rs                     # Army, 属性计算
│   ├── battle.rs                   # BattleInstance, 结算
│   ├── zone.rs                     # SafeZone, 收缩
│   └── encounter.rs                # 相遇检测
├── ai_protocol/
│   ├── mod.rs
│   ├── observation.rs              # Observation JSON
│   ├── action.rs                   # Action JSON + 校验
│   └── replay.rs                   # ReplayLog
├── agents/
│   ├── mod.rs
│   ├── llm_agent.rs                # LLM API 调用
│   ├── rule_agent.rs               # 规则 AI
│   └── random_agent.rs             # 随机 AI
├── viewer/
│   ├── mod.rs
│   ├── world_map.rs                # 大地图渲染
│   ├── battle_window.rs            # 战斗窗口
│   ├── resource_panel.rs           # 资源面板
│   ├── leaderboard.rs              # 排行榜
│   └── event_feed.rs               # 事件流
└── assets/
    ├── pix/                        # 字符/像素资源
    └── config.toml                 # 默认配置
```

## 7. 关键参数默认值

| 参数 | 设计值 | 实际值 | 说明 |
|------|--------|--------|------|
| map_size | 256×256 | 158×126 | 大地图尺寸（适配UI布局） |
| tick_rate | 10/s | 60/s | 仿真帧率 |
| llm_decision_interval | 10 tick | 未实现 | LLM 决策频率 |
| R_encounter | 0.8 tile | 5.0 | 遭遇触发半径 |
| R_release | 1.2 tile | 未实现 | 遭遇解除半径 |
| battle_duration | 80-120 tick | 最大100 | 战斗持续时间 |
| zone_shrink_interval | 600 tick | 500 tick | 缩圈间隔 |

## 8. 实现状态 (2025-03)

### 8.1 已实现功能

**核心仿真 (model.rs)**
- ✅ World 状态管理（地图、势力、军团、资源点、安全区）
- ✅ 军团移动、遭遇检测、战斗实例结算
- ✅ 安全区收缩 + 圈外伤害
- ✅ 资源采集系统（食物、弹药、宝石）
- ✅ 补给消耗 + 饿死机制
- ✅ 宝石自动转化（换粮/换弹）
- ✅ 地形系统（草地、森林、山脉、水域、沙漠、沼泽）
- ✅ seed + PRNG 确保可复现

**渲染层 (render.rs)**
- ✅ TUI + Sprite 混合渲染
- ✅ 地图面板 + 信息面板 + 日志面板 + 状态栏
- ✅ 地形渲染（PETSCII 符号 + 颜色）
- ✅ 军团渲染（阵营色块）
- ✅ 资源点渲染（食物/弹药/宝石/医疗）
- ✅ 战斗标记（⚔️）
- ✅ 视口平移 + 缩放（方向键 + [/]）

**交互控制**
- ✅ 暂停/继续（空格）
- ✅ 速度调节（+/-，1x-10x）
- ✅ 视口平移（方向键）
- ✅ 地图缩放（[/] 或 {/}，0.5x-4.0x）

### 8.2 与设计差异

| 设计项 | 设计 | 实现 | 原因 |
|--------|------|------|------|
| 文件结构 | core_sim/ ai_protocol/ agents/ viewer/ | model.rs render.rs | MVP 简化 |
| Army 属性 | range_type, discipline, fatigue, focus | 缺少 | 简化战斗模型 |
| UI 布局 | 资源面板左侧，地图右侧 | 地图左侧，面板右侧 | 适配显示 |
| AI 协议 | JSON schema | 无 | 待实现 |
| Agents | LLM/Rule/Random | 仅 Random | 待实现 |
| 战斗窗口 | 弹出式详细窗口 | 无 | 待实现 |

### 8.3 新增功能（超出原设计）

- **地形系统**：6 种地形类型，Perlin-like 生成，PETSCII 渲染
- **地图缩放**：0.5x-4.0x 缩放支持
- **宝石资源**：第三种资源类型，可自动转化为食物或弹药

### 8.4 待实现

- [ ] AI 协议层（Observation/Action JSON schema）
- [ ] LLM Agent / Rule Agent
- [ ] 战斗详情窗口
- [ ] 回放系统
- [ ] 联赛模式
