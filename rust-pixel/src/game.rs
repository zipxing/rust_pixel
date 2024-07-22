// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! Game encapsulate Model and Render classes
//! and implements the main loop
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
    any::Any,
    io,
    time::{Duration, Instant},
};

/// The Model interface, main entrance for data and core logic
/// as_any method is to downcast to game instance's own model implementation.
pub trait Model {
    fn init(&mut self, ctx: &mut Context);
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        //头几帧用于绘制logo
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
    fn as_any(&mut self) -> &mut dyn Any;
}

/// The Render interface, takes context and model as input params. It renders every single frame
/// The model param needs to be downcast to a GameModel instance
/// for example:
/// let gm = model.as_any().downcast_mut::<SnakeModel>().unwrap();
/// For decoupling reason，render can not be accessed from model，so as_any method is not
/// included here
pub trait Render {
    fn init<G: Model>(&mut self, ctx: &mut Context, m: &mut G);
    fn update<G: Model>(&mut self, ctx: &mut Context, m: &mut G, dt: f32) {
        self.handle_event(ctx, m, dt);
        self.handle_timer(ctx, m, dt);
        self.draw(ctx, m, dt);
    }
    fn handle_event<G: Model>(&mut self, ctx: &mut Context, model: &mut G, dt: f32);
    fn handle_timer<G: Model>(&mut self, ctx: &mut Context, model: &mut G, dt: f32);
    fn draw<G: Model>(&mut self, ctx: &mut Context, model: &mut G, dt: f32);
}

/// Game encapsulates a Model，a Render and a Context structure
pub struct Game<M, R>
where
    M: Model,
    R: Render,
{
    pub context: Context,
    pub model: M,
    pub render: R,
}

impl<M, R> Game<M, R>
where
    M: Model,
    R: Render,
{
    pub fn new(m: M, r: R, name: &str) -> Self {
        Self::new_with_project_path(m, r, name, None)
    }

    pub fn new_with_project_path(m: M, r: R, name: &str, project_path: Option<&str>) -> Self {
        let res: Vec<String> = name.to_string().split("/").map(|s| s.to_string()).collect();
        let path_name;
        let app_name;
        match res.len() {
            1 => {
                path_name = "games";
                app_name = name;
            }
            2 => {
                path_name = &res[0];
                app_name = &res[1];
            }
            _ => {
                path_name = "games";
                app_name = name;
            }
        };
        // If app embbed in rust_pixel directory, default asset path is 
        // "games/game_name/" or "apps/app_name/"
        // else you can set asset_path by yourself
        // example:
        // context.set_asset_path("./")...
        let ap = if project_path == None {
            format!("{}{}{}", path_name, std::path::MAIN_SEPARATOR, app_name).to_string()
        } else {
            project_path.unwrap().to_string()
        };
        let ctx = Context::new(path_name, app_name, &ap);
        init_log(
            log::LevelFilter::Info,
            &format!("log{}{}.log", std::path::MAIN_SEPARATOR, app_name),
        );
        info!("{}(rust_pixel) start...{:?}", name, res);
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
        let tick_rate = Duration::from_nanos(1000_000_000 / GAME_FRAME as u64);

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
                let dt = et.as_secs() as f32 + et.subsec_nanos() as f32 / 1000_000_000.0;
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
