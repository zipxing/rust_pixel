// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! layer.rs implements a Layer that contains a set of Sprites.
//! Sprites are stored in a vector and can be accessed via offset or by tag.
//! render_all method draws the sprites in a sorted order indicated by render_weight.

use crate::{
    asset::AssetManager,
    render::sprite::Sprite,
    render::buffer::Buffer,
    util::PointU16,
};
use crate::render::sprite::Widget;
// use log::info;
use std::{
    cmp::Reverse,
    collections::HashMap,
    ops::{Index, IndexMut},
};

/// A Layer contains a set of Sprites stored in a vector.
/// Sprites can be accessed via offset in the vector or by tag in the hashmap.
pub struct Layer {
    pub name: String,
    pub is_hidden: bool,
    pub sprites: Vec<Sprite>,
    pub tag_index: HashMap<String, usize>,

    // sprite index, render weight...
    pub render_index: Vec<(usize, i32)>,

    // render weight as layers in scene...
    pub render_weight: i32,
}

/// Implements Index and IndexMut protocol
impl Index<usize> for Layer {
    type Output = Sprite;
    fn index(&self, index: usize) -> &Self::Output {
        &self.sprites[index]
    }
}

impl IndexMut<usize> for Layer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.sprites[index]
    }
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_hidden: false,
            sprites: vec![],
            tag_index: HashMap::new(),
            render_index: vec![],
            render_weight: 0,
        }
    }

    pub fn active(&mut self) {
        self.is_hidden = false;
    }

    pub fn deactive(&mut self) {
        self.is_hidden = true;
    }

    pub fn get_max_size(&self) -> PointU16 {
        let mut mx: u16 = 0;
        let mut my: u16 = 0;
        for ts in &self.sprites {
            let sx = ts.content.area.x + ts.content.area.width;
            if sx > mx {
                mx = sx;
            }
            let sy = ts.content.area.y + ts.content.area.height;
            if sy > my {
                my = sy;
            }
        }
        PointU16 { x: mx, y: my }
    }

    pub fn add(&mut self, ts: Sprite, tag: &str) {
        self.sprites.push(ts);
        self.tag_index
            .insert(tag.to_string(), self.sprites.len() - 1);
        self.render_index.clear();
    }

    /// Add sprite with auto-generated tag (for backward compatibility)
    pub fn add_auto(&mut self, ts: Sprite) {
        self.add(ts, &format!("{}", self.sprites.len()));
    }

    pub fn get(&mut self, name: &str) -> &mut Sprite {
        let idx = self.tag_index.get(name).unwrap();
        &mut self.sprites[*idx]
    }

    /// Execute a closure with multiple sprites simultaneously
    ///
    /// This method allows you to work with multiple sprites at once.
    /// Panics if any tags are duplicated.
    ///
    /// # Safety
    /// Uses unsafe code internally but ensures safety by checking for duplicate indices.
    pub fn with_sprites<F, R>(&mut self, tags: &[&str], f: F) -> R
    where
        F: FnOnce(&mut [&mut Sprite]) -> R,
    {
        // Get all indices and check for duplicates
        let indices: Vec<usize> = tags
            .iter()
            .map(|tag| *self.tag_index.get(*tag).unwrap())
            .collect();

        // Check for duplicates
        let mut sorted = indices.clone();
        sorted.sort_unstable();
        for i in 1..sorted.len() {
            assert_ne!(sorted[i-1], sorted[i], "Cannot get multiple references to the same sprite");
        }

        // SAFETY: We've verified that all indices are unique, so we can safely
        // create multiple mutable references to different sprites
        let mut sprite_refs: Vec<&mut Sprite> = Vec::with_capacity(indices.len());
        for &idx in &indices {
            unsafe {
                let sprite_ptr = self.sprites.as_mut_ptr().add(idx);
                sprite_refs.push(&mut *sprite_ptr);
            }
        }

        f(&mut sprite_refs)
    }

    // to get a non-referencable variable, usually used to
    // copy_content an image from an image set
    pub fn get_immut(&self, name: &str) -> &Sprite {
        let idx = self.tag_index.get(name).unwrap();
        &self.sprites[*idx]
    }

    pub fn set_weight(&mut self, name: &str, w: i32) {
        let idx = self.tag_index.get(name).unwrap();
        self.sprites[*idx].render_weight = w;
        self.render_index.clear();
    }

    pub fn set_hidden(&mut self, name: &str, hidden: bool) {
        let idx = self.tag_index.get(name).unwrap();
        self.sprites[*idx].set_hidden(hidden);
    }

    pub fn update_render_index(&mut self) {
        // renders in an order by render_weight
        // bigger render_weight is rendered later (upper level)
        if self.render_index.is_empty() {
            for (i, s) in self.sprites.iter().enumerate() {
                self.render_index.push((i, s.render_weight));
            }
            self.render_index.sort_by_key(|d| Reverse(d.1));
            // info!("render_index...{:?}", self.render_index);
        }
    }

    /// Render all sprites to buffer by merging their content
    /// Scene::draw controls which layers should call this method
    pub fn render_all_to_buffer(&mut self, am: &mut AssetManager, buffer: &mut Buffer) {
        self.update_render_index();
        for v in &self.render_index {
            // Scene::draw controls which layers are processed based on render mode
            self.sprites[v.0].render(am, buffer);
        }
    }
}

// Type alias for backward compatibility
#[deprecated(note = "Use Layer instead")]
#[allow(dead_code)]
pub type Sprites = Layer;
