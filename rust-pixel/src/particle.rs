extern crate piston_window;
extern crate rand;

use piston_window::*;
use rand::Rng;
use std::f64::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const MAX_PARTICLES: usize = 1000;

#[derive(Debug, Clone)]
struct Particle {
    location: [f64; 2],
    velocity: [f64; 2],
    gravity: f64,
    radial_accel: f64,
    tangential_accel: f64,
    spin: f64,
    spin_delta: f64,
    size: f64,
    size_delta: f64,
    color: [f64; 4],
    color_delta: [f64; 4],
    age: f64,
    terminal_age: f64,
}

impl Particle {
    fn new() -> Particle {
        Particle {
            location: [0.0, 0.0],
            velocity: [0.0, 0.0],
            gravity: 0.0,
            radial_accel: 0.0,
            tangential_accel: 0.0,
            spin: 0.0,
            spin_delta: 0.0,
            size: 0.0,
            size_delta: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
            color_delta: [0.1, 0.05, 0.03, 0.2],
            age: 0.0,
            terminal_age: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct ParticleSystemInfo {
    emission_rate: f64,
    lifetime: f64,
    particle_life_min: f64,
    particle_life_max: f64,
    direction: f64,
    spread: f64,
    relative: bool,
    speed_min: f64,
    speed_max: f64,
    gravity_min: f64,
    gravity_max: f64,
    radial_accel_min: f64,
    radial_accel_max: f64,
    tangential_accel_min: f64,
    tangential_accel_max: f64,
    size_start: f64,
    size_end: f64,
    size_var: f64,
    spin_start: f64,
    spin_end: f64,
    spin_var: f64,
    color_start: [f64; 4],
    color_end: [f64; 4],
    color_var: f64,
    alpha_var: f64,
}

struct ParticleSystem {
    info: ParticleSystemInfo,
    particles: Vec<Particle>,
    emission_residue: f64,
    age: f64,
    location: [f64; 2],
    prev_location: [f64; 2],
}

impl ParticleSystem {
    fn new(info: ParticleSystemInfo) -> ParticleSystem {
        ParticleSystem {
            info,
            particles: Vec::with_capacity(MAX_PARTICLES),
            emission_residue: 0.0,
            age: -2.0,
            location: [0.0, 0.0],
            prev_location: [0.0, 0.0],
        }
    }

    fn update(&mut self, delta_time: f64) {
        // Update system age
        if self.age >= 0.0 {
            self.age += delta_time;
            if self.age >= self.info.lifetime {
                self.age = -2.0;
            }
        }
        let l0 = self.location[0];
        let l1 = self.location[1];

        // Update particles
        self.particles.retain_mut(|p| {
            p.age += delta_time;
            if p.age >= p.terminal_age {
                false
            } else {
                let mut accel = [
                    p.location[0] - l0,
                    p.location[1] - l1,
                ];
                let len = (accel[0] * accel[0] + accel[1] * accel[1]).sqrt();
                if len != 0.0 {
                    accel[0] /= len;
                    accel[1] /= len;
                }
                let tangential_accel = [
                    -accel[1] * p.tangential_accel,
                    accel[0] * p.tangential_accel,
                ];
                accel[0] *= p.radial_accel;
                accel[1] *= p.radial_accel;

                p.velocity[0] += (accel[0] + tangential_accel[0]) * delta_time;
                p.velocity[1] += (accel[1] + tangential_accel[1]) * delta_time;
                p.velocity[1] += p.gravity * delta_time;

                p.location[0] += p.velocity[0] * delta_time;
                p.location[1] += p.velocity[1] * delta_time;

                p.spin += p.spin_delta * delta_time;
                p.size += p.size_delta * delta_time;

                for i in 0..4 {
                    p.color[i] += p.color_delta[i] * delta_time;
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
                p.terminal_age = rand::thread_rng()
                    .gen_range(self.info.particle_life_min..self.info.particle_life_max);

                p.location = [
                    self.prev_location[0]
                        + (self.location[0] - self.prev_location[0])
                            * rand::thread_rng().gen_range(0.0..1.0),
                    self.prev_location[1]
                        + (self.location[1] - self.prev_location[1])
                            * rand::thread_rng().gen_range(0.0..1.0),
                ];

                let angle = self.info.direction - PI / 2.0
                    + rand::thread_rng().gen_range(0.0..self.info.spread)
                    - self.info.spread / 2.0;
                let speed = rand::thread_rng().gen_range(self.info.speed_min..self.info.speed_max);

                p.velocity = [angle.cos() * speed, angle.sin() * speed];
                p.gravity =
                    rand::thread_rng().gen_range(self.info.gravity_min..self.info.gravity_max);
                p.radial_accel = rand::thread_rng()
                    .gen_range(self.info.radial_accel_min..self.info.radial_accel_max);
                p.tangential_accel = rand::thread_rng()
                    .gen_range(self.info.tangential_accel_min..self.info.tangential_accel_max);

                p.size = rand::thread_rng().gen_range(
                    self.info.size_start
                        ..(self.info.size_start
                            + (self.info.size_end - self.info.size_start) * self.info.size_var),
                );
                p.size_delta = (self.info.size_end - p.size) / p.terminal_age;

                p.spin = rand::thread_rng().gen_range(
                    self.info.spin_start
                        ..(self.info.spin_start
                            + (self.info.spin_end - self.info.spin_start) * self.info.spin_var),
                );
                p.spin_delta = (self.info.spin_end - p.spin) / p.terminal_age;

                for i in 0..4 {
                    p.color[i] = rand::thread_rng().gen_range(
                        self.info.color_start[i]
                            ..(self.info.color_start[i]
                                + (self.info.color_end[i] - self.info.color_start[i])
                                    * self.info.color_var),
                    );
                    p.color_delta[i] =
                        (self.info.color_end[i] - p.color[i]) / p.terminal_age;
                }

                self.particles.push(p);
            }
        }

        self.prev_location = self.location;
    }

    fn render(&self, c: Context, g: &mut G2d) {
        for p in &self.particles {
            let color = [
                p.color[0] as f32,
                p.color[1] as f32,
                p.color[2] as f32,
                p.color[3] as f32,
            ];
            ellipse(
                color,
                [
                    p.location[1],
                    p.location[0],
                    p.size,
                    p.size,
                ],
                c.transform,
                g,
            );
        }
    }

    fn fire_at(&mut self, x: f64, y: f64) {
        self.stop();
        self.move_to(x, y);
        self.fire();
    }

    fn fire(&mut self) {
        if self.info.lifetime == -1.0 {
            self.age = -1.0;
        } else {
            self.age = 0.0;
        }
    }

    fn stop(&mut self) {
        self.age = -2.0;
        self.particles.clear();
    }

    fn move_to(&mut self, x: f64, y: f64) {
        self.prev_location = if self.age == -2.0 {
            [x, y]
        } else {
            self.location
        };
        self.location = [x, y];
    }
}

fn main() {
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
        gravity_min: 9.0,
        gravity_max: 10.0,
        radial_accel_min: 3.0,
        radial_accel_max: 5.0,
        tangential_accel_min: 1.0,
        tangential_accel_max: 5.0,
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

    let mut particle_system = ParticleSystem::new(particle_system_info);

    let mut window: PistonWindow = WindowSettings::new("Particle System", [WIDTH, HEIGHT])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut mx: f64 = 0.0;
    let mut my: f64 = 0.0;

    while let Some(event) = window.next() {
        match event {
            Event::Input(Input::Move(Motion::MouseCursor(mousepos_args)), _timestamp_not_used) =>
            {
                mx = mousepos_args[0];
                my = mousepos_args[1];
            }
            _ => {

            }
        }
        if let Some(Button::Mouse(MouseButton::Left)) = event.press_args() {
            println!("22222PPP...{:?}", event);
            println!("1111111111111111PPP");
            particle_system.fire_at(mx, my);
        }

        if let Some(UpdateArgs { dt }) = event.update_args() {
            particle_system.update(dt);
        }

        window.draw_2d(&event, |c, g, _| {
            clear([0.0, 0.0, 0.0, 1.0], g);
            particle_system.render(c, g);
        });
    }
}
