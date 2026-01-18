/// 运行时环境
///
/// 管理 BASIC 程序的执行状态，包括程序存储、变量、调用栈等

use std::collections::BTreeMap;
use super::ast::*;
use super::error::{BasicError, Result};

/// 调用栈帧（用于 GOSUB 和 FOR 循环）
#[derive(Debug, Clone, PartialEq)]
pub enum CallFrame {
    /// GOSUB 调用
    Gosub {
        return_line: u16,
        return_stmt: usize,
    },
    /// FOR 循环
    ForLoop {
        var_name: String,
        end_value: f64,
        step: f64,
        loop_line: u16,
        loop_stmt: usize,
    },
}

/// 程序执行状态
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    /// 未运行
    NotRunning,
    /// 正在运行
    Running,
    /// 已结束
    Ended,
    /// 已暂停（STOP 或 Ctrl+C）
    Paused {
        line: u16,
        stmt: usize,
    },
    /// 等待指定时间后恢复（协程）
    Waiting {
        line: u16,
        stmt: usize,
        resume_at: f64,  // 游戏时间，单位：秒
    },
    /// 让出执行，下一帧恢复（协程）
    Yielded {
        line: u16,
        stmt: usize,
    },
    /// 等待特定事件（协程）
    WaitingFor {
        line: u16,
        stmt: usize,
        event: WaitEvent,
    },
}

/// 等待的事件类型
#[derive(Debug, Clone, PartialEq)]
pub enum WaitEvent {
    /// 等待任意按键
    KeyPress,
    /// 等待鼠标点击
    MouseClick,
}

/// 运行时环境
pub struct Runtime {
    /// 程序存储（行号 -> 程序行）
    program: BTreeMap<u16, ProgramLine>,
    
    /// 调用栈（GOSUB 和 FOR 循环）
    call_stack: Vec<CallFrame>,
    
    /// 执行状态
    state: ExecutionState,
    
    /// 当前执行的行号
    current_line: Option<u16>,
    
    /// 当前执行的语句索引
    current_stmt: usize,
    
    /// 栈深度限制
    max_stack_depth: usize,
}

impl Runtime {
    /// 创建新的运行时环境
    pub fn new() -> Self {
        Runtime {
            program: BTreeMap::new(),
            call_stack: Vec::new(),
            state: ExecutionState::NotRunning,
            current_line: None,
            current_stmt: 0,
            max_stack_depth: 100,
        }
    }

    /// 添加或替换程序行
    pub fn add_line(&mut self, line: ProgramLine) {
        if line.statements.is_empty() {
            // 空语句列表表示删除该行
            self.program.remove(&line.line_number);
        } else {
            self.program.insert(line.line_number, line);
        }
    }

    /// 删除程序行
    pub fn delete_line(&mut self, line_number: u16) {
        self.program.remove(&line_number);
    }

    /// 获取程序行
    pub fn get_line(&self, line_number: u16) -> Option<&ProgramLine> {
        self.program.get(&line_number)
    }

    /// 检查程序是否为空
    pub fn is_empty(&self) -> bool {
        self.program.is_empty()
    }

    /// 获取程序行数
    pub fn line_count(&self) -> usize {
        self.program.len()
    }

    /// 获取所有程序行（按行号排序）
    pub fn get_all_lines(&self) -> Vec<&ProgramLine> {
        self.program.values().collect()
    }

    /// 获取指定范围的程序行
    pub fn get_lines_range(&self, start: Option<u16>, end: Option<u16>) -> Vec<&ProgramLine> {
        let start = start.unwrap_or(0);
        let end = end.unwrap_or(u16::MAX);
        
        self.program
            .range(start..=end)
            .map(|(_, line)| line)
            .collect()
    }

    /// 清空程序
    pub fn clear_program(&mut self) {
        self.program.clear();
        self.reset_execution_state();
    }
    
    /// 克隆整个程序（用于保存）
    pub fn clone_program(&self) -> BTreeMap<u16, ProgramLine> {
        self.program.clone()
    }
    
    /// 获取所有程序行的引用（用于LIST命令）
    pub fn get_program_lines(&self, start: Option<u16>, end: Option<u16>) -> Vec<&ProgramLine> {
        self.get_lines_range(start, end)
    }

