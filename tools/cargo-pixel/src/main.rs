// RustPixel
// copyright zipxing@hotmail.com 2022～2025

/// rust_pixel cargo build tools...
///
/// Usage:
/// cargo pixel run snake term
/// cargo pixel run snake sdl
/// cargo pixel creat games mygame
/// cargo pixel build snake web
/// cargo pixel asset input_folder output_folder
/// cargo pixel edit term .
/// cargo pixel edit wg . file.pix
/// cargo pixel petii image.png 40 25
/// cargo pixel ssf t . dance.ssf
/// cargo pixel symbol image.png 8
/// cargo pixel ttf
///
/// shortcut:
/// cargo pixel r snake t
/// cargo pixel r snake s
/// cargo pixel r snake w
/// cargo pixel asset ./sprites ./output     # equivalent to: cargo pixel r asset t -r ./sprites ./output
/// cargo pixel edit t .                     # equivalent to: cargo pixel r edit t -r .
/// cargo pixel edit wg . apps/demo/file.pix # equivalent to: cargo pixel r edit wg -r . apps/demo/file.pix
/// cargo pixel p image.png 40 25            # equivalent to: cargo pixel r petii t -r image.png 40 25
/// cargo pixel sf t . dance.ssf             # equivalent to: cargo pixel r ssf t -r . dance.ssf
/// cargo pixel sy image.png 8               # equivalent to: cargo pixel r symbol t -r image.png 8
/// cargo pixel tf                           # equivalent to: cargo pixel r ttf t -r
/// ...
///
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::str;
use clap::ArgMatches;

mod prepare_env;
use prepare_env::*;
mod command;
use command::*;
mod build_run;
use build_run::*;
mod creat;
use creat::*;
mod convert_gif;
use convert_gif::*;

// current dir state
// not pixel dir, rust_pixel root dir, depend rust_pixel project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
enum PState {
    #[default]
    NotPixel,
    PixelRoot,
    PixelProject,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct PixelContext {
    // rust_pixel repo local path
    rust_pixel_dir: Vec<String>,
    rust_pixel_idx: usize,
    // standalone projects
    projects: Vec<String>,
    project_idx: usize,
    // current dir is root or standalone,
    cdir_state: PState,
}

fn write_config(pc: &PixelContext, config_path: &Path) {
    let toml_string = toml::to_string(pc).expect("Failed to serialize PixelContext");

    let mut file = File::create(config_path).expect("Failed to create config file");
    file.write_all(toml_string.as_bytes())
        .expect("Failed to write to config file");
    println!("🍭 Configuration saved to {}", config_path.display());
}

fn replace_in_files(
    is_standalone: bool,
    dir: &Path,
    rust_pixel_path: &str,
    _dirname: &str,
    capname: &str,
    upname: &str,
    loname: &str,
) {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                replace_in_files(
                    is_standalone,
                    &path,
                    rust_pixel_path,
                    _dirname,
                    capname,
                    upname,
                    loname,
                );
            } else if path.is_file() {
                let fname = path.file_name().and_then(OsStr::to_str);
                let ext = path.extension().and_then(OsStr::to_str);
                if ext == Some("rs")
                    || ext == Some("toml")
                    || fname == Some("Makefile")
                    || fname == Some("index.js")
                    || fname == Some("test.cc")
                    || fname == Some("testffi.py")
                {
                    let content = fs::read(&path).unwrap();
                    let mut content_str = String::from_utf8_lossy(&content).to_string();
                    if is_standalone {
                        content_str = content_str.replace("$RUST_PIXEL_ROOT", rust_pixel_path);
                    }
                    content_str = content_str.replace("Template", capname);
                    content_str = content_str.replace("TEMPLATE", upname);
                    content_str = content_str.replace("template", loname);
                    fs::write(path, content_str).unwrap();
                }
            }
        }
    }
}

