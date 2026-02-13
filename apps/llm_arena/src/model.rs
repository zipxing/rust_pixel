use log::info;
use rust_pixel::context::Context;
use rust_pixel::event::{Event, KeyCode};
use rust_pixel::game::Model;
use rand::prelude::*;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

// Map dimensions
pub const MAP_WIDTH: u16 = 200;
pub const MAP_HEIGHT: u16 = 100;

// Screen dimensions
pub const SCREEN_WIDTH: u16 = 120;
pub const SCREEN_HEIGHT: u16 = 45;

// Encounter radius
pub const R_ENCOUNTER: f32 = 5.0;

// Resource system constants
pub const R_GATHER: f32 = 3.0;           // 采集半径
pub const SUPPLY_CONSUME_RATE: f32 = 0.001;  // 每tick每兵消耗补给
pub const GATHER_RATE: f32 = 2.0;        // 每tick采集量
pub const STARVE_RATE: f32 = 0.005;      // 补给耗尽时每tick饿死比例
pub const MORALE_STARVE_DROP: f32 = 0.5; // 饿死时士气下降
pub const RESOURCE_MAX: f32 = 500.0;     // 资源点最大容量
pub const AMMO_CONSUME_RATE: f32 = 0.5;  // 战斗中每tick弹药消耗
pub const NO_AMMO_PENALTY: f32 = 0.5;    // 无弹药时伤害降低比例

// 宝石系统常量
pub const GEMS_TO_FOOD_RATIO: f32 = 2.0;  // 1宝石 = 2食物
pub const GEMS_TO_AMMO_RATIO: f32 = 2.0;  // 1宝石 = 2弹药
pub const GEMS_CONVERT_RATE: f32 = 1.0;   // 每tick自动转化的宝石量

/// Game state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaState {
    Init,
    Running,
    Paused,
    GameOver,
}

/// Resource types for each faction
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Resources {
    pub food: f32,
    pub materials: f32,
    pub ammo: f32,
    pub med: f32,
    pub morale: f32,
    pub pop: u32,
}

/// A faction in the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub id: u8,
    pub name: String,
    pub color: u8,
    pub resources: Resources,
    pub is_alive: bool,
}

/// Army stance in battle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stance {
    Aggressive,
    Defensive,
    Balanced,
}

/// Army intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    Move,
    Attack,
    Defend,
    Retreat,
}

/// An army unit on the map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Army {
    pub id: u32,
    pub faction_id: u8,
    pub position: (f32, f32),
    pub target: Option<(f32, f32)>,
    pub troops: u32,
    pub supplies: f32,    // 食物补给 (0-150)
    pub ammo: f32,        // 弹药 (0-100)
    pub gems: f32,        // 宝石 (0-100) - 可转化为食物或弹药
    pub atk: f32,
    pub def: f32,
    pub speed: f32,
    pub morale: f32,
    pub fatigue: f32,
    pub stance: Stance,
    pub intent: Intent,
    pub retreat_threshold: f32,
    pub engaged_lock: u32,
    pub current_battle: Option<u32>,
}

impl Army {
    pub fn new(id: u32, faction_id: u8, x: f32, y: f32, troops: u32) -> Self {
        Self {
            id,
            faction_id,
            position: (x, y),
            target: None,
            troops,
            supplies: 100.0,
            ammo: 100.0,
            gems: 0.0,        // 宝石初始为0，需要采集
            atk: 10.0,
            def: 10.0,
            speed: 1.0,
            morale: 100.0,
            fatigue: 0.0,
            stance: Stance::Balanced,
            intent: Intent::Move,
            retreat_threshold: 0.3,
            engaged_lock: 0,
            current_battle: None,
        }
    }

    /// Calculate effective power
    pub fn effective_power(&self) -> f32 {
        let q = self.quality_factor();
        self.troops as f32 * q
    }

    /// Calculate quality factor
    pub fn quality_factor(&self) -> f32 {
        const K_ATK: f32 = 0.05;
        const K_DEF: f32 = 0.03;

        let base = 1.0 + self.atk * K_ATK - self.def * K_DEF;
        let morale_factor = 0.5 + self.morale / 200.0;
        let fatigue_factor = 1.0 - self.fatigue / 150.0;
        let supply_factor = (self.supplies / 100.0).clamp(0.6, 1.1);

        base * morale_factor * fatigue_factor * supply_factor
    }

    /// Move towards target for one tick
    pub fn move_tick(&mut self) -> bool {
        if let Some(target) = self.target {
            let dx = target.0 - self.position.0;
            let dy = target.1 - self.position.1;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > self.speed {
                self.position.0 += dx / dist * self.speed;
                self.position.1 += dy / dist * self.speed;
                false
            } else {
                self.position = target;
                self.target = None;
                true // 到达目标
            }
        } else {
            false
        }
    }
}

/// A battle participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleParticipant {
    pub army_id: u32,
    pub faction_id: u8,
    pub initial_troops: u32,
    pub current_troops: u32,
    pub effective_power: f32,
    pub casualties: u32,
}

/// Battle phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattlePhase {
    Engaging,
    Combat,
    Ended,
}

/// A battle instance
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

impl BattleInstance {
    pub fn new(id: u32, position: (f32, f32)) -> Self {
        Self {
            id,
            position,
            radius: R_ENCOUNTER,
            participants: Vec::new(),
            start_tick: 0,
            duration: 0,
            phase: BattlePhase::Engaging,
        }
    }
}

