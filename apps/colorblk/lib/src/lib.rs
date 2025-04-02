pub mod shape;
pub use crate::shape::*;
pub mod solver;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoardValue {
    pub obstacle: u8, // 0: no obstacle, 255: block all, other: allow_color
    pub block_id: u8,
}

pub type Board = Vec<Vec<BoardValue>>;

/// 表示障碍物的结构体
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Obstacle {
    pub x: u8,
    pub y: u8,
    pub allow_color: u8,
}

/// 表示一个关卡的初始状态
#[derive(Debug, Clone)]
pub struct ColorBlkStage {
    pub board_width: usize,
    pub board_height: usize,
    pub gates: Vec<Gate>,
    pub blocks: Vec<Block>,
    pub obstacles: Vec<Obstacle>,
}

impl ColorBlkStage {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            board_width: width,
            board_height: height,
            gates: Vec::new(),
            blocks: Vec::new(),
            obstacles: Vec::new(),
        }
    }

    /// 创建一个新的棋盘
    pub fn create_board(&self) -> Board {
        let mut board: Board = vec![vec![Default::default(); self.board_width]; self.board_height];

        // 放置障碍物
        for obstacle in &self.obstacles {
            if ((obstacle.x as usize) < self.board_width)
                && ((obstacle.y as usize) < self.board_height)
            {
                board[obstacle.y as usize][obstacle.x as usize].obstacle = obstacle.allow_color;
            }
        }

        board
    }
}

/// 描述门（Gate）的结构体
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gate {
    pub x: u8, // 对于上/下门：x ∈ [0, BOARD_WIDTH - gate.width]
    pub y: u8, // 对于左/右门：y ∈ [0, BOARD_HEIGHT - gate.height]
    pub color: u8,
    pub ice: u8,
    pub lock: u8,
    pub star: u8, // 是否star
    pub width: u8,    // 上/下门的宽度 ∈ [1,3]；左/右门的宽度应为 0
    pub height: u8,   // 左/右门的高度 ∈ [1,3]；上/下门的高度应为 0
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
    pub star: u8,    // 是否star
    pub dir: u8,       // 0: 自由，1: 只能横向，2: 只能纵向
    pub ice: u8,       // 冰层厚度（0 表示无冰）
    pub key: u8,       // 是否有钥匙（0 或 1）
    pub lock: u8,      // 是否上锁（0 或 1）
    pub scissor: u8,   // 0: 无剪刀，非0：剪刀颜色
    pub ropes: Vec<u8>,// 绳索颜色列表
    pub x: u8,         // 在棋盘中x坐标, 注意为bound_box(由BlockData.rect描述)的坐标
    pub y: u8,         // 在棋盘中y坐标, 注意为bound_box的坐标
    pub link: Vec<u8>, // 如果属于一个组，则保存组内所有块的id
}

/// 按编码规则将 Block 属性转换为 u32（不包含 link 部分）
/// 用于设置Board数组
pub fn encode_block(b: &Block) -> BoardValue {
    BoardValue {
        obstacle: 0,
        block_id: b.id,
    }
}

/// layout_to_board 函数，注意正确理解 BlockData 结构
pub fn layout_to_board(blocks: &[Block], stage: &ColorBlkStage) -> Board {
    let mut board = stage.create_board();
    // println!("board....{:?}", board);

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
                    if board_x < stage.board_width && board_y < stage.board_height {
                        board[board_y][board_x] = encode;
                    }
                }
            }
        }
    }

    board
}

/// move_block_to 函数，正确理解 BlockData 结构
fn move_block_to(
    board: &mut Board,
    block: &mut Block,
    new_x: u8,
    new_y: u8,
    stage: &ColorBlkStage,
) -> bool {
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
                if board_x >= stage.board_width || board_y >= stage.board_height {
                    return false;
                }

                // 检查新位置是否被其他方块占据
                let cell = board[board_y][board_x];
                // println!("MBT:({}, {}) cell = {} code = {}", board_x, board_y, cell, code);

                // 无法移动
                if !((cell == code) // 等于本体可以移动
                    || (cell.obstacle == 0 && cell.block_id == 0) // 空白处可以移动
                    || (cell.obstacle != 0 && cell.obstacle == block.color)) // 半透明障碍且颜色一致可以移动
                {
                    // if cell.obstacle != 0 {
                    //     println!("@@@ob...{:?} block..{:?}", cell.obstacle, block.color);
                    // }
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

                if old_x < stage.board_width && old_y < stage.board_height {
                    board[old_y][old_x].block_id = 0;
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

                if board_x < stage.board_width && board_y < stage.board_height {
                    board[board_y][board_x] = code;
                }
            }
        }
    }

    true
}