fn exec_cmd(cmd: &str) {
    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .expect("failed to execute process");
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Displays usage information for the asset command
fn print_asset_usage() {
    eprintln!("RustPixel Asset Packer");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel asset <INPUT_FOLDER> <OUTPUT_FOLDER>");
    eprintln!("    cargo pixel a <INPUT_FOLDER> <OUTPUT_FOLDER>");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <INPUT_FOLDER>     Folder containing images to pack");
    eprintln!("    <OUTPUT_FOLDER>    Folder where output files will be written");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Packs multiple images into a texture atlas and generates .pix files");
    eprintln!("    for use with the RustPixel engine. Images are automatically resized");
    eprintln!("    and positioned for optimal packing efficiency.");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    texture_atlas.png  - Combined texture atlas");
    eprintln!("    *.pix             - Metadata files for each input image");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo pixel asset ./input_images ./output");
    eprintln!("    cargo pixel asset sprites/ assets/");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    This command is equivalent to: cargo pixel r asset t -r <INPUT_FOLDER> <OUTPUT_FOLDER>");
}

/// Displays usage information for the edit command
fn print_edit_usage() {
    eprintln!("RustPixel Image/Sprite Editor");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel edit <MODE> <WORK_DIR> [FILE_PATH]");
    eprintln!("    cargo pixel e <MODE> <WORK_DIR> [FILE_PATH]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <MODE>       Running mode: t/term, s/sdl, w/web, g/winit, wg/wgpu");
    eprintln!("    <WORK_DIR>   Working directory (usually '.')");
    eprintln!("    [FILE_PATH]  File path to edit (optional, uses default if not specified)");
    eprintln!();
    eprintln!("MODES:");
    eprintln!("    t, term    Terminal mode");
    eprintln!("    s, sdl     SDL2 mode (graphics with OpenGL)");
    eprintln!("    w, web     Web mode (browser)");
    eprintln!("    g, winit   Winit mode (native window with OpenGL)");
    eprintln!("    wg, wgpu   WGPU mode (native window with modern GPU API)");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Launches the RustPixel image and sprite editor for creating and editing");
    eprintln!("    pixel art, sprites, and texture files for use with the RustPixel engine.");
    eprintln!("    If no file path is provided, a default file will be used.");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo pixel edit t .                        # Edit with default file in terminal mode");
    eprintln!("    cargo pixel edit wg .                       # Edit with default file in WGPU mode");
    eprintln!("    cargo pixel edit t . my_image.pix           # Edit my_image.pix in terminal mode");
    eprintln!("    cargo pixel edit s . sprites/player.png     # Edit player.png in SDL mode");
    eprintln!("    cargo pixel edit wg . apps/demo/asset.pix   # Edit asset.pix in WGPU mode");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    This command is equivalent to: cargo pixel r edit <MODE> -r <WORK_DIR> [FILE_PATH]");
}

/// Displays usage information for the petii command
fn print_petii_usage() {
    eprintln!("RustPixel PETSCII Converter");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel petii <IMAGE_FILE> [WIDTH] [HEIGHT] [IS_PETSCII] [CROP_PARAMS...]");
    eprintln!("    cargo pixel p <IMAGE_FILE> [WIDTH] [HEIGHT] [IS_PETSCII] [CROP_PARAMS...]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <IMAGE_FILE>   Input image file path");
    eprintln!("    [WIDTH]        Output width in characters (default: 40)");
    eprintln!("    [HEIGHT]       Output height in characters (default: 25)");
    eprintln!("    [IS_PETSCII]   Use PETSCII characters: true/false (default: false)");
    eprintln!("    [CROP_X]       Crop start X coordinate (requires all crop params)");
    eprintln!("    [CROP_Y]       Crop start Y coordinate");
    eprintln!("    [CROP_WIDTH]   Crop width");
    eprintln!("    [CROP_HEIGHT]  Crop height");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Converts images to PETSCII character art. Supports optional cropping");
    eprintln!("    and customizable output dimensions and character sets.");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo pixel petii image.png                    # Basic conversion");
    eprintln!("    cargo pixel petii image.png 80 50              # Custom size");
    eprintln!("    cargo pixel petii image.png 40 25 true         # Use PETSCII chars");
    eprintln!("    cargo pixel petii image.png 40 25 false 10 10 100 100  # With cropping");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    This command is equivalent to: cargo pixel r petii t -r <ARGS...>");
}

/// Displays usage information for the ssf command
fn print_ssf_usage() {
    eprintln!("RustPixel SSF Sequence Frame Player");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel ssf <MODE> <WORK_DIR> [SSF_FILE]");
    eprintln!("    cargo pixel sf <MODE> <WORK_DIR> [SSF_FILE]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <MODE>       Running mode: t/term, s/sdl, w/web, g/winit, wg/wgpu");
    eprintln!("    <WORK_DIR>   Working directory (usually '.')");
    eprintln!("    [SSF_FILE]   SSF file path (optional, uses default if not specified)");
    eprintln!();
    eprintln!("MODES:");
    eprintln!("    t, term    Terminal mode");
    eprintln!("    s, sdl     SDL2 mode (graphics with OpenGL)");
    eprintln!("    w, web     Web mode (browser)");
    eprintln!("    g, winit   Winit mode (native window with OpenGL)");
    eprintln!("    wg, wgpu   WGPU mode (native window with modern GPU API)");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Plays SSF (Sequence Frame) animation files. Supports various rendering");
    eprintln!("    modes and interactive playback controls.");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo pixel ssf t . dance.ssf              # Play specific file in terminal");
    eprintln!("    cargo pixel ssf wg .                       # Play default file in WGPU mode");
    eprintln!("    cargo pixel ssf s . assets/animation.ssf   # Play with SDL2");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    This command is equivalent to: cargo pixel r ssf <MODE> -r <WORK_DIR> [SSF_FILE]");
}

/// Displays usage information for the symbol command
fn print_symbol_usage() {
    eprintln!("RustPixel Symbol Extractor");
    eprintln!();
    eprintln!("USAGE:");
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
    eprintln!("    or character sets. Supports optional area selection for processing.");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo pixel symbol font.png 8              # Extract 8x8 symbols from entire image");
    eprintln!("    cargo pixel symbol charset.png 16          # Extract 16x16 symbols");
    eprintln!("    cargo pixel symbol image.png 8 0 0 128 64  # Extract from specific area");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    This command is equivalent to: cargo pixel r symbol t -r <ARGS...>");
}

/// Displays usage information for the ttf command
fn print_ttf_usage() {
    eprintln!("RustPixel TTF Font Processor");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel ttf");
    eprintln!("    cargo pixel tf");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Processes TTF font files to generate bitmap character data.");
    eprintln!("    Reads from 'assets/pixel8.ttf' and outputs character bitmaps.");
    eprintln!();
    eprintln!("REQUIREMENTS:");
    eprintln!("    - TTF font file must exist at: assets/pixel8.ttf");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    - Character bitmap data printed to console");
    eprintln!("    - 8x8 pixel character representations");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo pixel ttf                            # Process font file");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    This command is equivalent to: cargo pixel r ttf t -r");
}

/// Handle the asset subcommand by converting it to a run command
/// cargo pixel asset input_folder output_folder -> cargo pixel r asset t -r input_folder output_folder
fn pixel_asset(ctx: &PixelContext, sub_m: &ArgMatches) {
    // Get the input and output folder arguments
    let input_folder = match sub_m.value_of("input_folder") {
        Some(folder) => folder,
        None => {
            eprintln!("Error: Missing required argument <INPUT_FOLDER>");
            eprintln!();
            print_asset_usage();
            return;
        }
    };
    
    let output_folder = match sub_m.value_of("output_folder") {
        Some(folder) => folder,
        None => {
            eprintln!("Error: Missing required argument <OUTPUT_FOLDER>");
            eprintln!();
            print_asset_usage();
            return;
        }
    };
    
    println!("🎨 Running RustPixel Asset Packer...");
    println!("   Input folder: {}", input_folder);
    println!("   Output folder: {}", output_folder);
    println!("   Running: cargo pixel r asset t -r {} {}", input_folder, output_folder);
    println!();
    
    // Create a simulated ArgMatches for the run command
    // This is equivalent to: cargo pixel r asset t -r input_folder output_folder
    use clap::{App, Arg};
    let run_app = App::new("run")
        .arg(Arg::with_name("mod_name"))
        .arg(Arg::with_name("build_type"))
        .arg(Arg::with_name("other").multiple(true))
        .arg(Arg::with_name("release").short('r').long("release"));
    
    let run_args = vec![
        "run",
        "asset",
        "t",         // terminal mode (fastest for tools)
        "-r",        // release mode
        input_folder,
        output_folder,
    ];
    
    let run_matches = run_app.get_matches_from_safe(run_args);
    
    match run_matches {
        Ok(matches) => {
            pixel_run(ctx, &matches);
        }
        Err(e) => {
            eprintln!("Error: Failed to set up asset command: {}", e);
            eprintln!();
            print_asset_usage();
        }
    }
}

/// Handle the edit subcommand by converting it to a run command
/// cargo pixel edit mode work_dir [file_path] -> cargo pixel r edit mode -r work_dir [file_path]
fn pixel_edit(ctx: &PixelContext, sub_m: &ArgMatches) {
    // Get the mode argument
    let mode = match sub_m.value_of("mode") {
        Some(m) => m,
        None => {
            eprintln!("Error: Missing required argument <MODE>");
            eprintln!();
            print_edit_usage();
            return;
        }
    };
    
    // Get the work directory argument
    let work_dir = match sub_m.value_of("work_dir") {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing required argument <WORK_DIR>");
            eprintln!();
            print_edit_usage();
            return;
        }
    };
    
    // Get the file path argument (optional)
    let file_path = sub_m.value_of("file_path");
    
    println!("🎨 Running RustPixel Image/Sprite Editor...");
    println!("   Mode: {}", mode);
    println!("   Work directory: {}", work_dir);
    
    // Create a simulated ArgMatches for the run command
    // This is equivalent to: cargo pixel r edit mode -r work_dir [file_path]
    use clap::{App, Arg};
    let run_app = App::new("run")
        .arg(Arg::with_name("mod_name"))
        .arg(Arg::with_name("build_type"))
        .arg(Arg::with_name("other").multiple(true))
        .arg(Arg::with_name("release").short('r').long("release"));
    
    let mut run_args = vec![
        "run",
        "edit",
        mode,        // The running mode (t, s, w, g, wg, etc.)
        "-r",        // release mode
        work_dir,    // Working directory
    ];
    
    // Add file path if provided
    if let Some(fp) = file_path {
        println!("   File to edit: {}", fp);
        println!("   Running: cargo pixel r edit {} -r {} {}", mode, work_dir, fp);
        run_args.push(fp);
    } else {
        println!("   File to edit: (default)");
        println!("   Running: cargo pixel r edit {} -r {}", mode, work_dir);
    }
    println!();
    
    let run_matches = run_app.get_matches_from_safe(run_args);
    
    match run_matches {
        Ok(matches) => {
            pixel_run(ctx, &matches);
        }
        Err(e) => {
            eprintln!("Error: Failed to set up edit command: {}", e);
            eprintln!();
            print_edit_usage();
        }
    }
}