    /// 重置执行状态
    fn reset_execution_state(&mut self) {
        self.call_stack.clear();
        self.state = ExecutionState::NotRunning;
        self.current_line = None;
        self.current_stmt = 0;
    }

    /// 获取执行状态
    pub fn get_state(&self) -> &ExecutionState {
        &self.state
    }

    /// 获取当前行号
    pub fn get_current_line(&self) -> Option<u16> {
        self.current_line
    }
    
    /// 获取当前语句索引
    pub fn get_current_stmt_index(&self) -> usize {
        self.current_stmt
    }

    /// 获取调用栈深度
    pub fn stack_depth(&self) -> usize {
        self.call_stack.len()
    }
    
    /// 中断程序执行（Ctrl+C）
    pub fn interrupt(&mut self) {
        if let Some(line) = self.current_line {
            self.state = ExecutionState::Paused {
                line,
                stmt: self.current_stmt,
            };
        }
    }
    
    /// 检查是否已停止
    pub fn is_stopped(&self) -> bool {
        matches!(self.state, ExecutionState::Paused { .. } | ExecutionState::Ended)
    }
    
    /// 检查是否可以继续执行
    pub fn can_continue(&self) -> bool {
        matches!(self.state, ExecutionState::Paused { .. })
    }

    // ========== 协程状态管理 ==========

    /// 进入等待状态（WAIT 语句）
    pub fn enter_wait(&mut self, resume_at: f64) {
        if let Some(line) = self.current_line {
            self.state = ExecutionState::Waiting {
                line,
                stmt: self.current_stmt,
                resume_at,
            };
        }
    }

    /// 进入 Yield 状态（YIELD 语句）
    pub fn enter_yield(&mut self) {
        if let Some(line) = self.current_line {
            self.state = ExecutionState::Yielded {
                line,
                stmt: self.current_stmt,
            };
        }
    }

    /// 进入等待事件状态（WAITKEY/WAITCLICK 语句）
    pub fn enter_wait_for(&mut self, event: WaitEvent) {
        if let Some(line) = self.current_line {
            self.state = ExecutionState::WaitingFor {
                line,
                stmt: self.current_stmt,
                event,
            };
        }
    }

    /// 从等待/Yield 状态恢复执行
    pub fn resume_from_wait(&mut self) -> Result<()> {
        match &self.state {
            ExecutionState::Waiting { line, stmt, .. } |
            ExecutionState::Yielded { line, stmt } |
            ExecutionState::WaitingFor { line, stmt, .. } => {
                self.current_line = Some(*line);
                self.current_stmt = *stmt + 1;  // 移动到下一条语句
                self.state = ExecutionState::Running;
                Ok(())
            }
            _ => Err(BasicError::CantContinue),
        }
    }

    /// 检查是否可以从等待状态恢复（基于游戏时间）
    pub fn can_resume(&self, current_time: f64) -> bool {
        match &self.state {
            ExecutionState::Waiting { resume_at, .. } => current_time >= *resume_at,
            ExecutionState::Yielded { .. } => true,
            _ => false,
        }
    }

    /// 检查是否在等待特定事件
    pub fn is_waiting_for_event(&self, event: &WaitEvent) -> bool {
        match &self.state {
            ExecutionState::WaitingFor { event: wait_event, .. } => wait_event == event,
            _ => false,
        }
    }

    /// 检查是否处于协程等待状态
    pub fn is_coroutine_waiting(&self) -> bool {
        matches!(
            self.state,
            ExecutionState::Waiting { .. } |
            ExecutionState::Yielded { .. } |
            ExecutionState::WaitingFor { .. }
        )
    }

    /// GOSUB 调用（入栈）
    pub fn push_gosub(&mut self, return_line: u16, return_stmt: usize) -> Result<()> {
        if self.call_stack.len() >= self.max_stack_depth {
            return Err(BasicError::StackOverflow);
        }
        
        self.call_stack.push(CallFrame::Gosub {
            return_line,
            return_stmt,
        });
        
        Ok(())
    }

