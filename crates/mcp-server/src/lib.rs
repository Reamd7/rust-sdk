use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream};
use mcp_core::protocol::{JsonRpcError, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse};
use pin_project::pin_project;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tower_service::Service;

// 引入 errors 模块
mod errors;
// 公开 errors 模块中的类型
pub use errors::{BoxError, RouterError, ServerError, TransportError};

// 引入 router 模块
pub mod router;
// 公开 router 模块
pub use router::Router;

/// ByteTransport 结构体，用于处理基于字节流的 JSON-RPC 消息
#[pin_project]
pub struct ByteTransport<R, W> {
    // reader 是一个 BufReader，它在底层流（stdin 或类似）上进行缓冲
    // 在每次 poll 调用中，我们从这个缓冲区中清除一行 (\n)
    #[pin]
    reader: BufReader<R>,
    #[pin]
    writer: W,
}

impl<R, W> ByteTransport<R, W>
where
    R: AsyncRead,
    W: AsyncWrite,
{
    // 创建一个新的 ByteTransport 实例
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            // 默认 BufReader 容量是 8 * 1024，增加到 2MB，即文件大小限制
            // 允许缓冲区具有读取非常大的调用的能力
            reader: BufReader::with_capacity(2 * 1024 * 1024, reader),
            writer,
        }
    }
}

// 为 ByteTransport 实现 Stream trait
impl<R, W> Stream for ByteTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    // 定义 Stream 的 Item 类型为 Result<JsonRpcMessage, TransportError>
    type Item = Result<JsonRpcMessage, TransportError>;

    // 实现 poll_next 方法，用于从流中获取下一个 Item
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let mut buf = Vec::new();

        let mut reader = this.reader.as_mut();
        let mut read_future = Box::pin(reader.read_until(b'\n', &mut buf));
        match read_future.as_mut().poll(cx) {
            Poll::Ready(Ok(0)) => Poll::Ready(None), // EOF
            Poll::Ready(Ok(_)) => {
                // 转换为 UTF-8 字符串
                let line = match String::from_utf8(buf) {
                    Ok(s) => s,
                    Err(e) => return Poll::Ready(Some(Err(TransportError::Utf8(e)))),
                };
                // 在 serde 转换之前在此处记录传入消息
                // 跟踪不是有效 JSON 的不完整块
                tracing::info!(json = %line, "incoming message");

                // 解析 JSON 并验证消息格式
                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(value) => {
                        // 验证基本 JSON-RPC 结构
                        if !value.is_object() {
                            return Poll::Ready(Some(Err(TransportError::InvalidMessage(
                                "Message must be a JSON object".into(),
                            ))));
                        }
                        let obj = value.as_object().unwrap(); // Safe due to check above

                        // 检查 jsonrpc 版本字段
                        if !obj.contains_key("jsonrpc") || obj["jsonrpc"] != "2.0" {
                            return Poll::Ready(Some(Err(TransportError::InvalidMessage(
                                "Missing or invalid jsonrpc version".into(),
                            ))));
                        }

                        // 现在尝试解析为正确的消息
                        match serde_json::from_value::<JsonRpcMessage>(value) {
                            Ok(msg) => Poll::Ready(Some(Ok(msg))),
                            Err(e) => Poll::Ready(Some(Err(TransportError::Json(e)))),
                        }
                    }
                    Err(e) => Poll::Ready(Some(Err(TransportError::Json(e)))),
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(TransportError::Io(e)))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R, W> ByteTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    // 写入消息
    pub async fn write_message(
        self: &mut Pin<&mut Self>,
        msg: JsonRpcMessage,
    ) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(&msg)?;

        let mut this = self.as_mut().project();
        this.writer.write_all(json.as_bytes()).await?;
        this.writer.write_all(b"\n").await?;
        this.writer.flush().await?;

        Ok(())
    }
}

/// Server 结构体，用于处理传入的请求
pub struct Server<S> {
    service: S,
}

