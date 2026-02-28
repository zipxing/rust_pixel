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
use std::fs;
use std::path::Path;

use crate::PixelContext;
use crate::PState;
use crate::{create_dir_cmd, copy_cmd, remove_dir_cmd};
use crate::replace_in_files;
use crate::write_config;
use crate::capitalize;

// crate subcommand entry...
pub fn pixel_creat(ctx: &PixelContext, args: &ArgMatches) {
    if ctx.cdir_state != PState::PixelRoot {
        println!("🚫 Cargo pixel creat must run in rust_pixel root directory.");
        return;
    }
    let mut dir_name = "apps".to_string();
    let sa_dir = args.get_one::<String>("standalone_dir_name");
    let mod_name = args.get_one::<String>("mod_name").unwrap();
    let mut is_standalone = false;
    let upname = mod_name.to_uppercase();
    let loname = mod_name.to_lowercase();
    let capname = capitalize(mod_name);

    let cdir;
    if let Some(sdir) = sa_dir {
        cdir = sdir.to_string();
        println!(
            "🍀 creat standalone app folder...({}/{}/)",
            sdir, mod_name
        );
    } else {
        cdir = dir_name.to_string();
        println!(
            "🍀 creat app folder...({}/{}/)",
            dir_name, mod_name
        );
    }

    // Use cross-platform paths
    let tmp_template = Path::new("tmp").join("pixel_game_template");
    let _ = fs::remove_dir_all(&tmp_template);
    let _ = fs::create_dir_all(cdir);
    create_dir_cmd("tmp");
    
    let apps_template = Path::new("apps").join("template");
    copy_cmd(&apps_template.to_string_lossy(), &tmp_template.to_string_lossy(), true);

    if let Some(stand_dir) = sa_dir {
        is_standalone = true;
        
        // Cross-platform standalone template paths
        let standalone_path = apps_template.join("stand-alone");
        copy_cmd(
            &standalone_path.join("Cargo.toml.temp").to_string_lossy(), 
            &tmp_template.join("Cargo.toml").to_string_lossy(), 
            false
        );
        copy_cmd(
            &standalone_path.join("LibCargo.toml.temp").to_string_lossy(), 
            &tmp_template.join("lib").join("Cargo.toml").to_string_lossy(), 
            false
        );
        copy_cmd(
            &standalone_path.join("FfiCargo.toml.temp").to_string_lossy(), 
            &tmp_template.join("ffi").join("Cargo.toml").to_string_lossy(), 
            false
        );
        copy_cmd(
            &standalone_path.join("WasmCargo.toml.temp").to_string_lossy(), 
            &tmp_template.join("wasm").join("Cargo.toml").to_string_lossy(), 
            false
        );
        dir_name = stand_dir.to_string();

        copy_cmd("build_support.rs", &tmp_template.join("build_support.rs").to_string_lossy(), false);
        // Update build.rs first line to include!("build_support.rs");
        let build_rs_path = tmp_template.join("build.rs");
        if let Ok(lines) = fs::read_to_string(&build_rs_path) {
            let mut new_content = String::from("include!(\"build_support.rs\");\n");
            // Skip first line
            if let Some(pos) = lines.find('\n') {
                new_content.push_str(&lines[pos+1..]);
            }
            let _ = fs::write(&build_rs_path, new_content);
        }
    }
    let standalone_dir = tmp_template.join("stand-alone");
    remove_dir_cmd(&standalone_dir.to_string_lossy(), true);

    replace_in_files(
        is_standalone,
        &tmp_template,
        &ctx.rust_pixel_dir[ctx.rust_pixel_idx],
        &dir_name,
        &capname,
        &upname,
        &loname,
    );

    let new_path_buf = Path::new(&dir_name).join(mod_name);
    let mut new_path = new_path_buf.to_string_lossy().to_string();
    let mut count = 0;
    while Path::new(&new_path).exists() {
        // println!("  {} dir already exists, append {}!", new_path, count);
        new_path = format!("{}{}", new_path, count);
        count += 1;
        if count > 10 {
            break;
        }
    }
    println!("crate path: {:?}", new_path);
    fs::rename(&tmp_template, &new_path).unwrap();

    if is_standalone {
        let path = Path::new(&new_path);
        let absolute_path = if path.is_absolute() {
            fs::canonicalize(path).unwrap()
        } else {
            let current_dir = env::current_dir().unwrap();
            fs::canonicalize(current_dir.join(path)).unwrap()
        };
        let anp = absolute_path.to_str().unwrap().to_string();
        let mut ctxc = ctx.clone();
        if !ctxc.projects.contains(&anp) {
            ctxc.projects
                .push(absolute_path.to_str().unwrap().to_string());
        } else {
            println!("  new project but projects already contains path");
        }
        let config_dir = dirs_next::config_dir().expect("Could not find config directory");
        let pixel_config = config_dir.join("rust_pixel.toml");
        write_config(&ctxc, &pixel_config);
        println!(
            "🍀 compile & run: \n   cd {}\n   cargo pixel r {} term\n   cargo pixel r {} sdl",
            new_path, mod_name, mod_name
        );
    } else {
        println!(
            "🍀 compile & run: \n   cargo pixel r {} term\n   cargo pixel r {} sdl",
            mod_name, mod_name
        );
    }
}