/// Safe zone - shrinks over time to force engagement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeZone {
    pub center: (f32, f32),
    pub radius: f32,
    pub shrink_rate: f32,   // 每tick收缩量
    pub min_radius: f32,    // 最小半径
    pub damage_rate: f32,   // 圈外伤害率
}

impl SafeZone {
    pub fn new(center: (f32, f32), radius: f32) -> Self {
        Self {
            center,
            radius,
            shrink_rate: 0.0,    // 默认不收缩，后续可调
            min_radius: 20.0,
            damage_rate: 0.0,    // 默认无伤害，后续可调
        }
    }

    pub fn tick(&mut self) {
        if self.radius > self.min_radius && self.shrink_rate > 0.0 {
            self.radius -= self.shrink_rate;
            if self.radius < self.min_radius {
                self.radius = self.min_radius;
            }
        }
    }

    pub fn is_inside(&self, pos: (f32, f32)) -> bool {
        let dx = pos.0 - self.center.0;
        let dy = pos.1 - self.center.1;
        (dx * dx + dy * dy).sqrt() <= self.radius
    }
}

/// Resource point on the map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePoint {
    pub position: (f32, f32),
    pub resource_type: u8, // 0=food, 1=gems, 2=ammo, 3=med
    pub amount: f32,
    pub regen_rate: f32,
}

fn default_rng() -> StdRng {
    StdRng::seed_from_u64(0)
}

/// World state containing all game entities
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
    #[serde(skip, default = "default_rng")]
    pub rng: StdRng,
}

impl World {
    pub fn new(seed: u64, num_factions: u8) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let map_size = (MAP_WIDTH, MAP_HEIGHT);
        let center = (map_size.0 as f32 / 2.0, map_size.1 as f32 / 2.0);

        // Create factions
        let colors = [1, 2, 3, 4, 5, 6, 9, 10]; // ANSI colors
        let names = ["Red", "Blue", "Green", "Yellow", "Magenta", "Cyan", "Orange", "Purple"];
        let mut factions = Vec::new();

        for i in 0..num_factions {
            factions.push(Faction {
                id: i,
                name: names[i as usize % names.len()].to_string(),
                color: colors[i as usize % colors.len()],
                resources: Resources {
                    food: 1000.0,
                    materials: 500.0,
                    ammo: 300.0,
                    med: 200.0,
                    morale: 100.0,
                    pop: 1000,
                },
                is_alive: true,
            });
        }

        // Create initial armies
        let mut armies = Vec::new();
        let mut army_id = 0u32;
        for i in 0..num_factions {
            // Each faction starts with 2 armies
            let angle = (i as f32) * std::f32::consts::TAU / (num_factions as f32);
            let spawn_radius = center.0.min(center.1) * 0.6;
            let x = center.0 + angle.cos() * spawn_radius;
            let y = center.1 + angle.sin() * spawn_radius;

            armies.push(Army::new(army_id, i, x, y, 500));
            army_id += 1;

            let x2 = x + rng.random_range(-10.0..10.0);
            let y2 = y + rng.random_range(-10.0..10.0);
            armies.push(Army::new(army_id, i, x2, y2, 300));
            army_id += 1;
        }

        // Create resource points
        let mut resource_points = Vec::new();
        for _ in 0..20 {
            resource_points.push(ResourcePoint {
                position: (
                    rng.random_range(10.0..(map_size.0 as f32 - 10.0)),
                    rng.random_range(10.0..(map_size.1 as f32 - 10.0)),
                ),
                resource_type: rng.random_range(0..4),
                amount: rng.random_range(50.0..200.0),
                regen_rate: 0.5,
            });
        }

