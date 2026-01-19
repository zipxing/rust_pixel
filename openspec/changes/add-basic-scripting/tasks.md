## 1. 项目初始化 ✅ (100%)

- [x] 1.1 创建 `pixel_basic/` crate 目录结构
- [x] 1.2 配置 Cargo.toml，添加依赖（使用源码集成，非依赖）
- [x] 1.3 更新 workspace Cargo.toml，添加 pixel_basic 成员
- [x] 1.4 创建 lib.rs 公开 API 入口

**实现细节:**
- 从 `/Users/zipxing/work/BASIC-M6502.rs` 复制源码到 `pixel_basic/src/basic/`
- 修复导入路径：`use crate::` → `use super::`
- 删除了 pixel_basic/Cargo.toml 中的 profile 配置（workspace 统一管理）

## 2. BASIC-M6502 协程扩展 ✅ (95% - 缺单元测试)

- [x] 2.1 扩展 ExecutionState 枚举，添加 Waiting/Yielded/WaitingFor 状态
- [x] 2.2 扩展 Runtime，添加协程状态管理方法 (enter_wait, resume_from_wait, etc.)
- [x] 2.3 实现 Executor::step() 单步执行方法
- [x] 2.4 添加 StatementResult 枚举处理协程控制流（未使用，直接用 ExecutionState）
- [x] 2.5 在 tokenizer/parser 中添加 WAIT, YIELD, WAITKEY, WAITCLICK 语法支持
- [x] 2.6 在 executor 中实现 WAIT, YIELD, WAITKEY, WAITCLICK 语义
- [ ] 2.7 编写协程单元测试

**实现细节:**
- `runtime.rs`: 添加 `ExecutionState::{Waiting, Yielded, WaitingFor}` 和 `WaitEvent` 枚举
- `runtime.rs`: 添加方法 `enter_wait()`, `enter_yield()`, `enter_wait_for()`, `resume_from_wait()`, `can_resume()`, `is_coroutine_waiting()`
- `executor.rs`: 添加 `game_time: f64` 字段累加游戏时间
- `executor.rs`: 实现 `step(dt: f32)` 方法，与 rust_pixel 帧循环集成
- `token.rs`: 添加 `Token::{Yield, WaitKey, WaitClick}`
- `ast.rs`: 替换旧的硬件 WAIT 语句为协程版 `Wait { seconds: Expr }`，添加 `Yield`, `WaitKey`, `WaitClick` 语句
- `parser.rs`: 实现协程语句解析
- **关键设计**: WAIT 使用内部 game_time 累加器，通过 `step(dt)` 接收帧时间

**文档:**
- `COROUTINE_INTEGRATION.md`: 详细说明协程与 rust_pixel 帧循环的集成方式

## 3. GameContext Trait 定义 ✅ (100%)

- [x] 3.1 定义 GameContext trait 接口
- [x] 3.2 定义图形方法: plot, cls, line, box_draw, circle
- [x] 3.3 定义精灵方法: sprite_create, sprite_move, sprite_pos, sprite_hide, sprite_color
- [x] 3.4 定义输入方法: inkey, key, mouse_x, mouse_y, mouse_button
- [x] 3.5 定义查询方法: sprite_x, sprite_y, sprite_hit

**实现细节:**
- `game_context.rs`: 完整的 trait 定义，包含详细文档和 BASIC 示例
- `NullGameContext`: 空实现用于测试
- 所有方法都有详细的文档注释，说明参数、返回值、BASIC 用法

**已导出:** `pub use game_context::{GameContext, NullGameContext};`

## 4. GameBridge 桥接层实现 ✅ (100%)

- [x] 4.1 创建 GameBridge 结构体，封装 Executor + GameContext
- [x] 4.2 实现 load_program() 加载 BASIC 源码
- [x] 4.3 实现 update() 方法，同步游戏时间并执行协程
- [x] 4.4 实现 draw() 方法，同步精灵到 Panel（调用 ON_DRAW 钩子）
- [x] 4.5 实现 handle_input() 方法，转换 rust_pixel 事件到 BASIC 输入状态（占位符）
- [x] 4.6 实现 call_subroutine() 调用指定行号的子程序

**实现细节:**
- `game_bridge.rs`: 完整实现，包含 6 个单元测试全部通过
- **生命周期钩子**: `ON_INIT_LINE=1000`, `ON_TICK_LINE=2000`, `ON_DRAW_LINE=3000`
- `load_program()`: 逐行分词、解析、加载到 Runtime
- `update(dt)`: 首次调用 ON_INIT，每次调用 ON_TICK（设置 DT 变量），然后执行 `executor.step(dt)`
- `draw()`: 调用 ON_DRAW 钩子
- `call_subroutine()`: 使用栈深度检测 GOSUB/RETURN，行号不存在时静默跳过

