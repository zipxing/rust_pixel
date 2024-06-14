// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! This module implements a simple object pool
//! It is designed for recycling of objects, reducing costs for creating objects
//!
//! render::panel provides create_sprites, draw_objs methods to create
//! render sprite and render objects and can be used jointly

use crate::util::Point;
use std::collections::HashMap;
// use log::info;

/// game object interface, requires to implement new and reset method
pub trait GObj {
    fn new(t: u8, ps: &Vec<Point>) -> Self;
    fn reset(&mut self, t: u8, ps: &Vec<Point>);
}

/// game object, id is the index offset in the objpool
/// and to identify and get access to the object
/// active is to label whether an object is active,
/// to recycling an object, simply set the active flag to false
pub struct GameObject<T>
where
    T: GObj,
{
    pub id: usize,
    pub obj: T,
    pub active: bool,
}

/// put a game object in the pool
/// map is used to maintaining the mapping between sprite and game object
/// key is the id of the game object while value is the sprite's id
/// refer to panel.draw_objs for more details
pub struct GameObjPool<T>
where
    T: GObj,
{
    pub map: HashMap<usize, usize>,
    pub pool: Vec<GameObject<T>>,
    pub prefix: String,
    pub max_count: usize,
}

impl<T> GameObjPool<T>
where
    T: GObj,
{
    pub fn new(pre: &str, mc: usize) -> Self {
        Self {
            map: HashMap::new(),
            pool: vec![],
            prefix: pre.to_string(),
            max_count: mc,
        }
    }

    pub fn create(&mut self, otype: u8, ps: &Vec<Point>) {
        let mut find = false;
        // search for an available object
        for o in &mut self.pool {
            if !o.active {
                o.obj.reset(otype, ps);
                o.active = true;
                find = true;
                break;
            }
        }
        // if not found, create a new one and add to the pool
        if !find {
            let l = self.pool.len();
            let bo = GObj::new(otype, ps);
            self.pool.push(GameObject {
                id: l,
                obj: bo,
                active: true,
            });
        }
    }

    // processing active object by calling custom closure
    pub fn update_active<F>(&mut self, mut f: F) 
    where
        F: FnMut(&mut GameObject<T>),
    {
        for o in self.pool.iter_mut().filter(|o| o.active) {
            f(o);
        }
    }
}
