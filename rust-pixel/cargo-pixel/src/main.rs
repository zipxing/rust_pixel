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
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::str;

fn common_arg(app: App) -> App {
    app.arg(
        Arg::with_name("dir")
            .short('d')
            .long("dir")
            .default_value(".")
            .takes_value(true),
    )
    .arg(
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
    .arg(
        Arg::with_name("standalone")
            .short('s')
            .long("standalone")
            .takes_value(false),
    )
}

fn make_parser() -> ArgMatches {
    let matches = App::new("cargo pixel")
        .version("0.5.1")
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
                .arg(Arg::with_name("dir_name").required(true))
                .arg(Arg::with_name("mod_name").required(true)),
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
            "cargo {} --bin {} --features term {} {}",
            subcmd, // build or run
            mod_name,
            release,
            args.values_of("other")
                .unwrap_or_default()
                .collect::<Vec<&str>>()
                .join(" ")
        )),
        "sdl" | "s" => cmds.push(format!(
            "cargo {} --bin {} --features sdl {} {}",
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
            if ctx.standalone {
                crate_path = ".".to_string();
            } else {
                let cpath = format!("games/{}", mod_name);
                if Path::new(&cpath).exists() {
                    crate_path = cpath;
                }
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
                "cp {}/rust-pixel/web-templates/* {}",
                ctx.rust_pixel_path, tmpwd
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
    if ctx.standalone {
        println!("Cargo pixel creat must run in rust_pixel root directory.");
        return;
    }
    let dir_name = args.value_of("dir_name").unwrap();
    let mod_name = args.value_of("mod_name").unwrap();
    let is_standalone = args.is_present("standalone");
    let upname = mod_name.to_uppercase();
    let loname = mod_name.to_lowercase();
    let capname = capitalize(mod_name);

    if !is_standalone {
        println!("🍀 update Cargo.toml...");
        let ct = fs::read_to_string("Cargo.toml").unwrap();
        let mut doc = ct.parse::<toml::Value>().unwrap();
        if let Some(workspace) = doc.get_mut("workspace") {
            if let Some(members) = workspace.get_mut("members") {
                if let Some(members_array) = members.as_array_mut() {
                    let ds = format!("{}/*", dir_name);
                    if !members_array.contains(&ds.clone().into()) {
                        members_array.push(ds.into());
                    }
                }
            }
            if let Some(exclude) = workspace.get_mut("exclude") {
                if let Some(exclude_array) = exclude.as_array_mut() {
                    exclude_array.push(format!("{}/{}/ffi", dir_name, mod_name).into());
                    exclude_array.push(format!("{}/{}/wasm", dir_name, mod_name).into());
                }
            }
        }
        fs::write("Cargo.toml", toml::to_string_pretty(&doc).unwrap()).unwrap();
    }

    println!(
        "🍀 creat games folder...({})",
        format!("{}/{}/", dir_name, mod_name)
    );
    let cdir = format!("{}", dir_name);
    let _ = fs::remove_dir_all("tmp/pixel_game_template");
    let _ = fs::create_dir_all(cdir);

    exec_cmd("cp -r games/template tmp/pixel_game_template");

    if is_standalone {
        exec_cmd(
            "cp games/template/stand-alone/Cargo.toml.temp tmp/pixel_game_template/Cargo.toml",
        );
        exec_cmd("cp games/template/stand-alone/LibCargo.toml.temp tmp/pixel_game_template/lib/Cargo.toml");
        exec_cmd("cp games/template/stand-alone/FfiCargo.toml.temp tmp/pixel_game_template/ffi/Cargo.toml");
        exec_cmd("cp games/template/stand-alone/WasmCargo.toml.temp tmp/pixel_game_template/wasm/Cargo.toml");
        exec_cmd(
            "cp games/template/stand-alone/pixel.toml.temp tmp/pixel_game_template/pixel.toml",
        );
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
        &ctx.rust_pixel_path,
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

    println!("🍀 tpetii convert png to pix...");
    for x in 0..frame_count {
        print!("\r{}  ", x + 1);
        io::stdout().flush().unwrap();
        let cmd = format!(
            "cargo r --bin tpetii --release tmp/t{}.png  {} {} > tmp/t{}.pix 2>/dev/null",
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

#[derive(Debug)]
struct PixelContext {
    standalone: bool,
    rust_pixel_path: String,
}

fn check_pixel_toml() -> PixelContext {
    let ct = fs::read_to_string("pixel.toml")
        .expect("Can't find pixel.toml!\ncargo-pixel must run in rust_pixel or standalone_rust_pixel_project directory.\npixel.toml ");
    let doc = ct.parse::<toml::Value>().unwrap();
    let mut pc = PixelContext {
        standalone: false,
        rust_pixel_path: "./".to_string(),
    };
    if let Some(pixel) = doc.get("pixel") {
        if let Some(standalone) = pixel.get("standalone") {
            pc.standalone = standalone.as_bool().unwrap();
        }
        if let Some(rust_pixel) = pixel.get("rust_pixel") {
            let rpp = rust_pixel.to_string();
            pc.rust_pixel_path = rpp[1..rpp.len() - 1].to_string();
        }
        if let Some(cargo_pixel) = pixel.get("cargo_pixel") {
            let cps = cargo_pixel.to_string();
            if cps != "\"0.5.1\"" {
                panic!("Please update cargo pixel: cargo install --path tools/cargo-pixel --root ~/.cargo");
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

fn main() {
    let ctx = check_pixel_toml();
    let args = make_parser();
    match args.subcommand() {
        Some(("run", sub_m)) => pixel_run(&ctx, sub_m),
        Some(("build", sub_m)) => pixel_build(&ctx, sub_m),
        Some(("creat", sub_m)) => pixel_creat(&ctx, sub_m),
        Some(("convert_gif", sub_m)) => pixel_convert_gif(&ctx, sub_m),
        _ => {}
    }
}
