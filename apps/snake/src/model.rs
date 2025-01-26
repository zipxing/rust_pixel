use log::debug;
use rand::prelude::*;
use rust_pixel::event::{Event, KeyCode};
use rust_pixel::{
    context::Context,
    event::event_emit,
    game::Model,
    util::{Dir, ParticleSystem, ParticleSystemInfo, PointU16},
};
use std::f64::consts::PI;

pub const SNAKEW: usize = 60;
pub const SNAKEH: usize = 33;

enum SnakeState {
    Normal,
    OverSelf,
    OverBorder,
}

pub struct SnakeModel {
    pub pats: ParticleSystem,
    pub grid: [[i16; SNAKEW]; SNAKEH],
    pub seed: PointU16,
    pub body: Vec<PointU16>,
    pub dir: Dir,
    pub count: f64,
    pub timeout_auto: f32,
}

impl SnakeModel {
    pub fn new() -> Self {
        let particle_system_info = ParticleSystemInfo {
            emission_rate: 100.0,
            lifetime: -1.0,
            particle_life_min: 1.0,
            particle_life_max: 2.0,
            direction: PI / 2.0,
            spread: PI / 4.0,
            relative: false,
            speed_min: 50.0,
            speed_max: 100.0,
            g_min: 9.0,
            g_max: 10.0,
            rad_a_min: 3.0,
            rad_a_max: 5.0,
            tan_a_min: 1.0,
            tan_a_max: 5.0,
            size_start: 1.0,
            size_end: 5.0,
            size_var: 1.0,
            spin_start: 1.0,
            spin_end: 5.0,
            spin_var: 1.0,
            color_start: [0.0, 0.0, 0.0, 0.0],
            color_end: [1.0, 1.0, 1.0, 1.0],
            color_var: 0.1,
            alpha_var: 1.0,
        };
        let pats = ParticleSystem::new(particle_system_info);
        Self {
            pats,
            grid: [[0i16; SNAKEW]; SNAKEH],
            seed: PointU16 { x: 0, y: 0 },
            body: vec![],
            dir: Dir::Down,
            count: 0.0,
            timeout_auto: 0.0,
        }
    }

    pub fn make_grid(&mut self) {
        for i in 0..SNAKEH {
            for j in 0..SNAKEW {
                self.grid[i][j] = 0i16;
            }
        }
        for i in 0..self.body.len() {
            self.grid[self.body[i].y as usize][self.body[i].x as usize] = (i + 1) as i16;
        }
        self.grid[self.seed.y as usize][self.seed.x as usize] = 10000i16;
    }

    pub fn act(&mut self, d: Dir, context: &mut Context) {
        let dx: i16;
        let dy: i16;
        
        
        match d {
            Dir::Up => {
                if self.dir == Dir::Down {
                    return;
                };
                dx = 0;
                dy = -1
            }
            Dir::Down => {
                if self.dir == Dir::Up {
                    return;
                };
                dx = 0;
                dy = 1
            }
            Dir::Left => {
                if self.dir == Dir::Right {
                    return;
                };
                dx = -1;
                dy = 0
            }
            Dir::Right => {
                if self.dir == Dir::Left {
                    return;
                };
                dx = 1;
                dy = 0
            }
            _ => {
                dx = 0;
                dy = 0
            }
        }
        let cx: i16 = self.body[0].x as i16 + dx;
        let cy: i16 = self.body[0].y as i16 + dy;
        if cx >= SNAKEW as i16 || cy >= SNAKEH as i16 || cx < 0 || cy < 0 {
            context.state = SnakeState::OverBorder as u8;
            event_emit("Snake.RedrawGrid");
            return;
        }
        if self.grid[cy as usize][cx as usize] == 10000 {
            let mut rng = thread_rng();
            for i in 0..888 {
                let nx = rng.gen_range(0..SNAKEW) as u16;
                let ny = rng.gen_range(0..SNAKEH) as u16;
                let np = self.grid[ny as usize][nx as usize];
                //if np == 10000 || np == 0 {
                if np == 0 {
                    self.seed.x = nx;
                    self.seed.y = ny;
                    debug!("{:?} {:?} {:?} {:?}", i, nx, ny, np);
                    for j in 0..SNAKEH {
                        debug!("{:?}", self.grid[j]);
                    }
                    break;
                }
            }
        } else {
            if self.grid[cy as usize][cx as usize] != 0 {
                context.state = SnakeState::OverSelf as u8;
                event_emit("Snake.RedrawGrid");
                return;
            }
            self.body.pop();
        }
        self.body.splice(
            0..0,
            vec![PointU16 {
                x: cx as u16,
                y: cy as u16,
            }],
        );
        self.dir = d;
        self.make_grid();
        event_emit("Snake.RedrawGrid");
    }
}

impl Model for SnakeModel {
    fn init(&mut self, context: &mut Context) {
        self.body.clear();
        self.body.push(PointU16 {
            x: SNAKEW as u16 / 2,
            y: SNAKEH as u16 / 2,
        });
        let mut rng = thread_rng();
        self.seed.x = rng.gen_range(0..SNAKEW) as u16;
        self.seed.y = rng.gen_range(0..SNAKEH) as u16;
        self.make_grid();
        self.dir = Dir::Down;
        context.input_events.clear();
        context.state = SnakeState::Normal as u8;
        self.pats.fire_at(10.0, 10.0);

        event_emit("Snake.RedrawGrid");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            if let Event::Key(key) = e {
                let mut d: Option<Dir> = None;
                match key.code {
                    KeyCode::Char('w') => d = Some(Dir::Up),
                    KeyCode::Char('a') => d = Some(Dir::Left),
                    KeyCode::Char('s') => d = Some(Dir::Down),
                    KeyCode::Char('d') => d = Some(Dir::Right),
                    _ => {}
                }
                if d.is_some() {
                    self.act(d.unwrap(), context);
                }
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, context: &mut Context, dt: f32) {
        self.pats.update(dt as f64);
        self.count += 1.0;
        if self.count > 200.0 {
            self.count = 0.0f64;
        }
        self.pats
            .move_to(10.0 + 5.0 * self.count, 10.0 + 5.0 * self.count, false);
        if self.timeout_auto > 0.4 {
            self.timeout_auto = 0.0;
            self.act(self.dir, context);
        } else {
            self.timeout_auto += dt;
        }
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
