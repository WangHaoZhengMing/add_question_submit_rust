# 错误处理系统设计文档

## 概述

`src/error.rs` 定义了统一的错误类型系统，用于替代分散的 `anyhow::Error` 和 `unwrap()` 调用。

## 错误类型层次结构

```
AppError (顶层错误类型)
├── BrowserError      (浏览器相关错误)
├── ApiError          (API 调用错误)
├── FileError         (文件操作错误)
├── LlmError          (LLM 服务错误)
├── BusinessError     (业务逻辑错误)
├── ConfigError       (配置错误)
└── Other             (其他错误)
```

## 错误类型详细说明

### 1. BrowserError (浏览器错误)

- `ConnectionFailed`: 连接浏览器失败
- `PageCreationFailed`: 创建页面失败
- `NavigationFailed`: 导航到 URL 失败
- `ScriptExecutionFailed`: 执行 JavaScript 脚本失败
- `ConfigurationFailed`: 浏览器配置失败

### 2. ApiError (API 错误)

- `RequestFailed`: 网络请求失败
- `BadResponse`: API 返回错误响应（包含 code 和 message）
- `EmptyResponse`: API 返回空结果
- `RateLimited`: 请求频率限制
- `JsonParseFailed`: JSON 解析失败

### 3. FileError (文件错误)

- `NotFound`: 文件不存在
- `ReadFailed`: 读取文件失败
- `WriteFailed`: 写入文件失败
- `DeleteFailed`: 删除文件失败
- `TomlParseFailed`: TOML 解析失败
- `DirectoryNotFound`: 目录不存在

### 4. LlmError (LLM 错误)

- `ApiCallFailed`: LLM API 调用失败
- `EmptyResponse`: LLM 返回结果为空
- `EmptyContent`: LLM 返回内容为空
- `IndexParseFailed`: 无法解析 LLM 返回的索引
- `IndexOutOfRange`: 索引超出范围
- `EmptySearchResults`: 搜索结果列表为空

### 5. BusinessError (业务错误)

- `EmptyPaperId`: 试卷ID为空
- `EmptySearchResults`: 搜索结果为空
- `QuestionSubmitFailed`: 题目提交失败
- `PaperSubmitFailed`: 试卷提交失败
- `IndexOutOfRange`: 索引超出范围
- `SubjectParseFailed`: 科目解析失败

### 6. ConfigError (配置错误)

- `EnvVarParseFailed`: 环境变量解析失败
- `EnvVarNotFound`: 环境变量不存在

## 使用方式

### 基本用法

```rust
use crate::error::{AppError, AppResult};

// 使用 AppResult 作为返回类型
fn my_function() -> AppResult<String> {
    // 创建错误
    Err(AppError::Business(BusinessError::EmptyPaperId))
}

// 使用便捷构造函数
fn read_file(path: &str) -> AppResult<String> {
    std::fs::read_to_string(path)
        .map_err(|e| AppError::file_read_failed(path, e))
}
```

### 与 anyhow 集成

由于 `AppError` 实现了 `std::error::Error`，它可以自动转换为 `anyhow::Error`：

```rust
use anyhow::Result;

fn my_function() -> Result<String> {
    // AppError 可以自动转换为 anyhow::Error
    Err(AppError::Business(BusinessError::EmptyPaperId))?
}
```

### 从常见错误类型转换

系统已经提供了从常见错误类型的自动转换：

- `chromiumoxide::error::CdpError` → `AppError::Browser(BrowserError::ScriptExecutionFailed)`
- `serde_json::Error` → `AppError::Api(ApiError::JsonParseFailed)`
- `toml::de::Error` → `AppError::File(FileError::TomlParseFailed)`
- `std::io::Error` → `AppError::File(FileError::ReadFailed)`

### 错误匹配和处理

```rust
match result {
    Ok(value) => println!("成功: {}", value),
    Err(AppError::Browser(BrowserError::ConnectionFailed { port, .. })) => {
        eprintln!("无法连接到浏览器端口: {}", port);
    }
    Err(AppError::Api(ApiError::RateLimited { endpoint, .. })) => {
        eprintln!("API 频率限制: {}", endpoint);
        // 可以在这里实现重试逻辑
    }
    Err(e) => {
        eprintln!("其他错误: {}", e);
    }
}
```

## 重构建议

### 第一步：替换 unwrap() 和 expect()

将所有的 `unwrap()` 和 `expect()` 替换为适当的错误处理：

```rust
// 之前
let page_id = page_data.page_id.as_ref().unwrap();

// 之后
let page_id = page_data.page_id.as_ref()
    .ok_or(AppError::Business(BusinessError::EmptyPaperId))?;
```

### 第二步：使用 AppResult 替代 anyhow::Result

在业务逻辑层使用 `AppResult<T>`，在应用层可以继续使用 `anyhow::Result<T>`：

```rust
// domain/paper.rs
use crate::error::{AppError, AppResult};

pub async fn process_paper(...) -> AppResult<bool> {
    // ...
}

// app/app.rs
use anyhow::Result;

pub async fn run(&self) -> Result<()> {
    // AppResult 可以自动转换为 Result
    domain::paper::process_paper(...).await?;
    Ok(())
}
```

### 第三步：添加上下文信息

使用 `with_context` 或自定义错误构造函数添加上下文：

```rust
// 使用 with_context
let content = fs::read_to_string(path)
    .map_err(|e| AppError::file_read_failed(path, e))
    .with_context(|| format!("读取配置文件失败: {}", path))?;

// 或使用自定义错误
let page = browser.new_page(url).await
    .map_err(|e| AppError::Browser(BrowserError::PageCreationFailed {
        source: Box::new(e),
    }))?;
```

## 优势

1. **类型安全**: 明确的错误类型，编译时检查
2. **易于调试**: 结构化的错误信息，包含上下文
3. **易于测试**: 可以精确匹配错误类型
4. **易于维护**: 错误处理逻辑集中管理
5. **向后兼容**: 可以与 anyhow 无缝集成

## 下一步

1. 逐步重构现有代码，使用 `AppError` 替代 `anyhow::Error`
2. 移除所有 `unwrap()` 和 `expect()` 调用
3. 在关键路径添加详细的错误上下文
4. 添加错误恢复逻辑（如重试机制）

