use colorblk::shape::{SHAPE, SHAPE_IDX};
use colorblk::*;
use rand::Rng;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::io;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// 状态只包含 blocks，用于状态去重；history 用于记录路径，但不参与状态比较
#[derive(Clone, Debug)]
struct State {
    blocks: Vec<Block>,
    history: Vec<(u8, Option<Direction>, u8)>, // 每一步记录 (block id, move, steps)，Some(direction) 表示移动或退出，steps 表示连续移动步数
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
fn expand(state: &State, gates: &[Gate]) -> Vec<State> {
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

            if let Some(dir) = can_exit(block, gates) {
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
                        let mut sim_board = layout_to_board(&temp_state.blocks);

                        // 尝试将块移动一步
                        if move_entire_block(&mut sim_board, &mut temp_state.blocks[temp_idx], dir)
                        {
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
                    let mut sim_board = layout_to_board(&temp_state.blocks);

                    // 尝试移动整个组
                    if move_group(&mut sim_board, &mut temp_state.blocks, group, dir) {
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

/// 广度优先搜索求解（并行版）
fn solve(initial_blocks: Vec<Block>, gates: &[Gate]) -> Option<State> {
    let initial_state = State {
        blocks: initial_blocks,
        history: Vec::new(),
    };

    let visited = Arc::new(Mutex::new(HashSet::new()));
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let solution = Arc::new(Mutex::new(None));
    let steps = Arc::new(Mutex::new(0));

    // 添加初始状态
    {
        let mut v = visited.lock().unwrap();
        v.insert(initial_state.clone());

        let mut q = queue.lock().unwrap();
        q.push_back(initial_state);
    }

    // 设置并行度，这个值可以根据系统的CPU核心数调整
    let num_threads = rayon::current_num_threads();
    let chunk_size = 50; // 每次从队列取出的状态数，可以根据需要调整

    // 确保线程池已初始化
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap_or(());

    loop {
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
        {
            let mut s = steps.lock().unwrap();
            *s += states_to_process.len();

            if *s % 11 == 0 {
                let q = queue.lock().unwrap();
                println!("搜索中... 已探索 {} 个状态，队列中 {} 个状态", *s, q.len());
            }
        }

        // 并行处理每个状态
        let all_next_states: Vec<Vec<State>> = states_to_process
            .par_iter()
            .map(|state| {
                // 检查是否已找到解
                {
                    let sol = solution.lock().unwrap();
                    if sol.is_some() {
                        return Vec::new();
                    }
                }

                // 检查是否达到目标
                if is_goal(state) {
                    let mut sol = solution.lock().unwrap();
                    *sol = Some(state.clone());
                    return Vec::new();
                }

                // 扩展当前状态
                expand(state, gates)
            })
            .collect();

        // 检查是否已找到解
        {
            let sol = solution.lock().unwrap();
            if sol.is_some() {
                break;
            }
        }

        // 处理展开的新状态
        for next_states in all_next_states {
            for next_state in next_states {
                let mut v = visited.lock().unwrap();
                if !v.contains(&next_state) {
                    v.insert(next_state.clone());

                    let mut q = queue.lock().unwrap();
                    q.push_back(next_state);
                }
            }
        }
    }

    let final_steps = *steps.lock().unwrap();
    println!("共探索 {} 个状态", final_steps);

    let x = solution.lock().unwrap().clone();
    x
}

/// 原始打印棋盘状态函数
/*
fn print_board(board: &Board) {
    println!("┌{}┐", "─".repeat(BOARD_WIDTH * 2));
    for row in board.iter() {
        print!("│");
        for &cell in row.iter() {
            if cell == 0 {
                print!("  ");
            } else if cell == OBSTACLE {
                print!("██");
            } else {
                // 提取块的 id (最后两位数字)
                let id = cell % 100;
                print!("{:2}", id);
            }
        }
        println!("│");
    }
    println!("└{}┘", "─".repeat(BOARD_WIDTH * 2));
}
*/

/// 改进的打印棋盘状态函数，使用ANSI彩色打印，修复门的位置显示
/*
fn print_board_with_gates(board: &Board, gates: &[Gate]) {
    // 定义ANSI颜色代码
    const RESET: &str = "\x1b[0m";
    const RED: &str = "\x1b[31m";
    const BLUE: &str = "\x1b[34m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const BOLD: &str = "\x1b[1m";

    // 颜色映射函数
    fn color_code(color: u8) -> &'static str {
        match color {
            1 => RED,
            2 => BLUE,
            3 => GREEN,
            4 => YELLOW,
            5 => YELLOW, // 颜色5也映射为黄色
            _ => RESET,
        }
    }

    // 清空门数组并重新计算
    let mut top_gates = vec![None; BOARD_WIDTH * 2];
    let mut bottom_gates = vec![None; BOARD_WIDTH * 2];
    let mut left_gates = vec![None; BOARD_HEIGHT];
    let mut right_gates = vec![None; BOARD_HEIGHT];

    // 标记门的位置
    for gate in gates {
        // 上方门
        if gate.y == 0 && gate.height == 0 && gate.width > 0 {
            for i in 0..gate.width {
                let x_pos = (gate.x + i) as usize;
                if x_pos < BOARD_WIDTH {
                    // 每个格子是2个字符宽度
                    let display_pos = x_pos * 2;
                    top_gates[display_pos] = Some((gate.color, gate.switch));
                    top_gates[display_pos + 1] = Some((gate.color, gate.switch));
                }
            }
        }
        // 下方门
        else if gate.y as usize == BOARD_HEIGHT - 1 && gate.height == 0 && gate.width > 0 {
            for i in 0..gate.width {
                let x_pos = (gate.x + i) as usize;
                if x_pos < BOARD_WIDTH {
                    // 每个格子是2个字符宽度
                    let display_pos = x_pos * 2;
                    bottom_gates[display_pos] = Some((gate.color, gate.switch));
                    bottom_gates[display_pos + 1] = Some((gate.color, gate.switch));
                }
            }
        }
        // 左侧门
        else if gate.x == 0 && gate.width == 0 && gate.height > 0 {
            for i in 0..gate.height {
                let y_pos = (gate.y + i) as usize;
                if y_pos < BOARD_HEIGHT {
                    left_gates[y_pos] = Some((gate.color, gate.switch));
                }
            }
        }
        // 右侧门
        else if gate.x as usize == BOARD_WIDTH - 1 && gate.width == 0 && gate.height > 0 {
            for i in 0..gate.height {
                let y_pos = (gate.y + i) as usize;
                if y_pos < BOARD_HEIGHT {
                    right_gates[y_pos] = Some((gate.color, gate.switch));
                }
            }
        }
    }

    // 打印顶部门和顶部边框
    print!("  ");
    for x in 0..BOARD_WIDTH * 2 {
        if let Some((color, switch)) = top_gates[x] {
            // 根据开关状态显示不同的符号
            let symbol = if switch { "▼" } else { "v" };
            print!("{}{}{}{}", color_code(color), BOLD, symbol, RESET);
        } else {
            print!(" ");
        }
    }
    println!();

    println!("  ┌{}┐", "─".repeat(BOARD_WIDTH * 2));

    // 打印棋盘内容和左右门
    for (y, row) in board.iter().enumerate() {
        // 打印左侧门
        if let Some((color, switch)) = left_gates[y] {
            // 根据开关状态显示不同的符号
            let symbol = if switch { "►" } else { ">" };
            print!("{}{}{}{} ", color_code(color), BOLD, symbol, RESET);
        } else {
            print!("  ");
        }

        // 打印棋盘行内容
        print!("│");
        for &cell in row.iter() {
            if cell == 0 {
                print!("[]");
            } else if cell == OBSTACLE {
                print!("{}{}██{}", BOLD, "\x1b[90m", RESET); // 灰色障碍物
            } else {
                // 提取块的 id (最后两位数字)
                let id = cell % 100;
                let color = (cell / 100) % 100; // 颜色码

                // 显示带颜色的块ID
                print!("{}{}{:2}{}", color_code(color as u8), BOLD, id, RESET);
            }
        }
        print!("│");

        // 打印右侧门
        if let Some((color, switch)) = right_gates[y] {
            // 根据开关状态显示不同的符号
            let symbol = if switch { "◄" } else { "<" };
            print!(" {}{}{}{}", color_code(color), BOLD, symbol, RESET);
        } else {
            print!("  ");
        }

        println!();
    }

    // 打印底部边框和底部门
    println!("  └{}┘", "─".repeat(BOARD_WIDTH * 2));

    print!("  ");
    for x in 0..BOARD_WIDTH * 2 {
        if let Some((color, switch)) = bottom_gates[x] {
            // 根据开关状态显示不同的符号
            let symbol = if switch { "▲" } else { "^" };
            print!("{}{}{}{}", color_code(color), BOLD, symbol, RESET);
        } else {
            print!(" ");
        }
    }
    println!();
}
*/

/// 使用彩色打印的扩展函数，用于在解题过程中可视化当前布局
/*
fn print_colored_layout(blocks: &[Block], gates: &[Gate]) {
    let board = layout_to_board(blocks);
    print_board_with_gates(&board, gates);
}
*/

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

/// 彩色打印门的信息
/*
fn print_gates_info(gates: &[Gate]) {
    const RESET: &str = "\x1b[0m";
    const RED: &str = "\x1b[31m";
    const BLUE: &str = "\x1b[34m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const BOLD: &str = "\x1b[1m";

    println!("门的位置信息:");
    for (i, gate) in gates.iter().enumerate() {
        let dir = if gate.height == 0 {
            if gate.y == 0 {
                "上"
            } else {
                "下"
            }
        } else {
            if gate.x == 0 {
                "左"
            } else {
                "右"
            }
        };

        let color_ansi = match gate.color {
            1 => RED,
            2 => BLUE,
            3 => GREEN,
            4 => YELLOW,
            _ => RESET,
        };

        let color_name = get_color_name(gate.color);
        let status = if gate.switch { "开启" } else { "关闭" };

        println!(
            "门 {}: {} 方，{}{}{}{}, 状态: {}, 位置: x={}, y={}, 宽/高: {}/{}",
            i + 1,
            dir,
            color_ansi,
            BOLD,
            color_name,
            RESET,
            status,
            gate.x,
            gate.y,
            gate.width,
            gate.height
        );
    }
}
*/

/// 可视化解决方案函数（原始版）
/*
fn visualize_solution(solution: &State, gates: &[Gate]) {
    if solution.history.is_empty() {
        println!("无需移动，所有块已移出！");
        return;
    }

    // 重建解决方案的步骤
    let mut blocks = Vec::new();

    // 从最终状态回溯
    for &(block_id, move_opt, _) in solution.history.iter().rev() {
        if move_opt.is_none() {
            // 这个块曾经被移出，现在把它添加回来
            let block_idx = solution.blocks.iter().position(|b| b.id == block_id);
            if let Some(idx) = block_idx {
                blocks.push(solution.blocks[idx].clone());
            }
        }
    }

    let mut board = layout_to_board(&blocks);

    println!("\n解决方案步骤:");
    println!("初始状态:");
    print_board(&board);

    // 打印门的位置
    println!("门的位置:");
    for (i, gate) in gates.iter().enumerate() {
        let dir = if gate.height == 0 {
            if gate.y == 0 {
                "上"
            } else {
                "下"
            }
        } else {
            if gate.x == 0 {
                "左"
            } else {
                "右"
            }
        };
        println!(
            "门 {}: {} 方，颜色 {}, 位置: x={}, y={}, 宽/高: {}/{}",
            i + 1,
            dir,
            gate.color,
            gate.x,
            gate.y,
            gate.width,
            gate.height
        );
    }

    // 正序执行移动历史
    for (step, &(block_id, move_dir, _)) in solution.history.iter().enumerate() {
        println!("\n步骤 {}: 块 {} ", step + 1, block_id);

        match move_dir {
            Some(dir) => {
                let dir_name = match dir {
                    Direction::Up => "上",
                    Direction::Down => "下",
                    Direction::Left => "左",
                    Direction::Right => "右",
                };
                println!("移动方向: {}", dir_name);

                // 找到对应的块并移动
                if let Some(idx) = blocks.iter().position(|b| b.id == block_id) {
                    move_entire_block(&mut board, &mut blocks[idx], dir);
                }
            }
            None => {
                println!("退出棋盘");
                // 找到对应的块并移除
                blocks = remove_block_and_update_links(&blocks, block_id);
                board = layout_to_board(&blocks);
            }
        }

        print_board(&board);
    }

    println!("\n解决方案完成！总步数: {}", solution.history.len());
}
*/

/// 使用初始布局的可视化解决方案函数，并在棋盘上显示门，使用彩色
/*
fn visualize_solution_with_initial_colored(
    solution: &State,
    gates: &[Gate],
    initial_blocks: &[Block],
) {
    const RESET: &str = "\x1b[0m";
    const BOLD: &str = "\x1b[1m";

    if solution.history.is_empty() {
        println!("无需移动，所有块已移出！");
        return;
    }

    // 从初始布局开始
    let mut blocks = initial_blocks.to_vec();
    let mut board = layout_to_board(&blocks);

    println!("\n{}解决方案步骤:{}", BOLD, RESET);
    println!("{}初始状态:{}", BOLD, RESET);
    print_board_with_gates(&board, gates);

    // 打印门的位置信息（彩色）
    print_gates_info(gates);

    // 打印所有方块的位置信息
    println!("\n{}方块位置信息:{}", BOLD, RESET);
    for block in &blocks {
        let shape_data = &SHAPE[block.shape as usize];
        let rect = &shape_data.rect;
        println!(
            "  {}块 {} - 颜色: {}, 位置: ({}, {}), 大小: {}x{}{}",
            get_color_code(block.color),
            block.id,
            get_color_name(block.color),
            block.x,
            block.y,
            rect.width,
            rect.height,
            RESET
        );
    }

    // 按顺序执行移动历史
    for (step, &(block_id, move_dir, steps)) in solution.history.iter().enumerate() {
        println!("\n{}步骤 {}: 块 {} {}", BOLD, step + 1, block_id, RESET);

        // 查找并记录块的颜色
        let block_color = blocks
            .iter()
            .find(|b| b.id == block_id)
            .map(|b| b.color)
            .unwrap_or(0);

        let color_ansi = get_color_code(block_color);

        match move_dir {
            Some(dir) => {
                let dir_name = match dir {
                    Direction::Up => "上",
                    Direction::Down => "下",
                    Direction::Left => "左",
                    Direction::Right => "右",
                };
                println!(
                    "{}{}移动方向: {}, 移动步数: {}{}",
                    color_ansi, BOLD, dir_name, steps, RESET
                );

                // 找到对应的块
                if let Some(idx) = blocks.iter().position(|b| b.id == block_id) {
                    // 记录移动前的位置
                    let old_x = blocks[idx].x;
                    let old_y = blocks[idx].y;

                    // 执行指定的steps次移动
                    for _ in 0..steps {
                        move_entire_block(&mut board, &mut blocks[idx], dir);
                    }

                    // 打印移动前后的位置变化
                    println!(
                        "  {}{}方块从 ({}, {}) 移动到 ({}, {}){}",
                        color_ansi, BOLD, old_x, old_y, blocks[idx].x, blocks[idx].y, RESET
                    );
                }
            }
            None => {
                println!("{}{}退出棋盘{}", color_ansi, BOLD, RESET);

                // 打印方块退出前的位置
                if let Some(block) = blocks.iter().find(|b| b.id == block_id) {
                    let shape_data = &SHAPE[block.shape as usize];
                    let rect = &shape_data.rect;
                    println!(
                        "  {}{}ID: {}, 颜色: {}, 位置: ({}, {}), 大小: {}x{} 退出棋盘{}",
                        color_ansi,
                        BOLD,
                        block.id,
                        get_color_name(block.color),
                        block.x,
                        block.y,
                        rect.width,
                        rect.height,
                        RESET
                    );
                }

                // 找到对应的块并移除
                let new_blocks = remove_block_and_update_links(&blocks, block_id);
                blocks = new_blocks;
                board = layout_to_board(&blocks);
            }
        }

        print_board_with_gates(&board, gates);

        // 打印剩余方块的位置信息
        if !blocks.is_empty() {
            println!("\n{}剩余方块位置信息:{}", BOLD, RESET);
            for block in &blocks {
                let shape_data = &SHAPE[block.shape as usize];
                let rect = &shape_data.rect;
                println!(
                    "  {}块 {} - 颜色: {}, 位置: ({}, {}), 大小: {}x{}{}",
                    get_color_code(block.color),
                    block.id,
                    get_color_name(block.color),
                    block.x,
                    block.y,
                    rect.width,
                    rect.height,
                    RESET
                );
            }
        }
    }

    println!(
        "\n{}{}解决方案完成！总步数: {}{}",
        BOLD,
        "\x1b[32m",
        solution.history.len(),
        RESET
    );
}
*/

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

/// 创建随机布局
fn create_random_layout(num_blocks: u8) -> Vec<Block> {
    let mut rng = rand::thread_rng();
    let mut blocks = Vec::new();

    for id in 1..=num_blocks {
        let shape_idx = SHAPE_IDX[rng.gen_range(0..SHAPE_IDX.len())];
        let color = rng.gen_range(1..5);

        let block = Block {
            id,
            shape: shape_idx as u8,
            color,
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: rng.gen_range(0..5) as u8,
            y: rng.gen_range(0..5) as u8,
            link: Vec::new(),
        };

        blocks.push(block);
    }

    blocks
}

/// 创建默认的门
fn create_default_gates() -> Vec<Gate> {
    vec![
        // 上方门(红色)
        Gate {
            x: 0,
            y: 0,
            color: 1,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 上方门(红色)
        Gate {
            x: 3,
            y: 0,
            color: 2,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(蓝色)
        Gate {
            x: 1,
            y: (BOARD_HEIGHT - 1) as u8,
            color: 3,
            width: 3,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 右方门(绿色)
        Gate {
            x: (BOARD_WIDTH - 1) as u8,
            y: 4,
            color: 4,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
        // 左方门(黄色)
        Gate {
            x: 0,
            y: 4,
            color: 5,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
    ]
}

/// 手动创建布局（增强版，使用颜色）
fn create_manual_layout() -> Vec<Block> {
    // ANSI颜色常量
    const RESET: &str = "\x1b[0m";
    const BOLD: &str = "\x1b[1m";
    const RED: &str = "\x1b[31m";
    const BLUE: &str = "\x1b[34m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";

    let mut blocks = Vec::new();
    let mut id: u8 = 1;
    let gates = create_default_gates(); // 用于显示当前布局

    println!("\n{}可用形状索引:{}", BOLD, RESET);
    for (idx, &shape_idx) in SHAPE_IDX.iter().enumerate() {
        println!("{}{}.{} 形状: {}", GREEN, idx, RESET, shape_idx);
    }

    println!("\n{}可用颜色:{}", BOLD, RESET);
    println!("{}{}.{} 红色", RED, 1, RESET);
    println!("{}{}.{} 蓝色", BLUE, 2, RESET);
    println!("{}{}.{} 绿色", GREEN, 3, RESET);
    println!("{}{}.{} 黄色", YELLOW, 4, RESET);

    loop {
        println!(
            "\n{}添加第 {} 个方块 (输入 'done' 完成):{}",
            BOLD, id, RESET
        );

        let mut input = String::new();
        print!("{}输入 (done 或 形状索引[0-9]): {}", BOLD, RESET);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();
        if input == "done" {
            break;
        }

        let shape_idx = match input.parse::<usize>() {
            Ok(idx) if idx < SHAPE_IDX.len() => SHAPE_IDX[idx],
            _ => {
                println!("{}无效的形状索引，请输入 0-9 之间的数字{}", RED, RESET);
                continue;
            }
        };

        print!("{}颜色 (1-4): {}", BOLD, RESET);
        io::stdout().flush().unwrap();
        let mut color_input = String::new();
        io::stdin().read_line(&mut color_input).unwrap();
        let color = match color_input.trim().parse::<u8>() {
            Ok(c) if (1..=4).contains(&c) => c,
            _ => {
                println!("{}无效的颜色，请输入 1-4 之间的数字{}", RED, RESET);
                continue;
            }
        };

        // 显示所选颜色
        let color_ansi = match color {
            1 => RED,
            2 => BLUE,
            3 => GREEN,
            4 => YELLOW,
            _ => RESET,
        };
        println!(
            "{}已选择{}{}{}{}",
            BOLD,
            color_ansi,
            get_color_name(color),
            RESET,
            RESET
        );

        print!("{}X 坐标 (0-9): {}", BOLD, RESET);
        io::stdout().flush().unwrap();
        let mut x_input = String::new();
        io::stdin().read_line(&mut x_input).unwrap();
        let x = match x_input.trim().parse::<u8>() {
            Ok(x) if x < BOARD_WIDTH as u8 => x,
            _ => {
                println!("{}无效的 X 坐标，请输入 0-9 之间的数字{}", RED, RESET);
                continue;
            }
        };

        print!("{}Y 坐标 (0-9): {}", BOLD, RESET);
        io::stdout().flush().unwrap();
        let mut y_input = String::new();
        io::stdin().read_line(&mut y_input).unwrap();
        let y = match y_input.trim().parse::<u8>() {
            Ok(y) if y < BOARD_HEIGHT as u8 => y,
            _ => {
                println!("{}无效的 Y 坐标，请输入 0-9 之间的数字{}", RED, RESET);
                continue;
            }
        };

        let block = Block {
            id,
            shape: shape_idx as u8,
            color,
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x,
            y,
            link: Vec::new(),
        };

        blocks.push(block);
        id += 1;

        // 打印当前布局
        println!("\n{}当前布局:{}", BOLD, RESET);
        let board = layout_to_board(&blocks);
        print_board_with_gates(&board, &gates);
    }

    blocks
}

fn solve_main() {
    // // ANSI颜色常量
    // const RESET: &str = "\x1b[0m";
    // const BOLD: &str = "\x1b[1m";
    // const GREEN: &str = "\x1b[32m";
    // const CYAN: &str = "\x1b[36m";

    // println!("{}=== Color Block Jam 解题器 ==={}", BOLD, RESET);
    // println!("{}1.{} 使用随机布局", CYAN, RESET);
    // println!("{}2.{} 手动输入布局", CYAN, RESET);
    // println!("{}3.{} 使用简单测试布局", CYAN, RESET);
    // print!("{}请选择: {}", BOLD, RESET);
    // io::stdout().flush().unwrap();

    // let mut choice = String::new();
    // io::stdin().read_line(&mut choice).unwrap();

    // 创建一个简单的测试布局，便于快速测试
    let blocks = vec![
        Block {
            id: 1,
            shape: SHAPE_IDX[9] as u8, // 单个方块
            color: 1,                  // 红色，对应上方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 2,
            shape: SHAPE_IDX[9] as u8, // 横向两个方块
            color: 2,                  // 蓝色，对应下方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 3,
            shape: SHAPE_IDX[4] as u8, // 纵向两个方块
            color: 3,                  // 绿色，对应右方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 2,
            y: 0,
            link: Vec::new(),
        },
        Block {
            id: 4,
            shape: SHAPE_IDX[3] as u8, // 纵向两个方块
            color: 4,                  // 绿色，对应右方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 5,
            link: Vec::new(),
        },
        Block {
            id: 5,
            shape: SHAPE_IDX[3] as u8, // 纵向两个方块
            color: 5,                  // 绿色，对应右方门
            color2: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 4,
            link: Vec::new(),
        },
    ];

    let gates = create_default_gates();

    match solve(blocks.clone(), &gates) {
        Some(solution) => {
            // // 创建可视化状态
            // let mut visual_state = model::VisualState::new(blocks, gates, solution.history);

            // // 创建渲染器
            // let mut renderer =
            //     render_terminal::TerminalRenderer::new(render_terminal::RenderConfig::default());

            // // 渲染初始状态
            // renderer.render(&visual_state);
            // renderer.display();

            // // 等待用户输入
            // println!("\n按空格键继续，按 'q' 退出");
            // let mut input = String::new();
            // io::stdin().read_line(&mut input).unwrap();

            // // 开始动画
            // while input.trim() != "q" {
            //     if visual_state.next_step() {
            //         renderer.render(&visual_state);
            //         renderer.display();

            //         println!("\n按空格键继续，按 'q' 退出");
            //         input.clear();
            //         io::stdin().read_line(&mut input).unwrap();
            //     } else {
            //         break;
            //     }
            // }
        }
        None => println!("{}未找到解决方案！{}", BOLD, RESET),
    }
}
