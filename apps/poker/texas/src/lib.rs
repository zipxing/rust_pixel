#![allow(dead_code)]

use itertools::Itertools;
// use log::info;
use poker_lib::{sn2poker, PokerCard};
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use TexasType::*;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TexasType {
    NoCalc,
    HighCard,
    OnePair,
    TwoPair,
    Three,
    Straight,
    Flush,
    FullHouse,
    Four,
    StraightFlush,
    RoyalFlush,
}

#[derive(Debug)]
pub struct TexasCards {
    pub cards: Vec<PokerCard>,
    pub best: Vec<PokerCard>,
    pub texas: TexasType,
    pub score: u64,
    count_suit: [Vec<u8>; 4],
    count_num: [Vec<u8>; 15],
    order_by_count: Vec<(u8, u8)>,
    nums_uniq: Vec<u8>,
}

impl Display for TexasCards {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{:?} {:x} {} / {} / {} / {} / {}",
            self.texas,
            self.score,
            self.best[0],
            self.best[1],
            self.best[2],
            self.best[3],
            self.best[4],
        )
    }
}

impl TexasCards {
    pub fn new() -> Self {
        Self {
            cards: vec![],
            count_suit: Default::default(),
            count_num: Default::default(),
            order_by_count: Default::default(),
            nums_uniq: Default::default(),
            best: vec![],
            texas: NoCalc,
            score: 0,
        }
    }

    fn reset(&mut self) {
        self.cards.clear();
        for i in 0..4 {
            self.count_suit[i].clear();
        }
        for i in 0..15 {
            self.count_num[i].clear();
        }
        self.order_by_count.clear();
        self.nums_uniq.clear();
        self.best.clear();
        self.texas = NoCalc;
        self.score = 0;
    }

    pub fn assign(&mut self, cards: &[u16]) -> Result<u8, String> {
        self.reset();
        let ccount = cards.len();
        if ccount < 5 || ccount > 7 {
            return Err(format!("cards length {} not in [5~7]", ccount));
        }
        if cards.iter().unique().collect::<Vec<_>>().len() != ccount {
            return Err(format!("cards not unique {:?}", cards));
        }
        //按花色和点数统计，并整理出去重点数列表nums_uniq
        for i in 0..ccount {
            let c = if cards[i] < 100 {
                PokerCard::from_u8(cards[i] as u8)?
            } else {
                PokerCard::from_u16(cards[i])?
            };
            let (t, n) = c.get_suit_num();
            let cn = if n == 1 { 14 } else { n };
            //counter中1被转成了14
            self.count_suit[t as usize].push(cn);
            self.count_num[cn as usize].push(t);
            self.cards.push(c);
        }
        for i in 0..4 {
            self.count_suit[i].sort();
            self.count_suit[i].reverse();
        }
        for i in 0..15 {
            if self.count_num[i].len() > 0 {
                self.nums_uniq.push(i as u8);
                self.order_by_count
                    .push((self.count_num[i].len() as u8, i as u8));
            }
        }
        self.order_by_count.sort_by_key(|x| x.0);
        self.order_by_count.reverse();

        //计算牌型和分数
        self.calc_best();
        self.calc_score();

        // info!("{}", self);
        Ok(self.cards.len() as u8)
    }

    //返回0表示无顺子,14表示TJQKA,5表示A2345
    //其他返回顺子最大牌点
    fn find_max_seq(&self, nums: &[u8]) -> u8 {
        //去重排序
        //注意送进来的同花色和全局两种情况都已经去重了
        //这里的unique可以省略
        let ns = nums.iter().sorted().unique().collect::<Vec<_>>();

        //用索引-牌点进行分组,得到所有的连续牌
        //例:[1,3,4,5,7,8,9,T,J] -> 1 345 789TJ
        let s = ns
            .iter()
            .enumerate()
            .group_by(|i| (*i).0 as i32 - **((*i).1) as i32);

        //遍历找到最大的5张顺
        let mut smax: u8 = 0;
        for (_, g) in &s {
            let ps = g.map(|x| x.1).collect::<Vec<_>>();
            let maxp = **ps[ps.len() - 1];
            let maxn = *ns[ns.len() - 1];
            //5432A
            if ps.len() == 4 && maxp == 5 && maxn == 14 {
                return 5;
            }
            if ps.len() >= 5 && maxp > smax {
                smax = maxp;
            }
        }
        smax
    }

    fn push_best(&mut self, color: u8, num: u8) {
        match sn2poker!(color, num) {
            Ok(c) => self.best.push(c),
            Err(_) => (),
        }
    }

    //按从大到小补充剩下的牌，凑够5张best
    fn fill_best(&mut self) {
        let cb: HashSet<u8> = HashSet::from_iter(self.best.iter().map(|x| x.to_u8()));
        let mut fill_count = 5 - self.best.len();
        for i in 0..15 {
            for color in &self.count_num[15 - i - 1] {
                match sn2poker!(*color, 15 - i - 1) {
                    Ok(uc) => {
                        if !cb.contains(&uc.to_u8()) {
                            self.best.push(uc);
                            fill_count -= 1;
                            if fill_count == 0 {
                                return;
                            }
                        }
                    }
                    Err(_) => (),
                }
            }
        }
    }

