/// BASIC 解释器错误类型
///
/// 定义所有可能的错误类型，对应原 BASIC 6502 的错误消息
#[derive(Debug, Clone, PartialEq)]
pub enum BasicError {
    // 词法分析错误
    IllegalCharacter(char, usize, String),  // 字符，位置，上下文
    UnterminatedString(usize),
    InvalidNumber(String, usize),
    
    // 语法错误
    SyntaxError(String),
    ExpectedExpression(usize),
    UnmatchedParenthesis(usize),
    InvalidStatement(usize),
    
    // 运行时错误
    UndefinedLine(u16),
    UndefinedVariable(String),
    DivisionByZero,
    TypeMismatch(String),
    SubscriptOutOfRange(String),
    RedimensionedArray(String),
    OutOfData,
    OutOfMemory,
    StackOverflow,
    IllegalQuantity(String),
    
    // 流程控制错误
    ReturnWithoutGosub,
    NextWithoutFor(String),
    CantContinue,
    
    // I/O 错误
    FileNotFound(String),
    IoError(String),
    
    // 其他
    BreakIn(u16),  // STOP 或 Ctrl+C 中断
}

impl std::fmt::Display for BasicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BasicError::IllegalCharacter(ch, pos, context) => {
                write!(f, "ILLEGAL CHARACTER '{}' AT POSITION {}\n{}", ch, pos, context)
            }
            BasicError::UnterminatedString(pos) => {
                write!(f, "?UNTERMINATED STRING AT POSITION {}", pos)
            }
            BasicError::InvalidNumber(num, pos) => {
                write!(f, "?INVALID NUMBER '{}' AT POSITION {}", num, pos)
            }
            BasicError::SyntaxError(msg) => {
                write!(f, "?SYNTAX ERROR: {}", msg)
            }
            BasicError::ExpectedExpression(pos) => {
                write!(f, "?EXPECTED EXPRESSION AT POSITION {}", pos)
            }
            BasicError::UnmatchedParenthesis(pos) => {
                write!(f, "?UNMATCHED PARENTHESIS AT POSITION {}", pos)
            }
            BasicError::InvalidStatement(pos) => {
                write!(f, "?INVALID STATEMENT AT POSITION {}", pos)
            }
            BasicError::UndefinedLine(line) => {
                write!(f, "?UNDEF'D STATEMENT ERROR IN {}", line)
            }
            BasicError::UndefinedVariable(var) => {
                write!(f, "?UNDEFINED VARIABLE: {}", var)
            }
            BasicError::DivisionByZero => {
                write!(f, "?DIVISION BY ZERO ERROR")
            }
            BasicError::TypeMismatch(msg) => {
                write!(f, "?TYPE MISMATCH ERROR: {}", msg)
            }
            BasicError::SubscriptOutOfRange(var) => {
                write!(f, "?SUBSCRIPT OUT OF RANGE: {}", var)
            }
            BasicError::RedimensionedArray(var) => {
                write!(f, "?REDIM'D ARRAY ERROR: {}", var)
            }
            BasicError::OutOfData => {
                write!(f, "?OUT OF DATA ERROR")
            }
            BasicError::OutOfMemory => {
                write!(f, "?OUT OF MEMORY ERROR")
            }
            BasicError::StackOverflow => {
                write!(f, "?STACK OVERFLOW ERROR")
            }
            BasicError::IllegalQuantity(msg) => {
                write!(f, "?ILLEGAL QUANTITY: {}", msg)
            }
            BasicError::ReturnWithoutGosub => {
                write!(f, "?RETURN WITHOUT GOSUB ERROR")
            }
            BasicError::NextWithoutFor(var) => {
                write!(f, "?NEXT WITHOUT FOR: {}", var)
            }
            BasicError::CantContinue => {
                write!(f, "?CAN'T CONTINUE ERROR")
            }
            BasicError::FileNotFound(file) => {
                write!(f, "?FILE NOT FOUND: {}", file)
            }
            BasicError::IoError(msg) => {
                write!(f, "?I/O ERROR: {}", msg)
            }
            BasicError::BreakIn(line) => {
                write!(f, "?BREAK IN {}", line)
            }
        }
    }
}

impl std::error::Error for BasicError {}

/// Result 类型别名，简化错误处理
pub type Result<T> = std::result::Result<T, BasicError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BasicError::DivisionByZero;
        assert_eq!(err.to_string(), "?DIVISION BY ZERO ERROR");
        
        let err = BasicError::UndefinedLine(100);
        assert_eq!(err.to_string(), "?UNDEF'D STATEMENT ERROR IN 100");
        
        let err = BasicError::TypeMismatch("expected number".to_string());
        assert_eq!(err.to_string(), "?TYPE MISMATCH ERROR: expected number");
    }

    #[test]
    fn test_error_clone() {
        let err1 = BasicError::OutOfMemory;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_illegal_character() {
        let err = BasicError::IllegalCharacter('@', 5, "Context: test@\n     ^".to_string());
        assert!(err.to_string().contains("ILLEGAL CHARACTER '@'"));
        assert!(err.to_string().contains("AT POSITION 5"));
    }

    #[test]
    fn test_result_type() {
        fn test_fn() -> Result<i32> {
            Err(BasicError::OutOfData)
        }
        
        let result = test_fn();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), BasicError::OutOfData);
    }
}