/// Handle the petii subcommand by converting it to a run command
fn pixel_petii(ctx: &PixelContext, sub_m: &ArgMatches) {
    // Get the image file argument
    let image_file = match sub_m.value_of("image_file") {
        Some(f) => f,
        None => {
            eprintln!("Error: Missing required argument <IMAGE_FILE>");
            eprintln!();
            print_petii_usage();
            return;
        }
    };
    
    println!("🎨 Running RustPixel PETSCII Converter...");
    println!("   Image file: {}", image_file);
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "petii",
        "t",        // terminal mode (only mode supported)
        "-r",       // release mode
        image_file,
    ];
    
    // Add optional arguments if provided
    if let Some(width) = sub_m.value_of("width") {
        println!("   Width: {}", width);
        run_args.push(width);
        
        if let Some(height) = sub_m.value_of("height") {
            println!("   Height: {}", height);
            run_args.push(height);
            
            if let Some(is_petscii) = sub_m.value_of("is_petscii") {
                println!("   PETSCII mode: {}", is_petscii);
                run_args.push(is_petscii);
                
                // Check if all crop parameters are provided
                if let (Some(cx), Some(cy), Some(cw), Some(ch)) = (
                    sub_m.value_of("crop_x"),
                    sub_m.value_of("crop_y"),
                    sub_m.value_of("crop_width"),
                    sub_m.value_of("crop_height"),
                ) {
                    println!("   Crop area: {}x{} at ({},{})", cw, ch, cx, cy);
                    run_args.extend_from_slice(&[cx, cy, cw, ch]);
                }
            }
        }
    }
    
    println!("   Running: cargo pixel r petii t -r {}", run_args[4..].join(" "));
    println!();
    
    // Create and execute the run command
    use clap::{App, Arg};
    let run_app = App::new("run")
        .arg(Arg::with_name("mod_name"))
        .arg(Arg::with_name("build_type"))
        .arg(Arg::with_name("other").multiple(true))
        .arg(Arg::with_name("release").short('r').long("release"));
    
    let run_matches = run_app.get_matches_from_safe(run_args);
    
    match run_matches {
        Ok(matches) => {
            pixel_run(ctx, &matches);
        }
        Err(e) => {
            eprintln!("Error: Failed to set up petii command: {}", e);
            eprintln!();
            print_petii_usage();
        }
    }
}

