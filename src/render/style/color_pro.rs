// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Defines Professional Color
///
/// Refer:
///   https://en.wikipedia.org/wiki/Color_space
///   https://products.aspose.com/svg/zh/net/color-converter/rgb-to-hwb/
///
/// ColorSpace range:
///   sRGB r: 0.0 - 1.0 g: 0.0 - 1.0 b: 0.0 - 1.0
///   Linear RGB r: 0.0 - 1.0 g: 0.0 - 1.0 b: 0.0 - 1.0
///   CMYK c: 0.0 - 1.0 m: 0.0 - 1.0 y: 0.0 - 1.0 k: 0.0 - 1.0
///   HSLA h: 0.0 - 360.0 (degrees) s: 0.0 - 1.0 l: 0.0 - 1.0 a: 0.0 - 1.0
///   HSVA h: 0.0 - 360.0 (degrees) s: 0.0 - 1.0 v: 0.0 - 1.0 a: 0.0 - 1.0
///   HWBA h: 0.0 - 360.0 (degrees) w: 0.0 - 1.0 b: 0.0 - 1.0 a: 0.0 - 1.0
///   Lab l: 0.0 - 100.0 a: -128.0 - 127.0 b: -128.0 - 127.0
///   LCH l: 0.0 - 100.0 c: 0.0 - 100.0 (approximate, can exceed 100) h: 0.0 - 360.0 (degrees)
///   Oklab l: 0.0 - 1.0 a: -0.5 - 0.5 (approximate range) b: -0.5 - 0.5 (approximate range)
///   Oklch l: 0.0 - 1.0 c: 0.0 - 1.0 (approximate range) h: 0.0 - 360.0 (degrees)
///   XYZ x: 0.0 - 1.0 (normalized range) y: 0.0 - 1.0 (normalized range) z: 0.0 - 1.0 (normalized range)
///
/// Example:
/// ```
/// // test and print colorspace data...
/// let color = ColorPro::from_space_f64(SRGBA, 1.0, 0.0, 0.0, 1.0);
/// for i in 0..COLOR_SPACE_COUNT {
///     info!(
///         "{}:{:?}",
///         ColorSpace::from_usize(i).unwrap(),
///         color.space_matrix[i].unwrap()
///     );
/// }
/// ```
///
/// Example:
/// ```
/// // test delta_e...
/// let c1 = ColorPro::from_space_f64(LabA, 50.0, 0.8, -80.0, 1.0);
/// let c2 = ColorPro::from_space_f64(LabA, 100.0, 1.2, 90.0, 1.0);
/// let d1 = delta_e_cie76(c1[LabA].unwrap(), c2[LabA].unwrap());
/// let d2 = delta_e_ciede2000(c1[LabA].unwrap(), c2[LabA].unwrap());
/// info!("d76...{}, d2000...{}", d1, d2);
/// ````
///
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::f64::consts::PI;
use std::fmt;
use std::ops::{Index, IndexMut};
// use log::info;
use ColorSpace::*;

/// google hct color_space
mod hct;
use hct::*;

/// rgba, linear_rgba color_space
mod rgb;
use rgb::*;

/// hsv, hsl, hwa
mod hsv;
use hsv::*;

/// lab, lch, oklab, oklch
mod lab;
use lab::*;

/// cmyk
mod cmyk;
use cmyk::*;

/// color delta_e 
mod delta;
pub use delta::*;

/// color gradient
mod gradient;
pub use gradient::*;

// 0.3127 / 0.3290  (1.0 - 0.3127 - 0.3290) / 0.3290
pub const WHITE: [f64; 3] = [0.9504559270516716, 1.0, 1.0890577507598784];
pub const EPSILON_LSTAR: f64 = 216.0 / 24389.0;
pub const KAPPA: f64 = 24389.0 / 27.0;

pub const COLOR_SPACE_COUNT: usize = 13;

#[derive(Debug, Clone, Copy, FromPrimitive)]
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
    CAM16A,
    HCTA,
    XYZA,
}

impl fmt::Display for ColorSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let css = ["rgb", "lrgb", "cmyk", "hsl", "hsv", "hwb", "lab", "lch", "oklab", "oklch", "cam16", "hct", "xyz"];
        write!(f, "{:5}", css[*self as usize])
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorData {
    pub v: [f64; 4],
}

