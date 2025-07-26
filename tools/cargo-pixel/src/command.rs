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
use clap::{App, Arg, ArgMatches, SubCommand};

pub fn common_arg(app: App) -> App {
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

pub fn make_parser() -> ArgMatches {
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
                        .possible_values(["t", "s", "w", "g", "wg", "term", "sdl", "web", "winit", "wgpu"]),
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
                        .possible_values(["t", "s", "w", "term", "sdl", "web"]),
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

