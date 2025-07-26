use deltae::*;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use lab::Lab;
use rust_pixel::render::style::ANSI_COLOR_RGB;
use std::collections::HashMap;
use std::env;
use std::path::Path;

struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

fn print_symbol_usage() {
    eprintln!("RustPixel Symbol Extractor");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    symbol <IMAGE_FILE> <SYMSIZE> [START_X START_Y WIDTH HEIGHT]");
    eprintln!("    cargo pixel symbol <IMAGE_FILE> <SYMSIZE> [START_X START_Y WIDTH HEIGHT]");
    eprintln!("    cargo pixel sy <IMAGE_FILE> <SYMSIZE> [START_X START_Y WIDTH HEIGHT]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <IMAGE_FILE>  Input image file path");
    eprintln!("    <SYMSIZE>     Symbol size in pixels (e.g., 8 for 8x8 symbols)");
    eprintln!("    [START_X]     Start X coordinate for processing area");
    eprintln!("    [START_Y]     Start Y coordinate for processing area");
    eprintln!("    [WIDTH]       Width of processing area");
    eprintln!("    [HEIGHT]      Height of processing area");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Extracts symbols/characters from images for use in creating symbol fonts");
    eprintln!("    or character sets. Analyzes image blocks and generates unique symbol");
    eprintln!("    patterns with color mappings for optimal representation.");
    eprintln!();
    eprintln!("PROCESSING:");
    eprintln!("    - Divides image into blocks of specified symbol size");
    eprintln!("    - Analyzes each block for unique patterns");
    eprintln!("    - Maps colors to ANSI color palette");
    eprintln!("    - Generates symbol map and color associations");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    - Symbol patterns displayed in terminal");
    eprintln!("    - Color mappings for foreground/background");
    eprintln!("    - Statistics about unique symbols found");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    symbol font.png 8                          # Extract 8x8 symbols from entire image");
    eprintln!("    symbol charset.png 16                      # Extract 16x16 symbols");
    eprintln!("    symbol image.png 8 0 0 128 64              # Extract from 128x64 area at (0,0)");
    eprintln!("    symbol tiles.png 16 32 32 256 256          # Extract from specific region");
    eprintln!();
    eprintln!("FEATURES:");
    eprintln!("    - Color similarity analysis using Delta E");
    eprintln!("    - ANSI color palette mapping");
    eprintln!("    - Binary pattern recognition");
    eprintln!("    - Selective area processing");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    When used via cargo-pixel, equivalent to: cargo pixel r symbol t -r <ARGS...>");
}

