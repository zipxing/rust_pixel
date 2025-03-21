//
// implement core algorithm...
//

#![allow(dead_code)]
use rust_pixel::util::Rand;

pub struct ColorblkData {
    pub rand: Rand,
    pub pool: Vec<u8>,
    pub index: usize,
}

impl ColorblkData {
    pub fn new() -> Self {
        let mut rd = Rand::new();
        rd.srand_now();
        Self {
            rand: rd,
            pool: vec![],
            index: 0,
        }
    }

    pub fn shuffle(&mut self) {
        self.pool.clear();
        for i in 1..=52u8 {
            self.pool.push(i);
        }
        self.rand.shuffle(&mut self.pool);
        // println!("shuffle ok...");
    }

    pub fn next(&mut self) -> u8 {
        let ret;
        if self.pool.len() > 0 {
            ret = self.pool[self.index];
            self.index = (self.index + 1) % 52;
        } else {
            ret = 0;
        }
        ret
    }
}

pub mod shape;
use crate::shape::*;
pub mod solver;
use crate::solver::*;

pub const BOARD_WIDTH: usize = 5;
pub const BOARD_HEIGHT: usize = 6;
pub type Board = [[u32; BOARD_WIDTH]; BOARD_HEIGHT];
pub const OBSTACLE: u32 = 100_000_000;

/// 描述门（Gate）的结构体
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gate {
    pub x: u8, // 对于上/下门：x ∈ [0, BOARD_WIDTH - gate.width]
    pub y: u8, // 对于左/右门：y ∈ [0, BOARD_HEIGHT - gate.height]
    pub color: u8,
    pub width: u8,  // 上/下门的宽度 ∈ [1,3]；左/右门的宽度应为 0
    pub height: u8, // 左/右门的高度 ∈ [1,3]；上/下门的高度应为 0
    pub switch: bool, // 门的开关状态，默认为 true（开启状态）
}

/// 定义方向枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// 表示一个块的数据结构
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    pub id: u8,        // 块的唯一自增编号，从1开始
    pub shape: u8,     // 块使用的形状索引（取值范围：0..SHAPE.len()-1）
    pub color: u8,     // 主颜色
    pub color2: u8,    // 边缘颜色
    pub ice: u8,       // 冰层厚度（0 表示无冰）
    pub key: u8,       // 是否有钥匙（0 或 1）
    pub lock: u8,      // 是否上锁（0 或 1）
    pub x: u8,         // 在棋盘中左上角的 x 坐标, 注意为bound_box(由BlockData.rect描述)的坐标，不是grid的坐标
    pub y: u8,         // 在棋盘中左上角的 y 坐标, 注意为bound_box(由BlockData.rect描述)的坐标，不是grid的坐标
    pub link: Vec<u8>, // 如果属于一个组，则保存组内所有块的id
}

/// 按编码规则将 Block 属性转换为 u32（不包含 link 部分）
/// 用于设置Board数组
pub fn encode_block(b: &Block) -> u32 {
    (b.id as u32)
        + (b.color as u32 * 100)
        + (b.color2 as u32 * 10_000)
        + (b.ice as u32 * 100_000)
        + (b.key as u32 * 1_000_000)
        + (b.lock as u32 * 10_000_000)
}

/// layout_to_board 函数，注意正确理解 BlockData 结构
pub fn layout_to_board(blocks: &[Block]) -> Board {
    let mut board = [[0; BOARD_WIDTH]; BOARD_HEIGHT];

    for block in blocks {
        let shape_data = &SHAPE[block.shape as usize];
        let encode = encode_block(&block);

        // 遍历形状的5x5 grid
        for grid_y in 0..5 {
            for grid_x in 0..5 {
                // 只处理grid中值为1的位置
                if shape_data.grid[grid_y][grid_x] == 1 {
                    // 计算棋盘上的实际坐标
                    // block.x 和 block.y 是边界框在棋盘上的坐标
                    let board_x = block.x as usize + (grid_x - shape_data.rect.x);
                    let board_y = block.y as usize + (grid_y - shape_data.rect.y);

                    // 确保坐标在棋盘范围内
                    if board_x < BOARD_WIDTH && board_y < BOARD_HEIGHT {
                        // 设置格子的值：高8位是颜色，低8位是方块ID
                        board[board_y][board_x] = encode;
                    }
                }
            }
        }
    }

    board
}

/// move_block_to 函数，正确理解 BlockData 结构
fn move_block_to(board: &mut Board, block: &mut Block, new_x: u8, new_y: u8) -> bool {
    let shape_data = &SHAPE[block.shape as usize];
    let code = encode_block(&block);

    // 检查新位置是否合法
    for grid_y in 0..5 {
        for grid_x in 0..5 {
            if shape_data.grid[grid_y][grid_x] == 1 {
                // 计算新位置在棋盘上的坐标
                let board_x = new_x as usize + (grid_x - shape_data.rect.x);
                let board_y = new_y as usize + (grid_y - shape_data.rect.y);

                // 检查是否超出棋盘边界
                if board_x >= BOARD_WIDTH || board_y >= BOARD_HEIGHT {
                    return false;
                }

                // 检查新位置是否被其他方块占据
                let cell = board[board_y][board_x];
                // println!("MBT:({}, {}) cell = {} code = {}", board_x, board_y, cell, code);
                if cell != 0 && cell != code {
                    return false;
                }
            }
        }
    }

    // 从原位置移除方块
    for grid_y in 0..5 {
        for grid_x in 0..5 {
            if shape_data.grid[grid_y][grid_x] == 1 {
                // 计算原位置在棋盘上的坐标
                let old_x = block.x as usize + (grid_x - shape_data.rect.x);
                let old_y = block.y as usize + (grid_y - shape_data.rect.y);

                if old_x < BOARD_WIDTH && old_y < BOARD_HEIGHT {
                    board[old_y][old_x] = 0;
                }
            }
        }
    }

    // 更新方块位置
    block.x = new_x;
    block.y = new_y;

    // 在新位置放置方块
    for grid_y in 0..5 {
        for grid_x in 0..5 {
            if shape_data.grid[grid_y][grid_x] == 1 {
                // 计算新位置在棋盘上的坐标
                let board_x = new_x as usize + (grid_x - shape_data.rect.x);
                let board_y = new_y as usize + (grid_y - shape_data.rect.y);

                if board_x < BOARD_WIDTH && board_y < BOARD_HEIGHT {
                    board[board_y][board_x] = code;
                }
            }
        }
    }

    true
}