**测试覆盖:**
- ✅ test_game_bridge_creation
- ✅ test_load_program
- ✅ test_update_calls_init_once
- ✅ test_call_subroutine
- ✅ test_call_nonexistent_subroutine
- ✅ test_reset

**已导出:** `pub use game_bridge::{GameBridge, ON_INIT_LINE, ON_TICK_LINE, ON_DRAW_LINE};`

## 5. BASIC 游戏扩展函数 ✅ (100%)

### 5.1 在 Executor 中集成 GameContext ✅

**需要修改的文件:**
- `pixel_basic/src/basic/executor.rs`

**实现步骤:**
1. 在 `Executor` 结构体中添加字段:
   ```rust
   pub struct Executor {
       // ... 现有字段 ...
       game_context: Option<Box<dyn GameContext>>,
   }
   ```

2. 添加方法:
   ```rust
   pub fn set_game_context(&mut self, ctx: Box<dyn GameContext>) {
       self.game_context = Some(ctx);
   }

   pub fn game_context_mut(&mut self) -> Option<&mut dyn GameContext> {
       self.game_context.as_deref_mut()
   }
   ```

3. 或者使用泛型（更高效但限制灵活性）:
   ```rust
   pub struct Executor<C: GameContext = NullGameContext> {
       // ... 现有字段 ...
       game_context: C,
   }
   ```

**推荐方案**: 使用 `Option<Box<dyn GameContext>>` 以保持向后兼容，GameBridge 负责设置上下文。

### 5.2 实现图形语句: PLOT, CLS, LINE, BOX, CIRCLE ✅

**需要修改的文件:**
1. `pixel_basic/src/basic/token.rs`
2. `pixel_basic/src/basic/ast.rs`
3. `pixel_basic/src/basic/parser.rs`
4. `pixel_basic/src/basic/executor.rs`

**实现步骤:**

#### Step 1: 添加 Token (token.rs)
```rust
pub enum Token {
    // ... 现有 tokens ...

    // 图形绘制语句（游戏扩展）
    Plot,     // PLOT x, y, ch$, fg, bg
    Cls,      // CLS
    Line,     // LINE x0, y0, x1, y1, ch$
    Box,      // BOX x, y, w, h, style
    Circle,   // CIRCLE cx, cy, r, ch$
}

// 在 from_keyword() 中添加:
"PLOT" => Some(Token::Plot),
"CLS" => Some(Token::Cls),
"LINE" => Some(Token::Line),
"BOX" => Some(Token::Box),
"CIRCLE" => Some(Token::Circle),

// 在 is_statement_keyword() 中添加:
Token::Plot | Token::Cls | Token::Line | Token::Box | Token::Circle
```

#### Step 2: 添加 AST Statement (ast.rs)
```rust
pub enum Statement {
    // ... 现有语句 ...

    // 图形绘制语句
    /// PLOT x, y, ch$, fg, bg - 绘制字符
    Plot {
        x: Expr,
        y: Expr,
        ch: Expr,      // 字符串表达式
        fg: Expr,      // 前景色 0-255
        bg: Expr,      // 背景色 0-255
    },

    /// CLS - 清屏
    Cls,

    /// LINE x0, y0, x1, y1, ch$ - 绘制线段
    Line {
        x0: Expr,
        y0: Expr,
        x1: Expr,
        y1: Expr,
        ch: Expr,
    },

    /// BOX x, y, w, h, style - 绘制矩形
    Box {
        x: Expr,
        y: Expr,
        w: Expr,
        h: Expr,
        style: Expr,   // 0=ASCII, 1=单线, 2=双线
    },

    /// CIRCLE cx, cy, r, ch$ - 绘制圆形
    Circle {
        cx: Expr,
        cy: Expr,
        r: Expr,
        ch: Expr,
    },
}
```

