# 统一符号映射配置设计

## 背景

当前 `cell.rs` 中硬编码了三个映射表：
- `CELL_SYM_MAP`: Sprite 区域字符映射
- `TUI_CELL_SYM_MAP`: TUI 区域字符映射
- `EMOJI_MAP`: Emoji 区域字符映射

问题：
1. 硬编码在 Rust 代码中，修改需要重新编译
2. 三个独立的 HashMap，逻辑分散
3. 未来添加 CJK 支持需要再加一个映射表

## 目标

设计统一的 JSON 配置机制：
- 代码与配置分离，修改映射无需重新编译
- 统一的配置格式，逻辑清晰
- 支持 Sprite/TUI/Emoji/CJK 四种区域
- 保持高性能（启动时加载，运行时 O(1) 查询）

## JSON 配置格式

### symbol_map.json

```json
{
  "version": 1,
  "texture_size": 4096,
  "regions": {
    "sprite": {
      "type": "block",
      "block_range": [0, 159],
      "char_size": [16, 16],
      "chars_per_block": 256,
      "symbols": "@abcdefghijklmnopqrstuvwxyz[£]↑← !\"#$%&'()*+,-./0123456789:;<=>?─ABCDEFGHIJKLMNOPQRSTUVWXYZ┼",
      "extras": {
        "▇": [0, 209],
        "▒": [0, 94],
        "∙": [0, 122],
        "│": [0, 93],
        "┐": [0, 110],
        "╮": [0, 73],
        "┌": [0, 112],
        "╭": [0, 85],
        "└": [0, 109],
        "╰": [0, 74],
        "┘": [0, 125],
        "╯": [0, 75]
      }
    },
    "tui": {
      "type": "block",
      "block_range": [160, 169],
      "char_size": [16, 32],
      "chars_per_block": 256,
      "symbols": " !#$%&()*+,-./0123456789:;\"'<=>?@[\\]^_`{|}~⌐¬½¼¡«»∙·※⦿ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\u{E0B1}\u{E0B3}▀▄äàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈‾√ⁿ²♠♣♥♦░▒▓\u{E0B0}\u{E0B2}▙▟▛▜⚆⚇⚈⚉◐◑◓◒▴▾◂▸←↑→↓⭠⭡⭢⭣⠁⠂⠄⠈⠐⠠⡀⢀█▉▊▋▌▍▎▏█▇▆▅▄▃▂▁│║┃─═━┐╮╗┓┌╭╔┏┘╯╝┛└╰╚┗┤╣┫├╠┣┬╦┳┴╩┻┼╬╋≋"
    },
    "emoji": {
      "type": "block",
      "block_range": [170, 175],
      "char_size": [32, 32],
      "chars_per_block": 128,
      "symbols": [
        "😀", "😃", "😆", "😅", "😂", "😇", "😍", "😎", "😜", "🥺",
        "😢", "😟", "😤", "😭", "😱", "😡", "😵", "🤮", "🌼", "🍉",
        "🎃", "🍄", "🌹", "🌻", "🌸", "🪴", "🌷", "🌵", "🌲", "🌳",
        "🌴", "🎄", "🌿", "🍀", "🌱", "🪷", "🌞", "🌛", "⭐", "⚡",
        "🌈", "💦", "💧", "☔", "❄", "🍎", "🍋", "🍑", "🍌", "🍇",
        "🍓", "🥝", "🥭", "🍒", "🥬", "🍆", "🥕", "🥚", "🧅", "🍞",
        "🧄", "🍗", "🌶", "🍖", "🦴", "🍔", "🍟", "🍕", "🥦", "🍚",
        "🥟", "🍜", "🍺", "🍻", "🥂", "🍷", "🍸", "🍹", "🎂", "🧁",
        "🍰", "🏀", "⚽", "🏈", "🥎", "🏐", "🎱", "🏓", "⛳", "🏒",
        "🏹", "🥊", "🪂", "🎣", "🥇", "🥈", "🥉", "🎲", "🏆", "🚗",
        "🚑", "🚌", "🚀", "🚁", "⛵", "⚓", "🛬", "🛩", "⏰", "💰",
        "💣", "🧨", "💈", "🎁", "🎈", "🎉", "🔑", "👉", "👆", "👈",
        "👇", "👍", "👏", "👎", "👊", "👌", "👩", "🧑", "👨", "👵",
        "👷", "👮", "🥷", "🙏", "✌", "🐶", "🐱", "🐭", "🐹", "🐰",
        "🦊", "🐻", "🐼", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸", "🐵",
        "🐒", "🐥", "🦋", "🐬", "🐳", "🦀", "🐠", "🦈", "🐴", "🦂",
        "🦕", "🐙", "🐏", "🦒", "🦓", "🐆", "🐫", "🦌", "🐘", "🦛",
        "🦏", "🦚", "🦜", "🐓", "🦢", "🐇", "🐝", "🐞", "🐍", "🐢",
        "🎹", "🥁", "🎸", "🪗", "🎻", "🎺", "🎷", "🪕", "🪘", "🗿",
        "🗽", "🗼", "🏰", "🏯", "🎡", "🎢", "⛲", "⛰", "🎠", "⛱",
        "🏖", "🏝", "🏜", "🌋", "🏠", "🏡", "🏘", "🏚", "🏭", "🏥",
        "🏢", "🏬", "⛺", "🏕", "🛖", "🕌", "📱", "🎙", "📺", "📞",
        "🖥", "💻", "⌛", "🛠", "⚙", "🧸", "🪣", "📎", "🔗", "📒",
        "📅", "🔐", "✏", "🧲", "💕", "💝", "✅", "❎", "❌", "🆘",
        "🚫", "💤", "🚸", "🔴", "🟠", "🟡", "🟢", "🔵", "🟣", "⚫",
        "⚪", "🟤", "🟥", "🟧", "🟨", "🟩", "🟦", "🟪", "⬛", "⬜",
        "🟫", "🏧", "🛃", "🛅", "🛄", "🚹", "🚺", "🚼", "🔆", "❤"
      ]
    },
    "cjk": {
      "type": "grid",
      "pixel_region": [0, 3072, 4096, 1024],
      "char_size": [32, 32],
      "grid_cols": 128,
      "mappings": {}
    }
  }
}
```

### 配置说明

**通用字段**:
- `version`: 配置版本号
- `texture_size`: 纹理尺寸 (4096)

**区域类型**:
- `type: "block"`: Block-based 布局（Sprite/TUI/Emoji）
- `type: "grid"`: Grid 布局（CJK）

**Block 类型字段**:
- `block_range`: [起始Block, 结束Block]
- `char_size`: [宽度px, 高度px]
- `chars_per_block`: 每个 Block 的字符数
- `symbols`: 字符序列（字符串或数组）
- `extras`: 额外映射 { "字符": [block, index] }

**Grid 类型字段**:
- `pixel_region`: [x, y, width, height] 像素区域
- `char_size`: [宽度px, 高度px]
- `grid_cols`: 每行字符数
- `mappings`: { "字符": [grid_x, grid_y] } 或由工具生成

## Rust 实现

### 数据结构

```rust
// src/render/symbol_map.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 符号索引结果
#[derive(Debug, Clone, Copy)]
pub enum SymbolIndex {
    /// Sprite 区域: (block, index)
    Sprite(u8, u8),
    /// TUI 区域: (block, index)
    Tui(u8, u8),
    /// Emoji 区域: (block, index)
    Emoji(u8, u8),
    /// CJK 区域: (pixel_x, pixel_y)
    Cjk(u16, u16),
    /// 未找到
    NotFound,
}

