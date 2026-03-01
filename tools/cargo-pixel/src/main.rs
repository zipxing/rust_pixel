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
/// cargo pixel ttf font.ttf output.png 8
/// cargo pixel gen "Rust语言入门"
/// cargo pixel gen "Rust语言入门" --img
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
/// cargo pixel tf font.ttf output.png 8    # equivalent to: cargo pixel r ttf t -r font.ttf output.png 8
/// cargo pixel g "Rust语言入门"             # equivalent to: cargo run -p mdpt --features wgpu --bin gen -- "topic"
/// cargo pixel g "topic" --img              # also generate images for each slide
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
mod symbols;

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

/// Execute a command using cross-platform shell
/// For simple commands, consider using Command::new() directly instead
fn exec_cmd(cmd: &str) {
    println!("🍀 Executing: {}", cmd);
    
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", cmd])
            .status()
            .expect("failed to execute process");
        
        if !status.success() {
            eprintln!("❌ Command failed with exit code: {:?}", status.code());
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .expect("failed to execute process");
            
        if !status.success() {
            eprintln!("❌ Command failed with exit code: {:?}", status.code());
        }
    }
}

/// Cross-platform directory creation
fn create_dir_cmd(dir: &str) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("Failed to create directory {}: {}", dir, e);
    }
}

/// Cross-platform file/directory copying
fn copy_cmd(from: &str, to: &str, recursive: bool) {
    if recursive {
        copy_dir_recursive(from, to).unwrap_or_else(|_| panic!("Failed to copy {} to {}", from, to));
    } else {
        std::fs::copy(from, to).unwrap_or_else(|_| panic!("Failed to copy file {} to {}", from, to));
    }
}

/// Cross-platform directory removal
fn remove_dir_cmd(dir: &str, recursive: bool) {
    let result = if recursive {
        std::fs::remove_dir_all(dir)
    } else {
        std::fs::remove_dir(dir)
    };
    
    if let Err(e) = result {
        eprintln!("Failed to remove directory {}: {}", dir, e);
    }
}

/// Recursively copy a directory and its contents
fn copy_dir_recursive_impl(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    if !to.exists() {
        std::fs::create_dir_all(to)?;
    }
    
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = to.join(entry.file_name());
        
        if entry_path.is_dir() {
            copy_dir_recursive_impl(&entry_path, &dest_path)?;
        } else {
            std::fs::copy(&entry_path, &dest_path)?;
        }
    }
    
    Ok(())
}

/// Helper function to handle string paths for copy_dir_recursive
fn copy_dir_recursive(from: &str, to: &str) -> std::io::Result<()> {
    copy_dir_recursive_impl(std::path::Path::new(from), std::path::Path::new(to))
}

