/// 抽象语法树（AST）数据结构
///
/// 定义 BASIC 程序的语法元素

/// 表达式节点
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // 字面量
    Number(f64),
    String(String),
    Variable(String),
    
    // 数组访问
    ArrayAccess {
        name: String,
        indices: Vec<Expr>,
    },
    
    // 函数调用
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },
    
    // 二元运算
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    
    // 一元运算
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expr>,
    },
}

/// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // 算术
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    
    // 关系
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    
    // 逻辑
    And,
    Or,
}

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Minus,
    Not,
}

/// 语句
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // LET 赋值
    Let {
        target: AssignTarget,
        value: Expr,
    },
    
    // PRINT 输出
    Print {
        items: Vec<PrintItem>,
    },
    
    // IF 条件
    If {
        condition: Expr,
        then_part: Box<ThenPart>,
    },
    
    // GOTO 跳转
    Goto {
        line_number: Expr,
    },
    
    // GOSUB 子程序调用
    Gosub {
        line_number: Expr,
    },
    
    // RETURN 返回
    Return,
    
    // FOR 循环
    For {
        var: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
    },
    
    // NEXT 循环结束
    Next {
        var: Option<String>,
    },
    
    // ON...GOTO 或 ON...GOSUB
    On {
        expr: Expr,
        targets: Vec<u16>,
        is_gosub: bool,
    },
    
    // INPUT 输入
    Input {
        prompt: Option<String>,
        variables: Vec<AssignTarget>,
    },
    
    // DIM 数组声明
    Dim {
        arrays: Vec<ArrayDim>,
    },
    
    // DATA 数据
    Data {
        values: Vec<DataValue>,
    },
    
    // READ 读取数据
    Read {
        variables: Vec<AssignTarget>,
    },
    
    // RESTORE 重置数据指针
    Restore {
        line_number: Option<u16>,
    },
    
    // DEF FN 用户函数定义
    DefFn {
        name: String,
        param: String,
        body: Expr,
    },
    
    // REM 注释
    Rem {
        comment: String,
    },
    
    // END 结束
    End,
    
    // STOP 暂停
    Stop,
    
    // NEW 清空程序
    New,
    
    // CLEAR 清空变量
    Clear,
    
    // LIST 列出程序
    List {
        start: Option<u16>,
        end: Option<u16>,
    },
    
    // RUN 运行程序
    Run {
        line_number: Option<u16>,
    },
    
    // CONT 继续执行
    Cont,
    
    // POKE 内存写入
    Poke {
        address: Expr,
        value: Expr,
    },
    
    // WAIT 等待
    Wait {
        address: Expr,
        mask: Expr,
        value: Option<Expr>,
    },
    
    // GET 读取单字符
    Get {
        variable: String,
    },
    
    // NULL 空语句
    Null,
    
    // LOAD 加载程序
    Load {
        filename: String,
    },
    
    // SAVE 保存程序
    Save {
        filename: String,
    },
}

/// THEN 部分（行号或语句）
#[derive(Debug, Clone, PartialEq)]
pub enum ThenPart {
    LineNumber(u16),
    Statement(Statement),
    Statements(Vec<Statement>), // 支持 THEN 后跟多条语句
}

/// 赋值目标（变量或数组元素）
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Variable(String),
    ArrayElement {
        name: String,
        indices: Vec<Expr>,
    },
}

/// PRINT 语句的项
#[derive(Debug, Clone, PartialEq)]
pub enum PrintItem {
    Expr(Expr),
    Tab(Expr),       // TAB(x)
    Spc(Expr),       // SPC(x)
    Comma,           // 列分隔
    Semicolon,       // 紧密连接
}

/// 数组维度声明
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayDim {
    pub name: String,
    pub dimensions: Vec<Expr>,
}

/// DATA 语句的值
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    Number(f64),
    String(String),
}

/// 程序行
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramLine {
    pub line_number: u16,
    pub statements: Vec<Statement>,
}

impl Expr {
    /// 创建数值字面量
    pub fn number(val: f64) -> Self {
        Expr::Number(val)
    }
    
    /// 创建字符串字面量
    pub fn string(val: String) -> Self {
        Expr::String(val)
    }
    
    /// 创建变量引用
    pub fn variable(name: String) -> Self {
        Expr::Variable(name)
    }
    
