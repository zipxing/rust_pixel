use rust_pixel::event::Event;
// use log::info;
use rust_pixel::{
    context::Context,
    event::{event_check, event_emit, timer_fire, timer_register},
    game::Model,
    util::objpool::GameObjPool,
};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use tower_lib::{
    block::*, bomb::*, bullet::*, laser::*, monster::*, tower::*, MAX_BLOCK_COUNT, MAX_BOMB_COUNT,
    MAX_LASER_COUNT, MAX_MONSTER_COUNT, MAX_TOWER_COUNT, TOWERH, TOWERW,
};

enum TowerState {
    Normal,
}

pub struct TowerModel {
    // map grid...
    pub grid: Vec<Vec<u8>>,

    //  用于子弹进行碰撞检测
    //  key: grid ID
    //  value: set of monsters id
    pub monster_map: HashMap<usize, HashSet<usize>>,

    // pub timeout_auto: f32,
    pub bombs: GameObjPool<Bomb>,
    pub blocks: GameObjPool<Block>,
    pub towers: GameObjPool<Tower>,
    pub bullets: GameObjPool<Bullet>,
    pub lasers: GameObjPool<Laser>,
    pub monsters: GameObjPool<Monster>,
}

impl TowerModel {
    pub fn new() -> Self {
        Self {
            grid: vec![],
            monster_map: HashMap::new(),
            // timeout_auto: 0.0,
            bombs: GameObjPool::<Bomb>::new("BB", MAX_BOMB_COUNT),
            blocks: GameObjPool::<Block>::new("BL", MAX_BLOCK_COUNT),
            towers: GameObjPool::<Tower>::new("T", MAX_TOWER_COUNT),
            bullets: GameObjPool::<Bullet>::new("B", MAX_BLOCK_COUNT),
            lasers: GameObjPool::<Laser>::new("L", MAX_LASER_COUNT),
            monsters: GameObjPool::<Monster>::new("M", MAX_MONSTER_COUNT),
        }
    }

    pub fn make_grid(&mut self) {
        self.grid = vec![vec![]; TOWERH];
        for i in 0..TOWERH {
            self.grid[i] = vec![0u8; TOWERW];
        }
        for b in &self.blocks.pool {
            b.obj.set_in_grid(&mut self.grid);
        }
        for t in &self.towers.pool {
            t.obj.set_in_grid(&mut self.grid);
        }
    }
}

impl Model for TowerModel {
    fn init(&mut self, ctx: &mut Context) {
        ctx.rand.srand_now();
        ctx.input_events.clear();
        ctx.state = TowerState::Normal as u8;
        // 创建路障
        let bps = vec![
            (0u32, 1),
            (1, 1),
            (2, 3),
            (2, 4),
            (3, 6),
            (4, 6),
            (5, 6),
            (6, 6),
        ];
        for p in &bps {
            self.blocks.create(0, &vec![p.0, p.1]);
        }

        // 创建类型为0的塔
        let mut tps = vec![(5, 3), (10, 4)];
        for p in &tps {
            self.towers.create(0, &vec![p.0, p.1]);
        }
        // 创建类型为1的塔
        tps = vec![(2, 2), (8, 8), (10, 7), (12, 8)];
        for p in &tps {
            self.towers.create(1, &vec![p.0, p.1]);
        }
        // 创建类型为2的塔
        tps = vec![(2, 5), (15, 8)];
        for p in &tps {
            self.towers.create(2, &vec![p.0, p.1]);
        }

        // 注册创建怪物定时器，以便延迟创建怪物
        for i in 0..8 {
            let tstr = format!("Tower.CreatMonster{}", i);
            timer_register(&tstr, 0.1 + 1.5 * i as f32, "_");
            timer_fire(&tstr, 0u8);
        }

        // 更新grid
        self.make_grid();

        // 发射重绘事件
        event_emit("Tower.RedrawGrid");
    }

    fn handle_input(&mut self, ctx: &mut Context, _dt: f32) {
        let es = ctx.input_events.clone();
        for e in &es {
            match e {
                Event::Key(_key) => {}
                _ => {}
            }
        }
        ctx.input_events.clear();
    }

    fn handle_auto(&mut self, ctx: &mut Context, _dt: f32) {
        self.monsters.update_active(|m| {
            m.active = m.obj.update(
                m.id,
                &mut self.grid,
                &mut self.monster_map,
                ctx.adapter.cell_width(),
                ctx.adapter.cell_height(),
                &mut ctx.rand,
            );
        });
        self.bombs.update_active(|b| {
            b.active = b.obj.update();
        });
        self.bullets.update_active(|b| {
            b.active = b
                .obj
                .update(&mut self.bombs, &mut self.monsters, &self.monster_map);
        });
        self.lasers.update_active(|l| {
            l.active = l.obj.update(&mut self.bombs, &mut self.monsters);
        });
        self.towers.update_active(|t| {
            for v in &t.obj.update(&mut self.monsters, &mut ctx.rand) {
                let target_monster_pos = self.monsters.pool[*v].obj.pixel_pos;
                let dst_pos = (target_monster_pos.x as u32, target_monster_pos.y as u32);
                let cell_size = (
                    ctx.adapter.cell_width() as u32,
                    ctx.adapter.cell_height() as u32,
                );
                let mid = (*v as u32, 0u32);
                // cell_size, tower_pos, monster_pos
                if t.obj.ttype == 2 {
                    self.lasers.create(
                        t.obj.ttype,
                        &vec![
                            cell_size.0,
                            cell_size.1,
                            t.obj.pos.x as u32,
                            t.obj.pos.y as u32,
                            dst_pos.0,
                            dst_pos.1,
                            mid.0,
                            mid.1,
                        ],
                    );
                } else {
                    self.bullets.create(
                        t.obj.ttype,
                        &vec![
                            cell_size.0,
                            cell_size.1,
                            t.obj.pos.x as u32,
                            t.obj.pos.y as u32,
                            dst_pos.0,
                            dst_pos.1,
                        ],
                    );
                }
            }
        });
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, _dt: f32) {
        let csp = (
            ctx.adapter.cell_width() as u32,
            ctx.adapter.cell_height() as u32,
        );
        for i in 0..8 {
            let tstr = format!("Tower.CreatMonster{}", i);
            if event_check(&tstr, "_") {
                if i > 3 {
                    self.monsters.create(1, &vec![csp.0, csp.1]);
                } else {
                    self.monsters.create(0, &vec![csp.0, csp.1]);
                }
            }
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
