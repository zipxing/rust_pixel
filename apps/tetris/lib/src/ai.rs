//use crossterm::event::{Event, KeyCode};
//use log::debug;
//use rand::prelude::*;
use crate::{
    side::{Move, MoveRet, TetrisCell, TetrisCore},
    constant::*,
};
//use std::any::Any;
use std::cmp::min;
//use tge::{
//    event::{event_emit, timer_fire},
//    game::Model,
//    util::Rand,
//};

#[derive(Debug)]
pub struct AiDat {
    pub score: i64,
    pub acts: String,
    pub core: TetrisCore,
    pub cx: i8,
    pub cy: i8,
    pub cf: u8,
    pub cc: u8,
}

pub struct TetrisAi {
    pub mact_queue: String,
    pub tact_queue: String,
    pub max_score: i64,
    pub work2idx: i64,
    pub ms_scan: Vec<AiDat>,
    pub mode: usize,
}

pub struct AiScore {
    pub init: i64,
    pub clear_line: [i64; 5],
    pub fangcha: f32,
    pub top_avg: i64,
    pub hole: i64,
    pub combo: [i64; 2],
    pub xiagu_max: i64,
    pub xiagu: i64,
}

const AI_SCORES: [AiScore; 4] = [
    //vs safe...
    AiScore {
        init: 5000,
        clear_line: [0, 6000, 7200, 10800, 14400],
        fangcha: -0.5,
        top_avg: -300,
        hole: -2000,
        combo: [300000, 2500],
        xiagu_max: 2,
        xiagu: -500,
    },
    //vs normal...
    AiScore {
        init: 5000,
        clear_line: [0, -7000, -6400, 160, 240],
        fangcha: -0.5,
        top_avg: -30,
        hole: -2500,
        combo: [300000, 2500],
        xiagu_max: 2,
        xiagu: -500,
    },
    //adventure safe...
    AiScore {
        init: 5000,
        clear_line: [0, 6000, 7200, 10800, 14400],
        fangcha: -0.5,
        top_avg: -300,
        hole: -2000,
        combo: [300000, 2500],
        xiagu_max: 1,
        xiagu: -500,
    },
    //adventure normal...
    AiScore {
        init: 5000,
        clear_line: [0, 6000, 7200, 10800, 14400],
        fangcha: -0.5,
        top_avg: -30,
        hole: -2500,
        combo: [300000, 2500],
        xiagu_max: 1,
        xiagu: -500,
    },
];

impl TetrisAi {
    pub fn new() -> Self {
        Self {
            mact_queue: String::from(""),
            tact_queue: String::from(""),
            max_score: 0,
            work2idx: -1,
            ms_scan: vec![],
            mode: 0,
        }
    }

    pub fn get_mode(&mut self, tc: &TetrisCore) {
        if tc.top_line > 10 {
            self.mode = 0;
        } else {
            self.mode = 1;
        }
    }

