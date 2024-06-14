fn main() {
    let mut rn: u32  = 0;
    //let mut r = rn.wrapping_mul(214013).wrapping_add(2531011);
    for i in 0..1000 {
        let r = rn.wrapping_mul(1103515245).wrapping_add(12345);
        rn = r & 0x7FFFFFFF;
        if i % 2 != 0 {
            println!("r={}", rn % 4);
        }
    }
}
