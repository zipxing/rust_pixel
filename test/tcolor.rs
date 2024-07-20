#[derive(Debug, Copy, Clone)]
struct RGB {
    r: f64,
    g: f64,
    b: f64,
}

#[derive(Debug, Copy, Clone)]
struct XYZ {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Copy, Clone)]
struct Lab {
    l: f64,
    a: f64,
    b: f64,
}

#[derive(Debug, Copy, Clone)]
struct LCh {
    l: f64,
    c: f64,
    h: f64,
}

// Convert sRGB to linear RGB
fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

// Convert linear RGB to XYZ
fn linear_rgb_to_xyz(rgb: RGB) -> XYZ {
    XYZ {
        x: 0.4124564 * rgb.r + 0.3575761 * rgb.g + 0.1804375 * rgb.b,
        y: 0.2126729 * rgb.r + 0.7151522 * rgb.g + 0.0721750 * rgb.b,
        z: 0.0193339 * rgb.r + 0.1191920 * rgb.g + 0.9503041 * rgb.b,
    }
}

// Convert XYZ to Lab
fn xyz_to_lab(xyz: XYZ) -> Lab {
    let xr = xyz.x / 95.047;
    let yr = xyz.y / 100.0;
    let zr = xyz.z / 108.883;

    let fx = if xr > 0.008856 {
        xr.powf(1.0 / 3.0)
    } else {
        (7.787 * xr) + (16.0 / 116.0)
    };

    let fy = if yr > 0.008856 {
        yr.powf(1.0 / 3.0)
    } else {
        (7.787 * yr) + (16.0 / 116.0)
    };

    let fz = if zr > 0.008856 {
        zr.powf(1.0 / 3.0)
    } else {
        (7.787 * zr) + (16.0 / 116.0)
    };

    Lab {
        l: (116.0 * fy) - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

// Convert Lab to LCh
fn lab_to_lch(lab: Lab) -> LCh {
    let c = (lab.a.powi(2) + lab.b.powi(2)).sqrt();
    let mut h = lab.b.atan2(lab.a).to_degrees();
    if h < 0.0 {
        h += 360.0;
    }

    LCh {
        l: lab.l,
        c: c,
        h: h,
    }
}

// Convert hex to RGB
fn hex_to_rgb(hex: &str) -> RGB {
    let r = i64::from_str_radix(&hex[1..3], 16).unwrap() as f64 / 255.0;
    let g = i64::from_str_radix(&hex[3..5], 16).unwrap() as f64 / 255.0;
    let b = i64::from_str_radix(&hex[5..7], 16).unwrap() as f64 / 255.0;

    RGB { r, g, b }
}

fn main() {
    let hex = "#7f7f7f";
    let rgb = hex_to_rgb(hex);

    let linear_rgb = RGB {
        r: srgb_to_linear(rgb.r),
        g: srgb_to_linear(rgb.g),
        b: srgb_to_linear(rgb.b),
    };

    let xyz = linear_rgb_to_xyz(linear_rgb);
    let lab = xyz_to_lab(xyz);
    let lch = lab_to_lch(lab);

    println!("LCh: {:?}", lch);
}