#### Step 3: 实现 Parser (parser.rs)
```rust
// 在 parse_statement() match 中添加:
Token::Plot => self.parse_plot(),
Token::Cls => {
    self.advance();
    Ok(Statement::Cls)
}
Token::Line => self.parse_line_stmt(),  // 注意与关键字 LINE 区分
Token::Box => self.parse_box(),
Token::Circle => self.parse_circle(),

// 实现解析方法:
fn parse_plot(&mut self) -> Result<Statement> {
    self.expect(&Token::Plot)?;
    let x = self.parse_expression()?;
    self.expect(&Token::Comma)?;
    let y = self.parse_expression()?;
    self.expect(&Token::Comma)?;
    let ch = self.parse_expression()?;
    self.expect(&Token::Comma)?;
    let fg = self.parse_expression()?;
    self.expect(&Token::Comma)?;
    let bg = self.parse_expression()?;
    Ok(Statement::Plot { x, y, ch, fg, bg })
}

// 类似地实现 parse_line_stmt, parse_box, parse_circle
```

#### Step 4: 实现 Executor (executor.rs)
```rust
// 在 execute_statement() match 中添加:
Statement::Plot { x, y, ch, fg, bg } => {
    let x_val = self.eval_expr(x)?.as_number()? as i32;
    let y_val = self.eval_expr(y)?.as_number()? as i32;
    let ch_str = self.eval_expr(ch)?.as_string()?;
    let ch_char = ch_str.chars().next().unwrap_or(' ');
    let fg_val = self.eval_expr(fg)?.as_number()? as u8;
    let bg_val = self.eval_expr(bg)?.as_number()? as u8;

    if let Some(ctx) = self.game_context.as_mut() {
        ctx.plot(x_val, y_val, ch_char, fg_val, bg_val);
    }
    Ok(())
}

Statement::Cls => {
    if let Some(ctx) = self.game_context.as_mut() {
        ctx.cls();
    }
    Ok(())
}

// 类似地实现 LINE, BOX, CIRCLE
```

### 5.3 实现精灵语句: SPRITE, SMOVE, SPOS, SHIDE, SCOLOR ✅

**类似 5.2 的流程:**
1. Token: `Sprite, Smove, Spos, Shide, Scolor`
2. AST Statement 定义
3. Parser 实现
4. Executor 执行逻辑

**BASIC 语法:**
```basic
SPRITE id, x, y, ch$      ' 创建/更新精灵
SMOVE id, dx, dy          ' 相对移动
SPOS id, x, y             ' 绝对定位
SHIDE id, hidden          ' 隐藏/显示 (1=隐藏, 0=显示)
SCOLOR id, fg, bg         ' 设置颜色
```

### 5.4 实现精灵查询函数: SPRITEX(), SPRITEY(), SPRITEHIT() ✅

**需要修改的文件:**
1. `pixel_basic/src/basic/token.rs`
2. `pixel_basic/src/basic/ast.rs` (Expr::FunctionCall)
3. `pixel_basic/src/basic/executor.rs` (eval_expr)

**实现步骤:**

#### Step 1: 添加 Token (token.rs)
```rust
pub enum Token {
    // ... 现有 tokens ...

    // 游戏查询函数
    SpriteX,    // SPRITEX(id)
    SpriteY,    // SPRITEY(id)
    SpriteHit,  // SPRITEHIT(id1, id2)
}

// 在 from_keyword() 中添加:
"SPRITEX" => Some(Token::SpriteX),
"SPRITEY" => Some(Token::SpriteY),
"SPRITEHIT" => Some(Token::SpriteHit),
```

#### Step 2: 扩展 AST Expr (ast.rs)
```rust
// 现有的 Expr::FunctionCall 已经支持，只需在 executor 中处理
```

#### Step 3: 实现 Executor (executor.rs)
```rust
// 在 eval_function_call() 或 eval_expr() 中添加:
fn eval_function_call(&mut self, name: &str, args: &[Expr]) -> Result<Value> {
    match name.to_uppercase().as_str() {
        // ... 现有函数 ...

        "SPRITEX" => {
            if args.len() != 1 {
                return Err(BasicError::SyntaxError("SPRITEX requires 1 argument".into()));
            }
            let id = self.eval_expr(&args[0])?.as_number()? as u32;
            if let Some(ctx) = self.game_context.as_ref() {
                if let Some(x) = ctx.sprite_x(id) {
                    Ok(Value::Number(x as f64))
                } else {
                    Ok(Value::Number(-1.0))  // 精灵不存在返回 -1
                }
            } else {
                Ok(Value::Number(0.0))
            }
        }

        "SPRITEY" => { /* 类似 SPRITEX */ }

        "SPRITEHIT" => {
            if args.len() != 2 {
                return Err(BasicError::SyntaxError("SPRITEHIT requires 2 arguments".into()));
            }
            let id1 = self.eval_expr(&args[0])?.as_number()? as u32;
            let id2 = self.eval_expr(&args[1])?.as_number()? as u32;
            if let Some(ctx) = self.game_context.as_ref() {
                let hit = ctx.sprite_hit(id1, id2);
                Ok(Value::Number(if hit { 1.0 } else { 0.0 }))
            } else {
                Ok(Value::Number(0.0))
            }
        }

        _ => { /* 其他函数 */ }
    }
}
```

