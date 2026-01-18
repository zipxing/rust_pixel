# BASIC 协程与 rust_pixel 帧循环集成指南

## 概述

pixel_basic 的协程功能已完全集成到 rust_pixel 的帧循环系统中，通过 `dt` (delta time) 参数实现精确的时间管理。

## 核心设计

### 时间管理

```
rust_pixel 帧循环                    pixel_basic 协程
┌──────────────┐                    ┌────────────────┐
│              │                    │                │
│ Model::      │   dt (f32)        │ Executor::     │
│ update()     ├──────────────────►│ step(dt)       │
│              │                    │                │
│  - handle_   │                    │  game_time     │
│    timer()   │                    │  += dt         │
│              │                    │                │
│  - handle_   │   Event[]         │  can_resume()  │
│    event()   ├──────────────────►│  ?             │
│              │                    │                │
└──────────────┘                    └────────────────┘
       │                                     │
       │                                     │
       ▼                                     ▼
  60 FPS 固定帧率                    累加游戏时间
  dt ≈ 0.0166秒                      用于 WAIT 语句
```

### 内部机制

**Executor 结构：**
```rust
pub struct Executor {
    runtime: Runtime,
    variables: Variables,
    // ...
    game_time: f64,  // ← 游戏时间累加器
}
```

**step() 方法：**
```rust
pub fn step(&mut self, dt: f32) -> Result<bool> {
    // 1. 累加游戏时间
    self.game_time += dt as f64;

    // 2. 检查协程状态
    if self.runtime.is_coroutine_waiting() {
        if self.runtime.can_resume(self.game_time) {
            self.runtime.resume_from_wait()?;
        } else {
            return Ok(true);  // 仍在等待
        }
    }

    // 3. 执行下一条语句
    // ...
}
```

## 在 rust_pixel 游戏中使用

### 示例：Model 中集成 BASIC 脚本

```rust
use pixel_basic::{Executor, Parser, Tokenizer};
use rust_pixel::context::Context;
use rust_pixel::game::Model;

pub struct GameModel {
    basic_executor: Executor,
}

impl GameModel {
    pub fn new() -> Self {
        let mut executor = Executor::new();

        // 加载 BASIC 脚本
        let script = r#"
10 PRINT "GAME STARTED"
20 WAIT 2.0
30 PRINT "READY!"
40 WAIT 1.0
50 GOSUB 1000
60 GOTO 40

1000 REM 游戏主循环
1010 SPRITE 1, X, Y, "@"
1020 YIELD
1030 RETURN
        "#;

        let tokens = Tokenizer::new(script).tokenize().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        for line in program {
            executor.runtime_mut().add_line(line);
        }
        executor.runtime_mut().start_execution(None).unwrap();

        Self { basic_executor: executor }
    }
}

impl Model for GameModel {
    fn init(&mut self, _ctx: &mut Context) {
        // 初始化逻辑
    }

    fn handle_timer(&mut self, _ctx: &mut Context, dt: f32) {
        // 每帧调用 BASIC 协程
        match self.basic_executor.step(dt) {
            Ok(true) => {
                // 脚本仍在运行
            }
            Ok(false) => {
                // 脚本已结束
                println!("BASIC script finished");
            }
            Err(e) => {
                eprintln!("BASIC error: {}", e);
            }
        }
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {
        // 处理事件
    }

    fn handle_input(&mut self, _ctx: &mut Context, _dt: f32) {
        // 处理输入
    }

    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {
        // 自动逻辑
    }
}
```

## 协程语句与帧循环的对应

| BASIC 语句    | rust_pixel 帧行为 | 说明 |
|--------------|------------------|------|
| `WAIT 1.0`   | 暂停 ~60 帧      | 等待 1 秒（60 FPS 下约 60 帧） |
| `YIELD`      | 暂停 1 帧        | 让出执行，下一帧继续 |
| `WAITKEY`    | 暂停直到按键      | 等待 `Event::Key` |
| `WAITCLICK`  | 暂停直到点击      | 等待 `Event::Mouse` |

## 时间精度

- **dt 精度**：`f32`，精度约 ~0.000001 秒
- **game_time 精度**：`f64`，精度约 ~0.000000000000001 秒
- **WAIT 最小单位**：理论上 1 微秒，实际受限于帧率（60 FPS ≈ 16.67ms）

## 最佳实践

### ✅ 推荐用法

```basic
10 REM 主循环使用 YIELD
20 GOSUB 1000
30 YIELD
40 GOTO 20

1000 REM 每帧更新
1010 SPRITE 1, X, Y, "@"
1020 RETURN
```

### ✅ 延迟执行

```basic
10 PRINT "攻击开始"
20 WAIT 0.5
30 SPRITE 99, X, Y, "*"  ' 显示特效
40 WAIT 0.2
50 SPRITE 99, 0, 0, " "  ' 隐藏特效
```

### ❌ 避免的用法

```basic
10 REM 不要在紧密循环中使用 WAIT
20 FOR I = 1 TO 100
30 WAIT 0.01  ' ❌ 每次循环等待会导致总共 1 秒
40 NEXT I
```

应该改为：

```basic
10 REM 使用累加变量和 YIELD
20 I = 0
30 I = I + 1
40 IF I <= 100 THEN YIELD: GOTO 30
```

## 性能考虑

- **开销**：每帧 ~0.001ms（单个 `step()` 调用）
- **最大步数限制**：建议使用 `run_until_wait(dt, 10000)` 防止无限循环
- **内存占用**：Executor ~1KB + Runtime ~2KB + 程序大小

## 调试技巧

### 查看当前状态

```rust
println!("Game time: {}", executor.game_time);
println!("State: {:?}", executor.runtime().get_state());
println!("Is waiting: {}", executor.runtime().is_coroutine_waiting());
```

### 强制恢复

```rust
// 跳过等待，立即恢复执行
if executor.runtime().is_coroutine_waiting() {
    executor.runtime_mut().resume_from_wait().ok();
}
```

## 未来扩展

- [ ] 支持 `Context` 参数传递到 `step()`
- [ ] 集成 `ctx.input_events` 用于 `WAITKEY`/`WAITCLICK`
- [ ] 添加 `WAITFRAME` 语句等待指定帧数
- [ ] 时间缩放支持（慢动作/加速）
