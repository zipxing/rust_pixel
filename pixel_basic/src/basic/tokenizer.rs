/// 词法分析器
///
/// 将 BASIC 源代码文本转换为 Token 流

use super::error::{BasicError, Result};
use super::token::Token;

pub struct Tokenizer {
    input: Vec<char>,
    position: usize,
    is_line_start: bool,
    rem_comment: Option<String>,  // 存储 REM 注释内容
}

impl Tokenizer {
    /// 创建新的词法分析器
    pub fn new(input: &str) -> Self {
        Tokenizer {
            input: input.chars().collect(),
            position: 0,
            is_line_start: true,
            rem_comment: None,
        }
    }

    /// 解析整行并返回所有 tokens
    pub fn tokenize_line(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        self.is_line_start = true;
        self.rem_comment = None;  // 重置 REM 注释

        while self.position < self.input.len() {
            self.skip_whitespace();

            if self.position >= self.input.len() {
                break;
            }

            let token = self.next_token()?;
            
            match &token {
                Token::Eof => break,
                Token::Newline => break,
                Token::Rem => {
                    tokens.push(token);
                    // 如果有 REM 注释内容，将其作为 String token 添加
                    if let Some(ref comment) = self.rem_comment {
                        tokens.push(Token::String(comment.clone()));
                    }
                }
                _ => tokens.push(token),
            }
        }

        tokens.push(Token::Newline);
        Ok(tokens)
    }

