/// GameBridge - BASIC 执行器与游戏引擎的桥接层
///
/// 此模块提供高层接口，将 BASIC 解释器与 rust_pixel 游戏引擎集成。
///
/// # 架构
///
/// ```text
/// rust_pixel Game Loop          GameBridge          BASIC Executor
/// ┌───────────────┐              ┌────────┐          ┌──────────┐
/// │ Model::init() │─────────────►│ new()  │─────────►│ Runtime  │
/// │               │              │        │          │          │
/// │ Model::       │  dt (f32)   │ update │   dt     │ step()   │
/// │ handle_timer()│─────────────►│ ()     │─────────►│          │
/// │               │              │        │          │          │
/// │ Render::      │              │ draw() │          │ commands │
/// │ draw()        │◄─────────────│        │◄─────────│ sprites  │
/// └───────────────┘              └────────┘          └──────────┘
/// ```
///
/// # 生命周期钩子
///
/// GameBridge 自动调用 BASIC 程序中的特定子程序作为生命周期钩子:
///
/// - **ON_INIT (1000行)**: 游戏启动时调用一次
/// - **ON_TICK (2000行)**: 每帧调用，用于游戏逻辑更新，DT 变量设置为帧时间
/// - **ON_DRAW (3500行)**: 每帧调用，用于渲染操作
///
/// # BASIC 程序示例
///
/// ```basic
/// 10 REM 游戏主程序
/// 20 GOSUB 1000   ' 初始化
/// 30 YIELD        ' 让出控制权到游戏循环
/// 40 GOTO 30
///
/// 1000 REM ON_INIT - 初始化
/// 1010 CLS
/// 1020 X = 20: Y = 10
/// 1030 RETURN
///
/// 2000 REM ON_TICK - 每帧逻辑
/// 2010 IF KEY("W") THEN Y=Y-1
/// 2020 IF KEY("S") THEN Y=Y+1
/// 2030 IF KEY("A") THEN X=X-1
/// 2040 IF KEY("D") THEN X=X+1
/// 2050 RETURN
///
/// 3500 REM ON_DRAW - 渲染
/// 3510 CLS
/// 3520 PLOT X, Y, "@", 15, 0
/// 3530 RETURN
/// ```

use crate::basic::{
    error::{BasicError, Result},
    tokenizer::Tokenizer,
    parser::Parser,
    executor::Executor,
};
use crate::game_context::GameContext;
use crate::pixel_game_context::PixelGameContext;
use log;

/// 生命周期钩子的行号常量
pub const ON_INIT_LINE: u16 = 1000;  // 初始化钩子
pub const ON_TICK_LINE: u16 = 2000;  // 每帧逻辑钩子
pub const ON_DRAW_LINE: u16 = 3500;  // 渲染钩子

/// GameBridge - BASIC 与游戏引擎的桥接
///
/// # 设计
///
/// GameBridge 内部持有一个 `PixelGameContext`，用于收集 BASIC 脚本的绘制命令。
/// 在每帧渲染时，外部代码可以通过 `context()` 获取这些命令并应用到 Panel。
///
/// # 示例
///
/// ```no_run
/// use pixel_basic::{GameBridge, DrawCommand};
///
/// let mut bridge = GameBridge::new();
///
/// // 加载 BASIC 程序
/// let program = r#"
/// 10 PRINT "HELLO WORLD"
/// 20 END
/// "#;
/// bridge.load_program(program).unwrap();
///
/// // 游戏循环中调用
/// loop {
///     let dt = 0.016; // 16ms per frame (60 FPS)
///     if !bridge.update(dt).unwrap() {
///         break; // 程序结束
///     }
///
///     // 获取绘制命令并应用到 Panel
///     for cmd in bridge.context_mut().drain_commands() {
///         match cmd {
///             DrawCommand::Plot { x, y, ch, fg, bg } => {
///                 // sprite.set_color_str(x, y, ch, fg, bg);
///             }
///             DrawCommand::Clear => {
///                 // clear sprite
///             }
///         }
///     }
/// }
/// ```
pub struct GameBridge {
    /// BASIC 执行器
    executor: Executor,

    /// 游戏上下文（收集绘制命令和输入状态）
    context: PixelGameContext,

