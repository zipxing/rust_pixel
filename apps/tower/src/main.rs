fn main() {
    rust_pixel::only_graphics_mode!();
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    tower::run()
}
