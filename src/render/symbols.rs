// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Symbol processing utilities for RustPixel.
//! 
//! A symbol comprises a point vector with width * height elements.
//! This module provides functions for symbol manipulation, color processing, and block binarization.

use crate::render::style::ANSI_COLOR_RGB;
use deltae::*;
#[cfg(not(wasm))]
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma, Rgb};
use lab::Lab;
#[cfg(not(wasm))]
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

/// Find background colors from image
/// 
/// Given an original image and its luma8 grayscale version, finds both
/// the background gray value and background RGB color.
/// 
/// # Arguments
/// * `img` - Original RGB image
/// * `image` - Luma8 grayscale version of the image
/// * `w` - Image width
/// * `h` - Image height
/// 
/// # Returns
/// Tuple of (background_gray_value, background_rgb_color)
#[cfg(not(wasm))]
pub fn find_background_color(
    img: &DynamicImage,
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    w: u32,
    h: u32,
) -> (u8, u32) {
    // (first_x, first_y, count)
    let mut cc: HashMap<u32, (u32, u32, u32)> = HashMap::new();
    for i in 0..h {
        for j in 0..w {
            let p = img.get_pixel(j, i);
            let k: u32 = ((p[0] as u32) << 24)
                + ((p[1] as u32) << 16)
                + ((p[2] as u32) << 8)
                + (p[3] as u32);
            cc.entry(k).or_insert((j, i, 0)).2 += 1;
        }
    }
    let mut cv: Vec<_> = cc.iter().collect();
    cv.sort_by(|b, a| a.1 .2.cmp(&b.1 .2));
    let bx = cv[0].1 .0;
    let by = cv[0].1 .1;
    let bc = cv[0].0;
    // for c in cv {
    //     println!("cc..{:x} {:?}", c.0, c.1);
    // }
    // for i in 0..h {
    //     for j in 0..w {
    //         print!("{:?} ", image.get_pixel(j, i).0[0]);
    //     }
    //     println!("");
    // }
    let gray = image.get_pixel(bx, by).0[0];
    // println!("gray..{}", gray);
    (gray, *bc)
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
    *DeltaE::new(lab1, lab2, DE2000).value()
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
    *DeltaE::new(lab1, lab2, DE2000).value()
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

#[cfg(not(wasm))]
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
                    c.0 = &back_rgb;
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
        } else if clen == 1 {
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
            for item in &cv[1..] {
                let cd = color_distance(*item.0, base);
                if cd > 1.0 {
                    ccv.push(*item);
                }
                base = *item.0;
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
    for row in pixels.iter().take(height) {
        for px in row.iter().take(width) {
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
#[cfg(not(wasm))]
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
#[cfg(not(wasm))]
pub fn extract_image_block_rect(img: &DynamicImage, x: u32, y: u32, block_width: u32, block_height: u32) -> Vec<Vec<RGB>> {
    let mut block = vec![vec![RGB { r: 0, g: 0, b: 0 }; block_width as usize]; block_height as usize];

    for (dy, row) in block.iter_mut().enumerate().take(block_height as usize) {
        for (dx, cell) in row.iter_mut().enumerate().take(block_width as usize) {
            let pixel_x = x * block_width + dx as u32;
            let pixel_y = y * block_height + dy as u32;

            if pixel_x < img.width() && pixel_y < img.height() {
                let pixel = img.get_pixel(pixel_x, pixel_y);
                *cell = RGB {
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
#[cfg(not(wasm))]
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
#[cfg(not(wasm))]
impl From<RGB> for Rgb<u8> {
    fn from(rgb: RGB) -> Self {
        Rgb([rgb.r, rgb.g, rgb.b])
    }
}

// ============================================================================
// Character Block Processing for PETSCII/ASCII Conversion
// ============================================================================

/// Type alias for an NxM grayscale image block represented as a 2D vector of u8 values
/// Each value represents a grayscale pixel intensity from 0 (black) to 255 (white)
/// Block dimensions are configurable and not limited to 8x8
pub type BlockGrayImage = Vec<Vec<u8>>;

/// Generates NxM pixel representations of 256 PETSCII/C64 characters for pattern matching
/// 
/// This function creates binary images (using only 0 and 255 values) for each character
/// in the C64 character set by interpreting the bitmap data from the character ROM.
/// The generated images are scaled to match the specified block dimensions.
/// 
/// # Character Set Organization:
/// - Characters 0-127: Normal character set with foreground pixels as 255, background as 0
/// - Characters 128-255: Inverted versions with foreground as 0, background as 255
/// - Total: 256 character variants for comprehensive matching
/// 
/// # Arguments:
/// * `low_up` - Character set variant selector:
///   - `true`: Uses lowercase/uppercase character set (C64LOW)
///   - `false`: Uses uppercase/graphics character set (C64UP) - **typically used**
/// * `block_width` - Width of each character block in pixels
/// * `block_height` - Height of each character block in pixels
/// * `charset_data` - Reference to C64 character set data (256 characters, each 8 bytes)
/// 
/// # Returns:
/// * `Vec<BlockGrayImage>` - Vector of 256 NxM binary character images
/// 
/// # Implementation Details:
/// Each character is defined by 8 bytes representing 8 rows of 8 bits each.
/// The original 8x8 pattern is scaled to the target block dimensions.
/// Bits are processed right-to-left to match C64 character orientation.
#[cfg(not(wasm))]
#[allow(clippy::needless_range_loop)]
pub fn gen_charset_images(
    low_up: bool, 
    block_width: usize, 
    block_height: usize,
    c64low_data: &[[u8; 8]; 128],
    c64up_data: &[[u8; 8]; 128],
) -> Vec<BlockGrayImage> {
    let data = if low_up { c64low_data } else { c64up_data };
    let mut vcs = vec![vec![vec![0u8; block_width]; block_height]; 256];

    // Scale factors for converting from 8x8 to target dimensions
    let scale_x = block_width as f32 / 8.0;
    let scale_y = block_height as f32 / 8.0;

    for i in 0..128 {
        for y in 0..block_height {
            for x in 0..block_width {
                // Map target coordinates back to original 8x8 pattern
                let orig_x = (x as f32 / scale_x) as usize;
                let orig_y = (y as f32 / scale_y) as usize;
                let orig_x = orig_x.min(7);
                let orig_y = orig_y.min(7);
                
                // Extract bit from original pattern
                let bit = 7 - orig_x; // Right-to-left bit order
                if data[i][orig_y] >> bit & 1 == 1 {
                    vcs[i][y][x] = 255;
                    vcs[128 + i][y][x] = 0;
                } else {
                    vcs[i][y][x] = 0;
                    vcs[128 + i][y][x] = 255;
                }
            }
        }
    }
    vcs
}

/// Extracts an NxM grayscale pixel block from the source image at the specified character position
/// 
/// This function creates a local copy of image data for a single character cell,
/// converting the image coordinates from character space to pixel space.
/// The extracted block is used for character pattern matching and analysis.
/// 
/// # Coordinate System:
/// - Input coordinates (x,y) represent character positions in the output grid
/// - Pixel coordinates are calculated as (x*block_width, y*block_height)
/// - Handles boundary cases gracefully when blocks extend beyond image edges
/// 
/// # Arguments:
/// * `image` - Source grayscale image (single channel, u8 values)
/// * `x` - Character X position in the output grid
/// * `y` - Character Y position in the output grid
/// * `block_width` - Width of each character block in pixels
/// * `block_height` - Height of each character block in pixels
/// 
/// # Returns:
/// * `BlockGrayImage` - NxM matrix of grayscale values (0-255)
///   - `block[row][column]` format for easy iteration
///   - Out-of-bounds pixels default to 0 (black)
/// 
/// # Memory Layout:
/// The returned block uses [row][column] indexing where:
/// - First index (0..block_height-1) represents vertical position (Y)
/// - Second index (0..block_width-1) represents horizontal position (X)
#[cfg(not(wasm))]
pub fn get_grayscale_block_at(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>, 
    x: u32, 
    y: u32, 
    block_width: u32, 
    block_height: u32
) -> BlockGrayImage {
    let mut block = vec![vec![0u8; block_width as usize]; block_height as usize];

    for (i, row) in block.iter_mut().enumerate().take(block_height as usize) {
        for (j, cell) in row.iter_mut().enumerate().take(block_width as usize) {
            let pixel_x = x * block_width + j as u32;
            let pixel_y = y * block_height + i as u32;

            if pixel_x < image.width() && pixel_y < image.height() {
                *cell = image.get_pixel(pixel_x, pixel_y).0[0];
            }
        }
    }

    block
}

/// Binarize a grayscale block for PETSCII processing
///
/// This function extracts the binarization logic from calc_eigenvector
/// to provide a unified binarization step before character matching.
/// 
/// # Arguments:
/// * `img` - Input grayscale block to binarize
/// * `back` - Background gray value for comparison
/// * `block_width` - Width of the block in pixels
/// * `block_height` - Height of the block in pixels
/// 
/// # Returns:
/// * `BlockGrayImage` - Binarized block with only 0 and 255 values
pub fn binarize_grayscale_block(
    img: &BlockGrayImage, 
    back: u8, 
    block_width: usize, 
    block_height: usize
) -> BlockGrayImage {
    let mut binary_block = vec![vec![0u8; block_width]; block_height];
    let mut min = u8::MAX;
    let mut max = 0u8;
    let mut include_back = false;

    // Find min & max gray value and check for background color
    for row in img.iter().take(block_height) {
        for &p in row.iter().take(block_width) {
            if !include_back && p == back {
                include_back = true;
            }
            if p > max {
                max = p;
            }
            if p < min {
                min = p;
            }
        }
    }

    // Apply binarization logic
    for y in 0..block_height {
        for x in 0..block_width {
            let iyx = img[y][x];
            let binary_value = if include_back {
                // If block includes background color
                if iyx == back {
                    0
                } else {
                    255
                }
            } else {
                // No background color present, use threshold
                let threshold = (min + max) / 2;
                if iyx > threshold {
                    255
                } else {
                    0
                }
            };
            binary_block[y][x] = binary_value;
        }
    }

    binary_block
}

/// Calculates a 10-dimensional feature vector that captures structural patterns in an NxM image block
/// 
/// This function extracts geometric and structural features from grayscale image blocks
/// to enable robust character matching. Instead of comparing raw pixel values, it computes
/// higher-level features that are more invariant to brightness variations and minor distortions.
/// 
/// # Feature Components (10-dimensional vector):
/// - **v[0]**: Top-left quadrant sum
/// - **v[1]**: Top-right quadrant sum
/// - **v[2]**: Bottom-left quadrant sum
/// - **v[3]**: Bottom-right quadrant sum
/// - **v[4]**: Central region sum - captures core pattern
/// - **v[5]**: Main diagonals sum - detects diagonal patterns
/// - **v[6]**: Left edge sum - vertical line detection
/// - **v[7]**: Right edge sum - vertical line detection  
/// - **v[8]**: Top edge sum - horizontal line detection
/// - **v[9]**: Bottom edge sum - horizontal line detection
/// 
/// # Mathematical Properties:
/// - **Translation invariant**: Features remain stable under small shifts
/// - **Brightness adaptive**: Relative patterns matter more than absolute values
/// - **Structurally descriptive**: Captures key geometric properties of characters
/// 
/// # Arguments:
/// * `img` - NxM grayscale image block to analyze
/// * `block_width` - Width of the block in pixels
/// * `block_height` - Height of the block in pixels
/// 
/// # Returns:
/// * `Vec<i32>` - 10-element feature vector representing structural characteristics
/// 
/// # Usage:
/// Used by `calculate_mse()` to compare feature vectors rather than raw pixel data,
/// providing more robust character matching for ASCII/PETSCII conversion.
pub fn calc_eigenvector(img: &BlockGrayImage, block_width: usize, block_height: usize) -> Vec<i32> {
    let mut v = vec![0i32; 10];
    
    // Calculate relative thresholds based on block dimensions
    let half_width = block_width / 2;
    let half_height = block_height / 2;
    let quarter_width = block_width / 4;
    let quarter_height = block_height / 4;
    let center_start_x = quarter_width;
    let center_end_x = block_width - quarter_width;
    let center_start_y = quarter_height;
    let center_end_y = block_height - quarter_height;
    let max_x = block_width - 1;
    let max_y = block_height - 1;

    for (y, row) in img.iter().enumerate().take(block_height) {
        for (x, &pixel) in row.iter().enumerate().take(block_width) {
            let p = pixel as i32;

            // Quadrants
            if x < half_width && y < half_height {
                v[0] += p; // Top-left
            }
            if x >= half_width && y < half_height {
                v[1] += p; // Top-right
            }
            if x < half_width && y >= half_height {
                v[2] += p; // Bottom-left
            }
            if x >= half_width && y >= half_height {
                v[3] += p; // Bottom-right
            }
            
            // Central region
            if x >= center_start_x && x < center_end_x && y >= center_start_y && y < center_end_y {
                v[4] += p;
            }
            
            // Diagonal patterns (adapted for NxM)
            let diag_main = (x as f32 / block_width as f32 * block_height as f32) as usize;
            let diag_anti = (((block_width - 1 - x) as f32 / block_width as f32) * block_height as f32) as usize;
            if y == diag_main || y == diag_anti {
                v[5] += p;
            }
            
            // Edges
            if x == 0 {
                v[6] += p; // Left edge
            }
            if x == max_x {
                v[7] += p; // Right edge
            }
            if y == 0 {
                v[8] += p; // Top edge
            }
            if y == max_y {
                v[9] += p; // Bottom edge
            }
        }
    }
    v
}

/// Calculates the mean squared error between two NxM image blocks using feature vectors
/// 
/// This function computes a distance metric between image blocks by comparing their
/// structural features rather than raw pixel values. This approach provides more robust
/// and perceptually meaningful matching for character recognition applications.
/// 
/// # Algorithm Steps:
/// 1. **Feature extraction**: Computes 10-dimensional eigenvectors for both images
/// 2. **Component differences**: Calculates squared differences for each feature component
/// 3. **Distance calculation**: Sums squared differences and takes square root (Euclidean norm)
/// 
/// # Why Feature-Based MSE:
/// - **Structural focus**: Emphasizes shape and pattern over exact pixel intensity
/// - **Noise robustness**: Less sensitive to compression artifacts and minor variations
/// - **Efficiency**: 10 comparisons instead of NxM pixel comparisons
/// - **Perceptual relevance**: Features correlate better with human character recognition
/// 
/// # Mathematical Formula:
/// ```
/// MSE = sqrt(Σ(feature1[i] - feature2[i])²) for i = 0 to 9
/// ```
/// 
/// # Arguments:
/// * `img1` - First NxM image block (typically from source image)
/// * `img2` - Second NxM image block (typically character template)
/// * `block_width` - Width of the blocks in pixels
/// * `block_height` - Height of the blocks in pixels
/// 
/// # Returns:
/// * `f64` - Distance metric where:
///   - 0.0 = identical structural features
///   - Lower values = better matches
///   - Higher values = more dissimilar patterns
/// 
/// # Usage:
/// Core function used by `find_best_match()` to rank character template similarity
/// for optimal ASCII/PETSCII character selection.
pub fn calculate_mse(
    img1: &BlockGrayImage, 
    img2: &BlockGrayImage, 
    block_width: usize, 
    block_height: usize
) -> f64 {
    let mut mse = 0.0f64;
    let v1 = calc_eigenvector(img1, block_width, block_height);
    let v2 = calc_eigenvector(img2, block_width, block_height);
    
    for i in 0..10usize {
        mse += ((v1[i] - v2[i]) * (v1[i] - v2[i])) as f64;
    }
    
    mse.sqrt()
}

/// Finds the character that best matches the input NxM grayscale block using feature comparison
/// 
/// This function compares the input block against all available character templates
/// using a sophisticated feature-based matching algorithm rather than simple pixel comparison.
/// The matching process uses eigenvector analysis to capture structural patterns
/// and mean squared error for quantitative comparison.
/// 
/// # Matching Algorithm:
/// 1. **Feature extraction**: Computes eigenvector features for input block
/// 2. **Template comparison**: Calculates features for each character template
/// 3. **Distance calculation**: Uses MSE between feature vectors (not raw pixels)
/// 4. **Best match selection**: Returns character index with minimum MSE distance
/// 
/// # Why Feature-Based Matching:
/// - More robust than pixel-by-pixel comparison
/// - Captures structural patterns and shapes
/// - Less sensitive to minor brightness variations
/// - Better handles aliasing and scaling artifacts
/// 
/// # Arguments:
/// * `input_image` - NxM grayscale block to match (from source image)
/// * `char_images` - Array of 256 character template images (from `gen_charset_images`)
/// * `block_width` - Width of the blocks in pixels
/// * `block_height` - Height of the blocks in pixels
/// 
/// # Returns:
/// * `usize` - Index (0-255) of the best matching character in the character set
/// 
/// # Performance:
/// O(n) where n=256 characters, with feature comparison being much faster than full pixel MSE
pub fn find_best_match(
    input_image: &BlockGrayImage, 
    char_images: &[BlockGrayImage], 
    block_width: usize, 
    block_height: usize
) -> usize {
    let mut min_mse = f64::MAX;
    let mut best_match = 0;

    for (i, char_image) in char_images.iter().enumerate() {
        let mse = calculate_mse(input_image, char_image, block_width, block_height);

        if mse < min_mse {
            min_mse = mse;
            best_match = i;
        }
    }

    best_match
}

/// Analyzes an NxM pixel block to determine optimal foreground and background colors for PETSCII mode
/// 
/// PETSCII characters support only two colors per character cell (foreground and background).
/// This function analyzes the color distribution within a block to determine the best
/// two-color representation, considering the global background color detected in the image.
/// 
/// # Color Analysis Strategy:
/// 1. **Color enumeration**: Collects all unique RGB colors in the NxM block
/// 2. **Background consideration**: Checks if the global background color appears in the block
/// 3. **Optimal pairing**: Determines the best foreground/background color combination:
///    - If background color present: uses it as background, most contrasting color as foreground
///    - If no background: uses darker color as background, lighter as foreground
///    - Handles edge cases with single colors or too many colors gracefully
/// 
/// # Arguments:
/// * `image` - Source color image for RGB analysis
/// * `img` - Corresponding grayscale image for brightness comparison
/// * `x` - Block X coordinate in character grid
/// * `y` - Block Y coordinate in character grid  
/// * `back_rgb` - Global background color (packed as u32: RGBA)
/// * `block_width` - Width of each character block in pixels
/// * `block_height` - Height of each character block in pixels
/// 
/// # Returns:
/// * `(usize, usize)` - Tuple of (background_color_index, foreground_color_index)
///   where indices refer to positions in the ANSI color palette
/// 
/// # Error Handling:
/// Prints "ERROR!!!" or "ERROR2!!!" for invalid color configurations and returns (0,0)
#[cfg(not(wasm))]
pub fn get_petii_block_color(
    image: &DynamicImage,
    _img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    x: u32,
    y: u32,
    back_rgb: u32,
    block_width: u32,
    block_height: u32,
) -> (usize, usize) {
    let mut cc: HashMap<u32, (u32, u32)> = HashMap::new();
    for i in 0..block_height as usize {
        for j in 0..block_width as usize {
            let pixel_x = x * block_width + j as u32;
            let pixel_y = y * block_height + i as u32;
            if pixel_x < image.width() && pixel_y < image.height() {
                let p = image.get_pixel(pixel_x, pixel_y);
                let k: u32 = ((p[0] as u32) << 24)
                    + ((p[1] as u32) << 16)
                    + ((p[2] as u32) << 8)
                    + (p[3] as u32);
                cc.entry(k).or_insert((pixel_x, pixel_y));
            }
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
                c.0 = &back_rgb;
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
    } else if clen == 1 {
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
        for item in &cv[1..] {
            let cd = color_distance(*item.0, base);
            if cd > 1.0 {
                ccv.push(*item);
            }
            base = *item.0;
        }
        if ccv.len() >= 2 {
            let l1 = luminance(*ccv[0].0);
            let l2 = luminance(*ccv[1].0);
            if l2 > l1 {
                ret = Some((*ccv[0].0, *ccv[1].0));
            } else {
                ret = Some((*ccv[1].0, *ccv[0].0));
            }
        } else {
            println!("ERROR2!!!");
            ret = Some((0, 0));
        }
        // println!("ccv = {:?}", ccv);
    }

    match ret {
        Some(r) => (find_best_color_u32(r.0), find_best_color_u32(r.1)),
        _ => (0, 0),
    }
}

/// Calculates the average RGB color of an NxM pixel block for ASCII mode color matching
/// 
/// This function computes the mean color across all non-black pixels in the specified
/// character block. Pure black pixels (0,0,0) are excluded from the average to prevent
/// background pixels from skewing the color calculation toward darkness.
/// 
/// # Color Averaging Process:
/// 1. **Pixel iteration**: Scans all pixels in the NxM block
/// 2. **Black pixel filtering**: Skips pixels with RGB values of (0,0,0)
/// 3. **Component accumulation**: Sums red, green, and blue values separately
/// 4. **Average calculation**: Divides totals by pixel count for mean color
/// 
/// # Arguments:
/// * `image` - Source color image to analyze
/// * `x` - Block X coordinate in character grid
/// * `y` - Block Y coordinate in character grid
/// * `block_width` - Width of each character block in pixels
/// * `block_height` - Height of each character block in pixels
/// 
/// # Returns:
/// * `RGB` - Average color of the block, or black RGB{0,0,0} if no valid pixels found
/// 
/// # Use Case:
/// In ASCII mode, each character position gets a single color determined by this average.
/// The resulting color is then matched to the closest ANSI terminal color for display.
#[cfg(not(wasm))]
pub fn get_block_color(
    image: &DynamicImage, 
    x: u32, 
    y: u32, 
    block_width: u32, 
    block_height: u32
) -> RGB {
    let mut r = 0u32;
    let mut g = 0u32;
    let mut b = 0u32;

    let mut count = 0u32;

    for i in 0..block_height as usize {
        for j in 0..block_width as usize {
            let pixel_x = x * block_width + j as u32;
            let pixel_y = y * block_height + i as u32;

            if pixel_x < image.width() && pixel_y < image.height() {
                let p = image.get_pixel(pixel_x, pixel_y);
                if p[0] != 0 || p[1] != 0 || p[2] != 0 {
                    r += p[0] as u32;
                    g += p[1] as u32;
                    b += p[2] as u32;
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        return RGB { r: 0, g: 0, b: 0 };
    }

    RGB {
        r: (r / count) as u8,
        g: (g / count) as u8,
        b: (b / count) as u8,
    }
}
