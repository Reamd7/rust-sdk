//! 工具表示服务器可以执行的例程
//! 工具调用表示客户端执行例程的请求
use serde::{Deserialize, Serialize}; // 引入 serde 库，提供 Deserialize 和 Serialize trait，用于序列化和反序列化
use serde_json::Value; // 引入 serde_json 库，提供 Value 类型，用于处理 JSON 值

/// 模型可以使用的工具。
/// A tool that can be used by a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// 工具的名称
    /// The name of the tool
    pub name: String,
    /// 工具的功能描述
    /// A description of what the tool does
    pub description: String,
    /// 定义工具预期参数的 JSON Schema 对象
    /// A JSON Schema object defining the expected parameters for the tool
    pub input_schema: Value,
}

impl Tool {
    /// 使用给定的名称和描述创建新工具
    /// Create a new tool with the given name and description
    pub fn new<N, D>(name: N, description: D, input_schema: Value) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Tool {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// 扩展可以执行的工具调用请求
/// A tool call request that an extension can execute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    /// 要执行的工具的名称
    /// The name of the tool to execute
    pub name: String,
    /// 执行的参数
    /// The parameters for the execution
    pub arguments: Value,
}

impl ToolCall {
    /// 使用给定的名称和参数创建新的 ToolUse
    /// Create a new ToolUse with the given name and parameters
    pub fn new<S: Into<String>>(name: S, arguments: Value) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}
