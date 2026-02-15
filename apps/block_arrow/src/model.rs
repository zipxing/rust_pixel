#![allow(non_camel_case_types)]
use block_arrow_lib::{generate_level, builtin_levels, Board};
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
};

// Terminal cell size: 10 chars wide × 5 chars tall (same as colorblk)
pub const CELLW: usize = 10;
pub const CELLH: usize = 5;

// Panel size (fits 9×9 board: 9*10+2=92 wide, 9*5+2+2=49 tall)
pub const BLOCK_ARROWW: u16 = 92;
pub const BLOCK_ARROWH: u16 = 49;

#[repr(u8)]
pub enum GameState {
    Playing,
    Won,
}

pub struct Block_arrowModel {
    pub board: Board,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub selected_block: Option<usize>,
    pub game_state: GameState,
    pub level_index: usize,
    // Render state: (border_type, color) per grid cell, -1 color = empty
    pub render_state: Vec<(u8, i8)>,
    pub board_width: usize,
    pub board_height: usize,
    pub message: String,
}

impl Block_arrowModel {
    pub fn new() -> Self {
        let levels = builtin_levels();
        let bitmap = &levels[1];
        let level = generate_level(bitmap).expect("Failed to generate level 1");
        let w = level.width;
        let h = level.height;
        let board = Board::from_level(&level);

        let mut m = Self {
            board,
            cursor_x: w / 2,
            cursor_y: h / 2,
            selected_block: None,
            game_state: GameState::Playing,
            level_index: 1,
            render_state: vec![(0, -1); w * h],
            board_width: w,
            board_height: h,
            message: String::new(),
        };
        m.update_cursor_selection();
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
            self.board = Board::from_level(&level);
            self.board_width = w;
            self.board_height = h;
            self.cursor_x = w / 2;
            self.cursor_y = h / 2;
            self.selected_block = None;
            self.game_state = GameState::Playing;
            self.level_index = index;
            self.render_state = vec![(0, -1); w * h];
            self.message = String::new();
            self.update_cursor_selection();
            self.update_render_state();
        }
    }

    fn update_cursor_selection(&mut self) {
        self.selected_block = self.board.block_at(self.cursor_x, self.cursor_y);
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
                Event::Key(key) => {
                    match self.game_state {
                        GameState::Playing => match key.code {
                            KeyCode::Up => {
                                if self.cursor_y > 0 {
                                    self.cursor_y -= 1;
                                    self.update_cursor_selection();
                                    event_emit("BlockArrow.Redraw");
                                }
                            }
                            KeyCode::Down => {
                                if self.cursor_y + 1 < self.board_height {
                                    self.cursor_y += 1;
                                    self.update_cursor_selection();
                                    event_emit("BlockArrow.Redraw");
                                }
                            }
                            KeyCode::Left => {
                                if self.cursor_x > 0 {
                                    self.cursor_x -= 1;
                                    self.update_cursor_selection();
                                    event_emit("BlockArrow.Redraw");
                                }
                            }
                            KeyCode::Right => {
                                if self.cursor_x + 1 < self.board_width {
                                    self.cursor_x += 1;
                                    self.update_cursor_selection();
                                    event_emit("BlockArrow.Redraw");
                                }
                            }
                            KeyCode::Char(' ') | KeyCode::Enter => {
                                if let Some(bid) = self.selected_block {
                                    if self.board.try_fly(bid) {
                                        self.message = format!("Block {} flew away!", bid);
                                        self.update_render_state();
                                        self.update_cursor_selection();
                                        if self.board.all_removed() {
                                            self.game_state = GameState::Won;
                                            self.message =
                                                "YOU WIN! Press N for next level".to_string();
                                        }
                                    } else {
                                        self.message = "Blocked! Can't fly.".to_string();
                                    }
                                    event_emit("BlockArrow.Redraw");
                                }
                            }
                            KeyCode::Char('r') => {
                                self.load_level(self.level_index);
                                event_emit("BlockArrow.Redraw");
                            }
                            _ => {}
                        },
                        GameState::Won => match key.code {
                            KeyCode::Char('n') => {
                                let next = (self.level_index + 1) % builtin_levels().len();
                                self.load_level(next);
                                event_emit("BlockArrow.Redraw");
                            }
                            KeyCode::Char('r') => {
                                self.load_level(self.level_index);
                                event_emit("BlockArrow.Redraw");
                            }
                            _ => {}
                        },
                    }
                }
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
