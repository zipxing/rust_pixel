# RustPixel Architecture Principles

RustPixel is a 2D game engine supporting both text-mode and graphics-mode rendering across multiple platforms. This document explains the core architecture and rendering principles.

## 🎮 Game Loop Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         MAIN GAME LOOP                          │
│                                                                 │
│  ┌─────────────┐    ┌──────────────┐    ┌─────────────────────┐ │
│  │   Context   │◄──►│     Game     │◄──►│   Asset Manager     │ │
│  │             │    │              │    │                     │ │
│  │ • stage     │    │ ┌──────────┐ │    │ • ImgPix (.pix)     │ │
│  │ • state     │    │ │  Model   │ │    │ • ImgEsc (.esc)     │ │
│  │ • rand      │    │ │          │ │    │ • ImgSsf (.ssf)     │ │
│  │ • events    │    │ │ • init() │ │    │ • Async Loading     │ │
│  │ • adapter   │    │ │ • update │ │    │                     │ │
│  └─────────────┘    │ │ • handle*│ │    └─────────────────────┘ │
│                     │ └──────────┘ │                            │
│                     │              │                            │
│                     │ ┌──────────┐ │    ┌─────────────────────┐ │
│                     │ │  Render  │ │    │   Event System      │ │
│                     │ │          │ │    │                     │ │
│                     │ │ • init() │ │    │ • Global Timer      │ │
│                     │ │ • draw() │ │◄──►│ • Event Center      │ │
│                     │ │ • handle*│ │    │ • Input Events      │ │
│                     │ └──────────┘ │    │                     │ │
│                     └──────────────┘    └─────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘

Flow: Context.stage → Model.update() → Render.draw() → Platform Adapter
```

### Core Traits

- **Model**: Manages game state, logic, and input handling
- **Render**: Handles visual output and UI rendering  
- **Game**: Orchestrates the main loop and coordinates Model/Render
- **Context**: Shared state container with adapters and resources

## 🖥️ Multi-Platform Adapter Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                      RENDER ADAPTERS                           │
│                                                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  TEXT MODE      │  │  GRAPHICS MODE  │  │   WEB MODE      │ │
│  │                 │  │                 │  │                 │ │
│  │ CrosstermAdapter│  │   SdlAdapter    │  │   WebAdapter    │ │
│  │                 │  │                 │  │                 │ │
│  │ • Terminal I/O  │  │ • SDL2 + OpenGL │  │ • WebGL + WASM  │ │
│  │ • ASCII/Unicode │  │ • PETSCII chars │  │ • Browser       │ │
│  │ • Double Buffer │  │ • Texture Atlas │  │ • JavaScript    │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│                                                                │
│  ┌─────────────────┐  ┌─────────────────┐                      │
│  │ WINIT ADAPTERS  │  │   WGPU MODE     │                      │
│  │                 │  │                 │                      │
│  │WinitGlowAdapter │  │WinitWgpuAdapter │                      │
│  │                 │  │                 │                      │
│  │ • Winit + Glow  │  │ • Modern GPU    │                      │
│  │ • Cross Platform│  │ • Vulkan/Metal  │                      │
│  │ • OpenGL ES     │  │ • Compute Shader│                      │
│  └─────────────────┘  └─────────────────┘                      │
└────────────────────────────────────────────────────────────────┘

Unified Interface: draw_all_graph() → Platform-Specific Rendering
```

## 🎨 Rendering Hierarchy

