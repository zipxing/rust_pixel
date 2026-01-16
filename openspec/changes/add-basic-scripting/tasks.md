## 1. 项目初始化

- [ ] 1.1 创建 `pixel_basic/` crate 目录结构
- [ ] 1.2 配置 Cargo.toml，添加 basic-m6502 依赖
- [ ] 1.3 更新 workspace Cargo.toml，添加 pixel_basic 成员
- [ ] 1.4 创建 lib.rs 公开 API 入口

## 2. BASIC-M6502 协程扩展

- [ ] 2.1 扩展 ExecutionState 枚举，添加 Waiting/Yielded/WaitingFor 状态
- [ ] 2.2 扩展 Runtime，添加协程状态管理方法 (enter_wait, resume_from_wait, etc.)
- [ ] 2.3 实现 Executor::step() 单步执行方法
- [ ] 2.4 添加 StatementResult 枚举处理协程控制流
- [ ] 2.5 在 tokenizer/parser 中添加 WAIT, YIELD, WAITKEY 语法支持
- [ ] 2.6 在 executor 中实现 WAIT, YIELD, WAITKEY 语义
- [ ] 2.7 编写协程单元测试

## 3. GameContext Trait 定义

- [ ] 3.1 定义 GameContext trait 接口
- [ ] 3.2 定义图形方法: plot, cls, line, box_draw, circle
- [ ] 3.3 定义精灵方法: sprite_create, sprite_move, sprite_pos, sprite_hide, sprite_color
- [ ] 3.4 定义输入方法: inkey, key, mouse_x, mouse_y, mouse_button
- [ ] 3.5 定义查询方法: sprite_x, sprite_y, sprite_hit

## 4. GameBridge 桥接层实现

- [ ] 4.1 创建 GameBridge 结构体，封装 Executor + GameContext
- [ ] 4.2 实现 load_program() 加载 BASIC 源码
- [ ] 4.3 实现 update() 方法，同步游戏时间并执行协程
- [ ] 4.4 实现 draw() 方法，同步精灵到 Panel
- [ ] 4.5 实现 handle_input() 方法，转换 rust_pixel 事件到 BASIC 输入状态
- [ ] 4.6 实现 call_subroutine() 调用指定行号的子程序

## 5. BASIC 游戏扩展函数

- [ ] 5.1 在 Executor 中添加 game_context 字段
- [ ] 5.2 实现图形函数: PLOT, CLS, LINE, BOX, CIRCLE
- [ ] 5.3 实现精灵函数: SPRITE, SMOVE, SPOS, SHIDE, SCOLOR
- [ ] 5.4 实现精灵查询函数: SPRITEX(), SPRITEY(), SPRITEHIT()
- [ ] 5.5 实现输入函数: INKEY(), KEY(), MOUSEX(), MOUSEY(), MOUSEB()
- [ ] 5.6 实现音效函数 (可选): BEEP

## 6. rust_pixel 集成

- [ ] 6.1 创建 PixelGameContext 结构体，实现 GameContext trait
- [ ] 6.2 将 Panel/Sprite 操作映射到 GameContext 方法
- [ ] 6.3 将 rust_pixel Event 转换为 BASIC 输入状态
- [ ] 6.4 实现精灵管理 HashMap，支持按 ID 创建/更新/查询

## 7. 示例应用

- [ ] 7.1 创建 apps/basic_snake/ 目录结构
- [ ] 7.2 编写 game.bas BASIC 游戏逻辑（使用协程）
- [ ] 7.3 编写 main.rs Rust 启动代码
- [ ] 7.4 验证终端模式运行
- [ ] 7.5 验证 SDL/图形模式运行

## 8. 测试与验证

- [ ] 8.1 单元测试: 协程状态转换
- [ ] 8.2 单元测试: GameContext mock 测试
- [ ] 8.3 集成测试: 加载并运行示例 BASIC 程序
- [ ] 8.4 WASM 构建验证 (如支持)

## 9. 文档

- [ ] 9.1 编写 pixel_basic/README.md 使用指南
- [ ] 9.2 编写 BASIC 游戏扩展语法参考
- [ ] 9.3 添加协程编程示例（对话、动画、Boss 攻击模式）