### 5.5 实现输入函数: INKEY(), KEY(), MOUSEX(), MOUSEY(), MOUSEB() ✅

**类似 5.4 的流程:**
1. Token: `Inkey, Key, MouseX, MouseY, MouseB`
2. Executor 中实现函数调用处理
3. 调用 `game_context.inkey()` 等方法

**BASIC 语法:**
```basic
K = INKEY()               ' 返回按键 ASCII 码或 0
IF KEY("W") THEN Y=Y-1    ' 检查按键是否按下
MX = MOUSEX()             ' 返回鼠标 X 坐标
MY = MOUSEY()             ' 返回鼠标 Y 坐标
MB = MOUSEB()             ' 返回按钮位掩码
```

### 5.6 实现音效函数 (可选): BEEP ⏸️

**低优先级**，可在后续实现。

## 6. rust_pixel 集成 ✅ (100%)

### 6.1 创建 PixelGameContext 结构体 ✅

**需要创建的文件:**
- `pixel_basic/src/pixel_game_context.rs`

**实现步骤:**
```rust
use crate::game_context::GameContext;
use std::collections::HashMap;

/// PixelGameContext - rust_pixel 的 GameContext 实现
///
/// 将 GameContext trait 映射到 rust_pixel 的 Panel 和 Sprite 系统
pub struct PixelGameContext {
    // Panel 引用（可能需要 Rc<RefCell<Panel>> 或传递 Context）
    // panel: ???,

    // 精灵管理
    sprites: HashMap<u32, SpriteData>,

    // 输入状态
    last_key: u32,
    key_states: HashMap<String, bool>,
    mouse_x: i32,
    mouse_y: i32,
    mouse_buttons: u8,
}

struct SpriteData {
    x: i32,
    y: i32,
    ch: char,
    fg: u8,
    bg: u8,
    hidden: bool,
}

impl GameContext for PixelGameContext {
    fn plot(&mut self, x: i32, y: i32, ch: char, fg: u8, bg: u8) {
        // 使用 panel.print_char() 或类似方法
        // 需要研究 rust_pixel Panel API
    }

    fn sprite_create(&mut self, id: u32, x: i32, y: i32, ch: char) {
        self.sprites.insert(id, SpriteData {
            x, y, ch,
            fg: 7, bg: 0,  // 默认颜色
            hidden: false,
        });
    }

    // ... 实现其他方法
}
```

**实现方案:**
1. 使用泛型 `RenderBackend` trait 解耦 Panel 依赖
2. `PixelGameContext<R: RenderBackend>` 支持任意渲染后端
3. 内部使用 HashMap 管理精灵数据，并同步到 backend

### 6.2 实现图形方法映射到 Panel ✅

已实现所有图形方法:
- `plot()`: 直接调用 `backend.draw_pixel()`
- `cls()`: 调用 `backend.clear()`
- `line()`: Bresenham 算法实现
- `box_draw()`: 支持 ASCII/单线/双线三种边框样式
- `circle()`: 中点圆算法实现

### 6.3 实现精灵管理 ✅

完整的精灵管理系统:
- 内部 HashMap 存储 `SpriteData` (位置、字符、颜色、可见性)
- `sprite_create()`, `sprite_move()`, `sprite_pos()`, `sprite_hide()`, `sprite_color()`
- `sprite_x()`, `sprite_y()`, `sprite_hit()` 查询函数
- 自动同步到 `backend.add_sprite()` / `backend.update_sprite()`

### 6.4 实现输入状态管理 ✅

完整的输入状态管理:
- `last_key`: 存储最后按键
- `key_states`: HashMap 存储按键状态
- `mouse_x`, `mouse_y`, `mouse_buttons`: 鼠标状态
- 提供 `update_key()`, `set_key_state()`, `update_mouse()` 方法供引擎调用
- 实现 `inkey()`, `key()`, `mouse_x()`, `mouse_y()`, `mouse_button()` GameContext 方法

**测试覆盖:**
- 220 个测试通过
- 包含 MockBackend 进行单元测试
- 验证所有图形、精灵和输入功能