impl fmt::Debug for ColorData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:8.3} {:8.3} {:8.3}",
            self.v[0], self.v[1], self.v[2]
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    /// build colorpro with special colorspace and fill all colorspace data
    pub fn from_space(cs: ColorSpace, color: ColorData) -> Self {
        let mut smat = [None; COLOR_SPACE_COUNT];
        smat[cs as usize] = Some(color);
        let mut s = Self { space_matrix: smat };
        let _ = s.fill_all_spaces();
        s
    }

    /// build colorpro with special colorspace and f64 parameters
    pub fn from_space_f64(cs: ColorSpace, v0: f64, v1: f64, v2: f64, v3: f64) -> Self {
        let mut smat = [None; COLOR_SPACE_COUNT];
        let color = ColorData {
            v: [v0, v1, v2, v3],
        };
        smat[cs as usize] = Some(color);
        let mut s = Self { space_matrix: smat };
        let _ = s.fill_all_spaces();
        s
    }

    /// build colorpro with special colorspace and u8 parameters
    /// only vaild for srgb, linear, cmyk, xyz
    pub fn from_space_u8(cs: ColorSpace, v0: u8, v1: u8, v2: u8, v3: u8) -> Self {
        let mut smat = [None; COLOR_SPACE_COUNT];
        let mut color = ColorData {
            v: [0.0, 0.0, 0.0, 1.0],
        };
        match cs {
            SRGBA | LinearRGBA | CMYK | XYZA => {
                color = ColorData {
                    v: [
                        v0 as f64 / 255.0,
                        v1 as f64 / 255.0,
                        v2 as f64 / 255.0,
                        v3 as f64 / 255.0,
                    ],
                }
            }
            _ => {}
        };
        smat[cs as usize] = Some(color);
        let mut s = Self { space_matrix: smat };
        let _ = s.fill_all_spaces();
        s
    }

    /// lightness from 0.0(black) to 1.0(white)
    pub fn from_graytone(l: f64) -> Self {
        Self::from_space(
            HSLA,
            ColorData {
                v: [0.0, 0.0, l, 1.0],
            },
        )
    }

    pub fn get_srgba_u8(&self) -> (u8, u8, u8, u8) {
        let srgba = self[SRGBA].unwrap();
        let r = srgba.v[0];
        let g = srgba.v[1];
        let b = srgba.v[2];
        let a = srgba.v[3];
        (
            if r < 0.0 {
                0
            } else {
                (255.0 * r).round() as u8
            },
            if g < 0.0 {
                0
            } else {
                (255.0 * g).round() as u8
            },
            if b < 0.0 {
                0
            } else {
                (255.0 * b).round() as u8
            },
            (255.0 * a).round() as u8,
        )
    }

    /// See: <https://www.w3.org/TR/2008/REC-WCAG20-20081211/#relativeluminancedef>
    pub fn luminance(&self) -> f64 {
        let c = self[LinearRGBA].unwrap();
        0.2126 * c.v[0] + 0.7152 * c.v[1] + 0.0722 * c.v[2]
    }

    pub fn is_dark(&self) -> bool {
        self.luminance() <= 0.179
    }

    pub fn brightness(&self) -> f64 {
        let c = self[SRGBA].unwrap();
        0.299 * c.v[0] + 0.587 * c.v[1] + 0.114 * c.v[2]
    }

    pub fn chroma(&self) -> f64 {
        let c = self[OKLchA].unwrap();
        c.v[1]
    }

    pub fn hue(&self) -> f64 {
        let c = self[OKLchA].unwrap();
        c.v[2]
    }

    fn fill_all_spaces(&mut self) -> Result<(), String> {
        self.make_xyza()?;
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
        self.set_data(CAM16A, xyz_to_cam16(xyza));
        self.set_data(HCTA, xyz_to_hct(xyza));
        Ok(())
    }

    fn set_data(&mut self, cs: ColorSpace, data: ColorData) {
        if self[cs].is_none() {
            self[cs] = Some(data);
        }
    }

    fn make_xyza(&mut self) -> Result<(), String> {
        if self[XYZA].is_some() {
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
            
            let xyza = linear_to_xyz(linear_rgba);
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
            
            let xyza = laba_to_xyz(laba);
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
            
            let xyza = oklaba_to_xyz(oklaba);
            self.set_data(XYZA, xyza);
        }

        if let Some(oklcha) = self[OKLchA] {
            let xyza;
            let oklaba;
            (xyza, oklaba) = oklcha_to_xyz(oklcha);
            self.set_data(XYZA, xyza);
            self.set_data(OKLabA, oklaba);
        }

        if let Some(cam16) = self[CAM16A] {
            
            let xyza = cam16_to_xyz(cam16);
            self.set_data(XYZA, xyza);
            self.set_data(CAM16A, cam16);
        }

        if let Some(hct) = self[HCTA] {
            
            let xyza = hct_to_xyz(hct);
            self.set_data(XYZA, xyza);
            self.set_data(HCTA, hct);
        }

        if self[XYZA].is_none() {
            return Err("No color data available for conversion".to_string());
        };

        Ok(())
    }
}