    /// 是否已调用 ON_INIT 钩子
    init_called: bool,
}

impl Default for GameBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl GameBridge {
    /// 创建新的 GameBridge 实例
    ///
    /// # 返回值
    ///
    /// 返回新的 GameBridge 实例，内部 BASIC 解释器已初始化
    pub fn new() -> Self {
        Self {
            executor: Executor::new(),
            context: PixelGameContext::new(),
            init_called: false,
        }
    }

    /// 将 context 临时借给 executor 使用
    ///
    /// 使用 unsafe 但确保正确管理生命周期
    unsafe fn lend_context_to_executor(&mut self) {
        let ctx_ptr = &mut self.context as *mut PixelGameContext as *mut dyn GameContext;
        self.executor.set_game_context(Box::from_raw(ctx_ptr));
    }

    /// 从 executor 收回 context 的"借用"
    fn reclaim_context_from_executor(&mut self) {
        // 从 executor 取出 box 并转回指针，但不 drop
        if self.executor.has_game_context() {
            if let Some(boxed) = self.executor.game_context_mut() {
                let replacement: Box<dyn GameContext> = Box::new(crate::game_context::NullGameContext);
                let ptr = Box::into_raw(std::mem::replace(boxed, replacement));
                // 不要 drop 这个指针，因为它指向 self.context
                let _ = ptr;  // 忽略指针，避免 drop
            }
        }
    }

    /// 加载 BASIC 程序
    ///
    /// 解析 BASIC 源码并加载到解释器中。程序将在下次调用 `update()` 时开始执行。
    ///
    /// # 参数
    ///
    /// - `source`: BASIC 源代码字符串
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 加载成功
    /// - `Err(BasicError)`: 语法错误或其他加载错误
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use pixel_basic::GameBridge;
    /// let mut bridge = GameBridge::new();
    /// bridge.load_program("10 PRINT \"HELLO\"").unwrap();
    /// ```
    pub fn load_program(&mut self, source: &str) -> Result<()> {
        // 逐行处理源代码
        for line_text in source.lines() {
            let line_text = line_text.trim();
            if line_text.is_empty() {
                continue;
            }

            // 1. 分词
            let mut tokenizer = Tokenizer::new(line_text);
            let tokens = tokenizer.tokenize_line()?;

            // 2. 解析
            let mut parser = Parser::new(tokens);
            if let Some(program_line) = parser.parse_line()? {
                // 3. 加载到执行器
                self.executor.runtime_mut().add_line(program_line);
            }
        }

        // 4. 开始执行（从第一行开始）
        self.executor.runtime_mut().start_execution(None)?;

        // 5. 重置初始化标志
        self.init_called = false;

        Ok(())
    }

