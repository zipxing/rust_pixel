fn main() {
    rust_pixel::only_graphics_mode!();
    #[cfg(graphics_mode)]
    petview::run()
}
