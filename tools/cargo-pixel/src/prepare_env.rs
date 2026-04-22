// RustPixel
// copyright zipxing@hotmail.com 2022～2026

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
use std::io::{self, Write};
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

/// Get the default rust_pixel repo location.
/// Priority: RUST_PIXEL_HOME env var > ~/rust_pixel
fn default_repo_dir() -> std::path::PathBuf {
    if let Ok(custom) = env::var("RUST_PIXEL_HOME") {
        std::path::PathBuf::from(custom)
    } else {
        let home = dirs_next::home_dir().expect("Could not find home directory");
        home.join("rust_pixel")
    }
}

/// Clone the rust_pixel repo to the given directory, with user confirmation.
/// Returns true if the repo is ready (already existed or successfully cloned).
fn ensure_repo(repo_dir: &std::path::Path) -> bool {
    if repo_dir.exists() && is_pixel_root(&repo_dir.to_string_lossy()) {
        return true;
    }

    println!();
    println!("🎮 Welcome to RustPixel!");
    println!();
    println!("  RustPixel needs to download the framework (includes demos & templates).");
    println!("  Location: {}", repo_dir.display());
    println!();
    println!("  To customize, set RUST_PIXEL_HOME environment variable.");
    println!();
    print!("  Proceed? [Y/n] ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim().to_lowercase();
    if input == "n" || input == "no" {
        println!("  Aborted.");
        return false;
    }

    println!("  Cloning rust_pixel to {}...", repo_dir.display());

    let parent = repo_dir.parent().unwrap();
    let dir_name = repo_dir.file_name().unwrap().to_str().unwrap();

    if !parent.exists() {
        fs::create_dir_all(parent).expect("Failed to create parent directory");
    }

    let status = Command::new("git")
        .current_dir(parent)
        .args(["clone", "https://github.com/zipxing/rust_pixel", dir_name])
        .status()
        .expect("Failed to execute git command");

    if status.success() {
        println!("  ✅ Repository cloned successfully to {}", repo_dir.display());
        true
    } else {
        eprintln!("  ❌ Failed to clone rust_pixel repository");
        false
    }
}

/// Lightweight env check that does NOT trigger clone.
/// Used for help, version, and other commands that don't need the repo.
pub fn check_pixel_env_light() -> PixelContext {
    let mut pc: PixelContext = Default::default();
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    println!("🍭 Rust_pixel version: {}", current_version);

    let cdir = env::current_dir().unwrap();
    let cdir_s = cdir.to_str().unwrap().to_string();

    let config_dir = dirs_next::config_dir().expect("Could not find config directory");
    let pixel_config = config_dir.join("rust_pixel.toml");

    if pixel_config.exists() {
        if let Ok(config_content) = fs::read_to_string(&pixel_config) {
            if let Ok(loaded) = toml::from_str::<PixelContext>(&config_content) {
                pc = loaded;
            }
        }
    }

    // Detect current directory state
    pc.cdir_state = PState::NotPixel;
    if is_pixel_root(&cdir_s) {
        pc.cdir_state = PState::PixelRoot;
        // Register if not already known
        if !pc.rust_pixel_dir.contains(&cdir_s) {
            pc.rust_pixel_dir.push(cdir_s.clone());
        }
        pc.rust_pixel_idx = pc.rust_pixel_dir.iter().position(|x| x == &cdir_s).unwrap();
    } else if is_pixel_project(&cdir_s) {
        pc.cdir_state = PState::PixelProject;
        if !pc.projects.contains(&cdir_s) {
            pc.projects.push(cdir_s.clone());
        }
        pc.project_idx = pc.projects.iter().position(|x| x == &cdir_s).unwrap();
    } else if let Some(idx) = pc.rust_pixel_dir.iter().position(|x| x == &cdir_s) {
        pc.cdir_state = PState::PixelRoot;
        pc.rust_pixel_idx = idx;
    } else if let Some(pidx) = pc.projects.iter().position(|x| x == &cdir_s) {
        pc.cdir_state = PState::PixelProject;
        pc.project_idx = pidx;
    }

    pc
}

/// Full env check: ensures the repo exists (cloning if needed).
/// Used for commands that actually need the repo (run, build, creat, etc.).
pub fn check_pixel_env() -> PixelContext {
    let args: Vec<String> = env::args().collect();
    let command_line = args.join(" ");
    println!("🍭 Current command line: {}", command_line);

    let mut pc = check_pixel_env_light();
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let config_dir = dirs_next::config_dir().expect("Could not find config directory");
    let pixel_config = config_dir.join("rust_pixel.toml");

    // If we're already in a pixel root or project, no need to clone
    if pc.cdir_state == PState::PixelRoot || pc.cdir_state == PState::PixelProject {
        // Check version and auto-update if in pixel root
        if pc.cdir_state == PState::PixelRoot {
            check_version_update(&current_version, &command_line);
        }
        write_config(&pc, &pixel_config);
        return pc;
    }

    // Not in a pixel directory — ensure the default repo exists
    let repo_dir = default_repo_dir();
    if !ensure_repo(&repo_dir) {
        // User declined or clone failed — return context as-is
        return pc;
    }

    let repo_dir_s = repo_dir.to_str().unwrap().to_string();
    if !pc.rust_pixel_dir.contains(&repo_dir_s) {
        pc.rust_pixel_dir.push(repo_dir_s.clone());
    }
    pc.rust_pixel_idx = pc.rust_pixel_dir.iter().position(|x| x == &repo_dir_s).unwrap();

    write_config(&pc, &pixel_config);
    pc
}

/// Check if the installed cargo-pixel version matches the repo version,
/// and auto-update if they differ.
fn check_version_update(current_version: &str, command_line: &str) {
    if let Ok(ct) = fs::read_to_string("Cargo.toml") {
        if let Ok(doc) = ct.parse::<toml::Value>() {
            if let Some(package) = doc.get("package") {
                if let Some(name) = package.get("name") {
                    if &name.to_string() != "\"rust_pixel\"" {
                        return;
                    }
                    if let Some(new_version) = package.get("version") {
                        let nvs = new_version.to_string();
                        let cvs = format!("\"{}\"", current_version);
                        if nvs != cvs {
                            println!("🍭 Updating cargo-pixel...");
                            let status = Command::new("cargo")
                                .args(["install", "--path", ".", "--force"])
                                .status()
                                .expect("Failed to execute cargo install");

                            if status.success() {
                                println!("new ver:{:?} ver:{:?}", nvs, cvs);
                                println!("🍭 Updated cargo-pixel by: cargo install --path . --force");
                                println!("🍭 Re-run new version cargo-pixel");
                                exec_cmd(command_line);
                                std::process::exit(0);
                            } else {
                                eprintln!("❌ Failed to update cargo-pixel");
                            }
                        }
                    }
                }
            }
        }
    }
}
