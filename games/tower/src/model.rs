use rust_pixel::event::Event;
// use log::info;
use rust_pixel::{
    context::Context,
    event::{event_check, event_emit, timer_fire, timer_register},
    game::Model,
    util::{objpool::GameObjPool, Point},
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

    pub timeout_auto: f32,

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
            timeout_auto: 0.0,
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
            Point { x: 0, y: 1 },
            Point { x: 1, y: 1 },
            Point { x: 2, y: 3 },
            Point { x: 2, y: 4 },
            Point { x: 3, y: 6 },
            Point { x: 4, y: 6 },
            Point { x: 5, y: 6 },
            Point { x: 6, y: 6 },
        ];
        for p in &bps {
            self.blocks.create(0, &vec![*p]);
        }

        // 创建类型为0的塔
        let mut tps = vec![Point { x: 5, y: 3 }, Point { x: 10, y: 4 }];
        for p in &tps {
            self.towers.create(0, &vec![*p]);
        }
        // 创建类型为1的塔
        tps = vec![
            Point { x: 2, y: 2 },
            Point { x: 8, y: 8 },
            Point { x: 10, y: 7 },
            Point { x: 12, y: 8 },
        ];
        for p in &tps {
            self.towers.create(1, &vec![*p]);
        }
        // 创建类型为2的塔
        tps = vec![Point { x: 2, y: 5 }, Point { x: 15, y: 8 }];
        for p in &tps {
            self.towers.create(2, &vec![*p]);
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
                let dst_pos = Point {
                    x: target_monster_pos.x as u16,
                    y: target_monster_pos.y as u16,
                };
                let cell_size = Point {
                    x: ctx.adapter.cell_width() as u16,
                    y: ctx.adapter.cell_height() as u16,
                };
                let mid = Point { x: *v as u16, y: 0 };
                // cell_size, tower_pos, monster_pos
                if t.obj.ttype == 2 {
                    self.lasers
                        .create(t.obj.ttype, &vec![cell_size, t.obj.pos, dst_pos, mid]);
                } else {
                    self.bullets
                        .create(t.obj.ttype, &vec![cell_size, t.obj.pos, dst_pos]);
                }
            }
        });
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, _dt: f32) {
        let csp = Point {
            x: ctx.adapter.cell_width() as u16,
            y: ctx.adapter.cell_height() as u16,
        };
        for i in 0..8 {
            let tstr = format!("Tower.CreatMonster{}", i);
            if event_check(&tstr, "_") {
                if i > 3 {
                    self.monsters.create(1, &vec![csp]);
                } else {
                    self.monsters.create(0, &vec![csp]);
                }
            }
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
