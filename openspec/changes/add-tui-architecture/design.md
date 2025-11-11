## Context

rust_pixel 目前支持文本模式（终端）和图形模式（SDL/OpenGL/WGPU/WebGL），但在图形模式下缺乏对 TUI（Terminal User Interface）风格界面的良好支持。终端字符通常是瘦高的（8x16 像素），而图形模式使用的符号是矮胖的（8x8 像素）。这导致在图形模式下无法真实模拟终端 UI 的视觉效果。

**约束条件：**
- 必须保持文本模式完全向后兼容
- 必须保持单次 draw call 的高性能渲染
- 必须支持 TUI 和游戏精灵的混合渲染
- 使用统一的 2048x2048 符号纹理，包含 TUI 和 Sprite 两个区域

**相关方：**
- 游戏开发者：需要在图形模式下使用 TUI 界面
- UI 框架用户：需要正确的字符宽高比和鼠标交互
- 性能敏感应用：需要保持高效的渲染性能

## Goals / Non-Goals

**Goals:**
- 在图形模式下支持瘦高字符（8x16）的 TUI 渲染
- 提供清晰的 TUI 层和游戏精灵层分离
- 实现双坐标系统，正确处理 TUI 和游戏区域的鼠标事件
- TUI 层永远渲染在最上层，确保界面可见性
- 保持单次 draw call 的渲染性能
- 使用统一纹理简化纹理管理

**Non-Goals:**
- 不改变文本模式的任何行为
- 不引入复杂的窗口管理或布局系统
- 不支持可变宽度字符（如 CJK 全角字符的特殊处理）
- 不实现专业 GUI 框架的高级特性（如矢量绘制、富文本排版）

## Decisions

### Decision 1: 统一符号纹理与区域划分

**选择：** 使用统一的 2048x2048 `symbols.png` 纹理，上部256像素存储 TUI 符号（8x16），下部1792像素存储 Sprite 符号（8x8）

**布局规划：**
```
2048x2048 纹理布局：
┌────────────────────────────────────────┐
│ TUI 区域（行 0-255）                    │ 256px 高
│ - 16个水平区块                          │
│ - 每区块 16x16 字符，每字符 8x16 像素   │
│ - 总计 4096 个 TUI 字符                 │
├────────────────────────────────────────┤
│ Sprite 区域（行 256-2047）              │ 1792px 高
│ - 16列 × 14行 区块                      │
│ - 每区块 16x16 字符，每字符 8x8 像素    │
│ - 总计 57,344 个 Sprite 字符            │
└────────────────────────────────────────┘
```

**理由：**
- 单个纹理简化纹理管理，无需多个纹理绑定
- 区域明确分离，避免符号索引冲突
- 充分利用 2048x2048 纹理空间
- TUI 区域位于顶部，便于快速访问常用字符
- 保持高效的 GPU 纹理采样性能

**替代方案：**
- 独立 TUI 纹理文件 → 需要管理两个纹理，增加加载和绑定开销
- 运行时缩放 8x8 符号 → 视觉效果差，失真明显
- 更大的统一纹理 → 超出常见 GPU 纹理限制

### Decision 2: 双坐标系统

**选择：** `MouseEvent` 同时提供 `(column, row)` 和 `(column_tui, row_tui)` 两套坐标

**理由：**
- 职责分离：TUI 组件用 TUI 坐标，游戏对象用 Sprite 坐标
- 应用层自主选择，无需复杂的坐标转换逻辑
- 向后兼容：现有代码继续使用 `column/row`

**替代方案：**
- 单一坐标 + 区域判断 → 需要应用层维护区域映射，复杂度高
- 动态坐标类型 → 需要运行时类型检查，性能和易用性差

### Decision 3: TUI 层渲染顺序

**选择：** Main Buffer（TUI 层）在 `generate_render_buffer` 中最后添加到 `RenderCell` 数组

**理由：**
- GPU 按顺序渲染，后添加的在上层
- 确保 TUI 界面（如菜单、对话框）始终可见
- 无需修改 shader 或引入深度测试

**替代方案：**
- 使用 Z-index 或深度缓冲 → 增加渲染复杂度，违背简单原则
- 应用层控制渲染顺序 → 容易出错，不够健壮

### Decision 4: 符号尺寸配置

**选择：** 扩展为两套全局配置：
```rust
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();   // Sprite: 8
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();  // Sprite: 8
pub static PIXEL_TUI_WIDTH: OnceLock<f32> = OnceLock::new();   // TUI: 8
pub static PIXEL_TUI_HEIGHT: OnceLock<f32> = OnceLock::new();  // TUI: 16
```

