// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Scene supports rendering in both text and graphics modes.
//! The core of it is to draw whatever in the buffer on the screen.
//!
//! Scene contains multiple Layers:
//! - "tui" layer: Contains the mainbuffer sprite for TUI/Widget content
//! - "sprite" layer: Contains game sprites (pixel sprites)
//!
//! Terminal mode relies on the crossterm modules, and it has builtin double buffering.
//!
//! Graphics mode uses OpenGL/WebGL. To support opacity of sprites, buffer stores the char
//! and color of each cell. During rendering, cells are rendered according to their opacity
//! order first to render_texture, and later displayed on the canvas.
//!
//! WEB mode is similar to native graphics mode. However, in WEB mode, RustPixel renders
//! buffer to a shared memory block and shares it with JavaScript, then JS calls WebGL
//! in the browser to render this memory block.

use crate::{
    context::Context,
    render::{
        buffer::Buffer,
        sprite::{Sprite, Layer},
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

pub struct Scene {
    /// Double buffering for TUI content rendering.
    ///
    /// In text mode (Crossterm): Used for diff-based incremental rendering.
    /// - Current buffer holds the new frame content
    /// - Previous buffer holds the last frame content
    /// - Only changed cells are sent to terminal (via buffer.diff())
    ///
    /// In graphics mode (OpenGL/WGPU): Only current buffer is used.
    /// - Previous buffer is passed to adapter but not used (_pb parameter)
    /// - Each frame does full screen clear (glClear) and complete re-render
    /// - GPU rendering is fast enough that diff optimization is unnecessary
    pub tui_buffers: [Buffer; 2],
    pub current: usize,
    pub layer_tag_index: HashMap<String, usize>,
    pub layers: Vec<Layer>,

    // layer index, render weight...
    pub render_index: Vec<(usize, i32)>,
}

#[allow(unused)]
impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    #[allow(unused_mut)]
    pub fn new() -> Self {
        let (width, height) = (180, 80);
        let size = Rect::new(0, 0, width, height);

        let mut layers = vec![];
        let mut layer_tag_index = HashMap::new();

        // TUI layer - for UI elements (borders, messages, etc.)
        let mut tui_layer = Layer::new("tui");
        tui_layer.render_weight = 100;  // Higher weight = rendered on top
        layers.push(tui_layer);
        layer_tag_index.insert("tui".to_string(), 0);

        // Sprite layer - for game sprites
        let sprite_layer = Layer::new("sprite");
        layers.push(sprite_layer);
        layer_tag_index.insert("sprite".to_string(), 1);

        Scene {
            tui_buffers: [Buffer::empty(size), Buffer::empty(size)],
            current: 0,
            layer_tag_index,
            layers,
            render_index: vec![],
        }
    }

    pub fn init(&mut self, ctx: &mut Context) {
        let size = ctx.adapter.size();
        self.tui_buffers[0].resize(size);
        self.tui_buffers[1].resize(size);
        info!("scene init size...{:?}", size);
    }

    /// Get mutable reference to the current buffer (mainbuffer)
    /// This is where Widget/UIApp content should be rendered
    pub fn tui_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.tui_buffers[self.current]
    }

    /// Get the current rendering buffer
    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.tui_buffers[self.current]
    }

    /// Add a new layer with the given name
    pub fn add_layer(&mut self, name: &str) {
        let layer = Layer::new(name);
        self.layers.push(layer);
        self.layer_tag_index
            .insert(name.to_string(), self.layers.len() - 1);
    }

    /// Add a sprite to a specific layer
    pub fn add_layer_sprite(&mut self, sp: Sprite, layer_name: &str, tag: &str) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].add(sp, tag);
    }

    /// Get a sprite from a specific layer
    pub fn get_layer_sprite(&mut self, layer_name: &str, tag: &str) -> &mut Sprite {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].get(tag)
    }

    /// Set the render weight of a layer
    pub fn set_layer_weight(&mut self, layer_name: &str, w: i32) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].render_weight = w;
        self.render_index.clear();
    }

    /// Deactivate (hide) a layer
    pub fn deactive_layer(&mut self, layer_name: &str) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].deactive();
    }

    /// Activate (show) a layer
    pub fn active_layer(&mut self, layer_name: &str) {
        let idx = self.layer_tag_index.get(layer_name).unwrap();
        self.layers[*idx].active();
    }

    /// Add a sprite to the default sprite layer (for game objects)
    pub fn add_sprite(&mut self, sp: Sprite, tag: &str) {
        self.layers[1].add(sp, tag);
    }

    /// Get a sprite from the default sprite layer
    pub fn get_sprite(&mut self, tag: &str) -> &mut Sprite {
        self.layers[1].get(tag)
    }

    /// Execute a closure with multiple sprites simultaneously
    ///
    /// # Example
    /// ```ignore
    /// scene.with_sprites(&["sprite1", "sprite2", "sprite3"], |sprites| {
    ///     let p1 = &mut sprites[0];
    ///     let p2 = &mut sprites[1];
    ///     let p3 = &mut sprites[2];
    ///     // Use p1, p2, p3...
    /// });
    /// ```
    pub fn with_sprites<F, R>(&mut self, tags: &[&str], f: F) -> R
    where
        F: FnOnce(&mut [&mut Sprite]) -> R,
    {
        self.layers[1].with_sprites(tags, f)
    }

    /// Add a sprite to the TUI layer (internal use only)
    /// Users should use Widget system for TUI content
    #[allow(dead_code)]
    fn add_tui_sprite(&mut self, sp: Sprite, tag: &str) {
        self.layers[0].add(sp, tag);
    }

    /// Get a sprite from the TUI layer (internal use only)
    /// Users should use Widget system for TUI content
    #[allow(dead_code)]
    fn get_tui_sprite(&mut self, tag: &str) -> &mut Sprite {
        self.layers[0].get(tag)
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

    /// Draw scene content to RT2 without presenting to screen.
    ///
    /// Use this method when you need to customize the present stage.
    /// After calling this, use `ctx.adapter.present()` or `ctx.adapter.present_default()`
    /// to display the content.
    ///
    /// # Example (Custom Present)
    /// ```rust,ignore
    /// fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
    ///     // Custom RT operations
    ///     ctx.adapter.blend_rts(0, 1, 3, effect, progress);
    ///
    ///     // Render to RT2 (no present)
    ///     self.scene.draw_to_rt(ctx).unwrap();
    ///
    ///     // Custom present with specific viewport
    ///     ctx.adapter.present(&[
    ///         RtComposite::new(2),
    ///         RtComposite::new(3).with_viewport(custom_vp),
    ///     ]);
    /// }
    /// ```
    pub fn draw_to_rt(&mut self, ctx: &mut Context) -> io::Result<()> {
        if ctx.stage > LOGO_FRAME {
            self.update_render_index();
            for idx in &self.render_index {
                if !self.layers[idx.0].is_hidden {
                    // Terminal mode: all layers merge to buffer
                    // Graphics mode: only TUI layer (idx 0) merges to buffer
                    #[cfg(not(graphics_mode))]
                    {
                        self.layers[idx.0]
                            .render_all_to_buffer(&mut ctx.asset_manager, &mut self.tui_buffers[self.current]);
                    }
                    #[cfg(graphics_mode)]
                    {
                        // Only TUI layer merges to buffer in graphics mode
                        if idx.0 == 0 {
                            self.layers[idx.0]
                                .render_all_to_buffer(&mut ctx.asset_manager, &mut self.tui_buffers[self.current]);
                        }
                    }
                }
            }
        }
        let cb = &self.tui_buffers[self.current];
        let pb = &self.tui_buffers[1 - self.current];
        ctx.adapter
            .draw_all(cb, pb, &mut self.layers, ctx.stage)
            .unwrap();

        ctx.adapter.hide_cursor().unwrap();

        // Swap buffers
        if ctx.stage > LOGO_FRAME {
            self.tui_buffers[1 - self.current].reset();
            self.current = 1 - self.current;
        }

        Ok(())
    }

    /// Draw scene and present to screen with default settings.
    ///
    /// This is the standard rendering method for most apps.
    /// It renders all content to RT2 and presents RT2 & RT3 to screen.
    ///
    /// For custom present behavior, use `draw_to_rt()` instead.
    pub fn draw(&mut self, ctx: &mut Context) -> io::Result<()> {
        self.draw_to_rt(ctx)?;

        // Graphics mode: present RT2 & RT3 to screen
        #[cfg(graphics_mode)]
        ctx.adapter.present_default();

        Ok(())
    }

    /// Create a max number of sprites in the sprite layer
    /// and call closure f to init each one
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
            self.add_sprite(bl, &format!("{}{}", &pool.prefix, i));
        }
    }

    /// Draw sprites from object pool
    /// and call closure f to set content and pos
    pub fn draw_objpool<T, F>(&mut self, os: &mut GameObjPool<T>, mut f: F)
    where
        T: GObj,
        F: FnMut(&mut Sprite, &GameObject<T>),
    {
        for o in &os.pool {
            // clear inactive objects
            if !o.active {
                if let Some(oid) = os.map.remove(&o.id) {
                    self.get_sprite(&format!("{}{}", os.prefix, oid))
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
                        let pp = self.get_sprite(&format!("{}{}", os.prefix, i));
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
            let pl = self.get_sprite(&format!("{}{}", os.prefix, psid));
            pl.set_hidden(false);
            f(pl, o);
        }
    }
}

// Type alias for backward compatibility
#[deprecated(note = "Use Scene instead")]
#[allow(dead_code)]
pub type Panel = Scene;
