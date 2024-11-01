use rust_pixel::event::{Event, KeyCode, MouseButton, MouseEventKind::*};
//use log::info;
#[cfg(feature = "sdl")]
use crate::render::{SYMBOL_SDL, SYMBOL_SDL_LOW};
use rust_pixel::{context::Context, event::event_emit, game::Model};

pub const COLORW: u16 = 18;
pub const COLORH: u16 = 15;
pub const SYMW: u16 = 18;
pub const SYMH: u16 = 18;
#[cfg(not(feature = "sdl"))]
pub const EDITW: u16 = 80;
#[cfg(feature = "sdl")]
pub const EDITW: u16 = 48;
pub const EDITH: u16 = 35;

//画笔类型
#[derive(PartialEq)]
pub enum TeditPen {
    SYMBOL(u16),
    BACK(u16),
    FORE(u16),
}

//标记区域
pub enum TeditArea {
    ButtonNextSym,
    ButtonNextColor,
    ButtonSave,
    COLOR(u16),
    SYMBOL(u16),
    EDIT(u16),
}

pub struct TeditModel {
    pub curpen: TeditPen,
    pub curx: u16,
    pub cury: u16,
    pub sym_tab_idx: u8,
    pub sym_tab_count: u8,
    pub color_tab_idx: u8,
}

impl TeditModel {
    pub fn new() -> Self {
        #[cfg(not(feature = "sdl"))]
        let stc = 3;
        #[cfg(feature = "sdl")]
        let stc = 4u8;

        Self {
            curpen: TeditPen::SYMBOL(0),
            curx: 0,
            cury: 0,
            sym_tab_idx: 0,
            sym_tab_count: stc,
            color_tab_idx: 0,
        }
    }

    pub fn mouse_in(&self, x: u16, y: u16) -> Option<TeditArea> {
        if x >= 13 && x <= COLORW && y >= SYMH + COLORH + 1 && y <= SYMH + COLORH + 2 {
            return Some(TeditArea::ButtonNextColor);
        }
        if x >= 1 && x <= SYMW && y >= SYMH - 1 && y <= SYMH {
            return Some(TeditArea::ButtonNextSym);
        }
        if x >= SYMW + EDITW - 3 && y == EDITH + 2 {
            return Some(TeditArea::ButtonSave);
        }
        if x >= 1 && x <= SYMW && y >= 1 && y <= SYMH - 2 {
            let idx = (y - 1) * SYMW + x - 1;
            return Some(TeditArea::SYMBOL(idx));
        }
        if x >= 1 && x <= COLORW && y >= SYMH + 3 && y <= SYMH + 2 + COLORH {
            let idx = (y - SYMH - 3) * COLORW + x - 1;
            return Some(TeditArea::COLOR(idx));
        }
        if x >= SYMW + 3 && x <= SYMW + 2 + EDITW && y >= 1 && y <= EDITH {
            let idx = (y - 1) * EDITW + x - SYMW - 3;
            return Some(TeditArea::EDIT(idx));
        }
        None
    }
}

impl Model for TeditModel {
    fn init(&mut self, _context: &mut Context) {
        event_emit("Tedit.RedrawPen");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('s') {
                        event_emit("Tedit.Save");
                    }
                }
                Event::Mouse(mou) => {
                    //info!("{:?}", mou);
                    match self.mouse_in(mou.column, mou.row) {
                        Some(TeditArea::COLOR(idx)) => {
                            if mou.kind == Up(MouseButton::Left) {
                                if self.color_tab_idx == 0 {
                                    self.curpen = TeditPen::FORE(idx);
                                } else {
                                    self.curpen = TeditPen::BACK(idx);
                                }
                                event_emit("Tedit.RedrawPen");
                            }
                        }
                        Some(TeditArea::SYMBOL(idx)) => {
                            if mou.kind == Up(MouseButton::Left) {
                                #[cfg(not(feature = "sdl"))]
                                {
                                    self.curpen = TeditPen::SYMBOL(idx);
                                }
                                #[cfg(feature = "sdl")]
                                {
                                    let sym;
                                    if self.sym_tab_idx == 0 {
                                        sym = SYMBOL_SDL_LOW[idx as usize] as u16;
                                    } else if self.sym_tab_idx == 1 {
                                        sym = SYMBOL_SDL[idx as usize] as u16;
                                    } else {
                                        let i = idx / SYMW;
                                        let j = idx % SYMW;
                                        if j == 0 || j == 17 {
                                            sym = 32;
                                        } else {
                                            sym = i * (SYMW - 2) + j - 1;
                                        }
                                    }
                                    self.curpen = TeditPen::SYMBOL(sym);
                                }
                                event_emit("Tedit.RedrawPen");
                            }
                        }
                        Some(TeditArea::EDIT(idx)) => {
                            if mou.kind == Up(MouseButton::Left)
                                || mou.kind == Drag(MouseButton::Left)
                                || mou.kind == Down(MouseButton::Left)
                            {
                                self.curx = idx % EDITW;
                                self.cury = idx / EDITW;
                                event_emit("Tedit.RedrawEdit");
                                event_emit("Tedit.RedrawPen");
                            }
                        }
                        Some(TeditArea::ButtonNextSym) => {
                            if mou.kind == Up(MouseButton::Left) {
                                self.sym_tab_idx = (self.sym_tab_idx + 1) % self.sym_tab_count;
                                event_emit("Tedit.RedrawPen");
                            }
                        }
                        Some(TeditArea::ButtonNextColor) =>
                        {
                            #[cfg(not(feature = "sdl"))]
                            if mou.kind == Up(MouseButton::Left) {
                                if self.color_tab_idx == 0 {
                                    self.color_tab_idx = 1;
                                } else {
                                    self.color_tab_idx = 0;
                                }
                                match self.curpen {
                                    TeditPen::FORE(idx) | TeditPen::BACK(idx) => {
                                        if self.color_tab_idx == 0 {
                                            self.curpen = TeditPen::FORE(idx);
                                        } else {
                                            self.curpen = TeditPen::BACK(idx);
                                        }
                                    }
                                    _ => {}
                                }
                                event_emit("Tedit.RedrawPen");
                            }
                        }
                        Some(TeditArea::ButtonSave) => {
                            if mou.kind == Up(MouseButton::Left) {
                                event_emit("Tedit.Save");
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
