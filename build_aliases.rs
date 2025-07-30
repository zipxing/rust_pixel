// 共享的cfg_aliases配置函数
// 在各个crate的build.rs中调用：setup_cfg_aliases();

fn setup_cfg_aliases() {
    use cfg_aliases::cfg_aliases;
    
    cfg_aliases! {
        // 平台别名
        wasm: { target_arch = "wasm32" },
        mobile: { any(target_os = "android", target_os = "ios") },
        desktop: { not(any(wasm, mobile)) },
        
        // 渲染后端别名
        graphics_backend: { any(feature = "sdl", feature = "winit", feature = "wgpu") },
        terminal_backend: { not(graphics_backend) },
        
        // 具体后端别名
        sdl_backend: { all(feature = "sdl", not(wasm)) },
        winit_backend: { all(feature = "winit", not(wasm), not(feature = "wgpu")) },
        wgpu_backend: { all(feature = "wgpu", not(wasm)) },
        web_backend: { wasm },
        cross_backend: { terminal_backend },
        
        // 音频支持别名
        audio_support: { not(any(mobile, wasm)) },
        
        // 功能组合别名
        full_features: { all(graphics_backend, audio_support) },
        minimal_features: { feature = "base" },
        
        // 图形渲染模式 (包括wasm)
        graphics_mode: { any(graphics_backend, wasm) },
        
        // 不包含wgpu的图形模式 (用于某些特殊情况)
        graphics_simple: { any(feature = "sdl", feature = "winit", wasm) },
        
        // 常用组合
        native_graphics: { all(desktop, graphics_backend) },
        web_ready: { all(wasm, feature = "image") },
    }
}