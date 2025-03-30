use crate::shape::SHAPE_IDX;
use crate::*;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 状态只包含 blocks，用于状态去重；history 用于记录路径，但不参与状态比较
#[derive(Clone, Debug)]
struct State {
    blocks: Vec<Block>,
    // 每一步记录 (block id, move, steps)，Some(direction) 表示移动或退出，steps 表示连续移动步数
    history: Vec<(u8, Option<Direction>, u8)>,
}

// 仅比较 blocks 字段
impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.blocks == other.blocks
    }
}
impl Eq for State {}
impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.blocks.hash(state);
    }
}

/// 检查状态是否达到目标（所有块都已移出）
fn is_goal(state: &State) -> bool {
    state.blocks.is_empty()
}

/// 扩展当前状态，尝试所有可能的移动和退出操作（并行版）
fn expand(state: &State, stage: &ColorBlkStage) -> Vec<State> {
    let mut next_states = Vec::new();

    // 1. 退出操作：对每个块判断是否能退出
    let exit_states: Vec<State> = state
        .blocks
        .par_iter()
        .enumerate()
        .filter_map(|(_, block)| {
            // 如果块被锁定且没有钥匙，则不能退出
            if block.lock == 1 && block.key == 0 {
                return None;
            }

            if let Some(_dir) = can_exit(block, &stage.gates) {
                let new_blocks = remove_block_and_update_links(&state.blocks, block.id);
                let mut new_history = state.history.clone();
                new_history.push((block.id, None, 0)); // None 表示退出

                Some(State {
                    blocks: new_blocks,
                    history: new_history,
                })
            } else {
                None
            }
        })
        .collect();

    next_states.extend(exit_states);

    // 2. 移动操作：分为单个块移动和组合块移动
    // 2.1 先处理单个块移动
    let single_block_move_states: Vec<State> = state
        .blocks
        .par_iter()
        .enumerate()
        .filter(|(_, block)| block.link.is_empty()) // 只处理非组块
        .flat_map(|(_, block)| {
            let mut block_states = Vec::new();

            for &dir in &[
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                // 从当前状态克隆一个临时状态进行连续移动
                let mut temp_state = state.clone();
                // 获取当前块在临时状态中的索引
                if let Some(temp_idx) = temp_state.blocks.iter().position(|b| b.id == block.id) {
                    let mut moves_count = 0;

                    // 循环尝试移动，直到无法继续移动
                    loop {
                        let mut sim_board = layout_to_board(&temp_state.blocks, stage);

                        // 尝试将块移动一步
                        if move_entire_block(
                            &mut sim_board,
                            &mut temp_state.blocks[temp_idx],
                            dir,
                            stage,
                        ) {
                            moves_count += 1;

                            // 添加这个移动步骤到历史记录
                            let mut new_history = temp_state.history.clone();
                            new_history.push((block.id, Some(dir), moves_count));

                            // 创建移动后的新状态
                            let new_state = State {
                                blocks: temp_state.blocks.clone(),
                                history: new_history,
                            };

                            // 将新状态添加到扩展列表
                            block_states.push(new_state);
                        } else {
                            // 无法再移动，退出循环
                            break;
                        }
                    }
                }
            }

            block_states
        })
        .collect();

    next_states.extend(single_block_move_states);

    // 2.2 处理组合块移动
    // 找出所有的组
    let mut groups = HashSet::new();
    for block in &state.blocks {
        if !block.link.is_empty() {
            groups.insert(block.link.clone());
        }
    }

    let group_move_states: Vec<State> = groups
        .par_iter()
        .flat_map(|group| {
            let mut group_states = Vec::new();

            for &dir in &[
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                // 从当前状态克隆一个临时状态进行连续移动
                let mut temp_state = state.clone();
                let mut moves_count = 0;

                // 循环尝试移动，直到无法继续移动
                loop {
                    let mut sim_board = layout_to_board(&temp_state.blocks, stage);

                    // 尝试移动整个组
                    if move_group(&mut sim_board, &mut temp_state.blocks, group, dir, stage) {
                        moves_count += 1;

                        // 添加这个移动步骤到历史记录
                        let mut new_history = temp_state.history.clone();
                        new_history.push((group[0], Some(dir), moves_count));

                        // 创建移动后的新状态
                        let new_state = State {
                            blocks: temp_state.blocks.clone(),
                            history: new_history,
                        };

                        // 将新状态添加到扩展列表
                        group_states.push(new_state);
                    } else {
                        // 无法再移动，退出循环
                        break;
                    }
                }
            }

            group_states
        })
        .collect();

    next_states.extend(group_move_states);

    next_states
}

