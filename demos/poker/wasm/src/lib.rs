// use poker_lib::{Counter, PokerCard, PokerCards, Suit};
// use texas_lib::{TexasCards, TexasType};
use gin_rummy_lib::cards::GinRummyCards;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct WasmGinRummy {
    gcs: GinRummyCards,
    webbuf: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl WasmGinRummy {
    // js调用创建game结构
    pub fn new() -> Self {
       let gcs = GinRummyCards::new();
       Self {
           gcs,
           webbuf: vec![],
       }
    }
    
    pub fn sort(&mut self) {
        self.gcs.sort(); 
        // 有效的out数据格式：
        // suit长度 card1 card2...
        // number长度 card1 card2...
        self.webbuf.clear();
        self.webbuf.push(self.gcs.cards.cards.len() as u8);
        for v in &self.gcs.sort_cards_suit {
            self.webbuf.push(v.to_u8());
        }
        self.webbuf.push(self.gcs.cards.cards.len() as u8);
        for v in &self.gcs.sort_cards_number {
            self.webbuf.push(v.to_u8());
        }
    }

    pub fn assign(&mut self, arr: js_sys::Uint16Array, freeze: u8) {
        self.webbuf.clear();
        let cv = arr.to_vec();
        match self.gcs.assign(&cv, freeze != 0) {
            Ok(n) => {
                // 有效的out数据格式：
                // deadwood分数
                // deadwood长度 deadwood1 deadwood2 ...
                // meld1长度 meld1_1 meld1_2 ...
                // meld2长度 meld2_1 meld2_2...
                // ...
                // 长度32足够了
                // best deadwood value...
                self.webbuf.push(n);
                // best deadwood list...
                self.webbuf.push(self.gcs.best_deadwood.len() as u8);
                for p in &self.gcs.best_deadwood {
                    self.webbuf.push(p.to_u8());
                }
                // melds list...
                for v in &self.gcs.best_melds {
                    self.webbuf.push(v.len() as u8);
                    for p in v {
                        self.webbuf.push(p.to_u8());
                    }
                }
            }
            Err(_) => {
                self.webbuf.push(255);
            }
        }
    }

    pub fn web_buffer_len(&self) -> usize {
        self.webbuf.len()
    }

    pub fn web_buffer(&self) -> *const u8 {
        self.webbuf.as_slice().as_ptr()
    }
}

