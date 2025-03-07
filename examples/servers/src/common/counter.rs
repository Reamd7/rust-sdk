use std::{future::Future, pin::Pin, sync::Arc};

use mcp_core::{
    handler::{PromptError, ResourceError},
    prompt::{Prompt, PromptArgument},
    protocol::ServerCapabilities,
    Content, Resource, Tool, ToolError,
};
use mcp_server::router::CapabilitiesBuilder;
use serde_json::Value;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct CounterRouter {
    counter: Arc<Mutex<i32>>, // 计数器
}

impl CounterRouter {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)), // 初始值为 0
        }
    }

    // 增加计数器
    // Increment the counter
    async fn increment(&self) -> Result<i32, ToolError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(*counter)
    }

    // 减少计数器
    // Decrement the counter
    async fn decrement(&self) -> Result<i32, ToolError> {
        let mut counter = self.counter.lock().await;
        *counter -= 1;
        Ok(*counter)
    }

    // 获取计数器的值
    // Get the counter value
    async fn get_value(&self) -> Result<i32, ToolError> {
        let counter = self.counter.lock().await;
        Ok(*counter)
    }

    // 创建资源文本
    // Create resource text
    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        Resource::new(uri, Some("text/plain".to_string()), Some(name.to_string())).unwrap()
    }
}

impl mcp_server::Router for CounterRouter {
    fn name(&self) -> String {
        "counter".to_string() // 路由名称
    }

    fn instructions(&self) -> Option<String> {
        Some(
            "This server provides a counter tool that can increment and decrement values. The counter starts at 0 and can be modified using the 'increment' and 'decrement' tools. Use 'get_value' to check the current count.".to_string() // 路由说明
        )
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .with_prompts(false)
            .build()
    }

    async fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "increment".to_string(), // 工具名称
                "Increment the counter by 1".to_string(), // 工具描述
                serde_json::json!({ // 工具参数
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            Tool::new(
                "decrement".to_string(), // 工具名称
                "Decrement the counter by 1".to_string(), // 工具描述
                serde_json::json!({ // 工具参数
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            Tool::new(
                "get_value".to_string(), // 工具名称
                "Get the current counter value".to_string(), // 工具描述
                serde_json::json!({ // 工具参数
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
        ]
    }

    fn call_tool(
        &self,
        tool_name: &str,
        _arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            match tool_name.as_str() {
                "increment" => { // 增加计数器
                    let value = this.increment().await?;
                    Ok(vec![Content::text(value.to_string())])
                }
                "decrement" => { // 减少计数器
                    let value = this.decrement().await?;
                    Ok(vec![Content::text(value.to_string())])
                }
                "get_value" => { // 获取计数器值
                    let value = this.get_value().await?;
                    Ok(vec![Content::text(value.to_string())])
                }
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))), // 工具未找到
            }
        })
    }

    async fn list_resources(&self) -> Vec<Resource> {
        vec![
            self._create_resource_text("str:////Users/to/some/path/", "cwd"), // 当前工作目录
            self._create_resource_text("memo://insights", "memo-name"), // 备忘录名称
        ]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            match uri.as_str() {
                "str:////Users/to/some/path/" => { // 当前工作目录
                    let cwd = "/Users/to/some/path/";
                    Ok(cwd.to_string())
                }
                "memo://insights" => { // 备忘录
                    let memo =
                        "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                    Ok(memo.to_string())
                }
                _ => Err(ResourceError::NotFound(format!( // 资源未找到
                    "Resource {} not found",
                    uri
                ))),
            }
        })
    }

    async fn list_prompts(&self) -> Vec<Prompt> {
        vec![Prompt::new(
            "example_prompt",
            Some("This is an example prompt that takes one required agrument, message"),
            Some(vec![PromptArgument {
                name: "message".to_string(),
                description: Some("A message to put in the prompt".to_string()),
                required: Some(true),
            }]),
        )]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
        _arguments: &Value,
    ) -> impl Future<Output = Result<std::string::String, PromptError>> + Send {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            match prompt_name.as_str() {
                "example_prompt" => {
                    let prompt = "This is an example prompt with your message here: '{message}'";
                    Ok(prompt.to_string())
                }
                _ => Err(PromptError::NotFound(format!( // 提示未找到
                    "Prompt {} not found",
                    prompt_name
                ))),
            }
        })
    }
}
