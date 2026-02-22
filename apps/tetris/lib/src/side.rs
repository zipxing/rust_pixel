use crate::constant::*;
//use crossterm::event::{Event, KeyCode};
use log::debug;
//use rand::prelude::*;
//use std::any::Any;
use rust_pixel::{
    event::{event_check, event_emit, timer_cancel, timer_fire, timer_register, timer_stage},
    //game::{Context, Model, Render},
    util::{PointU16, Rand},
};

//https://harddrop.com/wiki/T-Spin_Triple_Setups
//https://katyscode.wordpress.com/2012/10/13/tetris-aside-coding-for-t-spins/

//操作
#[derive(Clone, Copy, PartialEq)]
pub enum Move {
    TurnCw,   //顺时针旋转 clockwise rotation
    TurnCcw,  //逆时针旋转 anti-clockwise rotation
    Down,     //下落 fall
    Left,     //左移 left
    Right,    //右移 right
    Save,     //暂存 save
    Set,      //不移动，仅填充Grid stand still and fill grid
    Clear,    //不移动，在Grid中清除 stand still and clear grid
    DropDown, //直落，不会留滑动时间  fall directly
    Restart,  //重开  restart
}

//用于MoveBlk的返回值
#[derive(PartialEq)]
pub enum MoveRet {
    Normal,      //移动正常 move
    ReadyBottom, //到达最底 reach bottom
    ReachBorder, //到达两边 reach border
    ReachBottom, //到底 reach bottom
}

#[derive(Debug, Default, Copy, Clone)]
pub struct TetrisCore {
    pub grid: [[u8; (HENG + 4) as usize]; (ZONG + 2) as usize],
    pub col_top: [i16; HENG as usize],
    pub col_hole: [i16; HENG as usize],
    pub top_line: i16,
    pub combo: u8,
    pub cur_x: i8,
    pub cur_y: i8,
    pub cur_z: i8,
    pub cur_block: i8,
    pub next_block: i8,
    pub save_block: i8,
    pub shadow_x: i8,
    pub shadow_y: i8,
    pub full_rows: [i8; (ZONG + 2) as usize],
    pub full_row_count: u8,
    pub cling_blocks: [PointU16; 6],
    pub cling_block_count: u8,
    pub block_index: u16,
    pub attack: [u16; 2], //0: line count, 1: attack hole random seed
    pub save_lock: bool,
    pub game_over: bool,
}

impl TetrisCore {
    pub fn new() -> Self {
        let side: TetrisCore = Default::default();
        side
    }

    pub fn dump_debug(&mut self) {
        for i in 0..ZONG {
            let mut s = String::from("");
            for j in 0..HENG {
                if self.grid[i as usize][(j + 2) as usize] > 100 {
                    s.push('B');
                } else {
                    s.push('.');
                }
            }
            debug!("{}", s);
        }
        debug!("col_hole={:?}", self.col_hole);
        debug!("col_top={:?}", self.col_top);
        debug!("top_line={:?}", self.top_line);
    }
}

#[derive(Default, Copy, Clone)]
pub struct TetrisStat {
    pub combo_total: u8,
    pub combo_max: u8,
    pub combo_current: u8,
    pub level: i16,
    pub score: i64,
    pub clear_lines: i32,
    pub attack_lines: i32,
}

impl TetrisStat {
    pub fn new() -> Self {
        let stat: TetrisStat = Default::default();
        stat
    }

    pub fn add_score(&mut self, s: i64) {
        self.score += s;
    }
}

#[derive(Default, Copy, Clone)]
pub struct TetrisCell {
    pub index: u8,
    pub core: TetrisCore,
    pub stat: TetrisStat,
    pub active: bool,
    pub need_draw: bool,
    pub need_stable: bool,
}

impl TetrisCell {
    pub fn new(index: u8) -> Self {
        Self {
            index: index,
            core: TetrisCore::new(),
            stat: TetrisStat::new(),
            active: true,
            need_draw: false,
            need_stable: false,
        }
    }

