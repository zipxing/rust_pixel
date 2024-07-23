// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! astar shortest path algorithm

//! # Example
//!
//! ```no_run
//! use rust_pixel::algorithm::astar::*;
//! fn main() {
//!     let map = vec![
//!         vec![1, 1, 1, 1, 1],
//!         vec![0, 1, 1, 0, 1],
//!         vec![1, 0, 1, 1, 1],
//!         vec![0, 1, 1, 0, 0],
//!         vec![1, 1, 1, 1, 1],
//!     ];
//!     let start = (0, 0);
//!     let end = (4, 4);
//!     if let Some(path) = a_star(&map, start, end, |x| {true}) {
//!         println!("Path found: {:?}", path);
//!         //[(0,0), (0,1), (1,1), (1,2), (2,2), (3,2), (4,2), (4,3), (4,4)]
//!     } else {
//!         println!("No path found");
//!     }
//! }
//! ```

use crate::util::PointU16;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

// (y, x)
pub type PointUsize = (usize, usize);

impl From<PointU16> for PointUsize {
    fn from(p: PointU16) -> Self {
        (p.y as usize, p.x as usize)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct ANode {
    pos: PointUsize,
    g: usize,
    f: usize,
}

impl Ord for ANode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f)
    }
}

impl PartialOrd for ANode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn a_star<F>(map: &Vec<Vec<u8>>, start: PointUsize, end: PointUsize, func: F) -> Option<Vec<PointUsize>>
where
    F: Fn(u8) -> bool,
{
    let mut open_set = BinaryHeap::new();
    let mut came_from = vec![vec![None; map[0].len()]; map.len()];

    open_set.push(ANode {
        pos: start,
        g: 0,
        f: manhattan_distance(start, end),
    });

    while let Some(current) = open_set.pop() {
        if current.pos == end {
            let mut path = Vec::new();
            let mut current_pos = end;
            while current_pos != start {
                path.push(current_pos);
                current_pos = came_from[current_pos.0][current_pos.1].unwrap();
            }
            path.push(start);
            path.reverse();
            return Some(path);
        }

        for (dx, dy) in &[(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
            let neighbor_pos = (
                (current.pos.0 as i32 + dx) as usize,
                (current.pos.1 as i32 + dy) as usize,
            );

            if !is_valid(neighbor_pos, &map, &func) {
                continue;
            }

            let tentative_g = current.g + 1;
            let neighbor_node = ANode {
                pos: neighbor_pos,
                g: tentative_g,
                f: tentative_g + manhattan_distance(neighbor_pos, end),
            };

            if came_from[neighbor_pos.0][neighbor_pos.1].is_none() {
                came_from[neighbor_pos.0][neighbor_pos.1] = Some(current.pos);
                open_set.push(neighbor_node);
            }
        }
    }

    None
}

fn manhattan_distance(a: PointUsize, b: PointUsize) -> usize {
    ((a.0 as isize - b.0 as isize).abs() + (a.1 as isize - b.1 as isize).abs()) as usize
}

fn is_valid<F>(pos: PointUsize, map: &Vec<Vec<u8>>, f: F) -> bool
where
    F: Fn(u8) -> bool,
{
    pos.0 < map.len() && pos.1 < map[0].len() && f(map[pos.0][pos.1])
}