```
┌────────────────────────────────────────────────────────────────┐
│                     RENDERING PIPELINE                         │
│                                                                │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                        PANEL                              │ │
│  │  ┌─────────────┐ ┌─────────────┐ ┌──────────────────────┐ │ │
│  │  │   Layer 0   │ │   Layer 1   │ │      Layer N...      │ │ │
│  │  │             │ │             │ │                      │ │ │
│  │  │  Sprites    │ │  Sprites    │ │      Sprites         │ │ │
│  │  │             │ │             │ │                      │ │ │
│  │  │ ┌─────────┐ │ │ ┌─────────┐ │ │  ┌─────────┐         │ │ │
│  │  │ │Sprite 0 │ │ │ │Sprite 0 │ │ │  │Sprite 0 │         │ │ │
│  │  │ │         │ │ │ │         │ │ │  │         │         │ │ │
│  │  │ │ Buffer  │ │ │ │ Buffer  │ │ │  │ Buffer  │         │ │ │
│  │  │ │         │ │ │ │         │ │ │  │         │         │ │ │
│  │  │ │┌───────┐│ │ │ │┌───────┐│ │ │  │┌───────┐│         │ │ │
│  │  │ ││ Cells ││ │ │ ││ Cells ││ │ │  ││ Cells ││         │ │ │
│  │  │ │└───────┘│ │ │ │└───────┘│ │ │  │└───────┘│         │ │ │
│  │  │ └─────────┘ │ │ └─────────┘ │ │  └─────────┘         │ │ │
│  │  │ ┌─────────┐ │ │ ┌─────────┐ │ │  ┌─────────┐         │ │ │
│  │  │ │Sprite 1 │ │ │ │Sprite 1 │ │ │  │Sprite 1 │         │ │ │
│  │  │ │   ...   │ │ │ │   ...   │ │ │  │   ...   │         │ │ │
│  │  │ └─────────┘ │ │ └─────────┘ │ │  └─────────┘         │ │ │
│  │  └─────────────┘ └─────────────┘ └──────────────────────┘ │ │
│  │                                                           │ │
│  │  ┌──────────────────────────────────────────────────────┐ │ │
│  │  │                 DOUBLE BUFFER                        │ │ │
│  │  │  ┌─────────────┐              ┌─────────────┐        │ │ │
│  │  │  │   Buffer 0  │  ◄────────►  │   Buffer 1  │        │ │ │
│  │  │  │ (Previous)  │              │ (Current)   │        │ │ │
│  │  │  └─────────────┘              └─────────────┘        │ │ │
│  │  └──────────────────────────────────────────────────────┘ │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                ↓                               │
│                        ┌─────────────┐                         │
│                        │   ADAPTER   │                         │
│                        │             │                         │
│                        │ Platform-   │                         │
│                        │ Specific    │                         │
│                        │ Rendering   │                         │
│                        └─────────────┘                         │
└────────────────────────────────────────────────────────────────┘
```

### Cell Structure

```
Cell {
    symbol: String,     // Character or symbol index
    fg: Color,         // Foreground color
    bg: Color,         // Background color (texture in graphics)  
    modifier: Modifier, // Bold, italic, etc.
    tex: u8,           // Texture ID (graphics mode only)
}
```

## 📟 Text Mode Rendering

```
┌───────────────────────────────────────────────────────────────┐
│                      TEXT MODE FLOW                           │
│                                                               │
│  Panel Layers → Merge Sprites → Main Buffer → Double Buffer   │
│       ↓              ↓              ↓              ↓          │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐     │
│  │Layer 0  │    │ Sprite  │    │ Main    │    │Previous │     │
│  │Sprites  │───►│ Merging │───►│ Buffer  │───►│ vs      │     │
│  │Layer 1  │    │ by      │    │ (Cells) │    │Current  │     │
│  │ ...     │    │ Weight  │    │         │    │ Buffer  │     │
│  │Layer N  │    │         │    │         │    │         │     │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘     │
│                                                     ↓         │
│                                              ┌─────────┐      │
│                                              │  Diff   │      │
│                                              │ Detect  │      │
│                                              │         │      │
│                                              └─────────┘      │
│                                                     ↓         │
│                                              ┌─────────┐      │
│                                              │Crossterm│      │
│                                              │Terminal │      │
│                                              │ Output  │      │
│                                              └─────────┘      │
└───────────────────────────────────────────────────────────────┘

Features:
• Full Unicode support (emojis, symbols)
• ANSI color and styling
• Efficient diff-based updates
• Cross-platform terminal compatibility
```

## 🎮 Graphics Mode Rendering

```
┌─────────────────────────────────────────────────────────────────┐
│                    GRAPHICS MODE PIPELINE                       │
│                                                                 │
│ ┌─────────────────┐                    ┌─────────────────────┐  │
│ │  REGULAR LAYERS │                    │   PIXEL LAYERS      │  │
│ │                 │                    │                     │  │
│ │  Sprite → Cell  │                    │  Pixel Sprites      │  │
│ │  Collections    │                    │  (Pixel-level       │  │
│ │                 │                    │   movement)         │  │
│ │  Background/UI  │                    │  Transparency       │  │
│ └─────────────────┘                    └─────────────────────┘  │
│          │                                       │              │
│          ▼                                       ▼              │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │                   RENDER BUFFER                             │ │
│ │                                                             │ │
│ │  Vec<RenderCell> {                                          │ │
│ │    fcolor: (f32, f32, f32, f32),  // Foreground RGBA        │ │
│ │    bcolor: Option<(...)>,          // Background (optional) │ │
│ │    texsym: usize,                  // Texture symbol index  │ │
│ │    x, y: f32,                      // Position              │ │
│ │    w, h: u32,                      // Dimensions            │ │
│ │    angle: f32,                     // Rotation              │ │
│ │    cx, cy: f32,                    // Center point          │ │
│ │  }                                                          │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │                     GPU SHADERS                             │ │
│ │                                                             │ │
│ │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │ │
│ │  │   Symbols   │  │ Transition  │  │     General2D       │  │ │
│ │  │   Shader    │  │   Shader    │  │     Shader          │  │ │
│ │  │             │  │             │  │                     │  │ │
│ │  │ • Instanced │  │ • Mix two   │  │ • Texture to        │  │ │
│ │  │   Rendering │  │   textures  │  │   Screen            │  │ │
│ │  │ • Texture   │  │ • Effects   │  │ • Final output      │  │ │
│ │  │   Atlas     │  │ • Blending  │  │                     │  │ │
│ │  └─────────────┘  └─────────────┘  └─────────────────────┘  │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│                    ┌─────────────────┐                          │
│                    │ Platform Output │                          │
│                    │                 │                          │
│                    │ • SDL2 Window   │                          │
│                    │ • Winit Window  │                          │
│                    │ • WebGL Canvas  │                          │
│                    └─────────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
```