    pub fn reset(&mut self, blocks: &[i8]) {
        //初始化各种变量
        //init
        for i in 0..(ZONG + 2) as usize {
            for j in 0..(HENG + 4) as usize {
                self.core.grid[i][j] = 200;
            }
        }
        for i in 0..ZONG as usize {
            for j in 0..HENG as usize {
                self.core.grid[i][j + 2] = 0;
            }
        }
        self.core.cur_block = blocks[0];
        self.core.next_block = blocks[1];
        self.core.save_block = -1;
        self.core.save_lock = false;
        self.core.cur_x = 5;
        self.core.cur_y = 0;
        self.core.cur_z = 0;
        self.core.game_over = false;
        self.core.block_index = 0;

        self.make_shadow();

        timer_register(&format!("next-block{}", self.index), 0.8, "_");
        timer_register(&format!("pre-stable{}", self.index), 0.8, "_");
        timer_register(&format!("clear-row{}", self.index), 0.3, "_");
        timer_register(&format!("game-over{}", self.index), 0.12, "_");
        timer_register(&format!("fall{}", self.index), 0.1, "_");
        timer_register(&format!("combo{}", self.index), 0.8, "_");
        timer_register(&format!("attack{}", self.index), 0.8, "_");
    }

    pub fn timer_process(&mut self, blocks: &[i8]) {
        if event_check(&format!("pre-stable{}", self.index), "_") {
            self.need_stable = true;
        }
        if event_check(&format!("clear-row{}", self.index), "_") {
            self.clear_row(false);
        }
        if event_check(&format!("game-over{}", self.index), "_") {
            self.core.game_over = true;
        }
        if event_check(&format!("fall{}", self.index), "_") {
            self.fall(blocks);
        }
    }

    pub fn next_block(&mut self, blocks: &[i8], ai: bool, save: bool) {
        if !ai {
            self.stat.add_score(10);
        }

        self.core.block_index += 1;
        self.core.cur_block = self.core.next_block;
        if !save {
            timer_fire(&format!("next-block{}", self.index), 0);
        }
        self.core.cur_x = 5;
        self.core.cur_y = 0;
        self.core.cur_z = 0;
        self.move_block(Move::Set, ai);
        self.core.next_block = blocks[((self.core.block_index + 1) % BLKQUEUE) as usize];

        if self.index == 0 {
            event_emit("Tetris.RedrawNext");
        }

        /*if  !ai && self.index==0{
            //this.fang_cha[mc.block_index] = this.calcFangCha();
        }*/
    }

    //暂存块,每次确认下落后才能再次存(save_lock)
    //save the block, every time needs to confirm fall to save again
    pub fn save_block(&mut self, blocks: &[i8], ai: bool) {
        if !self.core.save_lock {
            self.core.save_lock = true;
            self.move_block(Move::Clear, ai);
            if self.core.save_block >= 0 {
                let blktmp = self.core.cur_block;
                self.core.cur_block = self.core.save_block;
                self.core.save_block = blktmp;
                self.core.cur_x = 5;
                self.core.cur_y = 0;
                self.core.cur_z = 0;
                self.move_block(Move::Set, ai);
            } else {
                self.core.save_block = self.core.cur_block;
                self.next_block(blocks, ai, false);
            }
            if self.index == 0 {
                event_emit("Tetris.RedrawHold");
            }
            //触发保存块动画
            //this.mtimer.save_block = 10;
        }
    }

    pub fn clear_row(&mut self, ai: bool) {
        if !ai {
            if timer_stage(&format!("game-over{}", self.index)) != 0 {
                timer_cancel(&format!("game-over{}", self.index), false);
                self.core.game_over = false;
            }
        }

        let mg = &mut self.core.grid;

        if self.core.full_row_count != 0 {
            for n in 0..self.core.full_row_count as usize {
                for i in (0..(self.core.full_rows[n] + 1) as usize).rev() {
                    for j in 0..HENG as usize {
                        if i != 0 {
                            // TODO DEBUG : panicked at 'index out of bounds: 
                            // the len is 23 but the index is 18446744073709551613'
                            if mg[i - 1][j + 2] > 100 || mg[i - 1][j + 2] == 0 {
                                if !(mg[i][j + 2] < 10 && mg[i][j + 2] > 0) {
                                    mg[i][j + 2] = mg[i - 1][j + 2];
                                }
                            } else if !(mg[i][j + 2] < 10 && mg[i][j + 2] > 0) {
                                mg[i][j + 2] = 0;
                            }
                        } else {
                            if !(mg[i][j + 2] < 10 && mg[i][j + 2] > 0) {
                                mg[i][j + 2] = 0;
                            }
                        }
                    }
                }
                self.core.full_rows[n] = 0;
            }
            self.update_colholetop(2, 11);
        }
        self.core.full_row_count = 0;
    }

