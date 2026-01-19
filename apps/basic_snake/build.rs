include!("../../build_support.rs");

fn main() {
    setup_rust_pixel_cfg_aliases();

    // Embed BASIC script at compile time
    println!("cargo:rerun-if-changed=assets/game.bas");
}
