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
use clap::{App, Arg, ArgMatches, SubCommand};
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::str;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PixelContext {
    // rust_pixel repo local path
    pub rust_pixel_dir: String,
    // standalone projects
    pub standalone: Vec<String>,
}

fn common_arg(app: App) -> App {
    app.arg(
        Arg::with_name("release")
            .short('r')
            .long("release")
            .takes_value(false),
    )
    .arg(
        Arg::with_name("webport")
            .short('p')
            .long("webport")
            .default_value("8080")
            .takes_value(true),
    )
}

fn make_parser() -> ArgMatches {
    let matches = App::new("cargo pixel")
        .author("zipxing@hotmail.com")
        .about("RustPixel cargo build tool")
        .arg(Arg::with_name("pixel"))
        .subcommand(common_arg(
            SubCommand::with_name("run")
                .alias("r")
                .arg(Arg::with_name("mod_name").required(true))
                .arg(
                    Arg::with_name("build_type")
                        .required(true)
                        .possible_values(&["t", "s", "w", "term", "sdl", "web"]),
                )
                .arg(Arg::with_name("other").multiple(true)),
        ))
        .subcommand(common_arg(
            SubCommand::with_name("build")
                .alias("b")
                .arg(Arg::with_name("mod_name").required(true))
                .arg(
                    Arg::with_name("build_type")
                        .required(true)
                        .possible_values(&["t", "s", "w", "term", "sdl", "web"]),
                ),
        ))
        .subcommand(common_arg(
            SubCommand::with_name("creat")
                .alias("c")
                .arg(Arg::with_name("mod_name").required(true))
                .arg(Arg::with_name("standalone_dir_name").required(false)),
        ))
        .subcommand(common_arg(
            SubCommand::with_name("convert_gif")
                .alias("cg")
                .arg(Arg::with_name("gif").required(true))
                .arg(Arg::with_name("ssf").required(true))
                .arg(Arg::with_name("width").required(true))
                .arg(Arg::with_name("height").required(true)),
        ))
        .get_matches();

    matches
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
            if ctx.standalone.contains(&mod_name.to_string()) {
                crate_path = ".".to_string();
            } else {
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
                ctx.rust_pixel_dir, tmpwd
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

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn exec_cmd(cmd: &str) {
    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .expect("failed to execute process");
}

fn pixel_run(ctx: &PixelContext, args: &ArgMatches) {
    let cmds = get_cmds(ctx, args, "run");
    for cmd in cmds {
        println!("🍀 {}", cmd);
        exec_cmd(&cmd);
    }
}

fn pixel_build(ctx: &PixelContext, args: &ArgMatches) {
    let cmds = get_cmds(ctx, args, "build");
    for cmd in cmds {
        println!("🍀 {}", cmd);
        exec_cmd(&cmd);
    }
}

fn pixel_creat(ctx: &PixelContext, args: &ArgMatches) {
    // if ctx.standalone {
    //     println!("Cargo pixel creat must run in rust_pixel root directory.");
    //     return;
    // }
    let mut dir_name = "apps".to_string();
    let sa_dir = args.value_of("standalone_dir_name");
    let mod_name = args.value_of("mod_name").unwrap();
    let mut is_standalone = false;
    let upname = mod_name.to_uppercase();
    let loname = mod_name.to_lowercase();
    let capname = capitalize(mod_name);

    println!(
        "🍀 creat games folder...({})",
        format!("{}/{}/", dir_name, mod_name)
    );
    let cdir = format!("{}", dir_name);
    let _ = fs::remove_dir_all("tmp/pixel_game_template");
    let _ = fs::create_dir_all(cdir);

    exec_cmd("cp -r apps/template tmp/pixel_game_template");

    if let Some(stand_dir) = sa_dir {
        is_standalone = true;
        exec_cmd("cp apps/template/stand-alone/Cargo.toml.temp tmp/pixel_game_template/Cargo.toml");
        exec_cmd("cp apps/template/stand-alone/LibCargo.toml.temp tmp/pixel_game_template/lib/Cargo.toml");
        exec_cmd("cp apps/template/stand-alone/FfiCargo.toml.temp tmp/pixel_game_template/ffi/Cargo.toml");
        exec_cmd("cp apps/template/stand-alone/WasmCargo.toml.temp tmp/pixel_game_template/wasm/Cargo.toml");
        exec_cmd("cp apps/template/stand-alone/pixel.toml.temp tmp/pixel_game_template/pixel.toml");
        dir_name = format!("{}", stand_dir);
    }
    exec_cmd("rm -fr tmp/pixel_game_template/stand-alone");

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
                        if !is_standalone {
                            if dirname == "games" {
                            } else {
                                content_str = content_str.replace(
                                    "pixel_game!(Template)",
                                    &format!("pixel_game!(Template, \"{}\")", dirname),
                                );
                            }
                        } else {
                            content_str = content_str.replace("$RUST_PIXEL_ROOT", rust_pixel_path);
                            content_str = content_str.replace(
                                "pixel_game!(Template)",
                                &format!("pixel_game!(Template, \"app\", \".\")"),
                            );
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
    replace_in_files(
        is_standalone,
        Path::new("tmp/pixel_game_template"),
        &ctx.rust_pixel_dir,
        &dir_name,
        &capname,
        &upname,
        &loname,
    );

    println!("{:?}", format!("./{}/{}", dir_name, mod_name));
    fs::rename(
        "tmp/pixel_game_template",
        format!("{}/{}", dir_name, mod_name),
    )
    .unwrap();

    if is_standalone {
        println!(
            "🍀 compile & run: \n   cd {}/{}\n   cargo pixel r {} term\n   cargo pixel r {} sdl",
            dir_name, mod_name, mod_name, mod_name
        );
    } else {
        println!(
            "🍀 compile & run: \n   cargo pixel r {} term\n   cargo pixel r {} sdl",
            mod_name, mod_name
        );
    }
}

fn pixel_convert_gif(_ctx: &PixelContext, args: &ArgMatches) {
    let gif = args.value_of("gif").unwrap();
    let ssf = args.value_of("ssf").unwrap();
    let width: usize = args.value_of("width").unwrap().parse().unwrap();
    let height: usize = args.value_of("height").unwrap().parse().unwrap();

    println!("🍀 extract pngs use ffmpeg...");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("ffmpeg -i {} -vsync 0 tmp/t%d.png", gif))
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute process");

    let stderr = str::from_utf8(&output.stderr).unwrap();
    let rf = Regex::new(r"(.*)frame=(.*?)(\d+)(.*)").unwrap();
    let fg = rf.captures(stderr).unwrap();
    let frame_count: usize = fg.get(3).unwrap().as_str().parse().unwrap();
    println!("    frame_count = {}", frame_count);

    println!("🍀 pixel_petii convert png to pix...");
    for x in 0..frame_count {
        print!("\r{}  ", x + 1);
        io::stdout().flush().unwrap();
        let cmd = format!(
            "cargo r --bin pixel_petii --release tmp/t{}.png  {} {} > tmp/t{}.pix 2>/dev/null",
            x + 1,
            width,
            height,
            x + 1
        );
        exec_cmd(&cmd);
    }

    let mut fsdq = fs::File::create(ssf).unwrap();
    writeln!(
        fsdq,
        "width={},height={},texture=255,frame_count={}",
        width, height, frame_count
    )
    .unwrap();

    let rds = Regex::new(r"(\d+),(\d+),(\d+)(.*?)").unwrap();
    let mut datas = Vec::new();
    let mut flens = Vec::new();
    for x in 0..frame_count {
        let pix_file = format!("tmp/t{}.pix", x + 1);
        let content = fs::read_to_string(pix_file).unwrap();
        let mut sdatas = Vec::new();
        for line in content.lines().skip(1) {
            for cap in rds.captures_iter(line) {
                sdatas.push(cap[1].parse::<u8>().unwrap());
                sdatas.push(cap[2].parse::<u8>().unwrap());
                sdatas.push(cap[3].parse::<u8>().unwrap());
            }
        }
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&sdatas).unwrap();
        let compressed = encoder.finish().unwrap();
        flens.push(compressed.len());
        datas.extend_from_slice(&compressed);
    }

    for len in flens {
        write!(fsdq, "{},", len).unwrap();
    }
    writeln!(fsdq).unwrap();
    fsdq.write_all(&datas).unwrap();

    println!("\n🍀 {} write ok!", ssf);
    exec_cmd("rm tmp/t*.p*");
}

fn check_pixel_env() -> PixelContext {
    let mut pc: PixelContext = Default::default();

    // match env::current_dir() {
    //     Ok(current_dir) => {
    //         pc.current_dir = current_dir.clone();
    //         println!("🍭 current_dir：{}", current_dir.display());
    //     }
    //     Err(e) => {
    //         println!("get current_dir error：{}", e);
    //     }
    // }

    // match env::current_exe() {
    //     Ok(exe_path) => {
    //         pc.current_exe = exe_path.clone();
    //         println!("🍭 current_exe：{}", exe_path.display());
    //     }
    //     Err(e) => {
    //         println!("get current_exe error：{}", e);
    //     }
    // }

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    println!("🍭 rust_pixel version：{}", current_version);

    let config_dir = dirs_next::config_dir().expect("Could not find config directory");
    let pixel_config = config_dir.join("rust_pixel.toml");
    println!("🍭 config_dir：{:?}", config_dir);

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    }

    if pixel_config.exists() {
        let config_content = fs::read_to_string(&pixel_config).expect("Failed to read config file");
        let saved_pc: PixelContext =
            toml::from_str(&config_content).expect("Failed to parse config file");
        pc.rust_pixel_dir = saved_pc.rust_pixel_dir;
        pc.standalone = saved_pc.standalone;
        println!("🍭 Loaded configuration from rust_pixel.toml");
    } else {
        let home_dir = dirs_next::home_dir().expect("Could not find home directory");
        let repo_dir = home_dir.join("rust_pixel");
        if !repo_dir.exists() {
            println!("Cloning rust_pixel from GitHub...");
            let status = Command::new("git")
                .args(&[
                    "clone",
                    // "-b", "opt_crate",
                    "https://github.com/zipxing/rust_pixel",
                    repo_dir.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to execute git command");
            if status.success() {
                println!("Repository cloned successfully.");
                pc.rust_pixel_dir = repo_dir.to_str().unwrap().to_string();
                write_config(&pc, &pixel_config);
            } else {
                eprintln!("Failed to clone rust_pixel repository");
            }
        }
    }

    // match env::set_current_dir(&repo_dir) {
    //     Ok(_) => {
    //         println!("Successfully changed to directory: {}", repo_dir.display());
    //         println!("Updating rust_pixel from GitHub...");
    //         let status = Command::new("git")
    //             .args(&["pull"])
    //             .status()
    //             .expect("Failed to execute git command");
    //         if status.success() {
    //             println!("Repository update successfully.");
    //         } else {
    //             eprintln!("Failed to update rust_pixel repository");
    //         }
    //     }
    //     Err(e) => eprintln!("Error changing directory: {}", e),
    // }

    let ct = fs::read_to_string("pixel.toml").expect("Can't find pixel.toml!");
    let doc = ct.parse::<toml::Value>().unwrap();

    if let Some(package) = doc.get("package") {
        if let Some(new_version) = package.get("version") {
            let nvs = new_version.to_string();
            let cvs = format!("\"{}\"", current_version);
            eprintln!("new ver:{:?} ver:{:?}", nvs, cvs);
            if nvs != cvs {
                eprintln!("Please update cargo pixel: cargo install rust_pixel --force");
                std::process::exit(0);
            }
        }
    }

    // if !pc.standalone {
    //     let srcdir = PathBuf::from(&pc.rust_pixel_dir);
    //     let rpp = format!("{:?}", fs::canonicalize(&srcdir).unwrap());
    //     pc.rust_pixel_dir = rpp[1..rpp.len() - 1].to_string();
    // }
    pc
}

fn write_config(pc: &PixelContext, config_path: &Path) {
    let toml_string = toml::to_string(pc).expect("Failed to serialize PixelContext");

    let mut file = File::create(config_path).expect("Failed to create config file");
    file.write_all(toml_string.as_bytes())
        .expect("Failed to write to config file");
    println!("🍭 Configuration saved to {}", config_path.display());
}

fn main() {
    let ctx = check_pixel_env();
    let args = make_parser();
    match args.subcommand() {
        Some(("run", sub_m)) => pixel_run(&ctx, sub_m),
        Some(("build", sub_m)) => pixel_build(&ctx, sub_m),
        Some(("creat", sub_m)) => pixel_creat(&ctx, sub_m),
        Some(("convert_gif", sub_m)) => pixel_convert_gif(&ctx, sub_m),
        _ => {}
    }
}
