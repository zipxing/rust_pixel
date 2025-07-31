// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Symbol processing utilities for RustPixel.
//! 
//! A symbol comprises a point vector with width * height elements.
//! This module provides functions for symbol manipulation, color processing, and block binarization.

use crate::render::style::ANSI_COLOR_RGB;
use deltae::*;
#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
use image::{DynamicImage, GenericImageView, Rgb};
use lab::Lab;
#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Result of block binarization processing
/// 
/// Contains the binary pattern and extracted colors for a symbol block
#[derive(Debug, Clone)]
pub struct BinarizedBlock {
    /// Binary pattern (0 = background, 1 = foreground)
    pub bitmap: Vec<Vec<u8>>,
    /// Block size (width and height)
    pub width: usize,
    pub height: usize,
    /// Average foreground color
    pub foreground_color: RGB,
    /// Average background color  
    pub background_color: RGB,
    /// Threshold value used for binarization
    pub threshold: u8,
}

/// Configuration for block binarization
#[derive(Debug, Clone)]
pub struct BinarizationConfig {
    /// Minimum contrast ratio (0.0-1.0) for triggering Otsu's algorithm
    /// Recommended: 0.1 (10% brightness difference)
    pub min_contrast_ratio: f32,
}

// find big image background colors...
#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
pub fn find_background_color(img: &DynamicImage, w: u32, h: u32) -> u32 {
    // color_u32 : (first_x, first_y, count)
    let mut cc: HashMap<u32, (u32, u32, u32)> = HashMap::new();
    for i in 0..h {
        for j in 0..w {
            let p = img.get_pixel(j, i);
            let k: u32 = ((p[0] as u32) << 24)
                + ((p[1] as u32) << 16)
                + ((p[2] as u32) << 8)
                + (p[3] as u32);
            (*cc.entry(k).or_insert((j, i, 0))).2 += 1;
        }
    }
    let mut cv: Vec<_> = cc.iter().collect();
    cv.sort_by(|b, a| (&a.1 .2).cmp(&b.1 .2));
    *cv[0].0
}

pub fn find_best_color(color: RGB) -> usize {
    let mut min_mse = f32::MAX;
    let mut best_match = 0;

    for (i, pcolor) in ANSI_COLOR_RGB.iter().enumerate() {
        let pcrgb = RGB {
            r: pcolor[0],
            g: pcolor[1],
            b: pcolor[2],
        };
        let mse = color_distance_rgb(&pcrgb, &color);

        if mse < min_mse {
            min_mse = mse;
            best_match = i;
        }
    }

    best_match
}

pub fn find_best_color_u32(c: u32) -> usize {
    find_best_color(RGB {
        r: (c >> 24) as u8,
        g: (c >> 16) as u8,
        b: (c >> 8) as u8,
    })
}

// get color distance
pub fn color_distance_rgb(e1: &RGB, e2: &RGB) -> f32 {
    let l1 = Lab::from_rgb(&[e1.r, e1.g, e1.b]);
    let l2 = Lab::from_rgb(&[e2.r, e2.g, e2.b]);
    let lab1 = LabValue {
        l: l1.l,
        a: l1.a,
        b: l1.b,
    };
    let lab2 = LabValue {
        l: l2.l,
        a: l2.a,
        b: l2.b,
    };
    *DeltaE::new(&lab1, &lab2, DE2000).value()
}

pub fn luminance(e1: u32) -> f32 {
    let e1r = (e1 >> 24 & 0xff) as u8;
    let e1g = (e1 >> 16 & 0xff) as u8;
    let e1b = (e1 >> 8 & 0xff) as u8;
    0.299 * e1r as f32 + 0.587 * e1g as f32 + 0.114 * e1b as f32
}

// get color distance
pub fn color_distance(e1: u32, e2: u32) -> f32 {
    let e1r = (e1 >> 24 & 0xff) as u8;
    let e1g = (e1 >> 16 & 0xff) as u8;
    let e1b = (e1 >> 8 & 0xff) as u8;
    let e2r = (e2 >> 24 & 0xff) as u8;
    let e2g = (e2 >> 16 & 0xff) as u8;
    let e2b = (e2 >> 8 & 0xff) as u8;

    let l1 = Lab::from_rgb(&[e1r, e1g, e1b]);
    let l2 = Lab::from_rgb(&[e2r, e2g, e2b]);
    let lab1 = LabValue {
        l: l1.l,
        a: l1.a,
        b: l1.b,
    };
    let lab2 = LabValue {
        l: l2.l,
        a: l2.a,
        b: l2.b,
    };
    *DeltaE::new(&lab1, &lab2, DE2000).value()
}


