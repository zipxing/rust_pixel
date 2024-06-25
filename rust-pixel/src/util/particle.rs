use crate::util::{
    objpool::{GObj, GameObjPool},
    Rand,
};
use std::f64::consts::PI;

// const WIDTH: u32 = 800;
// const HEIGHT: u32 = 600;
const MAX_PARTICLES: usize = 1000;

#[derive(Default, Debug, Clone)]
pub struct Particle {
    pub ptype: u8,
    // location
    pub loc: [f64; 2],
    // velocity
    pub v: [f64; 2],
    // gravity
    pub g: f64,
    // radial accel
    pub rad_a: f64,
    // tangential accel
    pub tan_a: f64,
    pub spin: f64,
    pub spin_dt: f64,
    pub size: f64,
    pub size_dt: f64,
    pub color: [f64; 4],
    pub color_dt: [f64; 4],
    pub age: f64,
    pub term_age: f64,
}

impl GObj for Particle {
    fn new() -> Self {
        Default::default()
    }

    fn reset(&mut self, ptype: u8, pv: &Vec<u32>) {
        self.ptype = ptype;
        let mut idx = 0usize;
        self.age = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.term_age = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.loc[0] = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.loc[1] = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.v[0] = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.v[1] = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.g = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.rad_a = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.tan_a = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.size = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.size_dt = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.spin = pv[idx] as f64 / 1000.0;
        idx += 1;
        self.spin_dt = pv[idx] as f64 / 1000.0;
        idx += 1;
        for i in 0..4 {
            self.color[i] = pv[idx] as f64 / 1000.0;
            idx += 1;
            self.color_dt[i] = pv[idx] as f64 / 1000.0;
            idx += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParticleSystemInfo {
    pub emission_rate: f64,
    pub lifetime: f64,
    pub particle_life_min: f64,
    pub particle_life_max: f64,
    pub direction: f64,
    pub spread: f64,
    pub relative: bool,
    pub speed_min: f64,
    pub speed_max: f64,
    pub g_min: f64,
    pub g_max: f64,
    pub rad_a_min: f64,
    pub rad_a_max: f64,
    pub tan_a_min: f64,
    pub tan_a_max: f64,
    pub size_start: f64,
    pub size_end: f64,
    pub size_var: f64,
    pub spin_start: f64,
    pub spin_end: f64,
    pub spin_var: f64,
    pub color_start: [f64; 4],
    pub color_end: [f64; 4],
    pub color_var: f64,
    pub alpha_var: f64,
}

pub struct ParticleSystem {
    pub info: ParticleSystemInfo,
    pub rnd: Rand,
    pub particles: GameObjPool<Particle>,
    pub emission_residue: f64,
    pub age: f64,
    pub loc: [f64; 2],
    pub prev_loc: [f64; 2],
}

impl ParticleSystem {
    pub fn new(info: ParticleSystemInfo) -> ParticleSystem {
        let mut rnd = Rand::new();
        rnd.srand_now();
        ParticleSystem {
            info,
            rnd,
            particles: GameObjPool::<Particle>::new("PARTICLE", MAX_PARTICLES),
            emission_residue: 0.0,
            age: -2.0,
            loc: [0.0, 0.0],
            prev_loc: [0.0, 0.0],
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        // Update system age
        if self.age >= 0.0 {
            self.age += delta_time;
            if self.age >= self.info.lifetime {
                self.age = -2.0;
            }
        }
        let l0 = self.loc[0];
        let l1 = self.loc[1];

        // Update particles
        self.particles.update_active(|po| {
            let p = &mut po.obj;
            p.age += delta_time;
            po.active = if p.age >= p.term_age {
                false
            } else {
                let mut accel = [p.loc[0] - l0, p.loc[1] - l1];
                let len = (accel[0] * accel[0] + accel[1] * accel[1]).sqrt();
                if len != 0.0 {
                    accel[0] /= len;
                    accel[1] /= len;
                }
                let tan_a = [-accel[1] * p.tan_a, accel[0] * p.tan_a];
                accel[0] *= p.rad_a;
                accel[1] *= p.rad_a;

                p.v[0] += (accel[0] + tan_a[0]) * delta_time;
                p.v[1] += (accel[1] + tan_a[1]) * delta_time;
                p.v[1] += p.g * delta_time;

                p.loc[0] += p.v[0] * delta_time;
                p.loc[1] += p.v[1] * delta_time;

                p.spin += p.spin_dt * delta_time;
                p.size += p.size_dt * delta_time;

                for i in 0..4 {
                    p.color[i] += p.color_dt[i] * delta_time;
                }

                true
            }
        });

        // Emit new particles
        if self.age != -2.0 {
            let particles_needed = self.info.emission_rate * delta_time + self.emission_residue;
            let particles_to_create = particles_needed.floor() as usize;
            self.emission_residue = particles_needed - particles_to_create as f64;

            for _ in 0..particles_to_create {
                let mut pv: Vec<u32> = vec![];
                let age = 0.0;
                pv.push((age * 1000.0) as u32);
                let term_age = self
                    .rnd
                    .gen_range(self.info.particle_life_min, self.info.particle_life_max);
                pv.push((term_age * 1000.0) as u32);

                let loc0 = self.prev_loc[0]
                    + (self.loc[0] - self.prev_loc[0]) * self.rnd.gen_range(0.0, 1.0);
                pv.push((loc0 * 1000.0) as u32);
                let loc1 = self.prev_loc[1]
                    + (self.loc[1] - self.prev_loc[1]) * self.rnd.gen_range(0.0, 1.0);
                pv.push((loc1 * 1000.0) as u32);

                let angle = self.info.direction - PI / 2.0
                    + self.rnd.gen_range(0.0, self.info.spread)
                    - self.info.spread / 2.0;
                let speed = self.rnd.gen_range(self.info.speed_min, self.info.speed_max);

                let v0 = angle.cos() * speed;
                pv.push((v0 * 1000.0) as u32);
                let v1 = angle.sin() * speed;
                pv.push((v1 * 1000.0) as u32);
                let g = self.rnd.gen_range(self.info.g_min, self.info.g_max);
                pv.push((g * 1000.0) as u32);
                let rad_a = self.rnd.gen_range(self.info.rad_a_min, self.info.rad_a_max);
                pv.push((rad_a * 1000.0) as u32);
                let tan_a = self.rnd.gen_range(self.info.tan_a_min, self.info.tan_a_max);
                pv.push((tan_a * 1000.0) as u32);

                // size...
                let size = self.rnd.gen_range(
                    self.info.size_start,
                    self.info.size_start
                        + (self.info.size_end - self.info.size_start) * self.info.size_var,
                );
                pv.push((size * 1000.0) as u32);
                // size_dt...
                let size_dt = (self.info.size_end - size) / term_age;
                pv.push((size_dt * 1000.0) as u32);

                let spin = self.rnd.gen_range(
                    self.info.spin_start,
                    self.info.spin_start
                        + (self.info.spin_end - self.info.spin_start) * self.info.spin_var,
                );
                pv.push((spin * 1000.0) as u32);
                let spin_dt = (self.info.spin_end - spin) / term_age;
                pv.push((spin_dt * 1000.0) as u32);

                for i in 0..4 {
                    let color = self.rnd.gen_range(
                        self.info.color_start[i],
                        self.info.color_start[i]
                            + (self.info.color_end[i] - self.info.color_start[i])
                                * self.info.color_var,
                    );
                    pv.push((color * 1000.0) as u32);
                    let color_dt = (self.info.color_end[i] - color) / term_age;
                    pv.push((color_dt * 1000.0) as u32);
                }

                self.particles.create(0, &pv);
            }
        }

        self.prev_loc = self.loc;
    }

    pub fn fire_at(&mut self, x: f64, y: f64) {
        self.stop();
        self.move_to(x, y, false);
        self.fire();
    }

    pub fn fire(&mut self) {
        if self.info.lifetime == -1.0 {
            self.age = -1.0;
        } else {
            self.age = 0.0;
        }
    }

    pub fn stop(&mut self) {
        self.age = -2.0;
        self.particles.pool.clear();
    }

    pub fn move_to(&mut self, x: f64, y: f64, b_move_particles: bool) {
        if b_move_particles {
            let dx = x - self.loc[0];
            let dy = y - self.loc[1];

            for p in &mut self.particles.pool {
                if p.active {
                    p.obj.loc[0] += dx;
                    p.obj.loc[1] += dy;
                }
            }

            self.prev_loc[0] += dx;
            self.prev_loc[1] += dy;
        } else {
            if self.age == -2.0 {
                self.prev_loc = [x, y];
            } else {
                self.prev_loc = self.loc;
            }
        }

        self.loc = [x, y];
    }
}

