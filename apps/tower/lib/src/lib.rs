pub const BW: usize = 3;
pub const BH: usize = 3;
pub const TOWERW: usize = 16 * BW;
pub const TOWERH: usize = 12 * BH;

pub const MAX_MONSTER_COUNT: usize = 100;
pub const MAX_BOMB_COUNT: usize = 100;
pub const MAX_LASER_COUNT: usize = 500;
pub const MAX_BULLET_COUNT: usize = 500;
pub const MAX_BLOCK_COUNT: usize = 200;
pub const MAX_TOWER_COUNT: usize = 200;

pub fn check_passable(v: u8) -> bool {
    v > 5 || v == 0
}

pub mod block;
pub mod bomb;
pub mod bullet;
pub mod laser;
pub mod monster;
pub mod tower;
