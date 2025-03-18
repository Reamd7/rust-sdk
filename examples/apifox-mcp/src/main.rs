use reqwest::header::HeaderMap;
use anyhow::Result;
use mcp_server::router::RouterService;
use mcp_server::{ByteTransport, Server};
use tokio::io::{stdin, stdout};

use tracing_appender::rolling::{RollingFileAppender, Rotation};

use tracing_subscriber::{self, EnvFilter};
use std::env;
use std::path::Path;
use std::{future::Future, pin::Pin};

use mcp_core::{
    handler::{PromptError, ResourceError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    Content, Resource, Tool, ToolError,
};
use mcp_server::router::CapabilitiesBuilder;
use serde_json::Value;
use reqwest;
use serde_json::json;
use clap::{Parser, ValueHint, command};

static APIFOX_BASE_URL: &'static str = "https://api.apifox.com/api";

#[derive(Parser, Debug, Clone)]
#[command(name = "ApifoxMcp")]
#[command(author, about, long_about = None)]
struct Args {
    /// Apifox AccessToken
    #[arg(short, long, value_hint=ValueHint::Unknown, required = false)]
    token: Option<String>,

    #[arg(short = 'V', long, action = clap::ArgAction::SetTrue, help = "Show version information")]
    version: bool,
}

#[derive(Clone)]
pub struct ApifoxMcpServerRouter {
    apifox_user_access_token: String,
}


impl ApifoxMcpServerRouter {
    pub fn new(token: String) -> Self {
        Self {
            apifox_user_access_token: token
        }
    }

    // 增加计数器
    // Increment the counter
    async fn get_endpoint_oas_by_link(&self, project_id: &str, endpoint_id: &str) -> Result<String, ToolError> {
        let path = format!("/v1/projects/{}/export-openapi", project_id);
        let url = format!("{}{}", APIFOX_BASE_URL, path);
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            "application/json".parse().unwrap()
        );
        headers.insert(
            "X-Apifox-Version",
            "2024-03-28".parse().unwrap()
        );
        headers.insert(
            "X-Project-Id",
            project_id.parse().unwrap()
        );
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.apifox_user_access_token).parse().unwrap()
        );

        let body = json!({
            "projectId": project_id,
            "type": 2,
            "format": "json".to_owned(),
            "version": "3.0".to_owned(),
            "apiDetailId": [endpoint_id.parse::<i64>().unwrap()],
            "includeTags": [],
            "excludeTags": [],
            "checkedFolder": [],
            "selectedEnvironments": [],
            "excludeExtension": true,
            "excludeTagsWithFolder": true,
        });

        let resp = client.post(&url).headers(headers).body(
            body.to_string()
        )
        .send()
        .await;

        match resp  {
            Ok(resp) => {
                if !resp.status().is_success() {
                    
                    tracing::error!("Failed to fetch apifox openapi url: {}", url);
                    return Err(ToolError::NotFound(format!("Failed to fetch apifox openapi url: {}", url)));
                }

                let body = resp.text().await;
                match body  {
                    Ok(body) => {
                        
                        tracing::info!("Successfully fetched apifox openapi url: {}", url);
                        return Ok(body)
                    }
                    Err(err) => {
                        
                        tracing::error!("Failed to fetch body error: {}", err);
                        return Err(ToolError::NotFound(format!("Failed to fetch body error: ${err}")));
                    }
                }
                
            },
            Err(_err) => {
                
                tracing::error!("Failed to fetch apifox openapi url: {}", url);
                return Err(ToolError::NotFound(format!("Failed to fetch apifox openapi url: ${url}")));
            }
        }
    }
}

impl mcp_server::Router for ApifoxMcpServerRouter {
    fn name(&self) -> String {
        "Apifox MCP Server".to_string() // 路由名称
    }

    fn instructions(&self) -> Option<String> {
        Some(
            "".to_string() // 路由说明
        )
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(true)
            .with_resources(false, false)
            .with_prompts(false)
            .build()
    }