    //calc empty blocks, height of each row for AI uses
    pub fn update_colholetop(&mut self, gxs: u8, gxe: u8) {
        let mc = &mut self.core;
        for m in gxs as usize..(gxe + 1) as usize {
            mc.col_top[m - 2] = 0;
            mc.col_hole[m - 2] = 0;
            let mut ln: usize = 0;
            for n in (1usize..(ZONG + 1) as usize).rev() {
                if mc.grid[ZONG as usize - n][m] > 100 {
                    mc.col_top[m - 2] = n as i16;
                    ln = n;
                    break;
                }
            }
            for n in (1usize..(ln + 1) as usize).rev() {
                if mc.grid[ZONG as usize - n][m] == 0 {
                    mc.col_hole[m - 2] += n as i16;
                }
            }
        }
        mc.top_line = 0;
        for m in 0usize..HENG as usize {
            if mc.col_top[m] > mc.top_line {
                mc.top_line = mc.col_top[m];
            }
        }
    }

    //fall
    pub fn fall(&mut self, blocks: &[i8]) {
        while self.move_block(Move::DropDown, false) != MoveRet::ReachBottom {
            if self.core.game_over {
                break;
            }
        }
        self.next_block(blocks, false, false);
        self.make_shadow();
    }

    //draw the shadow while falling down, for better aiming
    pub fn make_shadow(&mut self) {
        let tmp = self.core;
        loop {
            if self.move_block(Move::DropDown, true) == MoveRet::ReachBottom {
                break;
            }
        }
        let x = self.core.cur_x;
        let y = self.core.cur_y;
        self.core = tmp;
        self.core.shadow_x = x;
        self.core.shadow_y = y;
    }

    pub fn is_in_grid(&mut self, y: i8, x: i8) -> bool {
        x >= 0 && y >= 0 && y < (ZONG + 2) as i8 && x < (HENG + 4) as i8
    }

    pub fn get_md(&mut self, a: i8, b: i8, c: i8) -> u8 {
        BLKDAT[a as usize][b as usize][c as usize]
    }

    pub fn get_gd(&self, y: i8, x: i8) -> u8 {
        self.core.grid[y as usize][x as usize]
    }

    pub fn set_gd(&mut self, y: i8, x: i8, val: u8) {
        self.core.grid[y as usize][x as usize] = val;
    }

    pub fn inner_rect4x4(&mut self, y: i8, x: i8) -> Vec<(i8, i8, i8, i8)> {
        let mut l: Vec<(i8, i8, i8, i8)> = vec![];
        for i in 0..4 {
            for j in 0..4 {
                if self.is_in_grid(y + i, x + j) {
                    l.push((i, j, y + i, x + j));
                }
            }
        }
        l
    }

