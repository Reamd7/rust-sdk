use async_trait::async_trait; // 引入 async_trait 库，用于定义异步 trait
use schemars::JsonSchema; // 引入 schemars 库，用于生成 JSON schema
use serde::{Deserialize, Serialize}; // 引入 serde 库，提供 Deserialize 和 Serialize trait，用于序列化和反序列化
use serde_json::Value; // 引入 serde_json 库，提供 Value 类型，用于处理 JSON 值
use thiserror::Error; // 引入 thiserror 库，用于简化错误类型的定义

/// 工具错误
#[non_exhaustive]
#[derive(Error, Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ToolError {
    /// 无效的参数
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    /// 执行失败
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    /// Schema 错误
    #[error("Schema error: {0}")]
    SchemaError(String),
    /// 工具未找到
    #[error("Tool not found: {0}")]
    NotFound(String),
}

/// 工具结果类型
pub type ToolResult<T> = std::result::Result<T, ToolError>;

/// 资源错误
#[derive(Error, Debug)]
pub enum ResourceError {
    /// 执行失败
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    /// 资源未找到
    #[error("Resource not found: {0}")]
    NotFound(String),
}

/// Prompt 错误
#[derive(Error, Debug)]
pub enum PromptError {
    /// 无效的参数
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    /// 内部错误
    #[error("Internal error: {0}")]
    InternalError(String),
    /// Prompt 未找到
    #[error("Prompt not found: {0}")]
    NotFound(String),
}

/// 用于实现 MCP 工具的 trait
#[async_trait]
pub trait ToolHandler: Send + Sync + 'static {
    /// 工具的名称
    fn name(&self) -> &'static str;

    /// 工具的功能描述
    fn description(&self) -> &'static str;

    /// 描述工具参数的 JSON schema
    fn schema(&self) -> Value;

    /// 使用给定的参数执行工具
    async fn call(&self, params: Value) -> ToolResult<Value>;
}

/// 用于实现 MCP 资源的 trait
#[async_trait]
pub trait ResourceTemplateHandler: Send + Sync + 'static {
    /// 此资源的 URL 模板
    fn template() -> &'static str;

    /// 描述资源参数的 JSON schema
    fn schema() -> Value;

    /// 获取资源值
    async fn get(&self, params: Value) -> ToolResult<String>;
}

/// 用于为类型生成 JSON schema 的辅助函数
pub fn generate_schema<T: JsonSchema>() -> ToolResult<Value> {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(schema).map_err(|e| ToolError::SchemaError(e.to_string()))
}