    /// RETURN 返回（出栈）
    pub fn pop_gosub(&mut self) -> Result<(u16, usize)> {
        // 从栈顶查找最近的 GOSUB 帧
        while let Some(frame) = self.call_stack.pop() {
            match frame {
                CallFrame::Gosub { return_line, return_stmt } => {
                    return Ok((return_line, return_stmt));
                }
                CallFrame::ForLoop { .. } => {
                    // 遇到 FOR 循环帧，说明有未配对的 NEXT
                    // 继续查找（或者报错，取决于 BASIC 的语义）
                    // 这里我们允许跨越 FOR 循环返回
                    continue;
                }
            }
        }
        
        Err(BasicError::ReturnWithoutGosub)
    }

    /// FOR 循环入栈
    pub fn push_for_loop(
        &mut self,
        var_name: String,
        end_value: f64,
        step: f64,
        loop_line: u16,
        loop_stmt: usize,
    ) -> Result<()> {
        if self.call_stack.len() >= self.max_stack_depth {
            return Err(BasicError::StackOverflow);
        }
        
        self.call_stack.push(CallFrame::ForLoop {
            var_name,
            end_value,
            step,
            loop_line,
            loop_stmt,
        });
        
        Ok(())
    }

    /// NEXT 循环结束处理
    pub fn pop_for_loop(&mut self, expected_var: Option<String>) -> Result<(String, f64, f64, u16, usize)> {
        // 从栈顶查找匹配的 FOR 循环
        let mut found_index = None;
        
        for (i, frame) in self.call_stack.iter().enumerate().rev() {
            if let CallFrame::ForLoop { var_name, .. } = frame {
                if let Some(ref expected) = expected_var {
                    if var_name == expected {
                        found_index = Some(i);
                        break;
                    }
                } else {
                    // 没有指定变量名，匹配最近的 FOR 循环
                    found_index = Some(i);
                    break;
                }
            }
        }
        
        if let Some(index) = found_index {
            let frame = self.call_stack.remove(index);
            
            if let CallFrame::ForLoop { var_name, end_value, step, loop_line, loop_stmt } = frame {
                Ok((var_name, end_value, step, loop_line, loop_stmt))
            } else {
                unreachable!()
            }
        } else {
            Err(BasicError::NextWithoutFor(
                expected_var.unwrap_or_else(|| "".to_string())
            ))
        }
    }

    /// 设置执行位置（用于 GOTO, GOSUB 等）
    pub fn set_execution_position(&mut self, line: u16, stmt: usize) -> Result<()> {
        // 检查行号是否存在
        if !self.program.contains_key(&line) {
            return Err(BasicError::UndefinedLine(line));
        }
        
        self.current_line = Some(line);
        self.current_stmt = stmt;
        
        Ok(())
    }

    /// 获取下一条要执行的语句（返回克隆）
    pub fn get_next_statement(&mut self) -> Option<Statement> {
        let line_num = self.current_line?;
        let line = self.program.get(&line_num)?.clone();
        
        if self.current_stmt < line.statements.len() {
            let stmt = line.statements[self.current_stmt].clone();
            self.current_stmt += 1;
            Some(stmt)
        } else {
            // 当前行执行完毕，移到下一行
            self.advance_to_next_line();
            self.get_next_statement()
        }
    }

    /// 前进到下一行
    fn advance_to_next_line(&mut self) {
        if let Some(current) = self.current_line {
            // 找到下一个行号
            let next_line = self.program
                .range((current + 1)..)
                .next()
                .map(|(num, _)| *num);
            
            self.current_line = next_line;
            self.current_stmt = 0;
        }
    }

    /// 开始执行程序
    pub fn start_execution(&mut self, start_line: Option<u16>) -> Result<()> {
        if self.program.is_empty() {
            return Err(BasicError::SyntaxError("No program to run".to_string()));
        }
        
        // 清空调用栈
        self.call_stack.clear();
        
        // 确定起始行
        let line = if let Some(num) = start_line {
            if !self.program.contains_key(&num) {
                return Err(BasicError::UndefinedLine(num));
            }
            num
        } else {
            // 从第一行开始
            *self.program.keys().next().unwrap()
        };
        
        self.current_line = Some(line);
        self.current_stmt = 0;
        self.state = ExecutionState::Running;
        
        Ok(())
    }

