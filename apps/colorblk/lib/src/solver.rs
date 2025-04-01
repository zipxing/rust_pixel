use crate::*;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use log::info;

/// 状态只包含 blocks，用于状态去重；history 用于记录路径，但不参与状态比较
#[derive(Clone, Debug)]
struct State {
    blocks: Vec<Block>,
    // 每一步记录 (block id, move, steps)
    // Some(direction) 表示移动或退出，
    // steps 表示连续移动步数
    history: Vec<(Vec<u8>, Option<Direction>, u8)>,
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

/// 扩展当前状态，尝试所有可能的移动和退出操作
fn expand(state: &State, stage: &ColorBlkStage) -> (bool, Vec<State>) {
    let mut next_states = Vec::new();

    // 1. 退出操作：对每个块判断是否能退出
    for block in &state.blocks {
        // 如果块被锁定且没有钥匙，则不能退出
        if block.lock == 1 && block.key == 0 {
            continue;
        }

        if let Some(_dir) = can_exit(block, &stage.gates) {
            let new_blocks = remove_block_and_update_links(&state.blocks, block.id);
            let mut new_history = state.history.clone();
            new_history.push((vec!(block.id), None, 0)); // None 表示退出

            // 发现可以移除的方块，直接返回这个状态
            return (
                true,
                vec![State {
                    blocks: new_blocks,
                    history: new_history,
                }],
            );
        }
    }

    // 2. 移动操作：分为单个块移动和组合块移动
    // 2.1 先处理单个块移动
    let mut single_block_move_states = Vec::new();

    // 替换 flat_map 为普通循环
    for (_, block) in state
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.link.is_empty())
    {
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
                        new_history.push((vec!(block.id), Some(dir), moves_count));

                        // 创建移动后的新状态
                        let new_state = State {
                            blocks: temp_state.blocks.clone(),
                            history: new_history,
                        };

                        // 移动后检查是否有方块可以退出
                        for moved_block in &new_state.blocks {
                            if moved_block.lock == 1 && moved_block.key == 0 {
                                continue;
                            }

                            if let Some(_) = can_exit(moved_block, &stage.gates) {
                                // println!("single exit....");
                                // 如果移动后有方块可以退出，直接返回这个状态
                                return (true, vec![new_state]);
                            }
                        }

                        // 将新状态添加到扩展列表
                        single_block_move_states.push(new_state);
                    } else {
                        // 无法再移动，退出循环
                        break;
                    }
                }
            }
        }
    }

    next_states.extend(single_block_move_states);

    // 2.2 处理组合块移动
    // 找出所有的组
    let mut groups = HashSet::new();
    for block in &state.blocks {
        if !block.link.is_empty() {
            groups.insert(block.link.clone());
        }
    }

    let mut group_move_states = Vec::new();

    // 替换 flat_map 为普通循环
    for group in &groups {
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
                    // println!("move group......{:?}", group);
                    moves_count += 1;

                    // 添加这个移动步骤到历史记录
                    let mut new_history = temp_state.history.clone();
                    new_history.push((group.clone().into_iter().collect(), Some(dir), moves_count));

                    // 创建移动后的新状态
                    let new_state = State {
                        blocks: temp_state.blocks.clone(),
                        history: new_history,
                    };

                    // 移动后检查是否有方块可以退出
                    for moved_block in &new_state.blocks {
                        if moved_block.lock == 1 && moved_block.key == 0 {
                            continue;
                        }

                        if let Some(_) = can_exit(moved_block, &stage.gates) {
                            // println!("move group and can_exit");
                            // 如果移动后有方块可以退出，直接返回这个状态
                            return (true, vec![new_state]);
                        }
                    }

                    // 将新状态添加到扩展列表
                    group_move_states.push(new_state);
                } else {
                    // 无法再移动，退出循环
                    break;
                }
            }
        }
    }

    next_states.extend(group_move_states);

    (false, next_states)
}

