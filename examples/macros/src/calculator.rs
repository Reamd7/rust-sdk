use mcp_core::handler::{ToolError, ToolHandler};
use mcp_macros::tool;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // 创建工具的实例
    // Create an instance of our tool
    let calculator = Calculator;

    // 打印工具信息
    // Print tool information
    println!("Tool name: {}", calculator.name());
    println!("Tool description: {}", calculator.description());
    println!("Tool schema: {}", calculator.schema());

    // 使用一些示例输入测试该工具
    // Test the tool with some sample input
    let input = serde_json::json!({
        "x": 5,
        "y": 3,
        "operation": "multiply"
    });

    let result = calculator.call(input).await?;
    println!("Result: {}", result);

    Ok(())
}

// 定义一个计算器工具
// Define a calculator tool
#[tool(
    name = "calculator",
    description = "Perform basic arithmetic operations",
    params(
        x = "First number in the calculation",
        y = "Second number in the calculation",
        operation = "The operation to perform (add, subtract, multiply, divide)"
    )
)]
async fn calculator(x: i32, y: i32, operation: String) -> Result<i32, ToolError> {
    match operation.as_str() {
        "add" => Ok(x + y), // 加法
        "subtract" => Ok(x - y), // 减法
        "multiply" => Ok(x * y), // 乘法
        "divide" => { // 除法
            if y == 0 {
                Err(ToolError::ExecutionError("Division by zero".into())) // 除数为零错误
            } else {
                Ok(x / y)
            }
        }
        _ => Err(ToolError::InvalidParameters(format!( // 无效参数错误
            "Unknown operation: {}",
            operation
        ))),
    }
}