#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub width: u8,
    pub height: u8,
    pub is_binary: bool,
    pub fore_color: u32,
    pub back_color: u32,
    pub data: Vec<Vec<u32>>,
    pub binary_data: Vec<Vec<u8>>,
}

#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
impl Symbol {
    pub fn new(width: u8, height: u8, is_binary: bool, img: &DynamicImage) -> Self {
        let mut data = vec![];
        for i in 0..height {
            let mut row = vec![];
            for j in 0..width {
                let p = img.get_pixel(j as u32, i as u32);
                let k: u32 = ((p[0] as u32) << 24)
                    + ((p[1] as u32) << 16)
                    + ((p[2] as u32) << 8)
                    + (p[3] as u32);
                row.push(k);
            }
            data.push(row);
        }
        let binary_data = vec![];
        let mut sym = Self {
            width,
            height,
            is_binary,
            fore_color: 0,
            back_color: 0,
            data,
            binary_data,
        };
        sym.make_binary(0);
        sym
    }

    pub fn make_binary(&mut self, back_rgb: u32) {
        let mut cc: HashMap<u32, (u32, u32)> = HashMap::new();
        let mut cm: Vec<u32> = vec![];
        self.binary_data = vec![vec![0u8; self.width as usize]; self.height as usize];
        for i in 0..self.height {
            for j in 0..self.width {
                let pixel_x = j as u32;
                let pixel_y = i as u32;
                let k = self.data[pixel_y as usize][pixel_x as usize];
                cc.entry(k).or_insert((pixel_x, pixel_y));
                cm.push(k);
            }
        }
        let mut cv: Vec<_> = cc.iter().collect();
        let mut include_back = false;
        let clen = cv.len();
        for c in &mut cv {
            if *c.0 == back_rgb {
                include_back = true;
            } else {
                let cd = color_distance(*c.0, back_rgb);
                // fix simliar color to back
                if cd < 1.0 {
                    // println!("cd={} c1={} c2={}", cd, *c.0, back_rgb);
                    (*c).0 = &back_rgb;
                    include_back = true;
                }
            }
        }
        let ret;
        if include_back {
            if clen == 1 {
                ret = Some((back_rgb, back_rgb));
                // println!("<B>{:?}", ret);
            } else if clen == 2 {
                let mut r = (back_rgb, back_rgb);
                if *cv[0].0 != back_rgb {
                    r.1 = *cv[0].0;
                }
                if *cv[1].0 != back_rgb {
                    r.1 = *cv[1].0;
                }
                ret = Some(r);
                // println!("<B,F>{:?}", ret);
            } else {
                // select bigest distance color to forecolor
                let mut bigd = 0.0f32;
                let mut bcv = cv[0];
                for c in &cv {
                    let cd = color_distance(*c.0, back_rgb);
                    if cd > bigd {
                        bigd = cd;
                        bcv = *c;
                    }
                }
                ret = Some((back_rgb, *bcv.0));
                // println!("ERROR!!! clen={} cv={:?}", clen, cv);
                // println!("bcv={:?}", bcv);
            }
        } else {
            if clen == 1 {
                ret = Some((*cv[0].0, *cv[0].0));
                // println!("<F>{:?}", ret);
            } else if clen == 2 {
                let l1 = luminance(*cv[0].0);
                let l2 = luminance(*cv[1].0);
                if l2 > l1 {
                    ret = Some((*cv[0].0, *cv[1].0));
                } else {
                    ret = Some((*cv[1].0, *cv[0].0));
                }
                // println!("<F1,F2>{:?}", ret);
            } else {
                let mut ccv = vec![];
                cv.sort();
                // println!("ERROR2!!! clen={} cv={:?}", clen, cv);
                let mut base = *cv[0].0;
                ccv.push(cv[0]);
                for i in 1..clen {
                    let cd = color_distance(*cv[i].0, base);
                    if cd > 1.0 {
                        ccv.push(cv[i]);
                    }
                    base = *cv[i].0;
                }
                let l1 = luminance(*ccv[0].0);
                let l2 = luminance(*ccv[1].0);
                if l2 > l1 {
                    ret = Some((*ccv[0].0, *ccv[1].0));
                } else {
                    ret = Some((*ccv[1].0, *ccv[0].0));
                }
                // println!("ccv = {:?}", ccv);
            }
        }

        for i in 0..self.height as usize {
            for j in 0..self.width as usize {
                let color = cm[i * self.width as usize + j];
                let cd0 = color_distance(color, ret.unwrap().0);
                let cd1 = color_distance(color, ret.unwrap().1);
                if cd0 <= cd1 {
                    self.binary_data[i][j] = 0;
                } else {
                    self.binary_data[i][j] = 1;
                }
            }
        }

        match ret {
            Some(r) => {
                self.back_color = find_best_color_u32(r.0) as u32;
                self.fore_color = find_best_color_u32(r.1) as u32;
            }
            _ => {
                self.back_color = 0;
                self.fore_color = 0;
            }
        }
    }
}