        Self {
            seed,
            tick: 0,
            map_size,
            factions,
            armies,
            battles: Vec::new(),
            resource_points,
            zone: SafeZone::new(center, center.0.min(center.1)),
            rng,
        }
    }

    /// Advance simulation by one tick
    pub fn tick(&mut self) {
        self.tick += 1;

        // Move armies (静默移动，不打日志)
        for army in &mut self.armies {
            if army.engaged_lock == 0 {
                army.move_tick();
            }
        }

        // 资源消耗和采集
        self.process_supplies();

        // Update safe zone
        let old_radius = self.zone.radius;
        self.zone.tick();
        if self.zone.shrink_rate > 0.0 && self.tick % 100 == 0 && self.zone.radius < old_radius {
            info!("[Tick {}] 安全区收缩: 半径 {:.1} -> {:.1}",
                self.tick, old_radius, self.zone.radius);
        }

        // Apply zone damage to armies outside
        if self.zone.damage_rate > 0.0 {
            for army in &mut self.armies {
                if army.troops > 0 && !self.zone.is_inside(army.position) {
                    let damage = (army.troops as f32 * self.zone.damage_rate) as u32;
                    if damage > 0 {
                        army.troops = army.troops.saturating_sub(damage);
                        info!("[Tick {}] ☢️ 军团#{} (阵营{}) 在安全区外受到伤害: -{}兵, 剩余{}兵",
                            self.tick, army.id, army.faction_id, damage, army.troops);
                    }
                }
            }
        }

        // Detect encounters
        self.detect_encounters();

        // Process battles
        self.process_battles();

        // Check faction elimination
        self.check_eliminations();
    }

    fn detect_encounters(&mut self) {
        let mut new_encounters = Vec::new();

        for i in 0..self.armies.len() {
            for j in (i + 1)..self.armies.len() {
                let a = &self.armies[i];
                let b = &self.armies[j];

                if a.faction_id == b.faction_id {
                    continue;
                }
                if a.engaged_lock > 0 || b.engaged_lock > 0 {
                    continue;
                }

                let dx = a.position.0 - b.position.0;
                let dy = a.position.1 - b.position.1;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= R_ENCOUNTER {
                    new_encounters.push((a.id, b.id, dist));
                }
            }
        }

        // Create battle instances for new encounters
        for (id_a, id_b, dist) in new_encounters {
            let battle_id = self.battles.len() as u32;

            let army_a = self.armies.iter().find(|a| a.id == id_a);
            let army_b = self.armies.iter().find(|a| a.id == id_b);

            if let (Some(a), Some(b)) = (army_a, army_b) {
                info!("[Tick {}] ⚔️ 遭遇! 军团#{} (阵营{}, {}兵) vs 军团#{} (阵营{}, {}兵) 距离{:.1}",
                    self.tick, a.id, a.faction_id, a.troops, b.id, b.faction_id, b.troops, dist);
            }

            let pos_a = self.armies.iter().find(|a| a.id == id_a).map(|a| a.position).unwrap_or((0.0, 0.0));
            let pos_b = self.armies.iter().find(|a| a.id == id_b).map(|a| a.position).unwrap_or((0.0, 0.0));
            let battle_pos = ((pos_a.0 + pos_b.0) / 2.0, (pos_a.1 + pos_b.1) / 2.0);

            let mut battle = BattleInstance::new(battle_id, battle_pos);
            battle.start_tick = self.tick;

            for army in &mut self.armies {
                if army.id == id_a || army.id == id_b {
                    army.engaged_lock = 100;
                    army.current_battle = Some(battle_id);
                    battle.participants.push(BattleParticipant {
                        army_id: army.id,
                        faction_id: army.faction_id,
                        initial_troops: army.troops,
                        current_troops: army.troops,
                        effective_power: army.effective_power(),
                        casualties: 0,
                    });
                }
            }

            info!("[Tick {}] 🏁 战斗#{} 开始于 ({:.1}, {:.1})",
                self.tick, battle_id, battle_pos.0, battle_pos.1);
            self.battles.push(battle);
        }
    }

    fn process_battles(&mut self) {
        let tick = self.tick;
        let mut ended_battles = Vec::new();

        // 收集战斗中军团的弹药信息，并消耗弹药
        let mut army_ammo_factors: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
        for army in &mut self.armies {
            if army.current_battle.is_some() && army.troops > 0 {
                // 消耗弹药
                army.ammo = (army.ammo - AMMO_CONSUME_RATE).max(0.0);
                // 计算伤害系数：有弹药=1.0，无弹药=0.5
                let factor = if army.ammo > 0.0 { 1.0 } else { NO_AMMO_PENALTY };
                army_ammo_factors.insert(army.id, factor);
            }
        }

        for battle in &mut self.battles {
            if battle.phase == BattlePhase::Ended {
                continue;
            }

            battle.duration += 1;
            battle.phase = BattlePhase::Combat;

            // Simple combat resolution
            if battle.participants.len() >= 2 {
                let total_power: f32 = battle.participants.iter().map(|p| p.effective_power).sum();
                let mut tick_casualties = Vec::new();

                for i in 0..battle.participants.len() {
                    let my_power = battle.participants[i].effective_power;
                    let enemy_power = total_power - my_power;
                    let army_id = battle.participants[i].army_id;

                    // 获取弹药系数（影响对敌方造成的伤害）
                    let ammo_factor = *army_ammo_factors.get(&army_id).unwrap_or(&1.0);

                    // 计算我方对敌方造成的伤亡比例 (受弹药影响)
                    let my_damage_rate = (my_power / (my_power + enemy_power + 1.0)) * 0.01 * ammo_factor;

                    // 计算敌方对我方造成的伤亡
                    let casualty_rate = (enemy_power / (my_power + enemy_power + 1.0)) * 0.01;
                    let casualties = (battle.participants[i].current_troops as f32 * casualty_rate) as u32;

                    if casualties > 0 {
                        tick_casualties.push((battle.participants[i].faction_id, casualties, ammo_factor < 1.0));
                    }

                    battle.participants[i].current_troops =
                        battle.participants[i].current_troops.saturating_sub(casualties);
                    battle.participants[i].casualties += casualties;
                }

                // 每10 tick报告一次伤亡
                if battle.duration % 10 == 0 && !tick_casualties.is_empty() {
                    let status: Vec<String> = battle.participants.iter()
                        .map(|p| {
                            let ammo_warning = if *army_ammo_factors.get(&p.army_id).unwrap_or(&1.0) < 1.0 {
                                "⚠️无弹药"
                            } else { "" };
                            format!("阵营{}:{}兵(-{}){}", p.faction_id, p.current_troops, p.casualties, ammo_warning)
                        })
                        .collect();
                    info!("[Tick {}] 战斗#{} 进行中 ({}轮): {}",
                        tick, battle.id, battle.duration, status.join(" vs "));
                }
            }

            // Check battle end
            let alive_factions: Vec<u8> = battle.participants
                .iter()
                .filter(|p| p.current_troops > 0)
                .map(|p| p.faction_id)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            if alive_factions.len() <= 1 || battle.duration > 300 {
                battle.phase = BattlePhase::Ended;

                // 战斗结束日志
                let result: Vec<String> = battle.participants.iter()
                    .map(|p| format!("阵营{}: {}->{}兵 (阵亡{})",
                        p.faction_id, p.initial_troops, p.current_troops, p.casualties))
                    .collect();

                let winner = if alive_factions.len() == 1 {
                    format!("阵营{}获胜", alive_factions[0])
                } else if alive_factions.is_empty() {
                    "全军覆没".to_string()
                } else {
                    "超时结束".to_string()
                };

                info!("[Tick {}] 🏆 战斗#{} 结束! {} | {} | 持续{}轮",
                    tick, battle.id, winner, result.join(", "), battle.duration);

                ended_battles.push(battle.id);
            }
        }

        // Update armies for ended battles
        for battle in &self.battles {
            if ended_battles.contains(&battle.id) {
                for participant in &battle.participants {
                    if let Some(army) = self.armies.iter_mut().find(|a| a.id == participant.army_id) {
                        army.troops = participant.current_troops;
                        army.engaged_lock = 0;
                        army.current_battle = None;

                        if army.troops == 0 {
                            info!("[Tick {}] 💀 军团#{} (阵营{}) 全军覆没!",
                                tick, army.id, army.faction_id);
                        }
                    }
                }
            }
        }

        // Remove ended battles
        self.battles.retain(|b| b.phase != BattlePhase::Ended);
    }

    fn check_eliminations(&mut self) {
        let tick = self.tick;
        for faction in &mut self.factions {
            if !faction.is_alive {
                continue;
            }

            let has_armies = self.armies.iter().any(|a| a.faction_id == faction.id && a.troops > 0);
            if !has_armies {
                faction.is_alive = false;
                info!("[Tick {}] ☠️ 阵营{} ({}) 被消灭! 已无存活军团",
                    tick, faction.id, faction.name);
            }
        }
    }

    /// 处理补给消耗、饥饿、资源采集和宝石转化
    fn process_supplies(&mut self) {
        let tick = self.tick;

        // 1. 军团消耗补给
        for army in &mut self.armies {
            if army.troops == 0 {
                continue;
            }

            // 按兵力消耗补给
            let consumption = army.troops as f32 * SUPPLY_CONSUME_RATE;
            army.supplies -= consumption;

            // 补给耗尽：饿死 + 士气下降
            if army.supplies <= 0.0 {
                army.supplies = 0.0;
                let starved = (army.troops as f32 * STARVE_RATE).ceil() as u32;
                if starved > 0 {
                    army.troops = army.troops.saturating_sub(starved);
                    army.morale = (army.morale - MORALE_STARVE_DROP).max(0.0);
                    info!("[Tick {}] 🍽️ 军团#{} (阵营{}) 补给耗尽! 饿死{}人, 剩余{}兵, 士气{:.0}",
                        tick, army.id, army.faction_id, starved, army.troops, army.morale);
                }
            }
        }

        // 2. 宝石自动转化（优先补给不足时转化为食物，其次弹药不足时转化为弹药）
        for army in &mut self.armies {
            if army.troops == 0 || army.gems <= 0.0 {
                continue;
            }

            let convert_amount = GEMS_CONVERT_RATE.min(army.gems);

            // 优先转化为食物（当补给不足时）
            if army.supplies < 70.0 {
                army.gems -= convert_amount;
                let food_gained = convert_amount * GEMS_TO_FOOD_RATIO;
                army.supplies = (army.supplies + food_gained).min(150.0);
                if tick % 50 == 0 {
                    info!("[Tick {}] 💎→🍞 军团#{} (阵营{}) 转化宝石: {:.1}宝石 → {:.1}食物, 当前补给{:.1}",
                        tick, army.id, army.faction_id, convert_amount, food_gained, army.supplies);
                }
            }
            // 其次转化为弹药（当弹药不足时）
            else if army.ammo < 50.0 && army.gems > 0.0 {
                let convert_amount = GEMS_CONVERT_RATE.min(army.gems);
                army.gems -= convert_amount;
                let ammo_gained = convert_amount * GEMS_TO_AMMO_RATIO;
                army.ammo = (army.ammo + ammo_gained).min(100.0);
                if tick % 50 == 0 {
                    info!("[Tick {}] 💎→🔫 军团#{} (阵营{}) 转化宝石: {:.1}宝石 → {:.1}弹药, 当前弹药{:.1}",
                        tick, army.id, army.faction_id, convert_amount, ammo_gained, army.ammo);
                }
            }
        }

        // 3. 采集资源（需要收集军团位置信息，避免借用冲突）
        // 收集军团信息：id, faction_id, position, needs_food, needs_ammo, needs_gems, gems
        let army_positions: Vec<(u32, u8, (f32, f32), bool, bool, bool, f32)> = self.armies.iter()
            .filter(|a| a.troops > 0 && a.engaged_lock == 0)
            .map(|a| (a.id, a.faction_id, a.position, a.supplies < 100.0, a.ammo < 80.0, a.gems < 80.0, a.gems))
            .collect();

        for (army_id, faction_id, pos, needs_food, needs_ammo, needs_gems, _current_gems) in &army_positions {
            // 查找范围内的资源点
            for (i, rp) in self.resource_points.iter().enumerate() {
                if rp.amount <= 0.0 {
                    continue;
                }
                let dx = rp.position.0 - pos.0;
                let dy = rp.position.1 - pos.1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > R_GATHER {
                    continue;
                }

                // 根据资源类型采集
                let (should_gather, resource_name) = match rp.resource_type {
                    0 => (*needs_food, "食物"),      // 食物 → supplies
                    1 => (*needs_gems, "宝石"),      // 宝石 → gems
                    2 => (*needs_ammo, "弹药"),      // 弹药 → ammo
                    _ => (false, ""),                // 医疗暂不使用
                };

                if !should_gather {
                    continue;
                }

                let rp = &mut self.resource_points[i];
                let gather = GATHER_RATE.min(rp.amount);
                rp.amount -= gather;

                // 补充对应资源
                if let Some(army) = self.armies.iter_mut().find(|a| a.id == *army_id) {
                    match rp.resource_type {
                        0 => {
                            army.supplies = (army.supplies + gather).min(150.0);
                            if tick % 50 == 0 {
                                info!("[Tick {}] 📦 军团#{} (阵营{}) 采集{}: +{:.1}, 当前补给{:.1}",
                                    tick, army_id, faction_id, resource_name, gather, army.supplies);
                            }
                        }
                        1 => {
                            army.gems = (army.gems + gather).min(100.0);
                            if tick % 50 == 0 {
                                info!("[Tick {}] 💎 军团#{} (阵营{}) 采集{}: +{:.1}, 当前宝石{:.1}",
                                    tick, army_id, faction_id, resource_name, gather, army.gems);
                            }
                        }
                        2 => {
                            army.ammo = (army.ammo + gather).min(100.0);
                            if tick % 50 == 0 {
                                info!("[Tick {}] 📦 军团#{} (阵营{}) 采集{}: +{:.1}, 当前弹药{:.1}",
                                    tick, army_id, faction_id, resource_name, gather, army.ammo);
                            }
                        }
                        _ => {}
                    }
                }
                break; // 每tick只采集一种资源
            }
        }

        // 4. 资源点再生
        for rp in &mut self.resource_points {
            if rp.amount < RESOURCE_MAX {
                rp.amount = (rp.amount + rp.regen_rate).min(RESOURCE_MAX);
            }
        }
    }

    /// Check if game is over
    pub fn is_game_over(&self) -> Option<u8> {
        let alive: Vec<&Faction> = self.factions.iter().filter(|f| f.is_alive).collect();
        if alive.len() == 1 {
            Some(alive[0].id)
        } else if alive.is_empty() {
            Some(255) // Draw
        } else {
            None
        }
    }

    /// 生成世界状态快照用于调试和比较
    pub fn snapshot(&self) -> String {
        let mut s = format!("tick:{} ", self.tick);

        // 阵营状态
        for f in &self.factions {
            s.push_str(&format!("f{}:{} ", f.id, if f.is_alive { "alive" } else { "dead" }));
        }

        // 军团状态（按ID排序确保确定性）
        let mut armies: Vec<_> = self.armies.iter().collect();
        armies.sort_by_key(|a| a.id);
        for a in armies {
            s.push_str(&format!(
                "a{}:({:.2},{:.2},{}troops) ",
                a.id, a.position.0, a.position.1, a.troops
            ));
        }

        // 战斗数量
        s.push_str(&format!("battles:{}", self.battles.len()));

        s
    }

}

