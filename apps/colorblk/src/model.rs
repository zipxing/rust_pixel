use colorblk_lib::solver::solve_main;
use colorblk_lib::{Block, ColorBlkStage, Direction, Gate, SHAPE, SHAPE_IDX};
use log::info;
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
};

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
    pub count: i16,
    // 存储初始关卡状态
    pub stage: ColorBlkStage,
    // 存储计算结果的字段
    pub solution: Option<Vec<(u8, Option<Direction>, u8)>>, // 存储移动步骤
    pub current_step: usize,                                // 当前执行到哪一步
    // 存储每个格子的渲染状态 (border_type, color, symbol)
    pub render_state: Vec<(u8, i8, &'static str)>,
}

impl ColorblkModel {
    pub fn new() -> Self {
        // let stage = ColorBlkStage::new(5, 6); // 默认使用5x6的棋盘
        let stage = ColorBlkStage::new(6, 6); // 默认使用5x6的棋盘
        Self {
            count: 0,
            stage,
            solution: None,
            current_step: 0,
            // render_state: vec![(0, -1, ""); (5 * 6)], // 默认大小
            render_state: vec![(0, -1, ""); 6 * 6], // 默认大小
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
        self.render_state = vec![(0, -1, ""); self.stage.board_width * self.stage.board_height];

        // 创建一个数组来跟踪每个方块的当前位置和是否被移除
        let mut current_positions: Vec<(i16, i16, bool)> = self
            .stage
            .blocks
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
        // Emit event...
        event_emit("Colorblk.RedrawTile");

        // 保存初始布局和门
        self.stage = ColorBlkStage::new(6, 6); // 默认大小
        context.state = ColorblkState::Normal as u8;

        // 添加默认方块和门
        self.stage.blocks = create_default_blocks();
        self.stage.gates = create_default_gates(&self.stage);

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

/// 创建默认的门
fn create_default_gates(stage: &ColorBlkStage) -> Vec<Gate> {
    // vec![
    //     // 上方门(红色)
    //     Gate {
    //         x: 0,
    //         y: 0,
    //         color: 1,
    //         width: 2,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 上方门(红色)
    //     Gate {
    //         x: 3,
    //         y: 0,
    //         color: 2,
    //         width: 2,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 下方门(蓝色)
    //     Gate {
    //         x: 1,
    //         y: (stage.board_height - 1) as u8,
    //         color: 3,
    //         width: 3,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 右方门(绿色)
    //     Gate {
    //         x: (stage.board_width - 1) as u8,
    //         y: 4,
    //         color: 4,
    //         width: 0,
    //         height: 2,
    //         switch: true, // 默认开启状态
    //     },
    //     // 左方门(黄色)
    //     Gate {
    //         x: 0,
    //         y: 4,
    //         color: 5,
    //         width: 0,
    //         height: 2,
    //         switch: true, // 默认开启状态
    //     },
    // ]
    vec![
        // 上方门(1色)
        Gate {
            x: 0,
            y: 0,
            color: 1,
            width: 3,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 上方门(2色)
        Gate {
            x: 3,
            y: 0,
            color: 2,
            width: 3,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(3色)
        Gate {
            x: 0,
            y: (stage.board_height - 1) as u8,
            color: 3,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(4色)
        Gate {
            x: 4,
            y: (stage.board_height - 1) as u8,
            color: 4,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 左方门(5色)
        Gate {
            x: 0, 
            y: 2,
            color: 5,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
        // 右方门(6色)
        Gate {
            x: (stage.board_width - 1) as u8,
            y: 2,
            color: 6,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
     ]
}

fn create_default_blocks() -> Vec<Block> {
    // vec![
    //     Block {
    //         id: 1,
    //         shape: SHAPE_IDX[9] as u8, // 单个方块
    //         color: 1,                  // 红色，对应上方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 3,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 2,
    //         shape: SHAPE_IDX[9] as u8, // 横向两个方块
    //         color: 2,                  // 蓝色，对应下方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 0,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 3,
    //         shape: SHAPE_IDX[4] as u8, // 纵向两个方块
    //         color: 3,                  // 绿色，对应右方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 2,
    //         y: 0,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 4,
    //         shape: SHAPE_IDX[3] as u8, // 纵向两个方块
    //         color: 4,                  // 绿色，对应右方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 1,
    //         y: 5,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 5,
    //         shape: SHAPE_IDX[3] as u8, // 纵向两个方块
    //         color: 5,                  // 绿色，对应右方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 1,
    //         y: 4,
    //         link: Vec::new(),
    //     },
    // ]
    vec![
        Block {
            id: 1,
            shape: SHAPE_IDX[3] as u8, // 单个方块
            color: 2,                  // 红色，对应上方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 1,
            link: Vec::new(),
        },
        Block {
            id: 2,
            shape: SHAPE_IDX[3] as u8, // 横向两个方块
            color: 1,                  // 蓝色，对应下方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 1,
            link: Vec::new(),
        },
        Block {
            id: 3,
            shape: SHAPE_IDX[2] as u8, // 纵向两个方块
            color: 4,                  // 绿色，对应右方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 4,
            shape: SHAPE_IDX[3] as u8, // 纵向两个方块
            color: 6,                  // 绿色，对应右方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 2,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 5,
            shape: SHAPE_IDX[2] as u8, 
            color: 3,                  
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 5,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 6,
            shape: SHAPE_IDX[3] as u8, 
            color: 5,                  
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 3,
            // link: vec![6, 8],
            link: Vec::new(),
        },
        Block {
            id: 7,
            shape: SHAPE_IDX[5] as u8, 
            color: 3,                  
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 4,
            link: Vec::new(),
        },
        Block {
            id: 8,
            shape: SHAPE_IDX[7] as u8, 
            color: 4,                  
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 4,
            link: Vec::new(),
            // link: vec![6, 8],
        },
    ]
}


