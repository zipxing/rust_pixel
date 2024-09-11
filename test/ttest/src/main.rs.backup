use glam::{Vec2, Vec3, Vec4};
use std::f32::consts::PI;

fn linear_ease(begin: f32, change: f32, duration: f32, time: f32) -> f32 {
    change * time / duration + begin
}

fn exponential_ease_in_out(begin: f32, change: f32, duration: f32, time: f32) -> f32 {
    if time == 0.0 {
        return begin;
    } else if time == duration {
        return begin + change;
    }
    let time = time / (duration / 2.0);
    if time < 1.0 {
        return change / 2.0 * (2.0_f32.powf(10.0 * (time - 1.0))) + begin;
    }
    return change / 2.0 * (-2.0_f32.powf(-10.0 * (time - 1.0)) + 2.0) + begin;
}

fn sinusoidal_ease_in_out(begin: f32, change: f32, duration: f32, time: f32) -> f32 {
    -change / 2.0 * ((PI * time / duration).cos() - 1.0) + begin
}

fn rand(co: Vec2) -> f32 {
    (co.dot(Vec2::new(12.9898, 78.233)).sin() * 43758.5453).fract()
}

fn cross_fade(uv: Vec2, dissolve: f32, from_color: Vec3, to_color: Vec3) -> Vec3 {
    from_color.lerp(to_color, dissolve)
}

fn transition(uv: Vec2, progress: f32, strength: f32, from_color_fn: fn(Vec2) -> Vec3, to_color_fn: fn(Vec2) -> Vec3) -> Vec4 {
    let tex_coord = uv / Vec2::ONE;

    // 线性插值图像中心在图像中间移动
    let center = Vec2::new(linear_ease(0.25, 0.5, 1.0, progress), 0.5);
    let dissolve = exponential_ease_in_out(0.0, 1.0, 1.0, progress);

    // 镜像正弦循环 0->strength 然后 strength->0
    let strength = sinusoidal_ease_in_out(0.0, strength, 0.5, progress);

    let mut color = Vec3::ZERO;
    let mut total = 0.0;
    let to_center = center - tex_coord;

    // 随机化查找值以隐藏固定数量的样本
    let offset = rand(uv);

    for t in 0..=40 {
        let percent = (t as f32 + offset) / 40.0;
        let weight = 4.0 * (percent - percent * percent);
        let sample_color = cross_fade(
            tex_coord + to_center * percent * strength,
            dissolve,
            from_color_fn(tex_coord),
            to_color_fn(tex_coord),
        );
        color += sample_color * weight;
        total += weight;
    }
    color /= total;
    Vec4::new(color[0], color[1], color[2], 1.0)
}

// 定义 Vec3 和 Vec4，Vec2 的 from_color_fn 和 to_color_fn 实现略过，可以根据需要实现。
fn main() {

}

// use itertools::Itertools;
// #[derive(Clone, Copy)]
// struct Point {
//     x: i32,
//     y: i32,
// }

// fn draw_line(p1: Point, p2: Point) -> Vec<Point> {
//     let mut points = Vec::new();

//     let dx = (p2.x - p1.x).abs();
//     let dy = (p2.y - p1.y).abs();

//     let sx = if p1.x < p2.x { 1 } else { -1 };
//     let sy = if p1.y < p2.y { 1 } else { -1 };

//     let mut err = if dx > dy { dx } else { -dy } / 2;
//     let mut err2: i32;

//     let mut x = p1.x;
//     let mut y = p1.y;

//     loop {
//         points.push(Point { x, y });

//         if x == p2.x && y == p2.y {
//             break;
//         }

//         err2 = err;

//         if err2 > -dx {
//             err -= dy;
//             x += sx;
//         }

//         if err2 < dy {
//             err += dx;
//             y += sy;
//         }
//     }

//     points
// }

// fn reverse_bresenham_next_point(p1: Point, p2: Point) -> Point {
//     let dx = (p2.x - p1.x).abs();
//     let dy = (p2.y - p1.y).abs();
//     let sx = if p1.x < p2.x { -1 } else { 1 };
//     let sy = if p1.y < p2.y { -1 } else { 1 };
//     let err = if dx > dy { dx } else { -dy } / 2;
//     let mut x = p1.x;
//     let mut y = p1.y;
//     if err > -dx {
//         x += sx;
//     }
//     if err < dy {
//         y += sy;
//     }
//     Point { x, y }
// }

// fn hand_list_com(hands: &Vec<u8>) -> Vec<Vec<u8>> {
//     let mut res: Vec<Vec<u8>> = Vec::new();
//     let shand: Vec<u8> = hands
//         .iter()
//         .cloned()
//         .collect::<std::collections::HashSet<_>>()
//         .into_iter()
//         .collect();

//     for i in 0..shand.len() {
//         if i + 1 == shand.len() {
//             continue;
//         }
//         for m in shand.iter().combinations(i + 1) {
//             let _com: Vec<u8> = m.into_iter().cloned().collect();
//             res.push(_com);
//         }
//     }

//     res
// }

// fn main() {
//     let p1 = Point { x: 3, y: 3 };
//     let p2 = Point { x: 18, y: 35 };

//     let line_points = draw_line(p1, p2);

//     for point in line_points.iter() {
//         println!("({}, {})", point.x, point.y);
//     }
//     let reverse_next_point = reverse_bresenham_next_point(p1, p2);
//     println!("rev_next:{} {}", reverse_next_point.x, reverse_next_point.y);

