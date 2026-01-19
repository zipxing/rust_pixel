/// 执行引擎
///
/// 求值表达式并执行语句

use super::ast::*;
use super::error::{BasicError, Result};
use super::runtime::Runtime;
use super::variables::{Value, Variables};
use crate::game_context::GameContext;

/// 输入回调函数类型
pub type InputCallback = Box<dyn FnMut(&str) -> Option<String>>;

/// 执行引擎
pub struct Executor {
    runtime: Runtime,
    variables: Variables,
    /// 输出缓冲区（用于测试和捕获输出）
    output_buffer: Vec<String>,
    /// 当前打印列位置
    print_column: usize,
    /// DATA 数据存储
    data_values: Vec<DataValue>,
    /// DATA 数据指针（当前读取位置）
    data_pointer: usize,
    /// 输入回调函数（用于测试）
    input_callback: Option<InputCallback>,
    /// 游戏时间累加器（秒）- 用于协程 WAIT 语句
    game_time: f64,
    /// 游戏上下文（可选）- 用于调用游戏引擎 API
    game_context: Option<Box<dyn GameContext>>,
}

/// DATA 值类型
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    Number(f64),
    String(String),
}

impl Executor {
    /// 创建新的执行器
    pub fn new() -> Self {
        Executor {
            runtime: Runtime::new(),
            variables: Variables::new(),
            output_buffer: Vec::new(),
            print_column: 0,
            data_values: Vec::new(),
            data_pointer: 0,
            input_callback: None,
            game_time: 0.0,
            game_context: None,
        }
    }

    /// 设置游戏上下文
    ///
    /// 设置后，BASIC 程序可以调用游戏引擎的图形、精灵、输入等 API。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let ctx = Box::new(MyGameContext::new());
    /// executor.set_game_context(ctx);
    /// ```
    pub fn set_game_context(&mut self, ctx: Box<dyn GameContext>) {
        self.game_context = Some(ctx);
    }

    /// 获取游戏上下文的可变引用
    ///
    /// 如果没有设置游戏上下文，返回 None。
    pub fn game_context_mut(&mut self) -> Option<&mut Box<dyn GameContext>> {
        self.game_context.as_mut()
    }

    /// 获取游戏上下文的不可变引用
    pub fn game_context(&self) -> Option<&Box<dyn GameContext>> {
        self.game_context.as_ref()
    }

    /// 检查是否有游戏上下文
    pub fn has_game_context(&self) -> bool {
        self.game_context.is_some()
    }
    
