//! 服务器提供给客户端的资源
use anyhow::{anyhow, Result}; // 引入 anyhow 库，提供 Result 类型和 anyhow! 宏，用于错误处理
use chrono::{DateTime, Utc}; // 引入 chrono 库，提供 DateTime 和 Utc 类型，用于处理时间和日期
use serde::{Deserialize, Serialize}; // 引入 serde 库，提供 Deserialize 和 Serialize trait，用于序列化和反序列化
use url::Url; // 引入 url 库，提供 Url 类型，用于处理 URL

use crate::content::Annotations; // 引入 content 模块中的 Annotations 类型

const EPSILON: f32 = 1e-6; // Tolerance for floating point comparison // 定义一个常量 EPSILON，用于浮点数比较的容差

/// Represents a resource in the extension with metadata
/// 表示扩展中的一个资源，包含元数据
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// URI representing the resource location (e.g., "file:///path/to/file" or "str:///content")
    /// URI，表示资源的位置（例如，"file:///path/to/file" 或 "str:///content"）
    pub uri: String,
    /// Name of the resource
    /// 资源的名称
    pub name: String,
    /// Optional description of the resource
    /// 资源的可选描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource content ("text" or "blob")
    /// 资源内容的 MIME 类型（"text" 或 "blob"）
    #[serde(default = "default_mime_type")]
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ResourceContents {
    TextResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        text: String,
    },
    BlobResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        blob: String,
    },
}

fn default_mime_type() -> String {
    "text".to_string()
}

impl Resource {
    /// Creates a new Resource from a URI with explicit mime type
    /// 从 URI 创建一个具有显式 MIME 类型的新 Resource
    pub fn new<S: AsRef<str>>(
        uri: S,
        mime_type: Option<String>,
        name: Option<String>,
    ) -> Result<Self> {
        let uri = uri.as_ref();
        let url = Url::parse(uri).map_err(|e| anyhow!("Invalid URI: {}", e))?; // 解析 URI，如果无效则返回错误

        // Extract name from the path component of the URI
        // Use provided name if available, otherwise extract from URI
        // 从 URI 的路径组件中提取名称
        // 如果提供了名称，则使用提供的名称，否则从 URI 中提取
        let name = match name {
            Some(n) => n,
            None => url
                .path_segments()
                .and_then(|segments| segments.last())
                .unwrap_or("unnamed")
                .to_string(),
        };

        // Use provided mime_type or default
        // 使用提供的 mime_type 或默认值
        let mime_type = match mime_type {
            Some(t) if t == "text" || t == "blob" => t,
            _ => default_mime_type(),
        };

        Ok(Self {
            uri: uri.to_string(),
            name,
            description: None,
            mime_type,
            annotations: Some(Annotations::for_resource(0.0, Utc::now())),
        })
    }

    /// Creates a new Resource with explicit URI, name, and priority
    /// 创建一个具有显式 URI、名称和优先级的新 Resource
    pub fn with_uri<S: Into<String>>(
        uri: S,
        name: S,
        priority: f32,
        mime_type: Option<String>,
    ) -> Result<Self> {
        let uri_string = uri.into();
        Url::parse(&uri_string).map_err(|e| anyhow!("Invalid URI: {}", e))?; // 解析 URI，如果无效则返回错误

        // Use provided mime_type or default
        // 使用提供的 mime_type 或默认值
        let mime_type = match mime_type {
            Some(t) if t == "text" || t == "blob" => t,
            _ => default_mime_type(),
        };

        Ok(Self {
            uri: uri_string,
            name: name.into(),
            description: None,
            mime_type,
            annotations: Some(Annotations::for_resource(priority, Utc::now())),
        })
    }

    /// Updates the resource's timestamp to the current time
    /// 将资源的时间戳更新为当前时间
    pub fn update_timestamp(&mut self) {
        self.annotations.as_mut().unwrap().timestamp = Some(Utc::now());
    }

