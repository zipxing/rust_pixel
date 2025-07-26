// RustPixel
// copyright zipxing@hotmail.com 2022～2025


//! This event module provides a global event center and a global timer center.
//! For comparison testing: thread_local! + Rc<RefCell<T>> vs Mutex + lazy_static
//!
//! One event in this class namely Event, describing I/Os from keyboard and mouse.
//! Input events triggered by renders such as web, SDL or cross are converted here to
//! unified Event.


use crate::GAME_FRAME;
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

// Version 1: thread_local! + Rc<RefCell<T>>
thread_local! {
    static GAME_TIMER: Rc<RefCell<Timers>> = Rc::new(RefCell::new(Timers::new()));
    static EVENT_CENTER: Rc<RefCell<HashMap<String, HashMap<String, bool>>>> = 
        Rc::new(RefCell::new(HashMap::new()));
}

/// A global HashMap is used to save callbacks of events
pub fn event_register(event: &str, func: &str) {
    EVENT_CENTER.with(|ec| {
        let mut ec_ref = ec.borrow_mut();
        match ec_ref.get_mut(event) {
            Some(ht) => {
                ht.insert(func.to_string(), false);
            }
            None => {
                let mut h: HashMap<String, bool> = HashMap::new();
                h.insert(func.to_string(), false);
                ec_ref.insert(event.to_string(), h);
            }
        }
    });
}

pub fn event_check(event: &str, func: &str) -> bool {
    EVENT_CENTER.with(|ec| {
        let mut ec_ref = ec.borrow_mut();
        if let Some(ht) = ec_ref.get_mut(event) { 
            if let Some(flag) = ht.get_mut(func) {
                if *flag {
                    *flag = false;
                    return true;
                }
            } 
        }
        false
    })
}

pub fn event_emit(event: &str) {
    EVENT_CENTER.with(|ec| {
        let mut ec_ref = ec.borrow_mut();
        if let Some(ht) = ec_ref.get_mut(event) {
            for value in ht.values_mut() {
                if !(*value) {
                    *value = true;
                }
            }
        }
    });
}

pub fn timer_register(name: &str, time: f32, func: &str) {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().register(name, time, func);
    });
}

pub fn timer_set_time(name: &str, time: f32) {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().set_time(name, time);
    });
}

pub fn timer_stage(name: &str) -> u32 {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().stage(name)
    })
}

pub fn timer_rstage(name: &str) -> u32 {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().rstage(name)
    })
}

pub fn timer_percent(name: &str) -> f32 {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().percent(name)
    })
}

pub fn timer_exdata(name: &str) -> Option<Vec<u8>> {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().exdata(name)
    })
}

pub fn timer_fire<T>(name: &str, value: T)
where
    T: Serialize,
{
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().fire(name, value);
    });
}

pub fn timer_cancel(name: &str, nall: bool) {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().cancel(name, nall);
    });
}

pub fn timer_update() {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().update();
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_thread_local_implementation() {
        // Test event registration and triggering
        event_register("test_event_tl", "test_callback");
        
        // Test event check (initially should be false)
        assert!(!event_check("test_event_tl", "test_callback"));
        
        // Trigger event
        event_emit("test_event_tl");
        
        // Now should be true
        assert!(event_check("test_event_tl", "test_callback"));
        
        // Check again should be false (because it has been consumed)
        assert!(!event_check("test_event_tl", "test_callback"));
    }
    
    #[test]
    fn test_timer_implementation() {
        // Test timer registration
        timer_register("test_timer_tl", 1.0, "timer_callback");
        
        // Test initial state
        assert_eq!(timer_stage("test_timer_tl"), 0);
        assert_eq!(timer_rstage("test_timer_tl"), 60); // 1.0 * GAME_FRAME
        assert_eq!(timer_percent("test_timer_tl"), 0.0);
        
        // Test time setting
        timer_set_time("test_timer_tl", 2.0);
        assert_eq!(timer_rstage("test_timer_tl"), 120); // 2.0 * GAME_FRAME
        
        // Test data storage
        timer_fire("test_timer_tl", "test_data");
        let exdata = timer_exdata("test_timer_tl");
        assert!(exdata.is_some());
    }
    
    #[test]
    fn benchmark_thread_local_performance() {
        let iterations = 10000;
        
        // Event operation benchmark
        let start = Instant::now();
        for i in 0..iterations {
            let event_name = format!("bench_event_{}", i % 10);
            let callback_name = format!("bench_callback_{}", i % 10);
            
            event_register(&event_name, &callback_name);
            event_emit(&event_name);
            event_check(&event_name, &callback_name);
        }
        let event_time = start.elapsed();
        
        // Timer operation benchmark
        let start = Instant::now();
        for i in 0..iterations {
            let timer_name = format!("bench_timer_{}", i % 10);
            let callback_name = format!("bench_callback_{}", i % 10);
            
            timer_register(&timer_name, 0.1, &callback_name);
            timer_stage(&timer_name);
            timer_percent(&timer_name);
        }
        let timer_time = start.elapsed();
        
        println!("Thread-local version:");
        println!("  Event operations {} times took: {:?}", iterations, event_time);
        println!("  Timer operations {} times took: {:?}", iterations, timer_time);
        println!("  Average per event operation: {:?}", event_time / iterations);
        println!("  Average per timer operation: {:?}", timer_time / iterations);
    }
}
