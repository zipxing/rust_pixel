// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Mutex + lazy_static implementation for comparison
//! This is the original implementation using Mutex

use crate::GAME_FRAME;
use lazy_static::lazy_static;
use serde::Serialize;
use std::{collections::HashMap, sync::Mutex};

// Version 2: Mutex + lazy_static (original)
lazy_static! {
    pub static ref GAME_TIMER_MUTEX: Mutex<Timers> = Mutex::new(Timers::new());
    pub static ref EVENT_CENTER_MUTEX: Mutex<HashMap<String, HashMap<String, bool>>> =
        Mutex::new(HashMap::new());
}

/// A global HashMap is used to save callbacks of events
pub fn event_register(event: &str, func: &str) {
    let mut ec = EVENT_CENTER_MUTEX.lock().unwrap();
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
    let mut ec = EVENT_CENTER_MUTEX.lock().unwrap();
    if let Some(ht) = ec.get_mut(event) { 
        if let Some(flag) = ht.get_mut(func) {
            if *flag {
                *flag = false;
                return true;
            }
        } 
    }
    false
}

pub fn event_emit(event: &str) {
    if let Some(ht) = EVENT_CENTER_MUTEX.lock().unwrap().get_mut(event) {
        for value in ht.values_mut() {
            if !(*value) {
                *value = true;
            }
        }
    }
}

pub fn timer_register(name: &str, time: f32, func: &str) {
    GAME_TIMER_MUTEX.lock().unwrap().register(name, time, func);
}

pub fn timer_set_time(name: &str, time: f32) {
    GAME_TIMER_MUTEX.lock().unwrap().set_time(name, time);
}

pub fn timer_stage(name: &str) -> u32 {
    GAME_TIMER_MUTEX.lock().unwrap().stage(name)
}

pub fn timer_rstage(name: &str) -> u32 {
    GAME_TIMER_MUTEX.lock().unwrap().rstage(name)
}

pub fn timer_percent(name: &str) -> f32 {
    GAME_TIMER_MUTEX.lock().unwrap().percent(name)
}

pub fn timer_exdata(name: &str) -> Option<Vec<u8>> {
    GAME_TIMER_MUTEX.lock().unwrap().exdata(name)
}

pub fn timer_fire<T>(name: &str, value: T)
where
    T: Serialize,
{
    GAME_TIMER_MUTEX.lock().unwrap().fire(name, value);
}

pub fn timer_cancel(name: &str, nall: bool) {
    GAME_TIMER_MUTEX.lock().unwrap().cancel(name, nall);
}

pub fn timer_update() {
    GAME_TIMER_MUTEX.lock().unwrap().update()
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
            // may cause count equals 0 and therefore can not be triggered if time is too small
            // to prevent this, reset the count to 1
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_mutex_implementation() {
        // Test event registration and triggering
        event_register("test_event_mutex", "test_callback");
        
        // Test event check (should be false initially)
        assert!(!event_check("test_event_mutex", "test_callback"));
        
        // Trigger event
        event_emit("test_event_mutex");
        
        // Should be true now
        assert!(event_check("test_event_mutex", "test_callback"));
        
        // Check again should be false (because it has been consumed)
        assert!(!event_check("test_event_mutex", "test_callback"));
    }
    
    #[test]
    fn test_timer_mutex_implementation() {
        // Test timer registration
        timer_register("test_timer_mutex", 1.0, "timer_callback");
        
        // Test initial state
        assert_eq!(timer_stage("test_timer_mutex"), 0);
        assert_eq!(timer_rstage("test_timer_mutex"), 60); // 1.0 * GAME_FRAME
        assert_eq!(timer_percent("test_timer_mutex"), 0.0);
        
        // Test setting time
        timer_set_time("test_timer_mutex", 2.0);
        assert_eq!(timer_rstage("test_timer_mutex"), 120); // 2.0 * GAME_FRAME
        
        // Test data storage
        timer_fire("test_timer_mutex", "test_data");
        let exdata = timer_exdata("test_timer_mutex");
        assert!(exdata.is_some());
    }
    
    #[test]
    fn benchmark_mutex_performance() {
        let iterations = 10000;
        
        // Event operation benchmark test
        let start = Instant::now();
        for i in 0..iterations {
            let event_name = format!("bench_event_mutex_{}", i % 10);
            let callback_name = format!("bench_callback_{}", i % 10);
            
            event_register(&event_name, &callback_name);
            event_emit(&event_name);
            event_check(&event_name, &callback_name);
        }
        let event_time = start.elapsed();
        
        // Timer operation benchmark test
        let start = Instant::now();
        for i in 0..iterations {
            let timer_name = format!("bench_timer_mutex_{}", i % 10);
            let callback_name = format!("bench_callback_{}", i % 10);
            
            timer_register(&timer_name, 0.1, &callback_name);
            timer_stage(&timer_name);
            timer_percent(&timer_name);
        }
        let timer_time = start.elapsed();
        
        println!("Mutex version:");
        println!("  Event operations {} times took: {:?}", iterations, event_time);
        println!("  Timer operations {} times took: {:?}", iterations, timer_time);
        println!("  Average time per event operation: {:?}", event_time / iterations);
        println!("  Average time per timer operation: {:?}", timer_time / iterations);
    }
} 