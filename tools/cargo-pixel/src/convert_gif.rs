// RustPixel
// copyright zipxing@hotmail.com 2022～2025

/// rust_pixel cargo build tools...
///
/// Usage:
/// cargo pixel run snake term
/// cargo pixel run snake sdl
/// cargo pixel creat games mygame
/// cargo pixel build snake web
///
/// shortcut:
/// cargo pixel r snake t
/// cargo pixel r snake s
/// cargo pixel r snake w
/// ...
///
use clap::ArgMatches;
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::str;

use crate::PixelContext;

pub fn pixel_convert_gif(_ctx: &PixelContext, args: &ArgMatches) {
    let gif = args.get_one::<String>("gif").unwrap();
    let ssf = args.get_one::<String>("ssf").unwrap();
    let width: usize = args.get_one::<String>("width").unwrap().parse().unwrap();
    let height: usize = args.get_one::<String>("height").unwrap().parse().unwrap();

    println!("🍀 extract pngs use ffmpeg...");
    
    // Create tmp directory if it doesn't exist
    let tmp_dir = Path::new("tmp");
    if !tmp_dir.exists() {
        fs::create_dir_all(tmp_dir).expect("Failed to create tmp directory");
    }
    
    // Cross-platform ffmpeg execution
    let output_pattern = tmp_dir.join("t%d.png").to_string_lossy().to_string();
    let output = Command::new("ffmpeg")
        .args(["-i", gif, "-vsync", "0", &output_pattern])
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute ffmpeg process");

    let stderr = str::from_utf8(&output.stderr).unwrap();
    let rf = Regex::new(r"(.*)frame=(.*?)(\d+)(.*)").unwrap();
    let fg = rf.captures(stderr).unwrap();
    let frame_count: usize = fg.get(3).unwrap().as_str().parse().unwrap();
    println!("    frame_count = {}", frame_count);

    println!("🍀 pixel_petii convert png to pix...");
    for x in 0..frame_count {
        print!("\r{}  ", x + 1);
        io::stdout().flush().unwrap();
        
        // Cross-platform path handling
        let input_png = tmp_dir.join(format!("t{}.png", x + 1));
        let output_pix = tmp_dir.join(format!("t{}.pix", x + 1));
        
        // Execute cargo pixel command and redirect output
        let output = Command::new("cargo")
            .args([
                "pixel", "p", 
                &input_png.to_string_lossy(),
                &width.to_string(),
                &height.to_string()
            ])
            .output()
            .expect("Failed to execute cargo pixel p");
            
        if output.status.success() {
            // Write stdout to .pix file
            fs::write(&output_pix, &output.stdout)
                .expect("Failed to write .pix file");
        } else {
            eprintln!("Warning: Failed to convert {}", input_png.display());
        }
    }

    let mut fsdq = fs::File::create(ssf).unwrap();
    writeln!(
        fsdq,
        "width={},height={},texture=255,frame_count={}",
        width, height, frame_count
    )
    .unwrap();

    let rds = Regex::new(r"(\d+),(\d+),(\d+)(.*?)").unwrap();
    let mut datas = Vec::new();
    let mut flens = Vec::new();
    for x in 0..frame_count {
        let pix_file = tmp_dir.join(format!("t{}.pix", x + 1));
        let content = fs::read_to_string(&pix_file).unwrap();
        let mut sdatas = Vec::new();
        for line in content.lines().skip(1) {
            for cap in rds.captures_iter(line) {
                sdatas.push(cap[1].parse::<u8>().unwrap());
                sdatas.push(cap[2].parse::<u8>().unwrap());
                sdatas.push(cap[3].parse::<u8>().unwrap());
            }
        }
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&sdatas).unwrap();
        let compressed = encoder.finish().unwrap();
        flens.push(compressed.len());
        datas.extend_from_slice(&compressed);
    }

    for len in flens {
        write!(fsdq, "{},", len).unwrap();
    }
    writeln!(fsdq).unwrap();
    fsdq.write_all(&datas).unwrap();

    println!("\n🍀 {} write ok!", ssf);
    
    // Clean up temporary files
    if let Ok(entries) = fs::read_dir(tmp_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            
            // Remove files matching pattern t*.p* (t1.png, t1.pix, etc.)
            if file_name_str.starts_with('t') && 
               (file_name_str.contains(".png") || file_name_str.contains(".pix")) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