/// Main game model - named LlmArenaModel for app! macro
pub struct LlmArenaModel {
    pub world: World,
    pub state: ArenaState,
    pub speed: u32,
    pub viewport: (f32, f32), // Camera position
    pub selected_army: Option<u32>,
}

impl Default for LlmArenaModel {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmArenaModel {
    pub fn new() -> Self {
        let seed = 12345u64;
        Self {
            world: World::new(seed, 2), // Start with 2 factions
            state: ArenaState::Init,
            speed: 1,
            viewport: (MAP_WIDTH as f32 / 2.0, MAP_HEIGHT as f32 / 2.0),
            selected_army: None,
        }
    }
}

impl Model for LlmArenaModel {
    fn init(&mut self, _context: &mut Context) {
        self.state = ArenaState::Running;

        info!("========== LLM Arena 游戏初始化 ==========");
        info!("种子: {} | 地图: {}x{}", self.world.seed, self.world.map_size.0, self.world.map_size.1);
        for faction in &self.world.factions {
            let armies: Vec<String> = self.world.armies.iter()
                .filter(|a| a.faction_id == faction.id)
                .map(|a| format!("军团#{}({}兵,位置{:.0},{:.0})", a.id, a.troops, a.position.0, a.position.1))
                .collect();
            info!("阵营{} ({}): {}", faction.id, faction.name, armies.join(", "));
        }
        info!("资源点: {}个", self.world.resource_points.len());
        info!("==========================================");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        // Handle keyboard input
        for key in &context.input_events {
            match key {
                Event::Key(key_event) => {
                    match key_event.code {
                        KeyCode::Char(' ') => {
                            self.state = if self.state == ArenaState::Paused {
                                ArenaState::Running
                            } else {
                                ArenaState::Paused
                            };
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            self.speed = (self.speed + 1).min(10);
                        }
                        KeyCode::Char('-') => {
                            self.speed = self.speed.saturating_sub(1).max(1);
                        }
                        KeyCode::Left => {
                            self.viewport.0 = (self.viewport.0 - 5.0).max(0.0);
                        }
                        KeyCode::Right => {
                            self.viewport.0 = (self.viewport.0 + 5.0).min(MAP_WIDTH as f32);
                        }
                        KeyCode::Up => {
                            self.viewport.1 = (self.viewport.1 - 5.0).max(0.0);
                        }
                        KeyCode::Down => {
                            self.viewport.1 = (self.viewport.1 + 5.0).min(MAP_HEIGHT as f32);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {
        // Random agent: 根据资源需求寻找对应资源点
        if self.state == ArenaState::Running {
            // 分类收集资源点：食物(type=0)、宝石(type=1)、弹药(type=2)
            let food_points: Vec<(f32, f32)> = self.world.resource_points.iter()
                .filter(|rp| rp.resource_type == 0 && rp.amount > 10.0)
                .map(|rp| (rp.position.0, rp.position.1))
                .collect();

            let gems_points: Vec<(f32, f32)> = self.world.resource_points.iter()
                .filter(|rp| rp.resource_type == 1 && rp.amount > 10.0)
                .map(|rp| (rp.position.0, rp.position.1))
                .collect();

            let ammo_points: Vec<(f32, f32)> = self.world.resource_points.iter()
                .filter(|rp| rp.resource_type == 2 && rp.amount > 10.0)
                .map(|rp| (rp.position.0, rp.position.1))
                .collect();

            for army in &mut self.world.armies {
                if army.target.is_none() && army.engaged_lock == 0 && army.troops > 0 {
                    let needs_food = army.supplies < 70.0;
                    let needs_ammo = army.ammo < 30.0;
                    let needs_gems = army.gems < 50.0; // 宝石不足时也去采集

                    // 辅助函数：找最近的资源点
                    let find_nearest = |points: &[(f32, f32)], pos: (f32, f32)| -> Option<(f32, f32)> {
                        points.iter()
                            .map(|&(rx, ry)| {
                                let dx = rx - pos.0;
                                let dy = ry - pos.1;
                                ((rx, ry), (dx * dx + dy * dy).sqrt())
                            })
                            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                            .map(|(pos, _)| pos)
                    };

                    // 优先级：食物 > 弹药 > 宝石 > 随机
                    // 宝石作为战略储备，当食物和弹药充足时采集
                    if needs_food && !food_points.is_empty() {
                        if let Some(target) = find_nearest(&food_points, army.position) {
                            army.target = Some(target);
                        }
                    } else if needs_ammo && !ammo_points.is_empty() {
                        if let Some(target) = find_nearest(&ammo_points, army.position) {
                            army.target = Some(target);
                        }
                    } else if needs_gems && !gems_points.is_empty() {
                        // 食物和弹药充足时，采集宝石作为储备
                        if let Some(target) = find_nearest(&gems_points, army.position) {
                            army.target = Some(target);
                        }
                    } else {
                        // 所有资源充足时随机移动
                        let x = self.world.rng.random_range(10.0..(MAP_WIDTH as f32 - 10.0));
                        let y = self.world.rng.random_range(10.0..(MAP_HEIGHT as f32 - 10.0));
                        army.target = Some((x, y));
                    }
                }
            }
        }
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {
        // Process custom events
    }

    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {
        // Process timer events - advance simulation
        if self.state == ArenaState::Running {
            for _ in 0..self.speed {
                self.world.tick();
            }
        }

        // Check game over
        if self.state == ArenaState::Running {
            if let Some(winner_id) = self.world.is_game_over() {
                self.state = ArenaState::GameOver;

                info!("==========================================");
                if winner_id == 255 {
                    info!("[Tick {}] 🎮 游戏结束: 平局!", self.world.tick);
                } else {
                    let winner_name = self.world.factions.iter()
                        .find(|f| f.id == winner_id)
                        .map(|f| f.name.as_str())
                        .unwrap_or("未知");
                    info!("[Tick {}] 🎮 游戏结束: 阵营{} ({}) 获得胜利!",
                        self.world.tick, winner_id, winner_name);
                }
                info!("==========================================");
            }
        }
    }
}

// ============== 单元测试 ==============

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数：运行仿真直到游戏结束或达到最大tick
    fn run_simulation(seed: u64, num_factions: u8, max_ticks: u32) -> (World, Option<u8>) {
        let mut world = World::new(seed, num_factions);

        for _ in 0..max_ticks {
            // 模拟 Random Agent: 为没有目标的军团设置随机目标
            for army in &mut world.armies {
                if army.target.is_none() && army.engaged_lock == 0 && army.troops > 0 {
                    let x = world.rng.random_range(10.0..(MAP_WIDTH as f32 - 10.0));
                    let y = world.rng.random_range(10.0..(MAP_HEIGHT as f32 - 10.0));
                    army.target = Some((x, y));
                }
            }

            world.tick();

            if let Some(winner) = world.is_game_over() {
                return (world, Some(winner));
            }
        }

        (world, None)
    }

    /// 获取世界状态的简化快照用于比较
    fn world_snapshot(world: &World) -> String {
        let mut s = format!("tick:{} ", world.tick);

        // 阵营状态
        for f in &world.factions {
            s.push_str(&format!("f{}:{} ", f.id, if f.is_alive { "alive" } else { "dead" }));
        }

        // 军团状态（按ID排序）
        let mut armies: Vec<_> = world.armies.iter().collect();
        armies.sort_by_key(|a| a.id);
        for a in armies {
            s.push_str(&format!(
                "a{}:({:.2},{:.2},{}troops) ",
                a.id, a.position.0, a.position.1, a.troops
            ));
        }

        // 战斗数量
        s.push_str(&format!("battles:{}", world.battles.len()));

        s
    }

    #[test]
    fn test_deterministic_same_seed() {
        // 使用相同的种子运行两次，结果应该完全相同
        let seed = 12345u64;
        let max_ticks = 500;

        let (world1, winner1) = run_simulation(seed, 2, max_ticks);
        let (world2, winner2) = run_simulation(seed, 2, max_ticks);

        let snapshot1 = world_snapshot(&world1);
        let snapshot2 = world_snapshot(&world2);

        assert_eq!(snapshot1, snapshot2, "相同种子应产生相同结果");
        assert_eq!(winner1, winner2, "相同种子应产生相同胜者");
        assert_eq!(world1.tick, world2.tick, "相同种子应在相同tick结束");
    }

    #[test]
    fn test_deterministic_different_seeds() {
        // 使用不同的种子，结果应该不同
        let (world1, _) = run_simulation(12345, 2, 200);
        let (world2, _) = run_simulation(54321, 2, 200);

        let snapshot1 = world_snapshot(&world1);
        let snapshot2 = world_snapshot(&world2);

        assert_ne!(snapshot1, snapshot2, "不同种子应产生不同结果");
    }

    #[test]
    fn test_world_initialization() {
        let world = World::new(12345, 4);

        assert_eq!(world.factions.len(), 4, "应有4个阵营");
        assert_eq!(world.armies.len(), 8, "应有8个军团 (每阵营2个)");
        assert_eq!(world.tick, 0, "初始tick应为0");
        assert!(world.battles.is_empty(), "初始应无战斗");

        for faction in &world.factions {
            assert!(faction.is_alive, "所有阵营初始应存活");
        }

        for army in &world.armies {
            assert!(army.troops > 0, "所有军团初始应有兵力");
        }
    }

    #[test]
    fn test_army_movement() {
        let mut army = Army::new(0, 0, 50.0, 50.0, 100);
        army.target = Some((60.0, 50.0));
        army.speed = 2.0;

        // 移动一步
        let reached = army.move_tick();
        assert!(!reached, "未到达目标");
        assert!(army.position.0 > 50.0, "应向目标移动");

        // 设置近距离目标
        army.target = Some((army.position.0 + 0.5, army.position.1));
        let reached = army.move_tick();
        assert!(reached, "应到达目标");
        assert!(army.target.is_none(), "到达后目标应清除");
    }

    #[test]
    fn test_encounter_detection() {
        let mut world = World::new(99999, 2);

        // 清空现有军团，手动放置
        world.armies.clear();
        world.armies.push(Army::new(0, 0, 50.0, 50.0, 100));
        world.armies.push(Army::new(1, 1, 50.0 + R_ENCOUNTER - 1.0, 50.0, 100));

        // 运行遭遇检测
        world.detect_encounters();

        assert_eq!(world.battles.len(), 1, "应检测到1场遭遇");
        assert_eq!(world.battles[0].participants.len(), 2, "战斗应有2个参与者");
    }

    #[test]
    fn test_battle_resolution() {
        let mut world = World::new(88888, 2);

        // 清空现有军团，手动放置两个军团在同一位置
        world.armies.clear();
        world.armies.push(Army::new(0, 0, 50.0, 50.0, 500));
        world.armies.push(Army::new(1, 1, 50.0, 50.0, 500));

        // 触发遭遇
        world.detect_encounters();
        assert_eq!(world.battles.len(), 1);

        // 运行战斗直到结束
        for _ in 0..500 {
            world.process_battles();
            if world.battles.is_empty() {
                break;
            }
        }

        // 战斗应该结束
        assert!(world.battles.is_empty(), "战斗应已结束");

        // 至少一方应有伤亡
        let total_troops: u32 = world.armies.iter().map(|a| a.troops).sum();
        assert!(total_troops < 1000, "应有伤亡");
    }

    #[test]
    fn test_faction_elimination() {
        let mut world = World::new(77777, 2);

        // 将阵营1的所有军团兵力设为0
        for army in &mut world.armies {
            if army.faction_id == 1 {
                army.troops = 0;
            }
        }

        world.check_eliminations();

        let faction1 = world.factions.iter().find(|f| f.id == 1).unwrap();
        assert!(!faction1.is_alive, "阵营1应被消灭");

        let winner = world.is_game_over();
        assert_eq!(winner, Some(0), "阵营0应获胜");
    }

    #[test]
    fn test_safe_zone() {
        let mut zone = SafeZone::new((100.0, 50.0), 50.0);
        zone.shrink_rate = 1.0;
        zone.min_radius = 20.0;

        assert!(zone.is_inside((100.0, 50.0)), "中心应在区域内");
        assert!(zone.is_inside((120.0, 50.0)), "边界内应在区域内");
        assert!(!zone.is_inside((160.0, 50.0)), "边界外应不在区域内");

        // 收缩
        for _ in 0..40 {
            zone.tick();
        }
        assert!((zone.radius - 20.0).abs() < 0.1, "应收缩到最小半径");
    }

    #[test]
    fn test_supply_consumption() {
        let mut world = World::new(55555, 2);

        // 记录初始补给
        let initial_supply = world.armies[0].supplies;
        let initial_troops = world.armies[0].troops;

        // 运行100 tick，让补给消耗
        for _ in 0..100 {
            world.tick();
        }

        // 补给应该减少
        assert!(world.armies[0].supplies < initial_supply, "补给应该消耗");

        // 强制耗尽补给测试饥饿
        for army in &mut world.armies {
            army.supplies = 0.0;
        }

        let troops_before = world.armies[0].troops;
        world.process_supplies();

        // 应该有饿死
        assert!(world.armies[0].troops < troops_before, "补给耗尽应导致饿死");
    }

    #[test]
    fn test_resource_gathering() {
        let mut world = World::new(66666, 2);

        // 找到一个食物资源点 (type=0)
        let food_rp_idx = world.resource_points.iter()
            .position(|rp| rp.resource_type == 0)
            .expect("应有食物资源点");

        // 把军团放到该资源点
        let rp_pos = world.resource_points[food_rp_idx].position;
        world.armies[0].position = rp_pos;
        world.armies[0].supplies = 50.0; // 低补给
        world.armies[0].engaged_lock = 0;

        let initial_supply = world.armies[0].supplies;
        let initial_rp_amount = world.resource_points[food_rp_idx].amount;

        // 运行采集
        world.process_supplies();

        // 补给应该增加
        assert!(world.armies[0].supplies > initial_supply, "应该采集到食物资源");
        // 资源点应该减少
        assert!(world.resource_points[food_rp_idx].amount < initial_rp_amount, "食物资源点应该被消耗");
    }

    #[test]
    fn test_ammo_gathering() {
        let mut world = World::new(77777, 2);

        // 找到一个弹药资源点 (type=2)
        let ammo_rp_idx = world.resource_points.iter()
            .position(|rp| rp.resource_type == 2)
            .expect("应有弹药资源点");

        // 把军团放到该资源点
        let rp_pos = world.resource_points[ammo_rp_idx].position;
        world.armies[0].position = rp_pos;
        world.armies[0].ammo = 30.0; // 低弹药
        world.armies[0].supplies = 100.0; // 食物充足
        world.armies[0].engaged_lock = 0;

        let initial_ammo = world.armies[0].ammo;
        let initial_rp_amount = world.resource_points[ammo_rp_idx].amount;

        // 运行采集
        world.process_supplies();

        // 弹药应该增加
        assert!(world.armies[0].ammo > initial_ammo, "应该采集到弹药资源");
        // 资源点应该减少
        assert!(world.resource_points[ammo_rp_idx].amount < initial_rp_amount, "弹药资源点应该被消耗");
    }

    #[test]
    fn test_gems_gathering() {
        let mut world = World::new(88888, 2);

        // 找到一个宝石资源点 (type=1)
        let gems_rp_idx = world.resource_points.iter()
            .position(|rp| rp.resource_type == 1)
            .expect("应有宝石资源点");

        // 把军团放到该资源点
        let rp_pos = world.resource_points[gems_rp_idx].position;
        world.armies[0].position = rp_pos;
        world.armies[0].gems = 10.0; // 低宝石
        world.armies[0].supplies = 100.0; // 食物充足
        world.armies[0].ammo = 100.0; // 弹药充足
        world.armies[0].engaged_lock = 0;

        let initial_gems = world.armies[0].gems;
        let initial_rp_amount = world.resource_points[gems_rp_idx].amount;

        // 运行采集
        world.process_supplies();

        // 宝石应该增加
        assert!(world.armies[0].gems > initial_gems, "应该采集到宝石资源");
        // 资源点应该减少
        assert!(world.resource_points[gems_rp_idx].amount < initial_rp_amount, "宝石资源点应该被消耗");
    }

    #[test]
    fn test_gems_conversion_to_food() {
        let mut world = World::new(99999, 2);

        // 设置军团：有宝石但缺食物
        world.armies[0].gems = 10.0;
        world.armies[0].supplies = 50.0; // 低于70，触发转化
        world.armies[0].ammo = 100.0;

        let initial_gems = world.armies[0].gems;
        let initial_supplies = world.armies[0].supplies;

        // 运行资源处理
        world.process_supplies();

        // 宝石应该减少
        assert!(world.armies[0].gems < initial_gems, "宝石应该被消耗");
        // 食物应该增加（减去消耗后仍应高于之前）
        // 注意：消耗 = troops * 0.001 ≈ 0.5，转化获得 = 1.0 * 2.0 = 2.0
        assert!(world.armies[0].supplies > initial_supplies - 1.0, "食物应该通过宝石转化增加");
    }

    #[test]
    fn test_gems_conversion_to_ammo() {
        let mut world = World::new(11111, 2);

        // 设置军团：有宝石，食物充足，但缺弹药
        world.armies[0].gems = 10.0;
        world.armies[0].supplies = 100.0; // 充足，不触发食物转化
        world.armies[0].ammo = 30.0; // 低于50，触发弹药转化

        let initial_gems = world.armies[0].gems;
        let initial_ammo = world.armies[0].ammo;

        // 运行资源处理
        world.process_supplies();

        // 宝石应该减少
        assert!(world.armies[0].gems < initial_gems, "宝石应该被消耗转化为弹药");
        // 弹药应该增加 (1.0 * 2.0 = 2.0)
        assert!(world.armies[0].ammo > initial_ammo, "弹药应该通过宝石转化增加");
    }
}