    /// 从文件加载 BASIC 程序
    ///
    /// # 参数
    ///
    /// - `path`: .bas 文件路径
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 加载成功
    /// - `Err(BasicError)`: 文件读取错误或语法错误
    pub fn load_program_from_file(&mut self, path: &str) -> Result<()> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| BasicError::SyntaxError(format!("Failed to read file {}: {}", path, e)))?;
        self.load_program(&source)
    }

    /// 更新游戏状态（每帧调用）
    ///
    /// 此方法应在游戏主循环的每一帧调用，负责:
    /// 1. 调用 ON_INIT 钩子（首次调用时）
    /// 2. 调用 ON_TICK 钩子（设置 DT 变量）
    /// 3. 执行协程（通过 `executor.step(dt)`）
    ///
    /// # 参数
    ///
    /// - `dt`: 距离上一帧的时间间隔（秒），通常为 ~0.016 (60 FPS)
    ///
    /// # 返回值
    ///
    /// - `Ok(true)`: 程序仍在运行
    /// - `Ok(false)`: 程序已结束
    /// - `Err(BasicError)`: 执行错误
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use pixel_basic::GameBridge;
    /// # let mut bridge = GameBridge::new();
    /// loop {
    ///     let dt = 0.016;
    ///     if !bridge.update(dt).unwrap() {
    ///         break; // 程序结束
    ///     }
    /// }
    /// ```
    pub fn update(&mut self, dt: f32) -> Result<bool> {
        // 设置 context 用于整个 update 期间
        unsafe { self.lend_context_to_executor(); }

        // 1. 首次调用时执行 ON_INIT 钩子
        if !self.init_called {
            log::info!("GameBridge: Calling ON_INIT (line {})", ON_INIT_LINE);
            self.call_subroutine_internal(ON_INIT_LINE)?;
            self.init_called = true;
            log::info!("GameBridge: ON_INIT completed");
        }

        // 2. 调用 ON_TICK 钩子（设置 DT 变量）
        log::debug!("GameBridge: Calling ON_TICK (line {}), dt={}", ON_TICK_LINE, dt);
        self.executor.variables_mut().set("DT", crate::basic::variables::Value::Number(dt as f64))?;
        self.call_subroutine_internal(ON_TICK_LINE)?;
        log::debug!("GameBridge: ON_TICK completed");

        // 3. 执行协程 step
        let result = self.executor.step(dt);

        // 收回 context
        self.reclaim_context_from_executor();

        result
    }

    /// 调用 ON_DRAW 钩子并收集绘制命令
    ///
    /// 此方法在渲染时调用，执行 BASIC 的 ON_DRAW 子程序。
    /// 执行后，可以通过 `context_mut().drain_commands()` 获取绘制命令。
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 绘制成功
    /// - `Err(BasicError)`: 执行错误
    pub fn draw(&mut self) -> Result<()> {
        // 清空之前的绘制命令
        self.context.clear_commands();

        // 设置 context 用于绘制
        unsafe { self.lend_context_to_executor(); }

        // 调用 ON_DRAW
        let result = self.call_subroutine_internal(ON_DRAW_LINE);

        // 收回 context
        self.reclaim_context_from_executor();

        result
    }

    /// 内部方法：调用 BASIC 子程序（假设 context 已经借出）
    fn call_subroutine_internal(&mut self, line_number: u16) -> Result<()> {
        // 检查行号是否存在
        if self.executor.runtime().get_line(line_number).is_none() {
            // 行号不存在，静默跳过（允许可选的钩子）
            return Ok(());
        }

        // 记录当前调用栈深度
        let initial_depth = self.executor.runtime().stack_depth();

        // 记录当前执行位置（用于 GOSUB 返回）
        let current_line = self.executor.runtime().get_current_line().unwrap_or(0);
        let current_stmt = self.executor.runtime().get_current_stmt_index();

        // 执行 GOSUB（压栈）
        self.executor.runtime_mut().push_gosub(current_line, current_stmt)?;

        // 跳转到子程序
        self.executor.runtime_mut().set_execution_position(line_number, 0)?;

        // 执行到 RETURN（栈深度恢复到初始值）
        loop {
            if let Some(stmt) = self.executor.runtime_mut().get_next_statement() {
                self.executor.execute_statement(&stmt)?;

                // 检查是否返回了（栈深度减少）
                if self.executor.runtime().stack_depth() <= initial_depth {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    /// 调用 BASIC 子程序（公开接口）
    ///
    /// 执行指定行号的 GOSUB 调用，常用于生命周期钩子。
    ///
    /// # 参数
    ///
    /// - `line_number`: 要调用的子程序行号
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 调用成功
    /// - `Err(BasicError)`: 行号不存在或执行错误
    pub fn call_subroutine(&mut self, line_number: u16) -> Result<()> {
        // 设置 context
        unsafe { self.lend_context_to_executor(); }

        let result = self.call_subroutine_internal(line_number);

        // 收回 context
        self.reclaim_context_from_executor();

        result
    }

    /// 获取 BASIC 执行器的可变引用
    ///
    /// 用于高级用例，如直接操作变量或运行时状态。
    pub fn executor_mut(&mut self) -> &mut Executor {
        &mut self.executor
    }

    /// 获取 BASIC 执行器的不可变引用
    pub fn executor(&self) -> &Executor {
        &self.executor
    }

    /// 获取游戏上下文的可变引用
    ///
    /// 允许外部代码:
    /// - 获取绘制命令: `context_mut().drain_commands()`
    /// - 更新输入状态: `context_mut().set_key_state(...)`
    /// - 获取精灵数据: `context().sprites()`
    pub fn context_mut(&mut self) -> &mut PixelGameContext {
        &mut self.context
    }

    /// 获取游戏上下文的不可变引用
    pub fn context(&self) -> &PixelGameContext {
        &self.context
    }

    /// 检查程序是否已结束
    pub fn is_ended(&self) -> bool {
        matches!(
            self.executor.runtime().get_state(),
            crate::basic::runtime::ExecutionState::Ended
        )
    }

    /// 重置解释器状态
    ///
    /// 清空所有变量、程序和运行时状态。
    pub fn reset(&mut self) {
        self.executor = Executor::new();
        self.context = PixelGameContext::new();
        self.init_called = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_bridge_creation() {
        let bridge = GameBridge::new();
        assert!(!bridge.init_called);
    }

    #[test]
    fn test_load_program() {
        let mut bridge = GameBridge::new();
        let program = r#"
10 PRINT "HELLO"
20 END
        "#;
        assert!(bridge.load_program(program).is_ok());
    }

    #[test]
    fn test_update_calls_init_once() {
        let mut bridge = GameBridge::new();
        let program = r#"
10 X = 0
20 YIELD
30 GOTO 20

1000 REM ON_INIT
1010 X = 100
1020 RETURN

2000 REM ON_TICK
2010 X = X + 1
2020 RETURN

3500 REM ON_DRAW
3510 RETURN
        "#;
        bridge.load_program(program).unwrap();

        // 第一次 update 应该调用 ON_INIT
        assert!(bridge.update(0.016).unwrap());
        assert!(bridge.init_called);

        // 检查 X 是否被 ON_INIT 设置为 100
        let x = bridge.executor().variables().get("X");
        assert_eq!(x.as_number().unwrap(), 101.0); // ON_INIT(100) + ON_TICK(+1)

        // 第二次 update 不应再调用 ON_INIT
        assert!(bridge.update(0.016).unwrap());
        let x = bridge.executor().variables().get("X");
        assert_eq!(x.as_number().unwrap(), 102.0); // 只有 ON_TICK(+1)
    }

    #[test]
    fn test_call_subroutine() {
        let mut bridge = GameBridge::new();
        let program = r#"
10 X = 0
20 END

1000 REM SUBROUTINE
1010 X = 42
1020 RETURN
        "#;
        bridge.load_program(program).unwrap();

        // 调用子程序
        bridge.call_subroutine(1000).unwrap();

        // 检查变量
        let x = bridge.executor().variables().get("X");
        assert_eq!(x.as_number().unwrap(), 42.0);
    }

    #[test]
    fn test_call_nonexistent_subroutine() {
        let mut bridge = GameBridge::new();
        let program = "10 END";
        bridge.load_program(program).unwrap();

        // 调用不存在的子程序应该静默跳过
        assert!(bridge.call_subroutine(9999).is_ok());
    }

    #[test]
    fn test_reset() {
        let mut bridge = GameBridge::new();
        bridge.load_program("10 X = 100").unwrap();
        bridge.update(0.016).unwrap();

        bridge.reset();
        assert!(!bridge.init_called);
        // 检查变量已被清空（get返回默认值0.0）
        let x = bridge.executor().variables().get("X");
        assert_eq!(x.as_number().unwrap(), 0.0);
    }

    #[test]
    fn test_draw_collects_commands() {
        let mut bridge = GameBridge::new();
        let program = r#"
10 END

3500 REM ON_DRAW
3510 CLS
3520 PLOT 10, 20, "@", 15, 0
3530 RETURN
        "#;
        bridge.load_program(program).unwrap();

        // 调用 draw
        bridge.draw().unwrap();

        // 检查绘制命令
        let commands = bridge.context().commands();
        assert_eq!(commands.len(), 2);

        use crate::pixel_game_context::DrawCommand;
        assert!(matches!(commands[0], DrawCommand::Clear));
        match &commands[1] {
            DrawCommand::Plot { x, y, ch, fg, bg } => {
                assert_eq!(*x, 10);
                assert_eq!(*y, 20);
                assert_eq!(*ch, '@');
                assert_eq!(*fg, 15);
                assert_eq!(*bg, 0);
            }
            _ => panic!("Expected Plot command"),
        }
    }
}
