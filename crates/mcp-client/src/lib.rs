//! MCP 客户端库。
//!
//! 该库提供了用于与 MCP 服务器通信的客户端。

pub mod client;
pub mod service;
pub mod transport;

pub use client::{ClientCapabilities, ClientInfo, Error, McpClient, McpClientTrait};
pub use service::McpService;
pub use transport::{SseTransport, StdioTransport, Transport, TransportHandle};
