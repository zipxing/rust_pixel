// RustPixel
// copyright zipxing@hotmail.com 2022～2025

/// rust_pixel cargo build tools...
///
/// Usage:
/// cargo pixel run snake term
/// cargo pixel run snake sdl
/// cargo pixel creat games mygame
/// cargo pixel build snake web
/// cargo pixel asset input_folder output_folder
/// cargo pixel edit term .
/// cargo pixel edit wg . file.pix
/// cargo pixel petii image.png 40 25
/// cargo pixel ssf t . dance.ssf
/// cargo pixel symbol image.png 8
/// cargo pixel ttf
///
/// shortcut:
/// cargo pixel r snake t
/// cargo pixel r snake s
/// cargo pixel r snake w
/// cargo pixel asset ./sprites ./output
/// cargo pixel edit t .
/// cargo pixel edit wg . file.pix
/// cargo pixel p image.png 40 25
/// cargo pixel sf t . dance.ssf
/// cargo pixel sy image.png 8
/// cargo pixel tf
/// ...
///
use clap::{App, Arg, ArgMatches, SubCommand};

/// Internal function to build the command line parser app structure
/// This eliminates code duplication between make_parser and make_parser_app
fn build_app() -> App<'static> {
    App::new("cargo pixel")
        .author("zipxing@hotmail.com")
        .about("RustPixel cargo tools")
        .arg(Arg::with_name("pixel"))
        .subcommand(common_arg(
            SubCommand::with_name("run")
                .alias("r")
                .about("Run RustPixel projects and tools")
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
                .about("Build RustPixel projects for different targets")
                .arg(Arg::with_name("mod_name").required(true))
                .arg(
                    Arg::with_name("build_type")
                        .required(true)
                        .possible_values(["t", "s", "w", "g", "wg", "term", "sdl", "web", "winit", "wgpu"]),
                ),
        ))
        .subcommand(common_arg(
            SubCommand::with_name("creat")
                .alias("c")
                .about("Create new RustPixel projects from templates")
                .arg(Arg::with_name("mod_name").required(true))
                .arg(Arg::with_name("standalone_dir_name").required(false)),
        ))
        .subcommand(common_arg(
            SubCommand::with_name("convert_gif")
                .alias("cg")
                .about("Convert GIF animations to SSF sequence frame format")
                .arg(Arg::with_name("gif").required(true))
                .arg(Arg::with_name("ssf").required(true))
                .arg(Arg::with_name("width").required(true))
                .arg(Arg::with_name("height").required(true)),
        ))
        .subcommand(
            SubCommand::with_name("asset")
                .alias("a")
                .about("Pack images into texture atlas and generate .pix files")
                .arg(
                    Arg::with_name("input_folder")
                        .help("Folder containing images to pack")
                        .required(false)
                        .index(1),
                )
                .arg(
                    Arg::with_name("output_folder")
                        .help("Folder where output files will be written")
                        .required(false)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("edit")
                .alias("e")
                .about("Run RustPixel image/sprite editor")
                .arg(
                    Arg::with_name("mode")
                        .help("Running mode")
                        .required(false)
                        .possible_values(["t", "s", "w", "g", "wg", "term", "sdl", "web", "winit", "wgpu"])
                        .index(1),
                )
                .arg(
                    Arg::with_name("image_file")
                        .help("Image file to edit")
                        .required(false)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("petii")
                .alias("pe")
                .about("Convert images to PETSCII art")
                .arg(
                    Arg::with_name("mode")
                        .help("Running mode")
                        .required(false)
                        .possible_values(["t", "s", "w", "g", "wg", "term", "sdl", "web", "winit", "wgpu"])
                        .index(1),
                )
                .arg(
                    Arg::with_name("image_file")
                        .help("Image file to convert")
                        .required(false)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("ssf")
                .alias("sf")
                .about("Run SSF sequence frame player (fixed wgpu mode)")
                .arg(
                    Arg::with_name("work_dir")
                        .help("Working directory")
                        .required(false)
                        .index(1),
                )
                .arg(
                    Arg::with_name("ssf_file")
                        .help("SSF file path (optional)")
                        .required(false)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("symbol")
                .alias("sy")
                .about("Extract symbols/characters from images")
                .arg(
                    Arg::with_name("image_file")
                        .help("Input image file path")
                        .required(false)
                        .index(1),
                )
                .arg(
                    Arg::with_name("output_file")
                        .help("Output text file path")
                        .required(false)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("ttf")
                .alias("tt")
                .about("Process TTF fonts"),
        )
}

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
    build_app().get_matches()
}

/// Create the parser app without getting matches (for help display)
pub fn make_parser_app() -> App<'static> {
    build_app()
}