    /// 暂停执行
    pub fn pause_execution(&mut self) {
        if let Some(line) = self.current_line {
            // 注意：current_stmt 已经指向下一条语句（因为 get_next_statement() 已经递增过了）
            // 所以我们需要减去 1 来获取当前语句的索引
            // 但如果是 0，说明当前行已经执行完，应该跳到下一行
            let stmt = if self.current_stmt > 0 {
                self.current_stmt - 1
            } else {
                // 当前行已经执行完，保持当前值（会被 continue_execution 处理）
                self.current_stmt
            };
            self.state = ExecutionState::Paused {
                line,
                stmt,
            };
        }
    }

    /// 继续执行
    pub fn continue_execution(&mut self) -> Result<()> {
        match &self.state {
            ExecutionState::Paused { line, stmt } => {
                self.current_line = Some(*line);
                // 恢复时，设置 current_stmt 为保存的语句索引 + 1
                // 这样 get_next_statement() 就能获取下一条语句
                self.current_stmt = *stmt + 1;
                
                // 如果 current_stmt 已经超出了当前行的语句数量，跳到下一行
                if let Some(line_num) = self.current_line {
                    if let Some(line) = self.program.get(&line_num) {
                        if self.current_stmt >= line.statements.len() {
                            self.advance_to_next_line();
                        }
                    }
                }
                
                self.state = ExecutionState::Running;
                Ok(())
            }
            _ => Err(BasicError::CantContinue),
        }
    }

    /// 结束执行
    pub fn end_execution(&mut self) {
        self.state = ExecutionState::Ended;
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        matches!(self.state, ExecutionState::Running)
    }

    /// 检查是否已暂停
    pub fn is_paused(&self) -> bool {
        matches!(self.state, ExecutionState::Paused { .. })
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Requirement: 程序存储和管理 - 添加程序行
    #[test]
    fn test_add_line() {
        let mut runtime = Runtime::new();
        
        let line = ProgramLine {
            line_number: 10,
            statements: vec![Statement::Print {
                items: vec![PrintItem::Expr(Expr::String("HELLO".to_string()))],
            }],
        };
        
        runtime.add_line(line);
        assert_eq!(runtime.line_count(), 1);
        assert!(runtime.get_line(10).is_some());
    }

    // Requirement: 程序存储和管理 - 替换现有行
    #[test]
    fn test_replace_line() {
        let mut runtime = Runtime::new();
        
        // 添加第一个版本
        let line1 = ProgramLine {
            line_number: 10,
            statements: vec![Statement::Print {
                items: vec![PrintItem::Expr(Expr::String("HELLO".to_string()))],
            }],
        };
        runtime.add_line(line1);
        
        // 替换
        let line2 = ProgramLine {
            line_number: 10,
            statements: vec![Statement::Print {
                items: vec![PrintItem::Expr(Expr::String("WORLD".to_string()))],
            }],
        };
        runtime.add_line(line2);
        
        assert_eq!(runtime.line_count(), 1);
        let line = runtime.get_line(10).unwrap();
        match &line.statements[0] {
            Statement::Print { items } => {
                match &items[0] {
                    PrintItem::Expr(Expr::String(s)) => assert_eq!(s, "WORLD"),
                    _ => panic!("Expected string"),
                }
            }
            _ => panic!("Expected Print statement"),
        }
    }

    // Requirement: 程序存储和管理 - 删除程序行
    #[test]
    fn test_delete_line() {
        let mut runtime = Runtime::new();
        
        let line = ProgramLine {
            line_number: 10,
            statements: vec![Statement::End],
        };
        runtime.add_line(line);
        assert_eq!(runtime.line_count(), 1);
        
        runtime.delete_line(10);
        assert_eq!(runtime.line_count(), 0);
        assert!(runtime.get_line(10).is_none());
    }

    // Requirement: 程序存储和管理 - 行号排序
    #[test]
    fn test_line_sorting() {
        let mut runtime = Runtime::new();
        
        // 乱序添加
        runtime.add_line(ProgramLine { line_number: 30, statements: vec![Statement::End] });
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        runtime.add_line(ProgramLine { line_number: 20, statements: vec![Statement::End] });
        
        let lines = runtime.get_all_lines();
        assert_eq!(lines[0].line_number, 10);
        assert_eq!(lines[1].line_number, 20);
        assert_eq!(lines[2].line_number, 30);
    }

    // Requirement: 程序执行 - 从第一行开始执行
    #[test]
    fn test_start_execution_from_first_line() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        runtime.add_line(ProgramLine { line_number: 20, statements: vec![Statement::End] });
        
        runtime.start_execution(None).unwrap();
        assert_eq!(runtime.get_current_line(), Some(10));
    }

