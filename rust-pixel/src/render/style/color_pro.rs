// https://products.aspose.com/svg/zh/net/color-converter/rgb-to-hwb/
// use log::info;
use std::f64::consts::PI;
use std::fmt;
use std::ops::{Index, IndexMut};
use ColorSpace::*;

pub const COLOR_SPACE_COUNT: usize = 11;

#[derive(Debug, Clone, Copy)]
pub enum ColorSpace {
    SRGBA,
    LinearRGBA,
    CMYK,
    HSLA,
    HSVA,
    HWBA,
    LabA,
    LchA,
    OKLabA,
    OKLchA,
    XYZA,
}

pub const COLOR_SPACE_NAME: [&'static str; COLOR_SPACE_COUNT] = [
    "srgb",
    "linear_rgb",
    "cmyk",
    "hsl",
    "hsv",
    "hwb",
    "lab",
    "lch",
    "oklab",
    "oklch",
    "xyz",
];

pub type ColorData = [f64; 4];

// wrap for debug trait...
pub struct ColorDataWrap(pub ColorData);

impl fmt::Debug for ColorDataWrap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:.6} {:.6} {:.6} {:.6}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

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
        let mut s = Self { space_matrix: smat };
        let _ = s.fill_all_spaces();
        s
    }

    pub fn fill_all_spaces(&mut self) -> Result<(), String> {
        self.to_xyza()?;
        let xyza = self[XYZA].unwrap();
        self.set_data(SRGBA, xyz_to_srgba(xyza));
        let srgba = self[SRGBA].unwrap();
        self.set_data(CMYK, srgba_to_cmyk(srgba));
        self.set_data(LinearRGBA, xyz_to_linear(xyza));
        self.set_data(HSLA, srgba_to_hsla(srgba));
        self.set_data(HSVA, srgba_to_hsva(srgba));
        self.set_data(HWBA, srgba_to_hwba(srgba));
        self.set_data(LabA, xyz_to_laba(xyza));
        self.set_data(LchA, laba_to_lcha(self[LabA].unwrap()));
        self.set_data(OKLabA, xyz_to_oklaba(xyza));
        self.set_data(OKLchA, oklaba_to_oklcha(self[OKLabA].unwrap()));
        Ok(())
    }

    pub fn get_srgba_u8(&mut self) -> (u8, u8, u8, u8) {
        let _ = self.fill_all_spaces();
        let srgba = self[SRGBA].unwrap();
        let r = srgba[0];
        let g = srgba[1];
        let b = srgba[2];
        let a = srgba[3];
        (
            if r < 0.0 { 0 } else { (255.0 * r) as u8 },
            if g < 0.0 { 0 } else { (255.0 * g) as u8 },
            if b < 0.0 { 0 } else { (255.0 * b) as u8 },
            (255.0 * a) as u8,
        )
    }

    fn set_data(&mut self, cs: ColorSpace, data: ColorData) {
        if self[cs] == None {
            self[cs] = Some(data);
        }
    }

    fn to_xyza(&mut self) -> Result<(), String> {
        if self[XYZA] != None {
            return Ok(());
        }

        if let Some(srgba) = self[SRGBA] {
            let linear;
            let xyza;
            (xyza, linear) = srgba_to_xyz(srgba);
            self.set_data(LinearRGBA, linear);
            self.set_data(XYZA, xyza);
        }

        if let Some(cmyk) = self[CMYK] {
            let linear;
            let xyza;
            let srgba = cmyk_to_srgba(cmyk);
            self.set_data(SRGBA, srgba);
            (xyza, linear) = srgba_to_xyz(srgba);
            self.set_data(LinearRGBA, linear);
            self.set_data(XYZA, xyza);
        }

        if let Some(linear_rgba) = self[LinearRGBA] {
            let xyza;
            xyza = linear_to_xyz(linear_rgba);
            self.set_data(XYZA, xyza);
        }

        if let Some(hsla) = self[HSLA] {
            let linear;
            let xyza;
            let srgba = hsla_to_srgba(hsla);
            self.set_data(SRGBA, srgba);
            (xyza, linear) = srgba_to_xyz(srgba);
            self.set_data(LinearRGBA, linear);
            self.set_data(XYZA, xyza);
        }

        if let Some(hsva) = self[HSVA] {
            let linear;
            let xyza;
            let srgba = hsva_to_srgba(hsva);
            self.set_data(SRGBA, srgba);
            (xyza, linear) = srgba_to_xyz(srgba);
            self.set_data(LinearRGBA, linear);
            self.set_data(XYZA, xyza);
        }

        if let Some(hwba) = self[HWBA] {
            let linear;
            let xyza;
            let srgba = hwba_to_srgba(hwba);
            self.set_data(SRGBA, srgba);
            (xyza, linear) = srgba_to_xyz(srgba);
            self.set_data(LinearRGBA, linear);
            self.set_data(XYZA, xyza);
        }

        if let Some(laba) = self[LabA] {
            let xyza;
            xyza = laba_to_xyz(laba);
            self.set_data(XYZA, xyza);
        }

        if let Some(lcha) = self[LchA] {
            let laba;
            let xyza;
            (xyza, laba) = lcha_to_xyz(lcha);
            self.set_data(LabA, laba);
            self.set_data(XYZA, xyza);
        }

        if let Some(oklaba) = self[OKLabA] {
            let xyza;
            xyza = oklaba_to_xyz(oklaba);
            self.set_data(XYZA, xyza);
        }

        if let Some(oklcha) = self[OKLchA] {
            let xyza;
            let oklaba;
            (xyza, oklaba) = oklcha_to_xyz(oklcha);
            self.set_data(XYZA, xyza);
            self.set_data(OKLabA, oklaba);
        }

        if self[XYZA] == None {
            return Err("No color data available for conversion".to_string());
        };

        Ok(())
    }
}