/// move_entire_block 函数，仅计算新位置
pub fn move_entire_block(board: &mut Board, block: &mut Block, direction: Direction) -> bool {
    if direction == Direction::Up && block.y == 0 {
        return false;
    }
    if direction == Direction::Left && block.x == 0 {
        return false;
    }
    // 计算新位置的坐标
    let (new_x, new_y) = match direction {
        Direction::Up => (block.x, block.y - 1),
        Direction::Down => (block.x, block.y + 1),
        Direction::Left => (block.x - 1, block.y),
        Direction::Right => (block.x + 1, block.y),
    };

    // 检查是否可以移动到新位置
    move_block_to(board, block, new_x, new_y)
}

/// move_group 函数
pub fn move_group(board: &mut Board, blocks: &mut [Block], group_ids: &[u8], direction: Direction) -> bool {
    // 复制一个棋盘用于尝试移动
    let mut test_board = *board;
    let mut test_blocks = blocks.to_vec();

    // 找出组中的所有方块
    let group_blocks: Vec<usize> = blocks.iter()
        .enumerate()
        .filter(|(_, b)| group_ids.contains(&b.id))
        .map(|(idx, _)| idx)
        .collect();

    // 尝试逐个移动组中的方块
    for &idx in &group_blocks {
        if !move_entire_block(&mut test_board, &mut test_blocks[idx], direction) {
            return false;
        }
    }

    // 如果所有方块都能移动，执行实际移动
    for &idx in &group_blocks {
        move_entire_block(board, &mut blocks[idx], direction);
    }

    true
}

// /// 初始化一个包含障碍的棋盘
// pub fn init_board_with_obstacles() -> Board {
//     let mut board: Board = [[0; BOARD_WIDTH]; BOARD_HEIGHT];
//     board[4][4] = OBSTACLE;
//     board[5][5] = OBSTACLE;
//     board[3][7] = OBSTACLE;
//     board
// }

/// 重构后的 can_exit 函数：合并四个方位门的处理逻辑
pub fn can_exit(block: &Block, gates: &[Gate]) -> Option<Direction> {
    let block_data = &SHAPE[block.shape as usize];
    let rect = block_data.rect;

    // 计算方块在棋盘上的实际边界
    let block_left = block.x as usize;
    let block_top = block.y as usize;
    let block_right = block_left + rect.width;
    let block_bottom = block_top + rect.height;

    // 检查每个门的条件
    for gate in gates {
        // 块颜色必须与门颜色匹配
        if block.color != gate.color {
            continue;
        }

        // 定义变量用于门的检查
        let (is_at_edge, block_size_fits, gate_range, block_range, direction) = match (gate.y, gate.height, gate.x, gate.width) {
            // 上方门
            (0, 0, _, w) if w > 0 => (
                block_top == 0,                        // 是否在边缘
                rect.width <= w as usize,              // 方块尺寸是否符合
                (gate.x as usize, gate.x as usize + w as usize), // 门范围
                (block_left, block_right),             // 方块范围
                Direction::Up                          // 方向
            ),
            // 下方门
            (y, 0, _, w) if y as usize == BOARD_HEIGHT - 1 && w > 0 => (
                block_bottom == BOARD_HEIGHT,
                rect.width <= w as usize,
                (gate.x as usize, gate.x as usize + w as usize),
                (block_left, block_right),
                Direction::Down
            ),
            // 左侧门
            (_, h, 0, 0) if h > 0 => (
                block_left == 0,
                rect.height <= h as usize,
                (gate.y as usize, gate.y as usize + h as usize),
                (block_top, block_bottom),
                Direction::Left
            ),
            // 右侧门
            (_, h, x, 0) if x as usize == BOARD_WIDTH - 1 && h > 0 => (
                block_right == BOARD_WIDTH,
                rect.height <= h as usize,
                (gate.y as usize, gate.y as usize + h as usize),
                (block_top, block_bottom),
                Direction::Right
            ),
            _ => continue // 不是有效的门
        };

        // 统一处理门的逻辑
        if is_at_edge && block_size_fits {
            let (gate_start, gate_end) = gate_range;
            let (block_start, block_end) = block_range;

            // 方块必须至少部分地与门重叠，并且完全在门的范围内
            if block_start < gate_end && block_end > gate_start && 
               block_start >= gate_start && block_end <= gate_end {
                return Some(direction);
            }
        }
    }
    
    None
}

/// 辅助函数：从块列表中移除指定块，并更新剩余块的 group 链接（删除已退出块的 id）
pub fn remove_block_and_update_links(blocks: &[Block], removed_id: u8) -> Vec<Block> {
    blocks
        .iter()
        .filter_map(|b| {
            if b.id == removed_id {
                None
            } else {
                let mut new_block = b.clone();
                new_block.link.retain(|&id| id != removed_id);
                Some(new_block)
            }
        })
        .collect()
}


#[cfg(test)]
mod tests {
    // use super::*;
    #[test]
    fn it_works() {
        // let result = ColorblkData::new();
    }
}

