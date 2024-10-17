#![allow(dead_code)]

use crate::Suit::*;
use std::fmt::{self, Display, Formatter};
use std::ops::{Index, IndexMut};

//多处用到, 由花色和点数合成牌ID, 封成一个宏
//用宏还有一个好处，可以用as强制转换类型
//方便的接受多种类型参数
#[macro_export]
macro_rules! sn2poker {
    ($suit:expr, $num:expr) => {
        PokerCard::from_suit_num($suit as u8, $num as u8)
    };
}

#[repr(C)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum Suit {
    Spade = 0,
    Heart = 1,
    Club = 2,
    Diamond = 3,
    Joker = 4,
}

impl Suit {
    pub fn from_u8(v: u8) -> Result<Self, String> {
        let sv = match v {
            0 => Suit::Spade,
            1 => Suit::Heart,
            2 => Suit::Club,
            3 => Suit::Diamond,
            4 => Suit::Joker,
            _ => {
                return Err(String::from(format!("invaild suit:{:?}", v)));
            }
        };
        Ok(sv)
    }

    pub fn to_string(&self) -> &str {
        match self {
            Suit::Spade => "♠",
            Suit::Heart => "♥",
            Suit::Club => "♣",
            Suit::Diamond => "♦",
            Suit::Joker => "J",
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Counter {
    pub t: Suit,
    pub n: u8,
    // 存放某个点数的牌有几张
    pub bucket: [u8; 14],
}

impl Counter {
    pub fn new(cs: Suit) -> Self {
        Self {
            t: cs,
            n: 0,
            bucket: [0; 14],
        }
    }

    pub fn reset(&mut self) {
        self.n = 0;
        let _ = &self.bucket[..].fill(0);
    }

    fn add(&mut self, num: u8, one: u8) {
        self.n += 1;
        self.bucket[num as usize] += one;
    }
}

impl Display for Counter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let tt = ["♠", "♥", "♣", "♦", "J"];
        write!(f, "{{{}{} ", tt[self.t as usize], self.n).unwrap();
        for i in 0..13 {
            write!(f, "{}", self.bucket[i + 1]).unwrap();
        }
        write!(f, "}}")
    }
}

pub struct PokerCards {
    pub cards: Vec<PokerCard>,
    pub counters: [Counter; 5],
    pub counter_all_without_joker: Counter,
}

impl Index<Suit> for PokerCards {
    type Output = Counter;
    fn index(&self, index: Suit) -> &Self::Output {
        &self.counters[index as usize]
    }
}

impl IndexMut<Suit> for PokerCards {
    fn index_mut(&mut self, index: Suit) -> &mut Self::Output {
        &mut self.counters[index as usize]
    }
}

impl PokerCards {
    pub fn new() -> Self {
        Self {
            cards: vec![],
            counters: [
                Counter::new(Spade),
                Counter::new(Heart),
                Counter::new(Club),
                Counter::new(Diamond),
                Counter::new(Joker),
            ],
            counter_all_without_joker: Counter::new(Joker),
        }
    }

    pub fn reset(&mut self) {
        self.cards.clear();
        for i in 0..5 {
            self.counters[i].reset();
        }
        self.counter_all_without_joker.reset();
    }

    pub fn assign(&mut self, vcard: &[u16]) -> Result<u8, String> {
        self.cards.clear();
        let cl = vcard.len();
        for i in 0..cl {
            let c = if vcard[i] < 100 {
                PokerCard::from_u8(vcard[i] as u8)?
            } else {
                PokerCard::from_u16(vcard[i])?
            };
            self.cards.push(c);
        }
        self.count_cards(&1);
        Ok(cl as u8)
    }

    pub fn assign_by_cards(&mut self, vcard: &Vec<PokerCard>) -> Result<u8, String> {
        self.cards.clear();
        let cl = vcard.len();
        for i in 0..cl {
            self.cards.push(vcard[i]);
        }
        self.count_cards(&1);
        Ok(cl as u8)
    }

    pub fn count_cards(&mut self, one: &u8) {
        for i in 0..5usize {
            self.counters[i].reset();
        }
        self.counter_all_without_joker.reset();
        let cl = self.cards.len();
        for i in 0..cl {
            let t = self.cards[i as usize];
            let mt = t.get_suit_num();
            self.counters[mt.0 as usize].add(mt.1, *one);
            self.counter_all_without_joker.add(mt.1, *one);
        }
    }

    pub fn get_suit_cards(&self, suit: Suit) -> Vec<u16> {
        let mut vc: Vec<u16> = vec![];
        for i in 1..=13 {
            if suit == Suit::Spade || suit == Suit::Joker {
                if self[Suit::Spade].bucket[i] != 0 {
                    vc.push(((Suit::Spade as u8 * 13) + i as u8) as u16);
                }
                if self[Suit::Joker].bucket[i] != 0 {
                    vc.push(((Suit::Joker as u8 * 13) + i as u8) as u16);
                }
            } else {
                if self[suit].bucket[i] != 0 {
                    vc.push(((suit as u8 * 13) + i as u8) as u16);
                }
            }
        }
        vc
    }
    pub fn get_suit_pocker_cards(&self, suit: Suit) -> Vec<PokerCard> {
        let mut vc: Vec<PokerCard> = vec![];
        for cards in &self.cards {
            let ct = cards.suit;
            if (suit == Suit::Spade || suit == Suit::Joker)
                && (ct == Suit::Spade || ct == Suit::Joker)
                || ct == suit
            {
                vc.push(*cards);
            }
        }
        vc
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn clear(&mut self) {
        let vc: Vec<u16> = vec![];
        self.assign(&vc).unwrap();
    }

    pub fn copy_from(&mut self, pcs: &PokerCards) {
        self.cards.clear();
        for c in &pcs.cards {
            self.cards.push(*c);
        }
        self.count_cards(&1);
    }

    pub fn add(&mut self, c: PokerCard) {
        self.cards.push(c);
        self.count_cards(&1);
    }

    pub fn remove(&mut self, c: PokerCard) {
        if let Some(pos) = self.cards.iter().position(|x| *x == c) {
            self.cards.remove(pos);
            self.count_cards(&1);
        }
    }

    pub fn contain(&self, c: PokerCard) -> bool {
        if let Some(_pos) = self.cards.iter().position(|x| *x == c) {
            return true;
        }
        false
    }
}

impl Display for PokerCards {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}\n{} {} {} {} {} {}\n",
            self.cards,
            self.counters[0],
            self.counters[1],
            self.counters[2],
            self.counters[3],
            self.counters[4],
            self.counter_all_without_joker,
        )
        //write!(f, "{:?}", self.cards)
    }
}

