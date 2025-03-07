//! 描述内容来源/所有权的角色
use serde::{Deserialize, Serialize}; // 引入 serde 库，提供 Deserialize 和 Serialize trait，用于序列化和反序列化

/// Role enum to describe the origin/ownership of content
/// Role 枚举，用于描述内容的来源/所有权
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User role
    /// 用户角色
    User,
    /// Assistant role
    /// 助手角色
    Assistant,
}
