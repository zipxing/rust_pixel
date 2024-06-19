// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! Implements an Adapter trait. Moreover,
//! all SDL related processing is handled here.
//! Includes resizing of height and width, init settings,
//! some code is also called in cell.rs
use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
use crate::{
    render::{
        adapter::{
            render_border, render_logo, render_main_buffer, render_pixel_sprites, ARect, Adapter,
            AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILES,
        },
        buffer::Buffer,
        sprite::Sprites,
    },
    util::Rand,
    LOGO_FRAME,
};
use sdl2::{
    event::Event as SEvent,
    image::{InitFlag, LoadSurface, LoadTexture},
    keyboard::Keycode as SKeycode,
    mouse::*,
    pixels::PixelFormatEnum,
    rect::{Point as SPoint, Rect as SRect},
    render::{Canvas, Texture},
    surface::Surface,
    video::{Window, WindowPos::Positioned},
    EventPump, Sdl,
};
use std::any::Any;
use std::time::Duration;
use log::info;

pub struct SdlAdapter {
    // 拖动窗口需要的数据
    // data for dragging the window
    pub drag_need: bool,
    pub drag_ing: bool,
    pub drag_mouse_x: i32,
    pub drag_mouse_y: i32,
    pub drag_dx: i32,
    pub drag_dy: i32,
    pub base: AdapterBase,
    // sdl object
    pub context: Sdl,
    pub event_pump: Option<EventPump>,
    // custom cursor in rust-sdl2
    pub cursor: Option<Cursor>,
    pub canvas: Option<Canvas<Window>>,
    // raw textures
    pub asset_textures: Option<Vec<Texture>>,
    // rendering target textures
    pub render_texture: Option<Texture>,
    pub rd: Rand,
}

pub enum SdlBorderArea {
    NOPE,
    CLOSE,
    TOPBAR,
    OTHER,
}

impl SdlAdapter {
    pub fn new(gn: &str) -> Self {
        Self {
            context: sdl2::init().unwrap(),
            event_pump: None,
            base: AdapterBase::new(gn),
            cursor: None,
            canvas: None,
            asset_textures: None,
            render_texture: None,
            drag_ing: false,
            drag_mouse_x: 0,
            drag_mouse_y: 0,
            drag_need: false,
            drag_dx: 0,
            drag_dy: 0,
            rd: Rand::new(),
        }
    }

    fn set_mouse_cursor(&mut self, s: &Surface) {
        self.cursor = Some(
            Cursor::from_surface(s, 0, 0)
                .map_err(|err| format!("failed to load cursor: {}", err))
                .unwrap(),
        );
        match &self.cursor {
            Some(cursor) => {
                cursor.set();
            }
            _ => {}
        }
        self.context.mouse().show_cursor(true);
    }

    fn in_border(&self, x: i32, y: i32) -> SdlBorderArea {
        let w = self.cell_width();
        let h = self.cell_height();
        let sw = self.base.cell_w + 2;
        if y >= 0 && y < h as i32 {
            if x >= 0 && x <= ((sw - 1) as f32 * w) as i32 {
                return SdlBorderArea::TOPBAR;
            }
            if x > ((sw - 1) as f32 * w) as i32 && x <= (sw as f32 * w) as i32 {
                return SdlBorderArea::CLOSE;
            }
        } else {
            if x > w as i32 && x <= ((sw - 1) as f32 * w) as i32 {
                return SdlBorderArea::NOPE;
            }
        }
        SdlBorderArea::OTHER
    }

    fn drag_window(&mut self, event: &SEvent) -> bool {
        match *event {
            SEvent::Quit { .. }
            | SEvent::KeyDown {
                keycode: Some(SKeycode::Q),
                ..
            } => return true,

            SEvent::MouseButtonDown {
                mouse_btn: sdl2::mouse::MouseButton::Left,
                x,
                y,
                ..
            } => {
                let bs = self.in_border(x, y);
                match bs {
                    SdlBorderArea::TOPBAR | SdlBorderArea::OTHER => {
                        // start dragging when mouse left click
                        self.drag_ing = true;
                        self.drag_mouse_x = x;
                        self.drag_mouse_y = y;
                    }
                    SdlBorderArea::CLOSE => {
                        return true;
                    }
                    _ => {}
                }
            }
            SEvent::MouseButtonUp {
                mouse_btn: sdl2::mouse::MouseButton::Left,
                ..
            } => {
                // stop dragging when mouse left button is release
                self.drag_ing = false;
            }
            SEvent::MouseMotion { x, y, .. } if self.drag_ing => {
                self.drag_need = true;
                // dragging window when mouse left button is hold and moving
                self.drag_dx = x - self.drag_mouse_x;
                self.drag_dy = y - self.drag_mouse_y;
            }
            _ => {}
        }
        false
    }

