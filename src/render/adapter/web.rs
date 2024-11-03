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
        gl::pixel::GlPixel, 
        Adapter, AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, init_sym_width, init_sym_height,
    },
    buffer::Buffer,
    sprite::Sprites,
};
use log::info;
use std::any::Any;
use std::time::Duration;

pub struct WebAdapter {
    pub base: AdapterBase,
}

impl WebAdapter {
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            base: AdapterBase::new(gn, project_path),
        }
    }

    pub fn init_glpix(&mut self, texwidth: i32, texheight: i32, tex: &[u8]) {
        PIXEL_SYM_WIDTH
            .set(init_sym_width(texwidth as u32))
            .expect("lazylock init");
        PIXEL_SYM_HEIGHT
            .set(init_sym_height(texheight as u32))
            .expect("lazylock init");
        self.set_pixel_size();
        self.base.gl_pixel = Some(GlPixel::new(
            self.base.gl.as_ref().unwrap(),
            "#version 300 es",
            self.base.pixel_w as i32,
            self.base.pixel_h as i32,
            texwidth,
            texheight,
            tex,
        ));
    }
}

impl Adapter for WebAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String) {
        self.set_size(w, h)
            .set_ratiox(rx)
            .set_ratioy(ry)
            // .set_pixel_size()
            .set_title(s);

        use wasm_bindgen::JsCast;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let webgl2_context = canvas
            .get_context("webgl2")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .unwrap();
        let gl = glow::Context::from_webgl2_context(webgl2_context);

        // Store the OpenGL context
        self.base.gl = Some(gl);
        info!("Window & gl init ok...");
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH.get().expect("lazylock init") / self.base.ratio_x
    }

    fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT.get().expect("lazylock init") / self.base.ratio_y
    }

    fn poll_event(&mut self, _timeout: Duration, _es: &mut Vec<Event>) -> bool {
        false
    }

    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        _p: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        self.draw_all_graph(current_buffer, _p, pixel_sprites, stage);
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

macro_rules! web_event {
    ($ek:expr, $ei:expr, $($btn:expr)* ) => {
        Some(MouseEvent {
            kind: $ek$(($btn))*,
            column: $ei.5 as u16,
            row: $ei.6 as u16,
            modifiers: KeyModifiers::NONE,
        })
    };
}

/// Convert web I/O events to RustPixel event, for the sake of unified event processing
/// For keyboard and mouse event, please refer to the handle_input method in game/unblock/model.rs
pub fn input_events_from_web(t: u8, e: web_sys::Event, pixel_h: u32, ratiox: f32, ratioy: f32) -> Option<Event> {
    let sym_width = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as f32;
    let sym_height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as f32;
    let mut mcte: Option<MouseEvent> = None;

    if let Some(key_e) = wasm_bindgen::JsCast::dyn_ref::<web_sys::KeyboardEvent>(&e) {
        assert!(t == 0);
        let kcc = (key_e.key_code(), key_e.char_code());
        match kcc.0 {
            32 | 48..=57 | 97..=122 => {
                let cte = KeyEvent::new(
                    KeyCode::Char(char::from_u32(kcc.0).unwrap()),
                    KeyModifiers::NONE,
                );
                return Some(Event::Key(cte));
            }
            _ => {
                return None;
            }
        }
    }

    if let Some(mouse_e) = wasm_bindgen::JsCast::dyn_ref::<web_sys::MouseEvent>(&e) {
        let medat = (
            mouse_e.buttons(),
            mouse_e.screen_x(),
            mouse_e.screen_y(),
            mouse_e.client_x(),
            mouse_e.client_y(),
            mouse_e.x(),
            mouse_e.y(),
        );
        match t {
            1 => {
                mcte = web_event!(Up, medat, Left);
            }
            2 => {
                mcte = web_event!(Down, medat, Left);
            }
            3 => {
                if medat.0 == 1 {
                    mcte = web_event!(Drag, medat, Left);
                } else {
                    mcte = web_event!(Moved, medat,);
                }
            }
            _ => {}
        }
    }
    if let Some(mut mc) = mcte {
        mc.column /= (sym_width / ratiox) as u16;
        // adjust row by upper space
        mc.row -= 800 - pixel_h as u16;
        mc.row /= (sym_height / ratioy) as u16;
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
