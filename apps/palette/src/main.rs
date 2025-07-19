fn main() {
    rust_pixel::only_terminal_mode!();
    #[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
    palette::run()
}