    //计算分数用于比较牌型大小，最高位是牌型，后面跟5张牌的
    pub fn calc_score(&mut self) {
        self.score = (self.texas as u64) << (5 * 6);
        for b in 0..5 {
            let (s, bn) = self.best[b].get_suit_num();
            let n = if bn == 1 { 14 } else { bn };
            let nc = n as u64 + (((3 - s) as u64) << 4); 
            // println!("{} {}#####...{:6b}", s, n, nc);
            self.score += nc << ((4 - b) * 6);
        }
    }

    //分析牌型，填充best...
    pub fn calc_best(&mut self) {
        for suit in 0..4 {
            let i = suit as usize;
            if self.count_suit[i].len() >= 5 {
                let smax = self.find_max_seq(&self.count_suit[i]);
                if smax == 14 {
                    self.texas = RoyalFlush;
                    for b in 0..5 {
                        self.push_best(suit, 14 - b);
                    }
                    return;
                } else if smax > 0 {
                    self.texas = StraightFlush;
                    for b in 0..5 {
                        self.push_best(suit, smax - b);
                    }
                    return;
                } else {
                    self.texas = Flush;
                    for b in 0..5 {
                        self.push_best(suit, self.count_suit[i][b]);
                    }
                    return;
                }
            }
        }
        if self.order_by_count[0].0 == 4 {
            self.texas = Four;
            for suit in 0..4 {
                self.push_best(suit, self.order_by_count[0].1);
            }
            self.fill_best();
            return;
        } else if self.order_by_count[0].0 == 3 && self.order_by_count[1].0 >= 2 {
            self.texas = FullHouse;
            let a = self.order_by_count[0].1;
            let b = self.order_by_count[1].1;
            for i in 0..3 {
                self.push_best(self.count_num[a as usize][i], a);
            }
            for i in 0..2 {
                self.push_best(self.count_num[b as usize][i], b);
            }
            return;
        }
        let smax = self.find_max_seq(&self.nums_uniq);
        if smax > 0 {
            self.texas = Straight;
            for b in 0..5 {
                let n = smax - b;
                let bn = if n == 1 { 14 } else { n };
                self.push_best(self.count_num[bn as usize][0], bn);
            }
            return;
        }
        if self.order_by_count[0].0 == 3 {
            self.texas = Three;
            let a = self.order_by_count[0].1;
            for i in 0..3 {
                self.push_best(self.count_num[a as usize][i], a);
            }
            self.fill_best();
            return;
        }
        if self.order_by_count[0].0 == 2 && self.order_by_count[1].0 == 2 {
            self.texas = TwoPair;
            let a = self.order_by_count[0].1;
            let b = self.order_by_count[1].1;
            for i in 0..2 {
                self.push_best(self.count_num[a as usize][i], a);
            }
            for i in 0..2 {
                self.push_best(self.count_num[b as usize][i], b);
            }
            self.fill_best();
            return;
        }
        if self.order_by_count[0].0 == 2 {
            self.texas = OnePair;
            let a = self.order_by_count[0].1;
            for i in 0..2 {
                self.push_best(self.count_num[a as usize][i], a);
            }
            self.fill_best();
            return;
        }
        self.texas = HighCard;
        self.fill_best();
        // return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let mut tc = TexasCards::new();
        //皇家同花顺
        tc.assign(&vec![1, 10, 11, 12, 13, 6, 8]).unwrap();
        assert_eq!(tc.texas, RoyalFlush);
        //同花顺
        tc.assign(&vec![1, 2, 3, 4, 5, 6 + 13, 8 + 13]).unwrap();
        assert_eq!(tc.texas, StraightFlush);
        //四条
        tc.assign(&vec![1, 1 + 13, 1 + 13 * 2, 1 + 13 * 3, 13, 6, 7])
            .unwrap();
        assert_eq!(tc.texas, Four);
        //福禄
        tc.assign(&vec![
            1,
            1 + 13,
            1 + 13 * 2,
            13 + 13 * 3,
            13,
            13 + 13 * 2,
            7,
        ])
        .unwrap();
        assert_eq!(tc.texas, FullHouse);
        //同花
        tc.assign(&vec![9, 10, 5, 12, 13, 6, 7]).unwrap();
        assert_eq!(tc.texas, Flush);
        //顺子
        //tc.assign(&vec!(9,10,11+13,12,13,6+13,7+13*2)).unwrap();
        tc.assign(&vec![1 + 13, 2 + 13, 3, 4, 5, 7 + 13, 8 + 13 * 2])
            .unwrap();
        assert_eq!(tc.texas, Straight);
        //三条
        tc.assign(&vec![9, 9 + 13, 9 + 13 * 3, 12, 13, 6 + 13, 7 + 13 * 2])
            .unwrap();
        assert_eq!(tc.texas, Three);
        //两对
        tc.assign(&vec![9, 9 + 13, 12 + 13 * 2, 12, 13, 6 + 13, 7 + 13 * 2])
            .unwrap();
        assert_eq!(tc.texas, TwoPair);
        //一对
        tc.assign(&vec![9, 9 + 13, 1 + 13 * 2, 12, 13, 6 + 13, 7 + 13 * 2])
            .unwrap();
        assert_eq!(tc.texas, OnePair);
        //高牌
        tc.assign(&vec![1, 9 + 13, 2 + 13 * 2, 12, 13, 6 + 13, 7 + 13 * 2])
            .unwrap();
        assert_eq!(tc.texas, HighCard);
    }
}