/// 广度优先搜索求解（支持并行和串行）
fn solve(initial_blocks: Vec<Block>, stage: &ColorBlkStage, use_parallel: bool) -> Option<State> {
    // 首先检查初始状态是否可以移除任何方块，如果可以，则以移除后的状态为起点
    let initial_state = State {
        blocks: initial_blocks,
        history: Vec::new(),
    };

    let visited = Arc::new(Mutex::new(HashSet::new()));
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let solution: Arc<Mutex<Option<State>>> = Arc::new(Mutex::new(None));
    // let steps = Arc::new(Mutex::new(0));

    // 标志是否需要重新开始搜索
    let restart_search = Arc::new(Mutex::new(false));
    // 保存需要作为起点的状态
    let next_start_state = Arc::new(Mutex::new(None::<State>));

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
        // 检查是否需要重新开始搜索
        {
            let mut restart = restart_search.lock().unwrap();
            if *restart {
                // 重置重启标志
                *restart = false;

                // 获取新的起点
                let mut new_start = next_start_state.lock().unwrap();
                if let Some(state) = new_start.take() {
                    // 清空队列
                    {
                        let mut q = queue.lock().unwrap();
                        q.clear();
                        q.push_back(state);
                    }

                    // println!("重新开始搜索，从新的起点状态出发");
                    continue;
                }
            }
        }

        // 从队列中获取一批状态进行处理
        let states_to_process = {
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
        // {
        //     let mut s = steps.lock().unwrap();
        //     *s += states_to_process.len();

        //     if *s % 11 == 0 {
        //         let q = queue.lock().unwrap();
        //         // println!("搜索中... 已探索 {} 个状态，队列中 {} 个状态", *s, q.len());
        //     }
        // }

        // 并行处理状态
        let next_states = if use_parallel {
            states_to_process
                .par_iter()
                .flat_map(|state| {
                    let (hasrm, expanded) = expand(state, stage);

                    // 检查是否有状态包含可移除的方块
                    if hasrm {
                        // 检查是否达到目标
                        if is_goal(&expanded[0]) {
                            return expanded;
                        }
                        // 找到可移除方块的状态，设置为新起点
                        {
                            let mut restart = restart_search.lock().unwrap();
                            *restart = true;

                            let mut new_start = next_start_state.lock().unwrap();
                            *new_start = Some(expanded[0].clone());

                            // println!(
                            //     "找到可移除方块的状态，将作为新起点{} {:?}",
                            //     hasrm,
                            //     expanded.len(),
                            // );
                        }
                    }

                    expanded
                })
                .collect::<Vec<_>>()
        } else {
            let mut all_next_states = Vec::new();

            for state in &states_to_process {
                let (_hasrm, expanded) = expand(state, stage);

                // 检查是否有状态包含可移除的方块
                let mut found_removable = false;
                for expanded_state in &expanded {
                    for block in &expanded_state.blocks {
                        if block.lock == 1 && block.key == 0 {
                            continue;
                        }

                        if let Some(_) = can_exit(block, &stage.gates) {
                            // 找到可移除方块的状态，设置为新起点
                            {
                                let mut restart = restart_search.lock().unwrap();
                                *restart = true;

                                let mut new_start = next_start_state.lock().unwrap();
                                *new_start = Some(expanded_state.clone());

                                // println!(
                                //     "找到可移除方块的状态，将作为新起点{} {:?}",
                                //     hasrm, expanded
                                // );
                            }

                            // 只添加这个状态，放弃其他状态
                            all_next_states.push(expanded_state.clone());
                            found_removable = true;
                            break;
                        }
                    }

                    if found_removable {
                        break;
                    }
                }

                if !found_removable {
                    all_next_states.extend(expanded);
                }
            }

            all_next_states
        };

        // 检查是否需要重新开始搜索
        {
            let restart = restart_search.lock().unwrap();
            if *restart {
                // 跳过处理新状态，直接进入下一轮循环
                continue;
            }
        }

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

// /// 获取颜色名称
// fn get_color_name(color: u8) -> &'static str {
//     match color {
//         1 => "红色",
//         2 => "蓝色",
//         3 => "绿色",
//         4 => "黄色",
//         _ => "未知",
//     }
// }

pub fn solve_main(stage: &ColorBlkStage) -> Option<Vec<(Vec<u8>, Option<Direction>, u8)>> {
    match solve(stage.blocks.clone(), &stage, true) {
        Some(solution) => {
            info!("solve ok!!!{:?}", solution);
            Some(solution.history)
        }
        None => {
            println!("no solution found");
            None
        }
    }
}