/// move_entire_block 函数，仅计算新位置
pub fn move_entire_block(
    board: &mut Board,
    block: &mut Block,
    direction: Direction,
    stage: &ColorBlkStage,
) -> bool {
    // 检查方向限制
    match block.dir {
        1 => { // 只能横向移动
            if direction == Direction::Up || direction == Direction::Down {
                return false;
            }
        },
        2 => { // 只能纵向移动
            if direction == Direction::Left || direction == Direction::Right {
                return false;
            }
        },
        _ => {}, // 自由移动，无限制
    }

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
    move_block_to(board, block, new_x, new_y, stage)
}

/// move_group 函数
pub fn move_group(
    board: &mut Board,
    blocks: &mut [Block],
    group_ids: &[u8],
    direction: Direction,
    stage: &ColorBlkStage,
) -> bool {
    // 复制一个棋盘用于尝试移动
    let mut test_board = board.clone();
    let mut test_blocks = blocks.to_vec();

    // 找出组中的所有方块
    let group_blocks: Vec<usize> = blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| group_ids.contains(&b.id))
        .map(|(idx, _)| idx)
        .collect();

    // 先检查组中所有方块的方向限制
    for &idx in &group_blocks {
        let block = &blocks[idx];
        match block.dir {
            1 => { // 只能横向移动
                if direction == Direction::Up || direction == Direction::Down {
                    return false;
                }
            },
            2 => { // 只能纵向移动
                if direction == Direction::Left || direction == Direction::Right {
                    return false;
                }
            },
            _ => {}, // 自由移动，无限制
        }
    }

    // 尝试逐个移动组中的方块
    for &idx in &group_blocks {
        if !move_entire_block(&mut test_board, &mut test_blocks[idx], direction, stage) {
            return false;
        }
    }

    // 如果所有方块都能移动，执行实际移动
    for &idx in &group_blocks {
        move_entire_block(board, &mut blocks[idx], direction, stage);
    }

    true
}

/// 重构后的 can_exit 函数：合并四个方位门的处理逻辑
pub fn can_exit(block: &Block, gates: &[Gate]) -> Option<Direction> {
    let shape_data = &SHAPE[block.shape as usize];

    // 计算方块的边界
    let block_left = block.x;
    let block_right = block.x + (shape_data.rect.width as u8 - 1);
    let block_top = block.y;
    let block_bottom = block.y + (shape_data.rect.height as u8 - 1);

    // 检查每个门
    for gate in gates {
        if !gate.switch {
            continue; // 如果门关闭，跳过
        }

        // 检查方块颜色与门颜色是否匹配,考虑星门
        if !(
            (block.star == 0 && gate.star == 0 && block.color == gate.color) ||
            (block.star == 0 && gate.star != 0 && block.color == gate.color) ||
            (block.star !=0 && gate.star != 0 && block.color == gate.color)
        ) {
            continue;
        }

        // 根据门的类型（由width和height确定）确定方向和检查位置
        match (gate.y, gate.height, gate.x, gate.width) {
            // 上方门
            (0, 0, _, w) if w > 0 => {
                // 方块顶部在顶边界，并且方块左右边界处于门的范围内
                if block_top == 0 && block_left <= gate.x + gate.width - 1 && block_right >= gate.x
                {
                    return Some(Direction::Up);
                }
            }
            // 下方门
            (y, 0, _, w) if w > 0 => {
                // 方块底部触及下边界，并且方块左右边界处于门的范围内
                if block_bottom == y
                    && block_left <= gate.x + gate.width - 1
                    && block_right >= gate.x
                {
                    return Some(Direction::Down);
                }
            }
            // 左方门
            (_, h, 0, 0) if h > 0 => {
                // 方块左侧在左边界，并且方块上下边界处于门的范围内
                if block_left == 0
                    && block_top <= gate.y + gate.height - 1
                    && block_bottom >= gate.y
                {
                    return Some(Direction::Left);
                }
            }
            // 右方门
            (_, h, x, 0) if h > 0 => {
                // 方块右侧触及右边界，并且方块上下边界处于门的范围内
                if block_right == x
                    && block_top <= gate.y + gate.height - 1
                    && block_bottom >= gate.y
                {
                    return Some(Direction::Right);
                }
            }
            _ => continue,
        }
    }

    None
}

/// 辅助函数：从块列表中移除指定块，并更新剩余块的 group 链接（删除已退出块的 id）
pub fn remove_block_and_update_links(blocks: &[Block], id: u8) -> Vec<Block> {
    let mut new_blocks = Vec::new();

    for block in blocks {
        if block.id != id {
            // 保留非目标块，但需要更新其链接
            let mut new_block = block.clone();
            if !new_block.link.is_empty() {
                // 从链接中移除目标块ID
                new_block.link.retain(|&linked_id| linked_id != id);
            }
            new_blocks.push(new_block);
        }
    }

    new_blocks
}

#[cfg(test)]
mod tests {
    // use super::*;
    #[test]
    fn it_works() {
        // let result = ColorblkData::new();
    }
}
