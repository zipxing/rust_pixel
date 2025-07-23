use log::info;
use std::env;
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
};

pub const SSFPLAYERW: u16 = 80;
pub const SSFPLAYERH: u16 = 40;

pub struct PixelSsfModel {
    pub ssf_file: String,
    pub frame_idx: usize,
    pub frame_count: usize,
    pub auto_play: bool,
    pub play_speed: f32,
    pub timer_accumulator: f32,
    pub loop_mode: bool,
}

impl PixelSsfModel {
    pub fn new() -> Self {
        // 从命令行参数获取SSF文件路径
        let args: Vec<String> = env::args().collect();
        
        // cargo-pixel传递参数格式: program_name project_path ssf_file_path
        // 或者直接运行: program_name [ssf_file_path]
        let ssf_file = if args.len() >= 3 {
            // cargo-pixel模式: args[1]是项目路径, args[2]是SSF文件
            let path = args[2].clone();
            // asset2sprite宏会自动添加"assets/"前缀，所以我们需要去除它
            if path.starts_with("assets/") {
                path.strip_prefix("assets/").unwrap().to_string()
            } else {
                path
            }
        } else if args.len() == 2 {
            // 直接运行模式: args[1]是SSF文件路径
            let path = args[1].clone();
            // asset2sprite宏会自动添加"assets/"前缀，所以我们需要去除它
            if path.starts_with("assets/") {
                path.strip_prefix("assets/").unwrap().to_string()
            } else {
                path
            }
        } else {
            // 默认文件
            "sdq/dance.ssf".to_string()
        };

        info!("Command line args: {:?}", args);
        info!("Loading SSF file: {}", ssf_file);

        Self {
            ssf_file,
            frame_idx: 0,
            frame_count: 0,
            auto_play: true,
            play_speed: 0.1, // 每0.1秒播放一帧
            timer_accumulator: 0.0,
            loop_mode: true,
        }
    }

    pub fn next_frame(&mut self) {
        if self.frame_count > 0 {
            self.frame_idx = (self.frame_idx + 1) % self.frame_count;
            event_emit("PixelSsf.UpdateFrame");
        }
    }

    pub fn prev_frame(&mut self) {
        if self.frame_count > 0 {
            self.frame_idx = if self.frame_idx == 0 {
                self.frame_count - 1
            } else {
                self.frame_idx - 1
            };
            event_emit("PixelSsf.UpdateFrame");
        }
    }

    pub fn reset_frame(&mut self) {
        self.frame_idx = 0;
        event_emit("PixelSsf.UpdateFrame");
    }

    pub fn toggle_auto_play(&mut self) {
        self.auto_play = !self.auto_play;
        info!("Auto play: {}", self.auto_play);
    }

    pub fn toggle_loop_mode(&mut self) {
        self.loop_mode = !self.loop_mode;
        info!("Loop mode: {}", self.loop_mode);
    }

    pub fn set_play_speed(&mut self, speed: f32) {
        self.play_speed = speed.max(0.01).min(2.0);
        info!("Play speed: {:.2}x", 1.0 / self.play_speed);
    }
}

impl Model for PixelSsfModel {
    fn init(&mut self, _context: &mut Context) {
        info!("PixelSSF Player initialized");
        info!("Controls:");
        info!("  Space: Toggle auto play");
        info!("  Left/Right: Previous/Next frame");
        info!("  R: Reset to first frame");
        info!("  L: Toggle loop mode");
        info!("  +/-: Increase/Decrease speed");
        info!("  Q: Quit");
    }

    fn handle_timer(&mut self, _ctx: &mut Context, dt: f32) {
        if self.auto_play && self.frame_count > 0 {
            self.timer_accumulator += dt;
            if self.timer_accumulator >= self.play_speed {
                self.timer_accumulator = 0.0;
                
                if self.loop_mode {
                    self.next_frame();
                } else {
                    if self.frame_idx < self.frame_count - 1 {
                        self.next_frame();
                    } else {
                        self.auto_play = false; // 停止播放当到达最后一帧
                    }
                }
            }
        } else if self.frame_count == 0 {
            // 调试信息：如果frame_count为0，每秒打印一次状态
            static mut DEBUG_COUNTER: f32 = 0.0;
            unsafe {
                DEBUG_COUNTER += dt;
                if DEBUG_COUNTER >= 1.0 {
                    DEBUG_COUNTER = 0.0;
                    info!("DEBUG: frame_count is 0, auto_play: {}, file: {}", 
                          self.auto_play, self.ssf_file);
                }
            }
        }
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => {
                    println!("DEBUG: Key pressed: {:?}", key.code);
                    match key.code {
                        KeyCode::Char(' ') => {
                            println!("DEBUG: Space key pressed - toggling auto play");
                            self.toggle_auto_play();
                        }
                        KeyCode::Left => {
                            println!("DEBUG: Left arrow - previous frame");
                            self.prev_frame();
                        }
                        KeyCode::Right => {
                            println!("DEBUG: Right arrow - next frame");
                            self.next_frame();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            self.reset_frame();
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => {
                            self.toggle_loop_mode();
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            self.set_play_speed(self.play_speed * 0.8);
                        }
                        KeyCode::Char('-') => {
                            self.set_play_speed(self.play_speed * 1.2);
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                },
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {}
} 