**理由：**
- 清晰区分 TUI 和 Sprite 的符号尺寸
- 保持现有代码对 `PIXEL_SYM_*` 的使用不变
- 允许未来支持其他宽高比（如 2:3）

**替代方案：**
- 单一尺寸 + 缩放因子 → 不够直观，容易混淆
- 运行时查表 → 性能开销，不必要的复杂度

### Decision 5: 渲染管线集成

**选择：** 修改 `render_main_buffer` 使用 TUI 符号尺寸，但仍合并到统一的 `RenderCell` 数组

**理由：**
- 保持单次 draw call 的高性能
- 复用现有的实例化渲染管线
- 最小化 shader 修改（已支持可变尺寸的 `RenderCell.w/h`）

**替代方案：**
- 分离 TUI 和 Sprite 的 draw call → 性能下降，违背设计目标
- 使用多个 render pass → 过度设计，不符合简单原则

### Decision 6: Sprite 符号使用 Unicode 私有使用区

**选择：** 使用 U+E000~U+E0FF (Private Use Area) 作为 Sprite 符号的 Unicode 映射范围

**背景：**
之前版本使用数学符号区域 (U+2200~U+22FF) 作为 Sprite 符号的索引。这导致了一个问题：当用户在 TUI 模式下想显示真实的数学符号（如 ∀∃∈∞≈≤≥⊕⊗）时，这些字符会被错误地映射到 Sprite 纹理索引。

**新方案：**
```rust
// U+E000~U+E0FF: BMP Private Use Area
// UTF-8: 11101110 100000xx 10xxxxxx (0xEE 0x80~0x83 0x80~0xBF)

pub fn cellsym(idx: u8) -> String {
    let codepoint = 0xE000u32 + idx as u32;
    char::from_u32(codepoint).unwrap().to_string()
}

fn symidx(symbol: &String) -> u8 {
    let sbts = symbol.as_bytes();
    if sbts.len() == 3 && sbts[0] == 0xEE && (sbts[1] >> 2 == 0x20) {
        let idx = ((sbts[1] & 3) << 6) + (sbts[2] & 0x3f);
        return idx;
    }
    // fallback to CELL_SYM_MAP...
}
```

**理由：**
1. **永不冲突** - Unicode Private Use Area (PUA) 专门为应用程序自定义使用保留，Unicode 标准永远不会在此分配字符
2. **编码简单** - 仍然是 3 字节 UTF-8 编码，与之前方案一致，无性能损失
3. **容量充足** - BMP PUA 有 6400 个码点 (U+E000~U+F8FF)，当前只用 256 个 (U+E000~U+E0FF)，未来可扩展
4. **TUI 兼容性** - TUI 模式可以自由显示数学符号、箭头符号等标准 Unicode 字符，不会与 Sprite 索引冲突

**使用场景对比：**

| 场景 | 旧方案 (U+2200~U+22FF) | 新方案 (U+E000~U+E0FF) |
|------|----------------------|----------------------|
| Sprite 渲染 | ✅ 可以工作 | ✅ 可以工作 |
| TUI 显示数学公式 | ❌ 冲突，无法显示 | ✅ 正常显示 |
| TUI 显示箭头符号 | ✅ 可以显示 | ✅ 可以显示 |
| 符号集扩展 | ⚠️ 仅 256 个数学符号 | ✅ PUA 有 6400 个码点 |

**替代方案：**
- U+F0000~U+FFFFD (PUA-A): 4 字节 UTF-8，编码复杂，性能稍差
- U+100000~U+10FFFD (PUA-B): 4 字节 UTF-8，编码复杂，性能稍差

### Decision 7: 区域感知的符号索引计算（方案 C）

**选择：** 在渲染层分离处理 TUI 和 Sprite 区域的符号索引计算，不修改 Cell 数据结构

**核心思想：**
- Main Buffer 始终使用 TUI 区域（符号索引 0-4095）
- Pixel Sprites 始终使用 Sprite 区域（符号索引 4096-61439）
- 索引计算在 `render_helper_tui` 和 `render_helper` 中分别实现

**数据流程：**
```
Cell.symbol → symidx() → 0-255 (区块内索引)
Cell.tex    → 区块索引 (TUI: 0-15, Sprite: 0-223)
                ↓
     render_helper_tui / render_helper
                ↓
   TUI区域计算 / Sprite区域计算
     (索引 0-4095) / (索引 4096-61439)
                ↓
        纹理符号索引
                ↓
           RenderCell.texsym
```

**索引计算公式：**

TUI 区域（顶部 256 像素）：
```rust
// texidx: 0-15 (16个水平区块)
// symidx: 0-255 (区块内256个字符)
block_x = texidx % 16
char_x = symidx % 16
char_y = symidx / 16
pixel_x = block_x * 128 + char_x * 8
pixel_y = char_y * 16
符号索引 = (pixel_y / 16) * 256 + (pixel_x / 8)  // 0-4095
```

