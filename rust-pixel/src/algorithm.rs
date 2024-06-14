// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! here integrates some common algorithms e.g. disjoint-set data structure, astar
pub mod union_find;
pub mod astar;

pub fn findv<T: std::cmp::PartialEq>(v1: &Vec<T>, val: &T) -> bool {
    v1.contains(val)
}

pub fn catvv<T: Clone>(v1: &mut Vec<T>, v2: &[T]) {
    v1.extend_from_slice(v2);
}

pub fn remove_nv<T: std::cmp::PartialEq>(vc: &mut Vec<T>, n: usize, v: T) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_nv() {
        let mut vc: Vec<u8>;
        let mut rcount;
        vc = vec![1, 2, 3, 2, 3, 3, 2];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 2);
        assert_eq!(vc, vec![1, 2, 2, 2, 3]);
        vc = vec![1, 2, 3];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 1);
        assert_eq!(vc, vec![1, 2]);
        vc = vec![2, 2, 2];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 0);
        assert_eq!(vc, vec![2, 2, 2]);
        vc = vec![1, 1, 3];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 1);
        assert_eq!(vc, vec![1, 1]);
        vc = vec![3, 1, 1, 3];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 2);
        assert_eq!(vc, vec![1, 1]);
        vc = vec![3, 3];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 2);
        assert_eq!(vc.len(), 0);
        vc = vec![3];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 1);
        assert_eq!(vc.len(), 0);
        vc = vec![];
        rcount = remove_nv::<u8>(&mut vc, 2, 3);
        assert_eq!(rcount, 0);
        assert_eq!(vc, Vec::<u8>::new());
    }
}

