//! 在代理、扩展和 LLM 之间发送的内容
//! 各种内容类型可以显示给人类，也可以被模型理解
//! 它们包括可选的注释，用于帮助告知代理使用情况
use super::role::Role; // 引入 role 模块中的 Role
use crate::resource::ResourceContents; // 引入 resource 模块中的 ResourceContents
use chrono::{DateTime, Utc}; // 引入 chrono 库，提供 DateTime 和 Utc 类型，用于处理时间和日期
use serde::{Deserialize, Serialize}; // 引入 serde 库，提供 Deserialize 和 Serialize trait，用于序列化和反序列化

/// 注释
/// Annotations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// 观众
    /// Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    /// 优先级
    /// Priority
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
    /// 时间戳
    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl Annotations {
    /// 专门为资源创建新的 Annotations 实例
    /// optional priority, and a timestamp (defaults to now if None)
    pub fn for_resource(priority: f32, timestamp: DateTime<Utc>) -> Self {
        assert!(
            (0.0..=1.0).contains(&priority),
            "Priority {priority} must be between 0.0 and 1.0"
        );
        Annotations {
            priority: Some(priority),
            timestamp: Some(timestamp),
            audience: None,
        }
    }
}

/// 文本内容
/// Text Content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// 文本
    /// Text
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// 图像内容
/// Image Content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// 数据
    /// Data
    pub data: String,
    /// MIME 类型
    /// Mime Type
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// 嵌入式资源
/// Embedded Resource
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedResource {
    /// 资源
    /// Resource
    pub resource: ResourceContents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

impl EmbeddedResource {
    pub fn get_text(&self) -> String {
        match &self.resource {
            ResourceContents::TextResourceContents { text, .. } => text.clone(),
            _ => String::new(),
        }
    }
}

/// 内容
/// Content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Content {
    /// 文本
    /// Text
    Text(TextContent),
    /// 图像
    /// Image
    Image(ImageContent),
    /// 资源
    /// Resource
    Resource(EmbeddedResource),
}

impl Content {
    /// 文本
    /// Text
    pub fn text<S: Into<String>>(text: S) -> Self {
        Content::Text(TextContent {
            text: text.into(),
            annotations: None,
        })
    }