    /// dynamic update of sym dot matrix
    /// the length of pdat should be 16 * 16 * 4(RGBA)
    pub fn update_cell_texture(&mut self, tex_idx: u8, sym_idx: u8, pdat: &[u8]) {
        match &mut self.asset_textures {
            Some(st) => {
                let w = PIXEL_SYM_WIDTH as i32;
                let h = PIXEL_SYM_HEIGHT as i32;
                let srcx = sym_idx as i32 % w;
                let srcy = sym_idx as i32 / w;
                let sr = SRect::new((w + 1) * srcx, (h + 1) * srcy, w as u32, h as u32);
                st[tex_idx as usize]
                    .update(sr, pdat, 4 * PIXEL_SYM_WIDTH as usize)
                    .unwrap();
            }
            _ => {}
        }
    }
}

impl Adapter for SdlAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String) {
        self.set_size(w, h)
            .set_ratiox(rx)
            .set_ratioy(ry)
            .set_pixel_size()
            .set_title(s);

        let video_subsystem = self.context.video().unwrap();
        let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG).unwrap();

        let window = video_subsystem
            .window(&self.base.title, self.base.pixel_w, self.base.pixel_h)
            .position_centered()
            .borderless()
            // .fullscreen()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let canvas = window
            .into_canvas()
            .software()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let texture_creator = canvas.texture_creator();
        let mut vt: Vec<Texture> = vec![];
        for t in 0..PIXEL_TEXTURE_FILES.len() {
            vt.push(
                texture_creator
                    .load_texture(format!(
                        "games{}{}{}{}",
                        std::path::MAIN_SEPARATOR,
                        self.base.game_name,
                        std::path::MAIN_SEPARATOR,
                        PIXEL_TEXTURE_FILES[t]
                    ))
                    .unwrap(),
            );
        }
        let rt = texture_creator
            .create_texture_target(
                PixelFormatEnum::RGBA8888,
                self.base.pixel_w,
                self.base.pixel_h,
            )
            .unwrap();

        let surface = Surface::from_file(format!(
                        "games{}{}{}{}",
                        std::path::MAIN_SEPARATOR,
                        self.base.game_name,
                        std::path::MAIN_SEPARATOR,
                        "assets/pix/cursor.png"))
            .map_err(|err| format!("failed to load cursor image: {}", err))
            .unwrap();
        self.set_mouse_cursor(&surface);

        self.canvas = Some(canvas);
        self.asset_textures = Some(vt);
        self.render_texture = Some(rt);
        self.event_pump = Some(self.context.event_pump().unwrap());
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH / self.base.ratio_x
    }

    fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT / self.base.ratio_y
    }

    fn poll_event(&mut self, timeout: Duration, es: &mut Vec<Event>) -> bool {
        let mut ses: Vec<SEvent> = vec![];
        match &mut self.event_pump {
            Some(ref mut ep) => {
                for event in ep.poll_iter() {
                    ses.push(event.clone());
                    // convert sdl events to pixel events, providing a unified processing interfaces
                    if let Some(et) =
                        input_events_from_sdl(&event, self.base.ratio_x, self.base.ratio_y)
                    {
                        if !self.drag_ing {
                            es.push(et);
                        }
                    }
                }
                for event in ses {
                    // sdl window is borderless, we draw the title and border ourselves
                    // processing mouse events such as dragging of borders, close, etc.
                    if self.drag_window(&event) {
                        return true;
                    }
                }
                ::std::thread::sleep(timeout);
            }
            _ => {}
        }
        false
    }

    fn render_buffer(
        &mut self,
        current_buffer: &Buffer,
        _p: &Buffer,
        pixel_sprites: &mut Sprites,
        stage: u32,
    ) -> Result<(), String> {
        let width = current_buffer.area.width;

        if let (Some(c), Some(rt), Some(texs)) = (
            &mut self.canvas,
            &mut self.render_texture,
            &mut self.asset_textures,
        ) {
            // dragging window, set the correct position of a window
            sdl_move_win(&mut self.drag_need, c, self.drag_dx, self.drag_dy);
            c.clear();
            c.with_texture_canvas(rt, |tc| {
                tc.clear();

                if stage <= LOGO_FRAME {
                    render_logo(
                        self.base.ratio_x,
                        self.base.ratio_y,
                        self.base.pixel_w,
                        self.base.pixel_h,
                        &mut self.rd,
                        stage,
                        |fc, ss1, ss2, texidx, _symidx| {
                            let s1 = SRect::new(ss1.x, ss1.y, ss1.w, ss1.h);
                            let s2 = SRect::new(ss2.x, ss2.y, ss2.w, ss2.h);
                            let tx = &mut texs[texidx / 4];
                            tx.set_color_mod(fc.0, fc.1, fc.2);
                            tx.set_alpha_mod(fc.3);
                            tc.copy(tx, s1, s2).unwrap();
                        },
                    );
                }

                let rx = self.base.ratio_x;
                let ry = self.base.ratio_y;
                let mut rfunc =
                    |fc: &(u8, u8, u8, u8), s1: ARect, s2: ARect, texidx: usize, _symidx: usize| {
                        let tx = &mut texs[texidx / 4];
                        let ss1 = SRect::new(s1.x, s1.y, s1.w, s1.h);
                        let ss2 = SRect::new(s2.x, s2.y, s2.w, s2.h);
                        tx.set_color_mod(fc.0, fc.1, fc.2);
                        tx.set_alpha_mod(fc.3);
                        tc.copy(tx, ss1, ss2).unwrap();
                    };
                if stage > LOGO_FRAME {
                    render_border(self.base.cell_w, self.base.cell_h, rx, ry, &mut rfunc);
                    render_main_buffer(current_buffer, width, rx, ry, &mut rfunc);
                    render_pixel_sprites(
                        pixel_sprites,
                        rx,
                        ry,
                        |fc, s1, s2, texidx, _symidx, angle, ccp| {
                            let tx = &mut texs[texidx / 4];
                            let ss1 = SRect::new(s1.x, s1.y, s1.w, s1.h);
                            let ss2 = SRect::new(s2.x, s2.y, s2.w, s2.h);
                            let cccp = SPoint::new(ccp.x, ccp.y);
                            tx.set_color_mod(fc.0, fc.1, fc.2);
                            tx.set_alpha_mod(fc.3);
                            tc.copy_ex(tx, ss1, ss2, angle, cccp, false, false).unwrap();
                        },
                    );
                }
            })
            .unwrap();
            c.copy(rt, None, None).unwrap();
            c.present();
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok((0, 0))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn sdl_move_win(drag_need: &mut bool, c: &mut Canvas<Window>, dx: i32, dy: i32) {
    // dragging window, set the correct position of a window
    if *drag_need {
        let win = c.window_mut();
        let (win_x, win_y) = win.position();
        win.set_position(Positioned(win_x + dx), Positioned(win_y + dy));
        *drag_need = false;
    }
}

// avoid repeated code by defining a marco
macro_rules! sdl_event {
    ($ek:expr, $x:expr, $y:expr, $($btn:expr)* ) => {
        Some(MouseEvent {
            kind: $ek$(($btn))*,
            column: $x as u16,
            row: $y as u16,
            modifiers: KeyModifiers::NONE,
        })
    };
}

/// Convert sdl input events to RustPixel event, for the sake of unified event processing
/// For keyboard and mouse event, please refer to the handle_input method in game/unblock/model.rs
pub fn input_events_from_sdl(e: &SEvent, adjx: f32, adjy: f32) -> Option<Event> {
    let sym_width = PIXEL_SYM_WIDTH as f32;
    let sym_height = PIXEL_SYM_HEIGHT as f32;
    let mut mcte: Option<MouseEvent> = None;
    match e {
        SEvent::KeyDown { keycode, .. } => {
            let kc;
            match keycode {
                Some(SKeycode::Space) => kc = ' ',
                Some(SKeycode::A) => kc = 'a',
                Some(SKeycode::B) => kc = 'b',
                Some(SKeycode::C) => kc = 'c',
                Some(SKeycode::D) => kc = 'd',
                Some(SKeycode::E) => kc = 'e',
                Some(SKeycode::F) => kc = 'f',
                Some(SKeycode::G) => kc = 'g',
                Some(SKeycode::H) => kc = 'h',
                Some(SKeycode::I) => kc = 'i',
                Some(SKeycode::J) => kc = 'j',
                Some(SKeycode::K) => kc = 'k',
                Some(SKeycode::L) => kc = 'l',
                Some(SKeycode::M) => kc = 'm',
                Some(SKeycode::N) => kc = 'n',
                Some(SKeycode::O) => kc = 'o',
                Some(SKeycode::P) => kc = 'p',
                Some(SKeycode::Q) => kc = 'q',
                Some(SKeycode::R) => kc = 'r',
                Some(SKeycode::S) => kc = 's',
                Some(SKeycode::T) => kc = 't',
                Some(SKeycode::U) => kc = 'u',
                Some(SKeycode::V) => kc = 'v',
                Some(SKeycode::W) => kc = 'w',
                Some(SKeycode::X) => kc = 'x',
                Some(SKeycode::Y) => kc = 'y',
                Some(SKeycode::Z) => kc = 'z',
                _ => {
                    return None;
                }
            }
            let cte = KeyEvent::new(KeyCode::Char(kc), KeyModifiers::NONE);
            return Some(Event::Key(cte));
        }
        SEvent::MouseButtonUp { x, y, .. } => {
            mcte = sdl_event!(Up, *x, *y, Left);
        }
        SEvent::MouseButtonDown { x, y, .. } => {
            mcte = sdl_event!(Down, *x, *y, Left);
        }
        SEvent::MouseMotion {
            x, y, mousestate, ..
        } => {
            if mousestate.left() {
                mcte = sdl_event!(Drag, *x, *y, Left);
            } else {
                mcte = sdl_event!(Moved, *x, *y,);
            }
        }
        _ => {}
    }
    if let Some(mut mc) = mcte {
        mc.column /= (sym_width / adjx) as u16;
        mc.row /= (sym_height / adjy) as u16;
        if mc.column >= 1 {
            mc.column -= 1;
        }
        if mc.row >= 1 {
            mc.row -= 1;
        }
        return Some(Event::Mouse(mc));
    }
    None
}
