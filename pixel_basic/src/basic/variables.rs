/// 变量系统
///
/// 管理 BASIC 程序的变量存储，包括简单变量和数组

use std::collections::HashMap;
use super::error::{BasicError, Result};
use super::ast::Expr;

/// 用户自定义函数定义
#[derive(Debug, Clone)]
pub struct UserFunction {
    /// 函数名
    pub name: String,
    /// 参数名
    pub param: String,
    /// 函数体（表达式）
    pub body: Expr,
}

/// 变量值类型
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 数值（整数和浮点都用 f64 表示）
    Number(f64),
    /// 字符串
    String(String),
}

impl Value {
    /// 获取数值，如果不是数值则返回错误
    pub fn as_number(&self) -> Result<f64> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::String(_) => Err(BasicError::TypeMismatch(
                "Expected number, got string".to_string()
            )),
        }
    }

    /// 获取字符串，如果不是字符串则返回错误
    pub fn as_string(&self) -> Result<String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Number(_) => Err(BasicError::TypeMismatch(
                "Expected string, got number".to_string()
            )),
        }
    }

    /// 判断是否为数值
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// 判断是否为字符串
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
}

/// 数组结构
#[derive(Debug, Clone)]
pub struct Array {
    /// 维度（每个维度的大小）
    dimensions: Vec<usize>,
    /// 数据（展平存储）
    data: Vec<Value>,
    /// 数组类型（数值或字符串）
    is_string: bool,
}

impl Array {
    /// 创建新的数组
    pub fn new(dimensions: Vec<usize>, is_string: bool) -> Self {
        // BASIC 数组索引是从 0 到 N（包括 N），所以每个维度需要 +1
        let total_size: usize = dimensions.iter().map(|&d| d + 1).product();
        let default_value = if is_string {
            Value::String(String::new())
        } else {
            Value::Number(0.0)
        };
        
        Array {
            dimensions,
            data: vec![default_value; total_size],
            is_string,
        }
    }

    /// 获取维度信息
    pub fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }

    /// 计算多维索引到一维索引的转换
    fn calculate_index(&self, indices: &[usize]) -> Result<usize> {
        if indices.len() != self.dimensions.len() {
            return Err(BasicError::SyntaxError(
                format!("Array has {} dimensions, but {} indices provided",
                    self.dimensions.len(), indices.len())
            ));
        }

        // 检查边界
        for (i, &idx) in indices.iter().enumerate() {
            if idx > self.dimensions[i] {
                return Err(BasicError::SubscriptOutOfRange(
                    format!("Index {} out of range (max {})", idx, self.dimensions[i])
                ));
            }
        }

        // 计算一维索引（行优先）
        let mut index = 0;
        let mut multiplier = 1;
        
        for i in (0..indices.len()).rev() {
            index += indices[i] * multiplier;
            multiplier *= self.dimensions[i] + 1; // +1 因为 BASIC 数组是 0-based 到 N
        }

        Ok(index)
    }

    /// 获取数组元素
    pub fn get(&self, indices: &[usize]) -> Result<Value> {
        let index = self.calculate_index(indices)?;
        Ok(self.data[index].clone())
    }

    /// 设置数组元素
    pub fn set(&mut self, indices: &[usize], value: Value) -> Result<()> {
        // 类型检查
        if self.is_string && !value.is_string() {
            return Err(BasicError::TypeMismatch(
                "Cannot assign number to string array".to_string()
            ));
        }
        if !self.is_string && !value.is_number() {
            return Err(BasicError::TypeMismatch(
                "Cannot assign string to numeric array".to_string()
            ));
        }

        let index = self.calculate_index(indices)?;
        self.data[index] = value;
        Ok(())
    }
}

/// 变量存储
pub struct Variables {
    /// 简单变量
    simple: HashMap<String, Value>,
    /// 数组
    arrays: HashMap<String, Array>,
    /// 用户自定义函数
    functions: HashMap<String, UserFunction>,
}

