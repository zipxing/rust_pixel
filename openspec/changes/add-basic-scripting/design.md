## Context

rust_pixel 是一个面向复古像素风格的 2D 游戏引擎，支持 PETSCII 字符渲染。项目需要一种脚本语言来降低开发门槛，让非 Rust 开发者也能快速创建游戏。

**约束条件：**
- 必须支持 WASM 编译（Web 平台）
- 必须与复古美学一致
- 必须对 AI 代码生成友好
- 必须支持协程/yield 语义，避免在 Rust 端编写复杂状态机

**利益相关者：**
- 游戏开发者（主要用户）
- AI 编程助手（代码生成）
- rust_pixel 维护者

## Goals / Non-Goals

**Goals:**
- 提供简单易学的脚本接口，覆盖 90% 的 2D 游戏开发需求
- 支持协程式编程，让游戏逻辑像写剧情一样自然
- 与 rust_pixel 渲染系统无缝集成
- 保持 WASM 兼容性

**Non-Goals:**
- 不追求脚本执行性能（游戏逻辑通常不是瓶颈）
- 不替代 Rust 进行底层系统开发
- 不实现完整的 BASIC 标准库（只实现游戏相关扩展）

## Decisions

### Decision 1: 选择 BASIC 而非 Lua/Python

**选择：** 使用现有的 BASIC-M6502.rs 解释器

**理由：**
- 美学一致：BASIC + PETSCII 是经典 C64 组合
- 开发成本低：已有完善的 Rust 实现（194/195 测试通过）
- AI 友好：所有主流 LLM 都熟悉 BASIC 语法
- 独特卖点：市面上没有 BASIC 驱动的现代像素游戏引擎

**备选方案：**
- Lua (mlua)：更强大但缺乏复古特色
- Rhai：纯 Rust 但 AI 了解较少
- Python (PyO3)：太重，WASM 支持差

### Decision 2: 协程实现方案

**选择：** 扩展 ExecutionState 枚举 + step() 单步执行

**理由：**
- 最小改动：复用现有 Runtime 的 Paused 状态机制
- 游戏友好：每帧调用 step()，自然支持 YIELD
- 易于理解：状态转换清晰可追踪

**实现方式：**
```rust
pub enum ExecutionState {
    // 现有状态
    NotRunning, Running, Ended, Paused { line, stmt },

    // 新增协程状态
    Waiting { line, stmt, resume_at: f64 },     // WAIT 秒数
    Yielded { line, stmt },                      // YIELD 下一帧
    WaitingFor { line, stmt, condition },        // WAITKEY 等
}
```

**备选方案：**
- Rust async/await：过度复杂，难以与游戏循环集成
- 独立协程库：引入额外依赖

### Decision 3: 桥接层架构

**选择：** GameBridge 结构体封装 Executor + 游戏状态

```
┌─────────────────────────────────────────┐
│            rust_pixel Game               │
│  ┌───────────────────────────────────┐  │
│  │         GameBridge                 │  │
│  │  ┌─────────────┬───────────────┐  │  │
│  │  │  Executor   │  GameContext  │  │  │
│  │  │  (BASIC)    │  (sprites,    │  │  │
│  │  │             │   input, etc) │  │  │
│  │  └─────────────┴───────────────┘  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

**理由：**
- 解耦：BASIC 解释器不依赖 rust_pixel
- 可测试：GameContext 可以 mock
- 扩展性：易于添加新的游戏函数

### Decision 4: 游戏函数扩展方式

**选择：** 在 Executor 中注入 GameContext trait object

```rust
pub trait GameContext {
    fn plot(&mut self, x: u16, y: u16, sym: &str, fg: u8, bg: u8);
    fn sprite(&mut self, id: u32, x: f64, y: f64, sym: &str);
    fn inkey(&self) -> u8;
    fn key(&self, name: &str) -> bool;
    // ...
}
```

**理由：**
- 接口清晰：所有游戏交互通过 trait 定义
- 可替换：测试时可注入 mock 实现
- 类型安全：Rust 编译器保证接口正确性

## Risks / Trade-offs

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| BASIC 功能不足 | 复杂游戏难以实现 | 提供 Rust 扩展点，关键逻辑可用 Rust 写 |
| 协程调试困难 | 开发体验差 | 添加 TRACE 语句，记录协程状态变化 |
| 性能问题 | 复杂脚本卡顿 | 限制每帧执行步数，超时自动 yield |
| WASM 兼容性 | Web 端无法运行 | 持续测试 WASM 构建，避免使用不兼容 API |

## Migration Plan

1. **Phase 1: 核心集成**
   - 创建 `pixel_basic` crate
   - 实现 GameBridge 基础框架
   - 添加图形绘制函数 (PLOT, CLS)

2. **Phase 2: 协程系统**
   - 扩展 ExecutionState
   - 实现 WAIT, YIELD, WAITKEY
   - 添加 step() 单步执行

3. **Phase 3: 精灵与输入**
   - 实现精灵系统 (SPRITE, SMOVE, etc.)
   - 实现输入函数 (INKEY, KEY, MOUSE*)
   - 碰撞检测 (SPRITEHIT)

4. **Phase 4: 示例与文档**
   - 创建 basic_snake 示例
   - 编写使用文档
   - WASM 构建验证

**回滚策略：** pixel_basic 是独立 crate，可随时移除而不影响核心引擎

## Open Questions

1. **BASIC-M6502.rs 引入方式？**
   - 选项 A：Git submodule
   - 选项 B：发布到 crates.io 作为依赖
   - 选项 C：直接复制源码（不推荐）

2. **是否支持多协程（SPAWN）？**
   - 初期可只支持单协程，后续按需扩展

3. **错误处理策略？**
   - BASIC 运行时错误是否中断游戏？
   - 是否提供 ON ERROR GOTO 机制？