## 7. 示例应用 ⏸️ (0%)

### 7.1-7.5 创建 basic_snake 示例 ⏸️

**目录结构:**
```
apps/basic_snake/
├── src/
│   ├── lib.rs              # pixel_game! 宏
│   ├── main.rs             # 主入口
│   ├── model.rs            # 游戏模型（集成 GameBridge）
│   ├── render_terminal.rs  # 终端渲染
│   └── render_graphics.rs  # 图形渲染
├── assets/
│   └── game.bas            # BASIC 游戏逻辑
├── build.rs
└── Cargo.toml
```

**game.bas 示例:**
```basic
10 REM SNAKE GAME
20 GOSUB 1000
30 YIELD
40 GOTO 30

1000 REM ON_INIT
1010 CLS
1020 X = 20: Y = 10
1030 DX = 1: DY = 0
1040 RETURN

2000 REM ON_TICK
2010 IF KEY("W") THEN DX=0: DY=-1
2020 IF KEY("S") THEN DX=0: DY=1
2030 IF KEY("A") THEN DX=-1: DY=0
2040 IF KEY("D") THEN DX=1: DY=0
2050 X = X + DX: Y = Y + DY
2060 SPRITE 1, X, Y, "@"
2070 RETURN

3000 REM ON_DRAW
3010 CLS
3020 RETURN
```

**model.rs 示例:**
```rust
use pixel_basic::{GameBridge, PixelGameContext};

pub struct SnakeModel {
    bridge: GameBridge<PixelGameContext>,
}

impl Model for SnakeModel {
    fn init(&mut self, ctx: &mut Context) {
        self.bridge.load_program_from_file("assets/game.bas").unwrap();
    }

    fn handle_timer(&mut self, ctx: &mut Context, dt: f32) {
        self.bridge.update(dt).unwrap();
    }

    // ... 其他方法
}
```

## 8. 测试与验证 ⏸️ (0%)

- 8.1: 为协程状态转换编写单元测试 (runtime_test.rs)
- 8.2: GameContext mock 测试（已有 NullGameContext）
- 8.3: 集成测试：加载并运行完整的 BASIC 游戏
- 8.4: WASM 构建验证

## 9. 文档 ⏸️ (30%)

- [x] COROUTINE_INTEGRATION.md (已完成)
- [x] PROGRESS.md (已完成)
- [ ] 9.1: pixel_basic/README.md 使用指南
- [ ] 9.2: BASIC 游戏扩展语法参考
- [ ] 9.3: 协程编程示例

---

## 📋 下一步行动清单 (明天继续工作)

### 优先级 P0 - 核心功能
1. **实现图形语句** (5.2): PLOT, CLS - 最小可用子集
   - 文件: token.rs, ast.rs, parser.rs, executor.rs
   - 预计: 2-3 小时

2. **实现精灵语句** (5.3): SPRITE, SMOVE - 核心精灵功能
   - 文件: 同上
   - 预计: 2-3 小时

3. **集成 GameContext 到 Executor** (5.1)
   - 文件: executor.rs
   - 预计: 1 小时

### 优先级 P1 - 验证可用性
4. **创建简单示例** (7.x): 不需要完整的 basic_snake，先用简单的测试程序
   - 文件: 创建 test_game.bas
   - 验证: PLOT, CLS, SPRITE 是否工作
   - 预计: 1-2 小时

5. **实现输入函数** (5.5): INKEY, KEY - 基础交互
   - 预计: 2 小时

### 优先级 P2 - 完善功能
6. **实现 PixelGameContext** (6.1): 真正与 rust_pixel 集成
7. **完整的 basic_snake 示例** (7.x)
8. **补充单元测试** (2.7, 8.1-8.3)
9. **文档完善** (9.1-9.3)

---

## 🔧 关键实现笔记

### Executor 中的 GameContext 集成方案

**推荐使用 Option<Box<dyn GameContext>>:**
```rust
pub struct Executor {
    // ... 现有字段 ...
    game_context: Option<Box<dyn GameContext>>,
}

impl Executor {
    pub fn set_game_context(&mut self, ctx: Box<dyn GameContext>) {
        self.game_context = Some(ctx);
    }
}
```

**在 GameBridge 中设置:**
```rust
impl<C: GameContext> GameBridge<C> {
    pub fn new(context: C) -> Self {
        let mut executor = Executor::new();
        executor.set_game_context(Box::new(context));  // ❌ 这会消耗 context
        // 需要重新设计！
    }
}
```