Sprite 区域（行 256-2047）：
```rust
// texidx: 0-223 (16×14个区块)
// symidx: 0-255 (区块内256个字符)
block_x = texidx % 16
block_y = texidx / 16
char_x = symidx % 16
char_y = symidx / 16
pixel_x = block_x * 128 + char_x * 8
pixel_y = 256 + block_y * 128 + char_y * 8
符号索引 = 4096 + ((pixel_y - 256) / 8) * 256 + (pixel_x / 8)  // 4096-61439
```

**理由：**
1. **最小侵入性**：无需修改 Cell 结构，保持向后兼容
2. **职责清晰**：区域逻辑集中在渲染层，应用层无感知
3. **性能优化**：符号索引计算 O(1) 复杂度，仅在渲染时执行
4. **易于维护**：区域差异处理逻辑集中在 render_helper 函数中
5. **自然分层**：Main Buffer 和 Pixel Sprites 本就是不同渲染层

**替代方案及弊端：**

**方案 A：扩展 Cell.tex 语义（使用高位标记区域）**
- ❌ 破坏 tex 字段原有语义
- ❌ 限制区块数量（最多 128 个）
- ❌ 需修改所有设置 tex 的代码

**方案 B：添加独立区域标识字段**
- ❌ 增加 Cell 内存占用
- ❌ 序列化/反序列化需要更新
- ❌ 所有创建 Cell 的代码需要设置 region

**方案 C 的优势（当前选择）：**
- ✅ 零内存开销：不修改 Cell 结构
- ✅ 零迁移成本：现有代码完全兼容
- ✅ 自动区域识别：渲染层自动计算正确索引
- ✅ 性能无损：索引计算仅在渲染时执行

## Risks / Trade-offs

### Risk 1: 符号纹理资源增加

**风险：** 新增 `symbols_tui.png` 增加约 256KB 资源大小

**缓解措施：**
- 按需加载：仅在启用 TUI 模式时加载
- 使用压缩纹理格式（如 PNG 压缩）
- 对于不使用 TUI 的应用，无额外开销

### Risk 2: 鼠标坐标计算复杂度

**风险：** 双坐标计算可能引入性能开销或精度问题

**缓解措施：**
- 坐标转换是简单的除法运算，开销可忽略
- 在输入事件层一次性计算，后续无额外开销
- 添加单元测试验证坐标精度

### Risk 3: 向后兼容性

**风险：** 现有应用可能受到 `MouseEvent` 结构变化影响

**缓解措施：**
- 保留原有 `column/row` 字段，现有代码无需修改
- 新增字段使用默认值（与 `column/row` 相同）
- 添加配置选项，默认禁用 TUI 模式

## Migration Plan

### Phase 1: 基础设施（不影响现有应用）
1. 添加 `PIXEL_TUI_WIDTH/HEIGHT` 全局配置
2. 扩展 `MouseEvent` 结构（向后兼容）
3. 实现双坐标转换逻辑

### Phase 2: TUI 渲染支持
1. 创建 `symbols_tui.png` 资源
2. 修改 `render_main_buffer` 支持 TUI 符号
3. 调整渲染顺序确保 TUI 在上层

### Phase 3: 应用集成
1. 更新 UI 组件使用 TUI 坐标
2. 在 `ui_demo` 中验证
3. 提供配置选项和文档

### Rollback Plan
- Phase 1 可随时回滚（仅添加代码，未修改行为）
- Phase 2 需要移除 TUI 符号加载逻辑
- Phase 3 需要恢复 UI 组件的坐标使用

## Open Questions

1. **TUI 符号纹理内容：** 是否需要为 TUI 专门设计字符集，还是复用现有符号？
   - **建议：** 初期复用现有符号，后续根据需要优化

2. **混合渲染性能：** 在大量 TUI 元素和游戏精灵混合时，单次 draw call 是否仍然高效？
   - **建议：** 在 `ui_demo` 中添加压力测试场景

3. **多分辨率支持：** 不同 DPI 下，8x16 的 TUI 字符是否需要特殊处理？
   - **建议：** 复用现有的 `ratio_x/ratio_y` 缩放机制

4. **TUI 模式配置：** TUI 模式总是启用，无需配置开关。
   - **决定：** TUI 架构是核心渲染模式，始终支持混合渲染（TUI + Sprites）
   - **理由：** 简化架构，避免配置复杂度；应用可自由选择是否使用 Main Buffer（TUI）或仅使用 Pixel Sprites