    async fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "get_endpoint_oas_by_link".to_string(), // 工具名称
                "通过 Apifox 的协作链接来获取此接口的 OpenAPI Specification 格式定义，协作链接格式如下：https://app.apifox.com/link/project/{projectId}/apis/api-{endpointId} ，{projectId} 为 Apifox 的项目 ID，{endpointId} 为接口（Endpoint）的 ID，由于该链接是无法直接访问的，所以需要通过本工具来获取。如发现有符合条件的链接，则调用本工具".to_string(), // 工具描述
                serde_json::json!({ // 工具参数
                    "type": "object",
                    "properties": {
                        "projectId": {
                            "type": "string",
                            "description": "Apifox 的项目 ID"
                        },
                        "endpointId": {
                            "type": "string",
                            "description": "接口（Endpoint）的 ID"
                        }
                    },
                    "required": ["projectId", "endpointId"]
                }),
            ),
            Tool::new(
                "通过链接获取接口的OAS 定义".to_string(), // 工具名称
                "通过 Apifox 的协作链接来获取此接口的 OpenAPI Specification 格式定义，协作链接格式如下：https://app.apifox.com/link/project/{projectId}/apis/api-{endpointId} ，{projectId} 为 Apifox 的项目 ID，{endpointId} 为接口（Endpoint）的 ID，由于该链接是无法直接访问的，所以需要通过本工具来获取。如发现有符合条件的链接，则调用本工具".to_string(), // 工具描述
                serde_json::json!({ // 工具参数
                    "type": "object",
                    "properties": {
                        "projectId": {
                            "type": "string",
                            "description": "Apifox 的项目 ID"
                        },
                        "endpointId": {
                            "type": "string",
                            "description": "接口（Endpoint）的 ID"
                        }
                    },
                    "required": ["projectId", "endpointId"]
                }),
            ),
        ]
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            match tool_name.as_str() {
                "通过链接获取接口的OAS 定义" |
                "get_endpoint_oas_by_link" => { // 增加计数器
                    let param = arguments.as_object().unwrap();
                    let project_id = param.get("projectId").unwrap().as_str().unwrap();
                    let endpoint_id = param.get("endpointId").unwrap().as_str().unwrap();

                    let value = this.get_endpoint_oas_by_link(project_id, endpoint_id).await?;
                    Ok(vec![Content::text(value.to_string())])
                }
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))), // 工具未找到
            }
        })
    }

    async fn list_resources(&self) -> Vec<Resource> {
        vec![]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            Err(ResourceError::NotFound(format!( // 资源未找到
                "Resource {} not found",
                uri
            )))
        })
    }

    async fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
        _arguments: &Value,
    ) -> impl Future<Output = Result<std::string::String, PromptError>> + Send {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!( // 提示未找到
                "Prompt {} not found",
                prompt_name
            )))
        })
    }
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

    if args.version {
        println!("0.1.0");
        std::process::exit(0);
    }

    dotenv::dotenv().ok();
    let token = args.token.unwrap_or(env::var("APIFOX_USER_ACCESS_TOKEN").unwrap_or(String::from("")));

    if token.is_empty() {
        use clap::CommandFactory;
        let mut cmd = Args::command();
        cmd.print_help()?;
        std::process::exit(1);
    }

    // -----------------------

    let current_path = env::current_exe().unwrap();
    let current_dir = current_path.parent().unwrap();
    // let current_path_str = current_dir.to_str().unwrap();
    // if token.is_empty() {
        // use clap::CommandFactory;
        // let mut cmd = Args::command();
        // cmd.print_help()?;
        // std::process::exit(1);
    // }

    // Set up file appender for logging
    // 设置文件追加器用于日志记录
    
    let file_appender = RollingFileAppender::new(Rotation::DAILY, 
        Path::new(current_dir).join("logs"),
        "mcp-server.log");

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
    let router = RouterService(ApifoxMcpServerRouter::new(token));

    // Create and run the server
    // 创建并运行服务器
    let server = Server::new(router);
    let transport = ByteTransport::new(stdin(), stdout());
    
    tracing::info!("Server initialized and ready to handle requests"); // 服务器已初始化并准备好处理请求
    Ok(server.run(transport).await?)
}