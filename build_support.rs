// RustPixel
// copyright zipxing@hotmail.com 2022～2025

// Shared cfg_aliases configuration function
// Called in each crate's build.rs: setup_cfg_aliases();

fn setup_rust_pixel_cfg_aliases() {
    use cfg_aliases::cfg_aliases;
    
    cfg_aliases! {
        // Platform aliases
        wasm: { target_arch = "wasm32" },
        mobile: { any(target_os = "android", target_os = "ios") },
        
        // Rendering backend aliases
        graphics_backend: { any(feature = "sdl", feature = "winit", feature = "wgpu") },
        
        // Specific backend aliases
        sdl_backend: { all(feature = "sdl", not(wasm)) },
        winit_backend: { all(feature = "winit", not(wasm), not(feature = "wgpu")) },
        wgpu_backend: { all(feature = "wgpu", not(wasm)) },
        cross_backend: { not(graphics_mode) },
        
        // Audio support aliases
        audio_support: { not(any(mobile, wasm)) },
        
        // Graphics rendering mode (including wasm)
        graphics_mode: { any(graphics_backend, wasm) },
    }
}
