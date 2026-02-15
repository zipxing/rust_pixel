#![allow(non_camel_case_types)]
use block_arrow_lib::{generate_level, builtin_levels, evaluate_difficulty, Board, Direction};
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode, MouseButton, MouseEventKind},
    game::Model,
};

// Terminal cell size: 10 chars wide × 5 chars tall (same as colorblk)
pub const CELLW: usize = 10;
pub const CELLH: usize = 5;

// Panel size (fits 9×9 board: 9*10+2=92 wide, 9*5+2+2=49 tall)
pub const BLOCK_ARROWW: u16 = 92;
pub const BLOCK_ARROWH: u16 = 49;

const FLASH_DURATION: u8 = 12;
const FLY_ANIM_FRAMES: u8 = 10;

#[repr(u8)]
pub enum GameState {
    Playing,
    Won,      // fly animation may still be running
    Showcase, // all done, show pixel art
}

/// One cell of the fly-away animation
pub struct FlyAnimCell {
    pub sx: u16,
    pub sy: u16,
    pub border_type: u8,
    pub color: i8,
    pub arrow_str: String,
}

/// Fly-away animation state
pub struct FlyAnim {
    pub cells: Vec<FlyAnimCell>,
    pub direction: Direction,
    pub frame: u8,
}

pub struct Block_arrowModel {
    pub board: Board,
    pub game_state: GameState,
    pub level_index: usize,
    pub render_state: Vec<(u8, i8)>,
    pub board_width: usize,
    pub board_height: usize,
    pub message: String,
    pub flash_block: Option<usize>,
    pub flash_timer: u8,
    pub fly_anim: Option<FlyAnim>,
    pub bitmap: Vec<Vec<u8>>, // original pixel art, preserved for showcase
    pub difficulty_score: f32,
}

impl Block_arrowModel {
    pub fn new() -> Self {
        let levels = builtin_levels();
        let bitmap = &levels[1];
        let level = generate_level(bitmap).expect("Failed to generate level 1");
        let w = level.width;
        let h = level.height;
        let saved_bitmap = level.bitmap.clone();
        let diff = evaluate_difficulty(&level);
        let board = Board::from_level(&level);

        let mut m = Self {
            board,
            game_state: GameState::Playing,
            level_index: 1,
            render_state: vec![(0, -1); w * h],
            board_width: w,
            board_height: h,
            message: String::new(),
            flash_block: None,
            flash_timer: 0,
            fly_anim: None,
            bitmap: saved_bitmap,
            difficulty_score: diff.score,
        };
        m.update_render_state();
        m
    }

    pub fn load_level(&mut self, index: usize) {
        let levels = builtin_levels();
        if index >= levels.len() {
            return;
        }
        let bitmap = &levels[index];
        if let Some(level) = generate_level(bitmap) {
            let w = level.width;
            let h = level.height;
            self.bitmap = level.bitmap.clone();
            self.difficulty_score = evaluate_difficulty(&level).score;
            self.board = Board::from_level(&level);
            self.board_width = w;
            self.board_height = h;
            self.game_state = GameState::Playing;
            self.level_index = index;
            self.render_state = vec![(0, -1); w * h];
            self.message = String::new();
            self.flash_block = None;
            self.flash_timer = 0;
            self.fly_anim = None;
            self.update_render_state();
        }
    }

    fn screen_to_board(&self, col: u16, row: u16) -> Option<(usize, usize)> {
        if col < 1 || row < 1 {
            return None;
        }
        let bx = (col as usize - 1) / CELLW;
        let by = (row as usize - 1) / CELLH;
        if bx < self.board_width && by < self.board_height {
            Some((bx, by))
        } else {
            None
        }
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        if self.fly_anim.is_some() {
            return;
        }

        if let Some((bx, by)) = self.screen_to_board(col, row) {
            if let Some(bid) = self.board.block_at(bx, by) {
                let block = &self.board.blocks[bid];
                let direction = block.arrow;
                let first_cell = block.cells[0];

                let anim_cells: Vec<FlyAnimCell> = block
                    .cells
                    .iter()
                    .map(|&(cx, cy)| {
                        let border = self.board.border_type(cx, cy, bid);
                        let color = block.color as i8;
                        let arrow_str = if (cx, cy) == first_cell {
                            direction.arrow_char().to_string()
                        } else {
                            String::new()
                        };
                        FlyAnimCell {
                            sx: (cx * CELLW) as u16 + 1,
                            sy: (cy * CELLH) as u16 + 1,
                            border_type: border,
                            color,
                            arrow_str,
                        }
                    })
                    .collect();

                if self.board.try_fly(bid) {
                    self.message = format!("Block {} flew away!", bid);
                    self.flash_block = None;
                    self.flash_timer = 0;
                    self.update_render_state();
                    self.fly_anim = Some(FlyAnim {
                        cells: anim_cells,
                        direction,
                        frame: 0,
                    });
                    if self.board.all_removed() {
                        self.game_state = GameState::Won;
                        self.message = "YOU WIN!".to_string();
                    }
                } else {
                    self.message = "Blocked! Can't fly.".to_string();
                    self.flash_block = Some(bid);
                    self.flash_timer = FLASH_DURATION;
                }
                event_emit("BlockArrow.Redraw");
            }
        }
    }

    pub fn update_render_state(&mut self) {
        let w = self.board_width;
        let h = self.board_height;
        self.render_state.clear();
        self.render_state.resize(w * h, (0, -1));

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                if let Some(block_id) = self.board.block_at(x, y) {
                    let border = self.board.border_type(x, y, block_id);
                    let color = self.board.blocks[block_id].color as i8;
                    self.render_state[idx] = (border, color);
                } else {
                    self.render_state[idx] = (0, -1);
                }
            }
        }
    }
}

impl Model for Block_arrowModel {
    fn init(&mut self, _context: &mut Context) {
        event_emit("BlockArrow.Redraw");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Mouse(me) => {
                    if let MouseEventKind::Down(MouseButton::Left) = me.kind {
                        match self.game_state {
                            GameState::Playing => {
                                self.handle_click(me.column, me.row);
                            }
                            GameState::Won | GameState::Showcase => {
                                if self.fly_anim.is_none() {
                                    let next =
                                        (self.level_index + 1) % builtin_levels().len();
                                    self.load_level(next);
                                    event_emit("BlockArrow.Redraw");
                                }
                            }
                        }
                    }
                }
                Event::Key(key) => match key.code {
                    KeyCode::Char('r') => {
                        self.load_level(self.level_index);
                        event_emit("BlockArrow.Redraw");
                    }
                    KeyCode::Char('n') => {
                        if !matches!(self.game_state, GameState::Playing) && self.fly_anim.is_none()
                        {
                            let next = (self.level_index + 1) % builtin_levels().len();
                            self.load_level(next);
                            event_emit("BlockArrow.Redraw");
                        }
                    }
                    _ => {}
                },
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {
        let mut need_redraw = false;

        // Flash countdown
        if self.flash_timer > 0 {
            self.flash_timer -= 1;
            if self.flash_timer == 0 {
                self.flash_block = None;
            }
            need_redraw = true;
        }

        // Fly animation
        if let Some(ref mut anim) = self.fly_anim {
            anim.frame += 1;
            if anim.frame >= FLY_ANIM_FRAMES {
                self.fly_anim = None;
                // Transition Won → Showcase after last fly animation ends
                if matches!(self.game_state, GameState::Won) {
                    self.game_state = GameState::Showcase;
                    self.message = "Click or press N for next level".to_string();
                }
            }
            need_redraw = true;
        }

        if need_redraw {
            event_emit("BlockArrow.Redraw");
        }
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
