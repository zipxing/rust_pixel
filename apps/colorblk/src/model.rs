use colorblk_lib::solver::solve_main;
use colorblk_lib::{Obstacle, Block, ColorBlkStage, Direction, Gate, SHAPE, SHAPE_IDX};
use log::info;
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
};

pub const COLORBLKW: u16 = 90;
pub const COLORBLKH: u16 = 57;
pub const CELLW: usize = 10;
pub const CELLH: usize = 5;

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
        info!("current_pos....{:?}", current_positions);
        info!("current_gates....{:?}", self.gates_state);

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
        
        info!("render_state....{:?}", self.render_state);
    }
}

impl Model for ColorblkModel {
    fn init(&mut self, context: &mut Context) {
        // Emit event...
        event_emit("Colorblk.RedrawTile");

        // 保存初始布局和门
        self.stage = ColorBlkStage::new(5, 9); // 默认大小
        context.state = ColorblkState::Normal as u8;

        // 添加默认方块和门
        self.stage.blocks = create_default_blocks();
        self.stage.gates = create_default_gates(&self.stage);
        self.stage.obstacles = create_default_obstacles();

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

/// 创建默认障碍
fn create_default_obstacles() -> Vec<Obstacle> {
    vec![
        Obstacle {
            x: 2,
            y: 3,
            allow_color: 3,
        },
        Obstacle {
            x: 2,
            y: 4,
            allow_color: 3,
        },
        Obstacle {
            x: 2,
            y: 5,
            allow_color: 3,
        },
    ]
}

/// 创建默认的门
fn create_default_gates(stage: &ColorBlkStage) -> Vec<Gate> {
    ////-------双色块---------
    //vec![
    //    // 上方门(1色)
    //    Gate {
    //        x: 0,
    //        y: 0,
    //        color: 1,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 2,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //    // 下方门(2色)
    //    Gate {
    //        x: 0,
    //        y: (stage.board_height - 1) as u8,
    //        color: 2,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 2,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //]
    //// 测试结束

    //-------星门&半透障碍测试---------
    vec![
        // 上方门(1色)
        Gate {
            x: 0,
            y: 0,
            color: 1,
            ice: 0,
            lock: 0,
            star: 0,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 上方门(2色)
        Gate {
            x: 2,
            y: 0,
            color: 2,
            ice: 0,
            lock: 0,
            star: 1,
            width: 1,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 上方门(3色)
        Gate {
            x: 3,
            y: 0,
            color: 3,
            ice: 0,
            lock: 0,
            star: 1,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(4色)
        Gate {
            x: 0,
            y: (stage.board_height - 1) as u8,
            color: 4,
            ice: 0,
            lock: 0,
            star: 0,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(5色)
        Gate {
            x: 2,
            y: (stage.board_height - 1) as u8,
            color: 5,
            ice: 0,
            lock: 0,
            star: 1,
            width: 1,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(6色)
        Gate {
            x: 3,
            y: (stage.board_height - 1) as u8,
            color: 6,
            ice: 0,
            lock: 0,
            star: 0,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 左方门(5色)
        Gate {
            x: 0,
            y: 2,
            color: 5,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
        // 左方门(7色)
        Gate {
            x: 0,
            y: 4,
            color: 7,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 3,
            switch: true, // 默认开启状态
        },
        // 右方门(2色)
        Gate {
            x: (stage.board_width - 1) as u8,
            y: 2,
            color: 2,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
        // 右方门(8色)
        Gate {
            x: (stage.board_width - 1) as u8,
            y: 4,
            color: 8,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 3,
            switch: true, // 默认开启状态
        },
    ]
    //-------------测试结束----------------

    ////-------组合块测试---------
    //vec![
    //    // 上方门(1色)
    //    Gate {
    //        x: 2,
    //        y: 0,
    //        color: 1,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 1,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //    // 下方门(2色)
    //    Gate {
    //        x: 2,
    //        y: (stage.board_height - 1) as u8,
    //        color: 2,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 1,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //    // 左方门(3色)
    //    Gate {
    //        x: 0,
    //        y: 0,
    //        color: 3,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 0,
    //        height: 1,
    //        switch: true, // 默认开启状态
    //    },
    //    // 左方门(4色)
    //    Gate {
    //        x: 0,
    //        y: 3,
    //        color: 4,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 0,
    //        height: 2,
    //        switch: true, // 默认开启状态
    //    },
    //    // 右方门(5色)
    //    Gate {
    //        x: (stage.board_width - 1) as u8,
    //        y: 3,
    //        color: 5,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 0,
    //        height: 2,
    //        switch: true, // 默认开启状态
    //    },
    //]
    ////-------------组合块测试结束----------------

    //-----普通关卡测试-----
    // vec![
    //     // 上方门(1色)
    //     Gate {
    //         x: 0,
    //         y: 0,
    //         color: 1,
    //         width: 3,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 上方门(2色)
    //     Gate {
    //         x: 3,
    //         y: 0,
    //         color: 2,
    //         width: 3,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 下方门(3色)
    //     Gate {
    //         x: 0,
    //         y: (stage.board_height - 1) as u8,
    //         color: 3,
    //         width: 2,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 下方门(4色)
    //     Gate {
    //         x: 4,
    //         y: (stage.board_height - 1) as u8,
    //         color: 4,
    //         width: 2,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 左方门(5色)
    //     Gate {
    //         x: 0,
    //         y: 2,
    //         color: 5,
    //         width: 0,
    //         height: 2,
    //         switch: true, // 默认开启状态
    //     },
    //     // 右方门(6色)
    //     Gate {
    //         x: (stage.board_width - 1) as u8,
    //         y: 2,
    //         color: 6,
    //         width: 0,
    //         height: 2,
    //         switch: true, // 默认开启状态
    //     },
    // ]
    //-----普通关卡测试结束-----
}

fn create_default_blocks() -> Vec<Block> {
    // vec![
    //     Block {
    //         id: 1,
    //         shape: SHAPE_IDX[0] as u8, 
    //         color: 1,                  
    //         color2: 2,
    //         star: 0,
    //         dir: 0,
    //         ropes: vec![],
    //         scissor: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 0,
    //         y: 3,
    //         link: Vec::new(),
    //     },
    // ]
    //-----星门半透明测试-----
    vec![
        Block {
            id: 1,
            shape: SHAPE_IDX[1] as u8, 
            color: 4,                  
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 0,
            link: Vec::new(),
        },
        Block {
            id: 2,
            shape: SHAPE_IDX[4] as u8, 
            color: 8,                  
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 2,
            y: 0,
            link: Vec::new(),
        },
        Block {
            id: 3,
            shape: SHAPE_IDX[1] as u8, 
            color: 7,                  
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 0,
            link: Vec::new(),
        },
        Block {
            id: 4,
            shape: SHAPE_IDX[2] as u8, // 纵向两个方块
            color: 2,                  // 绿色，对应右方门
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 1,
            link: Vec::new(),
        },
        Block {
            id: 5,
            shape: SHAPE_IDX[2] as u8,
            color: 6,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 1,
            link: Vec::new(),
        },
        Block {
            id: 6,
            shape: SHAPE_IDX[2] as u8,
            color: 5,
            color2: 0,
            star: 1,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 7,
            shape: SHAPE_IDX[2] as u8,
            color: 2,
            color2: 0,
            star: 1,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 4,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 8,
            shape: SHAPE_IDX[1] as u8,
            color: 6,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 4,
            link: Vec::new(),
        },
        Block {
            id: 9,
            shape: SHAPE_IDX[1] as u8,
            color: 3,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 4,
            link: Vec::new(),
        },
        Block {
            id: 10,
            shape: SHAPE_IDX[2] as u8,
            color: 8,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 5,
            link: Vec::new(),
        },
        Block {
            id: 11,
            shape: SHAPE_IDX[2] as u8,
            color: 1,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 5,
            link: Vec::new(),
        },

        Block {
            id: 12,
            shape: SHAPE_IDX[2] as u8,
            color: 4,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 5,
            link: Vec::new(),
        },

        Block {
            id: 13,
            shape: SHAPE_IDX[2] as u8,
            color: 7,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 4,
            y: 5,
            link: Vec::new(),
        },

        Block {
            id: 14,
            shape: SHAPE_IDX[9] as u8,
            color: 3,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 7,
            link: Vec::new(),
        },

        Block {
            id: 15,
            shape: SHAPE_IDX[4] as u8,
            color: 3,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 2,
            y: 6,
            link: Vec::new(),
        },
        Block {
            id: 16,
            shape: SHAPE_IDX[9] as u8,
            color: 1,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 7,
            link: Vec::new(),
        },
    ]
    //-----测试结束-----

    ////-----组合块测试-----
    //vec![
    //    Block {
    //        id: 1,
    //        shape: SHAPE_IDX[0] as u8, // 单个方块
    //        color: 2,                  // 红色，对应上方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 1,
    //        y: 0,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 2,
    //        shape: SHAPE_IDX[9] as u8, // 横向两个方块
    //        color: 4,                  // 蓝色，对应下方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 2,
    //        y: 0,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 3,
    //        shape: SHAPE_IDX[2] as u8, // 纵向两个方块
    //        color: 3,                  // 绿色，对应右方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 0,
    //        y: 1,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 4,
    //        shape: SHAPE_IDX[0] as u8, // 纵向两个方块
    //        color: 2,                  // 绿色，对应右方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 3,
    //        y: 2,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 5,
    //        shape: SHAPE_IDX[0] as u8,
    //        color: 3,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 0,
    //        y: 3,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 6,
    //        shape: SHAPE_IDX[1] as u8,
    //        color: 5,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 1,
    //        y: 3,
    //        // link: vec![1, 2],
    //        link: vec![6, 8],
    //        // link: Vec::new(),
    //    },
    //    Block {
    //        id: 7,
    //        shape: SHAPE_IDX[2] as u8,
    //        color: 4,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 3,
    //        y: 3,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 8,
    //        shape: SHAPE_IDX[0] as u8,
    //        color: 1,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 2,
    //        y: 4,
    //        // link: Vec::new(),
    //        link: vec![6, 8],
    //    },
    //]
    ////-----组合块测试结束-----

    // vec![
    //     Block {
    //         id: 1,
    //         shape: SHAPE_IDX[3] as u8, // 单个方块
    //         color: 2,                  // 红色，对应上方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 0,
    //         y: 1,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 2,
    //         shape: SHAPE_IDX[3] as u8, // 横向两个方块
    //         color: 1,                  // 蓝色，对应下方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 3,
    //         y: 1,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 3,
    //         shape: SHAPE_IDX[2] as u8, // 纵向两个方块
    //         color: 4,                  // 绿色，对应右方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 0,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 4,
    //         shape: SHAPE_IDX[3] as u8, // 纵向两个方块
    //         color: 6,                  // 绿色，对应右方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 2,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 5,
    //         shape: SHAPE_IDX[2] as u8,
    //         color: 3,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 5,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 6,
    //         shape: SHAPE_IDX[3] as u8,
    //         color: 5,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 1,
    //         y: 3,
    //         // link: vec![6, 8],
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 7,
    //         shape: SHAPE_IDX[5] as u8,
    //         color: 3,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 1,
    //         y: 4,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 8,
    //         shape: SHAPE_IDX[7] as u8,
    //         color: 4,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 3,
    //         y: 4,
    //         link: Vec::new(),
    //         // link: vec![6, 8],
    //     },
    // ]
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
