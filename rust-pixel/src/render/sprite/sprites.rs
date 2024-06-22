// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! sprites.rs实现了精灵集合Sprites，把一系列Sprite存储在vector中
//! 可以按照索引偏移直接访问，也可以通过hashmap来按照tag访问
//! render_all方法，会按照sprite的render_weight排序绘制
//!
//! sprites.rs implements a set of Sprites. Those Sprites are stored in a vector
//! Sprite can be accessed via offset in the vector or by tag in the hashmap
//! render_all method draws the sprites in a sorted order indicated by render_weight

use crate::{
    asset::AssetManager,
    render::sprite::Sprite,
    render::panel::Frame,
    util::PointU16,
};
// use log::info;
use std::{
    cmp::Reverse,
    collections::HashMap,
    ops::{Index, IndexMut},
};

/// 文本精灵集合，把一系列Sprite存储在vector中
/// 可以按照索引偏移直接访问，也可以通过hashmap来按照
/// tag访问
/// Set of text sprite, stored in a vector
/// Sprite can be accessed via offset in the vector or by tag in the hashmap
pub struct Sprites {
    pub name: String,
    pub sprites: Vec<Sprite>,
    pub tag_index: HashMap<String, usize>,

    // sprite index, render weight...
    pub render_index: Vec<(usize, i32)>,
}

/// 实现Index，IndexMut协议
/// Implements Index and IndexMut protocol
impl Index<usize> for Sprites {
    type Output = Sprite;
    fn index(&self, index: usize) -> &Self::Output {
        &self.sprites[index]
    }
}

impl IndexMut<usize> for Sprites {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.sprites[index]
    }
}

impl Sprites {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sprites: vec![],
            tag_index: HashMap::new(),
            render_index: vec![],
        }
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

    pub fn add(&mut self, ts: Sprite) {
        self.add_by_tag(ts, &format!("{}", self.sprites.len()));
    }

    pub fn add_by_tag(&mut self, ts: Sprite, tag: &str) {
        self.sprites.push(ts);
        self.tag_index
            .insert(tag.to_string(), self.sprites.len() - 1);
        self.render_index.clear();
    }

    pub fn get_by_tag(&mut self, name: &str) -> &mut Sprite {
        let idx = self.tag_index.get(name).unwrap();
        &mut self.sprites[*idx]
    }

    // 用于获取一个不可变引用，常用于
    // 从图集中获取一个图用于copy_content
    // to get a non-referencable variable, usually used to
    // copy_content an image from an image set
    pub fn get_by_tag_immut(&self, name: &str) -> &Sprite {
        let idx = self.tag_index.get(name).unwrap();
        &self.sprites[*idx]
    }

    pub fn set_weight_by_tag(&mut self, name: &str, w: i32) {
        let idx = self.tag_index.get(name).unwrap();
        self.sprites[*idx].render_weight = w;
        self.render_index.clear();
    }

    pub fn set_hidden_by_tag(&mut self, name: &str, hidden: bool) {
        let idx = self.tag_index.get(name).unwrap();
        self.sprites[*idx].set_hidden(hidden);
    }

    pub fn update_render_index(&mut self) {
        // 按照render_weight进行排序
        // render_weight越大的越后面渲染（上层)
        // renders in an order by render_weight
        // bigger render_weight is rendered later（upper level)
        if self.render_index.len() == 0 {
            let mut i = 0usize;
            for s in &self.sprites {
                self.render_index.push((i, s.render_weight));
                i += 1;
            }
            self.render_index.sort_by_key(|d| Reverse(d.1));
            // info!("render_index...{:?}", self.render_index);
        }
    }

    pub fn render_all(&mut self, am: &mut AssetManager, frame: &mut Frame) {
        self.update_render_index();
        for v in &self.render_index {
            frame.render_widget(am, &mut self.sprites[v.0]);
        }
    }
}