#[derive(Clone)]
struct SharedData {
    stage: ColorBlkStage,
    visited: Arc<Mutex<HashSet<State>>>,
    solution: Arc<Mutex<Option<Vec<Direction>>>>,
    max_depth: Arc<Mutex<usize>>,
    total_states: Arc<Mutex<usize>>,
    start_time: Arc<Mutex<Instant>>,
    last_report_time: Arc<Mutex<Instant>>,
    last_states: Arc<Mutex<usize>>,
    last_depth: Arc<Mutex<usize>>,
}

/// 广度优先搜索求解（支持并行和串行）
fn solve(initial_blocks: Vec<Block>, stage: &ColorBlkStage, use_parallel: bool) -> Option<State> {
    // 首先检查初始状态是否可以移除任何方块，如果可以，则以移除后的状态为起点
    let mut initial_state = State {
        blocks: initial_blocks,
        history: Vec::new(),
    };

    // 尝试移除初始状态中可以移除的方块
    let mut has_removable_blocks = true;
    while has_removable_blocks {
        has_removable_blocks = false;
        
        // 检查是否有方块可以移除
        let mut block_to_remove = None;
        for block in &initial_state.blocks {
            // 如果块被锁定且没有钥匙，则不能退出
            if block.lock == 1 && block.key == 0 {
                continue;
            }

            if let Some(_dir) = can_exit(block, &stage.gates) {
                block_to_remove = Some(block.id);
                break;  // 一次只移除一个方块，然后重新检查
            }
        }
        
        // 如果找到了可以移除的方块，则移除它
        if let Some(block_id) = block_to_remove {
            // 在移除前获取方块颜色
            let block_color = initial_state.blocks.iter()
                .find(|b| b.id == block_id)
                .map(|b| b.color)
                .unwrap_or(0);
                
            let new_blocks = remove_block_and_update_links(&initial_state.blocks, block_id);
            initial_state.history.push((block_id, None, 0)); // None 表示退出
            initial_state.blocks = new_blocks;
            has_removable_blocks = true;
            
            println!("预处理：移除方块 {}({})", block_id, get_color_name(block_color));
        }
    }

    let visited = Arc::new(Mutex::new(HashSet::new()));
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let solution: Arc<Mutex<Option<State>>> = Arc::new(Mutex::new(None));
    let steps = Arc::new(Mutex::new(0));

    // 添加初始状态
    {
        let mut v = visited.lock().unwrap();
        v.insert(initial_state.clone());

        let mut q = queue.lock().unwrap();
        q.push_back(initial_state);
    }

    // 设置并行度，这个值可以根据系统的CPU核心数调整
    let num_threads = if use_parallel {
        rayon::current_num_threads()
    } else {
        1
    };
    let chunk_size = 50; // 每次从队列取出的状态数，可以根据需要调整

    // 确保线程池已初始化
    if use_parallel {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .unwrap_or(());
    }

    loop {
        // 从队列中获取一批状态进行处理
        let mut states_to_process = {
            let mut q = queue.lock().unwrap();
            let mut batch = Vec::new();

            for _ in 0..chunk_size {
                if let Some(state) = q.pop_front() {
                    batch.push(state);
                } else {
                    break;
                }
            }

            batch
        };

        // 如果队列为空，且没有正在处理的状态，则结束搜索
        if states_to_process.is_empty() {
            break;
        }

        // 更新处理的步数
        {
            let mut s = steps.lock().unwrap();
            *s += states_to_process.len();

            if *s % 11 == 0 {
                let q = queue.lock().unwrap();
                println!("搜索中... 已探索 {} 个状态，队列中 {} 个状态", *s, q.len());
            }
        }

        // 优先处理能够移除方块的状态
        // 对于每个状态，检查是否可以移除方块
        for state_idx in 0..states_to_process.len() {
            let state = &mut states_to_process[state_idx];
            let mut has_removable_block = false;
            
            for (i, block) in state.blocks.iter().enumerate() {
                // 如果块被锁定且没有钥匙，则不能退出
                if block.lock == 1 && block.key == 0 {
                    continue;
                }

                if let Some(_dir) = can_exit(block, &stage.gates) {
                    let new_blocks = remove_block_and_update_links(&state.blocks, block.id);
                    let mut new_history = state.history.clone();
                    new_history.push((block.id, None, 0)); // None 表示退出
                    
                    *state = State {
                        blocks: new_blocks,
                        history: new_history,
                    };
                    
                    has_removable_block = true;
                    break;  // 一次只移除一个方块，然后处理下一个状态
                }
            }
            
            // 如果有方块被移除，检查该状态是否已访问过
            if has_removable_block {
                let mut v = visited.lock().unwrap();
                if v.contains(state) {
                    // 如果已访问过，标记为需要移除
                    state.blocks.clear(); // 清空表示需要移除
                } else {
                    v.insert(state.clone());
                    
                    // 检查是否达到目标
                    if is_goal(state) {
                        let mut s = solution.lock().unwrap();
                        *s = Some(state.clone());
                        return Some(state.clone());
                    }
                }
            }
        }
        
        // 移除已经处理过的状态
        states_to_process.retain(|state| !state.blocks.is_empty());

        // 并行处理状态
        let next_states = if use_parallel {
            states_to_process
                .par_iter()
                .flat_map(|state| expand(state, stage))
                .collect::<Vec<_>>()
        } else {
            states_to_process
                .iter()
                .flat_map(|state| expand(state, stage))
                .collect()
        };

        // 处理新状态
        for state in next_states {
            // 检查是否已访问过该状态
            let mut v = visited.lock().unwrap();
            if v.contains(&state) {
                continue;
            }
            v.insert(state.clone());

            // 检查是否达到目标
            if is_goal(&state) {
                let mut s = solution.lock().unwrap();
                *s = Some(state.clone());
                return Some(state);
            }

            // 将新状态加入队列
            let mut q = queue.lock().unwrap();
            q.push_back(state);
        }
    }

    // 返回最终解
    let s = solution.lock().unwrap();
    s.clone()
}

