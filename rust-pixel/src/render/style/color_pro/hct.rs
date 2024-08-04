#![allow(non_snake_case)]
use crate::render::style::color_pro::*;
use lazy_static::lazy_static;
use std::f64::consts::PI;

lazy_static! {
    static ref ENVS: Vec<Environment> = {
        let v1 = environment(WHITE, 64.0 / PI * 0.2, 20.0, &SURROUND_MAP[2], false);
        let v2 = environment(
            WHITE,
            200.0 / PI * from_lstar(50.0),
            from_lstar(50.0) * 100.0,
            &SURROUND_MAP[2],
            false,
        );
        let mut rv = vec![];
        rv.push(v1);
        rv.push(v2);
        rv
    };
}

const ADAPTED_COEF: f64 = 0.42;
const ADAPTED_COEF_INV: f64 = 1.0 / ADAPTED_COEF;
const TAU: f64 = 2.0 * PI;

const CAT16: [[f64; 3]; 3] = [
    [0.401288, 0.650173, -0.051461],
    [-0.250268, 1.204414, 0.045854],
    [-0.002079, 0.048952, 0.953127],
];

const CAT16_INV: [[f64; 3]; 3] = [
    [1.8620678550872327, -1.0112546305316843, 0.14918677544445175],
    [
        0.38752654323613717,
        0.6214474419314753,
        -0.008973985167612518,
    ],
    [
        -0.015841498849333856,
        -0.03412293802851557,
        1.0499644368778496,
    ],
];

const M1: [[f64; 3]; 3] = [
    [460.0, 451.0, 288.0],
    [460.0, -891.0, -261.0],
    [460.0, -220.0, -6300.0],
];

const SURROUND_MAP: [[f64; 3]; 3] = [[0.8, 0.525, 0.8], [0.9, 0.59, 0.9], [1.0, 0.69, 1.0]];

const HUE_QUAD_MAP: ([f64; 5], [f64; 5], [f64; 5]) = (
    [20.14, 90.00, 164.25, 237.53, 380.14],
    [0.8, 0.7, 1.0, 1.2, 0.8],
    [0.0, 100.0, 200.0, 300.0, 400.0],
);

const RAD2DEG: f64 = 180.0 / PI;
const DEG2RAD: f64 = PI / 180.0;

fn spow(x: f64, y: f64) -> f64 {
    x.powf(y)
}

fn copy_sign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}

fn multiply_matrices(a: [[f64; 3]; 3], b: [f64; 3]) -> [f64; 3] {
    let mut result = [0.0; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i] += a[i][j] * b[j];
        }
    }
    result
}

fn interpolate(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

fn zdiv(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        0.0
    } else {
        a / b
    }
}

fn constrain(v: f64) -> f64 {
    if v < 0.0 {
        v + 360.0
    } else {
        v % 360.0
    }
}

fn adapt(coords: [f64; 3], fl: f64) -> [f64; 3] {
    coords.map(|c| {
        let x = spow(fl * c.abs() * 0.01, ADAPTED_COEF);
        400.0 * copy_sign(x, c) / (x + 27.13)
    })
}

fn unadapt(adapted: [f64; 3], fl: f64) -> [f64; 3] {
    let constant = 100.0 / fl * (27.13_f64).powf(ADAPTED_COEF_INV);
    adapted.map(|c| {
        let cabs = c.abs();
        copy_sign(constant * spow(cabs / (400.0 - cabs), ADAPTED_COEF_INV), c)
    })
}

fn hue_quadrature(h: f64) -> f64 {
    let mut hp = constrain(h);
    if hp <= HUE_QUAD_MAP.0[0] {
        hp += 360.0;
    }

    let i = HUE_QUAD_MAP.0.iter().position(|&x| x > hp).unwrap() - 1;
    let hi = HUE_QUAD_MAP.0[i];
    let hii = HUE_QUAD_MAP.0[i + 1];
    let ei = HUE_QUAD_MAP.1[i];
    let eii = HUE_QUAD_MAP.1[i + 1];
    let Hi = HUE_QUAD_MAP.2[i];

    let t = (hp - hi) / ei;
    Hi + (100.0 * t) / (t + (hii - hp) / eii)
}

