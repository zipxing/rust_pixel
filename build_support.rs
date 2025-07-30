// Shared cfg_aliases configuration function
// Called in each crate's build.rs: setup_cfg_aliases();

fn setup_rust_pixel_cfg_aliases() {
    use cfg_aliases::cfg_aliases;
    
    cfg_aliases! {
        // Platform aliases
        wasm: { target_arch = "wasm32" },
        mobile: { any(target_os = "android", target_os = "ios") },
        desktop: { not(any(wasm, mobile)) },
        
        // Rendering backend aliases
        graphics_backend: { any(feature = "sdl", feature = "winit", feature = "wgpu") },
        terminal_backend: { not(graphics_backend) },
        
        // Specific backend aliases
        sdl_backend: { all(feature = "sdl", not(wasm)) },
        winit_backend: { all(feature = "winit", not(wasm), not(feature = "wgpu")) },
        wgpu_backend: { all(feature = "wgpu", not(wasm)) },
        web_backend: { wasm },
        cross_backend: { terminal_backend },
        
        // Audio support aliases
        audio_support: { not(any(mobile, wasm)) },
        
        // Feature combination aliases
        full_features: { all(graphics_backend, audio_support) },
        minimal_features: { feature = "base" },
        
        // Graphics rendering mode (including wasm)
        graphics_mode: { any(graphics_backend, wasm) },
        
        // Graphics mode without wgpu (for special cases)
        graphics_simple: { any(feature = "sdl", feature = "winit", wasm) },
        
        // Common combinations
        native_graphics: { all(desktop, graphics_backend) },
        web_ready: { all(wasm, feature = "image") },
    }
}