/// Cross-platform wildcard file removal using pure Rust
/// Removes files matching simple patterns
#[allow(dead_code)]
fn remove_files_pattern(pattern: &str) {
    use std::path::Path;
    
    // Parse pattern to extract directory and filename pattern
    let path = Path::new(pattern);
    let parent_dir = path.parent().unwrap_or(Path::new("."));
    let file_pattern = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    if parent_dir.exists() && parent_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(parent_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                
                // Simple pattern matching for common cases
                let should_remove = if file_pattern == "t*.p*" {
                    // Match files like t1.png, t2.pix, etc.
                    file_name_str.starts_with('t') && file_name_str.contains(".p")
                } else if file_pattern.contains('*') {
                    // Generic wildcard matching - simple implementation
                    let parts: Vec<&str> = file_pattern.split('*').collect();
                    if parts.len() == 2 {
                        file_name_str.starts_with(parts[0]) && file_name_str.ends_with(parts[1])
                    } else {
                        false
                    }
                } else {
                    // Exact match
                    file_name_str == file_pattern
                };
                
                if should_remove {
                    if let Err(e) = std::fs::remove_file(entry.path()) {
                        eprintln!("Warning: Failed to remove file {:?}: {}", entry.path(), e);
                    }
                }
            }
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

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
    if let Some(input_folder) = sub_m.get_one::<String>("input_folder") {
        run_args.push(input_folder.as_str());
    }
    if let Some(output_folder) = sub_m.get_one::<String>("output_folder") {
        run_args.push(output_folder.as_str());
    }
    
    println!("   Running: cargo pixel r asset t -r {}", run_args[4..].join(" "));
    println!();
    
    // Create a simulated ArgMatches for the run command
    use clap::{Command, Arg, ArgAction};
    let run_app = Command::new("run")
        .arg(Arg::new("mod_name"))
        .arg(Arg::new("build_type"))
        .arg(Arg::new("other").action(ArgAction::Append))
        .arg(Arg::new("release").short('r').long("release").action(ArgAction::SetTrue));
    
    let run_matches = run_app.try_get_matches_from(run_args);
    
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
/// cargo pixel edit [mode] [work_dir] [image_file] -> cargo pixel r edit mode -r [work_dir] [image_file]
fn pixel_edit(ctx: &PixelContext, sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel Image/Sprite Editor...");
    
    // For edit tool, default to terminal mode if not specified
    let mode = sub_m.get_one::<String>("mode").map(|s| s.as_str()).unwrap_or("t");
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "edit",
        mode,        // The running mode
        "-r",        // release mode
    ];
    
    // Add work directory if provided
    if let Some(work_dir) = sub_m.get_one::<String>("work_dir") {
        run_args.push(work_dir.as_str());
    }
    
    // Add image file if provided
    if let Some(image_file) = sub_m.get_one::<String>("image_file") {
        run_args.push(image_file.as_str());
    }
    
    println!("   Running: cargo pixel r edit {} -r {}", mode, run_args[4..].join(" "));
    println!();
    
    // Create a simulated ArgMatches for the run command
    use clap::{Command, Arg, ArgAction};
    let run_app = Command::new("run")
        .arg(Arg::new("mod_name"))
        .arg(Arg::new("build_type"))
        .arg(Arg::new("other").action(ArgAction::Append))
        .arg(Arg::new("release").short('r').long("release").action(ArgAction::SetTrue));
    
    let run_matches = run_app.try_get_matches_from(run_args);
    
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
    
    // Add the required image file argument
    if let Some(image_file) = sub_m.get_one::<String>("image_file") {
        run_args.push(image_file.as_str());
    } else {
        eprintln!("Error: Image file is required");
        return;
    }
    
    // Add all optional arguments in order
    if let Some(width) = sub_m.get_one::<String>("width") {
        run_args.push(width.as_str());
    }
    if let Some(height) = sub_m.get_one::<String>("height") {
        run_args.push(height.as_str());
    }
    if let Some(is_petscii) = sub_m.get_one::<String>("is_petscii") {
        run_args.push(is_petscii.as_str());
    }
    if let Some(crop_x) = sub_m.get_one::<String>("crop_x") {
        run_args.push(crop_x.as_str());
    }
    if let Some(crop_y) = sub_m.get_one::<String>("crop_y") {
        run_args.push(crop_y.as_str());
    }
    if let Some(crop_width) = sub_m.get_one::<String>("crop_width") {
        run_args.push(crop_width.as_str());
    }
    if let Some(crop_height) = sub_m.get_one::<String>("crop_height") {
        run_args.push(crop_height.as_str());
    }
    
    println!("   Running: cargo pixel r petii t -r {}", run_args[4..].join(" "));
    println!();
    
    // Create and execute the run command
    use clap::{Command, Arg, ArgAction};
    let run_app = Command::new("run")
        .arg(Arg::new("mod_name"))
        .arg(Arg::new("build_type"))
        .arg(Arg::new("other").action(ArgAction::Append))
        .arg(Arg::new("release").short('r').long("release").action(ArgAction::SetTrue));
    
    let run_matches = run_app.try_get_matches_from(run_args);
    
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
    // Check if no arguments are provided, show help
    if sub_m.get_one::<String>("work_dir").is_none() {
        use crate::command::make_parser_app;
        let mut app = make_parser_app();
        if let Some(ssf_subcommand) = app.find_subcommand_mut("ssf") {
            let _ = ssf_subcommand.print_help();
            println!(); // Add a newline after help
        }
        return;
    }
    
    println!("🎨 Running RustPixel SSF Player (WGPU mode)...");
    
    // Fixed to wgpu mode
    let mode = "wg";
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "ssf", 
        mode,       // Build type is required by build_run.rs
        "-r",       // release mode
    ];
    
    // Add remaining arguments that will be passed to ssf tool
    if let Some(work_dir) = sub_m.get_one::<String>("work_dir") {
        run_args.push(work_dir.as_str());
    }
    if let Some(ssf_file) = sub_m.get_one::<String>("ssf_file") {
        run_args.push(ssf_file.as_str());
    }
    
    println!("   Running: cargo pixel r ssf {} -r {}", mode, run_args[4..].join(" "));
    println!();
    
    // Create and execute the run command
    use clap::{Command, Arg, ArgAction};
    let run_app = Command::new("run")
        .arg(Arg::new("mod_name"))
        .arg(Arg::new("build_type"))
        .arg(Arg::new("other").action(ArgAction::Append))
        .arg(Arg::new("release").short('r').long("release").action(ArgAction::SetTrue));
    
    let run_matches = run_app.try_get_matches_from(run_args);
    
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
    
    // Add all provided arguments in order
    if let Some(image_file) = sub_m.get_one::<String>("image_file") {
        run_args.push(image_file.as_str());
    }
    if let Some(symsize) = sub_m.get_one::<String>("symsize") {
        run_args.push(symsize.as_str());
    }
    if let Some(start_x) = sub_m.get_one::<String>("start_x") {
        run_args.push(start_x.as_str());
    }
    if let Some(start_y) = sub_m.get_one::<String>("start_y") {
        run_args.push(start_y.as_str());
    }
    if let Some(width) = sub_m.get_one::<String>("width") {
        run_args.push(width.as_str());
    }
    if let Some(height) = sub_m.get_one::<String>("height") {
        run_args.push(height.as_str());
    }
    
    println!("   Running: cargo pixel r symbol t -r {}", run_args[4..].join(" "));
    println!();
    
    // Create and execute the run command
    use clap::{Command, Arg, ArgAction};
    let run_app = Command::new("run")
        .arg(Arg::new("mod_name"))
        .arg(Arg::new("build_type"))
        .arg(Arg::new("other").action(ArgAction::Append))
        .arg(Arg::new("release").short('r').long("release").action(ArgAction::SetTrue));
    
    let run_matches = run_app.try_get_matches_from(run_args);
    
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
fn pixel_ttf(ctx: &PixelContext, sub_m: &ArgMatches) {
    println!("🎨 Running RustPixel TTF to PNG Converter...");
    
    // Build argument list for the run command
    let mut run_args = vec![
        "run",
        "ttf",
        "t",        // terminal mode (only mode supported)
        "-r",       // release mode
    ];
    
    // Add all positional arguments in order
    if let Some(ttf_file) = sub_m.get_one::<String>("ttf_file") {
        run_args.push(ttf_file.as_str());
    }
    if let Some(output_file) = sub_m.get_one::<String>("output_file") {
        run_args.push(output_file.as_str());
    }
    if let Some(size) = sub_m.get_one::<String>("size") {
        run_args.push(size.as_str());
    }
    if let Some(chars_per_row) = sub_m.get_one::<String>("chars_per_row") {
        run_args.push(chars_per_row.as_str());
    }
    if let Some(verbose) = sub_m.get_one::<String>("verbose") {
        run_args.push(verbose.as_str());
    }
    
    println!("   Running: cargo pixel r ttf t -r {}", run_args[4..].join(" "));
    println!();
    
    // Create and execute the run command
    use clap::{Command, Arg, ArgAction};
    let run_app = Command::new("run")
        .arg(Arg::new("mod_name"))
        .arg(Arg::new("build_type"))
        .arg(Arg::new("other").action(ArgAction::Append))
        .arg(Arg::new("release").short('r').long("release").action(ArgAction::SetTrue));
    
    let run_matches = run_app.try_get_matches_from(run_args);
    
    match run_matches {
        Ok(matches) => {
            pixel_run(ctx, &matches);
        }
        Err(e) => {
            eprintln!("Error: Failed to set up ttf command: {}", e);
        }
    }
}

/// Handle the gen subcommand - generate MDPT presentation using AI
/// cargo pixel gen "topic" [--img] -> cargo run -p mdpt --features wgpu --bin gen -- "topic" [--img]
fn pixel_gen(ctx: &PixelContext, sub_m: &ArgMatches) {
    if ctx.cdir_state == PState::NotPixel {
        println!("🚫 Not pixel directory.");
        return;
    }

    let topic = sub_m.get_one::<String>("topic").unwrap();
    let gen_img = sub_m.get_flag("img");

    let mut cmd = format!(
        "cargo run -p mdpt --features wgpu --bin gen -- \"{}\"",
        topic
    );
    if gen_img {
        cmd.push_str(" --img");
    }

    println!("🤖 Generating MDPT presentation: \"{}\"", topic);
    if gen_img {
        println!("   (with image generation enabled)");
    }
    println!();
    exec_cmd(&cmd);
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
        Some(("gen", sub_m)) => pixel_gen(&ctx, sub_m),
        Some(("symbols", sub_m)) => symbols::generate_symbols(sub_m),
        _ => {
            // No subcommand provided, show help
            use crate::command::make_parser_app;
            let mut app = make_parser_app();
            let _ = app.print_help();
            println!(); // Add a newline after help
        }
    }
}
