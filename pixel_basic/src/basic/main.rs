use basic_m6502::{
    ast::DataValue, BasicError, Executor, Parser, Result, Statement, Tokenizer,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// REPL 提示符
const PROMPT: &str = "BASIC.rs.";

fn main() -> Result<()> {
    println!("Microsoft BASIC 6502 Interpreter (Rust Edition)");
    println!();

    // 设置 Ctrl+C 处理器
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = interrupted.clone();
    
    ctrlc::set_handler(move || {
        interrupted_clone.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // 创建执行器
    let mut executor = Executor::new();

    // 创建 rustyline 编辑器（带历史记录）
    let mut rl = DefaultEditor::new().map_err(|e| {
        BasicError::SyntaxError(format!("Failed to initialize editor: {}", e))
    })?;

    // 加载历史记录（如果存在）
    let history_file = ".basic_history";
    let _ = rl.load_history(history_file);

    // REPL 主循环
    // 提示符类型：None(无提示), Some(PROMPT)(命令执行后)
    let mut prompt_text: Option<&str> = Some(PROMPT);
    
    loop {
        // 使用println打印提示符（如果需要）
        if let Some(text) = prompt_text {
            println!("{}", text);
        }
        
        // 清除中断标志（准备接收新命令）
        interrupted.store(false, Ordering::SeqCst);
        
        // 读取一行（提示符设为空）
        let readline = rl.readline("");
        
        match readline {
            Ok(line) => {
                // 添加到历史记录
                rl.add_history_entry(line.as_str()).ok();
                
                // 处理输入行
                match process_line(&mut executor, &line, &interrupted) {
                    Ok(should_print_ready) => {
                        // 执行成功后，根据返回值决定是否显示提示符
                        if should_print_ready {
                            prompt_text = Some(PROMPT);
                        } else {
                            prompt_text = None;
                        }
                    }
                    Err(e) => {
                        // 输出错误
                        match e {
                            BasicError::CantContinue => {
                                // 正常退出信号，不输出错误
                                prompt_text = None;
                            }
                            _ => {
                                eprintln!("?{}", format_error(&e));
                                prompt_text = Some(PROMPT);
                            }
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C 中断
                if let Some(line) = executor.runtime().get_current_line() {
                    println!("?BREAK IN {}", line);
                } else {
                    println!("^C");
                }
                executor.runtime_mut().interrupt();
                prompt_text = Some(PROMPT);
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D - 退出
                println!("Bye.");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // 保存历史记录
    rl.save_history(history_file).ok();
    
    Ok(())
}

/// 处理一行输入
/// 返回值：Ok(bool) - true 表示应该打印提示符, false 表示不打印
fn process_line(executor: &mut Executor, line: &str, interrupted: &Arc<AtomicBool>) -> Result<bool> {
    let line = line.trim();
    
    // 空行
    if line.is_empty() {
        return Ok(false);
    }
    
    // 词法分析
    let mut tokenizer = Tokenizer::new(line);
    let tokens = tokenizer.tokenize_line()?;
    
    // 语法分析
    let mut parser = Parser::new(tokens);
    
    // 解析程序行
    if let Some(program_line) = parser.parse_line()? {
        // 检查是否是带行号的程序行
        if program_line.line_number > 0 {
            // 添加或删除程序行
            if program_line.statements.is_empty() {
                // 空行：删除该行
                executor.runtime_mut().delete_line(program_line.line_number);
            } else {
                // 非空行：添加到程序
                // 在添加之前，收集所有 DATA 语句的值
                for stmt in &program_line.statements {
                    if let Statement::Data { values } = stmt {
                        for value in values {
                            // 转换 ast::DataValue 到 executor::DataValue
                            let exec_value = match value {
                                DataValue::Number(n) => basic_m6502::DataValue::Number(*n),
                                DataValue::String(s) => basic_m6502::DataValue::String(s.clone()),
                            };
                            executor.add_data_value(exec_value);
                        }
                    }
                }
                executor.runtime_mut().add_line(program_line);
            }
            // 程序行输入后不打印 Ready
            return Ok(false);
        }
        
        // 直接模式：执行语句
        if program_line.statements.is_empty() {
            return Ok(false);
        }
        let statement = program_line.statements.into_iter().next().unwrap();
        
        // 特殊命令处理
        match &statement {
            Statement::List { start, end } => {
                list_program(executor, *start, *end);
                Ok(true)
            }
            Statement::Run { line_number } => {
                run_program(executor, *line_number, interrupted)?;
                Ok(true)
            }
            Statement::New => {
                executor.execute_statement(&statement)?;
                println!("New program started.");
                Ok(true)
            }
            Statement::End => {
                executor.execute_statement(&statement)?;
                Ok(true)
            }
            Statement::Stop => {
                executor.execute_statement(&statement)?;
                if let Some(line) = executor.runtime().get_current_line() {
                    println!("?BREAK IN {}", line);
                }
                Ok(true)
            }
            Statement::Cont => {
                continue_program(executor, interrupted)?;
                Ok(true)
            }
            _ => {
                // 其他语句：直接执行
                executor.execute_statement(&statement)?;
                Ok(true)
            }
        }
    } else {
        Ok(false)
    }
}

/// 列出程序
fn list_program(executor: &Executor, start: Option<u16>, end: Option<u16>) {
    let lines = executor.runtime().get_program_lines(start, end);
    
    if lines.is_empty() {
        if start.is_some() || end.is_some() {
            // 有范围但没有内容，不输出
            return;
        }
        println!("No program loaded.");
        return;
    }
    
    for line in lines {
        // 使用完整的序列化函数
        println!("{}", Executor::serialize_program_line(line));
    }
}

/// 运行程序
fn run_program(executor: &mut Executor, line_number: Option<u16>, interrupted: &Arc<AtomicBool>) -> Result<()> {
    // 只有在未运行时才启动执行
    if !executor.runtime().is_running() && !executor.runtime().is_paused() {
        // 在启动新执行前，清空所有变量和数组（经典 BASIC 行为）
        executor.variables_mut().clear();
        // 重置 DATA 指针，使其可以从头读取 DATA 语句
        executor.restore_data();
        executor.runtime_mut().start_execution(line_number)?;
    }
    
    // 执行循环
    loop {
        // 检查中断标志
        if interrupted.load(Ordering::SeqCst) {
            // 中断程序
            executor.runtime_mut().interrupt();
            if let Some(line) = executor.runtime().get_current_line() {
                println!("?BREAK IN {}", line);
            }
            interrupted.store(false, Ordering::SeqCst); // 清除标志
            return Ok(());
        }
        
        let stmt = match executor.runtime_mut().get_next_statement() {
            Some(s) => s,
            None => break, // 程序结束
        };
        
        // 执行语句
        if let Err(e) = executor.execute_statement(&stmt) {
            // 输出错误和行号
            if let Some(line) = executor.runtime().get_current_line() {
                eprintln!("?{} IN {}", format_error(&e), line);
            } else {
                eprintln!("?{}", format_error(&e));
            }
            // 错误已经打印，不再向上传播，直接返回 Ok
            return Ok(());
        }
        
        // 检查是否应该停止
        if executor.runtime().is_stopped() {
            // 如果是暂停状态（STOP），打印消息
            if executor.runtime().is_paused() {
                if let Some(line) = executor.runtime().get_current_line() {
                    println!("?BREAK IN {}", line);
                }
            }
            break;
        }
    }
    
    Ok(())
}

/// 继续执行程序
fn continue_program(executor: &mut Executor, interrupted: &Arc<AtomicBool>) -> Result<()> {
    if !executor.runtime().can_continue() {
        println!("?CAN'T CONTINUE");
        // 错误已经打印，返回 Ok 以便显示 Ready
        return Ok(());
    }
    
    // 从暂停点恢复执行
    executor.runtime_mut().continue_execution()?;
    run_program(executor, None, interrupted)
}

/// 格式化错误信息
fn format_error(error: &BasicError) -> String {
    match error {
        BasicError::IllegalCharacter(_, _, _) => {
            // 使用 Display 格式显示完整错误信息（包含上下文）
            format!("{}", error)
        }
        BasicError::SyntaxError(_) => "SYNTAX ERROR".to_string(),
        BasicError::DivisionByZero => "DIVISION BY ZERO".to_string(),
        BasicError::TypeMismatch(_) => "TYPE MISMATCH".to_string(),
        BasicError::UndefinedLine(n) => format!("UNDEFINED LINE {}", n),
        BasicError::UndefinedVariable(v) => format!("UNDEFINED VARIABLE {}", v),
        BasicError::SubscriptOutOfRange(_) => "SUBSCRIPT OUT OF RANGE".to_string(),
        BasicError::OutOfData => "OUT OF DATA".to_string(),
        BasicError::ReturnWithoutGosub => "RETURN WITHOUT GOSUB".to_string(),
        BasicError::NextWithoutFor(_) => "NEXT WITHOUT FOR".to_string(),
        BasicError::CantContinue => "CAN'T CONTINUE".to_string(),
        _ => format!("{}", error),  // 使用 Display 格式而不是 Debug
    }
}

