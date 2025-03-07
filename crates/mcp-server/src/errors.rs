use thiserror::Error;

// 定义一个 BoxError 类型，用于表示 trait object 类型的错误
pub type BoxError = Box<dyn std::error::Error + Sync + Send>;

// 定义 TransportError 枚举，表示传输过程中可能发生的错误
#[derive(Error, Debug)]
pub enum TransportError {
    // IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // JSON 序列化错误
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    // UTF-8 编码错误
    #[error("Invalid UTF-8 sequence: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    // 协议错误
    #[error("Protocol error: {0}")]
    Protocol(String),

    // 无效的消息格式错误
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
}

// 定义 ServerError 枚举，表示服务器可能发生的错误
#[derive(Error, Debug)]
pub enum ServerError {
    // 传输错误
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    // 服务错误
    #[error("Service error: {0}")]
    Service(String),

    // 内部错误
    #[error("Internal error: {0}")]
    Internal(String),

    // 请求超时错误
    #[error("Request timed out")]
    Timeout(#[from] tower::timeout::error::Elapsed),
}

// 定义 RouterError 枚举，表示路由过程中可能发生的错误
#[derive(Error, Debug)]
pub enum RouterError {
    // 方法未找到错误
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    // 无效的参数错误
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    // 内部错误
    #[error("Internal error: {0}")]
    Internal(String),

    // 工具未找到错误
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    // 资源未找到错误
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    // Prompt 未找到错误
    #[error("Not found: {0}")]
    PromptNotFound(String),
}

// 将 RouterError 转换为 mcp_core::protocol::ErrorData
impl From<RouterError> for mcp_core::protocol::ErrorData {
    fn from(err: RouterError) -> Self {
        use mcp_core::protocol::*;
        match err {
            RouterError::MethodNotFound(msg) => ErrorData {
                code: METHOD_NOT_FOUND,
                message: msg,
                data: None,
            },
            RouterError::InvalidParams(msg) => ErrorData {
                code: INVALID_PARAMS,
                message: msg,
                data: None,
            },
            RouterError::Internal(msg) => ErrorData {
                code: INTERNAL_ERROR,
                message: msg,
                data: None,
            },
            RouterError::ToolNotFound(msg) => ErrorData {
                code: INVALID_REQUEST,
                message: msg,
                data: None,
            },
            RouterError::ResourceNotFound(msg) => ErrorData {
                code: INVALID_REQUEST,
                message: msg,
                data: None,
            },
            RouterError::PromptNotFound(msg) => ErrorData {
                code: INVALID_REQUEST,
                message: msg,
                data: None,
            },
        }
    }
}

// 将 mcp_core::handler::ResourceError 转换为 RouterError
impl From<mcp_core::handler::ResourceError> for RouterError {
    fn from(err: mcp_core::handler::ResourceError) -> Self {
        match err {
            mcp_core::handler::ResourceError::NotFound(msg) => RouterError::ResourceNotFound(msg),
            _ => RouterError::Internal("Unknown resource error".to_string()),
        }
    }
}
