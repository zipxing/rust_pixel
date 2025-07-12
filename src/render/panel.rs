// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Panel supports rendering in both text and graphics modes
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
//! WEB mode is similar to SDL mode, both are graphics modes. However,
//! in WEB mode, RustPixel renders buffer to a shared memory block and shared it
//! with JavaScript in WEB, then JS calls webgl in the browser to render this memory block.
//! Refer to the implementation in pixel.js

use crate::{
    context::Context,
    render::{
        buffer::Buffer,
        sprite::{Sprite, Sprites},
    },
    util::{
        objpool::{GObj, GameObjPool, GameObject},
        Rect,
    },
    LOGO_FRAME,
};
use log::info;
use std::{collections::HashMap, io};
use std::cmp::Reverse;

pub struct Panel {
    pub buffers: [Buffer; 2],
    pub current: usize,
    pub layer_tag_index: HashMap<String, usize>,
    pub layers: Vec<Sprites>,

    // layer index, render weight...
    pub render_index: Vec<(usize, i32)>,
}

#[allow(unused)]
impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    #[allow(unused_mut)]
    pub fn new() -> Self {
        let (width, height) = (180, 80);
        let size = Rect::new(0, 0, width, height);

        let mut layers = vec![];
        let nsc = Sprites::new("main");
        layers.push(nsc);

        let mut sc = Sprites::new("pixel");
        sc.is_pixel = true;
        layers.push(sc);

        let mut layer_tag_index = HashMap::new();
        layer_tag_index.insert("main".to_string(), 0);
        layer_tag_index.insert("pixel".to_string(), 1);

        Panel {
            buffers: [Buffer::empty(size), Buffer::empty(size)],
            current: 0,
            layer_tag_index,
            layers,
            render_index: vec![],
        }
    }

    pub fn init(&mut self, ctx: &mut Context) {
        let size = ctx.adapter.size();
        self.buffers[0].resize(size);
        self.buffers[1].resize(size);
        info!("panel init size...{:?}", size);
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }

    fn add_layer_inner(&mut self, name: &str, is_pixel: bool) {
        let sps = if is_pixel {
            Sprites::new_pixel(name)
        } else {
            Sprites::new(name)
        };
        self.layers.push(sps);
        self.layer_tag_index
            .insert(name.to_string(), self.layers.len() - 1);
    }

    pub fn add_layer(&mut self, name: &str) {
        self.add_layer_inner(name, false);
    }

    pub fn add_layer_pixel(&mut self, name: &str) {
        self.add_layer_inner(name, true);
    }

    pub fn add_layer_sprite(&mut self, sp: Sprite, layer_name: &str, tag: &str) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].add_by_tag(sp, tag);
    }

    pub fn get_layer_sprite(&mut self, layer_name: &str, tag: &str) -> &mut Sprite {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].get_by_tag(tag)
    }

    pub fn set_layer_weight(&mut self, layer_name: &str, w: i32) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].render_weight = w;
        self.render_index.clear();
    }

    pub fn deactive_layer(&mut self, layer_name: &str) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].deactive();
    }

    pub fn active_layer(&mut self, layer_name: &str) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].active();
    }

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

    pub fn update_render_index(&mut self) {
        if self.render_index.is_empty() {
            for (i, s) in self.layers.iter().enumerate() {
                self.render_index.push((i, s.render_weight));
            }
            self.render_index.sort_by_key(|d| Reverse(d.1));
        }
    }

    pub fn draw(&mut self, ctx: &mut Context) -> io::Result<()> {
        if ctx.stage > LOGO_FRAME {
            self.update_render_index();
            for idx in &self.render_index {
                if !self.layers[idx.0].is_hidden {
                    self.layers[idx.0]
                        .render_all_to_buffer(&mut ctx.asset_manager, &mut self.buffers[self.current]);
                }
            }
        }
        let cb = &self.buffers[self.current];
        let pb = &self.buffers[1 - self.current];
        ctx.adapter
            .draw_all(cb, pb, &mut self.layers, ctx.stage)
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
                if let Some(oid) = os.map.remove(&o.id) {
                    //info!("render set hidden true...");
                    self.get_pixel_sprite(&format!("{}{}", os.prefix, oid))
                        .set_hidden(true);
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
