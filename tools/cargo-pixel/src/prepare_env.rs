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

fn is_pixel_project(dir_path: &str) -> bool {
    if let Ok(ct) = fs::read_to_string(format!("{}/Cargo.toml", dir_path)) {
        if let Ok(doc) = ct.parse::<toml::Value>() {
            if let Some(dep) = doc.get("dependencies") {
                if dep.get("rust_pixel").is_some() {
                    return true;
                }
            }
        }
    }
    false
}

fn can_write_to_dir(dir: &std::path::Path) -> bool {
    let test_file = dir.join(".rust_pixel_write_test");
    match fs::write(&test_file, "test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

fn decide_rust_pixel_location(current_dir: &std::path::Path) -> std::path::PathBuf {
    let current_dir_str = current_dir.to_str().unwrap();
    
    // Check if current directory is a rust_pixel project
    if is_pixel_project(current_dir_str) {
        println!("🍭 Detected rust_pixel project in current directory");
        
        // Try to create rust_pixel in parent directory
        if let Some(parent_dir) = current_dir.parent() {
            let parent_rust_pixel = parent_dir.join("rust_pixel");
            if can_write_to_dir(parent_dir) && !parent_rust_pixel.exists() {
                println!("  Will create rust_pixel in parent directory");
                return parent_rust_pixel;
            }
        }
        
        // Parent directory not writable or already exists, use home directory
        let home_dir = dirs_next::home_dir().expect("Could not find home directory");
        let home_rust_pixel = home_dir.join("rust_pixel_work");
        println!("  Will use home directory for rust_pixel");
        return home_rust_pixel;
    } else {
        // Regular directory, check if writable
        if can_write_to_dir(current_dir) {
            println!("  Will create rust_pixel in current directory");
            return current_dir.join("rust_pixel");
        } else {
            // Current directory not writable, use home directory
            let home_dir = dirs_next::home_dir().expect("Could not find home directory");
            let home_rust_pixel = home_dir.join("rust_pixel_work");
            println!("  Current directory not writable, will use home directory");
            return home_rust_pixel;
        }
    }
}

fn create_rust_pixel_repo(repo_dir: &std::path::Path) {
    println!("  Cloning rust_pixel to {:?}...", repo_dir);
    
    let parent_dir = repo_dir.parent().unwrap();
    let repo_name = repo_dir.file_name().unwrap().to_str().unwrap();
    
    let status = Command::new("git")
        .current_dir(parent_dir)
        .args([
            "clone",
            "https://github.com/zipxing/rust_pixel",
            repo_name,
        ])
        .status()
        .expect("Failed to execute git command");
        
    if status.success() {
        println!("  Repository cloned successfully to {:?}", repo_dir);
    } else {
        println!("🚫 Failed to clone rust_pixel repository");
        std::process::exit(1);
    }
}

pub fn check_pixel_env() -> PixelContext {
    let args: Vec<String> = env::args().collect();
    let command_line = args.join(" ");
    println!("🍭 Current command line：{}", command_line);
    
    let mut pc: PixelContext = Default::default();
    // compile into cargo-pixel binary file...
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
        // Check if current directory is PixelRoot
        if is_pixel_root(&cdir_s) {
            println!("🍭 Current directory is rust_pixel root, using it");
            pc.rust_pixel_dir.push(cdir_s.clone());
            pc.rust_pixel_idx = 0;
            pc.cdir_state = PState::PixelRoot;
        } else {
            // Current directory is not PixelRoot, need to decide where to create rust_pixel
            let repo_dir = decide_rust_pixel_location(&cdir);
            
            if !repo_dir.exists() {
                create_rust_pixel_repo(&repo_dir);
            } else {
                println!("🍭 Using existing rust_pixel directory at {:?}", repo_dir);
            }
            pc.rust_pixel_dir.push(repo_dir.to_str().unwrap().to_string());
            pc.rust_pixel_idx = 0;
        }
        write_config(&pc, &pixel_config);
    }

    // Check current directory status
    pc.cdir_state = PState::NotPixel;
    if let Some(idx) = pc.rust_pixel_dir.iter().position(|x| x == &cdir_s) {
        pc.cdir_state = PState::PixelRoot;
        pc.rust_pixel_idx = idx;
    } else if let Some(pidx) = pc.projects.iter().position(|x| x == &cdir_s) {
        pc.cdir_state = PState::PixelProject;
        pc.project_idx = pidx;
    }

    // Check version and update
    if let Ok(ct) = fs::read_to_string("Cargo.toml") {
        
        match ct.parse::<toml::Value>() {
            Ok(doc) => {
                // Process the TOML document
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
                                    // Update cargo-pixel using direct Command execution
                                    println!("🍭 Updating cargo-pixel...");
                                    let status = Command::new("cargo")
                                        .args(["install", "--path", ".", "--force"])
                                        .status()
                                        .expect("Failed to execute cargo install");
                                    
                                    if status.success() {
                                        println!("new ver:{:?} ver:{:?}", nvs, cvs);
                                        println!("🍭 Updated cargo-pixel by: cargo install --path . --force");
                                        println!("🍭 Re-run new version cargo-pixel");
                                        
                                        // Re-execute with the same command line
                                        exec_cmd(&command_line);
                                        std::process::exit(0);
                                    } else {
                                        eprintln!("❌ Failed to update cargo-pixel");
                                    }
                                }
                            }
                        } else if pc.cdir_state == PState::NotPixel {
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
            Err(e) => {
                println!("❌ TOML parsing failed: {}", e);
                println!("❌ Error details: {:?}", e);
                // Skip TOML processing but continue with function
                println!("⚠️ Skipping version check due to TOML parsing error");
            }
        }
    }
    pc
}