/// 统一符号映射表
pub struct SymbolMap {
    sprite: HashMap<String, (u8, u8)>,
    tui: HashMap<String, (u8, u8)>,
    emoji: HashMap<String, (u8, u8)>,
    cjk: HashMap<char, (u16, u16)>,
}

impl SymbolMap {
    /// 从 JSON 文件加载
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// 从 JSON 字符串解析
    pub fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: SymbolMapConfig = serde_json::from_str(json)?;
        Ok(Self::from_config(config))
    }

    /// 从内嵌默认配置加载（向后兼容）
    pub fn default() -> Self {
        let json = include_str!("../../assets/pix/symbol_map.json");
        Self::from_json(json).expect("Invalid embedded symbol_map.json")
    }

    fn from_config(config: SymbolMapConfig) -> Self {
        let mut sprite = HashMap::new();
        let mut tui = HashMap::new();
        let mut emoji = HashMap::new();
        let mut cjk = HashMap::new();

        // 解析 Sprite 区域
        if let Some(region) = config.regions.get("sprite") {
            Self::parse_block_region(region, &mut sprite);
        }

        // 解析 TUI 区域
        if let Some(region) = config.regions.get("tui") {
            Self::parse_block_region(region, &mut tui);
        }

        // 解析 Emoji 区域
        if let Some(region) = config.regions.get("emoji") {
            Self::parse_block_region(region, &mut emoji);
        }

        // 解析 CJK 区域
        if let Some(region) = config.regions.get("cjk") {
            Self::parse_grid_region(region, &mut cjk);
        }

        Self { sprite, tui, emoji, cjk }
    }

    fn parse_block_region(region: &RegionConfig, map: &mut HashMap<String, (u8, u8)>) {
        let block_start = region.block_range.as_ref().map(|r| r[0]).unwrap_or(0);
        let chars_per_block = region.chars_per_block.unwrap_or(256) as u8;

        // 解析 symbols 字符串或数组
        if let Some(symbols) = &region.symbols {
            let mut block = block_start;
            let mut idx = 0u8;

            match symbols {
                SymbolsValue::String(s) => {
                    for ch in s.chars() {
                        map.insert(ch.to_string(), (block, idx));
                        idx += 1;
                        if idx == chars_per_block {
                            idx = 0;
                            block += 1;
                        }
                    }
                }
                SymbolsValue::Array(arr) => {
                    for s in arr {
                        map.insert(s.clone(), (block, idx));
                        idx += 1;
                        if idx == chars_per_block {
                            idx = 0;
                            block += 1;
                        }
                    }
                }
            }
        }

        // 解析 extras
        if let Some(extras) = &region.extras {
            for (ch, coords) in extras {
                map.insert(ch.clone(), (coords[0], coords[1]));
            }
        }
    }

    fn parse_grid_region(region: &RegionConfig, map: &mut HashMap<char, (u16, u16)>) {
        let pixel_region = region.pixel_region.as_ref();
        let char_size = region.char_size.as_ref();
        let grid_cols = region.grid_cols.unwrap_or(128);

        if let (Some(pr), Some(cs)) = (pixel_region, char_size) {
            let base_x = pr[0] as u16;
            let base_y = pr[1] as u16;
            let char_w = cs[0] as u16;
            let char_h = cs[1] as u16;

            if let Some(mappings) = &region.mappings {
                for (ch, coords) in mappings {
                    if let Some(c) = ch.chars().next() {
                        let pixel_x = base_x + coords[0] as u16 * char_w;
                        let pixel_y = base_y + coords[1] as u16 * char_h;
                        map.insert(c, (pixel_x, pixel_y));
                    }
                }
            }
        }
    }

    /// 查询 Sprite 区域符号
    pub fn sprite_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        self.sprite.get(symbol).copied()
    }

    /// 查询 TUI 区域符号
    pub fn tui_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        self.tui.get(symbol).copied()
    }

    /// 查询 Emoji 区域符号
    pub fn emoji_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        self.emoji.get(symbol).copied()
    }

    /// 查询 CJK 区域符号（返回像素坐标）
    pub fn cjk_coords(&self, ch: char) -> Option<(u16, u16)> {
        self.cjk.get(&ch).copied()
    }

    /// 统一查询接口
    pub fn lookup(&self, symbol: &str) -> SymbolIndex {
        // 优先级: Emoji > TUI > Sprite > CJK
        if let Some((block, idx)) = self.emoji.get(symbol) {
            return SymbolIndex::Emoji(*block, *idx);
        }
        if let Some((block, idx)) = self.tui.get(symbol) {
            return SymbolIndex::Tui(*block, *idx);
        }
        if let Some((block, idx)) = self.sprite.get(symbol) {
            return SymbolIndex::Sprite(*block, *idx);
        }
        if let Some(ch) = symbol.chars().next() {
            if let Some((x, y)) = self.cjk.get(&ch) {
                return SymbolIndex::Cjk(*x, *y);
            }
        }
        SymbolIndex::NotFound
    }
}

