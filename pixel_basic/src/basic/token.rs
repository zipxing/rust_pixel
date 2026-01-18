/// Token 类型定义
///
/// 表示 BASIC 程序的词法单元
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // 行号
    LineNumber(u16),
    
    // 字面量
    Number(f64),
    String(String),
    Identifier(String),
    
    // 语句关键字（27个）
    End,
    For,
    Next,
    Data,
    Input,
    Dim,
    Read,
    Let,
    Goto,
    Run,
    If,
    Restore,
    Gosub,
    Return,
    Rem,
    Stop,
    On,
    Null,
    Wait,
    Load,
    Save,
    Def,
    Poke,
    Print,
    Cont,
    List,
    Clear,
    Get,
    New,
    
    // 控制流关键字
    Then,
    To,
    Step,
    Fn,  // FN 用于用户自定义函数调用
    
    // 内置函数（22个）
    // 数学函数
    Sgn,
    Int,
    Abs,
    Mod,  // MOD 取模函数
    Usr,
    Fre,
    Pos,
    Sqr,
    Rnd,
    Log,
    Exp,
    Cos,
    Sin,
    Tan,
    Atn,
    Peek,
    // 字符串函数
    Len,
    StrFunc,    // STR$
    Val,
    Asc,
    ChrFunc,    // CHR$
    LeftFunc,   // LEFT$
    RightFunc,  // RIGHT$
    MidFunc,    // MID$
    Instr,      // INSTR
    SpaceFunc,  // SPACE$
    
    // 格式化函数（PRINT 用）
    Tab,
    Spc,
    
    // 运算符
    Plus,           // +
    Minus,          // -
    Multiply,       // *
    Divide,         // /
    Power,          // ^
    Equal,          // =
    NotEqual,       // <>
    Less,           // <
    Greater,        // >
    LessEqual,      // <=
    GreaterEqual,   // >=
    And,
    Or,
    Not,
    
    // 分隔符和标点
    LeftParen,      // (
    RightParen,     // )
    Comma,          // ,
    Semicolon,      // ;
    Colon,          // : (语句分隔符)
    
    // 特殊
    Newline,
    Eof,
}

impl Token {
    /// 从字符串识别关键字
    pub fn from_keyword(s: &str) -> Option<Token> {
        let upper = s.to_uppercase();
        match upper.as_str() {
            // 语句关键字
            "END" => Some(Token::End),
            "FOR" => Some(Token::For),
            "NEXT" => Some(Token::Next),
            "DATA" => Some(Token::Data),
            "INPUT" => Some(Token::Input),
            "DIM" => Some(Token::Dim),
            "READ" => Some(Token::Read),
            "LET" => Some(Token::Let),
            "GOTO" => Some(Token::Goto),
            "RUN" => Some(Token::Run),
            "IF" => Some(Token::If),
            "RESTORE" => Some(Token::Restore),
            "GOSUB" => Some(Token::Gosub),
            "RETURN" => Some(Token::Return),
            "REM" => Some(Token::Rem),
            "STOP" => Some(Token::Stop),
            "ON" => Some(Token::On),
            "NULL" => Some(Token::Null),
            "WAIT" => Some(Token::Wait),
            "LOAD" => Some(Token::Load),
            "SAVE" => Some(Token::Save),
            "DEF" => Some(Token::Def),
            "POKE" => Some(Token::Poke),
            "PRINT" => Some(Token::Print),
            "CONT" => Some(Token::Cont),
            "LIST" => Some(Token::List),
            "CLEAR" => Some(Token::Clear),
            "GET" => Some(Token::Get),
            "NEW" => Some(Token::New),
            
            // 控制流关键字
            "THEN" => Some(Token::Then),
            "TO" => Some(Token::To),
            "STEP" => Some(Token::Step),
            "FN" => Some(Token::Fn),
            
            // 数学函数
            "SGN" => Some(Token::Sgn),
            "INT" => Some(Token::Int),
            "ABS" => Some(Token::Abs),
            "MOD" => Some(Token::Mod),
            "USR" => Some(Token::Usr),
            "FRE" => Some(Token::Fre),
            "POS" => Some(Token::Pos),
            "SQR" => Some(Token::Sqr),
            "RND" => Some(Token::Rnd),
            "LOG" => Some(Token::Log),
            "EXP" => Some(Token::Exp),
            "COS" => Some(Token::Cos),
            "SIN" => Some(Token::Sin),
            "TAN" => Some(Token::Tan),
            "ATN" => Some(Token::Atn),
            "PEEK" => Some(Token::Peek),
            
            // 字符串函数
            "LEN" => Some(Token::Len),
            "STR$" => Some(Token::StrFunc),
            "VAL" => Some(Token::Val),
            "ASC" => Some(Token::Asc),
            "CHR$" => Some(Token::ChrFunc),
            "LEFT$" => Some(Token::LeftFunc),
            "RIGHT$" => Some(Token::RightFunc),
            "MID$" => Some(Token::MidFunc),
            "INSTR" => Some(Token::Instr),
            "SPACE$" => Some(Token::SpaceFunc),
            
            // 格式化函数
            "TAB" => Some(Token::Tab),
            "SPC" => Some(Token::Spc),
            
            // 逻辑运算符
            "AND" => Some(Token::And),
            "OR" => Some(Token::Or),
            "NOT" => Some(Token::Not),
            
            _ => None,
        }
    }
    