fn main() {
    let input_image_path;
    let symsize: u32;
    let mut width: u32;
    let mut height: u32;
    let start_x: u32;
    let start_y: u32;
    // key: binary image, value: block index list
    let mut symbol_map = HashMap::new();
    // key: block index, value: bg color, fg color)
    let mut color_map = HashMap::new();

    // parse command line...
    let args: Vec<String> = env::args().collect();
    
    // Check for help argument
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_symbol_usage();
        return;
    }
    
    let arglen = args.len();
    if arglen != 3 && arglen != 7 {
        print_symbol_usage();
        return;
    }
    input_image_path = Path::new(&args[1]);
    symsize = args[2].parse().unwrap();

    // open image...
    let mut img = image::open(&input_image_path).expect("Failed to open the input image");
    width = img.width() as u32 / symsize;
    height = img.height() as u32 / symsize;

    // if set sx,sy,w,h then crop image...
    if arglen == 7 {
        start_x = args[3].parse().unwrap();
        start_y = args[4].parse().unwrap();
        width = args[5].parse::<u32>().unwrap() / symsize;
        height = args[6].parse::<u32>().unwrap() / symsize;
        img = img.crop(start_x, start_y, width * symsize, height * symsize);
    }
    println!("width={} height={}", width, height);

    // count pixels for dig background color
    let back_color = find_background_color(&img, width * symsize, height * symsize);

    // scan blocks
    for i in 0..height {
        for j in 0..width {
            let c = process_block(&img, symsize as usize, j, i, back_color);
            color_map.entry(i * width + j).or_insert((c.0, c.1));
            symbol_map
                .entry(c.2)
                .or_insert(Vec::new())
                .push(i * width + j);
        }
    }
    let symlen = symbol_map.len();
    let symw = 16;
    let symh = symlen / 16 + if symlen % 16 == 0 { 0 } else { 1 };

    // redraw image...
    let mut simg = ImageBuffer::new(symsize * symw as u32, symsize * symh as u32);
    let mut nimg = ImageBuffer::new(symsize * width, symsize * height);
    let mut scount = 0;
    for (k, v) in symbol_map.iter() {
        for y in 0..symsize {
            for x in 0..symsize {
                let pixel_value = if k[y as usize][x as usize] == 1 {
                    [255u8, 255, 255, 255]
                } else {
                    [0u8, 0, 0, 255]
                };
                simg.put_pixel(
                    (scount % 16) * symsize + x,
                    (scount / 16) * symsize + y,
                    Rgba(pixel_value),
                );
            }
        }
        scount += 1;

        for b in v {
            let i = b % width;
            let j = b / width;
            let color = color_map.get(b).unwrap();
            for y in 0..symsize {
                for x in 0..symsize {
                    let pixel_value = if k[y as usize][x as usize] == 1 {
                        let ac = ANSI_COLOR_RGB[color.1 as usize];
                        [ac[0], ac[1], ac[2], 255]
                    } else {
                        let ac = ANSI_COLOR_RGB[color.0 as usize];
                        [ac[0], ac[1], ac[2], 255]
                    };
                    nimg.put_pixel(i * symsize + x, j * symsize + y, Rgba(pixel_value));
                }
            }
        }
    }
    println!("dump symbols to sout.png({}symbols {}rows {}cols)", symlen, symh, symw);
    simg.save("sout.png").expect("save image error");
    println!("redraw to bout.png");
    nimg.save("bout.png").expect("save image error");
}

// find background colors...
fn find_background_color(img: &DynamicImage, w: u32, h: u32) -> u32 {
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

fn luminance(e1: u32) -> f32 {
    let e1r = (e1 >> 24 & 0xff) as u8;
    let e1g = (e1 >> 16 & 0xff) as u8;
    let e1b = (e1 >> 8 & 0xff) as u8;
    0.299 * e1r as f32 + 0.587 * e1g as f32 + 0.114 * e1b as f32
}

// get color distance
fn color_distance(e1: u32, e2: u32) -> f32 {
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

// get symbol block color
fn process_block(
    image: &DynamicImage,
    n: usize,
    x: u32,
    y: u32,
    back_rgb: u32,
) -> (usize, usize, Vec<Vec<u8>>) {
    let mut cc: HashMap<u32, (u32, u32)> = HashMap::new();
    let mut cm: Vec<u32> = vec![];
    let mut block = vec![vec![0u8; n]; n];
    for i in 0..n {
        for j in 0..n {
            let pixel_x = x * n as u32 + j as u32;
            let pixel_y = y * n as u32 + i as u32;
            if pixel_x < image.width() && pixel_y < image.height() {
                let p = image.get_pixel(pixel_x, pixel_y);
                let k: u32 = ((p[0] as u32) << 24)
                    + ((p[1] as u32) << 16)
                    + ((p[2] as u32) << 8)
                    + (p[3] as u32);
                cc.entry(k).or_insert((pixel_x, pixel_y));
                cm.push(k);
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

    for i in 0..n {
        for j in 0..n {
            let color = cm[i * n + j];
            let cd0 = color_distance(color, ret.unwrap().0);
            let cd1 = color_distance(color, ret.unwrap().1);
            if cd0 <= cd1 {
                block[i][j] = 0;
            } else {
                block[i][j] = 1;
            }
        }
    }

    match ret {
        Some(r) => (find_best_color_u32(r.0), find_best_color_u32(r.1), block),
        _ => (0, 0, block),
    }
}

fn find_best_color_u32(c: u32) -> usize {
    find_best_color(RGB {
        r: (c >> 24) as u8,
        g: (c >> 16) as u8,
        b: (c >> 8) as u8,
    })
}

// get color distance
fn color_distance_rgb(e1: &RGB, e2: &RGB) -> f32 {
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

fn find_best_color(color: RGB) -> usize {
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
