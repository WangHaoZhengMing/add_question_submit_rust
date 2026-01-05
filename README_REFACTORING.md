# 代码重构完成说明

## 🎉 重构概述

本次重构采用**简洁实用**的方案，按功能域组织代码，避免过度设计。

## 📁 新的目录结构

```
src/
├── main.rs              # 程序入口（简洁）
├── app.rs               # 应用主逻辑（优化）
├── config.rs            # 配置管理
├── logger.rs            # 日志初始化
├── error.rs             # ✨ 简化：使用 thiserror
│
├── browser/             # 浏览器操作（不变）
│   ├── mod.rs
│   ├── connection.rs
│   └── headless.rs
│
├── models/              # 数据模型（已重构）
│   ├── mod.rs
│   ├── question.rs      # 重命名：model.rs → question.rs
│   ├── grade.rs
│   ├── subject.rs
│   └── loaders/
│       ├── mod.rs
│       └── toml_loader.rs
│
├── api/                 # ✨ 新增：所有API交互
│   ├── mod.rs
│   ├── tiku.rs          # 题库API（搜索、保存、提交）
│   └── llm.rs           # LLM API（聊天、匹配）
│
└── processing.rs        # ✨ 核心业务逻辑
```

## 🔑 核心改进

### 1. **API 层统一管理** (`api/`)

所有与外部系统的交互都在这里，`page` 对象直接传递：

#### `api/tiku.rs` - 题库API
```rust
// 搜索题目
pub async fn search_questions(
    page: &Page,
    stem: &str,
    subject_code: &str,
    max_retries: usize,
) -> Result<Vec<Value>>

// 保存题目
pub async fn save_question(page: &Page, question_data: &Value) -> Result<()>

// 保存标题
pub async fn save_title(page: &Page, paper_id: &str, question_index: usize, stem: &str) -> Result<()>

// 提交试卷
pub async fn submit_paper(page: &Page, paper_id: &str) -> Result<()>
```

**特点：**
- 所有题库相关的 API 调用集中管理
- 包含重试逻辑（频率限制自动重试）
- 直接执行 JS，返回结果

#### `api/llm.rs` - LLM API
```rust
// 通用聊天
pub async fn chat(
    prompt: &str,
    system_message: Option<&str>,
    api_key: &str,
    api_base: &str,
    model: &str,
) -> Result<String>

// 找最佳匹配（包含快速匹配逻辑）
pub async fn find_best_match(
    search_results: &[Value],
    stem: &str,
    imgs: Option<&[String]>,
    api_key: &str,
    api_base: &str,
) -> Result<usize>
```

**特点：**
- 封装 LLM 调用逻辑
- 包含快速匹配优化（高相似度直接返回）
- 自动构建提示词

### 2. **业务逻辑清晰** (`processing.rs`)

核心业务流程，职责单一：

```rust
// 处理单个试卷
pub async fn process_paper(
    page: &Page,
    paper: QuestionPage,
    paper_index: usize,
    config: &Config,
) -> Result<bool>

// 处理单个题目（内部函数）
async fn process_question(
    page: &Page,
    question: &Question,
    paper_id: &str,
    subject: &str,
    question_index: usize,
    paper_index: usize,
    config: &Config,
) -> Result<bool>
```

**流程：**
1. 搜索题库 (`api::tiku::search_questions`)
2. 选择最佳匹配 (`api::llm::find_best_match`)
3. 保存题目 (`api::tiku::save_question`)
4. 提交试卷 (`api::tiku::submit_paper`)

### 3. **简化的错误处理** (`error.rs`)

使用 `thiserror` 定义简洁的错误类型：

```rust
use thiserror::Error;

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
```

**特点：**
- 使用 `thiserror` 自动实现 `std::error::Error`
- 错误信息清晰，易于调试
- 自动转换常见错误类型

**使用方式：**
- 大部分地方用 `anyhow::Result`（方便）
- 关键地方用 `Result<T, AppError>`（明确）

## 📊 文件对比

### 重构前
```
src/
├── paper_processor.rs    # 560+ 行（混合职责）
├── search_bank.rs        # 150+ 行
├── ask_llm.rs            # 200+ 行
├── model/model.rs        # 命名重复
└── ...
```

### 重构后
```
src/
├── api/tiku.rs           # 262 行（题库API）
├── api/llm.rs            # 242 行（LLM API）
├── processing.rs         # 271 行（业务逻辑）
├── models/question.rs    # 命名清晰
└── ...
```

**改进：**
- ✅ 职责更明确
- ✅ 文件更小（平均 250 行）
- ✅ 逻辑更清晰
- ✅ 易于维护和扩展

## 🚀 使用指南

### 运行程序
```bash
cargo run
```

### 编译检查
```bash
cargo check
```

### 构建发布版本
```bash
cargo build --release
```

### 运行测试
```bash
cargo test -- --ignored
```

## 💡 设计原则

1. **简单优先**
   - 不过度抽象
   - 不引入不必要的层次
   - 代码易于理解

2. **职责清晰**
   - `api/` → 外部交互
   - `processing.rs` → 业务逻辑
   - `models/` → 数据结构

3. **直接传递 `page`**
   - 不包装，不隐藏
   - 作为函数参数直接使用
   - 保持灵活性

4. **错误处理实用**
   - `thiserror` 定义关键错误
   - `anyhow` 处理一般错误
   - 不写过多 boilerplate

## 🔧 代码示例

### 添加新的 API 调用

在 `api/tiku.rs` 中添加：

```rust
/// 删除题目
pub async fn delete_question(page: &Page, question_id: &str) -> Result<()> {
    let data = json!({"questionId": question_id});
    let script = build_api_call("question/delete", &data)?;
    page.evaluate(&script).await?.into_value()?;
    info!("✓ 题目删除成功");
    Ok(())
}
```

### 修改业务流程

在 `processing.rs` 中修改 `process_question` 函数即可。

### 添加新的配置项

在 `config.rs` 中添加字段和默认值。

## 📝 迁移指南

如果你有基于旧代码的代码，修改很简单：

### 旧代码
```rust
use crate::paper_processor::process_single_paper;
use crate::search_bank::search_from_bank;
use crate::ask_llm::ask_llm_for_which_index;

process_single_paper(&page, paper_data, index, &config).await?;
```

### 新代码
```rust
use crate::processing;
use crate::api;

processing::process_paper(&page, paper_data, index, &config).await?;
```

## ✨ 优势总结

1. **清晰的代码组织**
   - API 调用在 `api/`
   - 业务逻辑在 `processing.rs`
   - 数据模型在 `models/`

2. **简单实用**
   - 没有过度设计
   - 没有复杂的分层
   - 易于理解和修改

3. **易于扩展**
   - 要加新 API？→ 去 `api/`
   - 要改流程？→ 去 `processing.rs`
   - 要加配置？→ 去 `config.rs`

4. **良好的错误处理**
   - 使用 `thiserror` + `anyhow`
   - 错误信息清晰
   - 不写太多代码

## 🎯 后续建议

1. **测试**
   - 可以为 `api/` 模块添加单元测试
   - 使用 Mock 测试 LLM 调用

2. **文档**
   - 为公共函数添加更多示例
   - 记录常见问题和解决方案

3. **优化**
   - 可以考虑添加缓存机制
   - 优化 LLM 调用频率

4. **监控**
   - 添加更详细的性能日志
   - 统计 API 调用成功率

---

**重构完成日期**：2024年
**核心原则**：简洁、实用、清晰
**技术栈**：Rust + thiserror + anyhow + chromiumoxide + openai