## Why

Rust 对普通游戏开发者门槛较高，需要一种 AI 友好、易学的脚本语言来降低 rust_pixel 的使用门槛。BASIC 语言与 rust_pixel 的复古像素/PETSCII 美学高度契合，且项目已有一个功能完善的 Rust 实现（BASIC-M6502.rs）可直接集成。

## What Changes

- **新增 `pixel_basic` crate**：作为 rust_pixel 的可选依赖，提供 BASIC 脚本集成
- **GameBridge 桥接层**：连接 BASIC 解释器与 rust_pixel 的 Panel/Sprite/Event 系统
- **BASIC 游戏扩展语法**：
  - 图形绘制：`PLOT`, `LINE`, `BOX`, `CIRCLE`, `CLS`
  - 精灵系统：`SPRITE`, `SMOVE`, `SPOS`, `SHIDE`, `SCOLOR`, `SPRITEX()`, `SPRITEY()`, `SPRITEHIT()`
  - 输入处理：`INKEY()`, `KEY()`, `MOUSEX()`, `MOUSEY()`, `MOUSEB()`
  - **协程/Yield**：`WAIT`, `YIELD`, `WAITKEY`, `WAITCLICK` - 让脚本像写剧情一样自然
  - 动画辅助：`TWEEN` 补间动画
- **生命周期钩子**：`ON_INIT` (1000), `ON_TICK` (2000), `ON_DRAW` (3000) 子程序约定
- **示例应用**：`apps/basic_snake` 演示完整集成

## Impact

- Affected specs: 新增 `basic-scripting` capability
- Affected code:
  - 新增 `pixel_basic/` crate
  - 修改 `Cargo.toml` workspace 配置
  - 新增 `apps/basic_snake/` 示例
- Dependencies: 引入 `basic-m6502` crate（外部或 git submodule）