fn inv_hue_quadrature(H: f64) -> f64 {
    let Hp = (H % 400.0 + 400.0) % 400.0;
    let i = (Hp / 100.0).floor() as usize;
    let Hp = Hp % 100.0;
    let hi = HUE_QUAD_MAP.0[i];
    let hii = HUE_QUAD_MAP.0[i + 1];
    let ei = HUE_QUAD_MAP.1[i];
    let eii = HUE_QUAD_MAP.1[i + 1];

    constrain((Hp * (eii * hi - ei * hii) - 100.0 * hi * eii) / (Hp * (eii - ei) - 100.0 * eii))
}

#[derive(Debug)]
struct Environment {
    fl: f64,
    fl_root: f64,
    n: f64,
    z: f64,
    nbb: f64,
    ncb: f64,
    c: f64,
    nc: f64,
    d_rgb: [f64; 3],
    d_rgb_inv: [f64; 3],
    a_w: f64,
}

fn environment(
    ref_white: [f64; 3],
    adapting_luminance: f64,
    background_luminance: f64,
    surround: &'static [f64; 3],
    discounting: bool,
) -> Environment {
    let xyz_w = ref_white.map(|c| c * 100.0);
    let la = adapting_luminance;
    let yb = background_luminance;
    let yw = xyz_w[1];
    let rgb_w = multiply_matrices(CAT16, xyz_w);

    let f = surround[0];
    let c = surround[1];
    let nc = surround[2];

    let k = 1.0 / (5.0 * la + 1.0);
    let k4 = k.powi(4);

    let fl = k4 * la + 0.1 * (1.0 - k4) * (1.0 - k4) * (5.0 * la).cbrt();
    let fl_root = fl.powf(0.25);

    let n = yb / yw;
    let z = 1.48 + n.sqrt();
    let nbb = 0.725 * n.powf(-0.2);
    let ncb = nbb;

    let d = if discounting {
        1.0
    } else {
        f * (1.0 - 1.0 / 3.6 * ((-la - 42.0) / 92.0).exp()).clamp(0.0, 1.0)
    };
    let d_rgb = rgb_w.map(|c| interpolate(1.0, yw / c, d));
    let d_rgb_inv = d_rgb.map(|c| 1.0 / c);

    let rgb_cw = [
        rgb_w[0] * d_rgb[0],
        rgb_w[1] * d_rgb[1],
        rgb_w[2] * d_rgb[2],
    ];
    let rgb_aw = adapt(rgb_cw, fl);
    let a_w = nbb * (2.0 * rgb_aw[0] + rgb_aw[1] + 0.05 * rgb_aw[2]);

    Environment {
        fl,
        fl_root,
        n,
        z,
        nbb,
        ncb,
        c,
        nc,
        d_rgb,
        d_rgb_inv,
        a_w,
    }
}

fn from_cam16(cam16: &Cam16Object, env: &Environment) -> [f64; 3] {
    if !(cam16.J.is_some() ^ cam16.Q.is_some()) {
        panic!("Conversion requires one and only one: 'J' or 'Q'");
    }

    if !(cam16.C.is_some() ^ cam16.M.is_some() ^ cam16.s.is_some()) {
        panic!("Conversion requires one and only one: 'C', 'M' or 's'");
    }

    if !(cam16.h.is_some() ^ cam16.H.is_some()) {
        panic!("Conversion requires one and only one: 'h' or 'H'");
    }

    if cam16.J == Some(0.0) || cam16.Q == Some(0.0) {
        return [0.0, 0.0, 0.0];
    }

    let h_rad = if let Some(h) = cam16.h {
        constrain(h) * DEG2RAD
    } else {
        inv_hue_quadrature(cam16.H.unwrap()) * DEG2RAD
    };

    let cos_h = h_rad.cos();
    let sin_h = h_rad.sin();

    let j_root = if let Some(J) = cam16.J {
        spow(J, 0.5) * 0.1
    } else {
        0.25 * env.c * cam16.Q.unwrap() / ((env.a_w + 4.0) * env.fl_root)
    };

    let alpha = if let Some(C) = cam16.C {
        C / j_root
    } else if let Some(M) = cam16.M {
        (M / env.fl_root) / j_root
    } else {
        0.0004 * spow(cam16.s.unwrap(), 2.0) * (env.a_w + 4.0) / env.c
    };

    let t = spow(alpha * spow(1.64 - spow(0.29, env.n), -0.73), 10.0 / 9.0);

    let et = 0.25 * ((h_rad + 2.0).cos() + 3.8);

    let A = env.a_w * spow(j_root, 2.0 / (env.c * env.z));

    let p1 = 50000.0 / 13.0 * env.nc * env.ncb * et;
    let p2 = A / env.nbb;

    let r = 23.0 * (p2 + 0.305) * zdiv(t, 23.0 * p1 + t * (11.0 * cos_h + 108.0 * sin_h));
    let a = r * cos_h;
    let b = r * sin_h;

    let rgb_c = unadapt(
        multiply_matrices(M1, [p2, a, b]).map(|c| c / 1403.0),
        env.fl,
    );
    let xyz = multiply_matrices(
        CAT16_INV,
        [
            rgb_c[0] * env.d_rgb_inv[0],
            rgb_c[1] * env.d_rgb_inv[1],
            rgb_c[2] * env.d_rgb_inv[2],
        ],
    );

    [xyz[0] / 100.0, xyz[1] / 100.0, xyz[2] / 100.0]
}

