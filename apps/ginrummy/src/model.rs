use ginrummy_lib::cards::*;
use log::info;
use rust_pixel::event::Event;
use rust_pixel::{context::Context, event::event_emit, game::Model, util::Rand};

pub const CARDW: usize = 7;
#[cfg(graphics_mode)]
pub const CARDH: usize = 7;
#[cfg(not(graphics_mode))]
pub const CARDH: usize = 5;

// enum GinRummyState {
//     Normal,
//     OverSelf,
//     OverBorder,
// }

pub struct GinRummyModel {
    pub rand: Rand,
    pub cards_a: GinRummyCards,
    pub cards_b: GinRummyCards,
    pub pool: Vec<u16>,
}

impl GinRummyModel {
    pub fn new() -> Self {
        Self {
            rand: Rand::new(),
            cards_a: GinRummyCards::new(),
            cards_b: GinRummyCards::new(),
            pool: vec![],
        }
    }

    pub fn shuffle_tiles(&mut self) {
        self.pool.clear();
        for i in 1..=52u16 {
            self.pool.push(i);
        }
        self.rand.shuffle(&mut self.pool);
    }

    // pub fn act(&mut self, _d: Dir, _context: &mut Context) {}
}

impl Model for GinRummyModel {
    fn init(&mut self, _context: &mut Context) {
        self.rand.srand_now();
        for _i in 0..500000000 {
            self.shuffle_tiles();
            // ac ad as 4c 5c 6c 7c qd 10h 4h
            // let _ = self
            //     .cards_a
            //     .assign(&vec![1, 27, 40, 30, 32, 33, 34, 51, 23, 17]);
            self.cards_a.assign(&self.pool[0..10], false).unwrap();
            // self.cards_b.assign(&self.pool[10..20], false).unwrap();
            self.cards_b
                .assign(&vec![2, 15, 28, 3, 4, 5, 17, 18, 31, 23], true)
                .unwrap();
            if self.cards_a.best < 10 && self.cards_b.best < 50 {
                self.cards_a.sort();
                self.cards_b.sort();
                info!(
                    "{:?} best...{:?}, best_melds...{:?}, deadwood...{:?}",
                    _i, self.cards_a.best, self.cards_a.best_melds, self.cards_a.best_deadwood
                );
                info!(
                    "sort_suit..len{}..{:?}",
                    self.cards_a.sort_cards_suit.len(),
                    self.cards_a.sort_cards_suit
                );
                info!(
                    "sort_number..len{}..{:?}",
                    self.cards_a.sort_cards_number.len(),
                    self.cards_a.sort_cards_number
                );
                info!(
                    "{:?} best...{:?}, best_melds...{:?}, deadwood...{:?}",
                    _i, self.cards_b.best, self.cards_b.best_melds, self.cards_b.best_deadwood
                );
                break;
            }
        }
        event_emit("GinRummy.RedrawTile");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(_key) => {}
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {}

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
