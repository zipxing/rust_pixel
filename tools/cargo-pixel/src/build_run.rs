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
use std::path::Path;
use std::str;
use std::env;

use crate::PixelContext;
use crate::PState;
use crate::exec_cmd;
use crate::capitalize;

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
    let webport = args.get_one::<String>("webport").map(|s| s.as_str()).unwrap_or("8080");

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
            "cargo {} -p {} --features winit {} {}",
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
                let cpath = format!("apps/{}", mod_name);
                if Path::new(&cpath).exists() {
                    crate_path = cpath;
                }
            }
            
            // Cross-platform wasm-pack build command
            if cfg!(target_os = "windows") {
                env::set_var("RUSTFLAGS", r#"--cfg getrandom_backend="wasm_js""#);
                // Windows: Use PowerShell script block to avoid complex escaping
                cmds.push(format!(
                    "wasm-pack build --target web {} {} {}",
                    crate_path,
                    release,
                    args.get_many::<String>("other")
                        .unwrap_or_default()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ")
                ));
            } else {
                // Unix-like systems (Linux, macOS)
                cmds.push(format!(
                    "RUSTFLAGS='--cfg getrandom_backend=\"wasm_js\"' wasm-pack build --target web {} {} {}",
                    crate_path,
                    release,
                    args.get_many::<String>("other")
                        .unwrap_or_default()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ")
                ));
            }
            
            let tmpwd = format!("tmp/web_{}/", mod_name);
            
            // Cross-platform directory and file operations
            if cfg!(target_os = "windows") {
                // Windows: Use cmd commands instead of PowerShell
                cmds.push(format!("if exist \"{}\" rmdir /s /q \"{}\"", tmpwd, tmpwd));
                cmds.push(format!("mkdir \"{}\"", tmpwd));
                
                // Copy assets if exists
                cmds.push(format!(
                    "if exist \"{}/assets\" robocopy \"{}/assets\" \"{}/assets\" /E /NFL /NDL /NJH /NJS /nc /ns /np",
                    crate_path, crate_path, tmpwd
                ));
                
                // Copy web-templates
                cmds.push(format!(
                    "robocopy \"{}/web-templates\" \"{}\" /E /NFL /NDL /NJH /NJS /nc /ns /np",
                    ctx.rust_pixel_dir[ctx.rust_pixel_idx], tmpwd
                ));
                
                // Replace content in index.js using Python (more reliable than PowerShell)
                cmds.push(format!(
                    "python -c \"import os; content=open('{}/index.js','r',encoding='utf-8').read().replace('Pixel','{}').replace('pixel','{}'); open('{}/index.js','w',encoding='utf-8').write(content)\" 2>nul || echo Warning: Failed to update index.js",
                    tmpwd, capname, loname, tmpwd
                ));
                
                // Copy pkg if exists (this happens after wasm-pack)
                cmds.push(format!(
                    "if exist \"{}/pkg\" robocopy \"{}/pkg\" \"{}/pkg\" /E /NFL /NDL /NJH /NJS /nc /ns /np",
                    crate_path, crate_path, tmpwd
                ));
            } else {
                // Unix-like systems (Linux, macOS)
                cmds.push(format!("rm -rf {}/*", tmpwd));
                cmds.push(format!("mkdir -p {}", tmpwd));
                
                // Copy assets if exists
                cmds.push(format!("[ -d \"{}/assets\" ] && cp -r \"{}/assets\" \"{}\" || true", crate_path, crate_path, tmpwd));
                
                // Copy web-templates
                cmds.push(format!(
                    "cp \"{}/web-templates/\"* \"{}\"",
                    ctx.rust_pixel_dir[ctx.rust_pixel_idx], tmpwd
                ));
                
                // Replace content in index.js
                cmds.push(format!(
                    "sed -i.bak 's/Pixel/{}/g; s/pixel/{}/g' \"{}/index.js\" && rm \"{}/index.js.bak\" 2>/dev/null || true",
                    capname, loname, tmpwd, tmpwd
                ));
                
                // Copy pkg if exists (this happens after wasm-pack)
                cmds.push(format!("[ -d \"{}/pkg\" ] && cp -r \"{}/pkg\" \"{}\" || true", crate_path, crate_path, tmpwd));
            }
            
            if subcmd == "run" {
                cmds.push(format!("python3 -m http.server -d {} {}", tmpwd, webport));
            }
        }
        _ => {}
    }

    cmds
}



