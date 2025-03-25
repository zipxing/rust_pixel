use colorblk_lib::solver::solve_main;
use colorblk_lib::{Block, ColorblkData, Direction, Gate, BOARD_HEIGHT, BOARD_WIDTH, SHAPE};
use log::info;
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
};

// pub const CARDW: usize = 7;
// #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
// pub const CARDH: usize = 7;
// #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
// pub const CARDH: usize = 5;
pub const COLORBLKW: u16 = 70;
pub const COLORBLKH: u16 = 40;
pub const CELLW: usize = 10;
pub const CELLH: usize = 5;

fn calculate_border_type(grid: &[[u8; 5]; 5], x: usize, y: usize) -> u8 {
    // 检查四个方向的邻居
    let mut border_bits = 0u8;

    // 上邻居
    if y == 0 || grid[y - 1][x] == 0 {
        border_bits |= 0b1000;
    }

    // 下邻居
    if y == 4 || grid[y + 1][x] == 0 {
        border_bits |= 0b0100;
    }

    // 左邻居
    if x == 0 || grid[y][x - 1] == 0 {
        border_bits |= 0b0010;
    }

    // 右邻居
    if x == 4 || grid[y][x + 1] == 0 {
        border_bits |= 0b0001;
    }
    border_bits
}

#[repr(u8)]
enum ColorblkState {
    Normal,
}

pub struct ColorblkModel {
    // ColorblkData defined in colorblk/lib/src/lib.rs
    pub data: ColorblkData,
    pub count: i16,
    pub card: i16,
    // 存储计算结果的字段
    pub initial_blocks: Vec<Block>,
    pub gates: Vec<Gate>,
    pub solution: Option<Vec<(u8, Option<Direction>, u8)>>, // 存储移动步骤
    pub current_step: usize,                                // 当前执行到哪一步
    pub is_paused: bool,
    pub animation_speed: f32,
    // 存储每个格子的渲染状态 (border_type, color, symbol)
    pub render_state: Vec<(u8, i8, &'static str)>,
}

impl ColorblkModel {
    pub fn new() -> Self {
        Self {
            data: ColorblkData::new(),
            count: 0,
            card: 0,
            initial_blocks: Vec::new(),
            gates: Vec::new(),
            solution: None,
            current_step: 0,
            is_paused: false,
            animation_speed: 1.0,
            render_state: vec![(0, -1, ""); (BOARD_WIDTH * BOARD_HEIGHT) as usize],
        }
    }

    pub fn reset(&mut self) {
        self.data = ColorblkData::new();
        self.count = 0;
        self.card = 0;
        self.initial_blocks.clear();
        self.gates.clear();
        self.solution = None;
        self.current_step = 0;
        self.is_paused = false;
        self.animation_speed = 1.0;
        self.render_state = vec![(0, -1, ""); (BOARD_WIDTH * BOARD_HEIGHT) as usize];
    }

    pub fn init(&mut self, data: ColorblkData) {
        self.data = data;
        // 从data中获取初始状态
        self.count = 0;
        self.card = 0;
        self.initial_blocks = Vec::new();
        self.gates = Vec::new();
        self.solution = None;
        self.current_step = 0;
        self.is_paused = false;
        self.animation_speed = 1.0;
        self.render_state = vec![(0, -1, ""); (BOARD_WIDTH * BOARD_HEIGHT) as usize];
        self.update_render_state();
    }

    pub fn set_solution(&mut self, solution: Vec<(u8, Option<Direction>, u8)>) {
        self.solution = Some(solution);
        self.current_step = 0;
        self.update_render_state();
    }

    pub fn next_step(&mut self) -> bool {
        if let Some(solution) = &self.solution {
            if self.current_step < solution.len() {
                self.current_step += 1;
                self.update_render_state();
                return true;
            }
        }
        false
    }

    pub fn prev_step(&mut self) -> bool {
        if self.current_step > 0 {
            self.current_step -= 1;
            self.update_render_state();
            true
        } else {
            false
        }
    }

    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.animation_speed = speed;
    }

    // 更新渲染状态
    fn update_render_state(&mut self) {
        // 重置渲染状态
        self.render_state = vec![(0, -1, ""); (BOARD_WIDTH * BOARD_HEIGHT) as usize];

        // 创建一个数组来跟踪每个方块的当前位置和是否被移除
        let mut current_positions: Vec<(i16, i16, bool)> = self
            .initial_blocks
            .iter()
            .map(|block| (block.x as i16, block.y as i16, true))
            .collect();

        // 根据solution更新方块位置
        if let Some(solution) = &self.solution {
            for step in 0..self.current_step {
                let (block_id, direction, mstep) = solution[step];
                if let Some(dir) = direction {
                    let (dx, dy) = match dir {
                        Direction::Up => (0, -1),
                        Direction::Down => (0, 1),
                        Direction::Left => (-1, 0),
                        Direction::Right => (1, 0),
                    };
                    for _ in 0..mstep {
                        if let Some(pos) = current_positions.get_mut(block_id as usize - 1) {
                            pos.0 += dx;
                            pos.1 += dy;
                        }
                    }
                } else {
                    // 标记方块为已移除
                    if let Some(pos) = current_positions.get_mut(block_id as usize - 1) {
                        pos.2 = false;
                    }
                }
            }
        }
        info!("current_pos....{:?}", current_positions);

        // 更新渲染状态
        for (block, &(current_x, current_y, is_active)) in
            self.initial_blocks.iter().zip(current_positions.iter())
        {
            if !is_active {
                continue;
            }

            let shape_data = &SHAPE[block.shape as usize];

            // 遍历形状的每个格子
            for grid_y in 0..5 {
                for grid_x in 0..5 {
                    if shape_data.grid[grid_y][grid_x] == 1 {
                        // 计算棋盘上的实际坐标
                        let board_x = current_x as usize + (grid_x - shape_data.rect.x);
                        let board_y = current_y as usize + (grid_y - shape_data.rect.y);

                        if board_x < BOARD_WIDTH as usize && board_y < BOARD_HEIGHT as usize {
                            let idx = board_y * BOARD_WIDTH as usize + board_x;
                            // 计算边框类型
                            let border_type =
                                calculate_border_type(&shape_data.grid, grid_x, grid_y);
                            self.render_state[idx] = (border_type as u8, block.color as i8, "");
                        }
                    }
                }
            }
        }
        info!("render_state....{:?}", self.render_state);
    }
}

impl Model for ColorblkModel {
    fn init(&mut self, context: &mut Context) {
        self.data.shuffle();
        self.card = self.data.next() as i16;

        // Emit event...
        event_emit("Colorblk.RedrawTile");

        // 保存初始布局和门
        context.state = ColorblkState::Normal as u8;
        let (blocks, gates, solution) = solve_main();
        self.initial_blocks = blocks;
        self.gates = gates;
        self.solution = solution;
        info!("solution....{:?}", self.solution);
        self.current_step = 0;
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    KeyCode::Char('s') => {
                        self.data.shuffle();
                        self.card = self.data.next() as i16;
                        // Emit event...
                        event_emit("Colorblk.RedrawTile");
                    }
                    KeyCode::Char('n') => {
                        self.card = self.data.next() as i16;
                        // Emit event...
                        event_emit("Colorblk.RedrawTile");
                    }
                    _ => {
                        context.state = ColorblkState::Normal as u8;
                    }
                },
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {
        self.count = (self.count + 1) % 20;
        if self.count == 0 {
            // self.current_step += 1;
            self.next_step();
        }
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
