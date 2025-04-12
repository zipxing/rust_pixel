use colorblk_lib::solver::solve_main;
use colorblk_lib::{ColorBlkStage, Direction, Gate, SHAPE};
use log::info;
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
};

pub const COLORBLKW: u16 = 100;
pub const COLORBLKH: u16 = 61;
pub const CELLW: usize = 10;
pub const CELLH: usize = 5;
use crate::level::load_level_from_json;

#[repr(u8)]
enum ColorblkState {
    Normal,
}

pub struct ColorblkModel {
    pub count: i16,
    // 存储初始关卡状态
    pub stage: ColorBlkStage,
    // 存储计算结果的字段
    pub solution: Option<Vec<(Vec<u8>, Option<Direction>, u8)>>,
    pub current_step: usize,
    // 存储门状态（从stage初始化，在处理solution时更新）
    pub gates_state: Vec<Gate>,
    // 存储每个格子的渲染状态 (border_type, color, symbol)
    pub render_state: Vec<(u8, i8)>,
}

impl ColorblkModel {
    pub fn new() -> Self {
        let stage = ColorBlkStage::new(1, 1); // 默认使用5x6的棋盘
        Self {
            count: 0,
            stage,
            solution: None,
            current_step: 0,
            gates_state: Vec::new(),
            render_state: vec![(0, -1); 1 * 1], // 默认大小
        }
    }

    // pub fn set_solution(&mut self, solution: Vec<(u8, Option<Direction>, u8)>) {
    //     self.solution = Some(solution);
    //     self.current_step = 0;
    //     self.update_render_state();
    // }

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

    // pub fn prev_step(&mut self) -> bool {
    //     if self.current_step > 0 {
    //         self.current_step -= 1;
    //         self.update_render_state();
    //         true
    //     } else {
    //         false
    //     }
    // }

    // pub fn toggle_pause(&mut self) {
    //     self.is_paused = !self.is_paused;
    // }

    // pub fn set_speed(&mut self, speed: f32) {
    //     self.animation_speed = speed;
    // }

    // 更新渲染状态
    fn update_render_state(&mut self) {
        // 重置渲染状态
        self.render_state = vec![(0, -1); self.stage.board_width * self.stage.board_height];

        // 复位gates_state为初始状态，然后根据历史步骤更新
        self.gates_state = self.stage.gates.clone();

        // 创建一个数组来跟踪每个方块的当前位置和是否被移除
        let mut current_positions: Vec<(i16, i16, bool)> = self
            .stage
            .blocks
            .iter()
            .map(|block| (block.x as i16, block.y as i16, true))
            .collect();

        // 根据solution更新方块位置和门状态
        if let Some(solution) = &self.solution {
            for step in 0..self.current_step {
                let (block_ids, direction, mstep) = &solution[step];
                if let Some(dir) = direction {
                    let (dx, dy) = match dir {
                        Direction::Up => (0, -1),
                        Direction::Down => (0, 1),
                        Direction::Left => (-1, 0),
                        Direction::Right => (1, 0),
                    };
                    for block_id in block_ids {
                        for _ in 0..*mstep {
                            if let Some(pos) = current_positions.get_mut(*block_id as usize - 1) {
                                pos.0 += dx;
                                pos.1 += dy;
                            }
                        }
                    }
                } else {
                    // 块退出时，需要更新门状态
                    for block_id in block_ids {
                        // 获取方块索引
                        let block_idx = *block_id as usize - 1;

                        if block_idx < self.stage.blocks.len() {
                            let block = &self.stage.blocks[block_idx];

                            // 找到方块通过的门并更新门状态
                            for gate in &mut self.gates_state {
                                if gate.color == block.color && gate.switch {
                                    // 执行门状态变化逻辑
                                    gate.switch = !gate.switch;

                                    // 如果门有锁且方块有钥匙，则解锁
                                    if gate.lock > 0 && block.key > 0 {
                                        gate.lock = 0;
                                    }

                                    break;
                                }
                            }

                            // 处理方块状态
                            if block.color2 != 0 {
                                // 双色方块：改变颜色而不是移除
                                if let Some(_pos) = current_positions.get_mut(block_idx) {
                                    // 不需要标记为移除，只需保持位置不变
                                }
                            } else {
                                // 普通方块：标记为已移除
                                if let Some(pos) = current_positions.get_mut(block_idx) {
                                    pos.2 = false;
                                }
                            }
                        }
                    }
                }
            }
        }
        // info!("current_pos....{:?}", current_positions);
        // info!("current_gates....{:?}", self.gates_state);

        // 更新渲染状态
        for (block, &(current_x, current_y, is_active)) in
            self.stage.blocks.iter().zip(current_positions.iter())
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

                        if board_x < self.stage.board_width && board_y < self.stage.board_height {
                            let idx = board_y * self.stage.board_width + board_x;
                            // 计算边框类型
                            let border_type =
                                calculate_border_type(&shape_data.grid, grid_x, grid_y);
                            self.render_state[idx] = (border_type as u8, block.color as i8);
                        }
                    }
                }
            }
        }

        // 这里可以添加代码来根据self.gates_state渲染门的状态

        // info!("render_state....{:?}", self.render_state);
    }
}

impl Model for ColorblkModel {
    fn init(&mut self, context: &mut Context) {
        // Emit event...
        event_emit("Colorblk.RedrawTile");

        // 加载关卡数据
        let level = load_level_from_json("test.json");

        // 保存初始布局和门
        self.stage = ColorBlkStage::new(level.width, level.height);
        context.state = ColorblkState::Normal as u8;

        // 使用从JSON加载的数据
        self.stage.blocks = level.blocks;
        self.stage.gates = level.gates;
        self.stage.obstacles = level.obstacles;

        // 初始化gates_state
        self.gates_state = self.stage.gates.clone();

        // 获取解决方案
        let solution = solve_main(&self.stage);

        // 设置关卡数据
        self.solution = solution;

        info!("solution....{:?}", self.solution);
        self.current_step = 0;
        self.update_render_state();
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    KeyCode::Char('s') => {
                        // Emit event...
                        event_emit("Colorblk.RedrawTile");
                    }
                    KeyCode::Char('n') => {
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
