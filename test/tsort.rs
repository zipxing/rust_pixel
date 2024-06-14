// #[derive(Debug)]
// struct Ts {
//     a: i32,
//     b: i32,
// }

// fn main() {
//     let mut rn: Vec<Ts> = vec![];
//     for i in 0..10 {
//         rn.push(Ts { a: i, b: 1 });
//     }
//     rn[2].b = 3;
//     rn.sort_by_key(|t| t.b);
//     println!("{:?}", rn);
// }
//
fn remove_nv<T: std::cmp::PartialEq>(vc: &mut Vec<T>, n: usize, v: T) -> usize {
    if n == 0 || vc.len() == 0 {
        return 0;
    }
    let mut head: usize = 0;
    let mut tail: usize = vc.len() - 1;
    let mut rcount = 0;

    for _i in 0..vc.len() {
        if vc[head] == v {
            let ct = tail - head + 1;
            for j in 0..ct {
                if vc[tail - j] != v {
                    vc.swap(tail - j, head);
                    head += 1;
                    tail -= j;
                    break;
                } 
            }
            rcount += 1;
            if rcount >= n {
                break;
            }
        } else {
            head += 1;
        }
        if head > tail {
            break;
        }
    }
    vc.truncate(vc.len() - rcount);
    rcount
}

fn main() {
    let mut vc: Vec<u8>;
    // vc = vec![1, 2, 3, 2, 3, 3, 2];
    // vc = vec![1, 2, 3];
    // vc = vec![2, 2, 2];
    // vc = vec![1, 1, 3];
    // vc = vec![3, 1, 1, 3];
    // vc = vec![3, 3];
    // vc = vec![3];
    // vc = vec![];
    // vc = vec![3,3,3,3,3,3];
    // vc = vec![3,3,1,1,3];
    // vc = vec![3,1,3,1];
    vc = vec![1,2,3,3];
    println!("vc...{:?}", vc);
    let rcount = remove_nv::<u8>(&mut vc, 2, 3);
    println!("vc...{:?} rcount={}", vc, rcount);
}
