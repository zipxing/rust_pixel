use crate::util::{
    Rand, 
    PointU16,
    objpool::{GObj, GameObjPool},
};
use std::f64::consts::PI;

// const WIDTH: u32 = 800;
// const HEIGHT: u32 = 600;
const MAX_PARTICLES: usize = 1000;

#[derive(Debug, Clone)]
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

impl Particle {
    pub fn new_d() -> Particle {
        Particle {
            ptype: 0,
            loc: [0.0, 0.0],
            v: [0.0, 0.0],
            g: 0.0,
            rad_a: 0.0,
            tan_a: 0.0,
            spin: 0.0,
            spin_dt: 0.0,
            size: 0.0,
            size_dt: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
            color_dt: [0.1, 0.05, 0.03, 0.2],
            age: 0.0,
            term_age: 0.0,
        }
    }
}

impl GObj for Particle {
    fn new(ptype: u8, ps: &Vec<PointU16>) -> Bullet {
        let mut bt = Particle::new_d(); 
        bt.reset(ptype, ps);
        bt
    }

    fn reset(&mut self, ptype: u8, ps: &Vec<PointU16>) {
        self.ptype = ptype;
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
            let mut p = po.obj;
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
                if self.particles.len() >= MAX_PARTICLES {
                    break;
                }

                let mut p = Particle::new();
                p.age = 0.0;
                p.term_age = self
                    .rnd
                    .gen_range(self.info.particle_life_min, self.info.particle_life_max);

                p.loc = [
                    self.prev_loc[0]
                        + (self.loc[0] - self.prev_loc[0]) * self.rnd.gen_range(0.0, 1.0),
                    self.prev_loc[1]
                        + (self.loc[1] - self.prev_loc[1]) * self.rnd.gen_range(0.0, 1.0),
                ];

                let angle = self.info.direction - PI / 2.0
                    + self.rnd.gen_range(0.0, self.info.spread)
                    - self.info.spread / 2.0;
                let speed = self.rnd.gen_range(self.info.speed_min, self.info.speed_max);

                p.v = [angle.cos() * speed, angle.sin() * speed];
                p.g = self.rnd.gen_range(self.info.g_min, self.info.g_max);
                p.rad_a = self.rnd.gen_range(self.info.rad_a_min, self.info.rad_a_max);
                p.tan_a = self.rnd.gen_range(self.info.tan_a_min, self.info.tan_a_max);

                p.size = self.rnd.gen_range(
                    self.info.size_start,
                    self.info.size_start
                        + (self.info.size_end - self.info.size_start) * self.info.size_var,
                );
                p.size_dt = (self.info.size_end - p.size) / p.term_age;

                p.spin = self.rnd.gen_range(
                    self.info.spin_start,
                    self.info.spin_start
                        + (self.info.spin_end - self.info.spin_start) * self.info.spin_var,
                );
                p.spin_dt = (self.info.spin_end - p.spin) / p.term_age;

                for i in 0..4 {
                    p.color[i] = self.rnd.gen_range(
                        self.info.color_start[i],
                        self.info.color_start[i]
                            + (self.info.color_end[i] - self.info.color_start[i])
                                * self.info.color_var,
                    );
                    p.color_dt[i] = (self.info.color_end[i] - p.color[i]) / p.term_age;
                }

                self.particles.create(p);
            }
        }

        self.prev_loc = self.loc;
    }

    // fn render(&self, c: Context, g: &mut G2d) {
    //     for p in &self.particles {
    //         let color = [
    //             p.color[0] as f32,
    //             p.color[1] as f32,
    //             p.color[2] as f32,
    //             p.color[3] as f32,
    //         ];
    //         ellipse(
    //             color,
    //             [
    //                 p.loc[1],
    //                 p.loc[0],
    //                 p.size,
    //                 p.size,
    //             ],
    //             c.transform,
    //             g,
    //         );
    //     }
    // }

    pub fn fire_at(&mut self, x: f64, y: f64) {
        self.stop();
        self.move_to(x, y);
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
        self.particles.clear();
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.prev_loc = if self.age == -2.0 { [x, y] } else { self.loc };
        self.loc = [x, y];
    }
}

// fn main() {
//     let particle_system_info = ParticleSystemInfo {
//         emission_rate: 100.0,
//         lifetime: -1.0,
//         particle_life_min: 1.0,
//         particle_life_max: 2.0,
//         direction: PI / 2.0,
//         spread: PI / 4.0,
//         relative: false,
//         speed_min: 50.0,
//         speed_max: 100.0,
//         g_min: 9.0,
//         g_max: 10.0,
//         rad_a_min: 3.0,
//         rad_a_max: 5.0,
//         tan_a_min: 1.0,
//         tan_a_max: 5.0,
//         size_start: 1.0,
//         size_end: 5.0,
//         size_var: 1.0,
//         spin_start: 1.0,
//         spin_end: 5.0,
//         spin_var: 1.0,
//         color_start: [0.0, 0.0, 0.0, 0.0],
//         color_end: [1.0, 1.0, 1.0, 1.0],
//         color_var: 0.1,
//         alpha_var: 1.0,
//     };

//     let mut particle_system = ParticleSystem::new(particle_system_info);

//     let mut window: PistonWindow = WindowSettings::new("Particle System", [WIDTH, HEIGHT])
//         .exit_on_esc(true)
//         .build()
//         .unwrap();

//     let mut mx: f64 = 0.0;
//     let mut my: f64 = 0.0;

//     while let Some(event) = window.next() {
//         match event {
//             Event::Input(Input::Move(Motion::MouseCursor(mousepos_args)), _timestamp_not_used) =>
//             {
//                 mx = mousepos_args[0];
//                 my = mousepos_args[1];
//             }
//             _ => {

//             }
//         }
//         if let Some(Button::Mouse(MouseButton::Left)) = event.press_args() {
//             println!("22222PPP, .{:?}", event);
//             println!("1111111111111111PPP");
//             particle_system.fire_at(mx, my);
//         }

//         if let Some(UpdateArgs { dt }) = event.update_args() {
//             particle_system.update(dt);
//         }

//         window.draw_2d(&event, |c, g, _| {
//             clear([0.0, 0.0, 0.0, 1.0], g);
//             particle_system.render(c, g);
//         });
//     }
// }
