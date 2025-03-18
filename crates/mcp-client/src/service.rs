//! MCP 服务实现。
//!
//! 该模块包含 MCP 服务的核心逻辑，用于处理 MCP 传输。

use futures::future::BoxFuture;
use mcp_core::protocol::JsonRpcMessage;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{timeout::Timeout, Service, ServiceBuilder};

use crate::transport::{Error, TransportHandle};

/// 一个包装服务，为 MCP 传输实现 Tower 的 Service trait。
#[derive(Clone)]
pub struct McpService<T: TransportHandle> {
    inner: Arc<T>,
}

impl<T: TransportHandle> McpService<T> {
    /// 创建一个新的 McpService。
    pub fn new(transport: T) -> Self {
        Self {
            inner: Arc::new(transport),
        }
    }
}

impl<T> Service<JsonRpcMessage> for McpService<T>
where
    T: TransportHandle + Send + Sync + 'static,
{
    type Response = JsonRpcMessage;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Most transports are always ready, but this could be customized if needed
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: JsonRpcMessage) -> Self::Future {
        let transport = self.inner.clone();
        Box::pin(async move { transport.send(request).await })
    }
}

// Add a convenience constructor for creating a service with timeout
impl<T> McpService<T>
where
    T: TransportHandle,
{
    /// 创建一个带有超时的新服务。
    pub fn with_timeout(transport: T, timeout: std::time::Duration) -> Timeout<McpService<T>> {
        ServiceBuilder::new()
            .timeout(timeout)
            .service(McpService::new(transport))
    }
}
