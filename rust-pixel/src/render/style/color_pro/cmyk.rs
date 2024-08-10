// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::style::color_pro::*;

#[inline(always)]
pub fn srgba_to_cmyk(srgb: ColorData) -> ColorData {
    let r = srgb.v[0];
    let g = srgb.v[1];
    let b = srgb.v[2];

    let k = 1.0 - r.max(g.max(b));
    let c = if k < 1.0 {
        (1.0 - r - k) / (1.0 - k)
    } else {
        0.0
    };
    let m = if k < 1.0 {
        (1.0 - g - k) / (1.0 - k)
    } else {
        0.0
    };
    let y = if k < 1.0 {
        (1.0 - b - k) / (1.0 - k)
    } else {
        0.0
    };

    ColorData { v: [c, m, y, k] }
}

#[inline(always)]
pub fn cmyk_to_srgba(cmyk: ColorData) -> ColorData {
    let c = cmyk.v[0];
    let m = cmyk.v[1];
    let y = cmyk.v[2];
    let k = cmyk.v[3];

    let r = (1.0 - c) * (1.0 - k);
    let g = (1.0 - m) * (1.0 - k);
    let b = (1.0 - y) * (1.0 - k);

    ColorData { v: [r, g, b, 1.0] }
}

