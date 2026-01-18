/// 语法解析器
///
/// 将 Token 流解析为 AST

use super::ast::*;
use super::error::{BasicError, Result};
use super::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// 创建新的解析器
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    /// 解析程序行（可能包含多条语句，用冒号分隔）
    pub fn parse_line(&mut self) -> Result<Option<ProgramLine>> {
        // 跳过空行
        if self.is_at_end() || self.current() == &Token::Newline {
            return Ok(None);
        }

        // 检查是否有行号
        let line_number = if let Token::LineNumber(num) = self.current() {
            let num = *num;
            self.advance();
            Some(num)
        } else {
            None
        };

        // 如果只有行号（删除该行）
        if self.current() == &Token::Newline {
            if let Some(num) = line_number {
                // 返回一个空的程序行，调用者可以据此删除该行
                return Ok(Some(ProgramLine {
                    line_number: num,
                    statements: vec![],
                }));
            } else {
                return Ok(None);
            }
        }

        // 解析语句（支持冒号分隔的多语句）
        let statements = self.parse_statements()?;

        if let Some(num) = line_number {
            Ok(Some(ProgramLine {
                line_number: num,
                statements,
            }))
        } else {
            // 直接模式：创建伪行号 0
            Ok(Some(ProgramLine {
                line_number: 0,
                statements,
            }))
        }
    }

    /// 解析多条语句（用冒号分隔）
    fn parse_statements(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();

        while !self.is_at_end() && self.current() != &Token::Newline {
            let stmt = self.parse_statement()?;
            statements.push(stmt);

            // 检查是否有冒号分隔符
            if self.current() == &Token::Colon {
                self.advance(); // 跳过冒号
                continue;
            } else {
                break;
            }
        }

        Ok(statements)
    }

    /// 解析单条语句
    fn parse_statement(&mut self) -> Result<Statement> {
        let token = self.current().clone();

        match token {
            Token::Print => self.parse_print(),
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            Token::Goto => self.parse_goto(),
            Token::Gosub => self.parse_gosub(),
            Token::Return => {
                self.advance();
                Ok(Statement::Return)
            }
            Token::For => self.parse_for(),
            Token::Next => self.parse_next(),
            Token::On => self.parse_on(),
            Token::Input => self.parse_input(),
            Token::Dim => self.parse_dim(),
            Token::Data => self.parse_data(),
            Token::Read => self.parse_read(),
            Token::Restore => self.parse_restore(),
            Token::Def => self.parse_def_fn(),
            Token::Rem => {
                self.advance();
                // 提取 REM 后面的注释内容
                // Tokenizer 会将注释内容作为 String token 返回
                let comment = if let Token::String(ref comment) = self.current() {
                    let c = comment.clone();
                    self.advance();
                    c
                } else {
                    String::new()
                };
                Ok(Statement::Rem { comment })
            }
            Token::End => {
                self.advance();
                Ok(Statement::End)
            }
            Token::Stop => {
                self.advance();
                Ok(Statement::Stop)
            }
            Token::New => {
                self.advance();
                Ok(Statement::New)
            }
            Token::Clear => {
                self.advance();
                Ok(Statement::Clear)
            }
            Token::List => self.parse_list(),
            Token::Run => self.parse_run(),
            Token::Cont => {
                self.advance();
                Ok(Statement::Cont)
            }
            Token::Poke => self.parse_poke(),
            Token::Wait => self.parse_wait(),
            Token::Get => self.parse_get(),
            Token::Null => {
                self.advance();
                Ok(Statement::Null)
            }
            Token::Load => self.parse_load(),
            Token::Save => self.parse_save(),
            // 协程扩展语句
            Token::Yield => {
                self.advance();
                Ok(Statement::Yield)
            }
            Token::WaitKey => {
                self.advance();
                Ok(Statement::WaitKey)
            }
            Token::WaitClick => {
                self.advance();
                Ok(Statement::WaitClick)
            }
            // 隐式 LET（赋值语句没有 LET 关键字）
            Token::Identifier(_) => {
                // 检查后面是否是 =, ( 或 ,
                self.parse_implicit_let()
            }
            _ => Err(BasicError::InvalidStatement(self.position)),
        }
    }

    /// 解析 PRINT 语句
    fn parse_print(&mut self) -> Result<Statement> {
        self.expect(&Token::Print)?;

        let mut items = Vec::new();

        // PRINT 后面可以为空（输出换行）
        if self.current() == &Token::Newline || self.current() == &Token::Colon {
            return Ok(Statement::Print { items });
        }

        loop {
            // 检查分隔符
            if self.current() == &Token::Comma {
                items.push(PrintItem::Comma);
                self.advance();
                // 逗号后可以直接结束（保留光标位置）
                if self.current() == &Token::Newline || self.current() == &Token::Colon {
                    break;
                }
                continue;
            }

            if self.current() == &Token::Semicolon {
                items.push(PrintItem::Semicolon);
                self.advance();
                // 分号后可以直接结束（不换行）
                if self.current() == &Token::Newline || self.current() == &Token::Colon {
                    break;
                }
                continue;
            }

            // 检查 TAB 和 SPC 函数
            if self.current() == &Token::Tab {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                items.push(PrintItem::Tab(expr));
                continue;
            }

            if self.current() == &Token::Spc {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                items.push(PrintItem::Spc(expr));
                continue;
            }

            // 解析表达式
            let expr = self.parse_expression()?;
            items.push(PrintItem::Expr(expr));

            // 检查是否结束
            if self.current() == &Token::Newline || self.current() == &Token::Colon {
                break;
            }
        }

        Ok(Statement::Print { items })
    }

    /// 解析 LET 语句
    fn parse_let(&mut self) -> Result<Statement> {
        self.expect(&Token::Let)?;
        self.parse_assignment()
    }

    /// 解析隐式 LET（没有 LET 关键字的赋值）
    fn parse_implicit_let(&mut self) -> Result<Statement> {
        self.parse_assignment()
    }

    /// 解析赋值（LET A = 10 或 A = 10）
    fn parse_assignment(&mut self) -> Result<Statement> {
        let target = self.parse_assign_target()?;
        self.expect(&Token::Equal)?;
        let value = self.parse_expression()?;

        Ok(Statement::Let { target, value })
    }

    /// 解析赋值目标（变量或数组元素）
    fn parse_assign_target(&mut self) -> Result<AssignTarget> {
        let name = self.expect_identifier()?;

        // 检查是否为数组元素
        if self.current() == &Token::LeftParen {
            self.advance();
            let indices = self.parse_expression_list()?;
            self.expect(&Token::RightParen)?;
            Ok(AssignTarget::ArrayElement { name, indices })
        } else {
            Ok(AssignTarget::Variable(name))
        }
    }

    /// 解析 IF 语句
    fn parse_if(&mut self) -> Result<Statement> {
        self.expect(&Token::If)?;
        
        let condition = self.parse_expression()?;
        
        self.expect(&Token::Then)?;
        
        // THEN 后可以是行号或语句
        let then_part = if let Token::Number(num) = self.current() {
            let line_num = *num as u16;
            self.advance();
            ThenPart::LineNumber(line_num)
        } else {
            // THEN 后跟语句（可能有冒号分隔的多条语句）
            let mut statements = vec![self.parse_statement()?];
            
            // 检查是否有后续语句
            while self.current() == &Token::Colon {
                self.advance();
                if self.current() != &Token::Newline {
                    statements.push(self.parse_statement()?);
                } else {
                    break;
                }
            }
            
            if statements.len() == 1 {
                ThenPart::Statement(statements.into_iter().next().unwrap())
            } else {
                ThenPart::Statements(statements)
            }
        };
        
        Ok(Statement::If {
            condition,
            then_part: Box::new(then_part),
        })
    }

    /// 解析 GOTO 语句
    fn parse_goto(&mut self) -> Result<Statement> {
        self.expect(&Token::Goto)?;
        let line_number = self.parse_expression()?;
        Ok(Statement::Goto { line_number })
    }

    /// 解析 GOSUB 语句
    fn parse_gosub(&mut self) -> Result<Statement> {
        self.expect(&Token::Gosub)?;
        let line_number = self.parse_expression()?;
        Ok(Statement::Gosub { line_number })
    }

    /// 解析 FOR 语句
    fn parse_for(&mut self) -> Result<Statement> {
        self.expect(&Token::For)?;
        
        let var = self.expect_identifier()?;
        self.expect(&Token::Equal)?;
        let start = self.parse_expression()?;
        self.expect(&Token::To)?;
        let end = self.parse_expression()?;
        
        let step = if self.current() == &Token::Step {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        Ok(Statement::For { var, start, end, step })
    }

    /// 解析 NEXT 语句
    fn parse_next(&mut self) -> Result<Statement> {
        self.expect(&Token::Next)?;
        
        let var = if let Token::Identifier(name) = self.current() {
            let name = name.clone();
            self.advance();
            Some(name)
        } else {
            None
        };
        
        Ok(Statement::Next { var })
    }

    /// 解析 ON...GOTO 或 ON...GOSUB
    fn parse_on(&mut self) -> Result<Statement> {
        self.expect(&Token::On)?;
        
        let expr = self.parse_expression()?;
        
        let is_gosub = if self.current() == &Token::Goto {
            self.advance();
            false
        } else if self.current() == &Token::Gosub {
            self.advance();
            true
        } else {
            return Err(BasicError::SyntaxError(
                "Expected GOTO or GOSUB after ON".to_string()
            ));
        };
        
        // 解析行号列表
        let mut targets = Vec::new();
        loop {
            if let Token::Number(num) = self.current() {
                targets.push(*num as u16);
                self.advance();
            } else {
                return Err(BasicError::SyntaxError(
                    "Expected line number in ON statement".to_string()
                ));
            }
            
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(Statement::On { expr, targets, is_gosub })
    }

    /// 解析 INPUT 语句
    fn parse_input(&mut self) -> Result<Statement> {
        self.expect(&Token::Input)?;
        
        // 检查是否有提示符
        let prompt = if let Token::String(s) = self.current() {
            let prompt = s.clone();
            self.advance();
            
            // 提示符后应该有分号或逗号
            if self.current() == &Token::Semicolon || self.current() == &Token::Comma {
                self.advance();
            }
            
            Some(prompt)
        } else {
            None
        };
        
        // 解析变量列表
        let mut variables = Vec::new();
        loop {
            variables.push(self.parse_assign_target()?);
            
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(Statement::Input { prompt, variables })
    }

    /// 解析 DIM 语句
    fn parse_dim(&mut self) -> Result<Statement> {
        self.expect(&Token::Dim)?;
        
        let mut arrays = Vec::new();
        
        loop {
            let name = self.expect_identifier()?;
            self.expect(&Token::LeftParen)?;
            let dimensions = self.parse_expression_list()?;
            self.expect(&Token::RightParen)?;
            
            arrays.push(ArrayDim { name, dimensions });
            
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(Statement::Dim { arrays })
    }

    /// 解析 DATA 语句
    fn parse_data(&mut self) -> Result<Statement> {
        self.expect(&Token::Data)?;
        
        let mut values = Vec::new();
        
        loop {
            if let Token::String(s) = self.current() {
                values.push(DataValue::String(s.clone()));
                self.advance();
            } else if let Token::Number(num) = self.current() {
                values.push(DataValue::Number(*num));
                self.advance();
            } else if let Token::Minus = self.current() {
                // 处理负数
                self.advance();
                if let Token::Number(num) = self.current() {
                    values.push(DataValue::Number(-num));
                    self.advance();
                } else {
                    return Err(BasicError::SyntaxError(
                        "Expected number after minus in DATA".to_string()
                    ));
                }
            } else {
                return Err(BasicError::SyntaxError(
                    "Expected number or string in DATA".to_string()
                ));
            }
            
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(Statement::Data { values })
    }

    /// 解析 READ 语句
    fn parse_read(&mut self) -> Result<Statement> {
        self.expect(&Token::Read)?;
        
        let mut variables = Vec::new();
        loop {
            variables.push(self.parse_assign_target()?);
            
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(Statement::Read { variables })
    }

    /// 解析 RESTORE 语句
    fn parse_restore(&mut self) -> Result<Statement> {
        self.expect(&Token::Restore)?;
        
        let line_number = if let Token::Number(num) = self.current() {
            let num = *num as u16;
            self.advance();
            Some(num)
        } else {
            None
        };
        
        Ok(Statement::Restore { line_number })
    }

    /// 解析 DEF FN 语句
    fn parse_def_fn(&mut self) -> Result<Statement> {
        self.expect(&Token::Def)?;
        
        // DEF FN 语句格式：DEF FN name(param) = expr
        self.expect(&Token::Fn)?;
        
        let name = self.expect_identifier()?;
        
        self.expect(&Token::LeftParen)?;
        let param = self.expect_identifier()?;
        self.expect(&Token::RightParen)?;
        
        self.expect(&Token::Equal)?;
        
        let body = self.parse_expression()?;
        
        Ok(Statement::DefFn { name, param, body })
    }

    /// 解析 LIST 语句
    fn parse_list(&mut self) -> Result<Statement> {
        self.expect(&Token::List)?;
        
        // TODO: 解析行号范围 (LIST 10-50)
        // 目前简化处理
        Ok(Statement::List {
            start: None,
            end: None,
        })
    }

    /// 解析 RUN 语句
    fn parse_run(&mut self) -> Result<Statement> {
        self.expect(&Token::Run)?;
        
        let line_number = if let Token::Number(num) = self.current() {
            let num = *num as u16;
            self.advance();
            Some(num)
        } else {
            None
        };
        
        Ok(Statement::Run { line_number })
    }

    /// 解析 POKE 语句
    fn parse_poke(&mut self) -> Result<Statement> {
        self.expect(&Token::Poke)?;
        
        let address = self.parse_expression()?;
        self.expect(&Token::Comma)?;
        let value = self.parse_expression()?;
        
        Ok(Statement::Poke { address, value })
    }

    /// 解析 WAIT 语句（协程等待）
    /// 语法: WAIT seconds
    /// 例如: WAIT 0.5  等待 0.5 秒
    fn parse_wait(&mut self) -> Result<Statement> {
        self.expect(&Token::Wait)?;
        let seconds = self.parse_expression()?;
        Ok(Statement::Wait { seconds })
    }

    /// 解析 GET 语句
    fn parse_get(&mut self) -> Result<Statement> {
        self.expect(&Token::Get)?;
        
        let variable = self.expect_identifier()?;
        
        Ok(Statement::Get { variable })
    }

    /// 解析 LOAD 语句
    fn parse_load(&mut self) -> Result<Statement> {
        self.expect(&Token::Load)?;
        
        if let Token::String(filename) = self.current() {
            let filename = filename.clone();
            self.advance();
            Ok(Statement::Load { filename })
        } else {
            Err(BasicError::SyntaxError(
                "Expected filename string in LOAD".to_string()
            ))
        }
    }

    /// 解析 SAVE 语句
    fn parse_save(&mut self) -> Result<Statement> {
        self.expect(&Token::Save)?;
        
        if let Token::String(filename) = self.current() {
            let filename = filename.clone();
            self.advance();
            Ok(Statement::Save { filename })
        } else {
            Err(BasicError::SyntaxError(
                "Expected filename string in SAVE".to_string()
            ))
        }
    }

    /// 解析表达式
    fn parse_expression(&mut self) -> Result<Expr> {
        self.parse_or_expression()
    }

    /// 解析 OR 表达式（最低优先级）
    fn parse_or_expression(&mut self) -> Result<Expr> {
        let mut left = self.parse_and_expression()?;
        
        while self.current() == &Token::Or {
            self.advance();
            let right = self.parse_and_expression()?;
            left = Expr::binary(left, BinaryOperator::Or, right);
        }
        
        Ok(left)
    }

    /// 解析 AND 表达式
    fn parse_and_expression(&mut self) -> Result<Expr> {
        let mut left = self.parse_not_expression()?;
        
        while self.current() == &Token::And {
            self.advance();
            let right = self.parse_not_expression()?;
            left = Expr::binary(left, BinaryOperator::And, right);
        }
        
        Ok(left)
    }

    /// 解析 NOT 表达式
    fn parse_not_expression(&mut self) -> Result<Expr> {
        if self.current() == &Token::Not {
            self.advance();
            let operand = self.parse_not_expression()?;
            Ok(Expr::unary(UnaryOperator::Not, operand))
        } else {
            self.parse_relational_expression()
        }
    }

    /// 解析关系表达式 (=, <>, <, >, <=, >=)
    fn parse_relational_expression(&mut self) -> Result<Expr> {
        let left = self.parse_additive_expression()?;
        
        let op = match self.current() {
            Token::Equal => BinaryOperator::Equal,
            Token::NotEqual => BinaryOperator::NotEqual,
            Token::Less => BinaryOperator::Less,
            Token::Greater => BinaryOperator::Greater,
            Token::LessEqual => BinaryOperator::LessEqual,
            Token::GreaterEqual => BinaryOperator::GreaterEqual,
            _ => return Ok(left),
        };
        
        self.advance();
        let right = self.parse_additive_expression()?;
        Ok(Expr::binary(left, op, right))
    }

    /// 解析加减表达式
    fn parse_additive_expression(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplicative_expression()?;
        
        loop {
            let op = match self.current() {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Subtract,
                _ => break,
            };
            
            self.advance();
            let right = self.parse_multiplicative_expression()?;
            left = Expr::binary(left, op, right);
        }
        
        Ok(left)
    }

    /// 解析乘除表达式
    fn parse_multiplicative_expression(&mut self) -> Result<Expr> {
        let mut left = self.parse_power_expression()?;
        
        loop {
            let op = match self.current() {
                Token::Multiply => BinaryOperator::Multiply,
                Token::Divide => BinaryOperator::Divide,
                _ => break,
            };
            
            self.advance();
            let right = self.parse_power_expression()?;
            left = Expr::binary(left, op, right);
        }
        
        Ok(left)
    }

    /// 解析乘方表达式（右结合）
    fn parse_power_expression(&mut self) -> Result<Expr> {
        let left = self.parse_unary_expression()?;
        
        if self.current() == &Token::Power {
            self.advance();
            let right = self.parse_power_expression()?; // 右结合
            Ok(Expr::binary(left, BinaryOperator::Power, right))
        } else {
            Ok(left)
        }
    }

    /// 解析一元表达式（负号）
    fn parse_unary_expression(&mut self) -> Result<Expr> {
        if self.current() == &Token::Minus {
            self.advance();
            let operand = self.parse_unary_expression()?;
            Ok(Expr::unary(UnaryOperator::Minus, operand))
        } else if self.current() == &Token::Plus {
            // 一元加号直接跳过
            self.advance();
            self.parse_unary_expression()
        } else {
            self.parse_primary_expression()
        }
    }

    /// 解析基本表达式（字面量、变量、函数调用、括号）
    fn parse_primary_expression(&mut self) -> Result<Expr> {
        match self.current() {
            Token::Number(num) => {
                let num = *num;
                self.advance();
                Ok(Expr::Number(num))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String(s))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                
                // 检查是否为数组访问或函数调用
                if self.current() == &Token::LeftParen {
                    self.advance();
                    let args = self.parse_expression_list()?;
                    self.expect(&Token::RightParen)?;
                    
                    // 判断是数组访问还是函数调用
                    // 简化处理：如果名称以$结尾，且是字符串函数，视为函数
                    // 否则根据上下文判断（这里默认为数组）
                    Ok(Expr::ArrayAccess {
                        name,
                        indices: args,
                    })
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            // FN 用户自定义函数调用
            Token::Fn => {
                self.advance();
                let func_name = self.expect_identifier()?;
                self.expect(&Token::LeftParen)?;
                let args = self.parse_expression_list()?;
                self.expect(&Token::RightParen)?;
                // 函数名格式为 "FNname"
                Ok(Expr::FunctionCall {
                    name: format!("FN{}", func_name),
                    args,
                })
            }
            // 内置函数
            _ if self.is_function_token(self.current()) => {
                self.parse_function_call()
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(expr)
            }
            _ => Err(BasicError::ExpectedExpression(self.position)),
        }
    }

    /// 检查是否为函数 token
    fn is_function_token(&self, token: &Token) -> bool {
        matches!(
            token,
            Token::Sgn | Token::Int | Token::Abs | Token::Mod | Token::Sqr | Token::Rnd |
            Token::Log | Token::Exp | Token::Sin | Token::Cos | Token::Tan |
            Token::Atn | Token::Len | Token::Val | Token::Asc | Token::Peek |
            Token::Fre | Token::Pos | Token::Usr |
            Token::StrFunc | Token::ChrFunc | Token::LeftFunc | Token::RightFunc | Token::MidFunc |
            Token::Instr | Token::SpaceFunc
        )
    }

    /// 解析函数调用
    fn parse_function_call(&mut self) -> Result<Expr> {
        let func_token = self.current().clone();
        self.advance();
        
        // 获取函数名
        let name = match func_token {
            Token::Sgn => "SGN",
            Token::Int => "INT",
            Token::Abs => "ABS",
            Token::Mod => "MOD",
            Token::Sqr => "SQR",
            Token::Rnd => "RND",
            Token::Log => "LOG",
            Token::Exp => "EXP",
            Token::Sin => "SIN",
            Token::Cos => "COS",
            Token::Tan => "TAN",
            Token::Atn => "ATN",
            Token::Len => "LEN",
            Token::Val => "VAL",
            Token::Asc => "ASC",
            Token::Peek => "PEEK",
            Token::Fre => "FRE",
            Token::Pos => "POS",
            Token::Usr => "USR",
            Token::StrFunc => "STR$",
            Token::ChrFunc => "CHR$",
            Token::LeftFunc => "LEFT$",
            Token::RightFunc => "RIGHT$",
            Token::MidFunc => "MID$",
            Token::Instr => "INSTR",
            Token::SpaceFunc => "SPACE$",
            _ => unreachable!(),
        }.to_string();
        
        self.expect(&Token::LeftParen)?;
        let args = self.parse_expression_list()?;
        self.expect(&Token::RightParen)?;
        
        Ok(Expr::FunctionCall { name, args })
    }

    /// 解析表达式列表（逗号分隔）
    fn parse_expression_list(&mut self) -> Result<Vec<Expr>> {
        let mut exprs = Vec::new();
        
        // 空列表
        if self.current() == &Token::RightParen {
            return Ok(exprs);
        }
        
        loop {
            exprs.push(self.parse_expression()?);
            
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(exprs)
    }

    // 辅助方法

    /// 获取当前 token
    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    /// 前进到下一个 token
    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    /// 检查是否到达末尾
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len() || self.current() == &Token::Eof
    }

    /// 期望特定 token
    fn expect(&mut self, expected: &Token) -> Result<()> {
        if self.current() == expected {
            self.advance();
            Ok(())
        } else {
            Err(BasicError::SyntaxError(format!(
                "Expected {:?}, found {:?}",
                expected,
                self.current()
            )))
        }
    }

    /// 期望标识符并返回名称
    fn expect_identifier(&mut self) -> Result<String> {
        if let Token::Identifier(name) = self.current() {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(BasicError::SyntaxError(format!(
                "Expected identifier, found {:?}",
                self.current()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basic::tokenizer::Tokenizer;

    fn parse_line_helper(input: &str) -> Result<Option<ProgramLine>> {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize_line()?;
        let mut parser = Parser::new(tokens);
        parser.parse_line()
    }

    // Requirement: Expression Parsing - 简单算术表达式
    #[test]
    fn test_parse_simple_arithmetic() {
        let line = parse_line_helper("PRINT 2 + 3").unwrap().unwrap();
        assert_eq!(line.statements.len(), 1);
        
        match &line.statements[0] {
            Statement::Print { items } => {
                assert_eq!(items.len(), 1);
                if let PrintItem::Expr(Expr::BinaryOp { op, .. }) = &items[0] {
                    assert_eq!(*op, BinaryOperator::Add);
                }
            }
            _ => panic!("Expected Print statement"),
        }
    }

    // Requirement: Expression Parsing - 运算符优先级
    #[test]
    fn test_parse_operator_precedence() {
        let line = parse_line_helper("PRINT 2 + 3 * 4").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Print { items } => {
                if let PrintItem::Expr(Expr::BinaryOp { left, op, right }) = &items[0] {
                    assert_eq!(*op, BinaryOperator::Add);
                    assert_eq!(**left, Expr::Number(2.0));
                    // right 应该是 3 * 4
                    if let Expr::BinaryOp { left: l2, op: op2, right: r2 } = &**right {
                        assert_eq!(*op2, BinaryOperator::Multiply);
                        assert_eq!(**l2, Expr::Number(3.0));
                        assert_eq!(**r2, Expr::Number(4.0));
                    } else {
                        panic!("Expected multiplication on right side");
                    }
                }
            }
            _ => panic!("Expected Print statement"),
        }
    }

    // Requirement: Expression Parsing - 括号改变优先级
    #[test]
    fn test_parse_parentheses() {
        let line = parse_line_helper("PRINT (2 + 3) * 4").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Print { items } => {
                if let PrintItem::Expr(Expr::BinaryOp { left, op, right }) = &items[0] {
                    assert_eq!(*op, BinaryOperator::Multiply);
                    // left 应该是 (2 + 3)
                    if let Expr::BinaryOp { op: op_inner, .. } = &**left {
                        assert_eq!(*op_inner, BinaryOperator::Add);
                    }
                    assert_eq!(**right, Expr::Number(4.0));
                }
            }
            _ => panic!("Expected Print statement"),
        }
    }

    // Requirement: LET 语句解析 - 简单赋值
    #[test]
    fn test_parse_let() {
        let line = parse_line_helper("10 LET A = 10").unwrap().unwrap();
        assert_eq!(line.line_number, 10);
        
        match &line.statements[0] {
            Statement::Let { target, value } => {
                assert_eq!(*target, AssignTarget::Variable("A".to_string()));
                assert_eq!(*value, Expr::Number(10.0));
            }
            _ => panic!("Expected Let statement"),
        }
    }

    // Requirement: LET 语句解析 - LET 可选
    #[test]
    fn test_parse_implicit_let() {
        let line = parse_line_helper("10 A = 10").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Let { target, value } => {
                assert_eq!(*target, AssignTarget::Variable("A".to_string()));
                assert_eq!(*value, Expr::Number(10.0));
            }
            _ => panic!("Expected Let statement"),
        }
    }

    // Requirement: PRINT 语句解析 - 打印单个值
    #[test]
    fn test_parse_print_single() {
        let line = parse_line_helper("10 PRINT 42").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Print { items } => {
                assert_eq!(items.len(), 1);
                assert!(matches!(&items[0], PrintItem::Expr(Expr::Number(42.0))));
            }
            _ => panic!("Expected Print statement"),
        }
    }

    // Requirement: PRINT 语句解析 - 打印多个值（逗号分隔）
    #[test]
    fn test_parse_print_comma() {
        let line = parse_line_helper("10 PRINT A, B, C").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Print { items } => {
                assert_eq!(items.len(), 5); // A, Comma, B, Comma, C
            }
            _ => panic!("Expected Print statement"),
        }
    }

    // Requirement: 流程控制语句解析 - GOTO
    #[test]
    fn test_parse_goto() {
        let line = parse_line_helper("10 GOTO 100").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Goto { line_number } => {
                assert_eq!(*line_number, Expr::Number(100.0));
            }
            _ => panic!("Expected Goto statement"),
        }
    }

    // Requirement: 流程控制语句解析 - IF...THEN
    #[test]
    fn test_parse_if_then_line_number() {
        let line = parse_line_helper("10 IF A > 10 THEN 200").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::If { condition: _, then_part } => {
                assert_eq!(**then_part, ThenPart::LineNumber(200));
            }
            _ => panic!("Expected If statement"),
        }
    }

    // Requirement: 流程控制语句解析 - IF...THEN 执行语句
    #[test]
    fn test_parse_if_then_statement() {
        let line = parse_line_helper("10 IF A > 10 THEN PRINT A").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::If { condition: _, then_part } => {
                match &**then_part {
                    ThenPart::Statement(Statement::Print { .. }) => (),
                    _ => panic!("Expected Print in THEN clause"),
                }
            }
            _ => panic!("Expected If statement"),
        }
    }

    // Requirement: 循环语句解析 - 基本 FOR 循环
    #[test]
    fn test_parse_for() {
        let line = parse_line_helper("10 FOR I = 1 TO 10").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::For { var, start, end, step } => {
                assert_eq!(*var, "I");
                assert_eq!(*start, Expr::Number(1.0));
                assert_eq!(*end, Expr::Number(10.0));
                assert_eq!(*step, None);
            }
            _ => panic!("Expected For statement"),
        }
    }

    // Requirement: 循环语句解析 - 带 STEP 的循环
    #[test]
    fn test_parse_for_with_step() {
        let line = parse_line_helper("10 FOR I = 10 TO 1 STEP -1").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::For { var, start: _, end: _, step } => {
                assert_eq!(*var, "I");
                assert_eq!(step.is_some(), true);
            }
            _ => panic!("Expected For statement"),
        }
    }

    // Requirement: 循环语句解析 - NEXT 语句
    #[test]
    fn test_parse_next() {
        let line = parse_line_helper("20 NEXT I").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Next { var } => {
                assert_eq!(*var, Some("I".to_string()));
            }
            _ => panic!("Expected Next statement"),
        }
    }

    // Requirement: 语句分隔符 - 单行多语句
    #[test]
    fn test_parse_multiple_statements() {
        let line = parse_line_helper("10 A=1: B=2: PRINT A+B").unwrap().unwrap();
        assert_eq!(line.statements.len(), 3);
    }

    // Requirement: 语句分隔符 - 单行 FOR 循环
    #[test]
    fn test_parse_single_line_for_loop() {
        let line = parse_line_helper("10 FOR I=1 TO 10: PRINT I: NEXT I").unwrap().unwrap();
        assert_eq!(line.statements.len(), 3);
        
        assert!(matches!(line.statements[0], Statement::For { .. }));
        assert!(matches!(line.statements[1], Statement::Print { .. }));
        assert!(matches!(line.statements[2], Statement::Next { .. }));
    }

    // Requirement: INPUT 语句解析 - 基本 INPUT
    #[test]
    fn test_parse_input() {
        let line = parse_line_helper("10 INPUT A").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Input { prompt, variables } => {
                assert_eq!(*prompt, None);
                assert_eq!(variables.len(), 1);
            }
            _ => panic!("Expected Input statement"),
        }
    }

    // Requirement: INPUT 语句解析 - 带提示符的 INPUT
    #[test]
    fn test_parse_input_with_prompt() {
        let line = parse_line_helper("10 INPUT \"ENTER VALUE\"; A").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Input { prompt, variables } => {
                assert_eq!(*prompt, Some("ENTER VALUE".to_string()));
                assert_eq!(variables.len(), 1);
            }
            _ => panic!("Expected Input statement"),
        }
    }

    // Requirement: DIM 语句解析 - 一维数组声明
    #[test]
    fn test_parse_dim() {
        let line = parse_line_helper("10 DIM A(10)").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Dim { arrays } => {
                assert_eq!(arrays.len(), 1);
                assert_eq!(arrays[0].name, "A");
                assert_eq!(arrays[0].dimensions.len(), 1);
            }
            _ => panic!("Expected Dim statement"),
        }
    }

    // Requirement: 函数调用解析 - 单参数函数
    #[test]
    fn test_parse_function_call() {
        let line = parse_line_helper("PRINT SIN(X)").unwrap().unwrap();
        
        match &line.statements[0] {
            Statement::Print { items } => {
                if let PrintItem::Expr(Expr::FunctionCall { name, args }) = &items[0] {
                    assert_eq!(name, "SIN");
                    assert_eq!(args.len(), 1);
                }
            }
            _ => panic!("Expected Print statement"),
        }
    }
}

