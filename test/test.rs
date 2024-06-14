pub const HENG: usize = 10;
pub const ZONG: usize = 20;

#[derive(Default, Copy, Clone)]
pub struct TetrisCell {
    pub grid: [[u8; (HENG + 4) as usize]; (ZONG + 2) as usize],
    pub col_top: [u8; HENG as usize],
    pub col_hole: [u8; HENG as usize],
    pub top_line: u8,
    pub cur_x: u8,
    pub cur_y: u8,
    pub cur_z: u8,
    pub cur_block: i8, 
    pub next_block: i8,
    pub save_block: i8,
    pub shadow_x: u8,
    pub shadow_y: u8,
    pub full_rows: [u8; (ZONG + 2) as usize],
    pub full_row_count: u8,
    pub cling_blocks: [Point; 6],
    pub cling_block_count: u8,
    pub block_index: u16,
    pub attack: [u16; 2],
    pub save_lock: bool,
    pub game_over: bool,
    pub game_result: u8,
    pub combo: u8,
	pub combo_total: i16,
	pub combo_max: i16,
	pub combo_current: i16,
	pub level: i16,
	pub score: i64,
	pub clear_lines: i32,
    pub is_active: bool,
}

impl TetrisCell {
    pub fn new() -> Self {
        let cell: TetrisCell = Default::default();
        cell
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

fn main() {
    for i in 0..12 {
        println!("{}...{}", i, i / 3);
    }
    let tc = TetrisCell::new();
    for i in (0..10).rev() {
        println!("...{}", i);
    }
    for i in 0..10000000 {
        let c = tc.clone();
    }
}