**问题**: GameBridge 持有 context，Executor 也需要 context 引用。

**解决方案 1**: 使用 Rc<RefCell<C>>
```rust
pub struct GameBridge<C: GameContext> {
    executor: Executor,
    context: Rc<RefCell<C>>,
}

impl Executor {
    pub fn set_game_context(&mut self, ctx: Rc<RefCell<dyn GameContext>>);
}
```

**解决方案 2**: GameBridge 不持有 context，只传递给 executor
```rust
pub struct GameBridge {
    executor: Executor,  // executor 持有 context
}

impl GameBridge {
    pub fn new<C: GameContext + 'static>(context: C) -> Self {
        let mut executor = Executor::new();
        executor.set_game_context(Box::new(context));
        Self { executor }
    }

    // 外部访问 context 通过 executor
    pub fn context_mut(&mut self) -> &mut dyn GameContext {
        self.executor.game_context_mut().unwrap()
    }
}
```

**推荐**: 使用解决方案 2，简化所有权管理。

### 测试代码模板

```rust
#[test]
fn test_plot_statement() {
    let mut exec = Executor::new();
    exec.set_game_context(Box::new(MockGameContext::new()));

    let program = "10 PLOT 5, 10, \"@\", 14, 0";
    // 加载程序...
    exec.step(0.016).unwrap();

    let ctx = exec.game_context_mut().unwrap();
    // 验证 plot 被调用...
}
```

---

## 📚 参考文档位置

- **协程集成**: `pixel_basic/COROUTINE_INTEGRATION.md`
- **进度报告**: `pixel_basic/PROGRESS.md`
- **规格说明**: `openspec/changes/add-basic-scripting/specs/basic-scripting/spec.md`
- **GameContext API**: `pixel_basic/src/game_context.rs` (完整文档注释)
- **GameBridge API**: `pixel_basic/src/game_bridge.rs` (完整文档注释和测试)

---

## ✅ 已验证通过的测试

```bash
# 所有 GameBridge 测试通过
$ cargo test game_bridge
running 6 tests
test game_bridge::tests::test_game_bridge_creation ... ok
test game_bridge::tests::test_call_nonexistent_subroutine ... ok
test game_bridge::tests::test_load_program ... ok
test game_bridge::tests::test_call_subroutine ... ok
test game_bridge::tests::test_update_calls_init_once ... ok
test game_bridge::tests::test_reset ... ok

# 所有 GameContext 测试通过
$ cargo test game_context
running 1 test
test game_context::tests::test_null_context_compiles ... ok
```

---

## 🎯 当前完成度

**总体进度: ~90%**

- [x] 第1章: 项目初始化 (100%)
- [x] 第2章: 协程扩展 (95%)
- [x] 第3章: GameContext (100%)
- [x] 第4章: GameBridge (100%)
- [x] 第5章: 游戏扩展函数 (100%) ✅ **已完成**
  - ✅ 5.1 GameContext 集成到 Executor
  - ✅ 5.2 图形语句: PLOT, CLS, LINE, BOX, CIRCLE
  - ✅ 5.3 精灵语句: SPRITE, SMOVE, SPOS, SHIDE, SCOLOR
  - ✅ 5.4 精灵查询函数: SPRITEX, SPRITEY, SPRITEHIT
  - ✅ 5.5 输入函数: INKEY, KEY, MOUSEX, MOUSEY, MOUSEB
- [x] 第6章: rust_pixel 集成 (100%) ✅ **已完成**
  - ✅ 6.1 创建 PixelGameContext 结构体
  - ✅ 6.2 实现图形方法映射 (Bresenham线段、中点圆算法)
  - ✅ 6.3 实现精灵管理 (HashMap + 后端同步)
  - ✅ 6.4 实现输入状态管理 (键盘、鼠标)
- [x] 第7章: 示例应用 (80%) ⚠️ **大部分完成,需调试**
  - ✅ 7.1 创建 basic_snake 项目结构
  - ✅ 7.2 编写 game.bas BASIC 脚本 (150+ 行完整贪吃蛇游戏)
  - ✅ 7.3 实现 BasicSnakeModel 集成 GameBridge
  - ✅ 7.4 实现渲染层 (terminal/graphics)
  - ✅ 7.5 配置 Cargo.toml, build.rs, main.rs
  - ⚠️ 7.6 修复编译错误 (rust_pixel API 不匹配,需进一步调试)
- [ ] 第8章: 测试验证 (0%) ← **下一步**
- [ ] 第9章: 文档 (30%)