    // Requirement: 程序执行 - 从指定行开始
    #[test]
    fn test_start_execution_from_specific_line() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        runtime.add_line(ProgramLine { line_number: 100, statements: vec![Statement::End] });
        
        runtime.start_execution(Some(100)).unwrap();
        assert_eq!(runtime.get_current_line(), Some(100));
    }

    // Requirement: 行号跳转 - GOTO 跳转
    #[test]
    fn test_goto_jump() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        runtime.add_line(ProgramLine { line_number: 100, statements: vec![Statement::End] });
        
        runtime.start_execution(Some(10)).unwrap();
        runtime.set_execution_position(100, 0).unwrap();
        
        assert_eq!(runtime.get_current_line(), Some(100));
    }

    // Requirement: 行号跳转 - 跳转到不存在的行
    #[test]
    fn test_goto_undefined_line() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        
        let result = runtime.set_execution_position(999, 0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::UndefinedLine(999)));
    }

    // Requirement: 子程序调用栈 - GOSUB 调用
    #[test]
    fn test_gosub_call() {
        let mut runtime = Runtime::new();
        
        runtime.push_gosub(10, 1).unwrap();
        assert_eq!(runtime.stack_depth(), 1);
    }

    // Requirement: 子程序调用栈 - RETURN 返回
    #[test]
    fn test_return() {
        let mut runtime = Runtime::new();
        
        runtime.push_gosub(10, 1).unwrap();
        let (line, stmt) = runtime.pop_gosub().unwrap();
        
        assert_eq!(line, 10);
        assert_eq!(stmt, 1);
        assert_eq!(runtime.stack_depth(), 0);
    }

    // Requirement: 子程序调用栈 - 嵌套 GOSUB
    #[test]
    fn test_nested_gosub() {
        let mut runtime = Runtime::new();
        
        runtime.push_gosub(10, 0).unwrap();
        runtime.push_gosub(20, 0).unwrap();
        runtime.push_gosub(30, 0).unwrap();
        
        assert_eq!(runtime.stack_depth(), 3);
        
        let (line, _) = runtime.pop_gosub().unwrap();
        assert_eq!(line, 30);
        
        let (line, _) = runtime.pop_gosub().unwrap();
        assert_eq!(line, 20);
        
        let (line, _) = runtime.pop_gosub().unwrap();
        assert_eq!(line, 10);
    }

    // Requirement: 子程序调用栈 - RETURN 无对应 GOSUB
    #[test]
    fn test_return_without_gosub() {
        let mut runtime = Runtime::new();
        
        let result = runtime.pop_gosub();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::ReturnWithoutGosub));
    }

    // Requirement: 子程序调用栈 - GOSUB 栈深度限制
    #[test]
    fn test_stack_overflow() {
        let mut runtime = Runtime::new();
        
        // 填满栈
        for _ in 0..runtime.max_stack_depth {
            runtime.push_gosub(10, 0).unwrap();
        }
        
        // 再次入栈应该失败
        let result = runtime.push_gosub(10, 0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::StackOverflow));
    }

    // Requirement: FOR 循环栈 - FOR 循环执行
    #[test]
    fn test_for_loop_push() {
        let mut runtime = Runtime::new();
        
        runtime.push_for_loop("I".to_string(), 10.0, 1.0, 10, 0).unwrap();
        assert_eq!(runtime.stack_depth(), 1);
    }

    // Requirement: FOR 循环栈 - NEXT 执行
    #[test]
    fn test_for_loop_pop() {
        let mut runtime = Runtime::new();
        
        runtime.push_for_loop("I".to_string(), 10.0, 1.0, 10, 0).unwrap();
        
        let (var, end_val, step, line, stmt) = runtime.pop_for_loop(Some("I".to_string())).unwrap();
        assert_eq!(var, "I");
        assert_eq!(end_val, 10.0);
        assert_eq!(step, 1.0);
        assert_eq!(line, 10);
        assert_eq!(stmt, 0);
    }

    // Requirement: FOR 循环栈 - 嵌套 FOR 循环
    #[test]
    fn test_nested_for_loops() {
        let mut runtime = Runtime::new();
        
        runtime.push_for_loop("I".to_string(), 10.0, 1.0, 10, 0).unwrap();
        runtime.push_for_loop("J".to_string(), 5.0, 1.0, 20, 0).unwrap();
        
        assert_eq!(runtime.stack_depth(), 2);
        
        // 先弹出内层循环
        let (var, _, _, _, _) = runtime.pop_for_loop(Some("J".to_string())).unwrap();
        assert_eq!(var, "J");
        
        // 再弹出外层循环
        let (var, _, _, _, _) = runtime.pop_for_loop(Some("I".to_string())).unwrap();
        assert_eq!(var, "I");
    }

    // Requirement: FOR 循环栈 - NEXT 变量不匹配
    #[test]
    fn test_next_without_for() {
        let mut runtime = Runtime::new();
        
        let result = runtime.pop_for_loop(Some("I".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::NextWithoutFor(_)));
    }

    // Requirement: NEW 命令 - 清空程序
    #[test]
    fn test_new_command() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        runtime.add_line(ProgramLine { line_number: 20, statements: vec![Statement::End] });
        assert_eq!(runtime.line_count(), 2);
        
        runtime.clear_program();
        assert_eq!(runtime.line_count(), 0);
        assert!(runtime.is_empty());
    }

    // Requirement: STOP 和 CONT - STOP 暂停
    #[test]
    fn test_stop_pause() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        runtime.start_execution(None).unwrap();
        
        runtime.pause_execution();
        assert!(runtime.is_paused());
    }

    // Requirement: STOP 和 CONT - CONT 继续
    #[test]
    fn test_cont_resume() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine { line_number: 10, statements: vec![Statement::End] });
        runtime.start_execution(None).unwrap();
        
        runtime.pause_execution();
        assert!(runtime.is_paused());
        
        runtime.continue_execution().unwrap();
        assert!(runtime.is_running());
    }

    // Requirement: STOP 和 CONT - 未暂停时 CONT
    #[test]
    fn test_cont_without_pause() {
        let mut runtime = Runtime::new();
        
        let result = runtime.continue_execution();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::CantContinue));
    }

    // Test: 获取下一条语句
    #[test]
    fn test_get_next_statement() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine {
            line_number: 10,
            statements: vec![
                Statement::Let {
                    target: AssignTarget::Variable("A".to_string()),
                    value: Expr::Number(5.0),
                },
                Statement::Print {
                    items: vec![PrintItem::Expr(Expr::Variable("A".to_string()))],
                },
            ],
        });
        
        runtime.start_execution(None).unwrap();
        
        // 获取第一条语句
        let stmt1 = runtime.get_next_statement();
        assert!(stmt1.is_some());
        assert!(matches!(stmt1.unwrap(), Statement::Let { .. }));
        
        // 获取第二条语句
        let stmt2 = runtime.get_next_statement();
        assert!(stmt2.is_some());
        assert!(matches!(stmt2.unwrap(), Statement::Print { .. }));
        
        // 没有更多语句了
        let stmt3 = runtime.get_next_statement();
        assert!(stmt3.is_none());
    }

    // Test: 跨行执行
    #[test]
    fn test_cross_line_execution() {
        let mut runtime = Runtime::new();
        
        runtime.add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Let {
                target: AssignTarget::Variable("A".to_string()),
                value: Expr::Number(1.0),
            }],
        });
        
        runtime.add_line(ProgramLine {
            line_number: 20,
            statements: vec![Statement::Let {
                target: AssignTarget::Variable("B".to_string()),
                value: Expr::Number(2.0),
            }],
        });
        
        runtime.start_execution(None).unwrap();
        
        // 执行第一行
        let _stmt1 = runtime.get_next_statement();
        assert_eq!(runtime.get_current_line(), Some(10));
        
        // 应该自动前进到第二行
        let _stmt2 = runtime.get_next_statement();
        assert_eq!(runtime.get_current_line(), Some(20));
    }
}

