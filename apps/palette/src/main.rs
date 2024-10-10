fn main() {
    rust_pixel::only_terminal_mode!();
    #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
    palette::run()
}
