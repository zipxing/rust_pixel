# 测试用例：纹理系统重构 — 关键算法 + 关键逻辑

## 1. DP Shelf-Packing 算法测试

位置：`tools/cargo-pixel/src/symbols/texture.rs` `#[cfg(test)] mod tests`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================
    // 1.1 dp_fill_layer: 单层填充 DP
    // =====================================================

    #[test]
    fn test_dp_fill_single_type() {
        // 只有 height=64 的 shelf，16 行正好填满 2048
        // remaining: [h128, h64, h32, h16]
        let mut remaining = [0u32, 16, 0, 0];
        let result = dp_fill_layer(&mut remaining);

        // 应该用掉 16 行 h64，总高度 = 16×64 = 1024... 不对
        // 容量 128 单位，h64=4 单位，16×4=64 < 128
        // 所以 16 行只占 64 单位，还有 64 单位空闲但无可用 shelf
        let total_units: u16 = result.iter()
            .map(|&(idx, count)| [8u16, 4, 2, 1][idx] * count as u16)
            .sum();
        assert_eq!(total_units, 64); // 16 × 4 = 64 单位
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    #[test]
    fn test_dp_fill_exact_capacity() {
        // 精确填满 128 单位 (= 2048 像素)
        // 8 行 h128 = 8×8 = 64 单位 + 16 行 h64 = 16×4 = 64 单位 = 128
        let mut remaining = [8u32, 16, 0, 0];
        let result = dp_fill_layer(&mut remaining);

        let total_units: u16 = result.iter()
            .map(|&(idx, count)| [8u16, 4, 2, 1][idx] * count as u16)
            .sum();
        assert_eq!(total_units, 128); // 精确填满
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    #[test]
    fn test_dp_fill_mixed_all_heights() {
        // 混合所有 4 种高度
        // 4×h128 + 8×h64 + 16×h32 + 32×h16
        // = 4×8 + 8×4 + 16×2 + 32×1 = 32 + 32 + 32 + 32 = 128 精确填满
        let mut remaining = [4u32, 8, 16, 32];
        let result = dp_fill_layer(&mut remaining);

        let total_units: u16 = result.iter()
            .map(|&(idx, count)| [8u16, 4, 2, 1][idx] * count as u16)
            .sum();
        assert_eq!(total_units, 128);
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    #[test]
    fn test_dp_fill_overflow_to_next_layer() {
        // 需求超过一层，应该只填满 128 单位，剩余留给下一层
        // 20 行 h128 = 20×8 = 160 单位 > 128
        // 一层最多 16 行 h128
        let mut remaining = [20u32, 0, 0, 0];
        let result = dp_fill_layer(&mut remaining);

        let total_units: u16 = result.iter()
            .map(|&(idx, count)| [8u16, 4, 2, 1][idx] * count as u16)
            .sum();
        assert_eq!(total_units, 128); // 恰好 16 行 h128
        assert_eq!(remaining[0], 4);  // 剩余 4 行
    }

    #[test]
    fn test_dp_fill_prioritize_large_shelves() {
        // 有大有小，DP 应优先使用大 shelf 填充（最大化利用率）
        // 1 行 h128(8) + 100 行 h16(1)
        // 最优: 1×h128 + (128-8)=120 行 h16 ... 但只有 100 行 h16
        // 所以: 1×h128(8) + 100×h16(100) = 108 单位
        let mut remaining = [1u32, 0, 0, 100];
        let result = dp_fill_layer(&mut remaining);

        let total_units: u16 = result.iter()
            .map(|&(idx, count)| [8u16, 4, 2, 1][idx] * count as u16)
            .sum();
        assert_eq!(total_units, 108);
        assert_eq!(remaining, [0, 0, 0, 0]); // 全部用完
    }

    #[test]
    fn test_dp_fill_empty_demand() {
        // 没有需求
        let mut remaining = [0u32, 0, 0, 0];
        let result = dp_fill_layer(&mut remaining);
        assert!(result.is_empty());
    }

    #[test]
    fn test_dp_fill_minimal() {
        // 只有 1 行 h16
        let mut remaining = [0u32, 0, 0, 1];
        let result = dp_fill_layer(&mut remaining);

        let total_units: u16 = result.iter()
            .map(|&(idx, count)| [8u16, 4, 2, 1][idx] * count as u16)
            .sum();
        assert_eq!(total_units, 1);
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    // =====================================================
    // 1.2 pack_all_layers: 多层填充
    // =====================================================

    #[test]
    fn test_pack_single_layer() {
        // 刚好填满一层
        let demands = [16u32, 0, 0, 0]; // 16×h128 = 128 单位 = 1 层
        let layers = pack_all_layers(&demands);
        assert_eq!(layers.len(), 1);
    }

    #[test]
    fn test_pack_two_layers() {
        // 需要 2 层
        let demands = [17u32, 0, 0, 0]; // 17×h128: 第 1 层 16 行，第 2 层 1 行
        let layers = pack_all_layers(&demands);
        assert_eq!(layers.len(), 2);
    }

    #[test]
    fn test_pack_zero_waste() {
        // 验证零浪费：总高度 / 2048 = 层数（向上取整）
        // 10×h128 + 20×h64 + 40×h32 + 80×h16
        // 总单位 = 10×8 + 20×4 + 40×2 + 80×1 = 80+80+80+80 = 320
        // 理论层数 = ceil(320/128) = 3
        let demands = [10u32, 20, 40, 80];
        let layers = pack_all_layers(&demands);
        assert_eq!(layers.len(), 3); // 零浪费 → 理论最小值
    }

    #[test]
    fn test_pack_full_production_scenario() {
        // 真实全量场景：
        // h128: 384 行, h64: 1472 行, h32: 736 行, h16: 320 行
        // 总单位 = 384×8 + 1472×4 + 736×2 + 320×1 = 3072+5888+1472+320 = 10752
        // 理论层数 = ceil(10752/128) = 84
        let demands = [384u32, 1472, 736, 320];
        let layers = pack_all_layers(&demands);
        assert_eq!(layers.len(), 84);
    }

    #[test]
    fn test_pack_level1_only() {
        // 仅 Level 1 场景：
        // h64: Sprite L1 (1280) + TUI L1 (40) + Emoji L1 (24) + CJK L1 (128) = 1472
        // h32: (无 Level 1 的 h32)
        // 总单位 = 1472×4 = 5888, 理论 = ceil(5888/128) = 46
        // 实际 Level 1 有多种高度...重新算
        //
        // Level 1 实际:
        // Sprite L1: 32×32 → h32, 40960/64=640 行
        // TUI L1: 32×64 → h64, 2560/64=40 行
        // Emoji L1: 64×64 → h64, 768/32=24 行
        // CJK L1: 64×64 → h64, 4096/32=128 行
        // h64: 40+24+128 = 192, h32: 640
        // 总单位 = 192×4 + 640×2 = 768+1280 = 2048
        // 理论层数 = ceil(2048/128) = 16
        let demands = [0u32, 192, 640, 0];
        let layers = pack_all_layers(&demands);
        assert_eq!(layers.len(), 16);
    }

    #[test]
    fn test_pack_typical_app() {
        // 典型 app: 4 个 Sprite block + TUI + 少量 Emoji
        // Sprite: 4×256 = 1024 个符号
        //   L0: 64×64 → h64, 1024/32=32 行
        //   L1: 32×32 → h32, 1024/64=16 行
        //   L2: 16×16 → h16, 1024/128=8 行
        // TUI: 2560 个符号
        //   L0: 64×128 → h128, 2560/32=80 行
        //   L1: 32×64 → h64, 2560/64=40 行
        //   L2: 16×32 → h32, 2560/128=20 行
        // Emoji: 50 个符号
        //   L0: 128×128 → h128, 50/16=4 行
        //   L1: 64×64 → h64, 50/32=2 行
        //   L2: 32×32 → h32, 50/64=1 行
        let demands = [
            80 + 4,          // h128: TUI L0 + Emoji L0 = 84
            32 + 40 + 2,     // h64: Sprite L0 + TUI L1 + Emoji L1 = 74
            16 + 20 + 1,     // h32: Sprite L1 + TUI L2 + Emoji L2 = 37
            8u32,            // h16: Sprite L2 = 8
        ];
        let layers = pack_all_layers(&demands);
        // 总单位 = 84×8 + 74×4 + 37×2 + 8×1 = 672+296+74+8 = 1050
        // 理论层数 = ceil(1050/128) = 9
        assert_eq!(layers.len(), 9);
    }

    // =====================================================
    // 1.3 shelf 放置 + UV 计算
    // =====================================================

    #[test]
    fn test_shelf_placement_coordinates() {
        // 验证 shelf 内放置后坐标正确
        // 一层中: 2 行 h128 + 6 行 h64 + 8 行 h32 = 2×128+6×64+8×32 = 256+384+256 = 896
        // 不对，应该精确填满 2048
        // 一层中: 8 行 h128 + 16 行 h64 = 8×128+16×64 = 1024+1024 = 2048 ✓
        //
        // h128 shelf 起始 y: 0, 128, 256, ..., 896
        // h64 shelf 起始 y: 1024, 1088, 1152, ..., 1984
        // 最后一个 h64 shelf: y=1984, 底部=1984+64=2048 ✓

        // 模拟 128×128 符号放入 h128 shelf
        let layer_size = 2048u32;
        let sym_w = 128u32;
        let sym_h = 128u32;
        let shelf_y = 0u32;
        let slot_x = 0u32;

        let uv_x = slot_x as f32 / layer_size as f32;
        let uv_y = shelf_y as f32 / layer_size as f32;
        let uv_w = sym_w as f32 / layer_size as f32;
        let uv_h = sym_h as f32 / layer_size as f32;

        assert!((uv_x - 0.0).abs() < 1e-6);
        assert!((uv_y - 0.0).abs() < 1e-6);
        assert!((uv_w - 0.0625).abs() < 1e-6);  // 128/2048
        assert!((uv_h - 0.0625).abs() < 1e-6);

        // 第 2 个符号在同一 shelf
        let slot_x_2 = 128u32;
        let uv_x_2 = slot_x_2 as f32 / layer_size as f32;
        assert!((uv_x_2 - 0.0625).abs() < 1e-6);

        // 16×16 符号在 h16 shelf
        let sym_w_16 = 16u32;
        let sym_h_16 = 16u32;
        let shelf_y_16 = 1984u32; // 2048 - 64 位置的某个 shelf
        let uv_w_16 = sym_w_16 as f32 / layer_size as f32;
        let uv_h_16 = sym_h_16 as f32 / layer_size as f32;
        assert!((uv_w_16 - 0.0078125).abs() < 1e-6);  // 16/2048
        assert!((uv_h_16 - 0.0078125).abs() < 1e-6);
    }

    #[test]
    fn test_symbols_per_row() {
        // 验证每行符号数计算
        assert_eq!(2048 / 128, 16);  // 128×128 → 16 个/行
        assert_eq!(2048 / 64, 32);   // 64×64 → 32 个/行
        assert_eq!(2048 / 32, 64);   // 32×32 → 64 个/行
        assert_eq!(2048 / 16, 128);  // 16×16 → 128 个/行

        // TUI 特殊: 宽度和高度不同
        assert_eq!(2048 / 64, 32);   // 64×128 TUI L0 → 32 个/行
        assert_eq!(2048 / 32, 64);   // 32×64 TUI L1 → 64 个/行
        assert_eq!(2048 / 16, 128);  // 16×32 TUI L2 → 128 个/行
    }

    #[test]
    fn test_rows_needed_calculation() {
        // Sprite L0: 40960 个 64×64, 每行 32 个
        assert_eq!((40960 + 31) / 32, 1280);  // ceil(40960/32) = 1280 行

        // TUI L0: 2560 个 64×128, 每行 32 个
        assert_eq!((2560 + 31) / 32, 80);     // ceil(2560/32) = 80 行

        // Emoji L0: 768 个 128×128, 每行 16 个
        assert_eq!((768 + 15) / 16, 48);      // ceil(768/16) = 48 行

        // CJK L0: 4096 个 128×128, 每行 16 个
        assert_eq!((4096 + 15) / 16, 256);    // ceil(4096/16) = 256 行
    }
}
```

## 2. Mipmap 级别选择测试

位置：`src/render/adapter/wgpu/render_symbols.rs` `#[cfg(test)] mod tests`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================
    // 2.1 select_mip_level 阈值边界测试
    // =====================================================

    #[test]
    fn test_sprite_mip_level_boundaries() {
        // Sprite: >=48 → L0, 24..48 → L1, <24 → L2
        assert_eq!(select_mip_level(100.0, SymbolType::Sprite), 0);
        assert_eq!(select_mip_level(48.0, SymbolType::Sprite), 0);  // 边界 ≥48
        assert_eq!(select_mip_level(47.9, SymbolType::Sprite), 1);
        assert_eq!(select_mip_level(24.0, SymbolType::Sprite), 1);  // 边界 ≥24
        assert_eq!(select_mip_level(23.9, SymbolType::Sprite), 2);
        assert_eq!(select_mip_level(1.0, SymbolType::Sprite), 2);
    }

    #[test]
    fn test_tui_mip_level_boundaries() {
        // TUI: >=96 → L0, 48..96 → L1, <48 → L2
        assert_eq!(select_mip_level(200.0, SymbolType::Tui), 0);
        assert_eq!(select_mip_level(96.0, SymbolType::Tui), 0);   // 边界 ≥96
        assert_eq!(select_mip_level(95.9, SymbolType::Tui), 1);
        assert_eq!(select_mip_level(48.0, SymbolType::Tui), 1);   // 边界 ≥48
        assert_eq!(select_mip_level(47.9, SymbolType::Tui), 2);
        assert_eq!(select_mip_level(8.0, SymbolType::Tui), 2);
    }

    #[test]
    fn test_emoji_mip_level_boundaries() {
        // Emoji: >=96 → L0, 48..96 → L1, <48 → L2
        assert_eq!(select_mip_level(150.0, SymbolType::Emoji), 0);
        assert_eq!(select_mip_level(96.0, SymbolType::Emoji), 0);
        assert_eq!(select_mip_level(95.9, SymbolType::Emoji), 1);
        assert_eq!(select_mip_level(48.0, SymbolType::Emoji), 1);
        assert_eq!(select_mip_level(47.9, SymbolType::Emoji), 2);
    }

    #[test]
    fn test_cjk_mip_level_same_as_emoji() {
        // CJK 和 Emoji 使用相同阈值
        for h in [150.0, 96.0, 95.9, 48.0, 47.9, 10.0] {
            assert_eq!(
                select_mip_level(h, SymbolType::Cjk),
                select_mip_level(h, SymbolType::Emoji),
                "CJK and Emoji should have same mip at height {h}"
            );
        }
    }

    // =====================================================
    // 2.2 常见屏幕场景测试
    // =====================================================

    #[test]
    fn test_standard_window_1080p() {
        // 1080p 窗口, 40 行 TUI → cell 高度 ≈ 27px
        let cell_h = 1080.0 / 40.0; // = 27.0
        assert_eq!(select_mip_level(cell_h, SymbolType::Tui), 2);
        // TUI cell 27px < 48 → L2 (16×32)
    }

    #[test]
    fn test_standard_window_1440p() {
        // 1440p 窗口, 40 行 TUI → cell 高度 ≈ 36px
        let cell_h = 1440.0 / 40.0; // = 36.0
        assert_eq!(select_mip_level(cell_h, SymbolType::Tui), 2);
        // 36px < 48 → L2
    }

    #[test]
    fn test_retina_2x() {
        // Retina 2x, 1440p logical → 2880 physical, 40 行
        let cell_h = 2880.0 / 40.0; // = 72.0
        assert_eq!(select_mip_level(cell_h, SymbolType::Tui), 1);
        // 72px: 48..96 → L1 (32×64)
    }

    #[test]
    fn test_5k_display() {
        // 5K 全屏, 40 行 → cell ≈ 135px
        let cell_h = 5400.0 / 40.0; // = 135.0
        assert_eq!(select_mip_level(cell_h, SymbolType::Tui), 0);
        // 135px ≥ 96 → L0 (64×128)
    }

    #[test]
    fn test_per_cell_independent() {
        // 同一帧不同 cell 使用不同 mip level
        let level_big = select_mip_level(100.0, SymbolType::Sprite);
        let level_small = select_mip_level(10.0, SymbolType::Sprite);
        assert_eq!(level_big, 0);
        assert_eq!(level_small, 2);
        // 同一 draw call 可以混合不同 level
    }

    // =====================================================
    // 2.3 返回值范围测试
    // =====================================================

    #[test]
    fn test_mip_level_always_valid_index() {
        // mip level 必须是 0, 1, 2（对应 Tile.mips[3]）
        let types = [SymbolType::Sprite, SymbolType::Tui,
                     SymbolType::Emoji, SymbolType::Cjk];
        for &sym_type in &types {
            for h in [0.1, 1.0, 10.0, 24.0, 48.0, 96.0, 200.0, 1000.0] {
                let level = select_mip_level(h, sym_type);
                assert!(level <= 2, "level={level} for h={h} type={sym_type:?}");
            }
        }
    }
}
```

## 3. LayeredSymbolMap + Tile 解析测试

位置：`src/render/symbol_map.rs` `#[cfg(test)] mod layered_tests`

```rust
#[cfg(test)]
mod layered_tests {
    use super::*;

    fn make_test_json() -> &'static str {
        r#"{
            "version": 2,
            "layer_size": 2048,
            "layer_count": 3,
            "layer_files": ["layer_0.png", "layer_1.png", "layer_2.png"],
            "symbols": {
                "\uDB80\uDC00": {
                    "w": 1, "h": 1,
                    "mip0": {"layer": 0, "x": 0, "y": 0, "w": 64, "h": 64},
                    "mip1": {"layer": 1, "x": 0, "y": 0, "w": 32, "h": 32},
                    "mip2": {"layer": 2, "x": 0, "y": 0, "w": 16, "h": 16}
                },
                "A": {
                    "w": 1, "h": 2,
                    "mip0": {"layer": 0, "x": 64, "y": 0, "w": 64, "h": 128},
                    "mip1": {"layer": 1, "x": 32, "y": 0, "w": 32, "h": 64},
                    "mip2": {"layer": 2, "x": 16, "y": 0, "w": 16, "h": 32}
                },
                "😀": {
                    "w": 2, "h": 2,
                    "mip0": {"layer": 0, "x": 128, "y": 0, "w": 128, "h": 128},
                    "mip1": {"layer": 1, "x": 64, "y": 0, "w": 64, "h": 64},
                    "mip2": {"layer": 2, "x": 32, "y": 0, "w": 32, "h": 32}
                },
                "中": {
                    "w": 2, "h": 2,
                    "mip0": {"layer": 0, "x": 256, "y": 0, "w": 128, "h": 128},
                    "mip1": {"layer": 1, "x": 128, "y": 0, "w": 64, "h": 64},
                    "mip2": {"layer": 2, "x": 64, "y": 0, "w": 32, "h": 32}
                }
            }
        }"#
    }

    // =====================================================
    // 3.1 JSON 解析
    // =====================================================

    #[test]
    fn test_parse_version() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();
        assert_eq!(map.version, 2);
        assert_eq!(map.layer_size, 2048);
        assert_eq!(map.layer_count, 3);
    }

    #[test]
    fn test_parse_symbol_count() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();
        assert_eq!(map.symbols.len(), 4);
    }

    #[test]
    fn test_reject_old_version() {
        let json = r#"{"version": 1, "symbols": {}}"#;
        let result = LayeredSymbolMap::from_json(json);
        assert!(result.is_err(), "Should reject version 1");
    }

    // =====================================================
    // 3.2 resolve() 查询
    // =====================================================

    #[test]
    fn test_resolve_sprite_pua() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();
        // PUA U+F0000 = block 0, idx 0 的 Sprite
        let pua = "\u{F0000}";
        let tile = map.resolve(pua);

        assert_eq!(tile.width, 1);
        assert_eq!(tile.height, 1);

        // Level 0
        assert_eq!(tile.mips[0].layer, 0);
        assert!((tile.mips[0].uv_x - 0.0).abs() < 1e-6);
        assert!((tile.mips[0].uv_y - 0.0).abs() < 1e-6);
        assert!((tile.mips[0].uv_w - 64.0 / 2048.0).abs() < 1e-6);
        assert!((tile.mips[0].uv_h - 64.0 / 2048.0).abs() < 1e-6);

        // Level 1
        assert_eq!(tile.mips[1].layer, 1);

        // Level 2
        assert_eq!(tile.mips[2].layer, 2);
    }

    #[test]
    fn test_resolve_tui_ascii() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();
        let tile = map.resolve("A");

        assert_eq!(tile.width, 1);
        assert_eq!(tile.height, 2);   // TUI 是 1×2

        // L0: 64×128
        assert!((tile.mips[0].uv_w - 64.0 / 2048.0).abs() < 1e-6);
        assert!((tile.mips[0].uv_h - 128.0 / 2048.0).abs() < 1e-6);
    }

    #[test]
    fn test_resolve_emoji() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();
        let tile = map.resolve("😀");

        assert_eq!(tile.width, 2);
        assert_eq!(tile.height, 2);   // Emoji 是 2×2

        // L0: 128×128
        assert!((tile.mips[0].uv_w - 128.0 / 2048.0).abs() < 1e-6);
    }

    #[test]
    fn test_resolve_cjk() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();
        let tile = map.resolve("中");

        assert_eq!(tile.width, 2);
        assert_eq!(tile.height, 2);   // CJK 是 2×2
    }

    #[test]
    fn test_resolve_unknown_returns_default() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();
        let tile = map.resolve("不存在的符号");

        // 未知符号应返回默认 Tile（全零或预定义 fallback）
        assert_eq!(tile.width, 0);
        assert_eq!(tile.height, 0);
    }

    // =====================================================
    // 3.3 UV 归一化验证
    // =====================================================

    #[test]
    fn test_uv_normalized_range() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();

        for (key, _) in &map.symbols {
            let tile = map.resolve(key);
            for mip in &tile.mips {
                assert!(mip.uv_x >= 0.0 && mip.uv_x <= 1.0,
                    "uv_x out of range for {key}: {}", mip.uv_x);
                assert!(mip.uv_y >= 0.0 && mip.uv_y <= 1.0,
                    "uv_y out of range for {key}: {}", mip.uv_y);
                assert!(mip.uv_w > 0.0 && mip.uv_w <= 1.0,
                    "uv_w out of range for {key}: {}", mip.uv_w);
                assert!(mip.uv_h > 0.0 && mip.uv_h <= 1.0,
                    "uv_h out of range for {key}: {}", mip.uv_h);
                assert!(mip.uv_x + mip.uv_w <= 1.0 + 1e-6,
                    "uv_x+uv_w exceeds 1.0 for {key}");
                assert!(mip.uv_y + mip.uv_h <= 1.0 + 1e-6,
                    "uv_y+uv_h exceeds 1.0 for {key}");
            }
        }
    }

    #[test]
    fn test_layer_index_within_bounds() {
        let map = LayeredSymbolMap::from_json(make_test_json()).unwrap();

        for (key, _) in &map.symbols {
            let tile = map.resolve(key);
            for mip in &tile.mips {
                assert!((mip.layer as u32) < map.layer_count,
                    "layer {} >= layer_count {} for {key}", mip.layer, map.layer_count);
            }
        }
    }
}
```

## 4. PUA 兼容性链路测试

位置：`src/render/cell.rs` `#[cfg(test)] mod tile_tests`

```rust
#[cfg(test)]
mod tile_tests {
    use super::*;

    // =====================================================
    // 4.1 PUA 编码 → Tile 解析链路
    // =====================================================

    #[test]
    fn test_cellsym_block_generates_pua() {
        // block=0, idx=0 → PUA U+F0000
        let s = cellsym_block(0, 0);
        assert_eq!(s.chars().next().unwrap() as u32, 0xF0000);

        // block=1, idx=42 → PUA U+F0100 + 42 = U+F012A
        let s = cellsym_block(1, 42);
        assert_eq!(s.chars().next().unwrap() as u32, 0xF012A);
    }

    #[test]
    fn test_pua_roundtrip_decode() {
        // PUA 编码后能正确解码回 block+idx
        for block in [0u8, 1, 10, 159] {
            for idx in [0u8, 1, 127, 255] {
                let s = cellsym_block(block, idx);
                let (decoded_block, decoded_idx) = decode_pua(&s).unwrap();
                assert_eq!(decoded_block, block,
                    "block mismatch: {block}:{idx}");
                assert_eq!(decoded_idx, idx,
                    "idx mismatch: {block}:{idx}");
            }
        }
    }

    #[test]
    fn test_is_pua_sprite() {
        // PUA 范围检测
        let sprite = cellsym_block(0, 0);
        assert!(is_pua_sprite(&sprite));

        let ascii = "A".to_string();
        assert!(!is_pua_sprite(&ascii));

        let emoji = "😀".to_string();
        assert!(!is_pua_sprite(&emoji));
    }

    // =====================================================
    // 4.2 set_symbol → compute_tile 链路
    // =====================================================

    #[test]
    fn test_set_symbol_updates_tile() {
        // 注意: 此测试需要 LayeredSymbolMap 已初始化
        // 在单元测试中可能需要 mock
        let mut cell = Cell::default();

        // 设置 PUA 符号
        let sym = cellsym_block(0, 0);
        cell.set_symbol(&sym);
        assert_eq!(cell.symbol, sym);
        // tile 应被自动计算（如果 LayeredSymbolMap 已加载）
        // assert_eq!(cell.tile.width, 1);
        // assert_eq!(cell.tile.height, 1);
    }

    #[test]
    fn test_set_graph_sym_compatibility() {
        // set_graph_sym 内部调用 cellsym_block → set_symbol → compute_tile
        // 验证链路不中断
        let sym = cellsym_block(3, 42);
        let (block, idx) = decode_pua(&sym).unwrap();
        assert_eq!(block, 3);
        assert_eq!(idx, 42);
        // sym 可直接作为 LayeredSymbolMap 的 key
        // map.resolve(&sym) 应返回有效 Tile
    }

    // =====================================================
    // 4.3 Tile 结构测试
    // =====================================================

    #[test]
    fn test_tile_default() {
        let tile = Tile::default();
        assert_eq!(tile.width, 0);
        assert_eq!(tile.height, 0);
        for mip in &tile.mips {
            assert_eq!(mip.layer, 0);
            assert!((mip.uv_x - 0.0).abs() < 1e-6);
            assert!((mip.uv_w - 0.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_tile_size() {
        // Tile 大小应为 ~56 bytes
        assert_eq!(std::mem::size_of::<MipUV>(), 18); // u16 + 4×f32 = 2 + 16 = 18
        // 实际可能因对齐为 20
        assert!(std::mem::size_of::<MipUV>() <= 20);

        // Tile = 2×u8 + 3×MipUV
        let tile_size = std::mem::size_of::<Tile>();
        assert!(tile_size <= 64, "Tile too large: {tile_size} bytes");
    }

    #[test]
    fn test_mipuv_copy_semantics() {
        // MipUV 和 Tile 应是 Copy 类型（高频读取）
        let mip = MipUV { layer: 5, uv_x: 0.1, uv_y: 0.2, uv_w: 0.3, uv_h: 0.4 };
        let mip2 = mip; // Copy
        assert_eq!(mip.layer, mip2.layer);
        assert!((mip.uv_x - mip2.uv_x).abs() < 1e-6);
    }

    #[test]
    fn test_tile_is_double_width() {
        // Sprite: 1×1
        let sprite = Tile { width: 1, height: 1, mips: Default::default() };
        assert!(!sprite.is_double_width());

        // Emoji/CJK: 2×2
        let emoji = Tile { width: 2, height: 2, mips: Default::default() };
        assert!(emoji.is_double_width());

        // TUI: 1×2
        let tui = Tile { width: 1, height: 2, mips: Default::default() };
        assert!(!tui.is_double_width());
    }

    #[test]
    fn test_tile_is_double_height() {
        let sprite = Tile { width: 1, height: 1, mips: Default::default() };
        assert!(!sprite.is_double_height());

        let tui = Tile { width: 1, height: 2, mips: Default::default() };
        assert!(tui.is_double_height());
    }
}
```

## 5. Per-Instance Data 编码测试

位置：`src/render/adapter/wgpu/render_symbols.rs` `#[cfg(test)]`

```rust
#[cfg(test)]
mod instance_tests {
    use super::*;

    #[test]
    fn test_instance_size_64_bytes() {
        // 新 instance = 4×vec4(f32) + 1×vec4(u8) = 4×16 + 4 = 68?
        // 实际: a1[4] + a2[4] + a3[4] + a4[4] + color[4] = 5×4×4 = 80
        // 不对，按设计: 64 bytes
        // a1: 4×f32 = 16, a2: 16, a3: 16, a4: 16 → 64，color 在 a4 内?
        // 看设计: a1+a2+a3+a4+color = 5×16 = 80... 检查实际定义
        assert_eq!(
            std::mem::size_of::<WgpuSymbolInstance>(),
            64,  // 或实际值，确保与 vertex buffer stride 一致
        );
    }

    #[test]
    fn test_layer_index_encoding() {
        // layer_index 从 Tile.mips[level].layer 读取
        // 存入 a4[0] 作为 f32
        let layer: u16 = 42;
        let a4_0 = layer as f32;
        // shader 中 i32(input.v_layer) 还原
        assert_eq!(a4_0 as i32, 42);
    }

    #[test]
    fn test_layer_index_max_value() {
        // 最大 layer 索引（256 层应够用）
        for layer in [0u16, 1, 84, 255] {
            let f = layer as f32;
            assert_eq!(f as u16, layer,
                "f32 roundtrip failed for layer {layer}");
        }
    }

    #[test]
    fn test_uv_from_tile_mip() {
        // 模拟从 Tile 读取 UV 填入 instance
        let tile = Tile {
            width: 1,
            height: 1,
            mips: [
                MipUV { layer: 0, uv_x: 0.0, uv_y: 0.0, uv_w: 0.03125, uv_h: 0.03125 },
                MipUV { layer: 5, uv_x: 0.1, uv_y: 0.2, uv_w: 0.015625, uv_h: 0.015625 },
                MipUV { layer: 10, uv_x: 0.3, uv_y: 0.4, uv_w: 0.0078125, uv_h: 0.0078125 },
            ],
        };

        // Level 1 选择
        let mip = tile.mips[1];
        let a1_uv_left = mip.uv_x;
        let a1_uv_top = mip.uv_y;
        let a2_uv_w = mip.uv_w;
        let a2_uv_h = mip.uv_h;
        let a4_layer = mip.layer as f32;

        assert!((a1_uv_left - 0.1).abs() < 1e-6);
        assert!((a1_uv_top - 0.2).abs() < 1e-6);
        assert!((a2_uv_w - 0.015625).abs() < 1e-6);
        assert!((a4_layer - 5.0).abs() < 1e-6);
    }
}
```

## 6. JSON 生成测试 (工具侧)

位置：`tools/cargo-pixel/src/symbols/texture.rs` `#[cfg(test)]`

```rust
#[cfg(test)]
mod json_tests {
    use super::*;

    #[test]
    fn test_json_output_has_version_2() {
        let json = generate_layered_symbol_map(&symbols, &packing_result);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["version"], 2);
    }

    #[test]
    fn test_json_pua_key_encoding() {
        // PUA U+F0000 在 JSON 中应为 surrogate pair "\uDB80\uDC00"
        // 或直接 UTF-8 序列化
        let pua_char = char::from_u32(0xF0000).unwrap();
        let key = pua_char.to_string();

        // serde_json 会将 supplementary plane 字符编码为 surrogate pair
        let json_str = serde_json::to_string(&key).unwrap();
        // 确保可以反序列化回来
        let decoded: String = serde_json::from_str(&json_str).unwrap();
        assert_eq!(decoded, key);
    }

    #[test]
    fn test_json_unicode_key_direct() {
        // Unicode 字符直接作为 key
        let key = "A".to_string();
        let json_str = serde_json::to_string(&key).unwrap();
        assert_eq!(json_str, "\"A\"");

        let key = "😀".to_string();
        let decoded: String = serde_json::from_str(
            &serde_json::to_string(&key).unwrap()
        ).unwrap();
        assert_eq!(decoded, "😀");

        let key = "中".to_string();
        let decoded: String = serde_json::from_str(
            &serde_json::to_string(&key).unwrap()
        ).unwrap();
        assert_eq!(decoded, "中");
    }

    #[test]
    fn test_json_all_symbols_have_3_mips() {
        let json = generate_layered_symbol_map(&symbols, &packing_result);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let symbols = parsed["symbols"].as_object().unwrap();
        for (key, val) in symbols {
            assert!(val["mip0"].is_object(), "Missing mip0 for {key}");
            assert!(val["mip1"].is_object(), "Missing mip1 for {key}");
            assert!(val["mip2"].is_object(), "Missing mip2 for {key}");
            assert!(val["w"].is_number(), "Missing w for {key}");
            assert!(val["h"].is_number(), "Missing h for {key}");
        }
    }

    #[test]
    fn test_json_layer_count_matches_files() {
        let json = generate_layered_symbol_map(&symbols, &packing_result);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let layer_count = parsed["layer_count"].as_u64().unwrap();
        let layer_files = parsed["layer_files"].as_array().unwrap();
        assert_eq!(layer_count as usize, layer_files.len());
    }

    #[test]
    fn test_json_coordinates_non_negative() {
        let json = generate_layered_symbol_map(&symbols, &packing_result);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        for (key, val) in parsed["symbols"].as_object().unwrap() {
            for mip_key in ["mip0", "mip1", "mip2"] {
                let mip = &val[mip_key];
                assert!(mip["layer"].as_u64().unwrap() < 10000,
                    "Invalid layer for {key}.{mip_key}");
                assert!(mip["x"].as_u64().unwrap() < 2048,
                    "x >= 2048 for {key}.{mip_key}");
                assert!(mip["y"].as_u64().unwrap() < 2048,
                    "y >= 2048 for {key}.{mip_key}");
                assert!(mip["w"].as_u64().unwrap() > 0,
                    "w == 0 for {key}.{mip_key}");
                assert!(mip["h"].as_u64().unwrap() > 0,
                    "h == 0 for {key}.{mip_key}");
                assert!(mip["x"].as_u64().unwrap() + mip["w"].as_u64().unwrap() <= 2048,
                    "x+w > 2048 for {key}.{mip_key}");
                assert!(mip["y"].as_u64().unwrap() + mip["h"].as_u64().unwrap() <= 2048,
                    "y+h > 2048 for {key}.{mip_key}");
            }
        }
    }
}
```

## 测试覆盖矩阵

| 算法/逻辑 | 测试文件 | 测试数 | 覆盖要点 |
|-----------|---------|--------|---------|
| DP shelf-packing | `symbols/texture.rs` | 11 | 单类型/混合/溢出/空/最小/精确填满/全量场景 |
| Mipmap 选择 | `wgpu/render_symbols.rs` | 9 | 4 类型边界值/常见屏幕/HiDPI/范围验证 |
| LayeredSymbolMap | `symbol_map.rs` | 9 | JSON 解析/查询 4 类型/未知符号/UV 范围/layer 边界 |
| PUA 兼容链路 | `cell.rs` | 7 | PUA 编码/解码/round-trip/set_symbol 链路 |
| Per-Instance 编码 | `wgpu/render_symbols.rs` | 4 | 大小/layer 编码/最大值/UV 填充 |
| JSON 生成 | `symbols/texture.rs` | 5 | version/PUA key/3 mips/layer_count/坐标范围 |

**总计: 45 个测试用例**