/// Handle the ssf subcommand by converting it to a run command
fn pixel_ssf(ctx: &PixelContext, sub_m: &ArgMatches) {
    // Get the mode argument
    let mode = match sub_m.value_of("mode") {
        Some(m) => m,
        None => {
            eprintln!("Error: Missing required argument <MODE>");
            eprintln!();
            print_ssf_usage();
            return;
        }
    };
    
    // Get the work directory argument
    let work_dir = match sub_m.value_of("work_dir") {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing required argument <WORK_DIR>");
            eprintln!();
            print_ssf_usage();
            return;
        }
    };
    
    // Get the SSF file argument (optional)
    let ssf_file = sub_m.value_of("ssf_file");
    
    println!("🎨 Running RustPixel SSF Player...");
    println!("   Mode: {}", mode);
    println!("   Work directory: {}", work_dir);
    
    // Create argument list for the run command
    let mut run_args = vec![
        "run",
        "ssf",
        mode,        // The running mode
        "-r",        // release mode
        work_dir,    // Working directory
    ];
    
    // Add SSF file if provided
    if let Some(sf) = ssf_file {
        println!("   SSF file: {}", sf);
        println!("   Running: cargo pixel r ssf {} -r {} {}", mode, work_dir, sf);
        run_args.push(sf);
    } else {
        println!("   SSF file: (default)");
        println!("   Running: cargo pixel r ssf {} -r {}", mode, work_dir);
    }
    println!();
    
    // Create and execute the run command
    use clap::{App, Arg};
    let run_app = App::new("run")
        .arg(Arg::with_name("mod_name"))
        .arg(Arg::with_name("build_type"))
        .arg(Arg::with_name("other").multiple(true))
        .arg(Arg::with_name("release").short('r').long("release"));
    
    let run_matches = run_app.get_matches_from_safe(run_args);
    
    match run_matches {
        Ok(matches) => {
            pixel_run(ctx, &matches);
        }
        Err(e) => {
            eprintln!("Error: Failed to set up ssf command: {}", e);
            eprintln!();
            print_ssf_usage();
        }
    }
}