impl fmt::Debug for PokerCards {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[repr(C)]
#[derive(Ord, PartialOrd, Eq, Copy, Clone, PartialEq)]
//黑桃,红心,草花,方片
pub struct PokerCard {
    pub suit: Suit,
    pub number: u8,
}

impl PokerCard {
    //54张牌用1～54序号表示
    pub fn from_u8(v: u8) -> Result<Self, String> {
        let suit;
        let number;
        match v {
            53 => {
                suit = Joker;
                number = 1;
            }
            54 => {
                suit = Joker;
                number = 2;
            }
            1..=52 => {
                number = (v - 1) % 13 + 1;
                suit = Suit::from_u8((v - 1) / 13).unwrap();
            }
            _ => return Err(String::from(format!("invaild cards:{:?}", v))),
        }
        Ok(PokerCard { suit, number })
    }

    //百位表示花色,余数表示点数,对人类友好
    //例如：101表示黑桃A，501表示小王，502表示大王
    pub fn from_u16(v: u16) -> Result<Self, String> {
        match v {
            1..=54 => Self::from_u8(v as u8),
            101..=113 | 201..=213 | 301..=313 | 401..=413 | 501..=502 => {
                Self::from_u8(((v / 100 - 1) * 13) as u8 + (v % 100) as u8)
            }
            _ => Err(String::from(format!("invaild card from_u16 num:{:?}", v))),
        }
    }

    //适配spades服务器的牌值
    //102表示黑桃2, 114表示黑桃A, 115小王，116大王
    //202表示红桃2，214表示红桃A
    //牌的范围102～414
    pub fn from_spades_n(v: u16) -> Result<Self, String> {
        match v {
            1..=54 => Self::from_u8(v as u8),
            102..=114 | 202..=214 | 302..=314 | 402..=414 => {
                let s = (v / 100 - 1) * 13;
                let mut n = v % 100;
                n = if n == 14 { 1 } else { n };
                Self::from_u8(s as u8 + n as u8)
            }
            115 => Self::from_u8(53),
            116 => Self::from_u8(54),
            _ => {
                return Err(String::from(format!(
                    "invaild card from_spades_n num:{:?}",
                    v
                )));
            }
        }
    }

    //转回 spades 服务器需要的值
    pub fn to_spades_n(&self) -> i16 {
        let mut n = self.number as i16;
        n = if n == 1 { 14 } else { n };
        match self.suit {
            Spade => 100 + n,
            Heart => 200 + n,
            Club => 300 + n,
            Diamond => 400 + n,
            Joker => 114 + self.number as i16,
        }
    }

    pub fn from_suit_num(t: u8, n: u8) -> Result<Self, String> {
        if n > 14 || n < 1 || t > 4 {
            return Err(String::from(format!("invaild card num:{:?}", n)));
        }
        let cn = if n == 14 { 1 } else { n };
        Self::from_u8(t * 13 + cn)
    }

    pub fn to_u8(&self) -> u8 {
        let n = self.number;
        match self.suit {
            Spade => n,
            Heart => 13 + n,
            Club => 26 + n,
            Diamond => 39 + n,
            Joker => 52 + n,
        }
    }

    pub fn get_suit_num(&self) -> (u8, u8) {
        let n = self.number;
        match self.suit {
            Spade => (0, n),
            Heart => (1, n),
            Club => (2, n),
            Diamond => (3, n),
            Joker => (4, n),
        }
    }

    pub fn get_suit(&self) -> u8 {
        let sn = self.get_suit_num();
        sn.0
    }

    pub fn get_number(&self) -> i16 {
        let sn = self.get_suit_num();
        let num = sn.1 as i16;
        if sn.0 == 4 {
            //王牌 rank 按  15 16 算
            return num + 14;
        }
        if num == 1 {
            //尖 按14算
            return 14;
        }
        num
    }
    pub fn is_trump_card(&self) -> bool {
        self.suit == Suit::Spade || self.suit == Suit::Joker
    }
}

impl Display for PokerCard {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let tt = ["♠", "♥", "♣", "♦", "🎩"];
        let nn = [
            "", "A", "2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K",
        ];
        let (t, n) = self.get_suit_num();
        write!(f, "{} {} {}", self.to_u8(), tt[t as usize], nn[n as usize])
    }
}

impl fmt::Debug for PokerCard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let result = PokerCard::from_u8(2 + 2).unwrap();
        let (t, n) = result.get_suit_num();
        assert_eq!(n, 4);
        assert_eq!(t, 0);
    }
}
