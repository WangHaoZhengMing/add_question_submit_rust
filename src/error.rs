use std::fmt;

/// 应用程序错误类型
#[derive(Debug)]
pub enum AppError {
    /// 浏览器相关错误
    Browser(BrowserError),
    /// API 调用错误
    Api(ApiError),
    /// 文件操作错误
    File(FileError),
    /// LLM 服务错误
    Llm(LlmError),
    /// 业务逻辑错误
    Business(BusinessError),
    /// 配置错误
    Config(ConfigError),
    /// 其他错误（用于包装第三方库错误）
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Browser(e) => write!(f, "浏览器错误: {}", e),
            AppError::Api(e) => write!(f, "API错误: {}", e),
            AppError::File(e) => write!(f, "文件错误: {}", e),
            AppError::Llm(e) => write!(f, "LLM错误: {}", e),
            AppError::Business(e) => write!(f, "业务错误: {}", e),
            AppError::Config(e) => write!(f, "配置错误: {}", e),
            AppError::Other(msg) => write!(f, "错误: {}", msg),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Browser(e) => Some(e),
            AppError::Api(e) => Some(e),
            AppError::File(e) => Some(e),
            AppError::Llm(e) => Some(e),
            AppError::Business(e) => Some(e),
            AppError::Config(e) => Some(e),
            AppError::Other(_) => None,
        }
    }
}

/// 浏览器相关错误
#[derive(Debug)]
pub enum BrowserError {
    /// 连接浏览器失败
    ConnectionFailed {
        port: u16,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 创建页面失败
    PageCreationFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 导航失败
    NavigationFailed {
        url: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 执行脚本失败
    ScriptExecutionFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 浏览器配置失败
    ConfigurationFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl fmt::Display for BrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowserError::ConnectionFailed { port, source } => {
                write!(f, "无法连接到浏览器 (端口: {}): {}", port, source)
            }
            BrowserError::PageCreationFailed { source } => {
                write!(f, "创建页面失败: {}", source)
            }
            BrowserError::NavigationFailed { url, source } => {
                write!(f, "导航到 {} 失败: {}", url, source)
            }
            BrowserError::ScriptExecutionFailed { source } => {
                write!(f, "执行脚本失败: {}", source)
            }
            BrowserError::ConfigurationFailed { source } => {
                write!(f, "浏览器配置失败: {}", source)
            }
        }
    }
}

impl std::error::Error for BrowserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BrowserError::ConnectionFailed { source, .. }
            | BrowserError::PageCreationFailed { source }
            | BrowserError::NavigationFailed { source, .. }
            | BrowserError::ScriptExecutionFailed { source }
            | BrowserError::ConfigurationFailed { source } => {
                Some(source.as_ref() as &(dyn std::error::Error + 'static))
            }
        }
    }
}

