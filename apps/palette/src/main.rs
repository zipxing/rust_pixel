fn main() {
    rust_pixel::only_terminal_mode!();
    #[cfg(not(graphics_mode))]
    palette::run()
}