fn to_cam16(xyzd65: [f64; 3], env: &Environment) -> Cam16Object {
    let xyz100 = xyzd65.map(|c| c * 100.0);
    let mut tmp: [f64; 3] = [0.0, 0.0, 0.0];
    let tmpmm = multiply_matrices(CAT16, xyz100);
    for i in 0..tmpmm.len() {
        tmp[i] = tmpmm[i] * env.d_rgb[i];
    }
    let rgb_a = adapt(tmp, env.fl);

    let a = rgb_a[0] + (-12.0 * rgb_a[1] + rgb_a[2]) / 11.0;
    let b = (rgb_a[0] + rgb_a[1] - 2.0 * rgb_a[2]) / 9.0;
    let h_rad = ((b.atan2(a) % TAU) + TAU) % TAU;

    let et = 0.25 * ((h_rad + 2.0).cos() + 3.8);

    let t = 50000.0 / 13.0
        * env.nc
        * env.ncb
        * zdiv(
            et * (a * a + b * b).sqrt(),
            rgb_a[0] + rgb_a[1] + 1.05 * rgb_a[2] + 0.305,
        );
    let alpha = spow(t, 0.9) * spow(1.64 - spow(0.29, env.n), 0.73);

    let a = env.nbb * (2.0 * rgb_a[0] + rgb_a[1] + 0.05 * rgb_a[2]);

    let j_root = spow(a / env.a_w, 0.5 * env.c * env.z);

    let j = 100.0 * spow(j_root, 2.0);

    let q = 4.0 / env.c * j_root * (env.a_w + 4.0) * env.fl_root;

    let c = alpha * j_root;

    let m = c * env.fl_root;

    let h = constrain(h_rad * RAD2DEG);

    let H = hue_quadrature(h);

    let s = 50.0 * spow(env.c * alpha / (env.a_w + 4.0), 0.5);

    Cam16Object {
        J: Some(j),
        C: Some(c),
        h: Some(h),
        s: Some(s),
        Q: Some(q),
        M: Some(m),
        H: Some(H),
    }
}

#[derive(Debug)]
struct Cam16Object {
    J: Option<f64>,
    C: Option<f64>,
    h: Option<f64>,
    s: Option<f64>,
    Q: Option<f64>,
    M: Option<f64>,
    H: Option<f64>,
}

fn to_lstar(y: f64) -> f64 {
    let fy = if y > EPSILON_LSTAR {
        y.cbrt()
    } else {
        (KAPPA * y + 16.0) / 116.0
    };
    (116.0 * fy) - 16.0
}

fn from_lstar(lstar: f64) -> f64 {
    if lstar > 8.0 {
        ((lstar + 16.0) / 116.0).powi(3)
    } else {
        lstar / KAPPA
    }
}