/// API 调用错误
#[derive(Debug)]
pub enum ApiError {
    /// 网络请求失败
    RequestFailed {
        endpoint: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// API 返回错误响应
    BadResponse {
        endpoint: String,
        code: Option<u64>,
        message: Option<String>,
    },
    /// API 返回空结果
    EmptyResponse {
        endpoint: String,
    },
    /// 请求频率限制
    RateLimited {
        endpoint: String,
        retry_after: Option<u64>,
    },
    /// JSON 解析失败
    JsonParseFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::RequestFailed { endpoint, source } => {
                write!(f, "API请求失败 ({}): {}", endpoint, source)
            }
            ApiError::BadResponse {
                endpoint,
                code,
                message,
            } => {
                write!(
                    f,
                    "API返回错误响应 ({}): code={:?}, message={:?}",
                    endpoint, code, message
                )
            }
            ApiError::EmptyResponse { endpoint } => {
                write!(f, "API返回空结果: {}", endpoint)
            }
            ApiError::RateLimited {
                endpoint,
                retry_after,
            } => {
                write!(
                    f,
                    "API请求频率限制 ({}), 建议等待: {:?}秒",
                    endpoint, retry_after
                )
            }
            ApiError::JsonParseFailed { source } => {
                write!(f, "JSON解析失败: {}", source)
            }
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApiError::RequestFailed { source, .. } | ApiError::JsonParseFailed { source } => {
                Some(source.as_ref() as &(dyn std::error::Error + 'static))
            }
            _ => None,
        }
    }
}

/// 文件操作错误
#[derive(Debug)]
pub enum FileError {
    /// 文件不存在
    NotFound {
        path: String,
    },
    /// 读取文件失败
    ReadFailed {
        path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 写入文件失败
    WriteFailed {
        path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 删除文件失败
    DeleteFailed {
        path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// TOML 解析失败
    TomlParseFailed {
        path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 目录不存在
    DirectoryNotFound {
        path: String,
    },
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::NotFound { path } => write!(f, "文件不存在: {}", path),
            FileError::ReadFailed { path, source } => {
                write!(f, "读取文件失败 ({}): {}", path, source)
            }
            FileError::WriteFailed { path, source } => {
                write!(f, "写入文件失败 ({}): {}", path, source)
            }
            FileError::DeleteFailed { path, source } => {
                write!(f, "删除文件失败 ({}): {}", path, source)
            }
            FileError::TomlParseFailed { path, source } => {
                write!(f, "TOML解析失败 ({}): {}", path, source)
            }
            FileError::DirectoryNotFound { path } => write!(f, "目录不存在: {}", path),
        }
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FileError::ReadFailed { source, .. }
            | FileError::WriteFailed { source, .. }
            | FileError::DeleteFailed { source, .. }
            | FileError::TomlParseFailed { source, .. } => {
                Some(source.as_ref() as &(dyn std::error::Error + 'static))
            }
            _ => None,
        }
    }
}

/// LLM 服务错误
#[derive(Debug)]
pub enum LlmError {
    /// API 调用失败
    ApiCallFailed {
        model: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 返回结果为空
    EmptyResponse {
        model: String,
    },
    /// 返回内容为空
    EmptyContent {
        model: String,
    },
    /// 索引解析失败
    IndexParseFailed {
        response: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// 索引超出范围
    IndexOutOfRange {
        index: usize,
        max_index: usize,
    },
    /// 搜索结果列表为空
    EmptySearchResults,
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::ApiCallFailed { model, source } => {
                write!(f, "LLM API调用失败 (模型: {}): {}", model, source)
            }
            LlmError::EmptyResponse { model } => {
                write!(f, "LLM返回结果为空 (模型: {})", model)
            }
            LlmError::EmptyContent { model } => {
                write!(f, "LLM返回内容为空 (模型: {})", model)
            }
            LlmError::IndexParseFailed { response, source } => {
                write!(f, "无法解析LLM返回的索引 (响应: {}): {}", response, source)
            }
            LlmError::IndexOutOfRange { index, max_index } => {
                write!(
                    f,
                    "LLM返回的索引 {} 超出范围 [0, {}]",
                    index, max_index
                )
            }
            LlmError::EmptySearchResults => {
                write!(f, "搜索结果列表不能为空")
            }
        }
    }
}

impl std::error::Error for LlmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LlmError::ApiCallFailed { source, .. } | LlmError::IndexParseFailed { source, .. } => {
                Some(source.as_ref() as &(dyn std::error::Error + 'static))
            }
            _ => None,
        }
    }
}

/// 业务逻辑错误
#[derive(Debug)]
pub enum BusinessError {
    /// 试卷ID为空
    EmptyPaperId,
    /// 搜索结果为空
    EmptySearchResults,
    /// 题目提交失败
    QuestionSubmitFailed {
        paper_index: usize,
    },
    /// 试卷提交失败
    PaperSubmitFailed {
        paper_index: usize,
    },
    /// 索引超出范围
    IndexOutOfRange {
        index: usize,
        max_index: usize,
    },
    /// 科目解析失败
    SubjectParseFailed {
        subject: String,
    },
}

impl fmt::Display for BusinessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusinessError::EmptyPaperId => write!(f, "试卷ID不能为空"),
            BusinessError::EmptySearchResults => write!(f, "搜索结果为空"),
            BusinessError::QuestionSubmitFailed { paper_index } => {
                write!(f, "题目提交失败 (试卷: {})", paper_index)
            }
            BusinessError::PaperSubmitFailed { paper_index } => {
                write!(f, "试卷提交失败 (试卷: {})", paper_index)
            }
            BusinessError::IndexOutOfRange { index, max_index } => {
                write!(f, "索引 {} 超出范围 [0, {}]", index, max_index)
            }
            BusinessError::SubjectParseFailed { subject } => {
                write!(f, "无法解析科目: {}", subject)
            }
        }
    }
}

impl std::error::Error for BusinessError {}

/// 配置错误
#[derive(Debug)]
pub enum ConfigError {
    /// 环境变量解析失败
    EnvVarParseFailed {
        var_name: String,
        value: String,
        expected_type: String,
    },
    /// 环境变量不存在
    EnvVarNotFound {
        var_name: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::EnvVarParseFailed {
                var_name,
                value,
                expected_type,
            } => {
                write!(
                    f,
                    "环境变量 {} 解析失败: 值 '{}' 无法转换为 {}",
                    var_name, value, expected_type
                )
            }
            ConfigError::EnvVarNotFound { var_name } => {
                write!(f, "环境变量 {} 不存在", var_name)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

// ========== 从常见错误类型转换 ==========
// 注意：不需要手动实现 From<AppError> for anyhow::Error，
// 因为 anyhow 已经为所有实现了 std::error::Error 的类型提供了自动实现

impl From<chromiumoxide::error::CdpError> for AppError {
    fn from(err: chromiumoxide::error::CdpError) -> Self {
        AppError::Browser(BrowserError::ScriptExecutionFailed {
            source: Box::new(err),
        })
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Api(ApiError::JsonParseFailed {
            source: Box::new(err),
        })
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        AppError::File(FileError::TomlParseFailed {
            path: String::new(), // TOML错误通常不包含路径信息
            source: Box::new(err),
        })
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::File(FileError::ReadFailed {
            path: String::new(),
            source: Box::new(err),
        })
    }
}

// ========== 便捷构造函数 ==========

impl AppError {
    /// 创建浏览器连接错误
    pub fn browser_connection_failed(port: u16, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        AppError::Browser(BrowserError::ConnectionFailed {
            port,
            source: Box::new(source),
        })
    }

    /// 创建API请求失败错误
    pub fn api_request_failed(endpoint: impl Into<String>, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        AppError::Api(ApiError::RequestFailed {
            endpoint: endpoint.into(),
            source: Box::new(source),
        })
    }

    /// 创建文件读取错误
    pub fn file_read_failed(path: impl Into<String>, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        AppError::File(FileError::ReadFailed {
            path: path.into(),
            source: Box::new(source),
        })
    }

    /// 创建LLM API调用错误
    pub fn llm_api_failed(model: impl Into<String>, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        AppError::Llm(LlmError::ApiCallFailed {
            model: model.into(),
            source: Box::new(source),
        })
    }
}

// ========== Result 类型别名 ==========

/// 应用程序结果类型
pub type AppResult<T> = Result<T, AppError>;

