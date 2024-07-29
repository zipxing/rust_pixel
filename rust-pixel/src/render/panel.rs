// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Panel supports rendering in both text and graphical modes
//! The core of it is to draw whatever in the buffer on the screen
//!
//! terminal mode relies on the crossterm modules, and it has builtin double buffering
//!
//! SDL is built on rustsdl2 module.To support opacity of sprites, buffer stores the char and color
//! of each cell in every framework in SDL mode.
//! During rendering, cell is rendered according to its opacity order first to render_texture,
//! and later render_text displays on the canvas.
//! To further enhance our functionality, a set of special sprites：pixel_sprites are provided in SDL mode.
//! They can be set per pixel, and are managed in the same way as cell.
//! During rendering, they can be rendered by its pixel position or can be rotated about its centre.
//! Please refer to the flush method or the tower defense game in games/tower where pixel_sprite is
//! massively applied.
//!
//! WEB mode is similar to SDL mode, both are graphical modes. However,
//! in WEB mode, RustPixel renders buffer to a shared memory block and shared it
//! with JavaScript in WEB, then JS calls webgl in the browser to render this memory block.
//! Refer to the implementation in pixel.js

use crate::{
    context::Context,
    render::{
        buffer::Buffer,
        cell::Cell,
        sprite::{Sprite, Sprites},
        style::Color,
    },
    util::{
        objpool::{GObj, GameObjPool, GameObject},
        Rect,
    },
    LOGO_FRAME,
};
use log::info;
use std::{
    io,
    collections::HashMap,
};

pub struct Panel {
    pub buffers: [Buffer; 2],
    pub current: usize,
    pub layer_tag_index: HashMap<String, usize>,
    pub layers: Vec<Sprites>,
}

#[allow(unused)]
impl Panel {
    #[allow(unused_mut)]
    pub fn new() -> Self {
        let (width, height) = (180, 80);
        let size = Rect::new(0, 0, width, height);
        let mut layers = vec![];
        let sc = Sprites::new("pixel");
        let nsc = Sprites::new("main");
        layers.push(nsc);
        layers.push(sc);
        let mut layer_tag_index = HashMap::new();
        layer_tag_index.insert("main".to_string(), 0);
        layer_tag_index.insert("pixel".to_string(), 1);
        Panel {
            buffers: [Buffer::empty(size), Buffer::empty(size)],
            current: 0,
            layer_tag_index,
            layers,
        }
    }

    pub fn init(&mut self, ctx: &mut Context) {
        let size = ctx.adapter.size();
        self.buffers[0].resize(size);
        self.buffers[1].resize(size);
        info!("panel init size...{:?}", size);
    }

    pub fn clear(&mut self, ctx: &mut Context, area: Rect, sym: &str, fg: Color, bg: Color) {
        let size = ctx.adapter.size();
        self.buffers[0] = Buffer::empty(size);
        self.buffers[1] = Buffer::empty(size);
        let mut cell: Cell = Default::default();
        cell.set_symbol(sym);
        cell.set_fg(fg);
        cell.set_bg(bg);
        let buf = Buffer::filled(area, &cell);
        self.buffers[0].merge(&buf, 255, false);
        ctx.adapter
            .render_buffer(
                &self.buffers[0],
                &self.buffers[1],
                &mut self.layers[1],
                ctx.stage,
            )
            .unwrap();
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }

    /*pub add_layer(&mut self, tag: &str) {

    }*/

    pub fn add_sprite(&mut self, sp: Sprite, tag: &str) {
        self.layers[0].add_by_tag(sp, tag);
    }

    pub fn get_sprite(&mut self, tag: &str) -> &mut Sprite {
        self.layers[0].get_by_tag(tag)
    }

    pub fn add_pixel_sprite(&mut self, sp: Sprite, tag: &str) {
        self.layers[1].add_by_tag(sp, tag);
    }

    pub fn get_pixel_sprite(&mut self, tag: &str) -> &mut Sprite {
        self.layers[1].get_by_tag(tag)
    }

    pub fn reset(&mut self, ctx: &mut Context) {
        ctx.adapter.reset();
    }

    pub fn draw(&mut self, ctx: &mut Context) -> io::Result<()> {
        self.layers[0]
            .render_all(&mut ctx.asset_manager, &mut self.buffers[self.current]);
        for idx in 0..self.layers[1].render_index.len() {
            let si = self.layers[1].render_index[idx];
            let s = &mut self.layers[1].sprites[si.0];
            if s.is_hidden() {
                continue;
            }
            s.check_asset_request(&mut ctx.asset_manager);
        }

        let cb = &self.buffers[self.current];
        let pb = &self.buffers[1 - self.current];
        ctx.adapter
            .render_buffer(cb, pb, &mut self.layers[1], ctx.stage)
            .unwrap();
        ctx.adapter.hide_cursor().unwrap();

        // Swap buffers
        if ctx.stage > LOGO_FRAME {
            self.buffers[1 - self.current].reset();
            self.current = 1 - self.current;
        }

        Ok(())
    }

    /// create a max number of sprites
    /// and calls f closure to init
    pub fn creat_objpool_sprites<T, F>(
        &mut self,
        pool: &GameObjPool<T>,
        size_x: u16,
        size_y: u16,
        mut f: F,
    ) where
        T: GObj,
        F: FnMut(&mut Sprite),
    {
        for i in 0..pool.max_count {
            let mut bl = Sprite::new(0, 0, size_x, size_y);
            f(&mut bl);
            bl.set_hidden(true);
            self.add_pixel_sprite(bl, &format!("{}{}", &pool.prefix, i));
        }
    }

    /// drawing sprites
    /// and calls f closure to set content and pos
    pub fn draw_objpool<T, F>(&mut self, os: &mut GameObjPool<T>, mut f: F)
    where
        T: GObj,
        F: FnMut(&mut Sprite, &GameObject<T>),
    {
        for o in &os.pool {
            // clear inactive objects
            if !o.active {
                match os.map.remove(&o.id) {
                    Some(oid) => {
                        //info!("render set hidden true...");
                        self.get_pixel_sprite(&format!("{}{}", os.prefix, oid))
                            .set_hidden(true);
                    }
                    _ => {}
                }
                continue;
            }
            let psid = match os.map.get(&o.id) {
                // if the map contains the object, set psid
                Some(oid) => *oid,
                _ => {
                    let mut mi = 0;
                    // find an available sprite
                    for i in 0..os.max_count {
                        let pp = self.get_pixel_sprite(&format!("{}{}", os.prefix, i));
                        if pp.is_hidden() {
                            mi = i;
                            break;
                        }
                    }
                    // key is GameObject id, and value is sprite id
                    os.map.insert(o.id, mi);
                    mi
                }
            };
            // concatenate pre and psid to get the sprite, set visible and draw
            let pl = self.get_pixel_sprite(&format!("{}{}", os.prefix, psid));
            pl.set_hidden(false);
            f(pl, o);
        }
    }
}
