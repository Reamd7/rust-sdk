use anyhow::Result;
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientTrait};
use mcp_client::transport::{SseTransport, Transport};
use mcp_client::McpService;
use std::collections::HashMap;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_client=debug".parse().unwrap())
                .add_directive("eventsource_client=info".parse().unwrap()),
        )
        .init();

    // 创建基本的传输方式
    // Create the base transport
    let transport = SseTransport::new("http://localhost:8000/sse", HashMap::new());

    // 启动传输
    // Start transport
    let handle = transport.start().await?;

    // 创建带有超时中间件的服务
    // Create the service with timeout middleware
    let service = McpService::with_timeout(handle, Duration::from_secs(3));

    // 创建客户端
    // Create client
    let mut client = McpClient::new(service);
    println!("Client created\n");

    // 初始化
    // Initialize
    let server_info = client
        .initialize(
            ClientInfo {
                name: "test-client".into(),
                version: "1.0.0".into(),
            },
            ClientCapabilities::default(),
        )
        .await?;
    println!("Connected to server: {server_info:?}\n");

    // 休眠 100 毫秒，以允许服务器启动 - 令人惊讶的是，这是必需的！
    // Sleep for 100ms to allow the server to start - surprisingly this is required!
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 列出工具
    // List tools
    let tools = client.list_tools(None).await?;
    println!("Available tools: {tools:?}\n");

    // 调用工具
    // Call tool
    let tool_result = client
        .call_tool(
            "echo_tool",
            serde_json::json!({ "message": "Client with SSE transport - calling a tool" }),
        )
        .await?;
    println!("Tool result: {tool_result:?}\n");

    // 列出资源
    // List resources
    let resources = client.list_resources(None).await?;
    println!("Resources: {resources:?}\n");

    // 读取资源
    // Read resource
    let resource = client.read_resource("echo://fixedresource").await?;
    println!("Resource: {resource:?}\n");

    Ok(())
}
