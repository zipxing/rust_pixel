use std::ops::{Index, IndexMut};
// use std::mem;
use ColorSpace::*;

pub enum ColorSpace {
    SRGBA,
    LinearRGBA,
    HSLA,
    HSVA,
    HWBA,
    LabA,
    LchA,
    OKLabA,
    OKLchA,
    XYZA,
}

// pub const COLOR_SPACE_COUNT: usize = mem::variant_count::<ColorSpace>();
pub const COLOR_SPACE_COUNT: usize = 10;

pub type ColorData = [f64; 4];

#[derive(Debug, Clone, Copy)]
pub struct ColorPro {
    pub space_matrix: [Option<ColorData>; COLOR_SPACE_COUNT], 
}

impl Index<ColorSpace> for ColorPro {
    type Output = Option<ColorData>;
    fn index(&self, index: ColorSpace) -> &Self::Output {
        &self.space_matrix[index as usize]
    }
}

impl IndexMut<ColorSpace> for ColorPro {
    fn index_mut(&mut self, index: ColorSpace) -> &mut Self::Output {
        &mut self.space_matrix[index as usize]
    }
}

impl ColorPro {
    pub fn from_space_data(cs: ColorSpace, color: ColorData) -> Self {
        let mut smat = [None; COLOR_SPACE_COUNT];
        smat[cs as usize] = Some(color);
        Self {
            space_matrix: smat,
        }
    }

    pub fn fill_all_spaces(&mut self) -> Result<(), String> {
        self.to_xyza()?;
        let xyza = self[XYZA].unwrap();
        if self[SRGBA] == None {
            self[SRGBA] = Some(xyz_to_srgba(xyza));
        }
        if self[LinearRGBA] == None {
            self[LinearRGBA] = Some(xyz_to_linear_rgba(xyza));
        }
        if self[HSLA] == None {
            self[HSLA] = Some(srgba_to_hsla(self[SRGBA].unwrap()));
        }
        if self[HSVA] == None {
            self[HSVA] = Some(srgba_to_hsva(self[SRGBA].unwrap()));
        }
        if self[HWBA] == None {
            self[HWBA] = Some(srgba_to_hwba(self[SRGBA].unwrap()));
        }
        if self[LabA] == None {
            self[LabA] = Some(xyz_to_laba(xyza));
        }
        if self[LchA] == None {
            self[LchA] = Some(laba_to_lcha(self[LabA].unwrap()));
        }
        if self[OKLabA] == None {
            self[OKLabA] = Some(xyz_to_oklaba(xyza));
        }
        if self[OKLchA] == None {
            self[OKLchA] = Some(oklaba_to_oklcha(self[OKLabA].unwrap()));
        }
        Ok(())
    }

    pub fn to_xyza(&mut self) -> Result<(), String> {
        if self[XYZA] != None {
            return Ok(());
        }

        let xyza; 
        if let Some(srgba) = self[SRGBA] {
            xyza = srgba_to_xyz(srgba);
        } else if let Some(linear_rgba) = self[LinearRGBA] {
            xyza = linear_rgba_to_xyz(linear_rgba);
        } else if let Some(hsla) = self[HSLA] {
            xyza = srgba_to_xyz(hsla_to_srgba(hsla));
        } else if let Some(hsva) = self[HSVA] {
            xyza = srgba_to_xyz(hsva_to_srgba(hsva));
        } else if let Some(hwba) = self[HWBA] {
            xyza = srgba_to_xyz(hwba_to_srgba(hwba));
        } else if let Some(laba) = self[LabA] {
            xyza = laba_to_xyz(laba);
        } else if let Some(lcha) = self[LchA] {
            xyza = lcha_to_xyz(lcha);
        } else if let Some(oklaba) = self[OKLabA] {
            xyza = oklaba_to_xyz(oklaba);
        } else if let Some(oklcha) = self[OKLchA] {
            xyza = oklcha_to_xyz(oklcha);
        } else {
            return Err("No color data available for conversion".to_string());
        };

        self[XYZA]= Some(xyza);
        Ok(())
    }
}

