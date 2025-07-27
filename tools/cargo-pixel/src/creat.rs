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
use crate::exec_cmd;
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

    let _ = fs::remove_dir_all("tmp/pixel_game_template");
    let _ = fs::create_dir_all(cdir);
    exec_cmd("mkdir tmp");
    exec_cmd("cp -r apps/template tmp/pixel_game_template");

    if let Some(stand_dir) = sa_dir {
        is_standalone = true;
        exec_cmd("cp apps/template/stand-alone/Cargo.toml.temp tmp/pixel_game_template/Cargo.toml");
        exec_cmd("cp apps/template/stand-alone/LibCargo.toml.temp tmp/pixel_game_template/lib/Cargo.toml");
        exec_cmd("cp apps/template/stand-alone/FfiCargo.toml.temp tmp/pixel_game_template/ffi/Cargo.toml");
        exec_cmd("cp apps/template/stand-alone/WasmCargo.toml.temp tmp/pixel_game_template/wasm/Cargo.toml");
        dir_name = stand_dir.to_string();
    }
    exec_cmd("rm -fr tmp/pixel_game_template/stand-alone");

    replace_in_files(
        is_standalone,
        Path::new("tmp/pixel_game_template"),
        &ctx.rust_pixel_dir[ctx.rust_pixel_idx],
        &dir_name,
        &capname,
        &upname,
        &loname,
    );

    let mut new_path;
    new_path = format!("{}/{}", dir_name, mod_name);
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
    fs::rename("tmp/pixel_game_template", &new_path).unwrap();

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