    pub fn get_side_score(
        &self,
        core: &TetrisCore,
        cx: i8,
        _cy: i8,
        cf: u8,
        nf: u8,
        ccombo: u8,
    ) -> i64 {
        let aip = &AI_SCORES[self.mode];
        let mut score = aip.init;
        let mut hole_count = 0;
        let mut top_total = 0;
        let mut xiagu: [i16; HENG as usize] = Default::default();
        let mut xiagu_count = 0;
        let mut xiagu_total = 0;

        //计算总空
        //calc total blanks
        for i in 0..HENG as usize {
            hole_count += core.col_hole[i];
            top_total += core.col_top[i] * 10;
            xiagu[i] = 0;
            if i == 0 {
                if core.col_top[1] > core.col_top[0] {
                    xiagu[i] = core.col_top[1] - core.col_top[0];
                }
            } else if i == (HENG - 1) as usize {
                if core.col_top[i - 1] > core.col_top[i] {
                    xiagu[i] = core.col_top[i - 1] - core.col_top[i];
                }
            } else {
                if (core.col_top[i + 1] > core.col_top[i])
                    && (core.col_top[i - 1] > core.col_top[i])
                {
                    xiagu[i] = min(core.col_top[i - 1], core.col_top[i + 1]) - core.col_top[i];
                }
            }
            if xiagu[i] > 2 {
                xiagu_count += 1;
            }
            xiagu_total += xiagu[i];
        }

        //计算平均行高,计算行高方差
        //calc average row height and row height variance
        let top_avg = top_total as f32 / HENG as f32;
        let mut fangcha = 0f32;
        for i in 0..HENG as usize {
            let t = core.col_top[i] as f32 * 10.0 - top_avg;
            fangcha += t * t;
        }

        //按方差评分
        //scoring based on variance
        score += (fangcha * aip.fangcha) as i64;

        //鼓励靠边...
        //encourage closer to border
        score += ((cx - 5) as i64 * (cx - 5) as i64
            + (core.cur_x - 5) as i64 * (core.cur_x - 5) as i64)
            * 5i64;

        //进攻模式不鼓励只消一两行，鼓励消掉两行以上
        //aggressive mode discourage few row clear, need to clear at least 2 rows
        score += aip.clear_line[cf as usize];
        score += aip.clear_line[nf as usize];

        //局面越低约均衡越好
        score += top_avg as i64 * aip.top_avg;

        //空洞越少越好
        score += hole_count as i64 * aip.hole;

        //连击加分
        //combo rewards
        if ccombo > 2 {
            score += ccombo as i64 * aip.combo[0];
        } else {
            score += ccombo as i64 * aip.combo[1];
        }

        //连击加分
        //combo rewards
        if core.combo > 2 {
            score += core.combo as i64 * aip.combo[0];
        } else {
            score += core.combo as i64 * aip.combo[1];
        }

        //峡谷越少越好，有一个大峡谷不怕，怕出现两个峡谷
        //fewer valley the better, one is ok, but two..

        if xiagu_count >= aip.xiagu_max {
            score += aip.xiagu * xiagu_total as i64;
        }

        score
    }