    /// 判断是否为语句关键字
    pub fn is_statement_keyword(&self) -> bool {
        matches!(
            self,
            Token::End | Token::For | Token::Next | Token::Data | Token::Input |
            Token::Dim | Token::Read | Token::Let | Token::Goto | Token::Run |
            Token::If | Token::Restore | Token::Gosub | Token::Return | Token::Rem |
            Token::Stop | Token::On | Token::Null | Token::Wait | Token::Load |
            Token::Save | Token::Def | Token::Poke | Token::Print | Token::Cont |
            Token::List | Token::Clear | Token::Get | Token::New
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_recognition_case_insensitive() {
        assert_eq!(Token::from_keyword("PRINT"), Some(Token::Print));
        assert_eq!(Token::from_keyword("print"), Some(Token::Print));
        assert_eq!(Token::from_keyword("Print"), Some(Token::Print));
        assert_eq!(Token::from_keyword("PrInT"), Some(Token::Print));
    }

    #[test]
    fn test_all_statement_keywords() {
        assert_eq!(Token::from_keyword("END"), Some(Token::End));
        assert_eq!(Token::from_keyword("FOR"), Some(Token::For));
        assert_eq!(Token::from_keyword("NEXT"), Some(Token::Next));
        assert_eq!(Token::from_keyword("GOTO"), Some(Token::Goto));
        assert_eq!(Token::from_keyword("GOSUB"), Some(Token::Gosub));
        assert_eq!(Token::from_keyword("RETURN"), Some(Token::Return));
        assert_eq!(Token::from_keyword("IF"), Some(Token::If));
        assert_eq!(Token::from_keyword("THEN"), Some(Token::Then));
        assert_eq!(Token::from_keyword("REM"), Some(Token::Rem));
    }

    #[test]
    fn test_function_keywords() {
        assert_eq!(Token::from_keyword("SGN"), Some(Token::Sgn));
        assert_eq!(Token::from_keyword("INT"), Some(Token::Int));
        assert_eq!(Token::from_keyword("ABS"), Some(Token::Abs));
        assert_eq!(Token::from_keyword("SQR"), Some(Token::Sqr));
        assert_eq!(Token::from_keyword("SIN"), Some(Token::Sin));
        assert_eq!(Token::from_keyword("COS"), Some(Token::Cos));
        assert_eq!(Token::from_keyword("TAN"), Some(Token::Tan));
    }

    #[test]
    fn test_string_functions() {
        assert_eq!(Token::from_keyword("STR$"), Some(Token::StrFunc));
        assert_eq!(Token::from_keyword("CHR$"), Some(Token::ChrFunc));
        assert_eq!(Token::from_keyword("LEFT$"), Some(Token::LeftFunc));
        assert_eq!(Token::from_keyword("RIGHT$"), Some(Token::RightFunc));
        assert_eq!(Token::from_keyword("MID$"), Some(Token::MidFunc));
        assert_eq!(Token::from_keyword("LEN"), Some(Token::Len));
        assert_eq!(Token::from_keyword("VAL"), Some(Token::Val));
        assert_eq!(Token::from_keyword("ASC"), Some(Token::Asc));
    }

    #[test]
    fn test_logical_operators() {
        assert_eq!(Token::from_keyword("AND"), Some(Token::And));
        assert_eq!(Token::from_keyword("OR"), Some(Token::Or));
        assert_eq!(Token::from_keyword("NOT"), Some(Token::Not));
    }

    #[test]
    fn test_non_keyword() {
        assert_eq!(Token::from_keyword("HELLO"), None);
        assert_eq!(Token::from_keyword("A1"), None);
        assert_eq!(Token::from_keyword("XYZ"), None);
    }

    #[test]
    fn test_is_statement_keyword() {
        assert!(Token::Print.is_statement_keyword());
        assert!(Token::For.is_statement_keyword());
        assert!(Token::Goto.is_statement_keyword());
        assert!(!Token::Then.is_statement_keyword());
        assert!(!Token::Plus.is_statement_keyword());
        assert!(!Token::Number(42.0).is_statement_keyword());
    }

    #[test]
    fn test_token_equality() {
        assert_eq!(Token::Number(42.0), Token::Number(42.0));
        assert_eq!(Token::String("HELLO".to_string()), Token::String("HELLO".to_string()));
        assert_eq!(Token::Identifier("A".to_string()), Token::Identifier("A".to_string()));
        assert_ne!(Token::Number(42.0), Token::Number(43.0));
    }

    #[test]
    fn test_token_clone() {
        let token1 = Token::String("TEST".to_string());
        let token2 = token1.clone();
        assert_eq!(token1, token2);
    }
}

