use anyhow::Result;
use mcp_server::router::RouterService;
use mcp_server::{ByteTransport, Server};
use tokio::io::{stdin, stdout};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

mod common;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file appender for logging
    // 设置文件追加器用于日志记录
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "mcp-server.log");

    // Initialize the tracing subscriber with file and stdout logging
    // 使用文件和标准输出日志记录初始化 tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting MCP server"); // 启动 MCP 服务器

    // Create an instance of our counter router
    // 创建计数器路由器的实例
    let router = RouterService(common::counter::CounterRouter::new());

    // Create and run the server
    // 创建并运行服务器
    let server = Server::new(router);
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests"); // 服务器已初始化并准备好处理请求
    Ok(server.run(transport).await?)
}
