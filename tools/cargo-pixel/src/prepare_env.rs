use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct PixelContext {
    pub standalone: bool,
    pub rust_pixel_path: String,
}

fn get_home_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        // windows
        env::var("USERPROFILE").map(PathBuf::from).ok()
    } else {
        env::var("HOME").map(PathBuf::from).ok()
    }
}

pub fn check_pixel_env() -> PixelContext {
    match env::current_dir() {
        Ok(current_dir) => {
            println!("🍭 current_dir：{}", current_dir.display());
        }
        Err(e) => {
            println!("get current_dir error：{}", e);
        }
    }

    match env::current_exe() {
        Ok(exe_path) => {
            println!("🍭 current_exe：{}", exe_path.display());
        }
        Err(e) => {
            println!("get current_exe error：{}", e);
        }
    }

    let version = env!("CARGO_PKG_VERSION");
    println!("🍭 rust_pixel version：{}", version);

    let home_dir = get_home_dir().expect("Could not find home directory");
    let repo_dir = home_dir.join("rust_pixel.workspace");

    if !repo_dir.exists() {
        println!("Cloning rust_pixel from GitHub...");
        let status = Command::new("git")
            .args(&[
                "clone",
                "-b", "opt_crate",
                "https://github.com/zipxing/rust_pixel",
                repo_dir.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to execute git command");
        if status.success() {
            println!("Repository cloned successfully.");
        } else {
            eprintln!("Failed to clone rust_pixel repository");
        }
    }

    match env::set_current_dir(&repo_dir) {
        Ok(_) => {
            println!("Successfully changed to directory: {}", repo_dir.display());
            println!("Updating rust_pixel from GitHub...");
            let status = Command::new("git")
                .args(&["pull"])
                .status()
                .expect("Failed to execute git command");
            if status.success() {
                println!("Repository update successfully.");
            } else {
                eprintln!("Failed to update rust_pixel repository");
            }
        }
        Err(e) => eprintln!("Error changing directory: {}", e),
    }

    let ct = fs::read_to_string("Cargo.toml").expect("Can't find Cargo.toml!");
    let doc = ct.parse::<toml::Value>().unwrap();

    let mut pc = PixelContext {
        standalone: false,
        rust_pixel_path: "./".to_string(),
    };

    if let Some(package) = doc.get("package") {
        if let Some(new_version) = package.get("version") {
            let nvs = new_version.to_string();
            let cvs = format!("\"{}\"", version);
            eprintln!("new ver:{:?} ver:{:?}", nvs, cvs);
            if nvs != cvs {
                eprintln!("Please update cargo pixel: cargo install rust_pixel --force");
                std::process::exit(0);
            }
        }
    }

    if !pc.standalone {
        let srcdir = PathBuf::from(&pc.rust_pixel_path);
        let rpp = format!("{:?}", fs::canonicalize(&srcdir).unwrap());
        pc.rust_pixel_path = rpp[1..rpp.len() - 1].to_string();
    }
    pc
}
