fn main() {
    rust_pixel::only_graphics_mode!();
    #[cfg(any(feature = "wgpu", target_arch = "wasm32"))]
    tower::run()
}
