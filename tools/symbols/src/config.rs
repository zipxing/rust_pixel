// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Texture configuration for symbol generation

/// Mipmap level definition for a specific symbol type and resolution
#[derive(Debug, Clone, Copy)]
pub struct MipLevelDef {
    pub width: u32,
    pub height: u32,
}

/// Mipmap configuration for a symbol type (3 levels: high/mid/low)
#[derive(Debug, Clone, Copy)]
pub struct SymbolMipConfig {
    pub levels: [MipLevelDef; 3],
}

/// Configuration for layered texture generation (Texture2DArray)
#[derive(Debug, Clone)]
pub struct LayeredTextureConfig {
    pub layer_size: u32,         // 4096
    pub sprite: SymbolMipConfig, // Sprite: 64×64, 32×32, 16×16
    pub tui: SymbolMipConfig,    // TUI: 64×128, 32×64, 16×32
    pub emoji: SymbolMipConfig,  // Emoji: 128×128, 64×64, 32×32
    pub cjk: SymbolMipConfig,    // CJK: 128×128, 64×64, 32×32

    // Render parameters (render at high res, then downscale for quality)
    pub tui_render_w: u32,       // 256 (4x Level 0 width)
    pub tui_render_h: u32,       // 512 (4x Level 0 height)
    pub cjk_render_size: u32,    // 512 (4x Level 0)
    pub emoji_render_size: u32,  // 256 (2x Level 0)
}

impl LayeredTextureConfig {
    pub fn new() -> Self {
        Self {
            layer_size: 4096,
            sprite: SymbolMipConfig {
                levels: [
                    MipLevelDef { width: 64, height: 64 },
                    MipLevelDef { width: 32, height: 32 },
                    MipLevelDef { width: 16, height: 16 },
                ],
            },
            tui: SymbolMipConfig {
                levels: [
                    MipLevelDef { width: 64, height: 128 },
                    MipLevelDef { width: 32, height: 64 },
                    MipLevelDef { width: 16, height: 32 },
                ],
            },
            emoji: SymbolMipConfig {
                levels: [
                    MipLevelDef { width: 128, height: 128 },
                    MipLevelDef { width: 64, height: 64 },
                    MipLevelDef { width: 32, height: 32 },
                ],
            },
            cjk: SymbolMipConfig {
                levels: [
                    MipLevelDef { width: 128, height: 128 },
                    MipLevelDef { width: 64, height: 64 },
                    MipLevelDef { width: 32, height: 32 },
                ],
            },
            tui_render_w: 256,
            tui_render_h: 512,
            cjk_render_size: 512,
            emoji_render_size: 256,
        }
    }

    /// Get the render size for a symbol type's highest mipmap level.
    /// Used to determine the size at which characters should be rendered
    /// before downscaling to lower mipmap levels.
    pub fn render_size(&self, sym_type: SymbolType) -> (u32, u32) {
        let mip = match sym_type {
            SymbolType::Sprite => &self.sprite,
            SymbolType::Tui => &self.tui,
            SymbolType::Emoji => &self.emoji,
            SymbolType::Cjk => &self.cjk,
        };
        (mip.levels[0].width, mip.levels[0].height)
    }
}

/// Symbol type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Sprite,
    Tui,
    Emoji,
    Cjk,
}

