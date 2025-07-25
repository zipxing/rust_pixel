// RustPixel
// copyright zipxing@hotmail.com 2022～2025

use crate::render::style::color_pro::*;

#[inline(always)]
pub fn xyz_to_laba(xyza: ColorData) -> ColorData {
    let epsilon = EPSILON_LSTAR;
    let kappa = KAPPA;

    // D65 XN YN ZN
    // 0.9504559270516716, 1.0, 1.0890577507598784
    let xr = xyza.v[0] / WHITE[0];
    let yr = xyza.v[1] / WHITE[1];
    let zr = xyza.v[2] / WHITE[2];

    let fx = if xr > epsilon {
        xr.powf(1.0 / 3.0)
    } else {
        (kappa * xr + 16.0) / 116.0
    };
    let fy = if yr > epsilon {
        yr.powf(1.0 / 3.0)
    } else {
        (kappa * yr + 16.0) / 116.0
    };
    let fz = if zr > epsilon {
        zr.powf(1.0 / 3.0)
    } else {
        (kappa * zr + 16.0) / 116.0
    };

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    ColorData {
        v: [l, a, b, xyza.v[3]],
    }
}

#[inline(always)]
pub fn laba_to_xyz(laba: ColorData) -> ColorData {
    let epsilon = EPSILON_LSTAR;
    let kappa = KAPPA;

    let fy = (laba.v[0] + 16.0) / 116.0;
    let fx = laba.v[1] / 500.0 + fy;
    let fz = fy - laba.v[2] / 200.0;

    let xr = if fx.powi(3) > epsilon {
        fx.powi(3)
    } else {
        (116.0 * fx - 16.0) / kappa
    };
    let yr = if laba.v[0] > kappa * epsilon {
        fy.powi(3)
    } else {
        laba.v[0] / kappa
    };
    let zr = if fz.powi(3) > epsilon {
        fz.powi(3)
    } else {
        (116.0 * fz - 16.0) / kappa
    };

    // D65 XN YN ZN
    // 0.9504559270516716, 1.0, 1.0890577507598784
    let x = xr * WHITE[0];
    let y = yr * WHITE[1];
    let z = zr * WHITE[2];

    ColorData {
        v: [x, y, z, laba.v[3]],
    }
}

#[inline(always)]
pub fn laba_to_lcha(laba: ColorData) -> ColorData {
    let l = laba.v[0];
    let a = laba.v[1];
    let b = laba.v[2];
    let c = (a * a + b * b).sqrt();
    let h = f64::atan2(b, a);
    let h = h.to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    ColorData {
        v: [l, c, h, laba.v[3]],
    }
}

#[inline(always)]
pub fn lcha_to_laba(lcha: ColorData) -> ColorData {
    let l = lcha.v[0];
    let a = lcha.v[1] * lcha.v[2].to_radians().cos();
    let b = lcha.v[1] * lcha.v[2].to_radians().sin();

    ColorData {
        v: [l, a, b, lcha.v[3]],
    }
}

#[inline(always)]
pub fn lcha_to_xyz(lcha: ColorData) -> (ColorData, ColorData) {
    let laba = lcha_to_laba(lcha);
    (laba_to_xyz(laba), laba)
}

#[inline(always)]
pub fn xyz_to_oklaba(xyza: ColorData) -> ColorData {
    let l = 0.8189330101 * xyza.v[0] + 0.3618667424 * xyza.v[1] - 0.1288597137 * xyza.v[2];
    let m = 0.0329845436 * xyza.v[0] + 0.9293118715 * xyza.v[1] + 0.0361456387 * xyza.v[2];
    let s = 0.0482003018 * xyza.v[0] + 0.2643662691 * xyza.v[1] + 0.6338517070 * xyza.v[2];

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    ColorData {
        v: [l, a, b, xyza.v[3]],
    }
}

#[inline(always)]
pub fn oklaba_to_xyz(oklaba: ColorData) -> ColorData {
    let l =
        (1.00000000 * oklaba.v[0] + 0.39633779 * oklaba.v[1] + 0.21580376 * oklaba.v[2]).powi(3);
    let m =
        (1.00000001 * oklaba.v[0] - 0.10556134 * oklaba.v[1] - 0.06385417 * oklaba.v[2]).powi(3);
    let s =
        (1.00000005 * oklaba.v[0] - 0.08948418 * oklaba.v[1] - 1.29148554 * oklaba.v[2]).powi(3);

    let x = 1.22701385 * l - 0.55779998 * m + 0.28125615 * s;
    let y = -0.04058018 * l + 1.11225687 * m - 0.07167668 * s;
    let z = -0.07638128 * l - 0.42148198 * m + 1.58616322 * s;

    ColorData {
        v: [x, y, z, oklaba.v[3]],
    }
}

#[inline(always)]
pub fn oklaba_to_oklcha(oklaba: ColorData) -> ColorData {
    let l = oklaba.v[0];
    let a = oklaba.v[1];
    let b = oklaba.v[2];

    let c = (a.powi(2) + b.powi(2)).sqrt();
    let h = b.atan2(a).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    ColorData {
        v: [l, c, h, oklaba.v[3]],
    }
}

#[inline(always)]
pub fn oklcha_to_oklaba(oklcha: ColorData) -> ColorData {
    let l = oklcha.v[0];
    let c = oklcha.v[1];
    let h = oklcha.v[2].to_radians();

    let a = c * h.cos();
    let b = c * h.sin();

    ColorData {
        v: [l, a, b, oklcha.v[3]],
    }
}

#[inline(always)]
pub fn oklcha_to_xyz(oklcha: ColorData) -> (ColorData, ColorData) {
    let oklaba = oklcha_to_oklaba(oklcha);
    (oklaba_to_xyz(oklaba), oklaba)
}


