use log::info;
use rust_pixel::{
    algorithm::union_find::{UnionFind, UF},
    context::Context,
    event::{
        event_check, event_emit, timer_fire, timer_register, timer_set_time, Event, MouseButton,
        MouseEventKind::*,
    },
    game::Model,
    util::Rand,
};
use std::collections::HashMap;
use std::fmt;

pub const NROW: usize = 5;
pub const NCOL: usize = 5;
#[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
pub const CELLW: usize = 8;
#[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32")))]
pub const CELLW: usize = 10;
#[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
pub const CELLH: usize = 8;
#[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32")))]
pub const CELLH: usize = 5;
#[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
pub const ADJX: usize = 9;
#[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
pub const ADJY: usize = 10;
#[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32")))]
pub const ADJX: usize = 3;
#[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32")))]
pub const ADJY: usize = 8;

//四种基本颜色 + Tower
pub const COLOR_COUNT: usize = 5;
pub const LEVELUP_TIME: f32 = 0.03;

#[derive(PartialEq)]
pub enum CityState {
    Normal,
    MergeMovie,
    LevelUpMovie,
    DropMovie,
}

impl From<u8> for CityState {
    fn from(orig: u8) -> Self {
        let cs;
        match orig {
            0 => cs = CityState::Normal,
            1 => cs = CityState::MergeMovie,
            2 => cs = CityState::LevelUpMovie,
            3 => cs = CityState::DropMovie,
            _ => cs = CityState::Normal,
        };
        cs
    }
}

#[derive(Default, Debug)]
pub struct CityCell {
    pub id: i16,
    pub from_id: Option<i16>,
    pub to_id: Option<i16>,

    //1,2,3,4:base, 5:tower, 6,7,8,9...:wonder
    //-1 means blank
    pub color: i8,

    //1~29:base, 30:tower, 60,90,120...:wonder
    pub level: i16,

    //marks the border
    pub border: u8,

    //marks is ready to consolidate to tower
    pub ready2t: bool,
}

impl CityCell {
    pub fn new(id: i16, color: i8, level: i16) -> Self {
        Self {
            id,
            from_id: None,
            to_id: None,
            color,
            level,
            border: 0,
            ready2t: false,
        }
    }

    pub fn assign(&self, cc: &mut CityCell) {
        cc.id = self.id;
        cc.from_id = self.from_id;
        cc.to_id = self.to_id;
        cc.color = self.color;
        cc.level = self.level;
        cc.border = self.border;
        cc.ready2t = self.ready2t;
    }
}

//用于打印日志，调试
//for logs and debugging
impl fmt::Display for CityCell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sf;
        let st;
        if self.from_id == None {
            sf = "*".to_string();
        } else {
            sf = format!("{}", self.from_id.unwrap()).to_string();
        }
        if self.to_id == None {
            st = "*".to_string();
        } else {
            st = format!("{}", self.from_id.unwrap()).to_string();
        }
        write!(
            f,
            "{}.{}~{}.c{}.l{}.b{}.{}",
            self.id, sf, st, self.color, self.level, self.border, self.ready2t
        )
    }
}

#[derive(Default, Debug)]
pub struct CityUnit {
    cells: Vec<i16>,
    merging: bool,
}

impl CityUnit {
    pub fn new() -> Self {
        Self {
            cells: vec![],
            merging: false,
        }
    }

    pub fn add(&mut self, id: i16) {
        if !self.cells.contains(&id) {
            self.cells.push(id);
        }
    }
}

pub struct CityLevelup {
    pub cellid: i16,
    pub from: i16,
    pub to: i16,
}

impl CityLevelup {
    pub fn new() -> Self {
        Self {
            cellid: -1,
            from: -1,
            to: -1,
        }
    }
}

#[derive(Debug)]
pub struct CityMerge {
    object_id: i16,
    merge_cells: Vec<i16>,
}

impl CityMerge {
    pub fn new() -> Self {
        Self {
            object_id: 0,
            merge_cells: vec![],
        }
    }
}

pub struct CityModel {
    pub rand: Rand,
    pub grid: [[CityCell; NCOL]; NROW],
    pub units: HashMap<usize, CityUnit>,
    pub cell2unit: HashMap<i16, usize>,
    pub merge: CityMerge,
    pub levelup: CityLevelup,
    pub ready_del: i16,
    pub move_cells: Vec<i16>,
    pub ready2t: bool,
}

//from_id might be negative, in this case returns y = 1
//and amend in the code render.rs:draw_moving line 112
pub fn get_xy(id: i16) -> (usize, usize) {
    let x = (id + 25) as usize % NCOL;
    if id >= 0 {
        return (x, id as usize / NCOL);
    }
    (x, 0)
}

