// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Game encapsulate Model and Render classes and implements the main loop
//! Be aware that all the Game, Model and Render instances have the same lifetime
//!
//! # Example
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!    init_log(log::LevelFilter::Info, "log/snake.log");
//!    info!("Snake(rust_pixel) start...");
//!    let ad = Audio::new();
//!    ad.play_file("assets/snake/back.mp3", true);
//!    let m = SnakeModel::new();
//!    let r = SnakeRender::new();
//!    let mut g = Game::new(m, r);
//!    g.init();
//!    g.run()?;
//!    g.render.term.reset(&mut g.context);
//!    Ok(())
//! }

use crate::{context::Context, event::timer_update, log::init_log, GAME_FRAME, LOGO_FRAME};
use log::info;
use std::{
    io,
    time::{Duration, Instant},
};

/// The Model interface, main entrance for data and core logic
pub trait Model {
    fn init(&mut self, ctx: &mut Context);
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        // render logo movie...
        if ctx.stage <= LOGO_FRAME {
            return;
        }
        timer_update();
        self.handle_event(ctx, dt);
        self.handle_timer(ctx, dt);
        self.handle_input(ctx, dt);
        self.handle_auto(ctx, dt);
    }
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32);
    fn handle_event(&mut self, ctx: &mut Context, dt: f32);
    fn handle_input(&mut self, ctx: &mut Context, dt: f32);
    fn handle_auto(&mut self, ctx: &mut Context, dt: f32);
}

/// The Render interface, takes context and model as input params. It renders every single frame
pub trait Render {
    type Model: Model;

    fn init(&mut self, ctx: &mut Context, m: &mut Self::Model);
    fn update(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32) {
        self.handle_event(ctx, m, dt);
        self.handle_timer(ctx, m, dt);
        self.draw(ctx, m, dt);
    }
    fn handle_event(&mut self, ctx: &mut Context, model: &mut Self::Model, dt: f32);
    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, dt: f32);
    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, dt: f32);
}

/// Game encapsulates a Model，a Render and a Context structure
pub struct Game<M, R>
where
    M: Model,
    R: Render<Model = M>,
{
    pub context: Context,
    pub model: M,
    pub render: R,
}

impl<M, R> Game<M, R>
where
    M: Model,
    R: Render<Model = M>,
{
    pub fn new(m: M, r: R, name: &str, project_path: &str) -> Self {
        let ap = project_path.to_string();
        let ctx = Context::new(name, &ap);
        init_log(
            log::LevelFilter::Trace,
            &format!("log{}{}.log", std::path::MAIN_SEPARATOR, name),
        );
        info!("{}(rust_pixel) start...{:?}", name, &ap);
        Self {
            context: ctx,
            model: m,
            render: r,
        }
    }

    /// Main loop, polling input events, processing timer and other events.
    /// It also calls tick at a constant framerate per second, executing the
    /// update method of model and render.
    pub fn run(&mut self) -> io::Result<()> {
        info!("Begin run...");

        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_nanos(1_000_000_000 / GAME_FRAME as u64);

        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_nanos(100));

            if self
                .context
                .adapter
                .poll_event(timeout, &mut self.context.input_events)
            {
                return Ok(());
            }

            let et = last_tick.elapsed();
            
            if et >= tick_rate {
                let dt = et.as_secs() as f32 + et.subsec_nanos() as f32 / 1_000_000_000.0;
                self.on_tick(dt);
                last_tick = Instant::now();
            }
        }
    }

    /// calls every frame, update timer, model logic and does rendering
    pub fn on_tick(&mut self, dt: f32) {
        self.context.stage += 1;
        self.model.update(&mut self.context, dt);
        self.render.update(&mut self.context, &mut self.model, dt);
    }

    /// init render and model
    pub fn init(&mut self) {
        info!("Init game...");
        self.model.init(&mut self.context);
        self.render.init(&mut self.context, &mut self.model);
    }
}

#[macro_export]
macro_rules! only_terminal_mode {
    () => {
        #[cfg(any(feature = "sdl", feature = "wgpu", feature = "winit", target_arch = "wasm32"))]
        {
            println!("Run in terminal only...");
            std::process::exit(0);
        }
    };
}

#[macro_export]
macro_rules! only_graphics_mode {
    () => {
        #[cfg(not(any(feature = "sdl", feature = "wgpu", feature = "winit", target_arch = "wasm32")))]
        {
            println!("Run in graphics only...");
            std::process::exit(0);
        }
    };
}
