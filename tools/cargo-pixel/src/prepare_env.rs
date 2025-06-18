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
use std::env;
use std::fs;
use std::process::Command;

use crate::write_config;
use crate::PixelContext;
use crate::PState;
use crate::exec_cmd;

fn is_pixel_root(dir_path: &str) -> bool {
    if let Ok(ct) = fs::read_to_string(format!("{}/Cargo.toml", dir_path)) {
        if let Ok(doc) = ct.parse::<toml::Value>() {
            if let Some(package) = doc.get("package") {
                if let Some(name) = package.get("name") {
                    return &name.to_string() == "\"rust_pixel\"";
                }
            }
        }
    }
    false
}

pub fn check_pixel_env() -> PixelContext {
    let args: Vec<String> = env::args().collect();
    let command_line = args.join(" ");
    println!("🍭 Current command line：{}", command_line);
    
    let mut pc: PixelContext = Default::default();
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    println!("🍭 Rust_pixel version：{}", current_version);

    let config_dir = dirs_next::config_dir().expect("Could not find config directory");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    }

    let pixel_config = config_dir.join("rust_pixel.toml");
    let cdir = env::current_dir().unwrap();
    let cdir_s = cdir.to_str().unwrap().to_string();
    
    if pixel_config.exists() {
        let config_content = fs::read_to_string(&pixel_config).expect("Failed to read config file");
        pc = toml::from_str(&config_content).expect("Failed to parse config file");
        println!("🍭 Loaded configuration from {:?}", pixel_config);
    } else {
        // 检查当前目录是否是 PixelRoot
        if is_pixel_root(&cdir_s) {
            println!("🍭 Current directory is rust_pixel root, using it");
            pc.rust_pixel_dir.push(cdir_s.clone());
            pc.rust_pixel_idx = 0;
            pc.cdir_state = PState::PixelRoot;
        } else {
            // 当前目录不是 PixelRoot，创建新的 rust_pixel_work
            let home_dir = dirs_next::home_dir().expect("Could not find home directory");
            let repo_dir = home_dir.join("rust_pixel_work");
            if !repo_dir.exists() {
                println!("  Cloning rust_pixel from GitHub...");
                let status = Command::new("git")
                    .args(&[
                        "clone",
                        "https://github.com/zipxing/rust_pixel",
                        repo_dir.to_str().unwrap(),
                    ])
                    .status()
                    .expect("Failed to execute git command");
                if status.success() {
                    println!("  Repository cloned successfully.");
                } else {
                    println!("🚫 Failed to clone rust_pixel repository");
                }
            } else {
                println!("🍭 Using existing rust_pixel_work directory");
            }
            pc.rust_pixel_dir.push(repo_dir.to_str().unwrap().to_string());
            pc.rust_pixel_idx = 0;
        }
        write_config(&pc, &pixel_config);
    }

    // 检查当前目录状态
    pc.cdir_state = PState::NotPixel;
    if let Some(idx) = pc.rust_pixel_dir.iter().position(|x| x == &cdir_s) {
        pc.cdir_state = PState::PixelRoot;
        pc.rust_pixel_idx = idx;
    } else {
        if let Some(pidx) = pc.projects.iter().position(|x| x == &cdir_s) {
            pc.cdir_state = PState::PixelProject;
            pc.project_idx = pidx;
        }
    }

    // 检查版本并更新
    if let Ok(ct) = fs::read_to_string("Cargo.toml") {
        let doc = ct.parse::<toml::Value>().unwrap();
        if let Some(package) = doc.get("package") {
            if let Some(name) = package.get("name") {
                if &name.to_string() == "\"rust_pixel\"" {
                    if pc.cdir_state == PState::NotPixel {
                        println!("🍭 Found a new pixel root:{:?}", cdir_s);
                        pc.cdir_state = PState::PixelRoot;
                        pc.rust_pixel_dir.push(cdir_s);
                        pc.rust_pixel_idx = pc.rust_pixel_dir.len() - 1;
                        write_config(&pc, &pixel_config);
                    }
                    if let Some(new_version) = package.get("version") {
                        let nvs = new_version.to_string();
                        let cvs = format!("\"{}\"", current_version);
                        if nvs != cvs {
                            exec_cmd("cargo install --path . --force");
                            println!("new ver:{:?} ver:{:?}", nvs, cvs);
                            println!("🍭 Updated cargo-pixel by: cargo install --path . --force");
                            println!("🍭 Re-run new version cargo-pixel");
                            exec_cmd(&command_line);
                            std::process::exit(0);
                        }
                    }
                } else {
                    if pc.cdir_state == PState::NotPixel {
                        if let Some(dep) = doc.get("dependencies") {
                            if let Some(_drp) = dep.get("rust_pixel") {
                                println!("🍭 Found a new pixel project:{:?}", cdir_s);
                                pc.cdir_state = PState::PixelProject;
                                pc.projects.push(cdir_s);
                                pc.project_idx = pc.projects.len() - 1;
                                write_config(&pc, &pixel_config);
                            }
                        }
                    }
                }
            }
        }
    }
    pc
}

