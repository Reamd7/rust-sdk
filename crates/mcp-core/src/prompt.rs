use crate::content::{Annotations, EmbeddedResource, ImageContent}; // 引入 content 模块中的 Annotations, EmbeddedResource, ImageContent
use crate::handler::PromptError; // 引入 handler 模块中的 PromptError
use crate::resource::ResourceContents; // 引入 resource 模块中的 ResourceContents
use base64::engine::{general_purpose::STANDARD as BASE64_STANDARD, Engine}; // 引入 base64 库，用于 base64 编码和解码
use serde::{Deserialize, Serialize}; // 引入 serde 库，提供 Deserialize 和 Serialize trait，用于序列化和反序列化

/// 可用于从模型生成文本的 Prompt
/// A prompt that can be used to generate text from a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// Prompt 的名称
    /// The name of the prompt
    pub name: String,
    /// Prompt 功能的可选描述
    /// Optional description of what the prompt does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 可用于自定义 Prompt 的可选参数
    /// Optional arguments that can be passed to customize the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

impl Prompt {
    /// 使用给定的名称、描述和参数创建新的 Prompt
    /// Create a new prompt with the given name, description and arguments
    pub fn new<N, D>(
        name: N,
        description: Option<D>,
        arguments: Option<Vec<PromptArgument>>,
    ) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Prompt {
            name: name.into(),
            description: description.map(Into::into),
            arguments,
        }
    }
}

/// 表示可传递以自定义 Prompt 的 Prompt 参数
/// Represents a prompt argument that can be passed to customize the prompt
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptArgument {
    /// 参数的名称
    /// The name of the argument
    pub name: String,
    /// 参数用途的描述
    /// A description of what the argument is used for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 此参数是否为必需参数
    /// Whether this argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// 表示 Prompt 对话中消息发送者的角色
/// Represents the role of a message sender in a prompt conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptMessageRole {
    /// 用户
    /// User
    User,
    /// 助手
    /// Assistant
    Assistant,
}

/// 可包含在 Prompt 消息中的内容类型
/// Content types that can be included in prompt messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PromptMessageContent {
    /// 纯文本内容
    /// Plain text content
    Text { text: String },
    /// 具有 base64 编码数据的图像内容
    /// Image content with base64-encoded data
    Image { image: ImageContent },
    /// 嵌入式服务器端资源
    /// Embedded server-side resource
    Resource { resource: EmbeddedResource },
}

/// Prompt 对话中的消息
/// A message in a prompt conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptMessage {
    /// 消息发送者的角色
    /// The role of the message sender
    pub role: PromptMessageRole,
    /// 消息的内容
    /// The content of the message
    pub content: PromptMessageContent,
}

impl PromptMessage {
    /// 使用给定的角色和文本内容创建新的文本消息
    /// Create a new text message with the given role and text content
    pub fn new_text<S: Into<String>>(role: PromptMessageRole, text: S) -> Self {
        Self {
            role,
            content: PromptMessageContent::Text { text: text.into() },
        }
    }

    /// 创建新的图像消息
    /// Create a new image message
    pub fn new_image<S: Into<String>>(
        role: PromptMessageRole,
        data: S,
        mime_type: S,
        annotations: Option<Annotations>,
    ) -> Result<Self, PromptError> {
        let data = data.into();
        let mime_type = mime_type.into();

        // 验证 base64 数据
        // Validate base64 data
        BASE64_STANDARD.decode(&data).map_err(|_| {
            PromptError::InvalidParameters("Image data must be valid base64".to_string())
        })?;

        // 验证 mime 类型
        // Validate mime type
        if !mime_type.starts_with("image/") {
            return Err(PromptError::InvalidParameters(
                "MIME type must be a valid image type (e.g. image/jpeg)".to_string(),
            ));
        }

        Ok(Self {
            role,
            content: PromptMessageContent::Image {
                image: ImageContent {
                    data,
                    mime_type,
                    annotations,
                },
            },
        })
    }

    /// 创建新的资源消息
    /// Create a new resource message
    pub fn new_resource(
        role: PromptMessageRole,
        uri: String,
        mime_type: String,
        text: Option<String>,
        annotations: Option<Annotations>,
    ) -> Self {
        let resource_contents = ResourceContents::TextResourceContents {
            uri,
            mime_type: Some(mime_type),
            text: text.unwrap_or_default(),
        };

        Self {
            role,
            content: PromptMessageContent::Resource {
                resource: EmbeddedResource {
                    resource: resource_contents,
                    annotations,
                },
            },
        }
    }
}

/// Prompt 的模板
/// A template for a prompt
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub template: String,
    pub arguments: Vec<PromptArgumentTemplate>,
}

/// Prompt 参数的模板，应与 PromptArgument 相同
/// A template for a prompt argument, this should be identical to PromptArgument
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptArgumentTemplate {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}