fn srgba_to_xyz(srgba: ColorData) -> ColorData {
    let sr = linearize(srgba[0]);
    let sg = linearize(srgba[1]);
    let sb = linearize(srgba[2]);

    let x = sr * 0.4124564 + sg * 0.3575761 + sb * 0.1804375;
    let y = sr * 0.2126729 + sg * 0.7151522 + sb * 0.0721750;
    let z = sr * 0.0193339 + sg * 0.1191920 + sb * 0.9503041;

    [x, y, z, srgba[3]]
}

fn linearize(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn delinearize(value: f64) -> f64 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn xyz_to_srgba(xyz: ColorData) -> ColorData {
    let r = xyz[0] * 3.2404542 - xyz[1] * 1.5371385 - xyz[2] * 0.4985314;
    let g = -xyz[0] * 0.9692660 + xyz[1] * 1.8760108 + xyz[2] * 0.0415560;
    let b = xyz[0] * 0.0556434 - xyz[1] * 0.2040259 + xyz[2] * 1.0572252;

    let sr = delinearize(r);
    let sg = delinearize(g);
    let sb = delinearize(b);

    [sr, sg, sb, xyz[3]]
}

fn linear_rgba_to_xyz(linear_rgba: ColorData) -> ColorData {
    let x = linear_rgba[0] * 0.4124564 + linear_rgba[1] * 0.3575761 + linear_rgba[2] * 0.1804375;
    let y = linear_rgba[0] * 0.2126729 + linear_rgba[1] * 0.7151522 + linear_rgba[2] * 0.0721750;
    let z = linear_rgba[0] * 0.0193339 + linear_rgba[1] * 0.1191920 + linear_rgba[2] * 0.9503041;

    [x, y, z, linear_rgba[3]]
}

fn xyz_to_linear_rgba(xyz: ColorData) -> ColorData {
    let r = xyz[0] * 3.2404542 - xyz[1] * 1.5371385 - xyz[2] * 0.4985314;
    let g = -xyz[0] * 0.9692660 + xyz[1] * 1.8760108 + xyz[2] * 0.0415560;
    let b = xyz[0] * 0.0556434 - xyz[1] * 0.2040259 + xyz[2] * 1.0572252;

    [r, g, b, xyz[3]]
}

fn hsla_to_srgba(hsla: ColorData) -> ColorData {
    let (h, s, l, a) = (hsla[0], hsla[1], hsla[2], hsla[3]);

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

    [r + m, g + m, b + m, a]
}

fn srgba_to_hsla(srgba: ColorData) -> ColorData {
    let (r, g, b, a) = (srgba[0], srgba[1], srgba[2], srgba[3]);

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

    [h, s, l, a]
}

fn hsva_to_srgba(hsva: ColorData) -> ColorData {
    let (h, s, v, a) = (hsva[0], hsva[1], hsva[2], hsva[3]);

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

    [r + m, g + m, b + m, a]
}

fn srgba_to_hsva(srgba: ColorData) -> ColorData {
    let (r, g, b, a) = (srgba[0], srgba[1], srgba[2], srgba[3]);

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

    [h, s, v, a]
}

fn hwba_to_srgba(hwba: ColorData) -> ColorData {
    let (h, w, b, a) = (hwba[0], hwba[1], hwba[2], hwba[3]);

    let v = 1.0 - b;
    let s = if v == 0.0 { 0.0 } else { 1.0 - w / v };

    hsva_to_srgba([h, s, v, a])
}

fn srgba_to_hwba(srgba: ColorData) -> ColorData {
    let hsva = srgba_to_hsva(srgba);
    let (h, s, v, a) = (hsva[0], hsva[1], hsva[2], hsva[3]);

    let w = v * (1.0 - s);
    let b = 1.0 - v;

    [h, w, b, a]
}

fn xyz_to_laba(xyza: ColorData) -> ColorData {
    let epsilon = 0.008856;
    let kappa = 903.3;

    let xr = xyza[0] / 0.95047;
    let yr = xyza[1] / 1.00000;
    let zr = xyza[2] / 1.08883;

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

    [l, a, b, xyza[3]]
}

fn laba_to_xyz(laba: ColorData) -> ColorData {
    let epsilon = 0.008856;
    let kappa = 903.3;

    let fy = (laba[0] + 16.0) / 116.0;
    let fx = laba[1] / 500.0 + fy;
    let fz = fy - laba[2] / 200.0;

    let xr = if fx.powi(3) > epsilon {
        fx.powi(3)
    } else {
        (116.0 * fx - 16.0) / kappa
    };
    let yr = if laba[0] > kappa * epsilon {
        fy.powi(3)
    } else {
        laba[0] / kappa
    };
    let zr = if fz.powi(3) > epsilon {
        fz.powi(3)
    } else {
        (116.0 * fz - 16.0) / kappa
    };

    let x = xr * 0.95047;
    let y = yr * 1.00000;
    let z = zr * 1.08883;

    [x, y, z, laba[3]]
}

fn laba_to_lcha(laba: ColorData) -> ColorData {
    let l = laba[0];
    let c = (laba[1].powi(2) + laba[2].powi(2)).sqrt();
    let h = laba[2].atan2(laba[1]).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    [l, c, h, laba[3]]
}

fn lcha_to_laba(lcha: ColorData) -> ColorData {
    let l = lcha[0];
    let a = lcha[1] * lcha[2].to_radians().cos();
    let b = lcha[1] * lcha[2].to_radians().sin();

    [l, a, b, lcha[3]]
}

fn lcha_to_xyz(lcha: ColorData) -> ColorData {
    let laba = lcha_to_laba(lcha);
    laba_to_xyz(laba)
}

fn xyz_to_oklaba(xyza: ColorData) -> ColorData {
    let l = 0.4121656120 * xyza[0] + 0.5362752080 * xyza[1] + 0.0514575653 * xyza[2];
    let m = 0.2118591070 * xyza[0] + 0.6807189584 * xyza[1] + 0.1074065790 * xyza[2];
    let s = 0.0883097947 * xyza[0] + 0.2818474174 * xyza[1] + 0.6298501064 * xyza[2];

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    [l, a, b, xyza[3]]
}

fn oklaba_to_xyz(oklaba: ColorData) -> ColorData {
    let l_ = oklaba[0] + 0.3963377774 * oklaba[1] + 0.2158037573 * oklaba[2];
    let m_ = oklaba[0] - 0.1055613458 * oklaba[1] - 0.0638541728 * oklaba[2];
    let s_ = oklaba[0] - 0.0894841775 * oklaba[1] - 1.2914855480 * oklaba[2];

    let l = l_.powi(3);
    let m = m_.powi(3);
    let s = s_.powi(3);

    let x = 0.9999999984 * l + 0.3963377774 * m + 0.2158037573 * s;
    let y = 1.0000000004 * l - 0.1055613458 * m - 0.0638541728 * s;
    let z = 1.0000000043 * l - 0.0894841775 * m - 1.2914855480 * s;

    [x, y, z, oklaba[3]]
}

fn oklaba_to_oklcha(oklaba: ColorData) -> ColorData {
    let l = oklaba[0];
    let a = oklaba[1];
    let b = oklaba[2];

    let c = (a.powi(2) + b.powi(2)).sqrt();
    let h = b.atan2(a).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    [l, c, h, oklaba[3]]
}

fn oklcha_to_oklaba(oklcha: ColorData) -> ColorData {
    let l = oklcha[0];
    let c = oklcha[1];
    let h = oklcha[2].to_radians();

    let a = c * h.cos();
    let b = c * h.sin();

    [l, a, b, oklcha[3]]
}

fn oklcha_to_xyz(oklcha: ColorData) -> ColorData {
    let oklaba = oklcha_to_oklaba(oklcha);
    oklaba_to_xyz(oklaba)
}

fn main() {
    let mut color = ColorPro::from_space_data(SRGBA, [0.5, 0.5, 0.5, 1.0]);
    let _ = color.fill_all_spaces();
    println!("{:?}", color.space_matrix);
}