    /// 图像
    /// Image
    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        Content::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
        })
    }

    /// 资源
    /// Resource
    pub fn resource(resource: ResourceContents) -> Self {
        Content::Resource(EmbeddedResource {
            resource,
            annotations: None,
        })
    }

    /// 嵌入式文本
    /// Embedded Text
    pub fn embedded_text<S: Into<String>, T: Into<String>>(uri: S, content: T) -> Self {
        Content::Resource(EmbeddedResource {
            resource: ResourceContents::TextResourceContents {
                uri: uri.into(),
                mime_type: Some("text".to_string()),
                text: content.into(),
            },
            annotations: None,
        })
    }

    /// 如果这是 TextContent 变体，则获取文本内容
    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Text(text) => Some(&text.text),
            _ => None,
        }
    }

    /// 如果这是 ImageContent 变体，则获取图像内容
    /// Get the image content if this is an ImageContent variant
    pub fn as_image(&self) -> Option<(&str, &str)> {
        match self {
            Content::Image(image) => Some((&image.data, &image.mime_type)),
            _ => None,
        }
    }

    /// 设置内容的受众
    /// Set the audience for the content
    pub fn with_audience(mut self, audience: Vec<Role>) -> Self {
        let annotations = match &mut self {
            Content::Text(text) => &mut text.annotations,
            Content::Image(image) => &mut image.annotations,
            Content::Resource(resource) => &mut resource.annotations,
        };
        *annotations = Some(match annotations.take() {
            Some(mut a) => {
                a.audience = Some(audience);
                a
            }
            None => Annotations {
                audience: Some(audience),
                priority: None,
                timestamp: None,
            },
        });
        self
    }

    /// 设置内容的优先级
    /// Set the priority for the content
    /// # Panics
    /// 如果优先级不在 0.0 和 1.0 之间（含 0.0 和 1.0），则会发生 panic
    /// Panics if priority is not between 0.0 and 1.0 inclusive
    pub fn with_priority(mut self, priority: f32) -> Self {
        if !(0.0..=1.0).contains(&priority) {
            panic!("Priority must be between 0.0 and 1.0");
        }
        let annotations = match &mut self {
            Content::Text(text) => &mut text.annotations,
            Content::Image(image) => &mut image.annotations,
            Content::Resource(resource) => &mut resource.annotations,
        };
        *annotations = Some(match annotations.take() {
            Some(mut a) => {
                a.priority = Some(priority);
                a
            }
            None => Annotations {
                audience: None,
                priority: Some(priority),
                timestamp: None,
            },
        });
        self
    }

    /// 获取受众（如果已设置）
    /// Get the audience if set
    pub fn audience(&self) -> Option<&Vec<Role>> {
        match self {
            Content::Text(text) => text.annotations.as_ref().and_then(|a| a.audience.as_ref()),
            Content::Image(image) => image.annotations.as_ref().and_then(|a| a.audience.as_ref()),
            Content::Resource(resource) => resource
                .annotations
                .as_ref()
                .and_then(|a| a.audience.as_ref()),
        }
    }

    /// 获取优先级（如果已设置）
    /// Get the priority if set
    pub fn priority(&self) -> Option<f32> {
        match self {
            Content::Text(text) => text.annotations.as_ref().and_then(|a| a.priority),
            Content::Image(image) => image.annotations.as_ref().and_then(|a| a.priority),
            Content::Resource(resource) => resource.annotations.as_ref().and_then(|a| a.priority),
        }
    }

    /// 取消注释
    /// Unannotated
    pub fn unannotated(&self) -> Self {
        match self {
            Content::Text(text) => Content::text(text.text.clone()),
            Content::Image(image) => Content::image(image.data.clone(), image.mime_type.clone()),
            Content::Resource(resource) => Content::resource(resource.resource.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_text() {
        let content = Content::text("hello");
        assert_eq!(content.as_text(), Some("hello"));
        assert_eq!(content.as_image(), None);
    }

    #[test]
    fn test_content_image() {
        let content = Content::image("data", "image/png");
        assert_eq!(content.as_text(), None);
        assert_eq!(content.as_image(), Some(("data", "image/png")));
    }

    #[test]
    fn test_content_annotations_basic() {
        let content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);
        assert_eq!(content.audience(), Some(&vec![Role::User]));
        assert_eq!(content.priority(), Some(0.5));
    }

    #[test]
    fn test_content_annotations_order_independence() {
        let content1 = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);
        let content2 = Content::text("hello")
            .with_priority(0.5)
            .with_audience(vec![Role::User]);

        assert_eq!(content1.audience(), content2.audience());
        assert_eq!(content1.priority(), content2.priority());
    }

    #[test]
    fn test_content_annotations_overwrite() {
        let content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5)
            .with_audience(vec![Role::Assistant])
            .with_priority(0.8);

        assert_eq!(content.audience(), Some(&vec![Role::Assistant]));
        assert_eq!(content.priority(), Some(0.8));
    }

    #[test]
    fn test_content_annotations_image() {
        let content = Content::image("data", "image/png")
            .with_audience(vec![Role::User])
            .with_priority(0.5);

        assert_eq!(content.audience(), Some(&vec![Role::User]));
        assert_eq!(content.priority(), Some(0.5));
    }

    #[test]
    fn test_content_annotations_preservation() {
        let text_content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);

        match &text_content {
            Content::Text(TextContent { annotations, .. }) => {
                assert!(annotations.is_some());
                let ann = annotations.as_ref().unwrap();
                assert_eq!(ann.audience, Some(vec![Role::User]));
                assert_eq!(ann.priority, Some(0.5));
            }
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    #[should_panic(expected = "Priority must be between 0.0 and 1.0")]
    fn test_invalid_priority() {
        Content::text("hello").with_priority(1.5);
    }

    #[test]
    fn test_unannotated() {
        let content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);
        let unannotated = content.unannotated();
        assert_eq!(unannotated.audience(), None);
        assert_eq!(unannotated.priority(), None);
    }

    #[test]
    fn test_partial_annotations() {
        let content = Content::text("hello").with_priority(0.5);
        assert_eq!(content.audience(), None);
        assert_eq!(content.priority(), Some(0.5));

        let content = Content::text("hello").with_audience(vec![Role::User]);
        assert_eq!(content.audience(), Some(&vec![Role::User]));
        assert_eq!(content.priority(), None);
    }
}
