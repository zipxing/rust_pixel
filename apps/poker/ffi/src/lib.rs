// We have a lot of c-types in here, stop warning about their names!
#![allow(non_camel_case_types)]

use poker_lib::{Counter, PokerCard, PokerCards, Suit};
use texas_lib::{TexasCards, TexasType};
use gin_rummy_lib::cards::GinRummyCards;

#[no_mangle]
pub extern "C" fn rs_GinRummyCards_new() -> *mut GinRummyCards {
    let gcs = GinRummyCards::new();
    Box::into_raw(Box::new(gcs))
}

#[no_mangle]
pub extern "C" fn rs_GinRummyCards_free(p_pcs: *mut GinRummyCards) {
    if !p_pcs.is_null() {
        unsafe {
            let _ = Box::from_raw(p_pcs);
        };
    }
}

#[no_mangle]
pub extern "C" fn rs_GinRummyCards_sort(
    p_pcs: *mut GinRummyCards,
    p_out: *mut u8,
) -> i8 {
    if p_pcs.is_null() {
        return -1;
    }
    let ret: i8;
    // 取结构
    let mut ps = unsafe { Box::from_raw(p_pcs) };
    // 要求传入足够的32字节的数据缓冲区
    let outs = unsafe { std::slice::from_raw_parts_mut(p_out, 32usize) };

    ps.sort(); 
    let mut idx = 0usize;
    // 有效的out数据格式：
    // suit长度 card1 card2...
    // number长度 card1 card2...
    // ...
    // 长度32足够了
    outs[idx] = ps.cards.cards.len() as u8;
    idx += 1;
    for v in &ps.sort_cards_suit {
        outs[idx] = v.to_u8();
        idx += 1;
    }
    outs[idx] = ps.cards.cards.len() as u8;
    idx += 1;
    for v in &ps.sort_cards_number {
        outs[idx] = v.to_u8();
        idx += 1;
    }
    // 返回out数据有效长度
    ret = idx as i8;
    std::mem::forget(ps);
    return ret;
}

#[no_mangle]
pub extern "C" fn rs_GinRummyCards_assign(
    p_pcs: *mut GinRummyCards,
    p_data: *const u16,
    data_len: usize,
    freeze: u8,
    p_out: *mut u8,
) -> i8 {
    if p_pcs.is_null() || p_data.is_null() || data_len == 0 {
        return -1;
    }
    let ret: i8;
    // 取结构
    let mut ps = unsafe { Box::from_raw(p_pcs) };
    // 取数据
    let slice = unsafe { std::slice::from_raw_parts(p_data, data_len as usize) };
    // 要求传入足够的32字节的数据缓冲区
    let outs = unsafe { std::slice::from_raw_parts_mut(p_out, 32usize) };

    match ps.assign(slice, freeze != 0) {
        Ok(n) => {
            let mut idx = 0usize;
            // 有效的out数据格式：
            // deadwood分数
            // deadwood长度 deadwood1 deadwood2 ...
            // meld1长度 meld1_1 meld1_2 ...
            // meld2长度 meld2_1 meld2_2...
            // ...
            // 长度32足够了
            // best deadwood value...
            outs[idx] = n;
            idx += 1;
            // best deadwood list...
            outs[idx] = ps.best_deadwood.len() as u8;
            idx += 1;
            for p in &ps.best_deadwood {
                outs[idx] = p.to_u8();
                idx += 1;
            }
            // melds list...
            for v in &ps.best_melds {
                outs[idx] = v.len() as u8;
                idx += 1;
                for p in v {
                    outs[idx] = p.to_u8();
                    idx += 1;
                }
            }
            // 返回out数据有效长度
            ret = idx as i8;
        }
        Err(_) => {
            // println!("{:?}", e);
            ret = -1;
        }
    }
    std::mem::forget(ps);
    return ret;
}

// 在堆上分配一个rust结构PokerCards，返回给c
// 由于含有vec字段，所以是透明结构，c中没有对应结构
#[no_mangle]
pub extern "C" fn rs_PokerCards_new() -> *mut PokerCards {
    let pcs: PokerCards = PokerCards::new();
    Box::into_raw(Box::new(pcs))
}

// 控制权归还给rust，rust在离开作用域时会释放内存
#[no_mangle]
pub extern "C" fn rs_PokerCards_free(p_pcs: *mut PokerCards) {
    if !p_pcs.is_null() {
        unsafe {
            let _ = Box::from_raw(p_pcs);
        };
    }
}

// 将unsigned short列表assign到PokerCards
#[no_mangle]
pub extern "C" fn rs_PokerCards_assign(
    p_pcs: *mut PokerCards,
    p_data: *const u16,
    data_len: usize,
) -> i8 {
    if p_pcs.is_null() || p_data.is_null() || data_len == 0 {
        return -1;
    }
    let ret: i8;
    // 取结构
    let mut ps = unsafe { Box::from_raw(p_pcs) };
    // 取数据
    let slice = unsafe { std::slice::from_raw_parts(p_data, data_len as usize) };
    match ps.assign(slice) {
        Ok(n) => {
            ret = n as i8;
        }
        Err(_) => {
            ret = -1;
        }
    }
    println!("{}", ps);
    std::mem::forget(ps);
    return ret;
}