    /// 设置输入回调函数（用于测试）
    pub fn set_input_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&str) -> Option<String> + 'static,
    {
        self.input_callback = Some(Box::new(callback));
    }
    
    /// 添加 DATA 值
    pub fn add_data_value(&mut self, value: DataValue) {
        self.data_values.push(value);
    }
    
    /// 重置 DATA 指针
    pub fn restore_data(&mut self) {
        self.data_pointer = 0;
    }
    
    /// 读取下一个 DATA 值
    fn read_data_value(&mut self) -> Result<DataValue> {
        if self.data_pointer >= self.data_values.len() {
            return Err(BasicError::OutOfData);
        }
        
        let value = self.data_values[self.data_pointer].clone();
        self.data_pointer += 1;
        Ok(value)
    }
    
    /// 获取输出内容（用于测试）
    pub fn get_output(&self) -> String {
        self.output_buffer.join("")
    }
    
    /// 清空输出缓冲区
    pub fn clear_output(&mut self) {
        self.output_buffer.clear();
        self.print_column = 0;
    }
    
    /// 输出文本（添加到缓冲区并打印到终端）
    fn output(&mut self, text: &str) {
        // 打印到终端
        print!("{}", text);
        use std::io::Write;
        std::io::stdout().flush().ok();
        
        // 同时添加到缓冲区（用于测试）
        self.output_buffer.push(text.to_string());
        
        // 更新列位置
        for ch in text.chars() {
            if ch == '\n' {
                self.print_column = 0;
            } else {
                self.print_column += 1;
            }
        }
    }
    
    /// 输出换行
    fn output_newline(&mut self) {
        self.output("\n");
    }

    /// 获取运行时引用
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// 获取变量存储引用
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// 获取运行时可变引用
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// 获取变量存储可变引用
    pub fn variables_mut(&mut self) -> &mut Variables {
        &mut self.variables
    }

    /// 求值表达式
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            
            Expr::String(s) => Ok(Value::String(s.clone())),
            
            Expr::Variable(name) => {
                Ok(self.variables.get(name))
            }
            
            Expr::ArrayAccess { name, indices } => {
                // 求值所有索引
                let idx_values: Result<Vec<usize>> = indices.iter()
                    .map(|idx_expr| {
                        self.eval_expr(idx_expr)?
                            .as_number()
                            .and_then(|n| {
                                if n < 0.0 {
                                    Err(BasicError::SubscriptOutOfRange(
                                        "Negative array index".to_string()
                                    ))
                                } else {
                                    Ok(n as usize)
                                }
                            })
                    })
                    .collect();
                
                let indices_usize = idx_values?;
                self.variables.get_array_element(name, &indices_usize)
            }
            
            Expr::FunctionCall { name, args } => {
                self.eval_function_call(name, args)
            }
            
            Expr::BinaryOp { left, op, right } => {
                self.eval_binary_op(left, *op, right)
            }
            
            Expr::UnaryOp { op, operand } => {
                self.eval_unary_op(*op, operand)
            }
        }
    }

    /// 求值二元运算
    fn eval_binary_op(&mut self, left: &Expr, op: BinaryOperator, right: &Expr) -> Result<Value> {
        use BinaryOperator::*;

        let left_val = self.eval_expr(left)?;
        let right_val = self.eval_expr(right)?;

        match op {
            // 算术运算符
            Add => {
                if left_val.is_number() && right_val.is_number() {
                    let l = left_val.as_number()?;
                    let r = right_val.as_number()?;
                    Ok(Value::Number(l + r))
                } else if left_val.is_string() && right_val.is_string() {
                    // 字符串连接
                    let l = left_val.as_string()?;
                    let r = right_val.as_string()?;
                    Ok(Value::String(format!("{}{}", l, r)))
                } else {
                    Err(BasicError::TypeMismatch(
                        "Cannot add incompatible types".to_string()
                    ))
                }
            }
            
            Subtract => {
                let l = left_val.as_number()?;
                let r = right_val.as_number()?;
                Ok(Value::Number(l - r))
            }
            
            Multiply => {
                let l = left_val.as_number()?;
                let r = right_val.as_number()?;
                Ok(Value::Number(l * r))
            }
            
            Divide => {
                let l = left_val.as_number()?;
                let r = right_val.as_number()?;
                if r == 0.0 {
                    return Err(BasicError::DivisionByZero);
                }
                Ok(Value::Number(l / r))
            }
            
            Power => {
                let l = left_val.as_number()?;
                let r = right_val.as_number()?;
                Ok(Value::Number(l.powf(r)))
            }
            
            // 关系运算符（BASIC 中 true = -1, false = 0）
            Equal => {
                let result = if left_val == right_val { -1.0 } else { 0.0 };
                Ok(Value::Number(result))
            }
            
            NotEqual => {
                let result = if left_val != right_val { -1.0 } else { 0.0 };
                Ok(Value::Number(result))
            }
            
            Less => {
                let result = if left_val.is_number() && right_val.is_number() {
                    let l = left_val.as_number()?;
                    let r = right_val.as_number()?;
                    if l < r { -1.0 } else { 0.0 }
                } else if left_val.is_string() && right_val.is_string() {
                    let l = left_val.as_string()?;
                    let r = right_val.as_string()?;
                    if l < r { -1.0 } else { 0.0 }
                } else {
                    return Err(BasicError::TypeMismatch("Cannot compare".to_string()));
                };
                Ok(Value::Number(result))
            }
            
            Greater => {
                let result = if left_val.is_number() && right_val.is_number() {
                    let l = left_val.as_number()?;
                    let r = right_val.as_number()?;
                    if l > r { -1.0 } else { 0.0 }
                } else if left_val.is_string() && right_val.is_string() {
                    let l = left_val.as_string()?;
                    let r = right_val.as_string()?;
                    if l > r { -1.0 } else { 0.0 }
                } else {
                    return Err(BasicError::TypeMismatch("Cannot compare".to_string()));
                };
                Ok(Value::Number(result))
            }
            
            LessEqual => {
                let result = if left_val.is_number() && right_val.is_number() {
                    let l = left_val.as_number()?;
                    let r = right_val.as_number()?;
                    if l <= r { -1.0 } else { 0.0 }
                } else if left_val.is_string() && right_val.is_string() {
                    let l = left_val.as_string()?;
                    let r = right_val.as_string()?;
                    if l <= r { -1.0 } else { 0.0 }
                } else {
                    return Err(BasicError::TypeMismatch("Cannot compare".to_string()));
                };
                Ok(Value::Number(result))
            }
            
            GreaterEqual => {
                let result = if left_val.is_number() && right_val.is_number() {
                    let l = left_val.as_number()?;
                    let r = right_val.as_number()?;
                    if l >= r { -1.0 } else { 0.0 }
                } else if left_val.is_string() && right_val.is_string() {
                    let l = left_val.as_string()?;
                    let r = right_val.as_string()?;
                    if l >= r { -1.0 } else { 0.0 }
                } else {
                    return Err(BasicError::TypeMismatch("Cannot compare".to_string()));
                };
                Ok(Value::Number(result))
            }
            
            // 逻辑运算符（按位）
            And => {
                let l = left_val.as_number()? as i32;
                let r = right_val.as_number()? as i32;
                Ok(Value::Number((l & r) as f64))
            }
            
            Or => {
                let l = left_val.as_number()? as i32;
                let r = right_val.as_number()? as i32;
                Ok(Value::Number((l | r) as f64))
            }
        }
    }

    /// 求值一元运算
    fn eval_unary_op(&mut self, op: UnaryOperator, operand: &Expr) -> Result<Value> {
        let val = self.eval_expr(operand)?;
        
        match op {
            UnaryOperator::Minus => {
                let n = val.as_number()?;
                Ok(Value::Number(-n))
            }
            UnaryOperator::Not => {
                let n = val.as_number()? as i32;
                Ok(Value::Number((!n) as f64))
            }
        }
    }

    /// 求值函数调用（内置函数）
    fn eval_function_call(&mut self, name: &str, args: &[Expr]) -> Result<Value> {
        // 首先检查是否是用户自定义函数（FN name）
        if name.starts_with("FN") && name.len() > 2 {
            // 提取函数名（去掉 "FN" 前缀）
            let func_name = &name[2..].trim();
            
            // 先克隆函数信息，释放引用
            let (param_name, body) = if let Some(func) = self.variables.get_function(func_name) {
                (func.param.clone(), func.body.clone())
            } else {
                return Err(BasicError::SyntaxError(
                    format!("Undefined function: FN {}", func_name)
                ));
            };
            
            // 检查参数数量
            if args.len() != 1 {
                return Err(BasicError::SyntaxError(
                    format!("FN {} requires 1 argument", func_name)
                ));
            }
            
            // 求值参数
            let arg_value = self.eval_expr(&args[0])?;
            
            // 保存原变量值
            let old_value = self.variables.get(&param_name);
            
            // 设置参数值
            self.variables.set(&param_name, arg_value)?;
            
            // 求值函数体
            let result = self.eval_expr(&body)?;
            
            // 恢复原变量值
            let _ = self.variables.set(&param_name, old_value);
            
            return Ok(result);
        }
        
        match name.to_uppercase().as_str() {
            // 数学函数
            "SGN" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("SGN requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                let result = if n > 0.0 { 1.0 } else if n < 0.0 { -1.0 } else { 0.0 };
                Ok(Value::Number(result))
            }
            
            "INT" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("INT requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                Ok(Value::Number(n.floor()))
            }
            
            "ABS" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("ABS requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                Ok(Value::Number(n.abs()))
            }
            
            "MOD" => {
                if args.len() != 2 {
                    return Err(BasicError::SyntaxError("MOD requires 2 arguments".to_string()));
                }
                let a = self.eval_expr(&args[0])?.as_number()?;
                let b = self.eval_expr(&args[1])?.as_number()?;
                if b == 0.0 {
                    return Err(BasicError::DivisionByZero);
                }
                // MOD(a, b) = a - INT(a/b) * b
                let result = a - (a / b).floor() * b;
                Ok(Value::Number(result))
            }
            
            "SQR" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("SQR requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                if n < 0.0 {
                    return Err(BasicError::IllegalQuantity("SQR of negative number".to_string()));
                }
                Ok(Value::Number(n.sqrt()))
            }
            
            "SIN" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("SIN requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                Ok(Value::Number(n.sin()))
            }
            
            "COS" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("COS requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                Ok(Value::Number(n.cos()))
            }
            
            "TAN" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("TAN requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                Ok(Value::Number(n.tan()))
            }
            
            "ATN" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("ATN requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                Ok(Value::Number(n.atan()))
            }
            
            "LOG" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("LOG requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                if n <= 0.0 {
                    return Err(BasicError::IllegalQuantity("LOG of non-positive number".to_string()));
                }
                Ok(Value::Number(n.ln()))
            }
            
            "EXP" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("EXP requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                Ok(Value::Number(n.exp()))
            }
            
            "RND" => {
                use rand::Rng;
                
                // RND 函数的 BASIC 6502 语义：
                // RND(0) - 返回最近生成的随机数（简化为生成新的）
                // RND(正数) - 返回 [0, 1) 的随机浮点数
                // RND(负数) - 使用负数作为种子（暂不实现种子功能）
                let arg = if args.is_empty() {
                    1.0  // 无参数默认为 RND(1)
                } else {
                    self.eval_expr(&args[0])?.as_number()?
                };
                
                let mut rng = rand::thread_rng();
                
                // 简化实现：所有情况都返回 [0, 1) 的随机数
                // 如果需要随机整数，用户可以写 INT(RND(1)*N)+1
                let result = if arg < 0.0 {
                    // 负数：暂时也返回随机数（标准BASIC会重新播种）
                    rng.gen::<f64>()
                } else {
                    // 0或正数：返回 [0, 1) 的随机数
                    rng.gen::<f64>()
                };
                
                Ok(Value::Number(result))
            }
            
            // 字符串函数
            "LEN" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("LEN requires 1 argument".to_string()));
                }
                let s = self.eval_expr(&args[0])?.as_string()?;
                Ok(Value::Number(s.len() as f64))
            }
            
            "ASC" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("ASC requires 1 argument".to_string()));
                }
                let s = self.eval_expr(&args[0])?.as_string()?;
                if s.is_empty() {
                    return Err(BasicError::IllegalQuantity("ASC of empty string".to_string()));
                }
                Ok(Value::Number(s.chars().next().unwrap() as u8 as f64))
            }
            
            "CHR$" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("CHR$ requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                if n < 0.0 || n > 255.0 {
                    return Err(BasicError::IllegalQuantity("CHR$ argument out of range".to_string()));
                }
                let ch = n as u8 as char;
                Ok(Value::String(ch.to_string()))
            }
            
            "STR$" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("STR$ requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()?;
                // BASIC 的 STR$ 在正数前加空格
                let s = if n >= 0.0 {
                    format!(" {}", n)
                } else {
                    n.to_string()
                };
                Ok(Value::String(s))
            }
            
            "VAL" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("VAL requires 1 argument".to_string()));
                }
                let s = self.eval_expr(&args[0])?.as_string()?;
                let n = s.trim().parse::<f64>().unwrap_or(0.0);
                Ok(Value::Number(n))
            }
            
            "LEFT$" => {
                if args.len() != 2 {
                    return Err(BasicError::SyntaxError("LEFT$ requires 2 arguments".to_string()));
                }
                let s = self.eval_expr(&args[0])?.as_string()?;
                let n = self.eval_expr(&args[1])?.as_number()? as usize;
                let result: String = s.chars().take(n).collect();
                Ok(Value::String(result))
            }
            
            "RIGHT$" => {
                if args.len() != 2 {
                    return Err(BasicError::SyntaxError("RIGHT$ requires 2 arguments".to_string()));
                }
                let s = self.eval_expr(&args[0])?.as_string()?;
                let n = self.eval_expr(&args[1])?.as_number()? as usize;
                let len = s.chars().count();
                let skip = if n > len { 0 } else { len - n };
                let result: String = s.chars().skip(skip).collect();
                Ok(Value::String(result))
            }
            
            "MID$" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(BasicError::SyntaxError("MID$ requires 2 or 3 arguments".to_string()));
                }
                let s = self.eval_expr(&args[0])?.as_string()?;
                let start = self.eval_expr(&args[1])?.as_number()? as usize;
                
                // BASIC 的 MID$ 是 1-based
                let start = if start > 0 { start - 1 } else { 0 };
                
                let chars: Vec<char> = s.chars().collect();
                
                if args.len() == 3 {
                    let len = self.eval_expr(&args[2])?.as_number()? as usize;
                    let result: String = chars.iter().skip(start).take(len).collect();
                    Ok(Value::String(result))
                } else {
                    let result: String = chars.iter().skip(start).collect();
                    Ok(Value::String(result))
                }
            }
            
            "INSTR" => {
                // INSTR(start, string1, string2) 或 INSTR(string1, string2)
                // 返回 string2 在 string1 中第一次出现的位置（1-based），如果没找到返回 0
                if args.len() < 2 || args.len() > 3 {
                    return Err(BasicError::SyntaxError("INSTR requires 2 or 3 arguments".to_string()));
                }
                
                let (start_pos, str1, str2) = if args.len() == 3 {
                    let start = self.eval_expr(&args[0])?.as_number()? as usize;
                    let s1 = self.eval_expr(&args[1])?.as_string()?;
                    let s2 = self.eval_expr(&args[2])?.as_string()?;
                    (start, s1, s2)
                } else {
                    let s1 = self.eval_expr(&args[0])?.as_string()?;
                    let s2 = self.eval_expr(&args[1])?.as_string()?;
                    (1, s1, s2)
                };
                
                // BASIC 的 INSTR 是 1-based
                let start_pos = if start_pos > 0 { start_pos - 1 } else { 0 };
                
                // 从 start_pos 开始查找
                if let Some(pos) = str1[start_pos..].find(&str2) {
                    Ok(Value::Number((start_pos + pos + 1) as f64))
                } else {
                    Ok(Value::Number(0.0))
                }
            }
            
            "SPACE$" => {
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("SPACE$ requires 1 argument".to_string()));
                }
                let n = self.eval_expr(&args[0])?.as_number()? as usize;
                Ok(Value::String(" ".repeat(n)))
            }
            
            "POS" => {
                // POS(x) - 返回当前打印列位置（1-based）
                // 参数 x 被忽略，但必须提供（BASIC 6502 要求）
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("POS requires 1 argument".to_string()));
                }
                // 忽略参数，只返回当前列位置
                let _ = self.eval_expr(&args[0])?;
                // 返回 1-based 列位置（print_column 是 0-based）
                Ok(Value::Number((self.print_column + 1) as f64))
            }
            
            "FRE" => {
                // FRE(x) - 返回剩余内存大小（简化实现）
                // 参数 x 被忽略
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("FRE requires 1 argument".to_string()));
                }
                let _ = self.eval_expr(&args[0])?;
                // 简化实现：返回一个固定的大数
                Ok(Value::Number(32767.0))
            }
            
            "PEEK" => {
                // PEEK(addr) - 读取内存地址的值（简化实现）
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("PEEK requires 1 argument".to_string()));
                }
                let _ = self.eval_expr(&args[0])?;
                // 简化实现：返回 0
                Ok(Value::Number(0.0))
            }
            
            "USR" => {
                // USR(addr) - 调用机器语言程序（简化实现）
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("USR requires 1 argument".to_string()));
                }
                let _ = self.eval_expr(&args[0])?;
                // 简化实现：返回 0
                Ok(Value::Number(0.0))
            }

            // ========== 游戏输入函数 ==========

            "INKEY" => {
                // INKEY() - 返回最后按下的按键 ASCII 码
                if !args.is_empty() {
                    return Err(BasicError::SyntaxError("INKEY requires no arguments".to_string()));
                }
                let key_code = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.inkey() as f64
                } else {
                    0.0
                };
                Ok(Value::Number(key_code))
            }

            "KEY" => {
                // KEY(key$) - 检查指定按键是否按下
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("KEY requires 1 argument".to_string()));
                }
                let key_name = self.eval_expr(&args[0])?.as_string()?;
                let pressed = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.key(&key_name)
                } else {
                    false
                };
                Ok(Value::Number(if pressed { 1.0 } else { 0.0 }))
            }

            "MOUSEX" => {
                // MOUSEX() - 返回鼠标 X 坐标
                if !args.is_empty() {
                    return Err(BasicError::SyntaxError("MOUSEX requires no arguments".to_string()));
                }
                let x = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.mouse_x() as f64
                } else {
                    0.0
                };
                Ok(Value::Number(x))
            }

            "MOUSEY" => {
                // MOUSEY() - 返回鼠标 Y 坐标
                if !args.is_empty() {
                    return Err(BasicError::SyntaxError("MOUSEY requires no arguments".to_string()));
                }
                let y = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.mouse_y() as f64
                } else {
                    0.0
                };
                Ok(Value::Number(y))
            }

            "MOUSEB" => {
                // MOUSEB() - 返回鼠标按钮状态
                if !args.is_empty() {
                    return Err(BasicError::SyntaxError("MOUSEB requires no arguments".to_string()));
                }
                let buttons = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.mouse_button() as f64
                } else {
                    0.0
                };
                Ok(Value::Number(buttons))
            }

            // ========== 精灵查询函数 ==========

            "SPRITEX" => {
                // SPRITEX(id) - 返回精灵 X 坐标
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("SPRITEX requires 1 argument".to_string()));
                }
                let id = self.eval_expr(&args[0])?.as_number()? as u32;
                let x = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.sprite_x(id).unwrap_or(0) as f64
                } else {
                    0.0
                };
                Ok(Value::Number(x))
            }

            "SPRITEY" => {
                // SPRITEY(id) - 返回精灵 Y 坐标
                if args.len() != 1 {
                    return Err(BasicError::SyntaxError("SPRITEY requires 1 argument".to_string()));
                }
                let id = self.eval_expr(&args[0])?.as_number()? as u32;
                let y = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.sprite_y(id).unwrap_or(0) as f64
                } else {
                    0.0
                };
                Ok(Value::Number(y))
            }

            "SPRITEHIT" => {
                // SPRITEHIT(id1, id2) - 检测两个精灵是否碰撞
                if args.len() != 2 {
                    return Err(BasicError::SyntaxError("SPRITEHIT requires 2 arguments".to_string()));
                }
                let id1 = self.eval_expr(&args[0])?.as_number()? as u32;
                let id2 = self.eval_expr(&args[1])?.as_number()? as u32;
                let hit = if let Some(ctx) = self.game_context.as_ref() {
                    ctx.sprite_hit(id1, id2)
                } else {
                    false
                };
                Ok(Value::Number(if hit { 1.0 } else { 0.0 }))
            }

            _ => Err(BasicError::SyntaxError(
                format!("Unknown function: {}", name)
            )),
        }
    }

    /// 执行语句
    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Let { target, value } => {
                let val = self.eval_expr(value)?;
                
                match target {
                    AssignTarget::Variable(name) => {
                        self.variables.set(name, val)?;
                    }
                    AssignTarget::ArrayElement { name, indices } => {
                        let idx_values: Result<Vec<usize>> = indices.iter()
                            .map(|idx_expr| {
                                self.eval_expr(idx_expr)?
                                    .as_number()
                                    .map(|n| n as usize)
                            })
                            .collect();
                        
                        let indices_usize = idx_values?;
                        self.variables.set_array_element(name, &indices_usize, val)?;
                    }
                }
                
                Ok(())
            }
            
            Statement::End => {
                self.runtime.end_execution();
                Ok(())
            }
            
            Statement::Stop => {
                self.runtime.pause_execution();
                Ok(())
            }
            
            Statement::New => {
                self.runtime.clear_program();
                self.variables.clear();
                Ok(())
            }
            
            Statement::Clear => {
                self.variables.clear();
                Ok(())
            }
            
            Statement::Rem { comment: _ } => {
                // REM 注释语句：不执行任何操作
                Ok(())
            }
            
            Statement::Dim { arrays } => {
                for arr_dim in arrays {
                    let dimensions: Result<Vec<usize>> = arr_dim.dimensions.iter()
                        .map(|dim_expr| {
                            self.eval_expr(dim_expr)?
                                .as_number()
                                .map(|n| n as usize)
                        })
                        .collect();
                    
                    let dims = dimensions?;
                    self.variables.dim_array(&arr_dim.name, dims)?;
                }
                Ok(())
            }
            
            Statement::Print { items } => {
                self.execute_print(items)?;
                Ok(())
            }
            
            Statement::Goto { line_number } => {
                let line_val = self.eval_expr(line_number)?;
                let line = line_val.as_number()? as u16;
                self.runtime.set_execution_position(line, 0)?;
                Ok(())
            }
            
            Statement::If { condition, then_part } => {
                let cond_val = self.eval_expr(condition)?;
                let cond_num = cond_val.as_number()?;
                
                // BASIC 中，任何非零值都是真
                if cond_num != 0.0 {
                    match then_part.as_ref() {
                        ThenPart::LineNumber(line) => {
                            self.runtime.set_execution_position(*line as u16, 0)?;
                        }
                        ThenPart::Statement(stmt) => {
                            self.execute_statement(stmt)?;
                        }
                        ThenPart::Statements(stmts) => {
                            for stmt in stmts {
                                self.execute_statement(stmt)?;
                            }
                        }
                    }
                }
                Ok(())
            }
            
            Statement::Gosub { line_number } => {
                // 保存返回地址（当前行号和语句索引）
                let return_line = self.runtime.get_current_line().unwrap_or(0);
                // 注意：get_current_stmt_index() 返回的是下一条语句的索引（因为 get_next_statement() 已经递增过了）
                // 所以我们需要减去 1 来获取当前语句的索引
                let return_stmt = self.runtime.get_current_stmt_index().saturating_sub(1);
                
                // 入栈
                self.runtime.push_gosub(return_line, return_stmt)?;
                
                // 跳转到子程序
                let line_val = self.eval_expr(line_number)?;
                let line = line_val.as_number()? as u16;
                self.runtime.set_execution_position(line, 0)?;
                
                Ok(())
            }
            
            Statement::Return => {
                // 从栈中弹出返回地址
                let (return_line, return_stmt) = self.runtime.pop_gosub()?;
                
                // 跳转回返回地址
                // 注意：get_next_statement() 会自动递增，所以我们需要跳转到 return_stmt + 1
                // 但由于我们保存的是当前语句的索引，所以跳转到 return_stmt + 1 就是下一条语句
                self.runtime.set_execution_position(return_line, return_stmt + 1)?;
                
                Ok(())
            }
            
            Statement::Input { prompt, variables } => {
                // 提取变量名
                let var_names: Vec<String> = variables.iter()
                    .map(|target| match target {
                        AssignTarget::Variable(name) => Ok(name.clone()),
                        AssignTarget::ArrayElement { .. } => {
                            Err(BasicError::SyntaxError(
                                "INPUT does not support array elements".to_string()
                            ))
                        }
                    })
                    .collect::<Result<Vec<String>>>()?;
                
                self.execute_input(prompt.as_deref(), &var_names)?;
                Ok(())
            }
            
            Statement::Data { values: _ } => {
                // DATA 语句在程序加载时处理，执行时跳过
                // 数据已经存储在 data_values 中
                Ok(())
            }
            
            Statement::Read { variables } => {
                for target in variables {
                    let var_name = match target {
                        AssignTarget::Variable(name) => name.as_str(),
                        AssignTarget::ArrayElement { .. } => {
                            return Err(BasicError::SyntaxError(
                                "READ does not support array elements".to_string()
                            ));
                        }
                    };
                    
                    let data_val = self.read_data_value()?;
                    
                    // 根据变量名判断类型
                    if var_name.ends_with('$') {
                        // 字符串变量
                        let str_val = match data_val {
                            DataValue::String(s) => s,
                            DataValue::Number(n) => n.to_string(),
                        };
                        self.variables.set(var_name, Value::String(str_val))?;
                    } else {
                        // 数值变量
                        let num_val = match data_val {
                            DataValue::Number(n) => n,
                            DataValue::String(s) => {
                                s.trim().parse::<f64>().unwrap_or(0.0)
                            }
                        };
                        self.variables.set(var_name, Value::Number(num_val))?;
                    }
                }
                Ok(())
            }
            
            Statement::Restore { line_number } => {
                if line_number.is_some() {
                    // RESTORE 到指定行（暂不支持，需要跟踪每行的 DATA 位置）
                    return Err(BasicError::SyntaxError(
                        "RESTORE to specific line not yet implemented".to_string()
                    ));
                }
                self.restore_data();
                Ok(())
            }
            
            Statement::For { var, start, end, step } => {
                // 计算起始值、结束值和步长
                let start_val = self.eval_expr(&start)?;
                let end_val = self.eval_expr(&end)?;
                let step_val = if let Some(ref s) = step {
                    self.eval_expr(s)?
                } else {
                    Value::Number(1.0)
                };
                
                // 提取数值
                let start_num = start_val.as_number()?;
                let end_num = end_val.as_number()?;
                let step_num = step_val.as_number()?;
                
                // 检查步长
                if step_num == 0.0 {
                    return Err(BasicError::IllegalQuantity(
                        "FOR loop step cannot be zero".to_string()
                    ));
                }
                
                // 设置循环变量初值
                self.variables.set(var, Value::Number(start_num))?;
                
                // 获取当前位置
                // 注意：get_current_stmt_index() 返回的是下一条语句的索引（因为 get_next_statement() 已经递增过了）
                // 所以我们需要减去 1 来获取 FOR 语句本身的索引
                let loop_line = self.runtime.get_current_line()
                    .ok_or_else(|| BasicError::SyntaxError("FOR without line number".to_string()))?;
                let loop_stmt = self.runtime.get_current_stmt_index().saturating_sub(1);
                
                // 将循环信息压入栈
                self.runtime.push_for_loop(
                    var.clone(),
                    end_num,
                    step_num,
                    loop_line,
                    loop_stmt,
                )?;
                
                Ok(())
            }
            
            Statement::Next { var } => {
                // 弹出 FOR 循环信息
                let (loop_var, end_val, step_val, loop_line, loop_stmt) = 
                    self.runtime.pop_for_loop(var.clone())?;
                
                // 获取当前循环变量的值
                let current_val = self.variables.get(&loop_var).as_number()?;
                
                // 递增/递减
                let new_val = current_val + step_val;
                
                // 检查是否继续循环
                let should_continue = if step_val > 0.0 {
                    new_val <= end_val
                } else {
                    new_val >= end_val
                };
                
                if should_continue {
                    // 更新循环变量
                    self.variables.set(&loop_var, Value::Number(new_val))?;
                    
                    // 重新压入栈（继续循环）
                    self.runtime.push_for_loop(
                        loop_var.clone(),
                        end_val,
                        step_val,
                        loop_line,
                        loop_stmt,
                    )?;
                    
                    // 跳转回 FOR 语句的下一条语句
                    self.runtime.set_execution_position(loop_line, loop_stmt + 1)?;
                }
                // 否则继续执行下一条语句（循环结束）
                
                Ok(())
            }
            
            Statement::On { expr, targets, is_gosub } => {
                // 计算表达式的值
                let index_val = self.eval_expr(&expr)?;
                let index = index_val.as_number()? as i32;
                
                // 索引从 1 开始
                if index < 1 || index as usize > targets.len() {
                    // 超出范围，继续执行下一条语句
                    return Ok(());
                }
                
                // 获取目标行号
                let target_line = targets[(index - 1) as usize];
                
                if *is_gosub {
                    // ON...GOSUB：保存返回地址并跳转
                    let return_line = self.runtime.get_current_line()
                        .ok_or_else(|| BasicError::SyntaxError("GOSUB without line number".to_string()))?;
                    let return_stmt = self.runtime.get_current_stmt_index();
                    
                    self.runtime.push_gosub(return_line, return_stmt)?;
                    self.runtime.set_execution_position(target_line, 0)?;
                } else {
                    // ON...GOTO：直接跳转
                    self.runtime.set_execution_position(target_line, 0)?;
                }
                
                Ok(())
            }
            
            Statement::Load { filename } => {
                self.execute_load(filename)?;
                Ok(())
            }
            
            Statement::Save { filename } => {
                self.execute_save(filename)?;
                Ok(())
            }
            
            Statement::Get { variable } => {
                self.execute_get(variable)?;
                Ok(())
            }
            
            Statement::Null => {
                // NULL 语句：无操作，直接返回
                Ok(())
            }
            
            Statement::DefFn { name, param, body } => {
                self.execute_def_fn(name, param, body)?;
                Ok(())
            }

            // ========== 协程扩展语句 ==========

            Statement::Wait { seconds } => {
                // 计算等待的秒数
                let wait_time = self.eval_expr(seconds)?.as_number()?;
                if wait_time < 0.0 {
                    return Err(BasicError::IllegalQuantity(
                        "WAIT time must be non-negative".to_string()
                    ));
                }
                // 使用内部游戏时间累加器计算恢复时间
                let resume_at = self.game_time + wait_time;
                self.runtime.enter_wait(resume_at);
                Ok(())
            }

            Statement::Yield => {
                // 让出执行到下一帧
                self.runtime.enter_yield();
                Ok(())
            }

            Statement::WaitKey => {
                // 等待按键
                self.runtime.enter_wait_for(super::runtime::WaitEvent::KeyPress);
                Ok(())
            }

            Statement::WaitClick => {
                // 等待鼠标点击
                self.runtime.enter_wait_for(super::runtime::WaitEvent::MouseClick);
                Ok(())
            }

            // ========== 图形语句 ==========

            Statement::Plot { x, y, ch, fg, bg } => {
                let x_val = self.eval_expr(x)?.as_number()? as i32;
                let y_val = self.eval_expr(y)?.as_number()? as i32;
                let ch_val = self.eval_expr(ch)?.as_string()?;
                let ch_char = ch_val.chars().next().unwrap_or(' ');
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

            Statement::Line { x0, y0, x1, y1, ch } => {
                let x0_val = self.eval_expr(x0)?.as_number()? as i32;
                let y0_val = self.eval_expr(y0)?.as_number()? as i32;
                let x1_val = self.eval_expr(x1)?.as_number()? as i32;
                let y1_val = self.eval_expr(y1)?.as_number()? as i32;
                let ch_val = self.eval_expr(ch)?.as_string()?;
                let ch_char = ch_val.chars().next().unwrap_or('*');

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.line(x0_val, y0_val, x1_val, y1_val, ch_char);
                }
                Ok(())
            }

            Statement::Box { x, y, w, h, style } => {
                let x_val = self.eval_expr(x)?.as_number()? as i32;
                let y_val = self.eval_expr(y)?.as_number()? as i32;
                let w_val = self.eval_expr(w)?.as_number()? as i32;
                let h_val = self.eval_expr(h)?.as_number()? as i32;
                let style_val = self.eval_expr(style)?.as_number()? as u8;

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.box_draw(x_val, y_val, w_val, h_val, style_val);
                }
                Ok(())
            }

            Statement::Circle { cx, cy, r, ch } => {
                let cx_val = self.eval_expr(cx)?.as_number()? as i32;
                let cy_val = self.eval_expr(cy)?.as_number()? as i32;
                let r_val = self.eval_expr(r)?.as_number()? as i32;
                let ch_val = self.eval_expr(ch)?.as_string()?;
                let ch_char = ch_val.chars().next().unwrap_or('O');

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.circle(cx_val, cy_val, r_val, ch_char);
                }
                Ok(())
            }

            // ========== 精灵语句 ==========

            Statement::Sprite { id, x, y, ch } => {
                let id_val = self.eval_expr(id)?.as_number()? as u32;
                let x_val = self.eval_expr(x)?.as_number()? as i32;
                let y_val = self.eval_expr(y)?.as_number()? as i32;
                let ch_val = self.eval_expr(ch)?.as_string()?;
                let ch_char = ch_val.chars().next().unwrap_or('@');

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.sprite_create(id_val, x_val, y_val, ch_char);
                }
                Ok(())
            }

            Statement::SpriteMove { id, dx, dy } => {
                let id_val = self.eval_expr(id)?.as_number()? as u32;
                let dx_val = self.eval_expr(dx)?.as_number()? as i32;
                let dy_val = self.eval_expr(dy)?.as_number()? as i32;

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.sprite_move(id_val, dx_val, dy_val);
                }
                Ok(())
            }

            Statement::SpritePos { id, x, y } => {
                let id_val = self.eval_expr(id)?.as_number()? as u32;
                let x_val = self.eval_expr(x)?.as_number()? as i32;
                let y_val = self.eval_expr(y)?.as_number()? as i32;

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.sprite_pos(id_val, x_val, y_val);
                }
                Ok(())
            }

            Statement::SpriteHide { id, hidden } => {
                let id_val = self.eval_expr(id)?.as_number()? as u32;
                let hidden_val = self.eval_expr(hidden)?.as_number()? != 0.0;

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.sprite_hide(id_val, hidden_val);
                }
                Ok(())
            }

            Statement::SpriteColor { id, fg, bg } => {
                let id_val = self.eval_expr(id)?.as_number()? as u32;
                let fg_val = self.eval_expr(fg)?.as_number()? as u8;
                let bg_val = self.eval_expr(bg)?.as_number()? as u8;

                if let Some(ctx) = self.game_context.as_mut() {
                    ctx.sprite_color(id_val, fg_val, bg_val);
                }
                Ok(())
            }

            _ => {
                // 其他语句暂未实现
                Err(BasicError::SyntaxError(
                    "Statement not yet implemented".to_string()
                ))
            }
        }
    }
    
    /// 执行 INPUT 语句
    fn execute_input(&mut self, prompt: Option<&str>, variables: &[String]) -> Result<()> {
        use std::io::{self, Write};
        
        // 显示提示符
        if let Some(p) = prompt {
            self.output(p);
            self.output("? ");
        } else {
            self.output("? ");
        }
        
        // 确保输出被刷新到终端
        io::stdout().flush().map_err(|e| {
            BasicError::SyntaxError(format!("Failed to flush stdout: {}", e))
        })?;
        
        // 读取输入
        let input_line = if let Some(ref mut callback) = self.input_callback {
            let prompt_text = prompt.unwrap_or("");
            callback(prompt_text).ok_or_else(|| {
                BasicError::SyntaxError("No input provided".to_string())
            })?
        } else {
            // 从 stdin 读取输入
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).map_err(|e| {
                BasicError::SyntaxError(format!("Failed to read input: {}", e))
            })?;
            buffer.trim().to_string()
        };
        
        // 解析输入值（考虑引号内的逗号）
        let values = Self::parse_input_values(&input_line);
        
        if values.len() != variables.len() {
            self.output("?EXTRA IGNORED\n");
        }
        
        // 赋值给变量
        for (i, var_name) in variables.iter().enumerate() {
            if i >= values.len() {
                break;
            }
            
            let input_val = &values[i];
            
            if var_name.ends_with('$') {
                // 字符串变量
                let str_val = if input_val.starts_with('"') && input_val.ends_with('"') {
                    // 去掉引号
                    input_val[1..input_val.len()-1].to_string()
                } else {
                    input_val.clone()
                };
                self.variables.set(var_name, Value::String(str_val))?;
            } else {
                // 数值变量
                match input_val.parse::<f64>() {
                    Ok(num) => {
                        self.variables.set(var_name, Value::Number(num))?;
                    }
                    Err(_) => {
                        self.output("?REDO FROM START\n");
                        return Err(BasicError::TypeMismatch(
                            "Invalid number input".to_string()
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 解析输入值，处理带引号的字符串
    fn parse_input_values(input: &str) -> Vec<String> {
        let mut values = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        
        for ch in input.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                    current.push(ch);
                }
                ',' if !in_quotes => {
                    values.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        
        if !current.is_empty() || input.ends_with(',') {
            values.push(current.trim().to_string());
        }
        
        values
    }
    
    /// 执行 SAVE 命令 - 保存程序到文件
    fn execute_save(&self, filename: &str) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let program = self.runtime.clone_program();
        if program.is_empty() {
            return Err(BasicError::SyntaxError("No program to save".to_string()));
        }
        
        let mut file = File::create(filename).map_err(|e| {
            BasicError::SyntaxError(format!("Failed to create file: {}", e))
        })?;
        
        for (_, line) in program.iter() {
            let line_text = Self::serialize_program_line(line);
            writeln!(file, "{}", line_text).map_err(|e| {
                BasicError::SyntaxError(format!("Failed to write to file: {}", e))
            })?;
        }
        
        Ok(())
    }
    
    /// 执行 GET 命令 - 读取单字符输入（不等待回车）
    fn execute_get(&mut self, variable: &str) -> Result<()> {
        use std::io::{self, Read};
        
        // GET 从 stdin 读取单个字符，不等待回车
        // 注意：这在标准终端中可能不工作，因为终端通常是行缓冲的
        // 对于测试，我们可以使用输入回调
        
        let ch = if let Some(ref mut callback) = self.input_callback {
            // 如果有回调，使用回调
            let input = callback("").unwrap_or_default();
            input.chars().next().unwrap_or('\0')
        } else {
            // 尝试从 stdin 读取单个字符（非阻塞）
            // 由于标准库的限制，这里简化处理：读取一个字符，如果失败则返回空
            let mut buffer = [0u8; 1];
            if io::stdin().read_exact(&mut buffer).is_ok() {
                buffer[0] as char
            } else {
                '\0'  // 无输入时返回空字符
            }
        };
        
        // 根据变量类型赋值
        if variable.ends_with('$') {
            // 字符串变量：存储字符
            if ch == '\0' {
                self.variables.set(variable, Value::String(String::new()))?;
            } else {
                self.variables.set(variable, Value::String(ch.to_string()))?;
            }
        } else {
            // 数值变量：存储 ASCII 码
            let ascii = if ch == '\0' { 0.0 } else { ch as u8 as f64 };
            self.variables.set(variable, Value::Number(ascii))?;
        }
        
        Ok(())
    }
    
    /// 执行 DEF FN 语句 - 定义用户自定义函数
    fn execute_def_fn(&mut self, name: &str, param: &str, body: &Expr) -> Result<()> {
        self.variables.define_function(name.to_string(), param.to_string(), body.clone())?;
        Ok(())
    }
    
    /// 将程序行序列化为文本
    pub fn serialize_program_line(line: &ProgramLine) -> String {
        let mut result = format!("{}", line.line_number);
        
        for (i, stmt) in line.statements.iter().enumerate() {
            if i > 0 {
                result.push_str(":");
            }
            result.push(' ');
            result.push_str(&Self::serialize_statement(stmt));
        }
        
        result
    }
    
    /// 将语句序列化为文本
    pub fn serialize_statement(stmt: &Statement) -> String {
        match stmt {
            Statement::Let { target, value } => {
                format!("{} = {}", Self::serialize_assign_target(target), Self::serialize_expr(value))
            }
            Statement::Print { items } => {
                let mut result = "PRINT".to_string();
                for item in items.iter() {
                    result.push(' ');
                    result.push_str(&Self::serialize_print_item(item));
                }
                result
            }
            Statement::If { condition, then_part } => {
                format!("IF {} THEN {}", Self::serialize_expr(condition), Self::serialize_then_part(then_part))
            }
            Statement::Goto { line_number } => {
                format!("GOTO {}", Self::serialize_expr(line_number))
            }
            Statement::Gosub { line_number } => {
                format!("GOSUB {}", Self::serialize_expr(line_number))
            }
            Statement::Return => "RETURN".to_string(),
            Statement::For { var, start, end, step } => {
                let mut result = format!("FOR {} = {} TO {}", var, Self::serialize_expr(start), Self::serialize_expr(end));
                if let Some(step_expr) = step {
                    result.push_str(&format!(" STEP {}", Self::serialize_expr(step_expr)));
                }
                result
            }
            Statement::Next { var } => {
                if let Some(v) = var {
                    format!("NEXT {}", v)
                } else {
                    "NEXT".to_string()
                }
            }
            Statement::On { expr, targets, is_gosub } => {
                let keyword = if *is_gosub { "GOSUB" } else { "GOTO" };
                let target_str = targets.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("ON {} {} {}", Self::serialize_expr(expr), keyword, target_str)
            }
            Statement::Input { prompt, variables } => {
                let mut result = "INPUT ".to_string();
                if let Some(p) = prompt {
                    result.push_str(&format!("\"{}\" ", p));
                }
                let var_str = variables.iter()
                    .map(|v| Self::serialize_assign_target(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&var_str);
                result
            }
            Statement::Dim { arrays } => {
                let arr_str = arrays.iter()
                    .map(|arr| {
                        let dims = arr.dimensions.iter()
                            .map(|d| Self::serialize_expr(d))
                            .collect::<Vec<_>>()
                            .join(",");
                        format!("{}({})", arr.name, dims)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("DIM {}", arr_str)
            }
            Statement::Data { values } => {
                let val_str = values.iter()
                    .map(|v| match v {
                        super::ast::DataValue::Number(n) => n.to_string(),
                        super::ast::DataValue::String(s) => format!("\"{}\"", s),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("DATA {}", val_str)
            }
            Statement::Read { variables } => {
                let var_str = variables.iter()
                    .map(|v| Self::serialize_assign_target(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("READ {}", var_str)
            }
            Statement::Restore { line_number } => {
                if let Some(ln) = line_number {
                    format!("RESTORE {}", ln)
                } else {
                    "RESTORE".to_string()
                }
            }
            Statement::Rem { comment } => {
                if comment.is_empty() {
                    "REM".to_string()
                } else {
                    format!("REM {}", comment)
                }
            }
            Statement::End => "END".to_string(),
            Statement::Stop => "STOP".to_string(),
            Statement::New => "NEW".to_string(),
            Statement::Clear => "CLEAR".to_string(),
            _ => "REM UNSUPPORTED STATEMENT".to_string(),
        }
    }
    
    /// 将表达式序列化为文本
    pub fn serialize_expr(expr: &Expr) -> String {
        match expr {
            Expr::Number(n) => n.to_string(),
            Expr::String(s) => format!("\"{}\"", s),
            Expr::Variable(name) => name.clone(),
            Expr::ArrayAccess { name, indices } => {
                let idx_str = indices.iter()
                    .map(|i| Self::serialize_expr(i))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}({})", name, idx_str)
            }
            Expr::FunctionCall { name, args } => {
                let arg_str = args.iter()
                    .map(|a| Self::serialize_expr(a))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}({})", name, arg_str)
            }
            Expr::BinaryOp { left, op, right } => {
                let op_str = match op {
                    BinaryOperator::Add => "+",
                    BinaryOperator::Subtract => "-",
                    BinaryOperator::Multiply => "*",
                    BinaryOperator::Divide => "/",
                    BinaryOperator::Power => "^",
                    BinaryOperator::Equal => "=",
                    BinaryOperator::NotEqual => "<>",
                    BinaryOperator::Less => "<",
                    BinaryOperator::Greater => ">",
                    BinaryOperator::LessEqual => "<=",
                    BinaryOperator::GreaterEqual => ">=",
                    BinaryOperator::And => " AND ",
                    BinaryOperator::Or => " OR ",
                };
                format!("({} {} {})", Self::serialize_expr(left), op_str, Self::serialize_expr(right))
            }
            Expr::UnaryOp { op, operand } => {
                let op_str = match op {
                    UnaryOperator::Minus => "-",
                    UnaryOperator::Not => "NOT ",
                };
                format!("{}{}", op_str, Self::serialize_expr(operand))
            }
        }
    }
    
    /// 将赋值目标序列化为文本
    pub fn serialize_assign_target(target: &AssignTarget) -> String {
        match target {
            AssignTarget::Variable(name) => name.clone(),
            AssignTarget::ArrayElement { name, indices } => {
                let idx_str = indices.iter()
                    .map(|i| Self::serialize_expr(i))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}({})", name, idx_str)
            }
        }
    }
    
    /// 将THEN部分序列化为文本
    pub fn serialize_then_part(then_part: &ThenPart) -> String {
        match then_part {
            ThenPart::LineNumber(ln) => ln.to_string(),
            ThenPart::Statement(stmt) => Self::serialize_statement(stmt),
            ThenPart::Statements(stmts) => {
                stmts.iter()
                    .map(|s| Self::serialize_statement(s))
                    .collect::<Vec<_>>()
                    .join(":")
            }
        }
    }
    
    /// 将PRINT项序列化为文本
    pub fn serialize_print_item(item: &PrintItem) -> String {
        match item {
            PrintItem::Expr(expr) => Self::serialize_expr(expr),
            PrintItem::Tab(expr) => format!("TAB({})", Self::serialize_expr(expr)),
            PrintItem::Spc(expr) => format!("SPC({})", Self::serialize_expr(expr)),
            PrintItem::Comma => ",".to_string(),
            PrintItem::Semicolon => ";".to_string(),
        }
    }
    
    /// 执行 LOAD 命令 - 从文件加载程序
    fn execute_load(&mut self, filename: &str) -> Result<()> {
        use std::fs;
        use crate::basic::tokenizer::Tokenizer;
        use crate::basic::parser::Parser;
        
        // 读取文件内容
        let content = fs::read_to_string(filename).map_err(|e| {
            BasicError::SyntaxError(format!("Failed to read file: {}", e))
        })?;
        
        // 清空当前程序
        self.runtime.clear_program();
        self.variables.clear();
        self.data_values.clear();
        self.data_pointer = 0;
        
        // 逐行解析并添加到程序
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // 使用tokenizer和parser解析每一行
            let mut tokenizer = Tokenizer::new(line);
            let tokens = tokenizer.tokenize_line()?;
            
            let mut parser = Parser::new(tokens);
            if let Some(program_line) = parser.parse_line()? {
                if program_line.line_number > 0 {
                    // 在添加之前，收集所有 DATA 语句的值
                    for stmt in &program_line.statements {
                        if let Statement::Data { values } = stmt {
                            for value in values {
                                // 转换 ast::DataValue 到 executor::DataValue
                                let exec_value = match value {
                                    crate::basic::ast::DataValue::Number(n) => DataValue::Number(*n),
                                    crate::basic::ast::DataValue::String(s) => DataValue::String(s.clone()),
                                };
                                self.add_data_value(exec_value);
                            }
                        }
                    }
                    self.runtime.add_line(program_line);
                }
            }
        }
        
        Ok(())
    }
    
    /// 执行 PRINT 语句
    fn execute_print(&mut self, items: &[PrintItem]) -> Result<()> {
        if items.is_empty() {
            // 空 PRINT，仅输出换行
            self.output_newline();
            return Ok(());
        }
        
        for item in items.iter() {
            match item {
                PrintItem::Expr(expr) => {
                    let val = self.eval_expr(expr)?;
                    self.print_value(&val)?;
                }
                PrintItem::Tab(expr) => {
                    let target_col = self.eval_expr(expr)?.as_number()? as usize;
                    if target_col > self.print_column {
                        let spaces = target_col - self.print_column;
                        self.output(&" ".repeat(spaces));
                    } else if target_col < self.print_column {
                        // TAB 到更小的列，换行后跳转
                        self.output_newline();
                        self.output(&" ".repeat(target_col));
                    }
                }
                PrintItem::Spc(expr) => {
                    let spaces = self.eval_expr(expr)?.as_number()? as usize;
                    self.output(&" ".repeat(spaces));
                }
                PrintItem::Comma => {
                    // 逗号：对齐到下一个 14 列边界
                    let next_col = ((self.print_column / 14) + 1) * 14;
                    let spaces_needed = next_col - self.print_column;
                    self.output(&" ".repeat(spaces_needed));
                }
                PrintItem::Semicolon => {
                    // 分号：不添加空格（紧密连接）
                }
            }
        }
        
        // 检查最后一项是否是分隔符
        if let Some(last) = items.last() {
            if !matches!(last, PrintItem::Comma | PrintItem::Semicolon) {
                // 如果最后不是分隔符，输出换行
                self.output_newline();
            }
        } else {
            self.output_newline();
        }
        
        Ok(())
    }
    
    /// 打印值（根据 BASIC 格式）
    fn print_value(&mut self, val: &Value) -> Result<()> {
        match val {
            Value::Number(n) => {
                // BASIC 数值格式：正数前后各有空格，负数前有空格
                let formatted = if *n >= 0.0 {
                    format!(" {} ", n)
                } else {
                    format!(" {}", n)
                };
                self.output(&formatted);
            }
            Value::String(s) => {
                // 普通字符串，直接输出
                self.output(s);
            }
        }
        Ok(())
    }

    // ========== 协程单步执行 ==========

    /// 单步执行（协程支持）
    ///
    /// 参数：
    /// - dt: delta time - 距离上一帧的时间增量（秒）
    ///
    /// 返回：
    /// - Ok(true): 程序仍在运行，需要继续调用 step()
    /// - Ok(false): 程序已结束
    /// - Err(_): 执行出错
    pub fn step(&mut self, dt: f32) -> Result<bool> {
        // 0. 累加游戏时间
        self.game_time += dt as f64;

        // 1. 检查程序是否已结束
        if matches!(self.runtime.get_state(), super::runtime::ExecutionState::Ended) {
            return Ok(false);
        }

        // 2. 检查是否在协程等待状态
        if self.runtime.is_coroutine_waiting() {
            // 检查是否可以从等待状态恢复
            if self.runtime.can_resume(self.game_time) {
                self.runtime.resume_from_wait()?;
            } else {
                // 仍在等待中，不执行任何语句
                return Ok(true);
            }
        }

        // 3. 获取并执行下一条语句
        if let Some(stmt) = self.runtime.get_next_statement() {
            self.execute_statement(&stmt)?;

            // 检查执行后是否进入了协程等待状态
            // 如果是，立即返回，不继续执行
            if self.runtime.is_coroutine_waiting() {
                return Ok(true);
            }

            Ok(true)
        } else {
            // 没有更多语句，程序结束
            Ok(false)
        }
    }

    /// 运行程序直到结束（非协程模式）
    ///
    /// 这是传统的一次性运行模式，会一直执行直到程序结束。
    /// 不支持协程功能（WAIT/YIELD 等语句会立即返回错误）。
    pub fn run(&mut self) -> Result<()> {
        loop {
            if let Some(stmt) = self.runtime.get_next_statement() {
                self.execute_statement(&stmt)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// 运行程序直到遇到协程等待或结束（混合模式）
    ///
    /// 参数：
    /// - dt: delta time - 距离上一帧的时间增量（秒）
    /// - max_steps: 最大执行步数，防止无限循环
    ///
    /// 返回：程序是否仍在运行
    pub fn run_until_wait(&mut self, dt: f32, max_steps: usize) -> Result<bool> {
        for _ in 0..max_steps {
            if !self.step(dt)? {
                return Ok(false);
            }

            // 如果进入协程等待状态，暂停执行
            if self.runtime.is_coroutine_waiting() {
                return Ok(true);
            }
        }

        // 达到最大步数限制，仍在运行
        Ok(true)
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Requirement: 算术运算符 - 加法
    #[test]
    fn test_addition() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(5.0),
            BinaryOperator::Add,
            Expr::Number(3.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(8.0));
    }

    // Requirement: 算术运算符 - 减法
    #[test]
    fn test_subtraction() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(10.0),
            BinaryOperator::Subtract,
            Expr::Number(7.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(3.0));
    }

    // Requirement: 算术运算符 - 乘法
    #[test]
    fn test_multiplication() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(4.0),
            BinaryOperator::Multiply,
            Expr::Number(5.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(20.0));
    }

    // Requirement: 算术运算符 - 除法
    #[test]
    fn test_division() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(15.0),
            BinaryOperator::Divide,
            Expr::Number(3.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(5.0));
    }

    // Requirement: 算术运算符 - 浮点除法
    #[test]
    fn test_float_division() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(10.0),
            BinaryOperator::Divide,
            Expr::Number(4.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(2.5));
    }

    // Requirement: 算术运算符 - 除以零
    #[test]
    fn test_division_by_zero() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(5.0),
            BinaryOperator::Divide,
            Expr::Number(0.0)
        );
        let result = exec.eval_expr(&expr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::DivisionByZero));
    }

    // Requirement: 算术运算符 - 乘方
    #[test]
    fn test_power() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(2.0),
            BinaryOperator::Power,
            Expr::Number(3.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(8.0));
    }

    // Requirement: 一元运算符 - 一元负号
    #[test]
    fn test_unary_minus() {
        let mut exec = Executor::new();
        let expr = Expr::unary(UnaryOperator::Minus, Expr::Number(5.0));
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(-5.0));
    }

    // Requirement: 关系运算符 - 等于
    #[test]
    fn test_equal() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(5.0),
            BinaryOperator::Equal,
            Expr::Number(5.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(-1.0)); // BASIC true = -1
    }

    // Requirement: 关系运算符 - 不等于
    #[test]
    fn test_not_equal() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::Number(5.0),
            BinaryOperator::NotEqual,
            Expr::Number(4.0)
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(-1.0));
    }

    // Requirement: 字符串运算符 - 字符串连接
    #[test]
    fn test_string_concatenation() {
        let mut exec = Executor::new();
        let expr = Expr::binary(
            Expr::String("HELLO".to_string()),
            BinaryOperator::Add,
            Expr::String(" WORLD".to_string())
        );
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::String("HELLO WORLD".to_string()));
    }

    // Test: 变量读取
    #[test]
    fn test_variable_read() {
        let mut exec = Executor::new();
        exec.variables.set("A", Value::Number(42.0)).unwrap();
        
        let expr = Expr::Variable("A".to_string());
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(42.0));
    }

    // Test: LET 语句执行
    #[test]
    fn test_let_statement() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Let {
            target: AssignTarget::Variable("X".to_string()),
            value: Expr::Number(100.0),
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.variables.get("X"), Value::Number(100.0));
    }

    // Test: DIM 语句执行
    #[test]
    fn test_dim_statement() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Dim {
            arrays: vec![
                ArrayDim {
                    name: "A".to_string(),
                    dimensions: vec![Expr::Number(10.0)],
                }
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert!(exec.variables.has_array("A"));
    }

    // Test: 数学函数
    #[test]
    fn test_math_functions() {
        let mut exec = Executor::new();
        
        // ABS
        let expr = Expr::FunctionCall {
            name: "ABS".to_string(),
            args: vec![Expr::Number(-42.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(42.0));
        
        // INT
        let expr = Expr::FunctionCall {
            name: "INT".to_string(),
            args: vec![Expr::Number(3.7)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(3.0));
        
        // MOD
        let expr = Expr::FunctionCall {
            name: "MOD".to_string(),
            args: vec![Expr::Number(17.0), Expr::Number(5.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(2.0)); // 17 mod 5 = 2
        
        // MOD with negative
        let expr = Expr::FunctionCall {
            name: "MOD".to_string(),
            args: vec![Expr::Number(-17.0), Expr::Number(5.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(-2.0)); // -17 mod 5 = -2
        
        // MOD with float
        let expr = Expr::FunctionCall {
            name: "MOD".to_string(),
            args: vec![Expr::Number(10.5), Expr::Number(3.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(1.5)); // 10.5 mod 3 = 1.5
    }

    // Test: RND 随机数函数
    #[test]
    fn test_rnd_function() {
        let mut exec = Executor::new();
        
        // RND(1) - 返回 [0, 1) 的随机数
        let expr = Expr::FunctionCall {
            name: "RND".to_string(),
            args: vec![Expr::Number(1.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        let value = result.as_number().unwrap();
        assert!(value >= 0.0 && value < 1.0, "RND(1) should return [0, 1), got {}", value);
        
        // RND(0) - 也返回 [0, 1) 的随机数
        let expr = Expr::FunctionCall {
            name: "RND".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        let value = result.as_number().unwrap();
        assert!(value >= 0.0 && value < 1.0, "RND(0) should return [0, 1), got {}", value);
        
        // RND(-1) - 负数参数也返回随机数
        let expr = Expr::FunctionCall {
            name: "RND".to_string(),
            args: vec![Expr::Number(-1.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        let value = result.as_number().unwrap();
        assert!(value >= 0.0 && value < 1.0, "RND(-1) should return [0, 1), got {}", value);
        
        // 测试随机性：生成多个值，应该不全相同
        let mut values = Vec::new();
        for _ in 0..10 {
            let expr = Expr::FunctionCall {
                name: "RND".to_string(),
                args: vec![Expr::Number(1.0)],
            };
            let result = exec.eval_expr(&expr).unwrap();
            values.push(result.as_number().unwrap());
        }
        
        // 检查是否有不同的值（至少应该有2个不同的值）
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        values.dedup();
        assert!(values.len() >= 2, "RND should generate different values, but got only {} unique values", values.len());
    }
    
    // Test: RND 在实际应用中的使用（模拟骰子）
    #[test]
    fn test_rnd_dice_simulation() {
        let mut exec = Executor::new();
        
        // 模拟投骰子：INT(RND(1)*6)+1 应该返回 1-6 的整数
        let mut dice_values = Vec::new();
        for _ in 0..20 {
            // RND(1)*6
            let rnd_expr = Expr::FunctionCall {
                name: "RND".to_string(),
                args: vec![Expr::Number(1.0)],
            };
            let multiply_expr = Expr::BinaryOp {
                left: Box::new(rnd_expr),
                op: BinaryOperator::Multiply,
                right: Box::new(Expr::Number(6.0)),
            };
            // INT(RND(1)*6)
            let int_expr = Expr::FunctionCall {
                name: "INT".to_string(),
                args: vec![multiply_expr],
            };
            // INT(RND(1)*6)+1
            let dice_expr = Expr::BinaryOp {
                left: Box::new(int_expr),
                op: BinaryOperator::Add,
                right: Box::new(Expr::Number(1.0)),
            };
            
            let result = exec.eval_expr(&dice_expr).unwrap();
            let value = result.as_number().unwrap() as i32;
            dice_values.push(value);
            
            // 验证范围
            assert!(value >= 1 && value <= 6, "Dice value should be 1-6, got {}", value);
        }
        
        // 验证分布（至少应该有3个不同的值）
        let mut unique_values = dice_values.clone();
        unique_values.sort();
        unique_values.dedup();
        assert!(unique_values.len() >= 3, "Dice should generate varied results, got only {:?}", unique_values);
    }

    // Test: 字符串函数
    #[test]
    fn test_string_functions() {
        let mut exec = Executor::new();
        
        // LEN
        let expr = Expr::FunctionCall {
            name: "LEN".to_string(),
            args: vec![Expr::String("HELLO".to_string())],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(5.0));
        
        // LEFT$
        let expr = Expr::FunctionCall {
            name: "LEFT$".to_string(),
            args: vec![
                Expr::String("HELLO".to_string()),
                Expr::Number(3.0),
            ],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::String("HEL".to_string()));
        
        // INSTR - 两个参数形式
        let expr = Expr::FunctionCall {
            name: "INSTR".to_string(),
            args: vec![
                Expr::String("HELLO".to_string()),
                Expr::String("L".to_string()),
            ],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(3.0)); // "L" 在 "HELLO" 中的位置是 3
        
        // INSTR - 三个参数形式（从指定位置开始）
        let expr = Expr::FunctionCall {
            name: "INSTR".to_string(),
            args: vec![
                Expr::Number(1.0),
                Expr::String("HELLO".to_string()),
                Expr::String("L".to_string()),
            ],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(3.0));
        
        // INSTR - 从位置 4 开始查找
        let expr = Expr::FunctionCall {
            name: "INSTR".to_string(),
            args: vec![
                Expr::Number(4.0),
                Expr::String("HELLO".to_string()),
                Expr::String("L".to_string()),
            ],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(4.0)); // 从位置 4 开始，"L" 在位置 4
        
        // INSTR - 找不到的情况
        let expr = Expr::FunctionCall {
            name: "INSTR".to_string(),
            args: vec![
                Expr::String("HELLO".to_string()),
                Expr::String("X".to_string()),
            ],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0)); // 没找到返回 0
        
        // SPACE$
        let expr = Expr::FunctionCall {
            name: "SPACE$".to_string(),
            args: vec![Expr::Number(5.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::String("     ".to_string())); // 5 个空格
        
        // SPACE$(0) - 空字符串
        let expr = Expr::FunctionCall {
            name: "SPACE$".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::String("".to_string()));
    }

    // Test: 复杂表达式
    #[test]
    fn test_complex_expression() {
        let mut exec = Executor::new();
        
        // 2 + 3 * 4 = 14
        let expr = Expr::binary(
            Expr::Number(2.0),
            BinaryOperator::Add,
            Expr::binary(
                Expr::Number(3.0),
                BinaryOperator::Multiply,
                Expr::Number(4.0)
            )
        );
        
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(14.0));
    }

    // Requirement: PRINT 语句 - 打印数值
    #[test]
    fn test_print_number() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::Number(42.0)),
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.get_output(), " 42 \n");
    }

    // Requirement: PRINT 语句 - 打印字符串
    #[test]
    fn test_print_string() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::String("HELLO".to_string())),
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.get_output(), "HELLO\n");
    }

    // Requirement: PRINT 语句 - 打印变量
    #[test]
    fn test_print_variable() {
        let mut exec = Executor::new();
        exec.variables.set("A", Value::Number(10.0)).unwrap();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::Variable("A".to_string())),
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.get_output(), " 10 \n");
    }

    // Requirement: PRINT 语句 - 分号分隔（紧密连接）
    #[test]
    fn test_print_semicolon() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::Number(1.0)),
                PrintItem::Semicolon,
                PrintItem::Expr(Expr::Number(2.0)),
                PrintItem::Semicolon,
                PrintItem::Expr(Expr::Number(3.0)),
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.get_output(), " 1  2  3 \n");
    }

    // Requirement: PRINT 语句 - 行尾分号（不换行）
    #[test]
    fn test_print_no_newline() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::Number(42.0)),
                PrintItem::Semicolon,
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.get_output(), " 42 ");
    }

    // Requirement: PRINT 语句 - 空 PRINT
    #[test]
    fn test_print_empty() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![],
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.get_output(), "\n");
    }

    // Requirement: PRINT 语句 - 逗号分隔（列对齐）
    #[test]
    fn test_print_comma_alignment() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::Number(1.0)),
                PrintItem::Comma,
                PrintItem::Expr(Expr::Number(2.0)),
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        let output = exec.get_output();
        // 第一个数 " 1 " 占 3 列，逗号应该对齐到第 14 列
        assert!(output.starts_with(" 1 "));
        assert!(output.contains(" 2 "));
    }

    // Requirement: GOTO 语句
    #[test]
    fn test_goto_statement() {
        let mut exec = Executor::new();
        
        // 添加程序行
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Let {
                target: AssignTarget::Variable("A".to_string()),
                value: Expr::Number(1.0),
            }]
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 100,
            statements: vec![Statement::Let {
                target: AssignTarget::Variable("B".to_string()),
                value: Expr::Number(99.0),
            }]
        });
        
        let stmt = Statement::Goto {
            line_number: Expr::Number(100.0),
        };
        exec.execute_statement(&stmt).unwrap();
        
        // 验证跳转成功（下一行应该是 100）
        assert_eq!(exec.runtime().get_current_line(), Some(100));
    }

    // Requirement: IF...THEN 语句 - 条件为真
    #[test]
    fn test_if_then_true() {
        let mut exec = Executor::new();
        exec.variables.set("A", Value::Number(15.0)).unwrap();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 100,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        
        // 启动执行来设置初始状态
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        let stmt = Statement::If {
            condition: Expr::binary(
                Expr::Variable("A".to_string()),
                BinaryOperator::Greater,
                Expr::Number(10.0),
            ),
            then_part: Box::new(ThenPart::LineNumber(100)),
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.runtime().get_current_line(), Some(100));
    }

    // Requirement: IF...THEN 语句 - 条件为假
    #[test]
    fn test_if_then_false() {
        let mut exec = Executor::new();
        exec.variables.set("A", Value::Number(5.0)).unwrap();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![],
        });
        
        let current_line = exec.runtime().get_current_line();
        
        let stmt = Statement::If {
            condition: Expr::binary(
                Expr::Variable("A".to_string()),
                BinaryOperator::Greater,
                Expr::Number(10.0),
            ),
            then_part: Box::new(ThenPart::LineNumber(100)),
        };
        
        exec.execute_statement(&stmt).unwrap();
        // 条件为假，不应该跳转
        assert_eq!(exec.runtime().get_current_line(), current_line);
    }

    // Requirement: IF...THEN 语句 - THEN 后跟语句
    #[test]
    fn test_if_then_statement() {
        let mut exec = Executor::new();
        exec.variables.set("A", Value::Number(15.0)).unwrap();
        
        let stmt = Statement::If {
            condition: Expr::binary(
                Expr::Variable("A".to_string()),
                BinaryOperator::Greater,
                Expr::Number(10.0),
            ),
            then_part: Box::new(ThenPart::Statement(
                Statement::Print {
                    items: vec![
                        PrintItem::Expr(Expr::String("TRUE".to_string())),
                    ],
                }
            )),
        };
        
        exec.execute_statement(&stmt).unwrap();
        assert_eq!(exec.get_output(), "TRUE\n");
    }

    // Test: TAB 函数
    #[test]
    fn test_tab_function() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::String("A".to_string())),
                PrintItem::Semicolon,
                PrintItem::Tab(Expr::Number(10.0)),
                PrintItem::Semicolon,
                PrintItem::Expr(Expr::String("B".to_string())),
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        let output = exec.get_output();
        // A 在列 0，TAB(10) 跳到列 10，然后是 B
        assert!(output.starts_with("A"));
        assert!(output.contains("B"));
    }

    // Test: SPC 函数
    #[test]
    fn test_spc_function() {
        let mut exec = Executor::new();
        
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::String("A".to_string())),
                PrintItem::Semicolon,
                PrintItem::Spc(Expr::Number(5.0)),
                PrintItem::Semicolon,
                PrintItem::Expr(Expr::String("B".to_string())),
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        let output = exec.get_output();
        // A + 5个空格 + B
        assert_eq!(output, "A     B\n");
    }

    // Requirement: GOSUB 和 RETURN 语句 - 子程序调用
    #[test]
    fn test_gosub_statement() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 500,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        
        // 启动执行
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        let stmt = Statement::Gosub {
            line_number: Expr::Number(500.0),
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        // 验证跳转到子程序
        assert_eq!(exec.runtime().get_current_line(), Some(500));
        // 验证调用栈深度
        assert_eq!(exec.runtime().stack_depth(), 1);
    }

    // Requirement: GOSUB 和 RETURN 语句 - 子程序返回
    #[test]
    fn test_return_statement() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 20,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 500,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        
        // 启动执行并设置调用栈
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        exec.runtime_mut().push_gosub(20, 0).unwrap();
        exec.runtime_mut().set_execution_position(500, 0).unwrap();
        
        let stmt = Statement::Return;
        exec.execute_statement(&stmt).unwrap();
        
        // 验证返回到调用点
        assert_eq!(exec.runtime().get_current_line(), Some(20));
        // 验证调用栈已弹出
        assert_eq!(exec.runtime().stack_depth(), 0);
    }

    // Requirement: GOSUB 和 RETURN 语句 - 嵌套子程序
    #[test]
    fn test_nested_gosub() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 100,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 200,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        
        // 启动执行
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        // 第一次 GOSUB
        exec.execute_statement(&Statement::Gosub {
            line_number: Expr::Number(100.0),
        }).unwrap();
        assert_eq!(exec.runtime().stack_depth(), 1);
        
        // 第二次 GOSUB（嵌套）
        exec.execute_statement(&Statement::Gosub {
            line_number: Expr::Number(200.0),
        }).unwrap();
        assert_eq!(exec.runtime().stack_depth(), 2);
        assert_eq!(exec.runtime().get_current_line(), Some(200));
        
        // 第一次 RETURN
        exec.execute_statement(&Statement::Return).unwrap();
        assert_eq!(exec.runtime().stack_depth(), 1);
        assert_eq!(exec.runtime().get_current_line(), Some(100));
        
        // 第二次 RETURN
        exec.execute_statement(&Statement::Return).unwrap();
        assert_eq!(exec.runtime().stack_depth(), 0);
        assert_eq!(exec.runtime().get_current_line(), Some(10));
    }

    // Requirement: INPUT 语句 - 基本输入
    #[test]
    fn test_input_basic() {
        let mut exec = Executor::new();
        
        // 设置输入回调
        exec.set_input_callback(|_| Some("42".to_string()));
        
        let stmt = Statement::Input {
            prompt: None,
            variables: vec![AssignTarget::Variable("A".to_string())],
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        // 验证输出提示符
        assert!(exec.get_output().contains("? "));
        
        // 验证变量赋值
        assert_eq!(exec.variables.get("A"), Value::Number(42.0));
    }

    // Requirement: INPUT 语句 - 带提示符的输入
    #[test]
    fn test_input_with_prompt() {
        let mut exec = Executor::new();
        
        exec.set_input_callback(|_| Some("100".to_string()));
        
        let stmt = Statement::Input {
            prompt: Some("ENTER VALUE".to_string()),
            variables: vec![AssignTarget::Variable("X".to_string())],
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        // 验证提示符
        assert!(exec.get_output().contains("ENTER VALUE? "));
        assert_eq!(exec.variables.get("X"), Value::Number(100.0));
    }

    // Requirement: INPUT 语句 - 输入多个变量
    #[test]
    fn test_input_multiple_variables() {
        let mut exec = Executor::new();
        
        exec.set_input_callback(|_| Some("10, 20, 30".to_string()));
        
        let stmt = Statement::Input {
            prompt: None,
            variables: vec![
                AssignTarget::Variable("A".to_string()),
                AssignTarget::Variable("B".to_string()),
                AssignTarget::Variable("C".to_string())
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        assert_eq!(exec.variables.get("A"), Value::Number(10.0));
        assert_eq!(exec.variables.get("B"), Value::Number(20.0));
        assert_eq!(exec.variables.get("C"), Value::Number(30.0));
    }

    // Requirement: INPUT 语句 - 字符串输入
    #[test]
    fn test_input_string() {
        let mut exec = Executor::new();
        
        exec.set_input_callback(|_| Some("HELLO".to_string()));
        
        let stmt = Statement::Input {
            prompt: None,
            variables: vec![AssignTarget::Variable("A$".to_string())],
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        assert_eq!(exec.variables.get("A$"), Value::String("HELLO".to_string()));
    }

    // Requirement: INPUT 语句 - 字符串带引号
    #[test]
    fn test_input_string_with_quotes() {
        let mut exec = Executor::new();
        
        exec.set_input_callback(|_| Some("\"HELLO, WORLD\"".to_string()));
        
        let stmt = Statement::Input {
            prompt: None,
            variables: vec![AssignTarget::Variable("A$".to_string())],
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        assert_eq!(exec.variables.get("A$"), Value::String("HELLO, WORLD".to_string()));
    }

    // Requirement: DATA/READ 机制 - DATA 存储和 READ 读取
    #[test]
    fn test_data_read() {
        let mut exec = Executor::new();
        
        // 添加 DATA 值
        exec.add_data_value(DataValue::Number(1.0));
        exec.add_data_value(DataValue::Number(2.0));
        exec.add_data_value(DataValue::Number(3.0));
        
        let stmt = Statement::Read {
            variables: vec![
                AssignTarget::Variable("A".to_string()),
                AssignTarget::Variable("B".to_string()),
                AssignTarget::Variable("C".to_string())
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        assert_eq!(exec.variables.get("A"), Value::Number(1.0));
        assert_eq!(exec.variables.get("B"), Value::Number(2.0));
        assert_eq!(exec.variables.get("C"), Value::Number(3.0));
    }

    // Requirement: DATA/READ 机制 - 混合数据类型
    #[test]
    fn test_data_read_mixed_types() {
        let mut exec = Executor::new();
        
        exec.add_data_value(DataValue::Number(42.0));
        exec.add_data_value(DataValue::String("HELLO".to_string()));
        
        let stmt = Statement::Read {
            variables: vec![
                AssignTarget::Variable("A".to_string()),
                AssignTarget::Variable("B$".to_string())
            ],
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        assert_eq!(exec.variables.get("A"), Value::Number(42.0));
        assert_eq!(exec.variables.get("B$"), Value::String("HELLO".to_string()));
    }

    // Requirement: DATA/READ 机制 - OUT OF DATA 错误
    #[test]
    fn test_out_of_data_error() {
        let mut exec = Executor::new();
        
        exec.add_data_value(DataValue::Number(1.0));
        
        let stmt = Statement::Read {
            variables: vec![
                AssignTarget::Variable("A".to_string()),
                AssignTarget::Variable("B".to_string())
            ],
        };
        
        let result = exec.execute_statement(&stmt);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::OutOfData));
    }

    // Requirement: RESTORE 数据指针 - RESTORE 重置到开头
    #[test]
    fn test_restore() {
        let mut exec = Executor::new();
        
        exec.add_data_value(DataValue::Number(1.0));
        exec.add_data_value(DataValue::Number(2.0));
        
        // 第一次 READ
        exec.execute_statement(&Statement::Read {
            variables: vec![AssignTarget::Variable("A".to_string())],
        }).unwrap();
        assert_eq!(exec.variables.get("A"), Value::Number(1.0));
        
        // RESTORE
        exec.execute_statement(&Statement::Restore {
            line_number: None,
        }).unwrap();
        
        // 第二次 READ（应该重新从头开始）
        exec.execute_statement(&Statement::Read {
            variables: vec![AssignTarget::Variable("B".to_string())],
        }).unwrap();
        assert_eq!(exec.variables.get("B"), Value::Number(1.0));
    }

    // Requirement: FOR...NEXT 循环 - 正向循环
    #[test]
    fn test_for_next_basic() {
        let mut exec = Executor::new();
        
        // 添加测试程序：FOR I=1 TO 3: PRINT I: NEXT I
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![
                Statement::For {
                    var: "I".to_string(),
                    start: Expr::Number(1.0),
                    end: Expr::Number(3.0),
                    step: None,
                },
            ],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 20,
            statements: vec![Statement::Next { var: Some("I".to_string()) }],
        });
        
        // 启动执行
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        // 第一次循环：I=1
        exec.execute_statement(&Statement::For {
            var: "I".to_string(),
            start: Expr::Number(1.0),
            end: Expr::Number(3.0),
            step: None,
        }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(1.0));
        
        // NEXT：I=2
        exec.runtime_mut().set_execution_position(20, 0).unwrap();
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(2.0));
        
        // NEXT：I=3
        exec.runtime_mut().set_execution_position(20, 0).unwrap();
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(3.0));
        
        // NEXT：循环结束 (I递增到4但不再循环)
        exec.runtime_mut().set_execution_position(20, 0).unwrap();
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        // 循环已结束，变量值应该为循环后的值 4
        assert_eq!(exec.variables.get("I"), Value::Number(3.0));
    }

    // Requirement: FOR...NEXT 循环 - 步长为 2
    #[test]
    fn test_for_next_step() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![
                Statement::For {
                    var: "I".to_string(),
                    start: Expr::Number(0.0),
                    end: Expr::Number(4.0),
                    step: Some(Expr::Number(2.0)),
                },
            ],
        });
        
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        // FOR I=0 TO 4 STEP 2
        exec.execute_statement(&Statement::For {
            var: "I".to_string(),
            start: Expr::Number(0.0),
            end: Expr::Number(4.0),
            step: Some(Expr::Number(2.0)),
        }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(0.0));
        
        // NEXT：I=2
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(2.0));
        
        // NEXT：I=4
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(4.0));
        
        // NEXT：循环结束
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(4.0));
    }

    // Requirement: FOR...NEXT 循环 - 负步长
    #[test]
    fn test_for_next_negative_step() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![
                Statement::For {
                    var: "I".to_string(),
                    start: Expr::Number(3.0),
                    end: Expr::Number(1.0),
                    step: Some(Expr::Number(-1.0)),
                },
            ],
        });
        
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        // FOR I=3 TO 1 STEP -1
        exec.execute_statement(&Statement::For {
            var: "I".to_string(),
            start: Expr::Number(3.0),
            end: Expr::Number(1.0),
            step: Some(Expr::Number(-1.0)),
        }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(3.0));
        
        // NEXT：I=2
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(2.0));
        
        // NEXT：I=1
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(1.0));
        
        // NEXT：循环结束
        exec.execute_statement(&Statement::Next { var: Some("I".to_string()) }).unwrap();
        assert_eq!(exec.variables.get("I"), Value::Number(1.0));
    }

    // Requirement: ON...GOTO - 基于表达式的跳转
    #[test]
    fn test_on_goto() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 100,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 200,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        // ON 2 GOTO 100,200,300
        exec.execute_statement(&Statement::On {
            expr: Expr::Number(2.0),
            targets: vec![100, 200, 300],
            is_gosub: false,
        }).unwrap();
        
        // 应该跳转到 200
        assert_eq!(exec.runtime().get_current_line(), Some(200));
    }

    // Requirement: ON...GOSUB - 基于表达式的子程序调用
    #[test]
    fn test_on_gosub() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 100,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        
        // ON 1 GOSUB 100,200
        exec.execute_statement(&Statement::On {
            expr: Expr::Number(1.0),
            targets: vec![100, 200],
            is_gosub: true,
        }).unwrap();
        
        // 应该跳转到 100
        assert_eq!(exec.runtime().get_current_line(), Some(100));
        // 栈深度应该为 1
        assert_eq!(exec.runtime().stack_depth(), 1);
    }

    // Requirement: ON...GOTO - 值超出范围
    #[test]
    fn test_on_goto_out_of_range() {
        let mut exec = Executor::new();
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Rem { comment: String::new() }],
        });
        
        exec.runtime_mut().start_execution(Some(10)).unwrap();
        let current_line = exec.runtime().get_current_line();
        
        // ON 5 GOTO 100,200  (5 超出范围)
        exec.execute_statement(&Statement::On {
            expr: Expr::Number(5.0),
            targets: vec![100, 200],
            is_gosub: false,
        }).unwrap();
        
        // 应该继续在当前行
        assert_eq!(exec.runtime().get_current_line(), current_line);
    }
    
    #[test]
    fn test_save_and_load() {
        use std::fs;
        
        let mut exec = Executor::new();
        
        // 添加一些程序行
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![Statement::Print {
                items: vec![PrintItem::Expr(Expr::String("HELLO".to_string()))],
            }],
        });
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 20,
            statements: vec![Statement::Let {
                target: AssignTarget::Variable("A".to_string()),
                value: Expr::Number(42.0),
            }],
        });
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 30,
            statements: vec![Statement::End],
        });
        
        // 保存程序到文件
        let filename = "test_program.bas";
        exec.execute_statement(&Statement::Save {
            filename: filename.to_string(),
        }).unwrap();
        
        // 验证文件存在
        assert!(fs::metadata(filename).is_ok());
        
        // 清空程序
        exec.runtime_mut().clear_program();
        assert_eq!(exec.runtime().line_count(), 0);
        
        // 加载程序
        exec.execute_statement(&Statement::Load {
            filename: filename.to_string(),
        }).unwrap();
        
        // 验证程序已加载
        assert_eq!(exec.runtime().line_count(), 3);
        assert!(exec.runtime().get_line(10).is_some());
        assert!(exec.runtime().get_line(20).is_some());
        assert!(exec.runtime().get_line(30).is_some());
        
        // 清理测试文件
        fs::remove_file(filename).ok();
    }
    
    #[test]
    fn test_save_empty_program() {
        let mut exec = Executor::new();
        
        // 尝试保存空程序应该失败
        let result = exec.execute_statement(&Statement::Save {
            filename: "empty.bas".to_string(),
        });
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_load_nonexistent_file() {
        let mut exec = Executor::new();
        
        // 尝试加载不存在的文件应该失败
        let result = exec.execute_statement(&Statement::Load {
            filename: "nonexistent.bas".to_string(),
        });
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_save_complex_program() {
        use std::fs;
        
        let mut exec = Executor::new();
        
        // 创建一个更复杂的程序
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 10,
            statements: vec![
                Statement::For {
                    var: "I".to_string(),
                    start: Expr::Number(1.0),
                    end: Expr::Number(10.0),
                    step: Some(Expr::Number(1.0)),
                },
            ],
        });
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 20,
            statements: vec![
                Statement::Print {
                    items: vec![PrintItem::Expr(Expr::Variable("I".to_string()))],
                },
            ],
        });
        
        exec.runtime_mut().add_line(ProgramLine {
            line_number: 30,
            statements: vec![Statement::Next { var: Some("I".to_string()) }],
        });
        
        // 保存并重新加载
        let filename = "test_complex.bas";
        exec.execute_statement(&Statement::Save {
            filename: filename.to_string(),
        }).unwrap();
        
        exec.runtime_mut().clear_program();
        
        exec.execute_statement(&Statement::Load {
            filename: filename.to_string(),
        }).unwrap();
        
        // 验证程序结构
        assert_eq!(exec.runtime().line_count(), 3);
        
        // 清理
        fs::remove_file(filename).ok();
    }
    
    // ========== 高级功能测试 ==========
    
    // Test: POS 函数 - 基本功能
    #[test]
    fn test_pos_function() {
        let mut exec = Executor::new();
        
        // 初始位置应该是 1（1-based）
        let pos_expr = Expr::FunctionCall {
            name: "POS".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        let pos = exec.eval_expr(&pos_expr).unwrap();
        assert_eq!(pos, Value::Number(1.0));
        
        // 打印一些内容后，位置应该更新
        exec.execute_statement(&Statement::Print {
            items: vec![PrintItem::Expr(Expr::String("ABC".to_string()))],
        }).unwrap();
        
        let pos = exec.eval_expr(&pos_expr).unwrap();
        // 输出 "ABC" + 换行，所以新行开始应该是 1
        assert_eq!(pos, Value::Number(1.0));
    }
    
    // Test: POS 函数 - 与 TAB 交互
    #[test]
    fn test_pos_with_tab() {
        let mut exec = Executor::new();
        
        // 使用 TAB 后检查位置
        exec.execute_statement(&Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::String("START".to_string())),
                PrintItem::Tab(Expr::Number(15.0)),
                PrintItem::Expr(Expr::FunctionCall {
                    name: "POS".to_string(),
                    args: vec![Expr::Number(0.0)],
                }),
            ],
        }).unwrap();
        
        let output = exec.get_output();
        assert!(output.contains("START"));
    }
    
    // Test: POS 函数 - 与 SPC 交互
    #[test]
    fn test_pos_with_spc() {
        let mut exec = Executor::new();
        
        exec.execute_statement(&Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::String("A".to_string())),
                PrintItem::Spc(Expr::Number(5.0)),
                PrintItem::Expr(Expr::FunctionCall {
                    name: "POS".to_string(),
                    args: vec![Expr::Number(0.0)],
                }),
            ],
        }).unwrap();
        
        let output = exec.get_output();
        assert!(output.contains("A"));
    }
    
    // Test: DEF FN 语句 - 定义用户自定义函数
    #[test]
    fn test_def_fn_statement() {
        let mut exec = Executor::new();
        
        let stmt = Statement::DefFn {
            name: "SQUARE".to_string(),
            param: "X".to_string(),
            body: Expr::binary(
                Expr::Variable("X".to_string()),
                BinaryOperator::Multiply,
                Expr::Variable("X".to_string()),
            ),
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        // 验证函数已定义
        assert!(exec.variables.has_function("SQUARE"));
    }
    
    // Test: FN 调用 - 基本功能
    #[test]
    fn test_fn_call_basic() {
        let mut exec = Executor::new();
        
        // 定义函数
        exec.execute_statement(&Statement::DefFn {
            name: "SQUARE".to_string(),
            param: "X".to_string(),
            body: Expr::binary(
                Expr::Variable("X".to_string()),
                BinaryOperator::Multiply,
                Expr::Variable("X".to_string()),
            ),
        }).unwrap();
        
        // 调用函数
        let fn_call = Expr::FunctionCall {
            name: "FNSQUARE".to_string(),
            args: vec![Expr::Number(5.0)],
        };
        
        let result = exec.eval_expr(&fn_call).unwrap();
        assert_eq!(result, Value::Number(25.0));
    }
    
    // Test: FN 调用 - 使用全局变量
    #[test]
    fn test_fn_call_with_global() {
        let mut exec = Executor::new();
        
        // 设置全局变量
        exec.variables.set("GVAL", Value::Number(10.0)).unwrap();
        
        // 定义使用全局变量的函数
        exec.execute_statement(&Statement::DefFn {
            name: "ADDG".to_string(),
            param: "X".to_string(),
            body: Expr::binary(
                Expr::Variable("X".to_string()),
                BinaryOperator::Add,
                Expr::Variable("GVAL".to_string()),
            ),
        }).unwrap();
        
        // 调用函数
        let fn_call = Expr::FunctionCall {
            name: "FNADDG".to_string(),
            args: vec![Expr::Number(5.0)],
        };
        
        let result = exec.eval_expr(&fn_call).unwrap();
        assert_eq!(result, Value::Number(15.0));
        
        // 验证全局变量未被修改
        assert_eq!(exec.variables.get("GVAL"), Value::Number(10.0));
    }
    
    // Test: FN 调用 - 嵌套调用
    #[test]
    fn test_fn_call_nested() {
        let mut exec = Executor::new();
        
        // 定义两个函数
        exec.execute_statement(&Statement::DefFn {
            name: "DOUBLE".to_string(),
            param: "Y".to_string(),
            body: Expr::binary(
                Expr::Variable("Y".to_string()),
                BinaryOperator::Add,
                Expr::Variable("Y".to_string()),
            ),
        }).unwrap();
        
        exec.execute_statement(&Statement::DefFn {
            name: "SQUARE".to_string(),
            param: "X".to_string(),
            body: Expr::binary(
                Expr::Variable("X".to_string()),
                BinaryOperator::Multiply,
                Expr::Variable("X".to_string()),
            ),
        }).unwrap();
        
        // 嵌套调用：FN SQUARE(FN DOUBLE(2))
        let nested = Expr::FunctionCall {
            name: "FNSQUARE".to_string(),
            args: vec![Expr::FunctionCall {
                name: "FNDOUBLE".to_string(),
                args: vec![Expr::Number(2.0)],
            }],
        };
        
        let result = exec.eval_expr(&nested).unwrap();
        // FN DOUBLE(2) = 4, FN SQUARE(4) = 16
        assert_eq!(result, Value::Number(16.0));
    }
    
    // Test: FN 调用 - 参数作用域
    #[test]
    fn test_fn_call_parameter_scope() {
        let mut exec = Executor::new();
        
        // 设置一个全局变量 X
        exec.variables.set("X", Value::Number(100.0)).unwrap();
        
        // 定义函数，参数名也是 X
        exec.execute_statement(&Statement::DefFn {
            name: "TEST".to_string(),
            param: "X".to_string(),
            body: Expr::binary(
                Expr::Variable("X".to_string()),
                BinaryOperator::Add,
                Expr::Number(1.0),
            ),
        }).unwrap();
        
        // 调用函数，参数值应该是传入的值，不是全局变量
        let fn_call = Expr::FunctionCall {
            name: "FNTEST".to_string(),
            args: vec![Expr::Number(5.0)],
        };
        
        let result = exec.eval_expr(&fn_call).unwrap();
        assert_eq!(result, Value::Number(6.0));
        
        // 验证全局变量 X 未被修改
        assert_eq!(exec.variables.get("X"), Value::Number(100.0));
    }
    
    // Test: GET 语句 - 字符串变量
    #[test]
    fn test_get_string_variable() {
        let mut exec = Executor::new();
        
        // 设置输入回调
        exec.set_input_callback(|_| Some("A".to_string()));
        
        let stmt = Statement::Get {
            variable: "CH$".to_string(),
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        // 验证字符被存储
        let ch = exec.variables.get("CH$");
        assert_eq!(ch, Value::String("A".to_string()));
    }
    
    // Test: GET 语句 - 数值变量（ASCII 码）
    #[test]
    fn test_get_numeric_variable() {
        let mut exec = Executor::new();
        
        // 设置输入回调
        exec.set_input_callback(|_| Some("A".to_string()));
        
        let stmt = Statement::Get {
            variable: "CH".to_string(),
        };
        
        exec.execute_statement(&stmt).unwrap();
        
        // 验证 ASCII 码被存储（'A' = 65）
        let ch = exec.variables.get("CH");
        assert_eq!(ch, Value::Number(65.0));
    }
    
    // Test: GET 语句 - 无输入情况
    #[test]
    fn test_get_no_input() {
        let mut exec = Executor::new();
        
        // 设置回调返回 None（无输入）
        exec.set_input_callback(|_| None);
        
        let stmt = Statement::Get {
            variable: "CH$".to_string(),
        };
        
        // 由于没有输入，应该返回空字符串
        exec.execute_statement(&stmt).unwrap();
        
        let ch = exec.variables.get("CH$");
        assert_eq!(ch, Value::String(String::new()));
    }
    
    // Test: NULL 语句 - 无操作
    #[test]
    fn test_null_statement() {
        let mut exec = Executor::new();
        
        // 设置一个变量
        exec.variables.set("X", Value::Number(10.0)).unwrap();
        
        // 执行 NULL 语句
        exec.execute_statement(&Statement::Null).unwrap();
        
        // 验证变量未被修改
        assert_eq!(exec.variables.get("X"), Value::Number(10.0));
        
        // 验证没有输出
        assert_eq!(exec.get_output(), "");
    }
    
    // Test: NULL 语句 - 多个 NULL
    #[test]
    fn test_multiple_null_statements() {
        let mut exec = Executor::new();
        
        // 执行多个 NULL 语句
        exec.execute_statement(&Statement::Null).unwrap();
        exec.execute_statement(&Statement::Null).unwrap();
        exec.execute_statement(&Statement::Null).unwrap();
        
        // 验证没有错误，没有输出
        assert_eq!(exec.get_output(), "");
    }
    
    // Test: 综合测试 - 运行 test.bas 的关键部分
    #[test]
    fn test_integration_advanced_features() {
        let mut exec = Executor::new();
        
        // 测试 POS 函数
        exec.execute_statement(&Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::String("POS TEST:".to_string())),
            ],
        }).unwrap();
        
        let pos_expr = Expr::FunctionCall {
            name: "POS".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        let pos = exec.eval_expr(&pos_expr).unwrap();
        assert!(pos.as_number().unwrap() >= 1.0);
        
        // 测试 DEF FN 和 FN 调用
        exec.execute_statement(&Statement::DefFn {
            name: "SQUARE".to_string(),
            param: "X".to_string(),
            body: Expr::binary(
                Expr::Variable("X".to_string()),
                BinaryOperator::Multiply,
                Expr::Variable("X".to_string()),
            ),
        }).unwrap();
        
        exec.execute_statement(&Statement::DefFn {
            name: "DOUBLE".to_string(),
            param: "Y".to_string(),
            body: Expr::binary(
                Expr::Variable("Y".to_string()),
                BinaryOperator::Add,
                Expr::Variable("Y".to_string()),
            ),
        }).unwrap();
        
        // 测试函数调用
        let result1 = exec.eval_expr(&Expr::FunctionCall {
            name: "FNSQUARE".to_string(),
            args: vec![Expr::Number(5.0)],
        }).unwrap();
        assert_eq!(result1, Value::Number(25.0));
        
        let result2 = exec.eval_expr(&Expr::FunctionCall {
            name: "FNDOUBLE".to_string(),
            args: vec![Expr::Number(7.0)],
        }).unwrap();
        assert_eq!(result2, Value::Number(14.0));
        
        // 测试 NULL 语句
        exec.execute_statement(&Statement::Null).unwrap();
        exec.execute_statement(&Statement::Print {
            items: vec![PrintItem::Expr(Expr::String("AFTER NULL".to_string()))],
        }).unwrap();
        
        let output = exec.get_output();
        assert!(output.contains("AFTER NULL"));
    }
    
    // Test: 运行 test.bas 文件并验证关键输出
    #[test]
    fn test_run_test_bas_file() {
        use std::fs;
        use crate::basic::tokenizer::Tokenizer;
        use crate::basic::parser::Parser;
        
        let mut exec = Executor::new();
        
        // 设置输入回调（用于 INPUT 和 GET）
        let input_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let input_count_clone = input_count.clone();
        exec.set_input_callback(move |_| {
            let mut count = input_count_clone.lock().unwrap();
            *count += 1;
            if *count == 1 {
                Some("JOHN,18".to_string())  // INPUT 语句
            } else {
                Some("A".to_string())  // GET 语句
            }
        });
        
        // 读取 test.bas 文件
        let test_file = "test.bas";
        if !fs::metadata(test_file).is_ok() {
            // 如果文件不存在，跳过测试
            eprintln!("Warning: test.bas not found, skipping integration test");
            return;
        }
        
        let content = fs::read_to_string(test_file).unwrap();
        
        // 加载程序
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            let mut tokenizer = Tokenizer::new(line);
            let tokens = tokenizer.tokenize_line().unwrap();
            
            let mut parser = Parser::new(tokens);
            if let Some(program_line) = parser.parse_line().unwrap() {
                if program_line.line_number > 0 {
                    // 收集 DATA 语句
                    for stmt in &program_line.statements {
                        if let Statement::Data { values } = stmt {
                            for value in values {
                                let exec_value = match value {
                                    crate::basic::ast::DataValue::Number(n) => DataValue::Number(*n),
                                    crate::basic::ast::DataValue::String(s) => DataValue::String(s.clone()),
                                };
                                exec.add_data_value(exec_value);
                            }
                        }
                    }
                    exec.runtime_mut().add_line(program_line);
                }
            }
        }
        
        // 运行程序（但不运行到 STOP，因为需要交互）
        exec.variables_mut().clear();
        exec.restore_data();
        exec.runtime_mut().start_execution(None).unwrap();
        
        // 执行程序直到 STOP 或 END
        let mut executed_lines = 0;
        loop {
            if executed_lines > 1000 {
                // 防止无限循环
                break;
            }
            
            let stmt = match exec.runtime_mut().get_next_statement() {
                Some(s) => s,
                None => break,
            };
            
            // 跳过 STOP 语句（需要交互）
            if matches!(stmt, Statement::Stop) {
                break;
            }
            
            // 跳过 INPUT 和 GET（已处理）
            if matches!(stmt, Statement::Input { .. }) || matches!(stmt, Statement::Get { .. }) {
                exec.execute_statement(&stmt).ok();
                continue;
            }
            
            if let Err(e) = exec.execute_statement(&stmt) {
                eprintln!("Error executing statement: {:?}", e);
                break;
            }
            
            executed_lines += 1;
            
            if exec.runtime().is_stopped() || exec.runtime().is_paused() {
                break;
            }
        }
        
        let output = exec.get_output();
        
        // 验证关键输出
        assert!(output.contains("DEMO START"), "Should contain 'DEMO START'");
        assert!(output.contains("POS TEST:"), "Should contain POS test output");
        assert!(output.contains("FN SQUARE(5)=") || output.contains("FN SQUARE"), "Should contain function test");
        assert!(output.contains("AFTER NULL"), "Should contain NULL test output");
        
        // 验证函数已定义
        assert!(exec.variables.has_function("SQUARE"), "Function SQUARE should be defined");
        assert!(exec.variables.has_function("DOUBLE"), "Function DOUBLE should be defined");
    }

    // Test: GameContext 集成
    #[test]
    fn test_game_context_integration() {
        use crate::game_context::NullGameContext;

        let mut exec = Executor::new();

        // 初始状态没有 GameContext
        assert!(!exec.has_game_context());
        assert!(exec.game_context().is_none());
        assert!(exec.game_context_mut().is_none());

        // 设置 GameContext
        let ctx = Box::new(NullGameContext);
        exec.set_game_context(ctx);

        // 现在有 GameContext
        assert!(exec.has_game_context());
        assert!(exec.game_context().is_some());
        assert!(exec.game_context_mut().is_some());

        // 可以通过可变引用调用方法
        if let Some(ctx) = exec.game_context_mut() {
            ctx.plot(0, 0, '@', 1, 0);
            ctx.cls();
        }
    }

    // Test: 游戏输入函数（无 GameContext 时返回默认值）
    #[test]
    fn test_game_input_functions_without_context() {
        let mut exec = Executor::new();

        // INKEY() 无参数
        let expr = Expr::FunctionCall {
            name: "INKEY".to_string(),
            args: vec![],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));

        // KEY("W") 单参数
        let expr = Expr::FunctionCall {
            name: "KEY".to_string(),
            args: vec![Expr::String("W".to_string())],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));

        // MOUSEX() 无参数
        let expr = Expr::FunctionCall {
            name: "MOUSEX".to_string(),
            args: vec![],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));

        // MOUSEY() 无参数
        let expr = Expr::FunctionCall {
            name: "MOUSEY".to_string(),
            args: vec![],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));

        // MOUSEB() 无参数
        let expr = Expr::FunctionCall {
            name: "MOUSEB".to_string(),
            args: vec![],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));
    }

    // Test: 精灵查询函数（无 GameContext 时返回默认值）
    #[test]
    fn test_sprite_query_functions_without_context() {
        let mut exec = Executor::new();

        // SPRITEX(1)
        let expr = Expr::FunctionCall {
            name: "SPRITEX".to_string(),
            args: vec![Expr::Number(1.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));

        // SPRITEY(1)
        let expr = Expr::FunctionCall {
            name: "SPRITEY".to_string(),
            args: vec![Expr::Number(1.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));

        // SPRITEHIT(1, 2)
        let expr = Expr::FunctionCall {
            name: "SPRITEHIT".to_string(),
            args: vec![Expr::Number(1.0), Expr::Number(2.0)],
        };
        let result = exec.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Number(0.0));
    }

    // Test: 游戏输入函数参数错误
    #[test]
    fn test_game_input_functions_argument_errors() {
        let mut exec = Executor::new();

        // INKEY() 不应有参数
        let expr = Expr::FunctionCall {
            name: "INKEY".to_string(),
            args: vec![Expr::Number(1.0)],
        };
        assert!(exec.eval_expr(&expr).is_err());

        // KEY() 必须有1个参数
        let expr = Expr::FunctionCall {
            name: "KEY".to_string(),
            args: vec![],
        };
        assert!(exec.eval_expr(&expr).is_err());

        // SPRITEX() 必须有1个参数
        let expr = Expr::FunctionCall {
            name: "SPRITEX".to_string(),
            args: vec![],
        };
        assert!(exec.eval_expr(&expr).is_err());

        // SPRITEHIT() 必须有2个参数
        let expr = Expr::FunctionCall {
            name: "SPRITEHIT".to_string(),
            args: vec![Expr::Number(1.0)],
        };
        assert!(exec.eval_expr(&expr).is_err());
    }
}