// JSON 配置结构
#[derive(Deserialize)]
struct SymbolMapConfig {
    version: u32,
    texture_size: u32,
    regions: HashMap<String, RegionConfig>,
}

#[derive(Deserialize)]
struct RegionConfig {
    #[serde(rename = "type")]
    region_type: Option<String>,
    block_range: Option<[u8; 2]>,
    char_size: Option<[u32; 2]>,
    chars_per_block: Option<u32>,
    symbols: Option<SymbolsValue>,
    extras: Option<HashMap<String, [u8; 2]>>,
    pixel_region: Option<[u32; 4]>,
    grid_cols: Option<u32>,
    mappings: Option<HashMap<String, [u32; 2]>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SymbolsValue {
    String(String),
    Array(Vec<String>),
}
```

### Context 集成

```rust
// src/context.rs

pub struct Context {
    // ... 现有字段
    pub symbol_map: SymbolMap,
}

impl Context {
    pub fn new() -> Self {
        Self {
            // ...
            symbol_map: SymbolMap::default(),
        }
    }

    /// 使用自定义符号映射
    pub fn with_symbol_map(mut self, path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        self.symbol_map = SymbolMap::load(path)?;
        Ok(self)
    }
}
```

### 渲染层调用

```rust
// src/render/graph.rs

fn render_main_buffer(..., symbol_map: &SymbolMap, ...) {
    for cell in buffer.iter() {
        match symbol_map.lookup(&cell.symbol) {
            SymbolIndex::Sprite(block, idx) => {
                // Sprite 渲染逻辑
            }
            SymbolIndex::Tui(block, idx) => {
                // TUI 渲染逻辑
            }
            SymbolIndex::Emoji(block, idx) => {
                // Emoji 渲染逻辑
            }
            SymbolIndex::Cjk(x, y) => {
                // CJK 渲染逻辑
            }
            SymbolIndex::NotFound => {
                // 回退到空格或默认符号
            }
        }
    }
}
```

## 工具支持

### cargo pixel cjk

生成 CJK 映射并更新 symbol_map.json:

```bash
cargo pixel cjk assets/fonts/simhei.ttf chars.txt assets/pix/symbols.png --map assets/pix/symbol_map.json
```

工具会：
1. 将汉字渲染到 symbols.png 的 CJK 区域
2. 更新 symbol_map.json 的 `regions.cjk.mappings` 字段

## 迁移计划

### Phase 1: 创建配置文件
1. 创建 `assets/pix/symbol_map.json`
2. 从现有硬编码 Map 导出初始配置

### Phase 2: 实现 SymbolMap
1. 添加 `src/render/symbol_map.rs`
2. 实现 JSON 加载和查询接口

### Phase 3: 集成到渲染层
1. 修改 `cell.rs` 使用 SymbolMap
2. 修改 `graph.rs` 使用统一查询接口
3. 删除硬编码的 CELL_SYM_MAP、TUI_CELL_SYM_MAP、EMOJI_MAP

### Phase 4: CJK 工具
1. 扩展 `cargo pixel cjk` 支持更新 symbol_map.json

## 优势

1. **配置与代码分离** - 修改映射无需重新编译
2. **统一机制** - 四种区域使用同一套配置格式
3. **可扩展** - 轻松添加新的字符映射
4. **可调试** - JSON 格式便于查看和验证
5. **向后兼容** - 内嵌默认配置，无需额外文件
6. **高性能** - 启动时一次性加载，运行时 O(1) 查询
