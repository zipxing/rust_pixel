use std::f64::consts::PI;

#[derive(Debug)]
struct SRGB {
    r: f64,
    g: f64,
    b: f64,
}

#[derive(Debug)]
struct LinearRGB {
    r: f64,
    g: f64,
    b: f64,
}

#[derive(Debug)]
struct XYZ {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug)]
struct Oklab {
    l: f64,
    a: f64,
    b: f64,
}

#[derive(Debug)]
struct Oklch {
    l: f64,
    c: f64,
    h: f64,
}

// sRGB to Linear RGB
fn srgb_to_linear(rgb: SRGB) -> LinearRGB {
    let linearize = |u: f64| {
        if u <= 0.04045 {
            u / 12.92
        } else {
            ((u + 0.055) / 1.055).powf(2.4)
        }
    };

    LinearRGB {
        r: linearize(rgb.r),
        g: linearize(rgb.g),
        b: linearize(rgb.b),
    }
}

// Linear RGB to XYZ
fn linear_to_xyz(rgb: LinearRGB) -> XYZ {
    let r = rgb.r;
    let g = rgb.g;
    let b = rgb.b;

    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

    XYZ { x, y, z }
}

// XYZ to Oklab
fn xyz_to_oklab(xyz: XYZ) -> Oklab {
    let l = 0.8189330101 * xyz.x + 0.3618667424 * xyz.y - 0.1288597137 * xyz.z;
    let m = 0.0329845436 * xyz.x + 0.9293118715 * xyz.y + 0.0361456387 * xyz.z;
    let s = 0.0482003018 * xyz.x + 0.2643662691 * xyz.y + 0.6338517070 * xyz.z;

    // let l = 0.4121656120 * xyz.x + 0.5362752080 * xyz.y + 0.0514575653 * xyz.z;
    // let m = 0.2118591070 * xyz.x + 0.6807189570 * xyz.y + 0.1074065790 * xyz.z;
    // let s = 0.0883097947 * xyz.x + 0.2818474174 * xyz.y + 0.6302613616 * xyz.z;

    let l = l.cbrt();
    let m = m.cbrt();
    let s = s.cbrt();

    Oklab {
        l: 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        a: 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        b: 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    }
}

// Oklab to Oklch
fn oklab_to_oklch(lab: Oklab) -> Oklch {
    let c = (lab.a * lab.a + lab.b * lab.b).sqrt();
    let h = lab.b.atan2(lab.a) * 180.0 / PI;
    let h = if h < 0.0 { h + 360.0 } else { h };

    Oklch {
        l: lab.l,
        c,
        h,
    }
}

// sRGB to Oklch
fn srgb_to_oklch(srgb: SRGB) -> Oklch {
    let linear_rgb = srgb_to_linear(srgb);
    let xyz = linear_to_xyz(linear_rgb);
    let oklab = xyz_to_oklab(xyz);
    oklab_to_oklch(oklab)
}

fn main() {
    let srgb_color = SRGB { r: 1.0, g: 0.0, b: 0.0 };
    let oklch_color = srgb_to_oklch(srgb_color);
    println!("{:?}", oklch_color);
}

