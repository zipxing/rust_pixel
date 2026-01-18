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
/// │ Model::       │  Event[]    │ handle │  INKEY/  │ input    │
/// │ handle_event()│─────────────►│ _input │  KEY()   │ state    │
/// │               │              │        │          │          │
/// │ Render::      │              │ draw() │          │ sprites  │
/// │ draw()        │◄─────────────│        │◄─────────│ Panel    │
/// └───────────────┘              └────────┘          └──────────┘
/// ```
///
/// # 生命周期钩子
///
/// GameBridge 自动调用 BASIC 程序中的特定子程序作为生命周期钩子:
///
/// - **ON_INIT (1000行)**: 游戏启动时调用一次
/// - **ON_TICK (2000行)**: 每帧调用，用于游戏逻辑更新，DT 变量设置为帧时间
/// - **ON_DRAW (3000行)**: 每帧调用，用于渲染操作
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
/// 2050 SPRITE 1, X, Y, "@"
/// 2060 RETURN
///
/// 3000 REM ON_DRAW - 渲染
/// 3010 CLS
/// 3020 RETURN
/// ```

use crate::basic::{
    error::{BasicError, Result},
    tokenizer::Tokenizer,
    parser::Parser,
    executor::Executor,
};
use crate::game_context::GameContext;

/// 生命周期钩子的行号常量
pub const ON_INIT_LINE: u16 = 1000;  // 初始化钩子
pub const ON_TICK_LINE: u16 = 2000;  // 每帧逻辑钩子
pub const ON_DRAW_LINE: u16 = 3000;  // 渲染钩子

/// GameBridge - BASIC 与游戏引擎的桥接
///
/// # 泛型参数
///
/// - `C`: 实现 `GameContext` trait 的游戏上下文类型
///
/// # 示例
///
/// ```no_run
/// use pixel_basic::{GameBridge, NullGameContext};
///
/// let mut bridge = GameBridge::new(NullGameContext);
///
/// // 加载 BASIC 程序
/// let program = r#"
/// 10 PRINT "HELLO WORLD"
/// 20 WAIT 1.0
/// 30 END
/// "#;
/// bridge.load_program(program).unwrap();
///
/// // 游戏循环中调用
/// loop {
///     let dt = 0.016; // 16ms per frame (60 FPS)
///     if !bridge.update(dt).unwrap() {
///         break; // 程序结束
///     }
/// }
/// ```
pub struct GameBridge<C: GameContext> {
    /// BASIC 执行器
    executor: Executor,

    /// 游戏上下文（提供图形/输入接口）
    context: C,

    /// 是否已调用 ON_INIT 钩子
    init_called: bool,
}

