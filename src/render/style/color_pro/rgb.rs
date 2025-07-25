// RustPixel
// copyright zipxing@hotmail.com 2022～2025

use crate::render::style::color_pro::*;

#[inline(always)]
pub fn linearize(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[inline(always)]
pub fn delinearize(value: f64) -> f64 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[inline(always)]
pub fn srgba_to_linear(s: ColorData) -> ColorData {
    let r = linearize(s.v[0]);
    let g = linearize(s.v[1]);
    let b = linearize(s.v[2]);

    ColorData {
        v: [r, g, b, s.v[3]],
    }
}

#[inline(always)]
pub fn linear_to_srgba(l: ColorData) -> ColorData {
    let sr = delinearize(l.v[0]);
    let sg = delinearize(l.v[1]);
    let sb = delinearize(l.v[2]);

    ColorData {
        v: [sr, sg, sb, l.v[3]],
    }
}

#[inline(always)]
pub fn linear_to_xyz(l: ColorData) -> ColorData {
    let x = l.v[0] * 0.4124564 + l.v[1] * 0.3575761 + l.v[2] * 0.1804375;
    let y = l.v[0] * 0.2126729 + l.v[1] * 0.7151522 + l.v[2] * 0.0721750;
    let z = l.v[0] * 0.0193339 + l.v[1] * 0.1191920 + l.v[2] * 0.9503041;

    ColorData {
        v: [x, y, z, l.v[3]],
    }
}

#[inline(always)]
pub fn xyz_to_linear(xyz: ColorData) -> ColorData {
    let r = xyz.v[0] * 3.2404542 - xyz.v[1] * 1.5371385 - xyz.v[2] * 0.4985314;
    let g = xyz.v[0] * -0.9692660 + xyz.v[1] * 1.8760108 + xyz.v[2] * 0.0415560;
    let b = xyz.v[0] * 0.0556434 - xyz.v[1] * 0.2040259 + xyz.v[2] * 1.0572252;

    ColorData {
        v: [r, g, b, xyz.v[3]],
    }
}

#[inline(always)]
pub fn srgba_to_xyz(srgba: ColorData) -> (ColorData, ColorData) {
    let l = srgba_to_linear(srgba);
    let xyza = linear_to_xyz(l);
    (xyza, l)
}

#[inline(always)]
pub fn xyz_to_srgba(xyz: ColorData) -> ColorData {
    let l = xyz_to_linear(xyz);
    linear_to_srgba(l)
}

