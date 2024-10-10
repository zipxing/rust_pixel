fn main() {
    rust_pixel::only_graphical_mode!();
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    petview::run()
}
