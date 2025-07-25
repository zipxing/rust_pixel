// RustPixel
// copyright zipxing@hotmail.com 2022～2025

use crate::render::style::color_pro::*;

#[inline(always)]
pub fn hsla_to_srgba(hsla: ColorData) -> ColorData {
    let (h, s, l, a) = (hsla.v[0], hsla.v[1], hsla.v[2], hsla.v[3]);

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    ColorData {
        v: [r + m, g + m, b + m, a],
    }
}

#[inline(always)]
pub fn srgba_to_hsla(srgba: ColorData) -> ColorData {
    let (r, g, b, a) = (srgba.v[0], srgba.v[1], srgba.v[2], srgba.v[3]);

    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    let l = (max + min) / 2.0;
    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    ColorData { v: [h, s, l, a] }
}

#[inline(always)]
pub fn hsva_to_srgba(hsva: ColorData) -> ColorData {
    let (h, s, v, a) = (hsva.v[0], hsva.v[1], hsva.v[2], hsva.v[3]);

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    ColorData {
        v: [r + m, g + m, b + m, a],
    }
}

#[inline(always)]
pub fn srgba_to_hsva(srgba: ColorData) -> ColorData {
    let (r, g, b, a) = (srgba.v[0], srgba.v[1], srgba.v[2], srgba.v[3]);

    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    ColorData { v: [h, s, v, a] }
}

#[inline(always)]
pub fn hwba_to_srgba(hwba: ColorData) -> ColorData {
    let (h, w, b, a) = (hwba.v[0], hwba.v[1], hwba.v[2], hwba.v[3]);

    let v = 1.0 - b;
    let s = if v == 0.0 { 0.0 } else { 1.0 - w / v };

    hsva_to_srgba(ColorData { v: [h, s, v, a] })
}

#[inline(always)]
pub fn srgba_to_hwba(srgba: ColorData) -> ColorData {
    let hsva = srgba_to_hsva(srgba);
    let (h, s, v, a) = (hsva.v[0], hsva.v[1], hsva.v[2], hsva.v[3]);

    let w = v * (1.0 - s);
    let b = 1.0 - v;

    ColorData { v: [h, w, b, a] }
}
