use anyhow::Result;
use clap::{Parser, ValueHint};
use mcp_client::transport::sse::SseTransportHandle;
use mcp_core::prompt::PromptMessageContent;
use mcp_core::protocol::{InitializeResult, JsonRpcRequest, JsonRpcResponse};
use mcp_core::ResourceContents;
use mcp_server::router::RouterService;
use mcp_server::{ByteTransport, RouterError, Server};
use tokio::io::{stdin, stdout};
#[cfg(debug_assertions)]
use tracing_appender::rolling::{RollingFileAppender, Rotation};
#[cfg(debug_assertions)]
use tracing_subscriber::{self, EnvFilter};

use std::{future::Future, pin::Pin, sync::Arc};

use mcp_client::McpService;
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientTrait};
use mcp_client::transport::{SseTransport, Transport};
use mcp_core::{
    Content, Resource, Tool, ToolError,
    handler::{PromptError, ResourceError},
    prompt::Prompt,
    protocol::ServerCapabilities,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;


type SseProxyClient = Arc<tokio::sync::Mutex<McpClient<tower::timeout::Timeout<McpService<SseTransportHandle>>>>>;

#[derive(Clone)]
pub struct SSEProxyRouter {
    server_info: InitializeResult,
    client: SseProxyClient,
}

impl SSEProxyRouter {
    pub fn new(server_info: InitializeResult, client: SseProxyClient) -> Self {
        Self {
            server_info,
            client
        }
    }

    async fn initialize(sse_url: String) -> Result<SSEProxyRouter> {
        // 创建基本的传输方式
        let transport = SseTransport::new(sse_url, HashMap::new());
        // 启动传输
        let handle = transport.start().await?;
        // 创建客户端zzx
        // Create client
        let client = Arc::new(Mutex::new(
            McpClient::new(async {
                // 创建带有超时中间件的服务
                McpService::with_timeout(handle, Duration::from_secs(3))
            }.await)
        ));
        #[cfg(debug_assertions)]
        tracing::info!("Client created\n");

        // 初始化
        // Initialize
        let server_info = client.lock().await
            .initialize(
                ClientInfo {
                    name: "mcp-proxy".into(),
                    version: "1.0.0".into(),
                },
                ClientCapabilities::default(),
        )
        .await?;
        // 休眠 100 毫秒，以允许服务器启动 - 令人惊讶的是，这是必需的！
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        #[cfg(debug_assertions)]
        tracing::info!("server_info initialize, {:?}", server_info);

        Ok(
            SSEProxyRouter {
                client,
                server_info,
            }
        )
    }
}

impl mcp_server::Router for SSEProxyRouter {
    fn handle_initialize(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            #[cfg(debug_assertions)]
            tracing::info!("handle_initialize, {:?}", self.server_info);
            
            let result = InitializeResult {
                protocol_version: self.server_info.protocol_version.clone(),
                capabilities: self.capabilities(),
                server_info: self.server_info.server_info.clone(),
                instructions: self.server_info.instructions.clone(),
            };

            let mut response = self.create_response(req.id);
            response.result =
                Some(serde_json::to_value(result).map_err(|e| {
                    RouterError::Internal(format!("JSON serialization error: {}", e))
                })?);

            Ok(response)
        }
    }

    fn name(&self) -> String {
        self.server_info.server_info.name.clone()
    }

    fn instructions(&self) -> Option<String> {
        self.server_info.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        // 构建服务器能力
        // self.server_info.capabilities.clone()
        #[cfg(debug_assertions)]
        tracing::info!("capabilities prompts, {:?}", self.server_info.capabilities.prompts);
        #[cfg(debug_assertions)]
        tracing::info!("capabilities resources, {:?}", self.server_info.capabilities.resources);
        #[cfg(debug_assertions)]
        tracing::info!("capabilities tools, {:?}", self.server_info.capabilities.tools);

        ServerCapabilities {
            prompts: Some(
                mcp_core::protocol::PromptsCapability {
                    list_changed: Some(true),
                }
            ),
            resources: Some(
                mcp_core::protocol::ResourcesCapability {
                    subscribe: Some(true),
                    list_changed: Some(true),
                }
            ),
            tools: Some(
                mcp_core::protocol::ToolsCapability {
                    list_changed: Some(true),
                }
            )
        }
    }

    fn list_tools(&self) -> impl Future<Output = Vec<Tool>> + Send {
        async move {
            let res = self.client.lock().await.list_tools(None).await;
            match res  {
                Ok(res) => res.tools,
                Err(e) => {
                    tracing::error!("Failed to list tools: {:?}", e);
                    vec![]
                }
            }
        }
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            let res = this.client.lock().await.call_tool(&tool_name, arguments).await;
            match res {
                Ok(res) => Ok(res.content),
                Err(e) => {
                    tracing::error!("Failed to call tool: {:?}", e);
                    Err(ToolError::NotFound(format!("Tool {} not found", tool_name)))
                }
            }
        })
    }

    fn list_resources(&self) -> impl Future<Output = Vec<Resource>> + Send {
        async move {
            let res = self.client.lock().await.list_resources(None).await;
            match res {
                Ok(res) => res.resources,
                Err(e) => {
                    tracing::error!("Failed to list resources: {:?}", e);
                    vec![
                        // self.create_resource_text("str:////Users/to/some/path/", "cwd"), // 当前工作目录
                        // self.create_resource_text("memo://insights", "memo-name"),       // 备忘录名称
                    ]
                }
            }
        }
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        let res = self.client.clone();
        Box::pin(async move {
            let content = res.lock().await.read_resource(&uri).await;
            match content {
                Ok(content) => {
                    if content.contents.is_empty() {
                        return Err(ResourceError::NotFound(format!(
                            "Resource {} not found",
                            uri
                        )))
                    }
                    for item in content.contents {
                        if let ResourceContents::TextResourceContents { uri:_, mime_type:_, text } = item {
                            return Ok(text.clone())
                        } else {
                            continue;
                        }
                    }

                    return Err(ResourceError::NotFound(format!(
                        "Resource {} not found",
                        uri
                    )))
                },
                Err(e) => {
                    tracing::error!("Failed to read resource: {:?}", e);
                    Err(ResourceError::NotFound(format!(
                        "Resource {} not found",
                        uri
                    )))
                }
            }
            // Ok(String::from(""))
        })
    }

    fn list_prompts(&self) -> impl Future<Output = Vec<Prompt>> +Send {
        async move {
            self.client.lock().await.list_prompts(None).await.unwrap().prompts
        }
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
        params: &Value
    ) -> impl Future<Output = Result<String, PromptError>> + Send {
        let prompt_name = prompt_name.to_string();
        async move {
            let res = self.client.lock().await.get_prompt(&prompt_name, params.clone()).await;
            match res {
                Ok(res) => {
                    // let mut prompt = res.messages[0].content.clone();
                    for message in res.messages {
                        // prompt.push_str(&message.content);
                        if let PromptMessageContent::Text { text } = message.content {
                            return Ok(text.clone())
                        } else {
                            continue;
                        }
                    }
                    return Err(PromptError::NotFound(format!(
                        "Prompt {} not found",
                        prompt_name
                    )))
                },
                Err(e) => {
                    tracing::error!("Failed to get prompt: {:?}", e);
                    Err(PromptError::NotFound(format!(
                        "Prompt {} not found",
                        prompt_name
                    )))
                }
            }
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// SSE MCP Server URL
    #[arg(short, long, value_hint=ValueHint::Url, required = true)]
    sse_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(_e) => {
            use clap::CommandFactory;
            let mut cmd = Args::command();
            cmd.print_help()?;
            std::process::exit(1);
        }
    };

    let url = args.sse_url;
    // let url = url;

    // Set up file appender for logging
    // 设置文件追加器用于日志记录
    #[cfg(debug_assertions)]
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "/Users/gemini/Documents/code/rust-sdk/logs", "mcp-server.log");

    // Initialize the tracing subscriber with file and stdout logging
    // 使用文件和标准输出日志记录初始化 tracing subscriber
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    #[cfg(debug_assertions)]
    tracing::info!("Starting MCP server, {:?}", url.to_string()); // 启动 MCP 服务器
    
    #[cfg(debug_assertions)]
    tracing::info!("Starting MCP server, {:?}", url.to_string()); // 启动 MCP 服务器
    
    let service_router: SSEProxyRouter = SSEProxyRouter::initialize(url.to_string()).await?;

    // Create an instance of our counter router
    // 创建计数器路由器的实例
    let router = RouterService(service_router);

    // Create and run the server
    // 创建并运行服务器
    let server = Server::new(router);
    let transport = ByteTransport::new(stdin(), stdout());

    #[cfg(debug_assertions)]
    tracing::info!("Server initialized and ready to handle requests"); // 服务器已初始化并准备好处理请求
    Ok(server.run(transport).await?)
}
