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
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::str;

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum PState {
    NotPixel,
    PixelRoot,
    PixelProject,
}

impl Default for PState {
    fn default() -> Self {
        PState::NotPixel
    }
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
    dirname: &str,
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
                    dirname,
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

fn main() {
    let ctx = check_pixel_env();
    // println!("{:?}", ctx);
    let args = make_parser();
    match args.subcommand() {
        Some(("run", sub_m)) => pixel_run(&ctx, sub_m),
        Some(("build", sub_m)) => pixel_build(&ctx, sub_m),
        Some(("creat", sub_m)) => pixel_creat(&ctx, sub_m),
        Some(("convert_gif", sub_m)) => pixel_convert_gif(&ctx, sub_m),
        _ => {}
    }
}
