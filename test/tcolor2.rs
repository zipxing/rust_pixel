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

// Linear RGB to sRGB
fn linear_to_srgb(linear: LinearRGB) -> SRGB {
    let delinearize = |u: f64| {
        if u <= 0.0031308 {
            12.92 * u
        } else {
            1.055 * u.powf(1.0 / 2.4) - 0.055
        }
    };

    SRGB {
        r: delinearize(linear.r),
        g: delinearize(linear.g),
        b: delinearize(linear.b),
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

// XYZ to Linear RGB
fn xyz_to_linear(xyz: XYZ) -> LinearRGB {
    let x = xyz.x;
    let y = xyz.y;
    let z = xyz.z;

    let r = x *  3.2404542 - y * 1.5371385 - z * 0.4985314;
    let g = x * -0.9692660 + y * 1.8760108 + z * 0.0415560;
    let b = x *  0.0556434 - y * 0.2040259 + z * 1.0572252;

    LinearRGB { r, g, b }
}

// XYZ to Oklab
fn xyz_to_oklab(xyz: XYZ) -> Oklab {
    let l = 0.8189330101 * xyz.x + 0.3618667424 * xyz.y - 0.1288597137 * xyz.z;
    let m = 0.0329845436 * xyz.x + 0.9293118715 * xyz.y + 0.0361456387 * xyz.z;
    let s = 0.0482003018 * xyz.x + 0.2643662691 * xyz.y + 0.6338517070 * xyz.z;

    let l = l.cbrt();
    let m = m.cbrt();
    let s = s.cbrt();

    println!("########{} {} {}", l, m, s);

    Oklab {
        l: 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        a: 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        b: 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    }
}

// Oklab to XYZ
fn oklab_to_xyz(lab: Oklab) -> XYZ {
    let l = lab.l;
    let a = lab.a;
    let b = lab.b;

    let l2 = l * 1.00000000 + 0.3963377774 * a + 0.2158037573 * b;
    let m2 = l * 1.00000001 - 0.1055613458 * a - 0.0638541728 * b;
    let s2 = l * 1.00000005 - 0.0894841775 * a - 1.2914855480 * b;

    let l3 = l2 * l2 * l2;
    let m3 = m2 * m2 * m2;
    let s3 = s2 * s2 * s2;

    XYZ {
        x:  1.2270138511 * l3 - 0.5577999806 * m3 + 0.2812561481 * s3,
        y: -0.0405801784 * l3 + 1.1122568696 * m3 - 0.0716766787 * s3,
        z: -0.0763812845 * l3 - 0.4214819784 * m3 + 1.5861632204 * s3,
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

// Oklch to Oklab
fn oklch_to_oklab(lch: Oklch) -> Oklab {
    let a = lch.c * (lch.h * PI / 180.0).cos();
    let b = lch.c * (lch.h * PI / 180.0).sin();

    Oklab {
        l: lch.l,
        a,
        b,
    }
}

// sRGB to Oklch
fn srgb_to_oklch(srgb: SRGB) -> Oklch {
    let linear_rgb = srgb_to_linear(srgb);
    println!("111....linear{:?}", linear_rgb);
    let xyz = linear_to_xyz(linear_rgb);
    println!("111....xyz{:?}", xyz);
    let oklab = xyz_to_oklab(xyz);
    println!("111....oklab{:?}", oklab);
    oklab_to_oklch(oklab)
}

// Oklch to sRGB
fn oklch_to_srgb(oklch: Oklch) -> SRGB {
    let oklab = oklch_to_oklab(oklch);
    println!("222....oklab{:?}", oklab);
    let xyz = oklab_to_xyz(oklab);
    println!("222....xyz{:?}", xyz);
    let linear_rgb = xyz_to_linear(xyz);
    println!("222....linear{:?}", linear_rgb);
    linear_to_srgb(linear_rgb)
}

fn main() {
    let srgb_color = SRGB { r: 1.0, g: 0.0, b: 0.0 };
    let oklch_color = srgb_to_oklch(srgb_color);
    println!("Oklch: {:?}", oklch_color);

    let converted_srgb_color = oklch_to_srgb(oklch_color);
    println!("Converted sRGB: {:?}", converted_srgb_color);
}