pub fn check_neighbor(x: i8, y: i8, u: &CityUnit, x1: usize, x2: usize) -> bool {
    if x < x1 as i8 || x >= x2 as i8 || y < 0 || y >= NROW as i8 {
        return true;
    }
    !u.cells.contains(&(y as i16 * NCOL as i16 + x as i16))
}

pub fn mouse_in(x: i16, y: i16) -> Option<(i16, i16)> {
    let sx = ADJX as i16;
    let sy = ADJY as i16;
    info!("mouse_in...1({}, {})", x, y);
    if x >= sx && y >= sy && x <= sx + (NCOL * CELLW) as i16 && y <= sy + (NROW * CELLH) as i16 {
        let s = Some(((x - sx) / CELLW as i16, (y - sy) / CELLH as i16));
        info!("mouse_in...2({:?})", s);
        return s;
    }
    None
}

pub fn get_neighbor_indices(row: i8, col: i8, x1: usize, x2: usize) -> Vec<(usize, usize)> {
    let offsets = [(-1, 0), (0, -1)];
    let mut r: Vec<(usize, usize)> = vec![];
    for (dy, dx) in offsets {
        let y = row + dy;
        let x = col + dx;
        if 0 <= y && y < NROW as i8 && x1 as i8 <= x && x < x2 as i8 {
            r.push((y as usize, x as usize));
        }
    }
    r
}

impl CityModel {
    pub fn new() -> Self {
        let grid: [[CityCell; NCOL]; NROW] = Default::default();
        let rand = Rand::new();
        let units = HashMap::new();
        let cell2unit = HashMap::new();
        Self {
            grid,
            rand,
            units,
            cell2unit,
            merge: CityMerge::new(),
            levelup: CityLevelup::new(),
            ready_del: -1,
            move_cells: vec![],
            ready2t: false,
        }
    }

    pub fn reset(&mut self) {
        self.rand.srand_now();

        //固定种子用于debug
        //self.rand.srand(0);

        //设置初始化矩阵
        //init matrix
        for i in 0..NROW {
            for j in 0..NCOL {
                self.grid[i][j] = CityCell::new(
                    (i * NCOL + j) as i16,
                    (self.rand.rand() as usize % (COLOR_COUNT - 2) + 1) as i8,
                    (self.rand.rand() % 2 + 1) as i16,
                );
            }
        }
        self.get_units_two_pass(0, NCOL);
        timer_register("merge", 0.1, "_");
        timer_register("levelup", 0.5, "_");
        timer_register("drop", 0.2, "_");
        event_emit("redraw_grid");
    }

    pub fn timer_process(&mut self, ctx: &mut Context) {
        if event_check("merge", "_") {
            ctx.state = CityState::LevelUpMovie as u8;
            let lc = self.post_merge();
            info!("POST_MERGE........");
            info!("{:?}\n {:?}\n {:?}\n {:?}\n", self.grid, self.units, self.cell2unit, self.move_cells);
            //lc有时候返回为0或1，导致set_time中timer.count未0
            //在timer_set_time中加了保护，让count至少为1
            timer_set_time("levelup", lc as f32 * LEVELUP_TIME);
            timer_fire("levelup", 0);
        }
        if event_check("levelup", "_") {
            ctx.state = CityState::DropMovie as u8;
            self.drop();
            timer_fire("drop", 0);
        }
        if event_check("drop", "_") {
            ctx.state = CityState::Normal as u8;
            self.get_units_two_pass(0, NCOL);
            event_emit("redraw_grid");
        }
    }

    #[allow(dead_code)]
    pub fn dump_grid(&self) {
        for i in 0..NROW {
            info!(
                "{} {} {} {} {}",
                self.grid[i][0], self.grid[i][1], self.grid[i][2], self.grid[i][3], self.grid[i][4]
            );
        }
    }