#[inline(always)]
fn linearize(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[inline(always)]
fn delinearize(value: f64) -> f64 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[inline(always)]
fn srgba_to_linear(s: ColorData) -> ColorData {
    let r = linearize(s[0]);
    let g = linearize(s[1]);
    let b = linearize(s[2]);

    [r, g, b, s[3]]
}

#[inline(always)]
fn linear_to_srgba(l: ColorData) -> ColorData {
    let sr = delinearize(l[0]);
    let sg = delinearize(l[1]);
    let sb = delinearize(l[2]);

    [sr, sg, sb, l[3]]
}

#[inline(always)]
fn linear_to_xyz(l: ColorData) -> ColorData {
    let x = l[0] * 0.4124564 + l[1] * 0.3575761 + l[2] * 0.1804375;
    let y = l[0] * 0.2126729 + l[1] * 0.7151522 + l[2] * 0.0721750;
    let z = l[0] * 0.0193339 + l[1] * 0.1191920 + l[2] * 0.9503041;

    [x, y, z, l[3]]
}

#[inline(always)]
fn xyz_to_linear(xyz: ColorData) -> ColorData {
    let r = xyz[0] * 3.2404542 - xyz[1] * 1.5371385 - xyz[2] * 0.4985314;
    let g = xyz[0] * -0.9692660 + xyz[1] * 1.8760108 + xyz[2] * 0.0415560;
    let b = xyz[0] * 0.0556434 - xyz[1] * 0.2040259 + xyz[2] * 1.0572252;

    [r, g, b, xyz[3]]
}

#[inline(always)]
fn srgba_to_xyz(srgba: ColorData) -> (ColorData, ColorData) {
    let l = srgba_to_linear(srgba);
    let xyza = linear_to_xyz(l);
    (xyza, l)
}

#[inline(always)]
fn xyz_to_srgba(xyz: ColorData) -> ColorData {
    let l = xyz_to_linear(xyz);
    linear_to_srgba(l)
}

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
fn hwba_to_srgba(hwba: ColorData) -> ColorData {
    let (h, w, b, a) = (hwba[0], hwba[1], hwba[2], hwba[3]);

    let v = 1.0 - b;
    let s = if v == 0.0 { 0.0 } else { 1.0 - w / v };

    hsva_to_srgba([h, s, v, a])
}

#[inline(always)]
fn srgba_to_hwba(srgba: ColorData) -> ColorData {
    let hsva = srgba_to_hsva(srgba);
    let (h, s, v, a) = (hsva[0], hsva[1], hsva[2], hsva[3]);

    let w = v * (1.0 - s);
    let b = 1.0 - v;

    [h, w, b, a]
}

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
fn laba_to_lcha(laba: ColorData) -> ColorData {
    let l = laba[0];
    let a = laba[1];
    let b = laba[2];
    let c = (a * a + b * b).sqrt();
    let h = f64::atan2(b, a);
    let h = h.to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    [l, c, h, laba[3]]
}

#[inline(always)]
fn lcha_to_laba(lcha: ColorData) -> ColorData {
    let l = lcha[0];
    let a = lcha[1] * lcha[2].to_radians().cos();
    let b = lcha[1] * lcha[2].to_radians().sin();

    [l, a, b, lcha[3]]
}

#[inline(always)]
fn lcha_to_xyz(lcha: ColorData) -> (ColorData, ColorData) {
    let laba = lcha_to_laba(lcha);
    (laba_to_xyz(laba), laba)
}

#[inline(always)]
fn xyz_to_oklaba(xyza: ColorData) -> ColorData {
    let l = 0.8189330101 * xyza[0] + 0.3618667424 * xyza[1] - 0.1288597137 * xyza[2];
    let m = 0.0329845436 * xyza[0] + 0.9293118715 * xyza[1] + 0.0361456387 * xyza[2];
    let s = 0.0482003018 * xyza[0] + 0.2643662691 * xyza[1] + 0.6338517070 * xyza[2];

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    [l, a, b, xyza[3]]
}

#[inline(always)]
fn oklaba_to_xyz(oklaba: ColorData) -> ColorData {
    let l = (1.00000000 * oklaba[0] + 0.39633779 * oklaba[1] + 0.21580376 * oklaba[2]).powi(3);
    let m = (1.00000001 * oklaba[0] - 0.10556134 * oklaba[1] - 0.06385417 * oklaba[2]).powi(3);
    let s = (1.00000005 * oklaba[0] - 0.08948418 * oklaba[1] - 1.29148554 * oklaba[2]).powi(3);

    let x = 1.22701385 * l - 0.55779998 * m + 0.28125615 * s;
    let y = -0.04058018 * l + 1.11225687 * m - 0.07167668 * s;
    let z = -0.07638128 * l - 0.42148198 * m + 1.58616322 * s;

    [x, y, z, oklaba[3]]
}

#[inline(always)]
fn oklaba_to_oklcha(oklaba: ColorData) -> ColorData {
    let l = oklaba[0];
    let a = oklaba[1];
    let b = oklaba[2];

    let c = (a.powi(2) + b.powi(2)).sqrt();
    let h = b.atan2(a).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    [l, c, h, oklaba[3]]
}

#[inline(always)]
fn oklcha_to_oklaba(oklcha: ColorData) -> ColorData {
    let l = oklcha[0];
    let c = oklcha[1];
    let h = oklcha[2].to_radians();

    let a = c * h.cos();
    let b = c * h.sin();

    [l, a, b, oklcha[3]]
}

#[inline(always)]
fn oklcha_to_xyz(oklcha: ColorData) -> (ColorData, ColorData) {
    let oklaba = oklcha_to_oklaba(oklcha);
    (oklaba_to_xyz(oklaba), oklaba)
}

#[inline(always)]
fn srgba_to_cmyk(srgb: ColorData) -> ColorData {
    let r = srgb[0];
    let g = srgb[1];
    let b = srgb[2];

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

    [c, m, y, k]
}

#[inline(always)]
fn cmyk_to_srgba(cmyk: ColorData) -> ColorData {
    let c = cmyk[0];
    let m = cmyk[1];
    let y = cmyk[2];
    let k = cmyk[3];

    let r = (1.0 - c) * (1.0 - k);
    let g = (1.0 - m) * (1.0 - k);
    let b = (1.0 - y) * (1.0 - k);

    [r, g, b, 1.0]
}

pub fn delta_e_cie76(lab1: ColorData, lab2: ColorData) -> f64 {
    ((lab1[0] - lab2[0]).powi(2) + (lab1[1] - lab2[1]).powi(2) + (lab1[2] - lab2[2]).powi(2)).sqrt()
}

fn deg_to_rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

fn rad_to_deg(rad: f64) -> f64 {
    rad * 180.0 / PI
}

pub fn delta_e_ciede2000(lab1: ColorData, lab2: ColorData) -> f64 {
    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    let delta_l_prime = lab2[0] - lab1[0];
    let l_bar = (lab1[0] + lab2[0]) / 2.0;
    let c1 = (lab1[1].powi(2) + lab1[2].powi(2)).sqrt();
    let c2 = (lab2[1].powi(2) + lab2[2].powi(2)).sqrt();
    let c_bar = (c1 + c2) / 2.0;
    let g = 0.5 * (1.0 - (c_bar.powi(7) / (c_bar.powi(7) + 25.0_f64.powi(7))).sqrt());

    let a1_prime = lab1[1] * (1.0 + g);
    let a2_prime = lab2[1] * (1.0 + g);
    let c1_prime = (a1_prime.powi(2) + lab1[2].powi(2)).sqrt();
    let c2_prime = (a2_prime.powi(2) + lab2[2].powi(2)).sqrt();
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;
    let delta_c_prime = c2_prime - c1_prime;

    let h1_prime = rad_to_deg(lab1[2].atan2(a1_prime)).rem_euclid(360.0);
    let h2_prime = rad_to_deg(lab2[2].atan2(a2_prime)).rem_euclid(360.0);
    let delta_h_prime = if (c1_prime * c2_prime).abs() < 1e-4 {
        0.0
    } else if (h2_prime - h1_prime).abs() <= 180.0 {
        h2_prime - h1_prime
    } else if h2_prime <= h1_prime {
        h2_prime - h1_prime + 360.0
    } else {
        h2_prime - h1_prime - 360.0
    };
    let delta_h_prime_radians = deg_to_rad(delta_h_prime);
    let delta_h_prime_2 = 2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_prime_radians / 2.0).sin();

    let h_bar_prime = if (c1_prime * c2_prime).abs() < 1e-4 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) / 2.0
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) / 2.0
    } else {
        (h1_prime + h2_prime - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * (deg_to_rad(h_bar_prime - 30.0)).cos()
        + 0.24 * (deg_to_rad(2.0 * h_bar_prime)).cos()
        + 0.32 * (deg_to_rad(3.0 * h_bar_prime + 6.0)).cos()
        - 0.20 * (deg_to_rad(4.0 * h_bar_prime - 63.0)).cos();
    let s_l = 1.0 + (0.015 * (l_bar - 50.0).powi(2) / (20.0 + (l_bar - 50.0).powi(2)).sqrt());
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;
    let r_t = -2.0 * (deg_to_rad(60.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp())).sin();

    ((delta_l_prime / (k_l * s_l)).powi(2)
        + (delta_c_prime / (k_c * s_c)).powi(2)
        + (delta_h_prime_2 / (k_h * s_h)).powi(2)
        + r_t * (delta_c_prime / (k_c * s_c)) * (delta_h_prime_2 / (k_h * s_h)))
        .sqrt()
}

fn interpolate(a: f64, b: f64, fra: Fraction) -> f64 {
    a + fra.value() * (b - a)
}

fn mix(c1: ColorData, c2: ColorData, fra: Fraction) -> ColorData {
    [
        interpolate(c1[0], c2[0], fra),
        interpolate(c1[1], c2[1], fra),
        interpolate(c1[2], c2[2], fra),
        interpolate(c1[3], c2[3], fra),
    ]
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
pub struct ColorScale {
    color_stops: Vec<ColorStop>,
}

impl ColorScale {
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