    /// 创建二元运算
    pub fn binary(left: Expr, op: BinaryOperator, right: Expr) -> Self {
        Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }
    
    /// 创建一元运算
    pub fn unary(op: UnaryOperator, operand: Expr) -> Self {
        Expr::UnaryOp {
            op,
            operand: Box::new(operand),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_number() {
        let expr = Expr::number(42.0);
        assert_eq!(expr, Expr::Number(42.0));
    }

    #[test]
    fn test_expr_variable() {
        let expr = Expr::variable("A".to_string());
        assert_eq!(expr, Expr::Variable("A".to_string()));
    }

    #[test]
    fn test_expr_binary() {
        let left = Expr::number(2.0);
        let right = Expr::number(3.0);
        let expr = Expr::binary(left, BinaryOperator::Add, right);
        
        match expr {
            Expr::BinaryOp { left, op, right } => {
                assert_eq!(*left, Expr::Number(2.0));
                assert_eq!(op, BinaryOperator::Add);
                assert_eq!(*right, Expr::Number(3.0));
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_expr_unary() {
        let operand = Expr::number(5.0);
        let expr = Expr::unary(UnaryOperator::Minus, operand);
        
        match expr {
            Expr::UnaryOp { op, operand } => {
                assert_eq!(op, UnaryOperator::Minus);
                assert_eq!(*operand, Expr::Number(5.0));
            }
            _ => panic!("Expected UnaryOp"),
        }
    }

    #[test]
    fn test_statement_let() {
        let stmt = Statement::Let {
            target: AssignTarget::Variable("A".to_string()),
            value: Expr::number(10.0),
        };
        
        match stmt {
            Statement::Let { target, value } => {
                assert_eq!(target, AssignTarget::Variable("A".to_string()));
                assert_eq!(value, Expr::Number(10.0));
            }
            _ => panic!("Expected Let statement"),
        }
    }

    #[test]
    fn test_statement_print() {
        let stmt = Statement::Print {
            items: vec![
                PrintItem::Expr(Expr::string("HELLO".to_string())),
            ],
        };
        
        match stmt {
            Statement::Print { items } => {
                assert_eq!(items.len(), 1);
            }
            _ => panic!("Expected Print statement"),
        }
    }

    #[test]
    fn test_statement_if_with_line_number() {
        let stmt = Statement::If {
            condition: Expr::binary(
                Expr::variable("A".to_string()),
                BinaryOperator::Greater,
                Expr::number(10.0),
            ),
            then_part: Box::new(ThenPart::LineNumber(100)),
        };
        
        match stmt {
            Statement::If { condition: _, then_part } => {
                assert_eq!(*then_part, ThenPart::LineNumber(100));
            }
            _ => panic!("Expected If statement"),
        }
    }

    #[test]
    fn test_statement_for() {
        let stmt = Statement::For {
            var: "I".to_string(),
            start: Expr::number(1.0),
            end: Expr::number(10.0),
            step: Some(Expr::number(1.0)),
        };
        
        match stmt {
            Statement::For { var, start, end, step } => {
                assert_eq!(var, "I");
                assert_eq!(start, Expr::Number(1.0));
                assert_eq!(end, Expr::Number(10.0));
                assert_eq!(step, Some(Expr::Number(1.0)));
            }
            _ => panic!("Expected For statement"),
        }
    }

    #[test]
    fn test_program_line() {
        let line = ProgramLine {
            line_number: 10,
            statements: vec![
                Statement::Let {
                    target: AssignTarget::Variable("A".to_string()),
                    value: Expr::number(5.0),
                },
            ],
        };
        
        assert_eq!(line.line_number, 10);
        assert_eq!(line.statements.len(), 1);
    }

    #[test]
    fn test_array_access() {
        let expr = Expr::ArrayAccess {
            name: "A".to_string(),
            indices: vec![Expr::number(5.0)],
        };
        
        match expr {
            Expr::ArrayAccess { name, indices } => {
                assert_eq!(name, "A");
                assert_eq!(indices.len(), 1);
            }
            _ => panic!("Expected ArrayAccess"),
        }
    }

    #[test]
    fn test_function_call() {
        let expr = Expr::FunctionCall {
            name: "SIN".to_string(),
            args: vec![Expr::variable("X".to_string())],
        };
        
        match expr {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "SIN");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected FunctionCall"),
        }
    }
}