    fn reach_bottom(&mut self, dir: Move, ai: bool, blk: i8, cx: i8, cy: i8, z: i8) -> MoveRet {
        if dir == Move::Down {
            //normal fall
            if !self.need_stable {
                if timer_stage(&format!("pre-stable{}", self.index)) == 0 {
                    timer_fire(&format!("pre-stable{}", self.index), 0u8);
                }
                for (m, n, my, mx) in self.inner_rect4x4(cy, cx) {
                    if self.get_md(blk, z, m * 4 + n) != 0 {
                        let md = self.get_md(blk, z, m * 4 + n);
                        self.set_gd(my, mx, md);
                    }
                }
                return MoveRet::ReadyBottom;
            }
        }

        self.need_stable = false;

        //加100设置为稳定块，并统计需要显示粘住光晕的块位置
        if !ai {
            self.core.cling_block_count = 0;
        }

        for (m, n, iy, ix) in self.inner_rect4x4(cy, cx) {
            if self.get_md(blk, z, m * 4 + n) != 0 {
                let md = 100 + self.get_md(blk, z, m * 4 + n); //加100,置为稳定块
                self.set_gd(iy, ix, md);
                if !ai {
                    if self.get_gd(iy, ix) != 100 {
                        //纪录下需要显示“粘住光晕”的块坐标及个数
                        self.core.cling_blocks[self.core.cling_block_count as usize] = PointU16 {
                            x: (cx + n - 2) as u16,
                            y: (cy + m) as u16,
                        };
                        self.core.cling_block_count += 1;
                    }
                }
            }
        }

        self.update_colholetop(2, 11);

        //标注满行，检测满行信息 标记到fullrow里 同时标记full_rows_count
        //扫描判断满行,放入fullrows数组
        for m in 0..4 {
            let mut fflag = true;
            let mut in_grid_count = 0;
            for n in 0..HENG {
                if self.is_in_grid(cy + m, (n + 2) as i8) {
                    in_grid_count += 1;
                    let gd = self.get_gd(cy + m, (n + 2) as i8);
                    if gd < 100 || gd == 200 {
                        fflag = false;
                        break;
                    }
                }
            }
            // Row must have all HENG cells in grid to be considered full
            if in_grid_count < HENG {
                fflag = false;
            }
            if fflag {
                let fr = cy + m;
                if fr >= 0 {
                    self.core.full_rows[self.core.full_row_count as usize] = cy + m;
                    self.core.full_row_count += 1;
                }
            }
        }

        //if the row is full, set full_rows_count
        if self.core.full_row_count > 0 {
            if !ai {
                if timer_stage(&format!("game-over{}", self.index)) != 0 {
                    timer_cancel(&format!("game-over{}", self.index), false);
                    self.core.game_over = false;
                }
            }
            self.core.combo += 1;
            if !ai {
                self.core.attack[0] = (self.core.full_row_count - 1) as u16;
                if self.core.combo >= 3 {
                    self.stat.combo_total += self.core.combo;
                    if self.core.combo > self.stat.combo_max {
                        self.stat.combo_max = self.core.combo as u8;
                    }
                    self.stat.combo_current = self.core.combo;
                    self.core.attack[0] += 1; // 如果连击数大于等于3   再给别人加一行
                    timer_fire(&format!("combo{}", self.index), self.core.combo);
                    self.stat.add_score(self.core.combo as i64 * 100 as i64);
                }
                self.core.attack[1] = self.core.block_index;
                self.stat.clear_lines += self.core.full_row_count as i32;
                let fs: [i64; 4] = [50, 150, 300, 500];
                let idx = ((self.core.full_row_count - 1) as usize).min(3);
                self.stat.add_score(fs[idx]);
                let mut fv: Vec<i8> = vec!();
                for f in 0..self.core.full_row_count {
                    fv.push(self.core.full_rows[f as usize]);
                }
                timer_fire(&format!("clear-row{}", self.index), fv);
            }
        } else {
            self.core.combo = 0;
            self.stat.combo_current = 0;
        }
        //进入了下一块处理,可以保存块了
        //proceed to the next block, can save now..
        self.core.save_lock = false;
        MoveRet::ReachBottom
    }

    fn update_block_xyz(&mut self, dir: Move, cx: i8, cy: i8, cz: i8) -> (i8, i8, i8) {
        let (mut x, mut y, mut z) = (0i8, 0i8, 0i8);
        match dir {
            Move::TurnCw => {
                x = cx;
                y = cy;
                z = (cz + 5) % 4;
                timer_cancel(&format!("pre-stable{}", self.index), true);
            }
            Move::TurnCcw => {
                x = cx;
                y = cy;
                z = (cz + 3) % 4;
                timer_cancel(&format!("pre-stable{}", self.index), true);
            }
            Move::Down | Move::DropDown => {
                x = cx;
                y = cy + 1;
                z = cz;
            }
            Move::Left => {
                x = cx - 1;
                y = cy;
                z = cz;
                timer_cancel(&format!("pre-stable{}", self.index), true);
            }
            Move::Right => {
                x = cx + 1;
                y = cy;
                z = cz;
                timer_cancel(&format!("pre-stable{}", self.index), true);
            }
            Move::Set | Move::Clear => {
                x = cx;
                y = cy;
                z = cz;
            }
            _ => {}
        }
        (x, y, z)
    }

