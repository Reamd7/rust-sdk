//! MCP 传输模块。
//!
//! 该模块包含 MCP 传输的核心逻辑，用于处理不同类型的传输方式。

use async_trait::async_trait;
use mcp_core::protocol::JsonRpcMessage;
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, RwLock};

/// 通用错误类型。
pub type BoxError = Box<dyn std::error::Error + Sync + Send>;

/// 传输操作的通用错误类型。
#[derive(Debug, Error)]
pub enum Error {
    /// I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// 传输未连接或已关闭。
    #[error("Transport was not connected or is already closed")]
    NotConnected,

    /// 通道已关闭。
    #[error("Channel closed")]
    ChannelClosed,

    /// 序列化错误。
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// 不支持的消息类型。JsonRpcMessage 只能是 Request 或 Notification。
    #[error("Unsupported message type. JsonRpcMessage can only be Request or Notification.")]
    UnsupportedMessage,

    /// Stdio 进程错误。
    #[error("Stdio process error: {0}")]
    StdioProcessError(String),

    /// SSE 连接错误。
    #[error("SSE connection error: {0}")]
    SseConnection(String),

    /// HTTP 错误。
    #[error("HTTP error: {status} - {message}")]
    HttpError { status: u16, message: String },
}

/// 可以通过传输发送的消息。
#[derive(Debug)]
pub struct TransportMessage {
    /// 要发送的 JSON-RPC 消息。
    pub message: JsonRpcMessage,
    /// 用于接收响应的通道（Notification 为 None）。
    pub response_tx: Option<oneshot::Sender<Result<JsonRpcMessage, Error>>>,
}

/// 具有基于通道的通信的通用异步传输 trait。
#[async_trait]
pub trait Transport {
    type Handle: TransportHandle;

    /// 启动传输并建立底层连接。
    /// 返回用于发送消息的传输句柄。
    async fn start(&self) -> Result<Self::Handle, Error>;

    /// 关闭传输并释放任何资源。
    async fn close(&self) -> Result<(), Error>;
}

#[async_trait]
pub trait TransportHandle: Send + Sync + Clone + 'static {
    /// 发送消息。
    async fn send(&self, message: JsonRpcMessage) -> Result<JsonRpcMessage, Error>;
}

// Helper function that contains the common send implementation
pub async fn send_message(
    sender: &mpsc::Sender<TransportMessage>,
    message: JsonRpcMessage,
) -> Result<JsonRpcMessage, Error> {
    match message {
        JsonRpcMessage::Request(request) => {
            let (respond_to, response) = oneshot::channel();
            let msg = TransportMessage {
                message: JsonRpcMessage::Request(request),
                response_tx: Some(respond_to),
            };
            sender.send(msg).await.map_err(|_| Error::ChannelClosed)?;
            Ok(response.await.map_err(|_| Error::ChannelClosed)??)
        }
        JsonRpcMessage::Notification(notification) => {
            let msg = TransportMessage {
                message: JsonRpcMessage::Notification(notification),
                response_tx: None,
            };
            sender.send(msg).await.map_err(|_| Error::ChannelClosed)?;
            Ok(JsonRpcMessage::Nil)
        }
        _ => Err(Error::UnsupportedMessage),
    }
}

// A data structure to store pending requests and their response channels
pub struct PendingRequests {
    requests: RwLock<HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>>,
}

impl Default for PendingRequests {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
        }
    }

    /// 插入一个挂起的请求。
    pub async fn insert(&self, id: String, sender: oneshot::Sender<Result<JsonRpcMessage, Error>>) {
        self.requests.write().await.insert(id, sender);
    }

    /// 响应一个挂起的请求。
    pub async fn respond(&self, id: &str, response: Result<JsonRpcMessage, Error>) {
        if let Some(tx) = self.requests.write().await.remove(id) {
            let _ = tx.send(response);
        }
    }

    /// 清除所有挂起的请求。
    pub async fn clear(&self) {
        self.requests.write().await.clear();
    }
}

pub mod stdio;
pub use stdio::StdioTransport;

pub mod sse;
pub use sse::SseTransport;
