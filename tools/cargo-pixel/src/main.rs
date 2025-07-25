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




/// Handle the asset subcommand by converting it to a run command
/// cargo pixel asset input_folder output_folder -> cargo pixel r asset t -r input_folder output_folder
fn pixel_asset(ctx: &PixelContext, sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel Asset Packer...");
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "asset",
        "t",         // terminal mode (fastest for tools)
        "-r",        // release mode
    ];
    
    // Add all provided arguments
    if let Some(input_folder) = sub_m.value_of("input_folder") {
        run_args.push(input_folder);
    }
    if let Some(output_folder) = sub_m.value_of("output_folder") {
        run_args.push(output_folder);
    }
    
    println!("   Running: cargo pixel r asset t -r {}", run_args[4..].join(" "));
    println!();
    
    // Create a simulated ArgMatches for the run command
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
            eprintln!("Error: Failed to set up asset command: {}", e);
        }
    }
}

/// Handle the edit subcommand by converting it to a run command
/// cargo pixel edit mode work_dir [file_path] -> cargo pixel r edit mode -r work_dir [file_path]
fn pixel_edit(ctx: &PixelContext, sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel Image/Sprite Editor...");
    
    // For edit tool, default to terminal mode if not specified
    let mode = sub_m.value_of("mode").unwrap_or("t");
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "edit",
        mode,        // The running mode
        "-r",        // release mode
    ];
    
    // Add work directory if provided
    if let Some(work_dir) = sub_m.value_of("work_dir") {
        run_args.push(work_dir);
    }
    
    // Add file path if provided
    if let Some(file_path) = sub_m.value_of("file_path") {
        run_args.push(file_path);
    }
    
    println!("   Running: cargo pixel r edit {} -r {}", mode, run_args[4..].join(" "));
    println!();
    
    // Create a simulated ArgMatches for the run command
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
            eprintln!("Error: Failed to set up edit command: {}", e);
        }
    }
}

/// Handle the petii subcommand by converting it to a run command
fn pixel_petii(ctx: &PixelContext, sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel PETSCII Converter...");
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "petii",
        "t",        // terminal mode (only mode supported)
        "-r",       // release mode
    ];
    
    // Add all provided arguments
    if let Some(image_file) = sub_m.value_of("image_file") {
        run_args.push(image_file);
    }
    if let Some(width) = sub_m.value_of("width") {
        run_args.push(width);
    }
    if let Some(height) = sub_m.value_of("height") {
        run_args.push(height);
    }
    if let Some(is_petscii) = sub_m.value_of("is_petscii") {
        run_args.push(is_petscii);
    }
    if let Some(crop_x) = sub_m.value_of("crop_x") {
        run_args.push(crop_x);
    }
    if let Some(crop_y) = sub_m.value_of("crop_y") {
        run_args.push(crop_y);
    }
    if let Some(crop_width) = sub_m.value_of("crop_width") {
        run_args.push(crop_width);
    }
    if let Some(crop_height) = sub_m.value_of("crop_height") {
        run_args.push(crop_height);
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
        }
    }
}

/// Handle the ssf subcommand by converting it to a run command
/// For pixel_game! based tools, we need to provide build_type but let tool handle params
fn pixel_ssf(ctx: &PixelContext, sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel SSF Player...");
    
    // For ssf tool (uses pixel_game! macro), we need to provide the mode/build_type
    // Default to terminal mode, but use provided mode if available
    let mode = sub_m.value_of("mode").unwrap_or("t");
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "ssf", 
        mode,       // Build type is required by build_run.rs
        "-r",       // release mode
    ];
    
    // Add remaining arguments that will be passed to ssf tool
    if let Some(work_dir) = sub_m.value_of("work_dir") {
        run_args.push(work_dir);
    }
    if let Some(ssf_file) = sub_m.value_of("ssf_file") {
        run_args.push(ssf_file);
    }
    
    println!("   Running: cargo pixel r ssf {} -r {}", mode, run_args[4..].join(" "));
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
        }
    }
}

/// Handle the symbol subcommand by converting it to a run command
fn pixel_symbol(ctx: &PixelContext, sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel Symbol Extractor...");
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "symbol",
        "t",        // terminal mode (only mode supported)
        "-r",       // release mode
    ];
    
    // Add all provided arguments
    if let Some(image_file) = sub_m.value_of("image_file") {
        run_args.push(image_file);
    }
    if let Some(symsize) = sub_m.value_of("symsize") {
        run_args.push(symsize);
    }
    if let Some(start_x) = sub_m.value_of("start_x") {
        run_args.push(start_x);
    }
    if let Some(start_y) = sub_m.value_of("start_y") {
        run_args.push(start_y);
    }
    if let Some(width) = sub_m.value_of("width") {
        run_args.push(width);
    }
    if let Some(height) = sub_m.value_of("height") {
        run_args.push(height);
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
        }
    }
}

/// Handle the ttf subcommand by converting it to a run command
fn pixel_ttf(ctx: &PixelContext, _sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel TTF Font Processor...");
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
        _ => {
            // No subcommand provided, show help
            use crate::command::make_parser_app;
            let mut app = make_parser_app();
            let _ = app.print_help();
            println!(); // Add a newline after help
        }
    }
}