### Texture Atlas System

```
Texture Files:
┌─────────────┬─────────────┬─────────────┬─────────────┐
│   c64l.png  │   c64u.png  │  c64e1.png  │  c64e2.png  │
│  (tex=0)    │   (tex=1)   │   (tex=2)   │   (tex=3)   │
│             │             │             │             │
│ Small case  │ Upper case  │ Extension 1 │ Extension 2 │
│ C64 chars   │ C64 chars   │ Custom      │ Custom      │
│             │             │             │             │
│ 16x16 grid  │ 16x16 grid  │ 16x16 grid  │ 16x16 grid  │
│ 256 symbols │ 256 symbols │ 256 symbols │ 256 symbols │
└─────────────┴─────────────┴─────────────┴─────────────┘

Unicode Range: 0x2200 ~ 0x22FF maps to texture indices
```

## 🎯 Asset Management System

```
┌─────────────────────────────────────────────────────────────────┐
│                      ASSET PIPELINE                             │
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │   .pix      │    │   .esc      │    │       .ssf          │  │
│  │   Files     │    │   Files     │    │     Files           │  │
│  │             │    │             │    │                     │  │
│  │ Graphics    │    │ Terminal    │    │ Sequence Frames     │  │
│  │ Mode Art    │    │ Mode Art    │    │ Animation           │  │
│  │             │    │             │    │                     │  │
│  │ Cell Data   │    │ ESC + UTF-8 │    │ Multiple Frames     │  │
│  └─────────────┘    └─────────────┘    └─────────────────────┘  │
│         │                   │                     │             │
│         ▼                   ▼                     ▼             │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   ASSET MANAGER                            │ │
│  │                                                            │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │   ImgPix    │  │   ImgEsc    │  │      ImgSsf         │ │ │
│  │  │             │  │             │  │                     │ │ │
│  │  │ AssetBase { │  │ AssetBase { │  │   AssetBase {       │ │ │
│  │  │   location  │  │   location  │  │     location        │ │ │
│  │  │   raw_data  │  │   raw_data  │  │     raw_data        │ │ │
│  │  │   buffers   │  │   buffers   │  │     buffers[]       │ │ │
│  │  │   state     │  │   state     │  │     frame_count     │ │ │
│  │  │ }           │  │ }           │  │     state           │ │ │
│  │  └─────────────┘  └─────────────┘  │   }                 │ │ │
│  │                                    └─────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    LOADING STATES                          │ │
│  │                                                            │ │
│  │     Loading → Parsing → Ready                              │ │
│  │        │         │        │                                │ │
│  │        ▼         ▼        ▼                                │ │
│  │    Fetch     Parse     Available                           │ │
│  │    Data      Buffer    for Use                             │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘

Features:
• Async loading (Native: file I/O, Web: JavaScript fetch)
• Cross-platform path resolution
• Efficient buffer caching
• State-based loading management
```

## 🔧 Event System Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                       EVENT SYSTEM                               │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    EVENT CENTER                             │ │
│  │                                                             │ │
│  │     HashMap<String, HashMap<String, bool>>                  │ │
│  │            │              │         │                       │ │
│  │        Event Name    Function   Triggered                   │ │
│  │                                                             │ │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐  │ │
│  │  │   Register  │    │    Emit     │    │     Check       │  │ │
│  │  │             │    │             │    │                 │  │ │
│  │  │ event_      │    │ event_      │    │ event_          │  │ │
│  │  │ register()  │    │ emit()      │    │ check()         │  │ │
│  │  └─────────────┘    └─────────────┘    └─────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    TIMER SYSTEM                             │ │
│  │                                                             │ │
│  │     HashMap<String, Timer>                                  │ │
│  │            │         │                                      │ │
│  │       Timer Name   Config                                   │ │
│  │                                                             │ │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐  │ │
│  │  │   Create    │    │   Update    │    │     Query       │  │ │
│  │  │             │    │             │    │                 │  │ │
│  │  │ timer_      │    │ timer_      │    │ timer_          │  │ │
│  │  │ create()    │    │ update()    │    │ check()         │  │ │
│  │  └─────────────┘    └─────────────┘    └─────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                   INPUT EVENTS                              │ │
│  │                                                             │ │
│  │  Platform → Unified Event → Context.input_events            │ │
│  │     │              │               │                        │ │
│  │     ▼              ▼               ▼                        │ │
│  │  SDL/Web/      Event Enum      Vec<Event>                   │ │
│  │  Terminal      • Key           in Context                   │ │
│  │  Input         • Mouse                                      │ │
│  │                • Window                                     │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

