//! PETSCII converter CLI. Historical positional arguments remain compatible.

mod ai_cli;
mod benchmark_cli;

use petii::{
    analyze_pix_corpus, convert_image, convert_image_dithered, optimize_grid, render_grid,
    ConversionConfig, OptimizationWeights,
};
use std::{env, fs, path::Path};

fn print_usage() {
    println!("PETSCII Converter Tool v2.1");
    println!("Converts images to fixed-character-set PETSCII art.");
    println!();
    println!("USAGE:");
    println!("  petii <IMAGE_FILE> [WIDTH] [HEIGHT] [MODE] [CROP_X CROP_Y CROP_W CROP_H]");
    println!("        [--optimize] [--top-k N] [--preview FILE.png]");
    println!();
    println!("MODE:");
    println!("  0=nearest PETSCII glyph for general images (single foreground color)");
    println!("  1=extract artwork already composed as exact PETSCII (per-cell fg/bg)");
    println!("  2=mode 0 without letters or digits");
    println!("Defaults: WIDTH=40 HEIGHT=25 MODE=0 TOP_K=4 when optimizing");
    println!();
    println!("EXPERIMENTAL:");
    println!("  petii ai \"PROMPT\" [--input IMAGE] [--offline] [--output-dir DIRECTORY]");
    println!(
        "  petii benchmark MANIFEST.json [--reference-dir DIRECTORY] [--output-dir DIRECTORY]"
    );
    println!("  petii corpus DIRECTORY [--output report.json]");
}

fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn positional(args: &[String]) -> Vec<&str> {
    let value_flags = ["--top-k", "--preview"];
    let mut output = Vec::new();
    let mut skip_next = false;
    for arg in args.iter().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if value_flags.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--") {
            continue;
        }
        output.push(arg.as_str());
    }
    output
}

fn parse_u32(value: Option<&&str>, default: u32, name: &str) -> u32 {
    value.map_or(default, |raw| {
        raw.parse().unwrap_or_else(|_| {
            eprintln!("Error: invalid {} '{}'", name, raw);
            std::process::exit(1);
        })
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|argument| argument == "ai") {
        if let Err(error) = ai_cli::run(&args[2..]) {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|argument| argument == "benchmark") {
        if let Err(error) = benchmark_cli::run(&args[2..]) {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|argument| argument == "corpus") {
        let Some(directory) = args.get(2) else {
            eprintln!("Error: corpus directory is required");
            std::process::exit(1);
        };
        let report = analyze_pix_corpus(Path::new(directory)).unwrap_or_else(|error| {
            eprintln!("Error: {error}");
            std::process::exit(1);
        });
        let json = serde_json::to_string_pretty(&report).unwrap();
        if let Some(path) = value_after(&args, "--output") {
            fs::write(&path, format!("{json}\n")).unwrap_or_else(|error| {
                eprintln!("Error: failed to save '{}': {error}", path);
                std::process::exit(1);
            });
        } else {
            println!("{json}");
        }
        return;
    }
    if args.len() < 2
        || args
            .iter()
            .any(|arg| arg == "--help" || arg == "-h" || arg == "help")
    {
        print_usage();
        if args.len() < 2 {
            std::process::exit(1);
        }
        return;
    }

    let pos = positional(&args);
    if pos.is_empty() {
        eprintln!("Error: an input image file is required");
        std::process::exit(1);
    }
    let input = pos[0];
    if !Path::new(input).is_file() {
        eprintln!("Error: image file '{}' does not exist", input);
        std::process::exit(1);
    }
    let width = parse_u32(pos.get(1), 40, "width");
    let height = parse_u32(pos.get(2), 25, "height");
    let mode = parse_u32(pos.get(3), 0, "mode") as u8;
    let optimize = args.iter().any(|arg| arg == "--optimize");
    let top_k = value_after(&args, "--top-k")
        .map(|raw| {
            raw.parse::<usize>().unwrap_or_else(|_| {
                eprintln!("Error: invalid top-k '{}'", raw);
                std::process::exit(1);
            })
        })
        .unwrap_or(if optimize { 4 } else { 1 });

    let mut image = image::open(input).unwrap_or_else(|error| {
        eprintln!("Error: failed to open '{}': {}", input, error);
        std::process::exit(1);
    });
    if pos.len() >= 8 {
        let crop_x = parse_u32(pos.get(4), 0, "crop x");
        let crop_y = parse_u32(pos.get(5), 0, "crop y");
        let crop_w = parse_u32(pos.get(6), image.width(), "crop width");
        let crop_h = parse_u32(pos.get(7), image.height(), "crop height");
        image = image.crop_imm(crop_x, crop_y, crop_w, crop_h);
    }

    let config = ConversionConfig {
        width,
        height,
        mode,
        top_k,
        contrast: 0.0,
    };
    let dither = args.iter().any(|arg| arg == "--dither");
    let convert = if dither {
        convert_image_dithered
    } else {
        convert_image
    };
    let result = convert(&image, &config).unwrap_or_else(|error| {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    });
    let (grid, score) = if optimize {
        optimize_grid(
            &result.grid,
            &result.alternatives,
            &result.reference,
            OptimizationWeights::default(),
        )
        .unwrap_or_else(|error| {
            eprintln!("Error: optimization failed: {}", error);
            std::process::exit(1);
        })
    } else {
        (result.grid, Default::default())
    };

    if let Some(path) = value_after(&args, "--preview") {
        let preview = render_grid(&grid, 2).unwrap_or_else(|error| {
            eprintln!("Error: preview failed: {}", error);
            std::process::exit(1);
        });
        preview.save(&path).unwrap_or_else(|error| {
            eprintln!("Error: failed to save '{}': {}", path, error);
            std::process::exit(1);
        });
    }

    if optimize {
        eprintln!("optimized_score={:.6}", score.total);
    }
    if let Err(error) = fs::create_dir_all("tmp") {
        eprintln!("Warning: failed to create tmp directory: {}", error);
    }
    let _ = result.reference.save("tmp/out1.png");
    let _ = result.reference.to_luma8().save("tmp/out2.png");
    print!("{}", grid.to_legacy_string(mode));
}
