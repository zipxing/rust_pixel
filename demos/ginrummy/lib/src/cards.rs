// use log::info;
use itertools::Itertools;
use poker_lib::{PokerCard, PokerCards};
use std::collections::HashSet;

// 3张或4张
// 3 or 4 cards
fn is_number_meld(pcs: &Vec<&PokerCard>) -> bool {
    let pl = pcs.len();
    if pl != 3 && pl != 4 {
        return false;
    }
    let n = pcs[0].number;
    for i in 1..pl {
        if pcs[i].number != n {
            return false;
        }
    }
    true
}

// 同花顺
// Flush
fn is_suit_meld(pcs: &Vec<&PokerCard>) -> bool {
    let pl = pcs.len();
    if pl < 3 {
        return false;
    }
    let s = pcs[0].suit;
    let n = pcs[0].number;
    let mut ns = vec![];
    ns.push(n as i8);
    for i in 1..pl {
        if pcs[i].suit != s {
            return false;
        }
        ns.push(pcs[i].number as i8);
    }
    ns.sort();
    for i in 1..pl {
        if ns[i] - ns[i - 1] != 1 {
            return false;
        }
    }
    true
}

// 是否能共存
// has conflicts?
fn is_conflict(pc: &Vec<&Vec<&PokerCard>>) -> bool {
    let mut bucket: [u8; 53] = [0; 53];
    for v in pc {
        for p in *v {
            let idx = p.to_u8() as usize;
            if bucket[idx] != 0 {
                return true;
            } else {
                bucket[idx] = 1;
            }
        }
    }
    false
}

// 计算deadwood和分数
// calc deadwood and score
fn deadwood(pc: &PokerCards, ms: &Vec<&Vec<&PokerCard>>) -> (u8, Vec<PokerCard>) {
    let mut pcset: HashSet<u8> = pc.cards.iter().map(|x| x.to_u8()).collect();
    let mut pcs: Vec<PokerCard> = vec![];
    for v in ms {
        for p in *v {
            pcset.remove(&p.to_u8());
        }
    }
    let mut val = 0u8;
    for p in &pcset {
        let pk = PokerCard::from_u8(*p).unwrap();
        pcs.push(pk);
        val += if pk.number > 10 { 10 } else { pk.number };
    }
    // info!("deadwood...{:?} {}", pcset, val);
    (val, pcs)
}

// 获取所有可能的组合，3张，4张，3同花顺，4同花顺，5同花顺
// 更长的同花顺可以通过3，4，5组合出来，所以先不用考虑
// get all possible combos, 3cards, 4cards, 3Flush, 4Flush, 5Flush
// longer flush can be combined using 3,4,5Flush, so ignore for now
fn get_all_melds(pc: &PokerCards) -> Vec<Vec<&PokerCard>> {
    let mut am = vec![];
    for cn in 3..=5 {
        for meld in pc.cards.iter().combinations(cn) {
            if is_number_meld(&meld) || is_suit_meld(&meld) {
                am.push(meld);
            }
        }
    }
    am
}

// 不准变牌的顺序，
// 获取所有可能的组合，3张，4张，3同花顺，4同花顺，5同花顺
// 更长的同花顺可以通过3，4，5组合出来，所以先不用考虑
// get all possible combos, 3cards, 4cards, 3Flush, 4Flush, 5Flush
// longer flush can be combined using 3,4,5Flush, so ignore for now
fn get_all_melds_freeze(pc: &PokerCards) -> Vec<Vec<&PokerCard>> {
    let mut am = vec![];
    let pclen = pc.cards.len();
    for cn in 3..=5 {
        for i in 0..=pclen - cn {
            let mut meld = vec![];
            for j in 0..cn {
                meld.push(&pc.cards[i + j]);
            }
            if is_number_meld(&meld) || is_suit_meld(&meld) {
                am.push(meld);
            }
        }
    }
    am
}

pub struct GinRummyCards {
    pub cards: PokerCards,
    pub sort_cards_suit: Vec<PokerCard>,
    pub sort_cards_number: Vec<PokerCard>,
    pub best: u8,
    pub best_deadwood: Vec<PokerCard>,
    pub best_melds: Vec<Vec<PokerCard>>,
}

impl GinRummyCards {
    pub fn new() -> Self {
        Self {
            cards: PokerCards::new(),
            best: 0,
            sort_cards_suit: vec![],
            sort_cards_number: vec![],
            best_deadwood: vec![],
            best_melds: vec![],
        }
    }

    // freeze为true时，不准改变手牌顺序，用于关闭自动排序时确定meld
    // freeze为false，用于自动排序
    // freeze=true means can not change hand order, used for get meld when auto sorting is turned off
    // freeze=false used for auto sorting
    pub fn assign(&mut self, cards: &[u16], freeze: bool) -> Result<u8, String> {
        if cards.len() != 10 && cards.len() != 11 {
            return Err(String::from(format!(
                "error cards length...{}",
                cards.len()
            )));
        }
        let mut bucket: [u8; 53] = [0; 53];
        for v in cards {
            let idx = (v - 1) as usize;
            if bucket[idx] != 0 {
                return Err(String::from(format!("card not distinct...")));
            } else {
                bucket[idx] = 1;
            }
        }
        self.cards.assign(cards)?;
        self.get_best_deadwood(freeze);
        Ok(self.best)
    }

    pub fn sort(&mut self) {
        self.sort_cards_suit.clear();
        self.sort_cards_number.clear();
        for v in &self.cards.cards {
            self.sort_cards_suit.push(*v);
            self.sort_cards_number.push(*v);
        }
        self.sort_cards_suit.sort_by(|a, b| {
            (a.suit as u16 * 100 + a.number as u16).cmp(&(b.suit as u16 * 100 + b.number as u16))
        });
        self.sort_cards_number.sort_by(|a, b| {
            (a.number as u16 * 100 + a.suit as u16).cmp(&(b.number as u16 * 100 + b.suit as u16))
        });
    }

    // 遍历所有组合，不冲突则算deadwood，找到最优解
    // freeze为true时，不准改变手牌顺序，用于关闭自动排序时确定meld
    // freeze为false，用于自动排序
    // iterate through all possible combos, no conflict means deadwood, optimal solution found
    // freeze=true means can not change hand order, used for get meld when auto sorting is turned off
    // freeze=false used for auto sorting
    pub fn get_best_deadwood(&mut self, freeze: bool) {
        self.best = 0;
        self.best_deadwood = vec![];
        self.best_melds = vec![];
        let am = if freeze {
            get_all_melds_freeze(&self.cards)
        } else {
            get_all_melds(&self.cards)
        };
        let dw = deadwood(&self.cards, &vec![]);
        let mut best = dw.0;
        let mut bestvp = vec![];
        let mut bestdw = dw.1;
        let amlen = am.len();
        for cn in 1..=amlen {
            for vp in am.iter().combinations(cn) {
                if !is_conflict(&vp) {
                    // info!("com...{:?}", vp);
                    let dw = deadwood(&self.cards, &vp);
                    if dw.0 < best {
                        best = dw.0;
                        bestvp = vp;
                        bestdw = dw.1;
                    }
                }
            }
        }
        for v in &bestvp {
            let mut meld: Vec<PokerCard> = vec![];
            for p in *v {
                meld.push(**p);
            }
            self.best_melds.push(meld);
        }
        self.best = best;
        self.best_deadwood = bestdw.clone();
    }
}