/// Handle the symbol subcommand by converting it to a run command
fn pixel_symbol(ctx: &PixelContext, sub_m: &ArgMatches) {
    // Get the image file argument
    let image_file = match sub_m.value_of("image_file") {
        Some(f) => f,
        None => {
            eprintln!("Error: Missing required argument <IMAGE_FILE>");
            eprintln!();
            print_symbol_usage();
            return;
        }
    };
    
    // Get the symsize argument
    let symsize = match sub_m.value_of("symsize") {
        Some(s) => s,
        None => {
            eprintln!("Error: Missing required argument <SYMSIZE>");
            eprintln!();
            print_symbol_usage();
            return;
        }
    };
    
    println!("🎨 Running RustPixel Symbol Extractor...");
    println!("   Image file: {}", image_file);
    println!("   Symbol size: {}", symsize);
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "symbol",
        "t",        // terminal mode (only mode supported)
        "-r",       // release mode
        image_file,
        symsize,
    ];
    
    // Add optional crop parameters if all are provided
    if let (Some(sx), Some(sy), Some(w), Some(h)) = (
        sub_m.value_of("start_x"),
        sub_m.value_of("start_y"),
        sub_m.value_of("width"),
        sub_m.value_of("height"),
    ) {
        println!("   Processing area: {}x{} at ({},{})", w, h, sx, sy);
        run_args.extend_from_slice(&[sx, sy, w, h]);
    }
    
    println!("   Running: cargo pixel r symbol t -r {}", run_args[4..].join(" "));
    println!();
    
    // Create and execute the run command
    use clap::{App, Arg};
    let run_app = App::new("run")
        .arg(Arg::with_name("mod_name"))
        .arg(Arg::with_name("build_type"))
        .arg(Arg::with_name("other").multiple(true))
        .arg(Arg::with_name("release").short('r').long("release"));
    
    let run_matches = run_app.get_matches_from_safe(run_args);
    
    match run_matches {
        Ok(matches) => {
            pixel_run(ctx, &matches);
        }
        Err(e) => {
            eprintln!("Error: Failed to set up symbol command: {}", e);
            eprintln!();
            print_symbol_usage();
        }
    }
}