## 🚀 Performance Optimizations

### WGPU Modern Graphics Pipeline

```
┌────────────────────────────────────────────────────────────────┐
│                       WGPU PIPELINE                            │
│                                                                │
│  CPU Side                           GPU Side                   │
│  ┌─────────────┐                   ┌─────────────────────────┐ │
│  │   Rust      │                   │       Shaders           │ │
│  │   Game      │    Buffer         │                         │ │
│  │   Logic     │───► Upload  ────► │  ┌─────────────────────┐│ │
│  │             │                   │  │  Vertex Shader      ││ │
│  │ RenderCell  │                   │  │  Fragment Shader    ││ │
│  │ Generation  │                   │  │  Compute Shader     ││ │
│  └─────────────┘                   │  └─────────────────────┘│ │
│                                    │           │             │ │
│  ┌─────────────┐                   │           ▼             │ │
│  │   Command   │                   │  ┌─────────────────────┐│ │
│  │   Buffer    │────Vulkan/────────►  │    GPU Memory       ││ │
│  │ Recording   │    Metal/         │  │                     ││ │
│  │             │    DX12           │  │  • Vertex Buffers   ││ │
│  └─────────────┘                   │  │  • Textures         ││ │
│                                    │  │  • Uniform Buffers  ││ │
│                                    │  └─────────────────────┘│ │
│                                    └─────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘

Benefits:
• Modern GPU API (Vulkan, Metal, DX12)
• Compute shader support
• Better memory management
• Cross-platform compatibility
```

### Rendering Optimizations

```
┌────────────────────────────────────────────────────────────────┐
│                    OPTIMIZATION STRATEGIES                     │
│                                                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ DOUBLE BUFFER   │  │ INSTANCED       │  │ TEXTURE ATLAS   │ │
│  │                 │  │ RENDERING       │  │                 │ │
│  │ • Diff Detection│  │                 │  │ • Batch Symbols │ │
│  │ • Minimal       │  │ • GPU Instancing│  │ • Reduce        │ │
│  │   Updates       │  │ • Single Draw   │  │   Draw Calls    │ │
│  │ • Flicker-free  │  │   Call          │  │ • Memory        │ │
│  │                 │  │ • Efficient     │  │   Efficiency    │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│                                                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ OBJECT POOLING  │  │ LAZY LOADING    │  │ CONDITIONAL     │ │
│  │                 │  │                 │  │ COMPILATION     │ │
│  │ • Reuse Objects │  │ • Load on       │  │                 │ │
│  │ • Reduce        │  │   Demand        │  │ • Platform      │ │
│  │   Allocations   │  │ • Async Assets  │  │   Specific      │ │
│  │ • Memory        │  │ • Progressive   │  │ • Feature       │ │
│  │   Efficiency    │  │   Loading       │  │   Flags         │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

## 🔗 Integration Points

### Macro System

```rust
// pixel_game! macro simplifies game creation
pixel_game!(MyGame);

// Generates:
// - mod model;
// - mod render_graphics; (or render_terminal)
// - Game initialization boilerplate
// - Platform-specific compilation

// asset2sprite! macro for easy asset loading
asset2sprite!(sprite, "path/to/asset.pix", ctx);
```

### Cross-Platform Features

```
┌────────────────────────────────────────────────────────────────┐
│                  PLATFORM FEATURES                             │
│                                                                │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │
│  │   Desktop   │ │     Web     │ │   Mobile    │ │ Terminal  │ │
│  │             │ │             │ │             │ │           │ │
│  │ • SDL2      │ │ • WebGL     │ │ • Touch     │ │ • SSH     │ │
│  │ • Winit     │ │ • WASM      │ │ • Sensors   │ │ • ASCII   │ │
│  │ • Native    │ │ • Browser   │ │ • Platform  │ │ • Colors  │ │
│  │   Audio     │ │   APIs      │ │   Specific  │ │ • Unicode │ │
│  │ • File I/O  │ │ • Canvas    │ │             │ │           │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘ │
│                                                                │
│  Common Interface: Context + Adapters + Asset Management       │
└────────────────────────────────────────────────────────────────┘
```

This architecture enables RustPixel to provide consistent game development experience across platforms while leveraging platform-specific optimizations where beneficial.
