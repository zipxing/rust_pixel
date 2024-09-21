// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements an Adapter trait. Moreover,
//! all SDL related processing is handled here.
//! Includes resizing of height and width, init settings,
//! some code is also called in cell.rs
use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
use crate::render::adapter::sdl::gl_color::GlColor;
use crate::render::adapter::sdl::gl_pix::GlPix;
use crate::render::adapter::sdl::gl_texture::GlCell;
use crate::render::adapter::sdl::gl_texture::GlRenderTexture;
use crate::render::adapter::sdl::gl_texture::GlTexture;
use crate::render::adapter::sdl::gl_transform::GlTransform;
use crate::render::{
    adapter::{Adapter, AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILES},
    buffer::Buffer,
    sprite::Sprites,
};
// use log::info;
use sdl2::{
    event::Event as SEvent,
    image::{InitFlag, LoadSurface},
    keyboard::Keycode as SKeycode,
    mouse::*,
    surface::Surface,
    video::{Window, WindowPos::Positioned},
    EventPump,
    Sdl,
};
use std::any::Any;
use std::time::Duration;

pub mod gl_color;
pub mod gl_pix;
pub mod gl_shader;
pub mod gl_texture;
pub mod gl_transform;

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
    pub gl: Option<glow::Context>,
    pub gl_context: Option<sdl2::video::GLContext>,
    pub gl_pix: Option<GlPix>,
    pub gl_symbols: Vec<GlCell>,
    pub gl_render_textures: Vec<GlRenderTexture>,

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
            gl: None,
            gl_pix: None,
            gl_symbols: vec![],
            gl_render_textures: vec![],
            drag: Default::default(),
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
        self.gl = Some(gl);
        self.sdl_window = Some(window);

        let mut pix = GlPix::new(
            self.gl.as_ref().unwrap(),
            self.base.pixel_w as i32,
            self.base.pixel_h as i32,
        );
        pix.set_clear_color(GlColor::new(0.0, 0.0, 0.1, 1.0));

        // init gl_symbols
        for texture_file in PIXEL_TEXTURE_FILES.iter() {
            let texture_path = format!(
                "{}{}{}",
                self.base.project_path,
                std::path::MAIN_SEPARATOR,
                texture_file
            );
            let mut sprite_sheet =
                GlTexture::new(self.gl.as_ref().unwrap(), &texture_path).unwrap();
            sprite_sheet.bind(self.gl.as_ref().unwrap());
            for i in 0..32 {
                for j in 0..32 {
                    let frame = pix.make_cell_frame(
                        &mut sprite_sheet,
                        j as f32 * 17.0,
                        i as f32 * 17.0,
                        16.0,
                        16.0,
                        8.0,
                        8.0,
                    );
                    let cell = GlCell::new(frame);
                    self.gl_symbols.push(cell);
                }
            }
        }
        self.gl_pix = Some(pix);

        // create 2 render texture for gl transition...
        for _i in 0..2 {
            let rt = GlRenderTexture::new(
                self.gl.as_ref().unwrap(),
                self.base.pixel_w,
                self.base.pixel_h,
            )
            .unwrap();
            self.gl_render_textures.push(rt);
        }

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
        match &mut self.event_pump {
            Some(ref mut ep) => {
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
            _ => {}
        }
        false
    }

    fn render_buffer(
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

        // render every thing to rbuf
        self.gen_render_buffer(current_buffer, _p, pixel_sprites, stage);

        let ratio_x = self.get_base().ratio_x;
        let ratio_y = self.get_base().ratio_y;

        // render rbuf to window use opengl
        if let (Some(pix), Some(gl)) = (&mut self.gl_pix, &mut self.gl) {
            pix.bind(gl);
            pix.clear(gl);

            for r in &self.base.rbuf {
                let texidx = r.texsym as usize;
                let spx = r.x as f32 + 16.0;
                let spy = r.y as f32 + 16.0;
                let ang = r.angle as f32 / 1000.0;
                let cpx = r.cx as f32;
                let cpy = r.cy as f32;

                let mut transform = GlTransform::new();
                transform.translate(spx + cpx - 16.0, spy + cpy - 16.0);
                if ang != 0.0 {
                    transform.rotate(ang);
                }
                transform.translate(-cpx + 8.0, -cpy + 8.0);
                transform.scale(1.0 / ratio_x, 1.0 / ratio_y);

                if r.back != 0 {
                    let back_color = GlColor::new(
                        r.br as f32 / 255.0,
                        r.bg as f32 / 255.0,
                        r.bb as f32 / 255.0,
                        r.ba as f32 / 255.0,
                    );
                    self.gl_symbols[320].draw(gl, pix, &transform, &back_color);
                }

                let color = GlColor::new(
                    r.r as f32 / 255.0,
                    r.g as f32 / 255.0,
                    r.b as f32 / 255.0,
                    r.a as f32 / 255.0,
                );
                self.gl_symbols[texidx].draw(gl, pix, &transform, &color);
            }

            pix.flush(gl);
            self.sdl_window.as_ref().unwrap().gl_swap_window();
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
