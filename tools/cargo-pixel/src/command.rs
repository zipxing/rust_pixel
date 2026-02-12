// RustPixel
// copyright zipxing@hotmail.com 2022～2025

/// rust_pixel cargo build tools...
///
/// Usage:
/// cargo pixel run snake term
/// cargo pixel run snake wgpu
/// cargo pixel creat games mygame
/// cargo pixel build snake web
/// cargo pixel asset input_folder output_folder
/// cargo pixel edit term .
/// cargo pixel edit wg . file.pix
/// cargo pixel petii image.png 40 25
/// cargo pixel ssf . dance.ssf
/// cargo pixel symbol image.png 8
/// cargo pixel ttf
///
/// shortcut:
/// cargo pixel r snake t
/// cargo pixel r snake wg
/// cargo pixel r snake w
/// cargo pixel asset ./sprites ./output
/// cargo pixel edit t .
/// cargo pixel edit wg . file.pix
/// cargo pixel p image.png 40 25
/// cargo pixel sf . dance.ssf
/// cargo pixel sy image.png 8
/// cargo pixel tf
/// ...
///
use clap::{Arg, ArgAction, ArgMatches, Command};

/// Internal function to build the command line parser app structure
/// This eliminates code duplication between make_parser and make_parser_app
fn build_app() -> Command {
    Command::new("cargo pixel")
        .author("zipxing@hotmail.com")
        .about("RustPixel cargo tools")
        .arg(Arg::new("pixel"))
        .subcommand(common_arg(
            Command::new("run")
                .alias("r")
                .about("Run RustPixel projects and tools")
                .arg(Arg::new("mod_name").required(true))
                .arg(
                    Arg::new("build_type")
                        .required(true)
                        .value_parser(["t", "w", "wg", "term", "web", "wgpu"]),
                )
                .arg(Arg::new("other").action(ArgAction::Append)),
        ))
        .subcommand(common_arg(
            Command::new("build")
                .alias("b")
                .about("Build RustPixel projects for different targets")
                .arg(Arg::new("mod_name").required(true))
                .arg(
                    Arg::new("build_type")
                        .required(true)
                        .value_parser(["t", "w", "wg", "term", "web", "wgpu"]),
                ),
        ))
        .subcommand(common_arg(
            Command::new("creat")
                .alias("c")
                .about("Create new RustPixel projects from templates")
                .arg(Arg::new("mod_name").required(true))
                .arg(Arg::new("standalone_dir_name").required(false)),
        ))
        .subcommand(common_arg(
            Command::new("convert_gif")
                .alias("cg")
                .about("Convert GIF animations to SSF sequence frame format")
                .arg(Arg::new("gif").required(true))
                .arg(Arg::new("ssf").required(true))
                .arg(Arg::new("width").required(true))
                .arg(Arg::new("height").required(true)),
        ))
        .subcommand(
            Command::new("asset")
                .alias("a")
                .about("Pack images into texture atlas and generate .pix files")
                .arg(
                    Arg::new("input_folder")
                        .help("Folder containing images to pack")
                        .required(false)
                        .index(1),
                )
                .arg(
                    Arg::new("output_folder")
                        .help("Folder where output files will be written")
                        .required(false)
                        .index(2),
                ),
        )
        .subcommand(
            Command::new("edit")
                .alias("e")
                .about("Run RustPixel image/sprite editor")
                .arg(
                    Arg::new("mode")
                        .help("Running mode")
                        .required(false)
                        .value_parser(["t", "w", "wg", "term", "web", "wgpu"])
                        .index(1),
                )
                .arg(
                    Arg::new("work_dir")
                        .help("Working directory")
                        .required(false)
                        .index(2),
                )
                .arg(
                    Arg::new("image_file")
                        .help("Image file to edit")
                        .required(false)
                        .index(3),
                ),
        )
        .subcommand(
            Command::new("petii")
                .alias("p")
                .about("Convert images to PETSCII art")
                .arg(
                    Arg::new("image_file")
                        .help("Input image file path")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("width")
                        .help("Output width in characters (default: 40)")
                        .required(false)
                        .index(2),
                )
                .arg(
                    Arg::new("height")
                        .help("Output height in characters (default: 25)")
                        .required(false)
                        .index(3),
                )
                .arg(
                    Arg::new("is_petscii")
                        .help("Use PETSCII characters: true/false (default: false)")
                        .required(false)
                        .index(4),
                )
                .arg(
                    Arg::new("crop_x")
                        .help("Crop start X coordinate (requires all crop params)")
                        .required(false)
                        .index(5),
                )
                .arg(
                    Arg::new("crop_y")
                        .help("Crop start Y coordinate")
                        .required(false)
                        .index(6),
                )
                .arg(
                    Arg::new("crop_width")
                        .help("Crop width")
                        .required(false)
                        .index(7),
                )
                .arg(
                    Arg::new("crop_height")
                        .help("Crop height")
                        .required(false)
                        .index(8),
                ),
        )
        .subcommand(
            Command::new("ssf")
                .alias("sf")
                .about("Run SSF sequence frame player (fixed wgpu mode)")
                .arg(
                    Arg::new("work_dir")
                        .help("Working directory")
                        .required(false)
                        .index(1),
                )
                .arg(
                    Arg::new("ssf_file")
                        .help("SSF file path (optional)")
                        .required(false)
                        .index(2),
                ),
        )
        .subcommand(
            Command::new("symbol")
                .alias("sy")
                .about("Extract symbols/characters from images")
                .arg(
                    Arg::new("image_file")
                        .help("Input image file path")
                        .required(false)
                        .index(1),
                )
                .arg(
                    Arg::new("symsize")
                        .help("Symbol size in pixels (e.g., 8 for 8x8 symbols)")
                        .required(false)
                        .index(2),
                )
                .arg(
                    Arg::new("start_x")
                        .help("Start X coordinate for processing area")
                        .required(false)
                        .index(3),
                )
                .arg(
                    Arg::new("start_y")
                        .help("Start Y coordinate for processing area")
                        .required(false)
                        .index(4),
                )
                .arg(
                    Arg::new("width")
                        .help("Width of processing area")
                        .required(false)
                        .index(5),
                )
                .arg(
                    Arg::new("height")
                        .help("Height of processing area")
                        .required(false)
                        .index(6),
                ),
        )
        .subcommand(
            Command::new("ttf")
                .alias("tt")
                .about("Convert TTF font files to PNG character atlas")
                .arg(
                    Arg::new("ttf_file")
                        .help("Input TTF font file path")
                        .required(false)
                        .index(1),
                )
                .arg(
                    Arg::new("output_file")
                        .help("Output PNG file path (default: font_atlas.png)")
                        .required(false)
                        .index(2),
                )
                .arg(
                    Arg::new("size")
                        .help("Character size in pixels (default: 16)")
                        .required(false)
                        .index(3),
                )
                .arg(
                    Arg::new("chars_per_row")
                        .help("Number of characters per row (default: 16)")
                        .required(false)
                        .index(4),
                )
                .arg(
                    Arg::new("verbose")
                        .help("Show detailed analysis: 0=false, 1=true (default: 0)")
                        .required(false)
                        .index(5),
                ),
        )
}

pub fn common_arg(app: Command) -> Command {
    app.arg(
        Arg::new("release")
            .short('r')
            .long("release")
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new("webport")
            .short('p')
            .long("webport")
            .default_value("8080"),
    )
}

pub fn make_parser() -> ArgMatches {
    build_app().get_matches()
}

/// Create the parser app without getting matches (for help display)
pub fn make_parser_app() -> Command {
    build_app()
}

