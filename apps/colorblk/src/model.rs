use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
};
// use log::info;
use colorblk_lib::{ColorblkData, Block, Gate, Direction};
use colorblk_lib::solver::solve_main;

pub const CARDW: usize = 7;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub const CARDH: usize = 7;
#[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
pub const CARDH: usize = 5;
pub const COLORBLKW: u16 = 80;
pub const COLORBLKH: u16 = 40;

#[repr(u8)]
enum ColorblkState {
    Solving,
    Normal,
}

pub struct ColorblkModel {
    // ColorblkData defined in colorblk/lib/src/lib.rs
    pub data: ColorblkData,
    pub count: f64,
    pub card: u8,
    // 存储计算结果的字段
    pub initial_blocks: Vec<Block>,
    pub gates: Vec<Gate>,
    pub solution: Option<Vec<(u8, Option<Direction>, u8)>>, // 存储移动步骤
    pub current_step: usize, // 当前执行到哪一步
}

impl ColorblkModel {
    pub fn new() -> Self {
        Self {
            data: ColorblkData::new(),
            count: 0.0,
            card: 0,
            initial_blocks: Vec::new(),
            gates: Vec::new(),
            solution: None,
            current_step: 0,
        }
    }
}

impl Model for ColorblkModel {
    fn init(&mut self, _context: &mut Context) {
        self.data.shuffle();
        self.card = self.data.next();

        // Emit event...
        event_emit("Colorblk.RedrawTile");

        // 保存初始布局和门
        let (blocks, gates, solution) = solve_main();
        self.initial_blocks = blocks;
        self.gates = gates;
        self.solution = solution;
        self.current_step = 0;
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    KeyCode::Char('s') => {
                        self.data.shuffle();
                        self.card = self.data.next();
                        // Emit event...
                        event_emit("Colorblk.RedrawTile");
                    }
                    KeyCode::Char('n') => {
                        self.card = self.data.next();
                        // Emit event...
                        event_emit("Colorblk.RedrawTile");
                    }
                    _ => {
                        context.state = ColorblkState::Solving as u8;
                    }
                },
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {
        self.count += 1.0;
        if self.count > 200.0 {
            self.count = 0.0f64;
        }
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