    pub fn get_units_two_pass(&mut self, x1: usize, x2: usize) {
        info!("begin get_units_2pass");
        //self.dump_grid();
        let mut label: Vec<u8> = vec![];
        for _ in 0..COLOR_COUNT {
            label.push(1u8);
        }

        let mut labels: [[[u8; COLOR_COUNT]; NCOL]; NROW] = Default::default();

        let mut ufs = vec![];
        for _i in 0..COLOR_COUNT {
            ufs.push(UnionFind::new(NCOL * NROW));
        }

        self.units.clear();
        self.cell2unit.clear();
        self.ready2t = false;

        //第一遍扫描，左边和上边如果有联通的块，
        //处理并记录并查集关系
        //1st scan, if left and upside have connected blocks
        //process and record disjoint-set data
        for i in 0..NROW {
            for j in x1..x2 {
                self.grid[i][j].from_id = None;
                self.grid[i][j].to_id = None;
                self.grid[i][j].ready2t = false;
                let color = self.grid[i][j].color as usize;
                if color == 0 || color > COLOR_COUNT {
                    continue;
                }
                let mut nlabels: Vec<Vec<u8>> = vec![];
                for _ in 0..COLOR_COUNT {
                    nlabels.push(vec![]);
                }
                for (y, x) in get_neighbor_indices(i as i8, j as i8, x1, x2) {
                    let nl = labels[y][x][color - 1];
                    if nl > 0 {
                        nlabels[color - 1].push(nl);
                    }
                }
                if nlabels[color - 1].len() > 0 {
                    let smallest = nlabels[color - 1].iter().min().unwrap();
                    labels[i][j][color - 1] = *smallest;
                    for l in &nlabels[color - 1] {
                        ufs[color - 1].union(*smallest as usize, *l as usize);
                    }
                } else {
                    labels[i][j][color - 1] = label[color - 1];
                    label[color - 1] += 1;
                }
            }
        }
        //第二遍扫描，生成units
        //2nd scan, create units
        for i in 0..NROW {
            for j in x1..x2 {
                for n in 0..COLOR_COUNT {
                    let mut l = labels[i][j][n] as usize;
                    l = ufs[n].find(l).unwrap();
                    if l != 0 {
                        let key = (n + 1) * 100 + l;
                        self.units
                            .entry(key)
                            .or_insert(CityUnit::new())
                            .add((i * NCOL + j) as i16);
                    }
                }
                //wonder
                if self.grid[i][j].color > COLOR_COUNT as i8 {
                    let cid = i * NCOL + j;
                    //用颜色+cid做键值，保证wonder独立
                    //use color+cid as the key, make sure each wonder is identifiable
                    let key = (self.grid[i][j].color as usize + 1) * 100 + cid;
                    self.units
                        .entry(key)
                        .or_insert(CityUnit::new())
                        .add(cid as i16);
                }
            }
        }
        //建立cellid到unit的map
        //maps cellid to unit
        for (key, val) in &mut self.units {
            let mut tl = 0i16;
            let mut allt = true;
            for id in &val.cells {
                self.cell2unit.insert(*id, *key);
                //统计块的类型用于绘制对应的边框
                //get block type to draw the border
                let (x, y) = get_xy(*id);
                let gl = self.grid[y as usize][x as usize].level;
                if gl != 30 {
                    allt = false;
                }
                tl += gl;
                let dd: [[i8; 2]; 4] = [[0, -1], [0, 1], [-1, 0], [1, 0]];
                let nd = [8u8, 4u8, 2u8, 1u8];
                let mut r = 0u8;
                for n in 0..4usize {
                    if check_neighbor(x as i8 + dd[n][0], y as i8 + dd[n][1], val, x1, x2) {
                        r += nd[n];
                    }
                }
                self.grid[y as usize][x as usize].border = r;
            }
            if (tl >= 30 && !allt) && val.cells.len() > 1 {
                self.ready2t = true;
                for c in &mut val.cells {
                    let (cx, cy) = get_xy(*c);
                    self.grid[cy as usize][cx as usize].ready2t = true;
                }
            }
        }
        //info!("{:#?}{:#?}", self.units, self.cell2unit);
    }

    pub fn merge_cell(&mut self, id: i16) -> bool {
        let unit_id = self.cell2unit.get(&id).unwrap();
        let u = self.units.get_mut(unit_id).unwrap();
        self.merge.object_id = id;
        self.merge.merge_cells.clear();
        if u.cells.len() == 1 {
            return false;
        }
        self.move_cells.clear();
        for cid in &u.cells {
            self.move_cells.push(*cid);
            if *cid == id {
                continue;
            }
            let (cx, cy) = get_xy(*cid);
            let cc = &mut self.grid[cy][cx];
            self.merge.merge_cells.push(*cid);
            cc.from_id = Some(*cid);
            cc.to_id = Some(id);
        }
        u.merging = true;
        if self.ready_del != -1 {
            let (rx, ry) = get_xy(self.ready_del);
            let rc = &mut self.grid[ry][rx];
            rc.color -= 100;
            self.ready_del = -1;
        }
        //info!("merge_cell {:?} {:?}", self.merge, self.move_cells);
        //self.dump_grid();
        true
    }

