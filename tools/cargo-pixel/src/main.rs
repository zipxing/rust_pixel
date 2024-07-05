// use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::Stdio;
// use std::path::Path;
use std::process::Command;
use std::str;
use clap::{App, Arg, ArgMatches, SubCommand};
use regex::Regex;
// use serde::Deserialize;
// use std::collections::HashMap;
use flate2::write::GzEncoder;
use flate2::Compression;

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
}

fn make_parser() -> ArgMatches {
    let matches = App::new("cargo pixel")
        .version("1.0")
        .author("zhouxin@tuyoogame.com")
        .about("RustPixel cargo build tool")
        .arg(Arg::with_name("pixel"))
        .subcommand(
            common_arg(SubCommand::with_name("run")
                .alias("r")
                .arg(Arg::with_name("mod_name").required(true))
                .arg(Arg::with_name("build_type").required(true).possible_values(&["t", "s", "w", "term", "sdl", "web"]))
                .arg(Arg::with_name("other").multiple(true))
            )
        )
        .subcommand(
            common_arg(SubCommand::with_name("build")
                .alias("b")
                .arg(Arg::with_name("mod_name").required(true))
                .arg(Arg::with_name("build_type").required(true).possible_values(&["t", "s", "w", "term", "sdl", "web"]))
            )
        )
        .subcommand(
            common_arg(SubCommand::with_name("creat")
                .alias("c")
                .arg(Arg::with_name("mod_name").required(true))
            )
        )
        .subcommand(
            common_arg(SubCommand::with_name("convert_gif")
                .alias("cg")
                .arg(Arg::with_name("gif").required(true))
                .arg(Arg::with_name("ssf").required(true))
                .arg(Arg::with_name("width").required(true))
                .arg(Arg::with_name("height").required(true))
            )
        )
        .get_matches();
    
    matches
}

fn get_cmds(args: &ArgMatches, subcmd: &str) -> Vec<String> {
    let mut cmds = Vec::new();
    // let curdir = args.value_of("dir").unwrap();
    let mod_name = args.value_of("mod_name").unwrap();
    let loname = mod_name.to_lowercase();
    let capname = capitalize(mod_name);
    let build_type = args.value_of("build_type").unwrap();
    let release = if args.is_present("release") { "--release" } else { "" };
    let webport = args.value_of("webport").unwrap_or("8080");

    match build_type {
        "term" | "t" => cmds.push(format!("cargo {} --bin {} {} {}", subcmd, mod_name, release, args.values_of("other").unwrap_or_default().collect::<Vec<&str>>().join(" "))),
        "sdl" | "s" => cmds.push(format!("cargo {} --bin {} --features sdl {} {}", subcmd, mod_name, release, args.values_of("other").unwrap_or_default().collect::<Vec<&str>>().join(" "))),
        "web" | "w" => {
            cmds.push(format!("wasm-pack build --target web games/{} {} {}", mod_name, release, args.values_of("other").unwrap_or_default().collect::<Vec<&str>>().join(" ")));
            if subcmd == "run" {
                let tmpwd = format!("tmp/web_{}/", mod_name);
                cmds.push(format!("rm -r {}/*", tmpwd));
                cmds.push(format!("mkdir -p {}", tmpwd));
                cmds.push(format!("cp -r games/{}/assets {}", mod_name, tmpwd));
                cmds.push(format!("cp rust-pixel/web-templates/* {}", tmpwd));
                cmds.push(format!("sed -i '' \"s/Pixel/{}/g\" {}/index.js", capname, tmpwd));
                cmds.push(format!("sed -i '' \"s/pixel/{}/g\" {}/index.js", loname, tmpwd));
                cmds.push(format!("cp -r games/{}/pkg {}", mod_name, tmpwd));
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

fn pixel_run(args: &ArgMatches) {
    let cmds = get_cmds(args, "run");
    for cmd in cmds {
        println!("🍀 {}", cmd);
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .expect("failed to execute process");
    }
}

fn pixel_build(args: &ArgMatches) {
    let cmds = get_cmds(args, "build");
    for cmd in cmds {
        println!("🍀 {}", cmd);
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .expect("failed to execute process");
    }
}

fn pixel_creat(args: &ArgMatches) {
    let curdir = args.value_of("dir").unwrap();
    let mod_name = args.value_of("mod_name").unwrap();
    let upname = mod_name.to_uppercase();
    let loname = mod_name.to_lowercase();
    let capname = capitalize(mod_name);

    println!("🍀 update Cargo.toml...");
    let ct = fs::read_to_string("Cargo.toml").unwrap();
    let mut doc = ct.parse::<toml::Value>().unwrap();
    if let Some(workspace) = doc.get_mut("workspace") {
        if let Some(exclude) = workspace.get_mut("exclude") {
            if let Some(exclude_array) = exclude.as_array_mut() {
                exclude_array.push(format!("games/{}/ffi", mod_name).into());
                exclude_array.push(format!("games/{}/wasm", mod_name).into());
            }
        }
    }
    fs::write("Cargo.toml", doc.to_string()).unwrap();

    println!("🍀 creat games folder...{}", format!("games/{}/", mod_name));
    let tmpdir = format!("{}/tmp", curdir);
    let _ = fs::remove_dir_all("pixel_game_template");
    let _ = fs::create_dir_all(&tmpdir);
    let _ = fs::copy("../games/template/", "./pixel_game_template");

    for entry in fs::read_dir("pixel_game_template").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let mut content = fs::read_to_string(&path).unwrap();
            content = content.replace("Template", &capname);
            content = content.replace("TEMPLATE", &upname);
            content = content.replace("template", &loname);
            fs::write(path, content).unwrap();
        }
    }
    fs::rename("pixel_game_template", format!("../games/{}", mod_name)).unwrap();

    println!("🍀 compile & run: \n   cargo pixel r {} term\n   cargo pixel r {} sdl", mod_name, mod_name);
}

fn pixel_convert_gif(args: &ArgMatches) {
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
        let cmd = format!("cargo r --bin tpetii --release tmp/t{}.png  {} {} > tmp/t{}.pix 2>/dev/null", x + 1, width, height, x + 1);
        Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .status()
            .expect("failed to execute process");
    }

    let mut fsdq = fs::File::create(ssf).unwrap();
    writeln!(fsdq, "width={},height={},texture=255,frame_count={}", width, height, frame_count).unwrap();

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
    Command::new("sh")
        .arg("-c")
        .arg("rm tmp/t*.p*")
        .status()
        .expect("failed to execute process");
}

fn main() {
    let args = make_parser();
    match args.subcommand() {
        Some(("run", sub_m)) => pixel_run(sub_m),
        Some(("build", sub_m)) => pixel_build(sub_m),
        Some(("creat", sub_m)) => pixel_creat(sub_m),
        Some(("convert_gif", sub_m)) => pixel_convert_gif(sub_m),
        _ => {}
    }
}

