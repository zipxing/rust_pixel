use log::info;
use rust_pixel::event::Event;
use rust_pixel::{context::Context, event::event_emit, game::Model, util::Rand};
use texas_lib::*;

pub const CARDW: usize = 7;
#[cfg(any(feature = "sdl", feature = "wgpu", feature = "winit", target_arch = "wasm32"))]
pub const CARDH: usize = 7;
#[cfg(not(any(feature = "sdl", feature = "wgpu", feature = "winit", target_arch = "wasm32")))]
pub const CARDH: usize = 5;

// enum PokerState {
//     Normal,
//     OverSelf,
//     OverBorder,
// }

pub struct PokerModel {
    pub rand: Rand,
    pub texas_cards_red: TexasCards,
    pub texas_cards_black: TexasCards,
    pub pool: Vec<u16>,
}

impl PokerModel {
    pub fn new() -> Self {
        Self {
            rand: Rand::new(),
            texas_cards_red: TexasCards::new(),
            texas_cards_black: TexasCards::new(),
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

impl Model for PokerModel {
    fn init(&mut self, _context: &mut Context) {
        self.rand.srand_now();
        self.shuffle_tiles();
        self.texas_cards_red.assign(&self.pool[0..5]).unwrap();
        self.texas_cards_black.assign(&self.pool[5..10]).unwrap();
        info!("red:{}", self.texas_cards_red);
        info!("black:{}", self.texas_cards_black);
        event_emit("Poker.RedrawTile");
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