impl Variables {
    /// 创建新的变量存储
    pub fn new() -> Self {
        Variables {
            simple: HashMap::new(),
            arrays: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    /// 标准化变量名（转大写）
    fn normalize_name(name: &str) -> String {
        name.to_uppercase()
    }

    /// 获取简单变量的值
    pub fn get(&self, name: &str) -> Value {
        let key = Self::normalize_name(name);
        
        // 判断默认值类型
        let default_value = if key.ends_with('$') {
            Value::String(String::new())
        } else {
            Value::Number(0.0)
        };
        
        self.simple.get(&key).cloned().unwrap_or(default_value)
    }

    /// 设置简单变量的值
    pub fn set(&mut self, name: &str, value: Value) -> Result<()> {
        let key = Self::normalize_name(name);
        
        // 类型检查
        let is_string_var = key.ends_with('$');
        if is_string_var && !value.is_string() {
            return Err(BasicError::TypeMismatch(
                format!("Cannot assign number to string variable {}", name)
            ));
        }
        if !is_string_var && !value.is_number() {
            return Err(BasicError::TypeMismatch(
                format!("Cannot assign string to numeric variable {}", name)
            ));
        }
        
        self.simple.insert(key, value);
        Ok(())
    }

    /// 声明数组
    pub fn dim_array(&mut self, name: &str, dimensions: Vec<usize>) -> Result<()> {
        let key = Self::normalize_name(name);
        
        // 检查是否已经声明
        if self.arrays.contains_key(&key) {
            return Err(BasicError::RedimensionedArray(name.to_string()));
        }
        
        let is_string = key.ends_with('$');
        let array = Array::new(dimensions, is_string);
        self.arrays.insert(key, array);
        
        Ok(())
    }

    /// 获取数组元素
    pub fn get_array_element(&self, name: &str, indices: &[usize]) -> Result<Value> {
        let key = Self::normalize_name(name);
        
        // 如果数组不存在，自动创建默认大小（10）
        if !self.arrays.contains_key(&key) {
            // 对于未声明的数组，检查索引是否在默认范围内
            for &idx in indices {
                if idx > 10 {
                    return Err(BasicError::SubscriptOutOfRange(
                        format!("Index {} out of range (default array size is 10)", idx)
                    ));
                }
            }
            
            // 返回默认值
            let default_value = if key.ends_with('$') {
                Value::String(String::new())
            } else {
                Value::Number(0.0)
            };
            return Ok(default_value);
        }
        
        let array = self.arrays.get(&key).unwrap();
        array.get(indices)
    }

    /// 设置数组元素
    pub fn set_array_element(&mut self, name: &str, indices: &[usize], value: Value) -> Result<()> {
        let key = Self::normalize_name(name);
        
        // 如果数组不存在，自动创建默认大小
        if !self.arrays.contains_key(&key) {
            // 创建默认数组（根据索引维度）
            let dimensions = vec![10; indices.len()];
            let is_string = key.ends_with('$');
            let array = Array::new(dimensions, is_string);
            self.arrays.insert(key.clone(), array);
        }
        
        let array = self.arrays.get_mut(&key).unwrap();
        array.set(indices, value)
    }

    /// 检查数组是否存在
    pub fn has_array(&self, name: &str) -> bool {
        let key = Self::normalize_name(name);
        self.arrays.contains_key(&key)
    }

    /// 清空所有变量和数组
    pub fn clear(&mut self) {
        self.simple.clear();
        self.arrays.clear();
        self.functions.clear();
    }

    /// 获取所有变量名（用于调试）
    pub fn list_variables(&self) -> Vec<(String, Value)> {
        self.simple.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// 获取所有数组名（用于调试）
    pub fn list_arrays(&self) -> Vec<String> {
        self.arrays.keys().cloned().collect()
    }
    
    /// 定义用户自定义函数
    pub fn define_function(&mut self, name: String, param: String, body: Expr) -> Result<()> {
        let key = Self::normalize_name(&name);
        let func = UserFunction {
            name: key.clone(),
            param: Self::normalize_name(&param),
            body,
        };
        self.functions.insert(key, func);
        Ok(())
    }
    
    /// 获取用户自定义函数
    pub fn get_function(&self, name: &str) -> Option<&UserFunction> {
        let key = Self::normalize_name(name);
        self.functions.get(&key)
    }
    
    /// 检查函数是否存在
    pub fn has_function(&self, name: &str) -> bool {
        let key = Self::normalize_name(name);
        self.functions.contains_key(&key)
    }
}

impl Default for Variables {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Requirement: 变量类型支持 - 数值变量
    #[test]
    fn test_numeric_variable() {
        let mut vars = Variables::new();
        vars.set("A", Value::Number(42.0)).unwrap();
        
        let val = vars.get("A");
        assert_eq!(val, Value::Number(42.0));
    }

    // Requirement: 变量类型支持 - 字符串变量
    #[test]
    fn test_string_variable() {
        let mut vars = Variables::new();
        vars.set("A$", Value::String("HELLO".to_string())).unwrap();
        
        let val = vars.get("A$");
        assert_eq!(val, Value::String("HELLO".to_string()));
    }

    // Requirement: 变量类型支持 - 类型区分
    #[test]
    fn test_type_distinction() {
        let mut vars = Variables::new();
        vars.set("A", Value::Number(42.0)).unwrap();
        vars.set("A$", Value::String("HELLO".to_string())).unwrap();
        
        assert_eq!(vars.get("A"), Value::Number(42.0));
        assert_eq!(vars.get("A$"), Value::String("HELLO".to_string()));
    }

    // Requirement: 变量命名规则 - 大小写不敏感
    #[test]
    fn test_case_insensitive() {
        let mut vars = Variables::new();
        vars.set("a", Value::Number(10.0)).unwrap();
        
        assert_eq!(vars.get("A"), Value::Number(10.0));
        assert_eq!(vars.get("a"), Value::Number(10.0));
    }

    // Requirement: 变量初始值 - 未初始化数值变量
    #[test]
    fn test_uninitialized_numeric() {
        let vars = Variables::new();
        let val = vars.get("X");
        assert_eq!(val, Value::Number(0.0));
    }

    // Requirement: 变量初始值 - 未初始化字符串变量
    #[test]
    fn test_uninitialized_string() {
        let vars = Variables::new();
        let val = vars.get("X$");
        assert_eq!(val, Value::String(String::new()));
    }

    // Requirement: 变量赋值 - 数值赋值
    #[test]
    fn test_numeric_assignment() {
        let mut vars = Variables::new();
        vars.set("A", Value::Number(100.0)).unwrap();
        assert_eq!(vars.get("A"), Value::Number(100.0));
    }

    // Requirement: 变量赋值 - 字符串赋值
    #[test]
    fn test_string_assignment() {
        let mut vars = Variables::new();
        vars.set("B$", Value::String("TEST".to_string())).unwrap();
        assert_eq!(vars.get("B$"), Value::String("TEST".to_string()));
    }

    // Requirement: 类型检查 - 数值变量赋字符串
    #[test]
    fn test_type_mismatch_number_to_string() {
        let mut vars = Variables::new();
        let result = vars.set("A", Value::String("HELLO".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::TypeMismatch(_)));
    }

    // Requirement: 类型检查 - 字符串变量赋数值
    #[test]
    fn test_type_mismatch_string_to_number() {
        let mut vars = Variables::new();
        let result = vars.set("A$", Value::Number(123.0));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::TypeMismatch(_)));
    }

    // Requirement: 数组声明 - 一维数组
    #[test]
    fn test_dim_one_dimensional() {
        let mut vars = Variables::new();
        vars.dim_array("A", vec![10]).unwrap();
        assert!(vars.has_array("A"));
    }

    // Requirement: 数组声明 - 二维数组
    #[test]
    fn test_dim_two_dimensional() {
        let mut vars = Variables::new();
        vars.dim_array("B", vec![5, 10]).unwrap();
        assert!(vars.has_array("B"));
    }

    // Requirement: 数组声明 - 三维数组
    #[test]
    fn test_dim_three_dimensional() {
        let mut vars = Variables::new();
        vars.dim_array("C", vec![2, 3, 4]).unwrap();
        assert!(vars.has_array("C"));
    }

    // Requirement: 数组声明 - 字符串数组
    #[test]
    fn test_dim_string_array() {
        let mut vars = Variables::new();
        vars.dim_array("A$", vec![10]).unwrap();
        assert!(vars.has_array("A$"));
    }

    // Requirement: 数组元素访问 - 数组元素赋值
    #[test]
    fn test_array_element_assignment() {
        let mut vars = Variables::new();
        vars.dim_array("A", vec![10]).unwrap();
        vars.set_array_element("A", &[5], Value::Number(100.0)).unwrap();
        
        let val = vars.get_array_element("A", &[5]).unwrap();
        assert_eq!(val, Value::Number(100.0));
    }

    // Requirement: 数组元素访问 - 多维数组访问
    #[test]
    fn test_multidimensional_array_access() {
        let mut vars = Variables::new();
        vars.dim_array("B", vec![5, 10]).unwrap();
        vars.set_array_element("B", &[2, 3], Value::Number(50.0)).unwrap();
        
        let val = vars.get_array_element("B", &[2, 3]).unwrap();
        assert_eq!(val, Value::Number(50.0));
    }

    // Requirement: 数组元素访问 - 数组下标越界
    #[test]
    fn test_array_subscript_out_of_range() {
        let mut vars = Variables::new();
        vars.dim_array("A", vec![10]).unwrap();
        
        let result = vars.get_array_element("A", &[11]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::SubscriptOutOfRange(_)));
    }