    pub fn post_merge(&mut self) -> i16 {
        let (x, y) = get_xy(self.merge.object_id);
        let g = &mut self.grid;
        let is_base = g[y][x].level < 30;
        self.levelup.cellid = self.merge.object_id;
        self.levelup.from = g[y][x].level;
        for cid in &self.merge.merge_cells {
            let (cx, cy) = get_xy(*cid);
            g[y][x].level += g[cy][cx].level;
            g[cy][cx].color = -1;
            g[cy][cx].from_id = None;
            g[cy][cx].to_id = None;
        }
        if g[y][x].level >= 30 {
            if is_base {
                //新合并成T
                //merge to T
                g[y][x].level = 30;
                g[y][x].color = COLOR_COUNT as i8;
                self.levelup.to = g[y][x].level;
                return (g[y][x].level - self.levelup.from) / 3;
            } else {
                //T合并
                //T merges
                g[y][x].color = COLOR_COUNT as i8 + (g[y][x].level / 30) as i8 - 1;
                self.levelup.to = g[y][x].level;
                return g[y][x].level / 30 - 1;
            }
        } else {
            //未成T
            //not T yet
            self.levelup.to = g[y][x].level;
            return g[y][x].level - self.levelup.from;
        }
    }

    pub fn drop(&mut self) {
        self.move_cells.clear();
        let g = &mut self.grid;
        for x in 0..NCOL {
            let mut holes: Vec<i16> = vec![];
            for y in 0..NROW {
                if g[y][x].color == -1 {
                    holes.push((y * NCOL + x) as i16);
                }
            }
            if holes.len() == 0 {
                continue;
            }
            let mut tmpcs: Vec<CityCell> = vec![];
            for _i in 0..NROW {
                tmpcs.push(CityCell::new(0, 0, 0));
            }
            //处理非空洞块
            //process non-empty block
            let mut no_hole = 0usize;
            for y in 0..NROW {
                if g[y][x].color == -1 {
                    continue;
                }
                let mut dropcnt = 0usize;
                for n in y + 1..NROW {
                    if g[n][x].color == -1 {
                        dropcnt += 1;
                    }
                }
                g[y][x].from_id = Some(g[y][x].id);
                g[y][x].to_id = Some(g[y][x].id + (dropcnt * NCOL) as i16);
                if dropcnt != 0 {
                    self.move_cells.push(g[y][x].to_id.unwrap());
                }
                g[y][x].assign(&mut tmpcs[holes.len() + no_hole]);
                tmpcs[holes.len() + no_hole].id = g[y][x].to_id.unwrap();
                no_hole += 1;
            }
            //处理空洞
            //process empty block
            for i in 0..holes.len() {
                let h = &mut tmpcs[i];
                h.color = (self.rand.rand() as usize % (COLOR_COUNT - 1) + 1) as i8;
                h.level = (self.rand.rand() % 2 + 1) as i16;
                h.from_id = Some((i as i16 - holes.len() as i16) * NCOL as i16 + x as i16);
                h.to_id = Some((i * NCOL + x) as i16);
                h.id = h.to_id.unwrap();
                self.move_cells.push(h.id);
            }
            for i in 0..NROW {
                tmpcs[i].assign(&mut g[i][x]);
            }
        }
        info!("after drop");
        //self.dump_grid();
    }

    pub fn del_cell(&mut self, id: i16) -> bool {
        let (x, y) = get_xy(id);
        let c = &mut self.grid[y][x];
        if c.color < 100 {
            c.color += 100;
            if self.ready_del != id {
                if self.ready_del != -1 {
                    let (rx, ry) = get_xy(self.ready_del);
                    self.grid[ry][rx].color -= 100;
                }
                self.ready_del = id;
            }
            return false;
        } else {
            c.color = -1;
            self.ready_del = -1;
            self.drop();
            return true;
        }
    }

    pub fn act(&mut self, row: i16, col: i16, ctx: &mut Context) {
        let cid = row * NCOL as i16 + col;
        if self.merge_cell(cid) {
            event_emit("redraw_grid");
            timer_fire("merge", 0);
            ctx.state = CityState::MergeMovie as u8;
        } else {
            if !self.del_cell(cid) {
                event_emit("redraw_grid");
            } else {
                timer_fire("drop", 0);
                ctx.state = CityState::DropMovie as u8;
            }
        }
    }
}

impl Model for CityModel {
    fn init(&mut self, _ctx: &mut Context) {
        self.reset();
    }

    fn handle_input(&mut self, ctx: &mut Context, _dt: f32) {
        let es = ctx.input_events.clone();
        for e in &es {
            match e {
                Event::Mouse(mou) => {
                    if mou.kind == Up(MouseButton::Left) {
                        match mouse_in(mou.column as i16, mou.row as i16) {
                            Some((c, r)) => {
                                if ctx.state == (CityState::Normal as u8) {
                                    self.act(r, c, ctx);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        ctx.input_events.clear();
    }

    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_timer(&mut self, ctx: &mut Context, _dt: f32) {
        self.timer_process(ctx);
    }
}