fn from_hct(coords: [f64; 3], env: &Environment) -> [f64; 3] {
    let [h, c, t] = coords;
    let mut xyz;
    let mut j;

    if t == 0.0 {
        return [0.0, 0.0, 0.0];
    }

    let y = from_lstar(t);

    if t > 0.0 {
        j = 0.00379058511492914 * t.powi(2) + 0.608983189401032 * t + 0.9155088574762233;
    } else {
        j = 9.514440756550361e-06 * t.powi(2) + 0.08693057439788597 * t - 21.928975842194614;
    }

    const THRESHOLD: f64 = 2e-12;
    const MAX_ATTEMPTS: usize = 15;

    let mut attempt = 0;
    let mut last = f64::INFINITY;
    let mut best = j;

    while attempt <= MAX_ATTEMPTS {
        let cam16 = Cam16Object {
            J: Some(j),
            C: Some(c),
            H: None,
            s: None,
            Q: None,
            M: None,
            h: Some(h),
        };

        xyz = from_cam16(&cam16, env);

        let delta = (xyz[1] - y).abs();
        if delta < last {
            if delta <= THRESHOLD {
                return xyz;
            }
            best = j;
            last = delta;
        }

        j -= (xyz[1] - y) * j / (2.0 * xyz[1]);

        attempt += 1;
    }

    let cam16 = Cam16Object {
        J: Some(best),
        C: Some(c),
        H: None,
        s: None,
        Q: None,
        M: None,
        h: Some(h),
    };

    from_cam16(&cam16, env)
}

fn to_hct(xyz: [f64; 3], env: &Environment) -> [f64; 3] {
    let t = to_lstar(xyz[1]);
    if t == 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let cam16 = to_cam16(xyz, env);
    [constrain(cam16.h.unwrap()), cam16.C.unwrap(), t]
}

#[inline(always)]
pub fn cam16_to_xyz(l: ColorData) -> ColorData {
    let j = l.v[0];
    let m = l.v[1];
    let h = l.v[2];
    let cam16 = Cam16Object {
        J: Some(j),
        C: None,
        H: None,
        s: None,
        Q: None,
        M: Some(m),
        h: Some(h),
    };
    let xyz = from_cam16(&cam16, &ENVS[0]);
    ColorData {
        v: [xyz[0], xyz[1], xyz[2], l.v[3]],
    }
}

#[inline(always)]
pub fn xyz_to_cam16(l: ColorData) -> ColorData {
    let x = l.v[0];
    let y = l.v[1];
    let z = l.v[2];
    let cam16 = to_cam16([x, y, z], &ENVS[0]);
    ColorData {
        v: [cam16.J.unwrap(), cam16.M.unwrap(), cam16.h.unwrap(), l.v[3]],
    }
}

#[inline(always)]
pub fn hct_to_xyz(l: ColorData) -> ColorData {
    let hct = [l.v[0], l.v[1], l.v[2]];
    let xyz = from_hct(hct, &ENVS[1]);
    ColorData {
        v: [xyz[0], xyz[1], xyz[2], l.v[3]],
    }
}

#[inline(always)]
pub fn xyz_to_hct(l: ColorData) -> ColorData {
    let xyz = [l.v[0], l.v[1], l.v[2]];
    let hct = to_hct(xyz, &ENVS[1]);
    ColorData {
        v: [hct[0], hct[1], hct[2], l.v[3]],
    }
}

// fn main() {
//     let viewing_conditions = environment(WHITE, 64.0 / PI * 0.2, 20.0, &SURROUND_MAP[2], false);
//     let viewing_conditions2 = environment(WHITE, 200.0 / PI * from_lstar(50.0), from_lstar(50.0) * 100.0, &SURROUND_MAP[2], false);

//     // [79.10134572991937, 78.2155216870714, 142.22342095435386]
//     let cam16 = Cam16Object {
//         J: Some(79.10134572991937),
//         C: None,
//         H: None,
//         s: None,
//         Q: None,
//         M: Some(78.2155216870714),
//         h: Some(142.22342095435386),
//     };

//     let xyz = from_cam16(&cam16, &viewing_conditions);
//     println!("XYZ: {:?}", xyz);

//     let hct = to_hct(xyz, &viewing_conditions2);
//     println!("HCT: {:?}", hct);

//     let xyz = from_hct(hct, &viewing_conditions2);
//     println!("XYZ2: {:?}", xyz);

//     // let cam16_converted = to_cam16(xyz, &viewing_conditions);
//     // println!("CAM16: {:?}", cam16_converted);
// }
