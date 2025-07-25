// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Bezier curve algorithms for smooth animations and paths.
//!
//! This module provides functions to calculate and draw Bezier curves,
//! commonly used for smooth movement animations in games.

use crate::util::PointF32;

fn bezier_interpolation_func(t: f32, points: &[PointF32], count: usize) -> PointF32 {
    assert!(count > 0);

    let mut tmp_points = points.to_vec();
    for i in 1..count {
        for j in 0..(count - i) {
            if i == 1 {
                tmp_points[j].x = points[j].x * (1.0 - t) + points[j + 1].x * t;
                tmp_points[j].y = points[j].y * (1.0 - t) + points[j + 1].y * t;
                continue;
            }
            tmp_points[j].x = tmp_points[j].x * (1.0 - t) + tmp_points[j + 1].x * t;
            tmp_points[j].y = tmp_points[j].y * (1.0 - t) + tmp_points[j + 1].y * t;
        }
    }
    tmp_points[0]
}

pub fn draw_bezier_curves(points: &[PointF32], out_points: &mut [PointF32]) {
    let step = 1.0 / out_points.len() as f32;
    let mut t = 0.0;
    for item in out_points.iter_mut() {
        let temp_point = bezier_interpolation_func(t, points, points.len());
        t += step;
        *item = temp_point;
    }
}

