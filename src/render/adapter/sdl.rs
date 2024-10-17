// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements an Adapter trait. Moreover, all SDL related processing is handled here.
//! Includes resizing of height and width, init settings.
//! Use opengl and glow mod for rendering.
use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
use crate::render::{
    adapter::{
        gl::pixel::GlPixel, Adapter, AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
        PIXEL_TEXTURE_FILES,
    },
    buffer::Buffer,
    sprite::Sprites,
};
use log::info;
use sdl2::{
    event::Event as SEvent,
    image::{InitFlag, LoadSurface},
    keyboard::Keycode as SKeycode,
    mouse::*,
    surface::Surface,
    video::{Window, WindowPos::Positioned},
    EventPump, Sdl,
};
use std::any::Any;
use std::time::Duration;

// data for drag window...
#[derive(Default)]
struct Drag {
    need: bool,
    draging: bool,
    mouse_x: i32,
    mouse_y: i32,
    dx: i32,
    dy: i32,
}

pub struct SdlAdapter {
    pub base: AdapterBase,

    // sdl object
    pub sdl_context: Sdl,
    pub sdl_window: Option<Window>,
    pub event_pump: Option<EventPump>,

    // gl object
    pub gl_context: Option<sdl2::video::GLContext>,

    // custom cursor
    pub cursor: Option<Cursor>,

    // data for dragging the window
    drag: Drag,
}

pub enum SdlBorderArea {
    NOPE,
    CLOSE,
    TOPBAR,
    OTHER,
}

impl SdlAdapter {
    pub fn new(pre: &str, gn: &str, project_path: &str) -> Self {
        Self {
            base: AdapterBase::new(pre, gn, project_path),
            sdl_context: sdl2::init().unwrap(),
            event_pump: None,
            cursor: None,
            sdl_window: None,
            gl_context: None,
            drag: Default::default(),
        }
    }

    fn set_mouse_cursor(&mut self, s: &Surface) {
        self.cursor = Some(
            Cursor::from_surface(s, 0, 0)
                .map_err(|err| format!("failed to load cursor: {}", err))
                .unwrap(),
        );
        if let Some(cursor) = &self.cursor {
            cursor.set();
        }
        self.sdl_context.mouse().show_cursor(true);
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
        } else if x > w as i32 && x <= ((sw - 1) as f32 * w) as i32 {
            return SdlBorderArea::NOPE;
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
                        self.drag.draging = true;
                        self.drag.mouse_x = x;
                        self.drag.mouse_y = y;
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
                self.drag.draging = false;
            }
            SEvent::MouseMotion { x, y, .. } if self.drag.draging => {
                self.drag.need = true;
                // dragging window when mouse left button is hold and moving
                self.drag.dx = x - self.drag.mouse_x;
                self.drag.dy = y - self.drag.mouse_y;
            }
            _ => {}
        }
        false
    }
}

impl Adapter for SdlAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String) {
        self.set_size(w, h)
            .set_ratiox(rx)
            .set_ratioy(ry)
            .set_pixel_size()
            .set_title(s);

        let video_subsystem = self.sdl_context.video().unwrap();
        let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG).unwrap();

        // Set OpenGL attributes
        {
            let gl_attr = video_subsystem.gl_attr();
            gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
            gl_attr.set_context_version(3, 3);
        }

        let window = video_subsystem
            .window(&self.base.title, self.base.pixel_w, self.base.pixel_h)
            .opengl()
            .position_centered()
            .borderless()
            // .fullscreen()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let gl_context = window.gl_create_context().unwrap();
        self.gl_context = Some(gl_context);
        video_subsystem.gl_set_swap_interval(1).unwrap(); // Enable vsync

        // Create the OpenGL context using glow
        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                video_subsystem.gl_get_proc_address(s) as *const _
            })
        };

        // Store the OpenGL context
        self.base.gl = Some(gl);
        self.sdl_window = Some(window);

        for texture_file in PIXEL_TEXTURE_FILES.iter() {
            let texture_path = format!(
                "{}{}{}",
                self.base.project_path,
                std::path::MAIN_SEPARATOR,
                texture_file
            );
            info!("gl_pixel load texture...{}", texture_path);
            let img = image::open(texture_path)
                .map_err(|e| e.to_string())
                .unwrap()
                .to_rgba8();
            let width = img.width();
            let height = img.height();
            self.base.gl_pixel = Some(GlPixel::new(
                self.base.gl.as_ref().unwrap(),
                "#version 330 core",
                self.base.pixel_w as i32,
                self.base.pixel_h as i32,
                width as i32,
                height as i32,
                &img,
            ));
        }

        info!("Window & gl init ok...");

        // custom mouse cursor image
        let surface = Surface::from_file(format!(
            "{}{}{}",
            self.base.project_path,
            std::path::MAIN_SEPARATOR,
            "assets/pix/cursor.png"
        ))
        .map_err(|err| format!("failed to load cursor image: {}", err))
        .unwrap();
        self.set_mouse_cursor(&surface);

        // init event_pump
        self.event_pump = Some(self.sdl_context.event_pump().unwrap());
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
        if let Some(ref mut ep) = self.event_pump {
            for event in ep.poll_iter() {
                ses.push(event.clone());
                // convert sdl events to pixel events, providing a unified processing interfaces
                if let Some(et) =
                    input_events_from_sdl(&event, self.base.ratio_x, self.base.ratio_y)
                {
                    if !self.drag.draging {
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
        false
    }

    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        _p: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        // process window draging move...
        sdl_move_win(
            &mut self.drag.need,
            self.sdl_window.as_mut().unwrap(),
            self.drag.dx,
            self.drag.dy,
        );

        self.draw_all_graph(current_buffer, _p, pixel_sprites, stage);

        // swap window for display
        self.sdl_window.as_ref().unwrap().gl_swap_window();
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

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn sdl_move_win(drag_need: &mut bool, win: &mut Window, dx: i32, dy: i32) {
    // dragging window, set the correct position of a window
    if *drag_need {
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
    let sym_width = PIXEL_SYM_WIDTH;
    let sym_height = PIXEL_SYM_HEIGHT;
    let mut mcte: Option<MouseEvent> = None;
    match e {
        SEvent::KeyDown { keycode, .. } => {
            let kc = match keycode {
                Some(SKeycode::Space) => ' ',
                Some(SKeycode::A) => 'a',
                Some(SKeycode::B) => 'b',
                Some(SKeycode::C) => 'c',
                Some(SKeycode::D) => 'd',
                Some(SKeycode::E) => 'e',
                Some(SKeycode::F) => 'f',
                Some(SKeycode::G) => 'g',
                Some(SKeycode::H) => 'h',
                Some(SKeycode::I) => 'i',
                Some(SKeycode::J) => 'j',
                Some(SKeycode::K) => 'k',
                Some(SKeycode::L) => 'l',
                Some(SKeycode::M) => 'm',
                Some(SKeycode::N) => 'n',
                Some(SKeycode::O) => 'o',
                Some(SKeycode::P) => 'p',
                Some(SKeycode::Q) => 'q',
                Some(SKeycode::R) => 'r',
                Some(SKeycode::S) => 's',
                Some(SKeycode::T) => 't',
                Some(SKeycode::U) => 'u',
                Some(SKeycode::V) => 'v',
                Some(SKeycode::W) => 'w',
                Some(SKeycode::X) => 'x',
                Some(SKeycode::Y) => 'y',
                Some(SKeycode::Z) => 'z',
                _ => {
                    return None;
                }
            };
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