// ============================================================================
// Block Binarization Functions (Advanced Algorithms)
// ============================================================================

impl Default for BinarizationConfig {
    fn default() -> Self {
        Self {
            min_contrast_ratio: 0.1, // 10% brightness difference threshold
        }
    }
}

/// Process a variable-sized block using adaptive thresholding and color analysis
///
/// This function implements intelligent contrast detection and binarization:
/// 1. Calculate brightness values for all pixels using standard luminance formula
/// 2. Analyze contrast to distinguish between noise and genuine patterns
/// 3. Apply appropriate thresholding strategy based on contrast level
/// 4. Generate binary pattern and extract representative colors
///
/// # Arguments
/// * `pixels` - 2D array of RGB pixels representing the symbol block
/// * `config` - Binarization configuration parameters
///
/// # Returns
/// BinarizedBlock with optimized binary pattern and extracted colors
///
/// # Algorithm Details
/// - Low contrast (< threshold): Single-color classification based on average brightness
/// - High contrast (≥ threshold): Otsu's algorithm with fallback to average-based threshold
/// - Color extraction: Separate averaging of foreground and background pixel groups
pub fn binarize_block(pixels: &[Vec<RGB>], config: &BinarizationConfig) -> BinarizedBlock {
    let height = pixels.len();
    if height == 0 {
        panic!("Invalid block size: height must be non-zero");
    }
    let width = pixels[0].len();
    if width == 0 {
        panic!("Invalid block size: width must be non-zero");
    }

    // Calculate brightness values for all pixels
    let mut brightnesses = Vec::with_capacity(height * width);
    for y in 0..height {
        for x in 0..width {
            let px = &pixels[y][x];
            let brightness =
                (0.299 * px.r as f32 + 0.587 * px.g as f32 + 0.114 * px.b as f32) as u8;
            brightnesses.push(brightness);
        }
    }

    // Calculate statistics
    let min_brightness = *brightnesses.iter().min().unwrap();
    let max_brightness = *brightnesses.iter().max().unwrap();
    let avg_brightness = brightnesses.iter().map(|&b| b as f32).sum::<f32>() / (height * width) as f32;

    // Determine threshold using adaptive contrast-based method
    let threshold = {
        let contrast = max_brightness - min_brightness;
        let min_contrast = (config.min_contrast_ratio * 255.0) as u8;
        
        // The core principle: Only apply sophisticated thresholding to blocks with sufficient contrast
        // This approach prevents noise amplification while preserving meaningful patterns
        if contrast < min_contrast {
            // Low contrast case (< 10% brightness difference):
            // - Likely uniform/gradient regions, noise, or compression artifacts
            // - Classify entire block as single color to avoid false pattern detection
            // - Use midpoint (128) as the decision boundary for foreground vs background
            if avg_brightness > 128.0 {
                0   // Bright region → treat as background (all pixels = 0)
            } else {
                255 // Dark region → treat as foreground (all pixels = 1)
            }
        } else {
            // High contrast case (≥ 10% brightness difference):
            // - Contains genuine patterns, edges, or symbol features
            // - Apply Otsu's algorithm for optimal binary separation
            // - Fallback to average brightness if Otsu fails (edge case protection)
            find_optimal_threshold(&brightnesses).unwrap_or(avg_brightness as u8)
        }
    };

    // Perform binarization and color grouping
    let mut bitmap = vec![vec![0u8; width]; height];
    let mut foreground_pixels = Vec::new();
    let mut background_pixels = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let px = &pixels[y][x];
            let brightness = brightnesses[y * width + x];

            if brightness > threshold {
                bitmap[y][x] = 1;
                foreground_pixels.push(*px);
            } else {
                bitmap[y][x] = 0;
                background_pixels.push(*px);
            }
        }
    }

    // Calculate average foreground and background colors
    let foreground_color = if foreground_pixels.is_empty() {
        RGB { r: 255, g: 255, b: 255 }
    } else {
        let r = foreground_pixels.iter().map(|p| p.r as u32).sum::<u32>()
            / foreground_pixels.len() as u32;
        let g = foreground_pixels.iter().map(|p| p.g as u32).sum::<u32>()
            / foreground_pixels.len() as u32;
        let b = foreground_pixels.iter().map(|p| p.b as u32).sum::<u32>()
            / foreground_pixels.len() as u32;
        RGB { r: r as u8, g: g as u8, b: b as u8 }
    };

    let background_color = if background_pixels.is_empty() {
        RGB { r: 0, g: 0, b: 0 }
    } else {
        let r = background_pixels.iter().map(|p| p.r as u32).sum::<u32>()
            / background_pixels.len() as u32;
        let g = background_pixels.iter().map(|p| p.g as u32).sum::<u32>()
            / background_pixels.len() as u32;
        let b = background_pixels.iter().map(|p| p.b as u32).sum::<u32>()
            / background_pixels.len() as u32;
        RGB { r: r as u8, g: g as u8, b: b as u8 }
    };

    BinarizedBlock {
        bitmap,
        width,
        height,
        foreground_color,
        background_color,
        threshold,
    }
}