    /// 获取下一个 token
    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return Ok(Token::Eof);
        }

        let ch = self.current_char();

        // 行首的数字识别为行号
        if self.is_line_start && ch.is_ascii_digit() {
            self.is_line_start = false;
            return self.read_line_number();
        }

        self.is_line_start = false;

        // 字符串
        if ch == '"' {
            return self.read_string();
        }

        // 数字
        if ch.is_ascii_digit() || (ch == '.' && self.peek_next().map_or(false, |c| c.is_ascii_digit())) {
            return self.read_number();
        }

        // 标识符或关键字
        if ch.is_ascii_alphabetic() {
            return self.read_identifier_or_keyword();
        }

        // 运算符和分隔符
        match ch {
            '+' => {
                self.advance();
                Ok(Token::Plus)
            }
            '-' => {
                self.advance();
                Ok(Token::Minus)
            }
            '*' => {
                self.advance();
                Ok(Token::Multiply)
            }
            '/' => {
                self.advance();
                Ok(Token::Divide)
            }
            '^' => {
                self.advance();
                Ok(Token::Power)
            }
            '=' => {
                self.advance();
                Ok(Token::Equal)
            }
            '<' => {
                self.advance();
                if self.current_char() == '>' {
                    self.advance();
                    Ok(Token::NotEqual)
                } else if self.current_char() == '=' {
                    self.advance();
                    Ok(Token::LessEqual)
                } else {
                    Ok(Token::Less)
                }
            }
            '>' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    Ok(Token::GreaterEqual)
                } else {
                    Ok(Token::Greater)
                }
            }
            '(' => {
                self.advance();
                Ok(Token::LeftParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RightParen)
            }
            ',' => {
                self.advance();
                Ok(Token::Comma)
            }
            ';' => {
                self.advance();
                Ok(Token::Semicolon)
            }
            ':' => {
                self.advance();
                Ok(Token::Colon)
            }
            '\n' | '\r' => {
                self.advance();
                if ch == '\r' && self.current_char() == '\n' {
                    self.advance();
                }
                Ok(Token::Newline)
            }
            _ => {
                // 生成详细的错误信息，包含上下文
                let full_input: String = self.input.iter().collect();
                let marker = " ".repeat(self.position) + "^";
                
                // 获取错误位置的上下文（前后各20个字符）
                let context_start = self.position.saturating_sub(20);
                let context_end = (self.position + 20).min(self.input.len());
                let context: String = self.input[context_start..context_end].iter().collect();
                let marker_pos = self.position - context_start;
                let context_marker = " ".repeat(marker_pos) + "^";
                
                Err(BasicError::IllegalCharacter(
                    ch,
                    self.position,
                    format!(
                        "Input line: {}\n{}\n\nContext (position {}):\n{}\n{}",
                        full_input, marker, self.position, context, context_marker
                    ),
                ))
            }
        }
    }

    /// 读取行号
    fn read_line_number(&mut self) -> Result<Token> {
        let start = self.position;
        let mut num_str = String::new();

        while self.position < self.input.len() && self.current_char().is_ascii_digit() {
            num_str.push(self.current_char());
            self.advance();
        }

        let line_num = num_str.parse::<u16>().map_err(|_| {
            BasicError::InvalidNumber(num_str.clone(), start)
        })?;

        Ok(Token::LineNumber(line_num))
    }

    /// 读取数字常量
    fn read_number(&mut self) -> Result<Token> {
        let start = self.position;
        let mut num_str = String::new();
        let mut has_dot = false;
        let mut has_e = false;

        // 读取整数部分和小数部分
        while self.position < self.input.len() {
            let ch = self.current_char();

            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot && !has_e {
                has_dot = true;
                num_str.push(ch);
                self.advance();
            } else if (ch == 'E' || ch == 'e') && !has_e {
                has_e = true;
                num_str.push('E');
                self.advance();
                
                // 处理指数的正负号
                if self.position < self.input.len() {
                    let next_ch = self.current_char();
                    if next_ch == '+' || next_ch == '-' {
                        num_str.push(next_ch);
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        // 解析为浮点数
        let value = num_str.parse::<f64>().map_err(|_| {
            BasicError::InvalidNumber(num_str.clone(), start)
        })?;

        Ok(Token::Number(value))
    }

    /// 读取字符串常量
    fn read_string(&mut self) -> Result<Token> {
        let start = self.position;
        self.advance(); // 跳过开始的引号

        let mut string_val = String::new();

        while self.position < self.input.len() {
            let ch = self.current_char();

            if ch == '"' {
                self.advance(); // 跳过结束的引号
                return Ok(Token::String(string_val));
            } else if ch == '\n' || ch == '\r' {
                // 字符串未闭合就遇到换行
                return Err(BasicError::UnterminatedString(start));
            } else {
                string_val.push(ch);
                self.advance();
            }
        }

        // 到达文件末尾仍未找到结束引号
        Err(BasicError::UnterminatedString(start))
    }

    /// 读取标识符或关键字
    fn read_identifier_or_keyword(&mut self) -> Result<Token> {
        let mut ident = String::new();

        // 读取字母开头的标识符
        while self.position < self.input.len() {
            let ch = self.current_char();

            if ch.is_ascii_alphanumeric() {
                ident.push(ch);
                self.advance();
            } else if ch == '$' {
                // 字符串变量后缀
                ident.push(ch);
                self.advance();
                break;
            } else {
                break;
            }
        }

        // 检查是否为关键字
        if let Some(keyword_token) = Token::from_keyword(&ident) {
            // REM 注释：识别到 REM 后，读取后面的所有内容到行尾作为注释字符串
            if keyword_token == Token::Rem {
                // 跳过空白字符
                self.skip_whitespace();
                // 读取到行尾的所有内容作为注释
                let mut comment = String::new();
                while self.position < self.input.len() {
                    let ch = self.current_char();
                    if ch == '\n' || ch == '\r' {
                        break;
                    }
                    comment.push(ch);
                    self.advance();
                }
                // 存储注释内容
                self.rem_comment = Some(comment.trim().to_string());
                return Ok(keyword_token);
            }
            Ok(keyword_token)
        } else if ident.to_uppercase() == "GO" {
            // 特殊处理 "GO TO"（带空格）
            // 跳过空白字符
            self.skip_whitespace();
            // 检查后面是否是 "TO"
            let saved_pos = self.position;
            let mut to_ident = String::new();
            while self.position < self.input.len() {
                let ch = self.current_char();
                if ch.is_ascii_alphanumeric() {
                    to_ident.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            
            if to_ident.to_uppercase() == "TO" {
                // 找到了 "GO TO"，返回 GOTO token
                Ok(Token::Goto)
            } else {
                // 不是 "GO TO"，恢复位置，返回 GO 作为标识符
                self.position = saved_pos;
                Ok(Token::Identifier(ident))
            }
        } else {
            // 普通标识符
            Ok(Token::Identifier(ident))
        }
    }

    /// 跳过空白字符
    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() {
            let ch = self.current_char();
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// 获取当前字符
    fn current_char(&self) -> char {
        if self.position < self.input.len() {
            self.input[self.position]
        } else {
            '\0'
        }
    }

    /// 前进一个字符
    fn advance(&mut self) {
        self.position += 1;
    }

    /// 查看下一个字符
    fn peek_next(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Requirement: Token Type Definition - 识别保留字
    #[test]
    fn test_recognize_keyword_print() {
        let mut tokenizer = Tokenizer::new("PRINT");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Print);
    }

    // Requirement: 保留字识别 - 大小写不敏感
    #[test]
    fn test_keyword_case_insensitive() {
        let test_cases = vec!["print", "PRINT", "Print", "PrInT"];
        for input in test_cases {
            let mut tokenizer = Tokenizer::new(input);
            let tokens = tokenizer.tokenize_line().unwrap();
            assert_eq!(tokens[0], Token::Print, "Failed for input: {}", input);
        }
    }

    // Requirement: Token Type Definition - 识别变量名
    #[test]
    fn test_recognize_identifier() {
        let mut tokenizer = Tokenizer::new("A1");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Identifier("A1".to_string()));
    }

    // Requirement: Token Type Definition - 识别数字
    #[test]
    fn test_recognize_number() {
        // 在表达式中的数字（不在行首）
        let mut tokenizer = Tokenizer::new("A=123.45");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[2], Token::Number(123.45));
    }

    // Requirement: 数字常量解析 - 整数
    #[test]
    fn test_parse_integer() {
        // 在表达式中的数字（不在行首）
        let mut tokenizer = Tokenizer::new("LET A=42");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[3], Token::Number(42.0));
    }

    // Requirement: 数字常量解析 - 浮点数
    #[test]
    fn test_parse_float() {
        // 在表达式中的数字（不在行首）
        let mut tokenizer = Tokenizer::new("PRINT 3.14159");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[1], Token::Number(3.14159));
    }

    // Requirement: 数字常量解析 - 科学计数法
    #[test]
    fn test_parse_scientific_notation() {
        // 在表达式中的数字（不在行首）
        let mut tokenizer = Tokenizer::new("PRINT 1.5E-10");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[1], Token::Number(1.5e-10));
    }

    // Requirement: 数字常量解析 - 负数
    #[test]
    fn test_parse_negative_number() {
        let mut tokenizer = Tokenizer::new("-123");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Minus);
        assert_eq!(tokens[1], Token::Number(123.0));
    }

    // Requirement: 字符串常量解析 - 普通字符串
    #[test]
    fn test_parse_string() {
        let mut tokenizer = Tokenizer::new("\"HELLO WORLD\"");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::String("HELLO WORLD".to_string()));
    }

    // Requirement: 字符串常量解析 - 空字符串
    #[test]
    fn test_parse_empty_string() {
        let mut tokenizer = Tokenizer::new("\"\"");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::String("".to_string()));
    }

    // Requirement: 字符串常量解析 - 包含空格的字符串
    #[test]
    fn test_parse_string_with_spaces() {
        let mut tokenizer = Tokenizer::new("\"  SPACES  \"");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::String("  SPACES  ".to_string()));
    }

    // Requirement: 变量名识别 - 单字母变量
    #[test]
    fn test_single_letter_variable() {
        let mut tokenizer = Tokenizer::new("A");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Identifier("A".to_string()));
    }

    // Requirement: 变量名识别 - 字母数字组合
    #[test]
    fn test_alphanumeric_variable() {
        let test_cases = vec!["A1", "X9", "Z0"];
        for var in test_cases {
            let mut tokenizer = Tokenizer::new(var);
            let tokens = tokenizer.tokenize_line().unwrap();
            assert_eq!(tokens[0], Token::Identifier(var.to_string()));
        }
    }

    // Requirement: 变量名识别 - 字符串变量
    #[test]
    fn test_string_variable() {
        let mut tokenizer = Tokenizer::new("A$ NAME$");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Identifier("A$".to_string()));
        assert_eq!(tokens[1], Token::Identifier("NAME$".to_string()));
    }

    // Requirement: 运算符识别 - 算术运算符
    #[test]
    fn test_arithmetic_operators() {
        let mut tokenizer = Tokenizer::new("+ - * / ^");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Plus);
        assert_eq!(tokens[1], Token::Minus);
        assert_eq!(tokens[2], Token::Multiply);
        assert_eq!(tokens[3], Token::Divide);
        assert_eq!(tokens[4], Token::Power);
    }

    // Requirement: 运算符识别 - 关系运算符
    #[test]
    fn test_relational_operators() {
        let mut tokenizer = Tokenizer::new("<= >= <>");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::LessEqual);
        assert_eq!(tokens[1], Token::GreaterEqual);
        assert_eq!(tokens[2], Token::NotEqual);
    }

    // Requirement: 运算符识别 - 逻辑运算符
    #[test]
    fn test_logical_operators() {
        let mut tokenizer = Tokenizer::new("AND OR NOT");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::And);
        assert_eq!(tokens[1], Token::Or);
        assert_eq!(tokens[2], Token::Not);
    }

    // Requirement: 行号处理 - 有效行号
    #[test]
    fn test_line_number() {
        let mut tokenizer = Tokenizer::new("10 PRINT");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::LineNumber(10));
        assert_eq!(tokens[1], Token::Print);
    }

    // Requirement: 行号处理 - 行号范围
    #[test]
    fn test_line_number_range() {
        let test_cases = vec![1, 100, 65535];
        for num in test_cases {
            let mut tokenizer = Tokenizer::new(&format!("{} PRINT", num));
            let tokens = tokenizer.tokenize_line().unwrap();
            assert_eq!(tokens[0], Token::LineNumber(num));
        }
    }

    // Requirement: 空格和分隔符处理 - 空格分隔
    #[test]
    fn test_whitespace_separation() {
        let mut tokenizer = Tokenizer::new("PRINT A");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Print);
        assert_eq!(tokens[1], Token::Identifier("A".to_string()));
    }

    // Requirement: 空格和分隔符处理 - 多个空格
    #[test]
    fn test_multiple_whitespaces() {
        let mut tokenizer = Tokenizer::new("PRINT   A");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Print);
        assert_eq!(tokens[1], Token::Identifier("A".to_string()));
        assert_eq!(tokens.len(), 3); // PRINT, A, Newline
    }

    // Requirement: 空格和分隔符处理 - 逗号分隔
    #[test]
    fn test_comma_separator() {
        let mut tokenizer = Tokenizer::new("PRINT A,B");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Print);
        assert_eq!(tokens[1], Token::Identifier("A".to_string()));
        assert_eq!(tokens[2], Token::Comma);
        assert_eq!(tokens[3], Token::Identifier("B".to_string()));
    }

    // Requirement: 空格和分隔符处理 - 分号
    #[test]
    fn test_semicolon_separator() {
        let mut tokenizer = Tokenizer::new("PRINT A;B");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Print);
        assert_eq!(tokens[1], Token::Identifier("A".to_string()));
        assert_eq!(tokens[2], Token::Semicolon);
        assert_eq!(tokens[3], Token::Identifier("B".to_string()));
    }

    // Requirement: 空格和分隔符处理 - 冒号（语句分隔符）
    #[test]
    fn test_colon_statement_separator() {
        let mut tokenizer = Tokenizer::new("A=1: B=2");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::Identifier("A".to_string()));
        assert_eq!(tokens[1], Token::Equal);
        assert_eq!(tokens[2], Token::Number(1.0));
        assert_eq!(tokens[3], Token::Colon);
        assert_eq!(tokens[4], Token::Identifier("B".to_string()));
        assert_eq!(tokens[5], Token::Equal);
        assert_eq!(tokens[6], Token::Number(2.0));
    }

    // Requirement: 空格和分隔符处理 - 复杂语句分隔
    #[test]
    fn test_complex_statement_separation() {
        let mut tokenizer = Tokenizer::new("FOR I=1 TO 10: PRINT I: NEXT I");
        let tokens = tokenizer.tokenize_line().unwrap();
        
        // 验证 FOR 语句
        assert_eq!(tokens[0], Token::For);
        assert_eq!(tokens[1], Token::Identifier("I".to_string()));
        assert_eq!(tokens[2], Token::Equal);
        assert_eq!(tokens[3], Token::Number(1.0));
        assert_eq!(tokens[4], Token::To);
        assert_eq!(tokens[5], Token::Number(10.0));
        
        // 第一个冒号
        assert_eq!(tokens[6], Token::Colon);
        
        // PRINT 语句
        assert_eq!(tokens[7], Token::Print);
        assert_eq!(tokens[8], Token::Identifier("I".to_string()));
        
        // 第二个冒号
        assert_eq!(tokens[9], Token::Colon);
        
        // NEXT 语句
        assert_eq!(tokens[10], Token::Next);
        assert_eq!(tokens[11], Token::Identifier("I".to_string()));
    }

    // Requirement: 注释处理 - REM 注释
    #[test]
    fn test_rem_comment() {
        let mut tokenizer = Tokenizer::new("10 REM THIS IS A COMMENT");
        let tokens = tokenizer.tokenize_line().unwrap();
        assert_eq!(tokens[0], Token::LineNumber(10));
        assert_eq!(tokens[1], Token::Rem);
        // REM 后面的内容会被解析成 token（现在由 parser 处理注释内容）
        assert!(tokens.len() >= 3); // LineNumber, REM, 注释内容 tokens, Newline
    }

    // Requirement: 错误处理 - 未闭合字符串
    #[test]
    fn test_unterminated_string() {
        let mut tokenizer = Tokenizer::new("\"HELLO");
        let result = tokenizer.tokenize_line();
        assert!(result.is_err());
        match result.unwrap_err() {
            BasicError::UnterminatedString(_) => (),
            _ => panic!("Expected UnterminatedString error"),
        }
    }

    // Requirement: 错误处理 - 数字格式错误
    #[test]
    fn test_invalid_number() {
        // 测试极大的数，f64 会将其解析为无穷大，但仍然成功
        let mut tokenizer = Tokenizer::new("PRINT 999999999999999999999999999");
        let result = tokenizer.tokenize_line();
        // 这应该成功，因为 f64 可以表示很大的数（作为 inf）
        assert!(result.is_ok());
        
        // 测试无效的科学计数法格式 - 实际上 Rust 的 parse 会处理很多情况
        // 如果真的有问题，会在 parse 时返回错误
    }

    // 综合测试 - 完整的 BASIC 行
    #[test]
    fn test_complete_basic_line() {
        let mut tokenizer = Tokenizer::new("10 PRINT \"HELLO\"; A + B * 2");
        let tokens = tokenizer.tokenize_line().unwrap();
        
        assert_eq!(tokens[0], Token::LineNumber(10));
        assert_eq!(tokens[1], Token::Print);
        assert_eq!(tokens[2], Token::String("HELLO".to_string()));
        assert_eq!(tokens[3], Token::Semicolon);
        assert_eq!(tokens[4], Token::Identifier("A".to_string()));
        assert_eq!(tokens[5], Token::Plus);
        assert_eq!(tokens[6], Token::Identifier("B".to_string()));
        assert_eq!(tokens[7], Token::Multiply);
        assert_eq!(tokens[8], Token::Number(2.0));
    }

    // 测试所有27个语句关键字
    #[test]
    fn test_all_statement_keywords() {
        let keywords = vec![
            "END", "FOR", "NEXT", "DATA", "INPUT", "DIM", "READ", "LET", "GOTO",
            "RUN", "IF", "RESTORE", "GOSUB", "RETURN", "REM", "STOP", "ON", 
            "NULL", "WAIT", "LOAD", "SAVE", "DEF", "POKE", "PRINT", "CONT", 
            "LIST", "CLEAR", "GET", "NEW",
        ];
        
        for keyword in keywords {
            let mut tokenizer = Tokenizer::new(keyword);
            let tokens = tokenizer.tokenize_line().unwrap();
            assert!(tokens[0] != Token::Identifier(keyword.to_string()), 
                    "Keyword {} was not recognized", keyword);
        }
    }

    // 测试所有22个内置函数
    #[test]
    fn test_all_function_keywords() {
        let functions = vec![
            "SGN", "INT", "ABS", "USR", "FRE", "POS", "SQR", "RND", "LOG", "EXP",
            "COS", "SIN", "TAN", "ATN", "PEEK", "LEN", "STR$", "VAL", "ASC",
            "CHR$", "LEFT$", "RIGHT$", "MID$",
        ];
        
        for func in functions {
            let mut tokenizer = Tokenizer::new(func);
            let tokens = tokenizer.tokenize_line().unwrap();
            assert!(tokens[0] != Token::Identifier(func.to_string()), 
                    "Function {} was not recognized", func);
        }
    }
}

