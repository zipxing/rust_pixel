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
use std::env;
use std::path::Path;
use std::str;
use std::process::Command;
use std::fs;

use crate::capitalize;
use crate::exec_cmd;
use crate::PState;
use crate::PixelContext;

// run subcommand entry...
pub fn pixel_run(ctx: &PixelContext, args: &ArgMatches) {
    if ctx.cdir_state == PState::NotPixel {
        println!("🚫 Not pixel directory.");
        return;
    }
    let cmds = get_cmds(ctx, args, "run");
    for cmd in cmds {
        println!("🍀 {}", cmd);
        exec_cmd(&cmd);
    }
}

// build subcommand entry...
pub fn pixel_build(ctx: &PixelContext, args: &ArgMatches) {
    if ctx.cdir_state == PState::NotPixel {
        println!("🚫 Not pixel directory.");
        return;
    }
    let cmds = get_cmds(ctx, args, "build");
    for cmd in cmds {
        println!("🍀 {}", cmd);
        exec_cmd(&cmd);
    }
}

fn get_cmds(ctx: &PixelContext, args: &ArgMatches, subcmd: &str) -> Vec<String> {
    let mut cmds = Vec::new();
    let mod_name = args.get_one::<String>("mod_name").unwrap();
    let loname = mod_name.to_lowercase();
    let capname = capitalize(mod_name);
    let build_type = args.get_one::<String>("build_type").unwrap();
    let release = if args.get_flag("release") {
        "--release"
    } else {
        ""
    };
    let webport = args
        .get_one::<String>("webport")
        .map(|s| s.as_str())
        .unwrap_or("8080");

    match build_type.as_str() {
        "term" | "t" => cmds.push(format!(
            "cargo {} -p {} --features term {} {}",
            subcmd, // build or run
            mod_name,
            release,
            args.get_many::<String>("other")
                .unwrap_or_default()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
        )),
        "glow" | "g" => cmds.push(format!(
            "cargo {} -p {} --features glow {} {}",
            subcmd, // build or run
            mod_name,
            release,
            args.get_many::<String>("other")
                .unwrap_or_default()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
        )),
        "wgpu" | "wg" => cmds.push(format!(
            "cargo {} -p {} --features wgpu {} {}",
            subcmd, // build or run
            mod_name,
            release,
            args.get_many::<String>("other")
                .unwrap_or_default()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
        )),
        "sdl" | "s" => cmds.push(format!(
            "cargo {} -p {} --features sdl {} {}",
            subcmd, // build or run
            mod_name,
            release,
            args.get_many::<String>("other")
                .unwrap_or_default()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
        )),
        "web" | "w" => {
            let mut crate_path = "".to_string();
            if ctx.cdir_state == PState::PixelProject {
                // standalone
                crate_path = ".".to_string();
            } else if ctx.cdir_state == PState::PixelRoot {
                // root
                let cpath = Path::new("apps").join(&mod_name);
                if cpath.exists() {
                    crate_path = cpath.to_string_lossy().to_string();
                }
            }

            // Execute wasm-pack build directly
            env::set_var("RUSTFLAGS", r#"--cfg getrandom_backend="wasm_js""#);
            
            let mut wasm_cmd = Command::new("wasm-pack");
            wasm_cmd.args(&["build", "--target", "web", &crate_path]);
            
            if !release.is_empty() {
                wasm_cmd.arg(&release);
            }
            
            // Add other arguments
            if let Some(other_args) = args.get_many::<String>("other") {
                for arg in other_args {
                    wasm_cmd.arg(arg);
                }
            }
            
            println!("🍀 Executing: {:?}", wasm_cmd);
            let output = wasm_cmd.output().expect("Failed to execute wasm-pack");
            
            if !output.status.success() {
                eprintln!("❌ wasm-pack failed:");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                return Vec::new();
            }
            
            println!("✅ wasm-pack build completed successfully");

            // Cross-platform file operations using Rust std library
            let tmpwd_path = Path::new("tmp").join(format!("web_{}", mod_name));
            
            // Clean up and create temporary directory
            if tmpwd_path.exists() {
                if let Err(e) = fs::remove_dir_all(&tmpwd_path) {
                    eprintln!("Warning: Failed to remove directory {:?}: {}", tmpwd_path, e);
                }
            }
            if let Err(e) = fs::create_dir_all(&tmpwd_path) {
                eprintln!("Error: Failed to create directory {:?}: {}", tmpwd_path, e);
                return Vec::new();
            }
            
            // Copy assets if exists
            let assets_src = Path::new(&crate_path).join("assets");
            if assets_src.exists() {
                let assets_dst = tmpwd_path.join("assets");
                if let Err(e) = copy_dir_all(&assets_src, &assets_dst) {
                    eprintln!("Warning: Failed to copy assets: {}", e);
                }
            }
            
            // Copy web-templates
            let rust_pixel_path = Path::new(&ctx.rust_pixel_dir[ctx.rust_pixel_idx]);
            let templates_src = rust_pixel_path.join("web-templates");
            if let Err(e) = copy_dir_contents(&templates_src, &tmpwd_path) {
                eprintln!("Warning: Failed to copy web templates: {}", e);
            }
            
            // Replace content in index.js
            let index_js_path = tmpwd_path.join("index.js");
            if index_js_path.exists() {
                if let Ok(content) = fs::read_to_string(&index_js_path) {
                    let content = content.replace("Pixel", &capname).replace("pixel", &loname);
                    if let Err(e) = fs::write(&index_js_path, content) {
                        eprintln!("Warning: Failed to update index.js: {}", e);
                    }
                }
            }
            
            // Copy pkg directory (generated by wasm-pack)
            let pkg_src = Path::new(&crate_path).join("pkg");
            if pkg_src.exists() {
                let pkg_dst = tmpwd_path.join("pkg");
                if let Err(e) = copy_dir_all(&pkg_src, &pkg_dst) {
                    eprintln!("Warning: Failed to copy pkg: {}", e);
                }
            }

            if subcmd == "run" {
                cmds.push(format!("python3 -m http.server -d {} {}", tmpwd_path.display(), webport));
            }
        }
        _ => {}
    }

    cmds
}

// Helper function to recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source directory does not exist: {:?}", src)
        ));
    }
    
    // Create destination directory
    fs::create_dir_all(dst)?;
    
    // Copy all entries in the source directory
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_entry = entry.path();
        let dst_entry = dst.join(entry.file_name());
        
        if src_entry.is_dir() {
            copy_dir_all(&src_entry, &dst_entry)?;
        } else {
            fs::copy(&src_entry, &dst_entry)?;
        }
    }
    
    Ok(())
}

// Helper function to copy all contents of a directory to another directory
fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source directory does not exist: {:?}", src)
        ));
    }
    
    // Create destination directory if it doesn't exist
    fs::create_dir_all(dst)?;
    
    // Copy all entries from source to destination
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_entry = entry.path();
        let dst_entry = dst.join(entry.file_name());
        
        if src_entry.is_dir() {
            copy_dir_all(&src_entry, &dst_entry)?;
        } else {
            fs::copy(&src_entry, &dst_entry)?;
        }
    }
    
    Ok(())
}
