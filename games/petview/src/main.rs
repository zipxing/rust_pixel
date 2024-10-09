fn main() {
    #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
    {
        println!("Run in graphics only...");
        return;
    }
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    petview::run()
}