/// 获取颜色名称
fn get_color_name(color: u8) -> &'static str {
    match color {
        1 => "红色",
        2 => "蓝色",
        3 => "绿色",
        4 => "黄色",
        _ => "未知",
    }
}

// 获取颜色ANSI代码的辅助函数
fn get_color_code(color: u8) -> &'static str {
    match color {
        1 => "\x1b[31m", // 红色
        2 => "\x1b[34m", // 蓝色
        3 => "\x1b[32m", // 绿色
        4 => "\x1b[33m", // 黄色
        5 => "\x1b[33m", // 为颜色5使用黄色
        _ => "\x1b[0m",  // 默认
    }
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

pub fn solve_main() -> (
    Vec<Block>,
    Vec<Gate>,
    Option<Vec<(u8, Option<Direction>, u8)>>,
) {
    // 创建一个新的关卡
    let mut stage = ColorBlkStage::new(6, 6);

    // 添加默认方块和门
    stage.blocks = create_default_blocks();
    stage.gates = create_default_gates(&stage);

    match solve(stage.blocks.clone(), &stage, true) {
        // match solve(stage.blocks.clone(), &stage, false) {
        Some(solution) => {
            println!("solve ok!!!{:?}", solution);
            (stage.blocks, stage.gates, Some(solution.history))
        }
        None => {
            println!("no solution found");
            (stage.blocks, stage.gates, None)
        }
    }
}
