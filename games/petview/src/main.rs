fn main() {
    rust_pixel::only_graphics_mode!();
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    petview::run()
}