impl<C: GameContext> GameBridge<C> {
    /// 创建新的 GameBridge 实例
    ///
    /// # 参数
    ///
    /// - `context`: 实现 `GameContext` trait 的游戏上下文
    ///
    /// # 返回值
    ///
    /// 返回新的 GameBridge 实例，内部 BASIC 解释器已初始化
    pub fn new(context: C) -> Self {
        Self {
            executor: Executor::new(),
            context,
            init_called: false,
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
    /// # use pixel_basic::{GameBridge, NullGameContext};
    /// let mut bridge = GameBridge::new(NullGameContext);
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
    /// # use pixel_basic::{GameBridge, NullGameContext};
    /// # let mut bridge = GameBridge::new(NullGameContext);
    /// loop {
    ///     let dt = 0.016;
    ///     if !bridge.update(dt).unwrap() {
    ///         break; // 程序结束
    ///     }
    /// }
    /// ```
    pub fn update(&mut self, dt: f32) -> Result<bool> {
        // 1. 首次调用时执行 ON_INIT 钩子
        if !self.init_called {
            self.call_subroutine(ON_INIT_LINE)?;
            self.init_called = true;
        }

        // 2. 调用 ON_TICK 钩子（设置 DT 变量）
        self.executor.variables_mut().set("DT", crate::basic::variables::Value::Number(dt as f64))?;
        self.call_subroutine(ON_TICK_LINE)?;

        // 3. 执行协程 step
        self.executor.step(dt)
    }

    /// 绘制游戏画面（每帧调用）
    ///
    /// 调用 ON_DRAW 钩子，允许 BASIC 程序执行渲染操作。
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 绘制成功
    /// - `Err(BasicError)`: 执行错误
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use pixel_basic::{GameBridge, NullGameContext};
    /// # let mut bridge = GameBridge::new(NullGameContext);
    /// bridge.draw().unwrap();
    /// ```
    pub fn draw(&mut self) -> Result<()> {
        self.call_subroutine(ON_DRAW_LINE)?;
        Ok(())
    }

    /// 处理输入事件
    ///
    /// 此方法将 rust_pixel 的输入事件转换为 BASIC 可查询的输入状态。
    /// 目前这是一个占位符，实际实现需要:
    /// 1. 解析 rust_pixel 的 Event 类型
    /// 2. 更新 GameContext 中的输入状态
    /// 3. 检查是否有 WAITKEY/WAITCLICK 需要恢复
    ///
    /// # 参数
    ///
    /// - `events`: rust_pixel 的输入事件切片（未来扩展）
    ///
    /// # TODO
    ///
    /// - [ ] 定义 rust_pixel Event 类型的接口
    /// - [ ] 实现键盘事件到 INKEY/KEY 的映射
    /// - [ ] 实现鼠标事件到 MOUSEX/MOUSEY/MOUSEB 的映射
    /// - [ ] 处理 WAITKEY/WAITCLICK 协程恢复
    pub fn handle_input(&mut self, _events: &[()]) -> Result<()> {
        // TODO: 实现输入事件处理
        // 1. 遍历 events
        // 2. 更新 context 的输入状态
        // 3. 检查 runtime 是否在等待输入事件
        // 4. 如果匹配，调用 runtime.resume_from_wait()
        Ok(())
    }

    /// 调用 BASIC 子程序
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
    ///
    /// # 注意
    ///
    /// 此方法会立即执行子程序直到 RETURN，不支持协程暂停。
    /// 如果子程序中有 WAIT/YIELD，会触发错误。
    pub fn call_subroutine(&mut self, line_number: u16) -> Result<()> {
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
    /// 允许外部代码直接操作游戏上下文（如手动绘制图形）。
    pub fn context_mut(&mut self) -> &mut C {
        &mut self.context
    }

    /// 获取游戏上下文的不可变引用
    pub fn context(&self) -> &C {
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
        self.init_called = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_context::NullGameContext;

    #[test]
    fn test_game_bridge_creation() {
        let bridge = GameBridge::new(NullGameContext);
        assert!(!bridge.init_called);
    }

    #[test]
    fn test_load_program() {
        let mut bridge = GameBridge::new(NullGameContext);
        let program = r#"
10 PRINT "HELLO"
20 END
        "#;
        assert!(bridge.load_program(program).is_ok());
    }

    #[test]
    fn test_update_calls_init_once() {
        let mut bridge = GameBridge::new(NullGameContext);
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

3000 REM ON_DRAW
3010 RETURN
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
        let mut bridge = GameBridge::new(NullGameContext);
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
        let mut bridge = GameBridge::new(NullGameContext);
        let program = "10 END";
        bridge.load_program(program).unwrap();

        // 调用不存在的子程序应该静默跳过
        assert!(bridge.call_subroutine(9999).is_ok());
    }

    #[test]
    fn test_reset() {
        let mut bridge = GameBridge::new(NullGameContext);
        bridge.load_program("10 X = 100").unwrap();
        bridge.update(0.016).unwrap();

        bridge.reset();
        assert!(!bridge.init_called);
        // 检查变量已被清空（get返回默认值0.0）
        let x = bridge.executor().variables().get("X");
        assert_eq!(x.as_number().unwrap(), 0.0);
    }
}