impl<S> Server<S>
where
    S: Service<JsonRpcRequest, Response = JsonRpcResponse> + Send,
    S::Error: Into<BoxError>,
    S::Future: Send,
{
    // 创建一个新的 Server 实例
    pub fn new(service: S) -> Self {
        Self { service }
    }

    // 运行服务器
    // TODO transport trait instead of byte transport if we implement others
    pub async fn run<R, W>(self, mut transport: ByteTransport<R, W>) -> Result<(), ServerError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        use futures::StreamExt;
        let mut service = self.service;
        let mut transport = Pin::new(&mut transport);

        tracing::info!("Server started");
        while let Some(msg_result) = transport.next().await {
            let _span = tracing::span!(tracing::Level::INFO, "message_processing");
            let _enter = _span.enter();
            match msg_result {
                Ok(msg) => {
                    match msg {
                        JsonRpcMessage::Request(request) => {
                            // 序列化请求以进行日志记录
                            let id = request.id;
                            let request_json = serde_json::to_string(&request)
                                .unwrap_or_else(|_| "Failed to serialize request".to_string());

                            tracing::info!(
                                request_id = ?id,
                                method = ?request.method,
                                json = %request_json,
                                "Received request"
                            );

                            // 使用我们的服务处理请求
                            let response = match service.call(request).await {
                                Ok(resp) => resp,
                                Err(e) => {
                                    let error_msg = e.into().to_string();
                                    tracing::error!(error = %error_msg, "Request processing failed");
                                    JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id,
                                        result: None,
                                        error: Some(mcp_core::protocol::ErrorData {
                                            code: mcp_core::protocol::INTERNAL_ERROR,
                                            message: error_msg,
                                            data: None,
                                        }),
                                    }
                                }
                            };

                            // 序列化响应以进行日志记录
                            let response_json = serde_json::to_string(&response)
                                .unwrap_or_else(|_| "Failed to serialize response".to_string());

                            tracing::info!(
                                response_id = ?response.id,
                                json = %response_json,
                                "Sending response"
                            );
                            // 发送响应
                            if let Err(e) = transport
                                .write_message(JsonRpcMessage::Response(response))
                                .await
                            {
                                return Err(ServerError::Transport(TransportError::Io(e)));
                            }
                        }
                        JsonRpcMessage::Response(_)
                        | JsonRpcMessage::Notification(_)
                        | JsonRpcMessage::Nil
                        | JsonRpcMessage::Error(_) => {
                            // 暂时忽略响应、通知和 nil 消息
                            continue;
                        }
                    }
                }
                Err(e) => {
                    // 将传输错误转换为 JSON-RPC 错误响应
                    let error = match e {
                        TransportError::Json(_) | TransportError::InvalidMessage(_) => {
                            mcp_core::protocol::ErrorData {
                                code: mcp_core::protocol::PARSE_ERROR,
                                message: e.to_string(),
                                data: None,
                            }
                        }
                        TransportError::Protocol(_) => mcp_core::protocol::ErrorData {
                            code: mcp_core::protocol::INVALID_REQUEST,
                            message: e.to_string(),
                            data: None,
                        },
                        _ => mcp_core::protocol::ErrorData {
                            code: mcp_core::protocol::INTERNAL_ERROR,
                            message: e.to_string(),
                            data: None,
                        },
                    };

                    let error_response = JsonRpcMessage::Error(JsonRpcError {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        error,
                    });

                    if let Err(e) = transport.write_message(error_response).await {
                        return Err(ServerError::Transport(TransportError::Io(e)));
                    }
                }
            }
        }

        Ok(())
    }
}

// 定义一个特定的服务实现，我们需要任何
// 任何路由器都实现这个
pub trait BoundedService:
    Service<
        JsonRpcRequest,
        Response = JsonRpcResponse,
        Error = BoxError,
        Future = Pin<Box<dyn Future<Output = Result<JsonRpcResponse, BoxError>> + Send>>,
    > + Send
    + 'static
{
}

// 为满足边界的任何类型实现它
impl<T> BoundedService for T where
    T: Service<
            JsonRpcRequest,
            Response = JsonRpcResponse,
            Error = BoxError,
            Future = Pin<Box<dyn Future<Output = Result<JsonRpcResponse, BoxError>> + Send>>,
        > + Send
        + 'static
{
}
