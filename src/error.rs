//! 简化的错误处理
//!
//! 使用 thiserror 定义关键错误类型，其他地方使用 anyhow::Result

use thiserror::Error;

/// 应用程序错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("题库API错误: {0}")]
    TikuApi(String),

    #[error("LLM调用错误: {0}")]
    Llm(String),

    #[error("浏览器操作错误: {0}")]
    Browser(String),

    #[error("文件操作错误: {0}")]
    File(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("搜索结果为空")]
    EmptySearchResult,

    #[error("索引超出范围: {index} >= {max}")]
    IndexOutOfRange { index: usize, max: usize },
}

/// 应用程序 Result 类型别名
pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    /// 创建题库API错误
    pub fn tiku_api(msg: impl Into<String>) -> Self {
        Self::TikuApi(msg.into())
    }

    /// 创建LLM错误
    pub fn llm(msg: impl Into<String>) -> Self {
        Self::Llm(msg.into())
    }

    /// 创建浏览器错误
    pub fn browser(msg: impl Into<String>) -> Self {
        Self::Browser(msg.into())
    }

    /// 创建文件错误
    pub fn file(msg: impl Into<String>) -> Self {
        Self::File(msg.into())
    }
}

// 从常见错误类型自动转换
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::File(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::TikuApi(format!("JSON解析错误: {}", err))
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        Self::File(format!("TOML解析错误: {}", err))
    }
}