    pub fn move_block(&mut self, dir: Move, ai: bool) -> MoveRet {
        if self.core.game_over {
            if dir == Move::Left || dir == Move::Right {
                return MoveRet::ReachBorder;
            } else {
                return MoveRet::ReachBottom;
            }
        }

        if !ai {
            self.need_draw = true;
        }

        let blk = self.core.cur_block;
        let cx = self.core.cur_x;
        let cy = self.core.cur_y;
        let cz = self.core.cur_z;
        let (x, y, z) = self.update_block_xyz(dir, cx, cy, cz);

        //不稳定块置0,100以上为已经下落稳定的块
        for (_i, _j, iy, ix) in self.inner_rect4x4(cy, cx) {
            if self.get_gd(iy, ix) < 100 {
                self.set_gd(iy, ix, 0);
            }
        }

        //清除不稳定块
        if dir == Move::Clear {
            return MoveRet::Normal;
        }

        //检测到了碰撞,可能是到底,到边,或者遇到了别的块,无法下落
        for (i, j, iy, ix) in self.inner_rect4x4(y, x) {
            if self.get_gd(iy, ix) != 0 && self.get_md(blk, z, i * 4 + j) != 0 {
                match dir {
                    Move::Down | Move::DropDown => {
                        return self.reach_bottom(dir, ai, blk, cx, cy, z);
                    }
                    Move::Left | Move::Right | Move::TurnCw | Move::TurnCcw => {
                        for (i, j, iy, ix) in self.inner_rect4x4(cy, cx) {
                            if self.get_gd(iy, ix) == 0 {
                                let gd = self.get_gd(iy, ix);
                                let md = self.get_md(blk, z, i * 4 + j);
                                self.set_gd(iy, ix, gd + md);
                            }
                        }
                        return MoveRet::ReachBorder;
                    }
                    _ => {
                        if dir == Move::Set && !ai {
                            timer_fire(&format!("game-over{}", self.index), 0u8);
                        }
                        return MoveRet::Normal;
                    }
                }
            }
        }

        //更新真正的Grid,置当前x,y,z,返回
        //update the real grid, set x,y,z and return
        for (i, j, iy, ix) in self.inner_rect4x4(y, x) {
            let gd = self.get_gd(iy, ix);
            let md = self.get_md(blk, z, i * 4 + j);
            self.set_gd(iy, ix, gd + md);
        }
        self.core.cur_x = x;
        self.core.cur_y = y;
        self.core.cur_z = z;
        if !ai {
            self.make_shadow();
        }

        MoveRet::Normal
    }

    //旋转动作的辅助函数
    //helping function for doing rotation
    pub fn help_turn(&mut self, d: Move, cmd: &str) -> bool {
        let tcore = self.core.clone();

        for c in cmd.bytes() {
            if c == b'L' {
                self.move_block(Move::Left, false);
            }
            if c == b'R' {
                self.move_block(Move::Right, false);
            }
        }
        let mret = self.move_block(d, false);
        if mret == MoveRet::Normal {
            self.make_shadow();
            return true;
        } else {
            self.core = tcore;
        }
        return false;
    }

    pub fn attacked(&mut self, tr: &mut Rand, line: u16, space_seed: u16) {
        let mut flowflag = 0u8;

        if self.core.game_over || line <= 0 {
            return;
        }

        if timer_stage(&format!("clear-row{}", self.index)) != 0 {
            timer_cancel(&format!("clear-row{}", self.index), false);
        }

        if timer_stage(&format!("fall{}", self.index)) != 0 {
            timer_cancel(&format!("fall{}", self.index), false);
        }

        tr.srand(space_seed as u64);
        let mut tgrid = self.core.grid.clone();
        for i in 0..ZONG - line {
            for j in 0..HENG {
                let gd = self.get_gd((i + line) as i8, (2 + j) as i8);
                tgrid[i as usize][(2 + j) as usize] = gd;
                if gd < 10 && gd > 0 {
                    flowflag = 1;
                    tgrid[i as usize][(2 + j) as usize] = 0;
                }
            }
        }

        for i in 0..line {
            let r = tr.rand() as u16 % HENG;
            for j in 0..HENG {
                if r == j {
                    tgrid[(ZONG - 1 - i) as usize][(2 + j) as usize] = 0;
                } else {
                    tgrid[(ZONG - 1 - i) as usize][(2 + j) as usize] = 111;
                }
            }
        }

        self.core.grid = tgrid;

        if flowflag != 0 {
            let x = self.core.cur_x;
            let mut y = self.core.cur_y;
            let z = self.core.cur_z;
            let blk = self.core.cur_block;
            let mut needup = false;
            for i in 0..4 {
                for j in 0..4 {
                    if self.is_in_grid(y + i, x + j) {
                        if self.get_gd(y + i, x + j) != 0 && self.get_md(blk, z, i * 4 + j) != 0 {
                            needup = true;
                        }
                    }
                }
            }
            if needup {
                self.core.cur_y -= line as i8;
                y = self.core.cur_y;
            }
            for i in 0..4 {
                for j in 0..4 {
                    if self.is_in_grid(y + i, x + j) {
                        let gd = self.get_gd(y + i, x + j);
                        let md = self.get_md(blk, z, i * 4 + j);
                        self.set_gd(y + i, x + j, gd + md);
                    }
                }
            }
        }

        for i in 0..HENG {
            self.core.col_top[i as usize] += line as i16;
        }

        if self.core.full_row_count != 0 {
            for f in 0..self.core.full_row_count {
                self.core.full_rows[f as usize] -= line as i8;
            }
        }
    }
}