#[repr(C)]
pub struct CardBuffer {
    data: *mut PokerCard,
    len: usize,
}

#[no_mangle]
pub extern "C" fn rs_PokerCards_get_cards(p_pcs: *mut PokerCards) -> CardBuffer {
    // 取结构
    let ps = unsafe { Box::from_raw(p_pcs) };
    let buf = ps.cards.clone().into_boxed_slice();
    let len = buf.len();
    let data: *mut PokerCard = Box::into_raw(buf) as _;
    // std::mem::forget(data);
    std::mem::forget(ps);
    CardBuffer { data, len }
}

#[no_mangle]
pub extern "C" fn rs_CardBuffer_free(buf: CardBuffer) {
    let s = unsafe { std::slice::from_raw_parts_mut(buf.data, buf.len) };
    let ps = s.as_mut_ptr();
    unsafe {
        let _ = Box::from_raw(ps);
    };
}

#[no_mangle]
pub extern "C" fn rs_PokerCards_get_counter(p_stu: *mut PokerCards, s: Suit) -> *mut Counter {
    if p_stu.is_null() {
        return std::ptr::null::<Counter>() as *mut _;
    }
    unsafe { &mut ((*p_stu)[s]) as *mut Counter }
}

#[no_mangle]
pub extern "C" fn rs_Counter_new(s: Suit) -> *mut Counter {
    let pcs: Counter = Counter::new(s);
    Box::into_raw(Box::new(pcs))
}

#[no_mangle]
pub extern "C" fn rs_Counter_free(p_counter: *mut Counter) {
    if !p_counter.is_null() {
        unsafe {
            let _ = Box::from_raw(p_counter);
        };
    }
}

#[no_mangle]
pub extern "C" fn rs_PokerCard_new(n: u16) -> *mut PokerCard {
    let pcs = PokerCard::from_spades_n(n);
    match pcs {
        Ok(p) => Box::into_raw(Box::new(p)),
        Err(_) => std::ptr::null::<PokerCard>() as *mut _,
    }
}

#[no_mangle]
pub extern "C" fn rs_PokerCard_free(p_poker: *mut PokerCard) {
    if !p_poker.is_null() {
        unsafe {
            let _ = Box::from_raw(p_poker);
        };
    }
}

// 德州扑克接口
#[no_mangle]
pub extern "C" fn rs_TexasCards_new() -> *mut TexasCards {
    let pcs = TexasCards::new();
    Box::into_raw(Box::new(pcs))
}

#[no_mangle]
pub extern "C" fn rs_TexasCards_free(p_poker: *mut TexasCards) {
    if !p_poker.is_null() {
        unsafe {
            let _ = Box::from_raw(p_poker);
        };
    }
}

#[no_mangle]
pub extern "C" fn rs_TexasCards_assign(
    p_pcs: *mut TexasCards,
    p_data: *const u16,
    data_len: usize,
) -> i8 {
    if p_pcs.is_null() || p_data.is_null() || data_len == 0 {
        return -1;
    }
    let ret: i8;
    // 取结构
    let mut ps = unsafe { Box::from_raw(p_pcs) };
    // 取数据
    let slice = unsafe { std::slice::from_raw_parts(p_data, data_len as usize) };
    match ps.assign(slice) {
        Ok(n) => {
            ret = n as i8;
        }
        Err(_) => {
            ret = -1;
        }
    }
    println!("{}", ps);
    std::mem::forget(ps);
    return ret;
}

#[repr(C)]
pub struct TexasCardBuffer {
    cardbuf: CardBuffer,
    texas: TexasType,
    score: u64,
}

#[no_mangle]
pub extern "C" fn rs_TexasCards_get_best(p_pcs: *mut TexasCards) -> TexasCardBuffer {
    // 取结构
    let ps = unsafe { Box::from_raw(p_pcs) };
    let buf = ps.best.clone().into_boxed_slice();
    let len = buf.len();
    let data: *mut PokerCard = Box::into_raw(buf) as _;
    let texas = ps.texas;
    let score = ps.score;
    // std::mem::forget(data);
    std::mem::forget(ps);
    TexasCardBuffer {
        cardbuf: CardBuffer { data, len },
        texas,
        score,
    }
}

#[no_mangle]
pub extern "C" fn rs_TexasCardBuffer_free(buf: TexasCardBuffer) {
    let s = unsafe { std::slice::from_raw_parts_mut(buf.cardbuf.data, buf.cardbuf.len) };
    let ps = s.as_mut_ptr();
    unsafe {
        let _ = Box::from_raw(ps);
    };
}
