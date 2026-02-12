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
        // Both native wgpu and wasm are graphics backends
        graphics_backend: { any(feature = "wgpu", wasm) },

        // Specific backend aliases
        // Native wgpu with winit window management
        wgpu_backend: { all(feature = "wgpu", not(wasm)) },
        // Web wgpu - uses wgpu with canvas (no winit)
        // Uses wasm target detection (no feature flag needed)
        wgpu_web_backend: { wasm },
        // Terminal mode (crossterm)
        cross_backend: { not(graphics_mode) },

        // Audio support aliases
        audio_support: { not(any(mobile, wasm)) },

        // Graphics rendering mode (both native wgpu and web wgpu)
        graphics_mode: { any(graphics_backend, wasm) },
    }
}
