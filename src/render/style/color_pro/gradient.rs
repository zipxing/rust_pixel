// RustPixel
// copyright zipxing@hotmail.com 2022～2025

use crate::render::style::color_pro::*;

fn interpolate(a: f64, b: f64, fra: Fraction) -> f64 {
    a + fra.value() * (b - a)
}

pub fn mod_positive(x: f64, y: f64) -> f64 {
    (x % y + y) % y
}

pub fn interpolate_angle(a: f64, b: f64, fraction: Fraction) -> f64 {
    let paths = [(a, b), (a, b + 360.0), (a + 360.0, b)];

    let dist = |&(x, y): &(f64, f64)| (x - y).abs();
    let shortest = paths
        .iter()
        .min_by(|p1, p2| dist(p1).partial_cmp(&dist(p2)).unwrap_or(Ordering::Less))
        .unwrap();

    mod_positive(interpolate(shortest.0, shortest.1, fraction), 360.0)
}

fn mix(c1: ColorData, c2: ColorData, fra: Fraction) -> ColorData {
    let self_hue = if c1.v[1] < 0.1 { c2.v[2] } else { c1.v[2] };
    let other_hue = if c2.v[1] < 0.1 { c1.v[2] } else { c2.v[2] };

    ColorData {
        v: [
            interpolate(c1.v[0], c2.v[0], fra),
            interpolate(c1.v[1], c2.v[1], fra),
            interpolate_angle(self_hue, other_hue, fra),
            interpolate(c1.v[3], c2.v[3], fra),
        ],
    }
}

pub fn clamp(lower: f64, upper: f64, x: f64) -> f64 {
    f64::max(f64::min(upper, x), lower)
}

#[derive(Debug, Clone, Copy)]
pub struct Fraction {
    f: f64,
}

impl Fraction {
    pub fn from(s: f64) -> Self {
        Fraction {
            f: clamp(0.0, 1.0, s),
        }
    }

    pub fn value(self) -> f64 {
        self.f
    }
}

#[derive(Debug, Clone)]
struct ColorStop {
    color: ColorPro,
    position: Fraction,
}

#[derive(Debug, Clone)]
pub struct ColorGradient {
    color_stops: Vec<ColorStop>,
}

impl ColorGradient {
    pub fn empty() -> Self {
        Self {
            color_stops: Vec::new(),
        }
    }

    pub fn add_stop(&mut self, color: ColorPro, position: Fraction) -> &mut Self {
        #![allow(clippy::float_cmp)]
        let same_position = self
            .color_stops
            .iter_mut()
            .find(|c| position.value() == c.position.value());

        match same_position {
            Some(color_stop) => color_stop.color = color,
            None => {
                let next_index = self
                    .color_stops
                    .iter()
                    .position(|c| position.value() < c.position.value());

                let index = next_index.unwrap_or(self.color_stops.len());

                let color_stop = ColorStop { color, position };

                self.color_stops.insert(index, color_stop);
            }
        };

        self
    }

    pub fn sample(&self, position: Fraction, cs: ColorSpace) -> Option<ColorData> {
        if self.color_stops.len() < 2 {
            return None;
        }

        let left_stop = self
            .color_stops
            .iter()
            .rev()
            .find(|c| position.value() >= c.position.value());

        let right_stop = self
            .color_stops
            .iter()
            .find(|c| position.value() <= c.position.value());

        match (left_stop, right_stop) {
            (Some(left_stop), Some(right_stop)) => {
                let diff_color_stops = right_stop.position.value() - left_stop.position.value();
                let diff_position = position.value() - left_stop.position.value();
                let local_position = Fraction::from(diff_position / diff_color_stops);

                let color = mix(
                    left_stop.color[cs].unwrap(),
                    right_stop.color[cs].unwrap(),
                    local_position,
                );

                Some(color)
            }
            _ => None,
        }
    }
}