    pub fn ai_f1(
        &mut self,
        blocks: &[i8],
        tc: &mut TetrisCell,
        cx: i8,
        cy: i8,
        cf: u8,
        combo: u8,
        fs: bool,
        save: bool,
        scan: bool,
    ) {
        let b1 = tc.core;

        let bq = self.tact_queue.clone();
        let tmpz = ZCOUNT[tc.core.cur_block as usize];

        let mut tmpsave = 0;
        if save {
            tmpsave = 1;
        }

        for s2 in 0..tmpsave + 1 {
            tc.core = b1;
            self.tact_queue = bq.clone();
            if s2 != 0 {
                self.tact_queue.push('S');
                tc.save_block(blocks, true);
            }
            let b2 = tc.core;
            let bq1 = self.tact_queue.clone();

            for nz in 0..tmpz {
                tc.core = b2;
                self.tact_queue = bq1.clone();

                //旋转
                for _n in 0..nz {
                    self.tact_queue.push('T');
                    tc.move_block(Move::TurnCw, true);
                }

                let b3 = tc.core;
                let bq2 = self.tact_queue.clone();

                for x2 in 0..3 {
                    tc.core = b3;
                    self.tact_queue = bq2.clone();
                    match x2 {
                        0 => {
                            if fs {
                                self.ai_f2(blocks, tc, cx, cy, cf, combo, scan);
                            } else {
                                self.ai_f3(blocks, tc, cx, cy, cf, combo, scan);
                            }
                        }
                        1 => {
                            while tc.move_block(Move::Left, true) != MoveRet::ReachBorder {
                                self.tact_queue.push('L');
                                if fs {
                                    self.ai_f2(blocks, tc, cx, cy, cf, combo, scan);
                                } else {
                                    self.ai_f3(blocks, tc, cx, cy, cf, combo, scan);
                                }
                            }
                        }
                        2 => {
                            while tc.move_block(Move::Right, true) != MoveRet::ReachBorder {
                                self.tact_queue.push('R');
                                if fs {
                                    self.ai_f2(blocks, tc, cx, cy, cf, combo, scan);
                                } else {
                                    self.ai_f3(blocks, tc, cx, cy, cf, combo, scan);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn ai_f2(
        &mut self,
        blocks: &[i8],
        tg: &mut TetrisCell,
        _cxs: i8,
        _cys: i8,
        _cfs: u8,
        combo: u8,
        scan: bool,
    ) {
        //保存现场
        //save
        let b0 = tg.core;
        let bq0 = self.tact_queue.clone();

        //直接下落
        //fall
        while tg.move_block(Move::DropDown, true) != MoveRet::ReachBottom {}
        self.tact_queue.push('W');
        let ccombo = tg.core.combo;
        let cf = tg.core.full_row_count;
        tg.clear_row(true);
        let cx = tg.core.cur_x;
        let cy = tg.core.cur_y;
        tg.next_block(blocks, true, false);
        self.tact_queue.push('N');

        let s = self.get_side_score(&tg.core, cx, cy, cf, 0, combo);
        if scan {
            self.ms_scan.push(AiDat {
                score: s,
                acts: self.tact_queue.clone(),
                core: tg.core,
                cx,
                cy,
                cf,
                cc: ccombo,
            });
        } else {
            //
        }

        //恢复现场
        //resume
        tg.core = b0;
        self.tact_queue = bq0.clone();
    }

    pub fn ai_f3(
        &mut self,
        _blocks: &[i8],
        tg: &mut TetrisCell,
        cx: i8,
        cy: i8,
        cf: u8,
        combo: u8,
        _scan: bool,
    ) {
        let b = tg.core;
        let bq = self.tact_queue.clone();

        let mut mx = 0;
        for i in 0..4 {
            let xx = tg.core.cur_x + i;
            if xx >= HENG as i8 {
                break;
            }
            if tg.core.col_top[xx as usize] > mx {
                mx = tg.core.col_top[xx as usize];
            }
        }
        tg.move_block(Move::Set, true);

        //直接下落
        //fall
        while tg.move_block(Move::DropDown, true) != MoveRet::ReachBottom {}
        self.tact_queue.push('W');
        let nf = tg.core.full_row_count;
        tg.clear_row(true);
        let s = self.get_side_score(&tg.core, cx, cy, cf, nf, combo);
        if s > self.max_score {
            self.mact_queue = self.tact_queue.clone();
            self.mact_queue.push('N');
            self.max_score = s;
        }

        tg.core = b;
        self.tact_queue = bq.clone();
    }

    //如果自动运行动作序列为空则计算生成指令序列，否则返回动作指令
    //if auto run sequence is empty, compute new action, otherwise return actions
    pub fn get_ai_act(&mut self, blocks: &[i8], tg: &mut TetrisCell) -> char {
        let len = self.mact_queue.len();
        let topn = 6;
        let min_score = -9000000000i64;

        //需要计算自动队列,第一帧先扫描第一块
        //needs to compute auto run queue, scan the 1st block in the 1st frame
        if len == 0 && self.work2idx < 0 {
            self.max_score = min_score;
            //保存现场
            //save
            let b = tg.core;
            //遍历第一块获取分数和ms_scan列表
            //search the score and ms_scan list of the 1st block
            self.tact_queue = String::from("");
            tg.clear_row(true);
            self.get_mode(&tg.core);
            self.ms_scan.clear();
            self.ai_f1(blocks, tg, 0, 0, 0, 0, true, false, true);
            //高分在前面
            //higher score first
            self.ms_scan.sort_by(|a, b| b.score.cmp(&a.score));
            //debug!("AiSort::{:#?}", self.ms_scan);
            //删除并释放topN个最低分数
            //delete and release top N least scores
            if self.ms_scan.len() > topn {
                for _i in 0..topn {
                    let _tmp = self.ms_scan.pop();
                }
            }
            self.work2idx = self.ms_scan.len() as i64 - 1;
            tg.core = b;
            return ' ';
        }

        //针对ms_scan列表,进行第二块的遍历运算,每帧只算1个布局
        //for ms_scan list, iterate the 2nd block, 1 layout per frame
        if self.work2idx >= 0 {
            let b = tg.core;
            let m = &self.ms_scan[self.work2idx as usize];
            tg.core = m.core;
            let (cx, cy, cf, cc) = (m.cx, m.cy, m.cf, m.cc);
            self.tact_queue = m.acts.clone();
            self.ai_f1(blocks, tg, cx, cy, cf, cc, false, false, false);
            tg.core = b;
            self.work2idx -= 1;
            if self.work2idx == -1 {
                self.ms_scan.clear();
            }
            return ' ';
        }

        //取走第一个动作码，返回
        //get the 1st action, return
        let cret = self.mact_queue.chars().nth(0).unwrap();
        self.mact_queue.remove(0);
        cret
    }
}