    /// Sets the priority of the resource and returns self for method chaining
    /// 设置资源的优先级并返回 self，用于方法链式调用
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.annotations.as_mut().unwrap().priority = Some(priority);
        self
    }

    /// Mark the resource as active, i.e. set its priority to 1.0
    /// 将资源标记为活动状态，即将优先级设置为 1.0
    pub fn mark_active(self) -> Self {
        self.with_priority(1.0)
    }

    // Check if the resource is active
    // 检查资源是否处于活动状态
    pub fn is_active(&self) -> bool {
        if let Some(priority) = self.priority() {
            (priority - 1.0).abs() < EPSILON
        } else {
            false
        }
    }

    /// Returns the priority of the resource, if set
    /// 返回资源的优先级（如果已设置）
    pub fn priority(&self) -> Option<f32> {
        self.annotations.as_ref().and_then(|a| a.priority)
    }

    /// Returns the timestamp of the resource, if set
    /// 返回资源的时间戳（如果已设置）
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.annotations.as_ref().and_then(|a| a.timestamp)
    }

    /// Returns the scheme of the URI
    /// 返回 URI 的 scheme
    pub fn scheme(&self) -> Result<String> {
        let url = Url::parse(&self.uri)?;
        Ok(url.scheme().to_string())
    }

    /// Sets the description of the resource
    /// 设置资源的描述
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the MIME type of the resource
    /// 设置资源的 MIME 类型
    pub fn with_mime_type<S: Into<String>>(mut self, mime_type: S) -> Self {
        let mime_type = mime_type.into();
        match mime_type.as_str() {
            "text" | "blob" => self.mime_type = mime_type,
            _ => self.mime_type = default_mime_type(),
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_new_resource_with_file_uri() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "test content")?;

        let uri = Url::from_file_path(temp_file.path())
            .map_err(|_| anyhow!("Invalid file path"))?
            .to_string();

        let resource = Resource::new(&uri, Some("text".to_string()), None)?;
        assert!(resource.uri.starts_with("file:///"));
        assert_eq!(resource.priority(), Some(0.0));
        assert_eq!(resource.mime_type, "text");
        assert_eq!(resource.scheme()?, "file");

        Ok(())
    }

    #[test]
    fn test_resource_with_str_uri() -> Result<()> {
        let test_content = "Hello, world!";
        let uri = format!("str:///{}", test_content);
        let resource = Resource::with_uri(
            uri.clone(),
            "test.txt".to_string(),
            0.5,
            Some("text".to_string()),
        )?;

        assert_eq!(resource.uri, uri);
        assert_eq!(resource.name, "test.txt");
        assert_eq!(resource.priority(), Some(0.5));
        assert_eq!(resource.mime_type, "text");
        assert_eq!(resource.scheme()?, "str");

        Ok(())
    }

    #[test]
    fn test_mime_type_validation() -> Result<()> {
        // Test valid mime types
        let resource = Resource::new("file:///test.txt", Some("text".to_string()), None)?;
        assert_eq!(resource.mime_type, "text");

        let resource = Resource::new("file:///test.bin", Some("blob".to_string()), None)?;
        assert_eq!(resource.mime_type, "blob");

        // Test invalid mime type defaults to "text"
        let resource = Resource::new("file:///test.txt", Some("invalid".to_string()), None)?;
        assert_eq!(resource.mime_type, "text");

        // Test None defaults to "text"
        let resource = Resource::new("file:///test.txt", None, None)?;
        assert_eq!(resource.mime_type, "text");

        Ok(())
    }

    #[test]
    fn test_with_description() -> Result<()> {
        let resource = Resource::with_uri("file:///test.txt", "test.txt", 0.0, None)?
            .with_description("A test resource");

        assert_eq!(resource.description, Some("A test resource".to_string()));
        Ok(())
    }

    #[test]
    fn test_with_mime_type() -> Result<()> {
        let resource =
            Resource::with_uri("file:///test.txt", "test.txt", 0.0, None)?.with_mime_type("blob");

        assert_eq!(resource.mime_type, "blob");

        // Test invalid mime type defaults to "text"
        let resource = resource.with_mime_type("invalid");
        assert_eq!(resource.mime_type, "text");
        Ok(())
    }

    #[test]
    fn test_invalid_uri() {
        let result = Resource::new("not-a-uri", None, None);
        assert!(result.is_err());
    }
}