/// Find optimal threshold using Otsu's algorithm
///
/// Otsu's method automatically selects an optimal threshold by maximizing
/// the between-class variance of the foreground and background pixels.
///
/// # Arguments
/// * `brightnesses` - Array of brightness values
///
/// # Returns
/// Optimal threshold value if calculation succeeds
pub fn find_optimal_threshold(brightnesses: &[u8]) -> Option<u8> {
    if brightnesses.len() < 2 {
        return None;
    }

    let mut histogram = [0u32; 256];
    for &brightness in brightnesses {
        histogram[brightness as usize] += 1;
    }

    let total = brightnesses.len() as f32;
    let mut best_threshold = 128u8;
    let mut best_variance = 0.0f32;

    for threshold in 1..=254 {
        let mut w0 = 0.0f32; // Background weight
        let mut w1 = 0.0f32; // Foreground weight
        let mut sum0 = 0.0f32; // Background brightness sum
        let mut sum1 = 0.0f32; // Foreground brightness sum

        // Calculate background portion
        for i in 0..threshold {
            let count = histogram[i as usize] as f32;
            w0 += count;
            sum0 += i as f32 * count;
        }

        // Calculate foreground portion
        for i in threshold..=255 {
            let count = histogram[i as usize] as f32;
            w1 += count;
            sum1 += i as f32 * count;
        }

        if w0 == 0.0 || w1 == 0.0 {
            continue;
        }

        let mean0 = sum0 / w0; // Background mean
        let mean1 = sum1 / w1; // Foreground mean

        // Between-class variance
        let between_class_variance = (w0 / total) * (w1 / total) * (mean0 - mean1).powi(2);

        if between_class_variance > best_variance {
            best_variance = between_class_variance;
            best_threshold = threshold;
        }
    }

    Some(best_threshold)
}

/// Extract a square block from image at specified position
/// 
/// # Arguments
/// * `img` - Source image
/// * `x` - Block X coordinate (in block units)
/// * `y` - Block Y coordinate (in block units) 
/// * `block_size` - Size of each block
///
/// # Returns
/// 2D array of RGB pixels
#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
pub fn extract_image_block(img: &DynamicImage, x: u32, y: u32, block_size: u32) -> Vec<Vec<RGB>> {
    extract_image_block_rect(img, x, y, block_size, block_size)
}

/// Extract a rectangular block from image at specified position
/// 
/// # Arguments
/// * `img` - Source image
/// * `x` - Block X coordinate (in block units)
/// * `y` - Block Y coordinate (in block units) 
/// * `block_width` - Width of each block
/// * `block_height` - Height of each block
///
/// # Returns
/// 2D array of RGB pixels
#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
pub fn extract_image_block_rect(img: &DynamicImage, x: u32, y: u32, block_width: u32, block_height: u32) -> Vec<Vec<RGB>> {
    let mut block = vec![vec![RGB { r: 0, g: 0, b: 0 }; block_width as usize]; block_height as usize];

    for dy in 0..block_height as usize {
        for dx in 0..block_width as usize {
            let pixel_x = x * block_width + dx as u32;
            let pixel_y = y * block_height + dy as u32;

            if pixel_x < img.width() && pixel_y < img.height() {
                let pixel = img.get_pixel(pixel_x, pixel_y);
                block[dy][dx] = RGB {
                    r: pixel[0],
                    g: pixel[1], 
                    b: pixel[2],
                };
            }
        }
    }

    block
}

/// Convert image RGB to symbols RGB
#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
impl From<Rgb<u8>> for RGB {
    fn from(rgb: Rgb<u8>) -> Self {
        RGB {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }
}

/// Convert symbols RGB to image RGB  
#[cfg(all(feature = "sdl", feature = "winit", not(wasm)))]
impl From<RGB> for Rgb<u8> {
    fn from(rgb: RGB) -> Self {
        Rgb([rgb.r, rgb.g, rgb.b])
    }
}
