// RustPixel
// copyright zipxing@hotmail.com 2022~2024

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
    let mod_name = args.value_of("mod_name").unwrap();
    let loname = mod_name.to_lowercase();
    let capname = capitalize(mod_name);
    let build_type = args.value_of("build_type").unwrap();
    let release = if args.is_present("release") {
        "--release"
    } else {
        ""
    };
    let webport = args.value_of("webport").unwrap_or("8080");

    match build_type {
        "term" | "t" => cmds.push(format!(
            "cargo {} -p {} --features term {} {}",
            subcmd, // build or run
            mod_name,
            release,
            args.values_of("other")
                .unwrap_or_default()
                .collect::<Vec<&str>>()
                .join(" ")
        )),
        "sdl" | "s" => cmds.push(format!(
            "cargo {} -p {} --features sdl {} {}",
            subcmd, // build or run
            mod_name,
            release,
            args.values_of("other")
                .unwrap_or_default()
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
            cmds.push(format!(
                "wasm-pack build --target web {} {} {}",
                crate_path,
                release,
                args.values_of("other")
                    .unwrap_or_default()
                    .collect::<Vec<&str>>()
                    .join(" ")
            ));
            let tmpwd = format!("tmp/web_{}/", mod_name);
            cmds.push(format!("rm -r {}/*", tmpwd));
            cmds.push(format!("mkdir -p {}", tmpwd));
            cmds.push(format!("cp -r {}/assets {}", crate_path, tmpwd));
            cmds.push(format!(
                "cp {}/web-templates/* {}",
                ctx.rust_pixel_dir[ctx.rust_pixel_idx], tmpwd
            ));
            cmds.push(format!(
                "sed -i '' \"s/Pixel/{}/g\" {}/index.js",
                capname, tmpwd
            ));
            cmds.push(format!(
                "sed -i '' \"s/pixel/{}/g\" {}/index.js",
                loname, tmpwd
            ));
            cmds.push(format!("cp -r {}/pkg {}", crate_path, tmpwd));
            if subcmd == "run" {
                cmds.push(format!("python3 -m http.server -d {} {}", tmpwd, webport));
            }
        }
        _ => {}
    }

    cmds
}