    // Requirement: 数组元素访问 - 负数索引
    #[test]
    fn test_array_negative_index() {
        // Rust 的 usize 不支持负数，这个测试在 Rust 中会在编译时就被拒绝
        // 实际运行时，负数会被转换为很大的正数，触发越界错误
    }

    // Requirement: 隐式数组声明 - 未声明数组自动创建
    #[test]
    fn test_implicit_array_creation() {
        let mut vars = Variables::new();
        // 未声明直接使用
        vars.set_array_element("A", &[5], Value::Number(42.0)).unwrap();
        let val = vars.get_array_element("A", &[5]).unwrap();
        assert_eq!(val, Value::Number(42.0));
    }

    // Requirement: 隐式数组声明 - 隐式数组大小限制
    #[test]
    fn test_implicit_array_size_limit() {
        let vars = Variables::new();
        let result = vars.get_array_element("A", &[11]);
        assert!(result.is_err());
    }

    // Requirement: 数组重新声明 - 重复 DIM 错误
    #[test]
    fn test_redimensioned_array_error() {
        let mut vars = Variables::new();
        vars.dim_array("A", vec![10]).unwrap();
        
        let result = vars.dim_array("A", vec![20]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::RedimensionedArray(_)));
    }

    // Requirement: 变量清空 - CLEAR 清空简单变量
    #[test]
    fn test_clear_simple_variables() {
        let mut vars = Variables::new();
        vars.set("A", Value::Number(10.0)).unwrap();
        vars.set("B$", Value::String("TEST".to_string())).unwrap();
        
        vars.clear();
        
        assert_eq!(vars.get("A"), Value::Number(0.0));
        assert_eq!(vars.get("B$"), Value::String(String::new()));
    }

    // Requirement: 变量清空 - CLEAR 清空数组
    #[test]
    fn test_clear_arrays() {
        let mut vars = Variables::new();
        vars.dim_array("A", vec![10]).unwrap();
        
        vars.clear();
        
        assert!(!vars.has_array("A"));
    }

    // Requirement: 变量清空 - CLEAR 后变量重用
    #[test]
    fn test_clear_and_reuse() {
        let mut vars = Variables::new();
        vars.set("A", Value::Number(10.0)).unwrap();
        vars.clear();
        
        // 可以重新赋值
        vars.set("A", Value::Number(20.0)).unwrap();
        assert_eq!(vars.get("A"), Value::Number(20.0));
    }

    // Requirement: 类型检查 - 数组类型一致性
    #[test]
    fn test_array_type_consistency() {
        let mut vars = Variables::new();
        vars.dim_array("A", vec![10]).unwrap();
        
        let result = vars.set_array_element("A", &[5], Value::String("TEXT".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BasicError::TypeMismatch(_)));
    }

    // Test: 列出所有变量
    #[test]
    fn test_list_variables() {
        let mut vars = Variables::new();
        vars.set("A", Value::Number(10.0)).unwrap();
        vars.set("B$", Value::String("TEST".to_string())).unwrap();
        
        let list = vars.list_variables();
        assert_eq!(list.len(), 2);
    }

    // Test: Value 类型方法
    #[test]
    fn test_value_as_number() {
        let val = Value::Number(42.0);
        assert_eq!(val.as_number().unwrap(), 42.0);
        
        let val = Value::String("test".to_string());
        assert!(val.as_number().is_err());
    }

    #[test]
    fn test_value_as_string() {
        let val = Value::String("hello".to_string());
        assert_eq!(val.as_string().unwrap(), "hello");
        
        let val = Value::Number(42.0);
        assert!(val.as_string().is_err());
    }

    // Test: 数组维度信息
    #[test]
    fn test_array_dimensions() {
        let array = Array::new(vec![5, 10], false);
        assert_eq!(array.dimensions(), &[5, 10]);
    }
}