/// Handle the ttf subcommand by converting it to a run command
fn pixel_ttf(ctx: &PixelContext, _sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel TTF Font Processor...");
    println!("   Processing: assets/pixel8.ttf");
    println!("   Running: cargo pixel r ttf t -r");
    println!();
    
    // Create argument list for the run command
    let run_args = vec![
        "run",
        "ttf",
        "t",        // terminal mode (only mode supported)
        "-r",       // release mode
    ];
    
    // Create and execute the run command
    use clap::{App, Arg};
    let run_app = App::new("run")
        .arg(Arg::with_name("mod_name"))
        .arg(Arg::with_name("build_type"))
        .arg(Arg::with_name("other").multiple(true))
        .arg(Arg::with_name("release").short('r').long("release"));
    
    let run_matches = run_app.get_matches_from_safe(run_args);
    
    match run_matches {
        Ok(matches) => {
            pixel_run(ctx, &matches);
        }
        Err(e) => {
            eprintln!("Error: Failed to set up ttf command: {}", e);
            eprintln!();
            print_ttf_usage();
        }
    }
}

fn main() {
    let ctx = check_pixel_env();
    // println!("{:?}", ctx);
    let args = make_parser();
    match args.subcommand() {
        Some(("run", sub_m)) => pixel_run(&ctx, sub_m),
        Some(("build", sub_m)) => pixel_build(&ctx, sub_m),
        Some(("creat", sub_m)) => pixel_creat(&ctx, sub_m),
        Some(("convert_gif", sub_m)) => pixel_convert_gif(&ctx, sub_m),
        Some(("asset", sub_m)) => pixel_asset(&ctx, sub_m),
        Some(("edit", sub_m)) => pixel_edit(&ctx, sub_m),
        Some(("petii", sub_m)) => pixel_petii(&ctx, sub_m),
        Some(("ssf", sub_m)) => pixel_ssf(&ctx, sub_m),
        Some(("symbol", sub_m)) => pixel_symbol(&ctx, sub_m),
        Some(("ttf", sub_m)) => pixel_ttf(&ctx, sub_m),
        _ => {}
    }
}
