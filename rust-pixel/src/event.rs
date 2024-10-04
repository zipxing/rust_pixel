// RustPixel
// copyright zipxing@hotmail.com 2022~2024


//! this event module provides a global event centre and a global timer centre.
//! It is based on Mutex. Mutex is easy to use despite a tiny loss of performance.
//! Another way to achieve this is to put Event and Timer in context. However, it
//! requires access to context from everywhere, which is again not ideal
//!
//! one event in this class namely Event, describing I/Os from keyboard and mouse
//! Input events triggered by renders such as web, sdl or cross are converted here to
//! unified Event


use crate::GAME_FRAME;
use lazy_static::lazy_static;
use serde::Serialize;
use std::{collections::HashMap, sync::Mutex};

// uses global Mutex variable
lazy_static! {
    pub static ref GAME_TIMER: Mutex<Timers> = Mutex::new(Timers::new());
    pub static ref EVENT_CENTER: Mutex<HashMap<String, HashMap<String, bool>>> =
        Mutex::new(HashMap::new());
}

/// A global HashMap is used to save callbacks of events
pub fn event_register(event: &str, func: &str) {
    let mut ec = EVENT_CENTER.lock().unwrap();
    match ec.get_mut(event) {
        Some(ht) => {
            ht.insert(func.to_string(), false);
        }
        None => {
            let mut h: HashMap<String, bool> = HashMap::new();
            h.insert(func.to_string(), false);
            ec.insert(event.to_string(), h);
        }
    }
}

pub fn event_check(event: &str, func: &str) -> bool {
    let mut ec = EVENT_CENTER.lock().unwrap();
    if let Some(ht) = ec.get_mut(event) { if let Some(flag) = ht.get_mut(func) {
        if *flag {
            *flag = false;
            return true;
        }
    } }
    false
}

pub fn event_emit(event: &str) {
    if let Some(ht) = EVENT_CENTER.lock().unwrap().get_mut(event) {
        for (_key, value) in ht {
            if !(*value) {
                *value = true;
            }
        }
    }
}

pub fn timer_register(name: &str, time: f32, func: &str) {
    GAME_TIMER.lock().unwrap().register(name, time, func);
}

pub fn timer_set_time(name: &str, time: f32) {
    GAME_TIMER.lock().unwrap().set_time(name, time);
}

pub fn timer_stage(name: &str) -> u32 {
    GAME_TIMER.lock().unwrap().stage(name)
}

pub fn timer_rstage(name: &str) -> u32 {
    GAME_TIMER.lock().unwrap().rstage(name)
}

pub fn timer_percent(name: &str) -> f32 {
    GAME_TIMER.lock().unwrap().percent(name)
}

pub fn timer_exdata(name: &str) -> Option<Vec<u8>> {
    GAME_TIMER.lock().unwrap().exdata(name)
}

pub fn timer_fire<T>(name: &str, value: T)
where
    T: Serialize,
{
    GAME_TIMER.lock().unwrap().fire(name, value);
}

pub fn timer_cancel(name: &str, nall: bool) {
    GAME_TIMER.lock().unwrap().cancel(name, nall);
}

pub fn timer_update() {
    GAME_TIMER.lock().unwrap().update()
}

pub struct Timer {
    time: u32,
    count: u32,
    exdata: Vec<u8>,
}

#[derive(Default)]
pub struct Timers {
    pub timers: HashMap<String, Timer>,
}

impl Timers {
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    pub fn register(&mut self, name: &str, time: f32, callback: &str) {
        match self.timers.get_mut(name) {
            Some(_) => {}
            None => {
                let timer = Timer {
                    time: 0,
                    count: (time * GAME_FRAME as f32) as u32,
                    exdata: vec![],
                };
                self.timers.insert(name.to_string(), timer);
                event_register(name, callback);
            }
        }
    }

    pub fn stage(&mut self, name: &str) -> u32 {
        match self.timers.get_mut(name) {
            Some(timer) => {
                timer.time
            }
            None => {
                0
            }
        }
    }

    pub fn rstage(&mut self, name: &str) -> u32 {
        match self.timers.get_mut(name) {
            Some(timer) => {
                timer.count - timer.time
            }
            None => {
                0
            }
        }
    }

    pub fn percent(&mut self, name: &str) -> f32 {
        match self.timers.get_mut(name) {
            Some(timer) => {
                timer.time as f32 / timer.count as f32
            }
            None => {
                0f32
            }
        }
    }

    pub fn set_time(&mut self, name: &str, time: f32) {
        if let Some(timer) = self.timers.get_mut(name) {
            timer.count = (time * GAME_FRAME as f32) as u32;
            //may cause count equals 0 and therefore can not be triggered if time is too small
            //to prevent this, reset the count to 1
            if timer.count == 0 {
                timer.count += 1;
            }
        }
    }

    pub fn exdata(&mut self, name: &str) -> Option<Vec<u8>> {
        match self.timers.get_mut(name) {
            Some(timer) => {
                Some(timer.exdata.clone())
            }
            None => {
                None
            }
        }
    }

    pub fn fire<T>(&mut self, name: &str, value: T)
    where
        T: Serialize,
    {
        if let Some(timer) = self.timers.get_mut(name) {
            timer.time = timer.count;
            timer.exdata = bincode::serialize(&value).unwrap();
        }
    }

    pub fn cancel(&mut self, name: &str, nocall: bool) {
        if let Some(timer) = self.timers.get_mut(name) {
            timer.time = 0;
            if !nocall {
                event_emit(name);
            }
        }
    }

    pub fn update(&mut self) {
        for (name, timer) in &mut self.timers {
            if timer.time > 0 {
                timer.time -= 1;
                if timer.time == 0 {
                    event_emit(name);
                }
            }
        }
    }
}

mod input;
pub use input::*;
