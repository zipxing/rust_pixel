# core_sim 规格说明

## 概述

`core_sim` 是 LLM Arena 的核心仿真模块，实现纯逻辑的、确定性的游戏规则引擎。该模块不依赖任何渲染代码，可独立运行和测试。

## 设计原则

1. **Deterministic**: 给定相同的 seed 和输入序列，必须产生完全相同的结果
2. **No Rendering**: 纯逻辑模块，不包含任何 UI 或渲染代码
3. **Testable**: 所有公开方法都应该可以单元测试
4. **Serializable**: 所有状态都可以序列化/反序列化

## 模块结构

```
core_sim/
├── mod.rs          # 公开接口
├── world.rs        # World 状态和 tick 逻辑
├── faction.rs      # Faction 和 Resources
├── army.rs         # Army 属性和计算
├── battle.rs       # BattleInstance 结算
├── zone.rs         # SafeZone 收缩
└── encounter.rs    # 遭遇检测逻辑
```

## 核心类型

### World

世界状态的根容器，负责管理所有实体和推进仿真。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub seed: u64,
    pub tick: u32,
    pub map_size: (u16, u16),
    pub factions: Vec<Faction>,
    pub armies: Vec<Army>,
    pub battles: Vec<BattleInstance>,
    pub resource_points: Vec<ResourcePoint>,
    pub zone: SafeZone,
    #[serde(skip)]
    rng: StdRng,
}

impl World {
    /// 创建新世界
    pub fn new(seed: u64, config: WorldConfig) -> Self;

    /// 推进一个 tick
    pub fn tick(&mut self);

    /// 应用一个势力的动作
    pub fn apply_action(&mut self, faction_id: u8, action: Action) -> Result<(), ActionError>;

    /// 生成势力的观察数据
    pub fn generate_observation(&self, faction_id: u8) -> Observation;

    /// 获取当前维护乘数
    pub fn maintenance_multiplier(&self) -> f32;

    /// 检查游戏是否结束
    pub fn is_game_over(&self) -> Option<GameResult>;
}
```

### Faction

势力代表一个玩家或 AI 控制的阵营。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub id: u8,
    pub name: String,
    pub color: u8,  // ANSI color index
    pub resources: Resources,
    pub bases: Vec<Base>,
    pub is_alive: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Resources {
    pub food: f32,
    pub materials: f32,
    pub ammo: f32,
    pub med: f32,
    pub morale: f32,
    pub pop: u32,
}

impl Resources {
    /// 应用每 tick 的消耗
    pub fn apply_consumption(&mut self, consumption: &Resources);

    /// 检查是否有崩盘状态
    pub fn check_collapse(&self) -> Vec<CollapseType>;
}
```

### Army

军团是地图上可移动和战斗的单位。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Army {
    pub id: u32,
    pub faction_id: u8,
    pub position: (f32, f32),
    pub target: Option<(f32, f32)>,

    // 规模
    pub troops: u32,
    pub supplies: f32,

    // 质量
    pub atk: f32,
    pub def: f32,
    pub range_type: RangeType,
    pub speed: f32,
    pub discipline: f32,

    // 状态
    pub morale: f32,
    pub fatigue: f32,
    pub intent: Intent,

    // 战术
    pub stance: Stance,
    pub focus: Focus,
    pub retreat_threshold: f32,

    // 战斗状态
    pub engaged_lock: u32,
    pub current_battle: Option<u32>,
}

impl Army {
    /// 计算有效战力
    pub fn effective_power(&self) -> f32 {
        let q = self.quality_factor();
        self.troops as f32 * q
    }

    /// 计算质量因子
    pub fn quality_factor(&self) -> f32 {
        const K_ATK: f32 = 0.05;
        const K_DEF: f32 = 0.03;

        let base = 1.0 + self.atk * K_ATK - self.def * K_DEF;
        let morale_factor = 0.5 + self.morale / 200.0;
        let fatigue_factor = 1.0 - self.fatigue / 150.0;
        let supply_factor = (self.supplies / 100.0).clamp(0.6, 1.1);

        base * morale_factor * fatigue_factor * supply_factor
    }

    /// 移动一个 tick
    pub fn move_tick(&mut self);

    /// 检查是否应该撤退
    pub fn should_retreat(&self) -> bool;
}
```

### BattleInstance

战斗实例管理一场战斗的全部状态。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleInstance {
    pub id: u32,
    pub position: (f32, f32),
    pub radius: f32,
    pub participants: Vec<BattleParticipant>,
    pub start_tick: u32,
    pub duration: u32,
    pub phase: BattlePhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleParticipant {
    pub army_id: u32,
    pub faction_id: u8,
    pub initial_troops: u32,
    pub current_troops: u32,
    pub effective_power: f32,
    pub casualties: u32,
}

impl BattleInstance {
    /// 执行一个 tick 的战斗结算
    pub fn tick_combat(&mut self, rng: &mut StdRng);

    /// 检查战斗是否结束
    pub fn check_end_condition(&self) -> bool;

    /// 生成战斗结果
    pub fn generate_result(&self) -> BattleResult;
}
```

## 遭遇检测算法

```rust
pub fn detect_encounters(armies: &[Army], r_encounter: f32) -> Vec<(u32, u32)> {
    let mut encounters = Vec::new();

    for i in 0..armies.len() {
        for j in (i + 1)..armies.len() {
            let a = &armies[i];
            let b = &armies[j];

            // 跳过同阵营
            if a.faction_id == b.faction_id {
                continue;
            }

            // 跳过已锁定
            if a.engaged_lock > 0 || b.engaged_lock > 0 {
                continue;
            }

            // 计算距离
            let dx = a.position.0 - b.position.0;
            let dy = a.position.1 - b.position.1;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= r_encounter {
                encounters.push((a.id, b.id));
            }
        }
    }

    encounters
}
```

## 维护递增公式

```rust
/// 计算当前 tick 的维护乘数
/// M(t) = 1 + alpha * (t / T0)^beta
pub fn maintenance_multiplier(tick: u32) -> f32 {
    const ALPHA: f32 = 0.8;
    const T0: f32 = 6000.0;  // 10 分钟
    const BETA: f32 = 1.2;

    let t = tick as f32;
    1.0 + ALPHA * (t / T0).powf(BETA)
}
```

## 测试用例

### 确定性验证

```rust
#[test]
fn test_deterministic() {
    let config = WorldConfig::default();

    let mut world1 = World::new(12345, config.clone());
    let mut world2 = World::new(12345, config.clone());

    for _ in 0..1000 {
        world1.tick();
        world2.tick();
    }

    assert_eq!(world1.tick, world2.tick);
    assert_eq!(world1.factions.len(), world2.factions.len());
    assert_eq!(world1.armies.len(), world2.armies.len());
    // ... 验证所有状态相等
}
```

### 战斗结算验证

```rust
#[test]
fn test_battle_casualties() {
    let mut battle = BattleInstance::new(/* ... */);
    battle.participants.push(BattleParticipant {
        army_id: 1,
        faction_id: 0,
        initial_troops: 1000,
        current_troops: 1000,
        effective_power: 1000.0,
        casualties: 0,
    });
    battle.participants.push(BattleParticipant {
        army_id: 2,
        faction_id: 1,
        initial_troops: 1000,
        current_troops: 1000,
        effective_power: 1000.0,
        casualties: 0,
    });

    let mut rng = StdRng::seed_from_u64(12345);

    for _ in 0..100 {
        battle.tick_combat(&mut rng);
    }

    // 验证双方都有伤亡
    assert!(battle.participants[0].casualties > 0);
    assert!(battle.participants[1].casualties > 0);
}
```