//     let vc: Vec<u8> = vec![1, 2, 3, 4, 5];
//     for com in vc.iter().combinations(2) {
//         let _v: Vec<u8> = com.into_iter().cloned().collect();
//         println!("com..{:?}", _v);
//     }
//     println!("hand_list_com...{:?}", hand_list_com(&vc));
// }
// /*
// use rand::Rng;

// fn draw_lightning(x1: f64, y1: f64, x2: f64, y2: f64, displace: f64, cur_detail: f64) {
//     if displace < cur_detail {
//         println!("move_to...{},{}", x1 as u16, y1 as u16);
//         println!("line_to...{},{}", x2 as u16, y2 as u16);
//         // graf.move_to(x1, y1);
//         // graf.line_to(x2, y2);
//     } else {
//         let mid_x = (x2 + x1) / 2.0;
//         let mid_y = (y2 + y1) / 2.0;
//         let mut rng = rand::thread_rng();
//         let random_offset_x = rng.gen_range(-0.5..0.5) * displace;
//         let random_offset_y = rng.gen_range(-0.5..0.5) * displace;
//         let new_mid_x = mid_x + random_offset_x;
//         let new_mid_y = mid_y + random_offset_y;

//         draw_lightning(x1, y1, new_mid_x, new_mid_y, displace / 2.0, cur_detail);
//         draw_lightning(x2, y2, new_mid_x, new_mid_y, displace / 2.0, cur_detail);
//     }
// }

// fn main() {
//     //let mut graf = Graf::new(); // Assuming Graf is a custom drawing library struct

//     let x1 = 0.0;
//     let y1 = 0.0;
//     let x2 = 100.0;
//     let y2 = 100.0;
//     let displace = 30.0;
//     let cur_detail = 8.0;

//     draw_lightning(x1, y1, x2, y2, displace, cur_detail);
// }
// */

// /*
// use std::cmp::Ordering;
// use std::collections::BinaryHeap;
// use bevy_ecs::prelude::*;

// use unicode_segmentation::UnicodeSegmentation;

// #[derive(Component)]
// #[derive(Debug)]
// struct Position { x: f32, y: f32 }

// type APoint = (usize, usize);

// #[derive(Copy, Clone, PartialEq, Eq, Debug)]
// struct ANode {
//     pos: APoint,
//     g: usize,
//     f: usize,
// }

// impl Ord for ANode {
//     fn cmp(&self, other: &Self) -> Ordering {
//         other.f.cmp(&self.f)
//     }
// }

// impl PartialOrd for ANode {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

// fn a_star(map: &Vec<Vec<u8>>, start: APoint, end: APoint) -> Option<Vec<APoint>> {
//     let mut open_set = BinaryHeap::new();
//     let mut came_from = vec![vec![None; map[0].len()]; map.len()];

//     open_set.push(ANode {
//         pos: start,
//         g: 0,
//         f: manhattan_distance(start, end),
//     });

//     while let Some(current) = open_set.pop() {
//         if current.pos == end {
//             let mut path = Vec::new();
//             let mut current_pos = end;
//             while current_pos != start {
//                 path.push(current_pos);
//                 current_pos = came_from[current_pos.0][current_pos.1].unwrap();
//             }
//             path.push(start);
//             path.reverse();
//             return Some(path);
//         }

//         for (dx, dy) in &[(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
//             let neighbor_pos = (
//                 (current.pos.0 as i32 + dx) as usize,
//                 (current.pos.1 as i32 + dy) as usize,
//             );

//             if !is_valid(neighbor_pos, &map) {
//                 continue;
//             }

//             let tentative_g = current.g + 1;
//             let neighbor_node = ANode {
//                 pos: neighbor_pos,
//                 g: tentative_g,
//                 f: tentative_g + manhattan_distance(neighbor_pos, end),
//             };

//             if came_from[neighbor_pos.0][neighbor_pos.1].is_none() {
//                 came_from[neighbor_pos.0][neighbor_pos.1] = Some(current.pos);
//                 open_set.push(neighbor_node);
//             }
//         }
//     }

//     None
// }

// fn manhattan_distance(a: APoint, b: APoint) -> usize {
//     ((a.0 as isize - b.0 as isize).abs() + (a.1 as isize - b.1 as isize).abs()) as usize
// }

// fn is_valid(pos: APoint, map: &Vec<Vec<u8>>) -> bool {
//     pos.0 < map.len() && pos.1 < map[0].len() && map[pos.0][pos.1] == 1
// }

// fn movement(mut query:Query<&mut Position>) {
//     for mut pos in &mut query {
//         pos.x += 1.0;
//         pos.y += 1.0;
//         println!("pos..{:?}", pos);
//     }
// }

// fn main() {

//     let name = "José Guimarães\ra🧲a\n";
//     let graphemes = UnicodeSegmentation::graphemes(name, true)
//         .collect::<Vec<&str>>();
//     println!("{:?}", graphemes);

//     let map = vec![
//         vec![1, 1, 1, 1, 1],
//         vec![0, 1, 1, 0, 1],
//         vec![1, 0, 1, 1, 1],
//         vec![0, 1, 1, 0, 0],
//         vec![1, 1, 1, 1, 1],
//     ];

//     let start = (0, 0);
//     let end = (4, 4);

//     if let Some(path) = a_star(&map, start, end) {
//         println!("Path found: {:?}", path);
//     } else {
//         println!("No path found");
//     }

//     let mut world = World::new();
//     world.spawn(Position{x: 0.0, y: 0.0});
//     let mut sc = Schedule::default();
//     sc.add_system(movement);
//     sc.run(&mut world);
// }
// */
