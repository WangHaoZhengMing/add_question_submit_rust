# 快速参考指南

## 📁 项目结构一览

```
src/
├── api/              ← 所有 API 调用都在这里
│   ├── tiku.rs      ← 题库 API（搜索、保存、提交）
│   └── llm.rs       ← LLM API（聊天、匹配）
├── processing.rs     ← 核心业务逻辑
├── models/           ← 数据结构
├── browser/          ← 浏览器操作
├── config.rs         ← 配置
└── error.rs          ← 错误定义
```

## 🔑 核心概念

### `page` 对象
- **什么是 `page`？** 浏览器页面对象，用于执行 JS
- **如何使用？** 直接作为函数参数传递
- **在哪使用？** 主要在 `api/` 模块中

### API 层 (`api/`)
**职责：** 与外部系统交互，执行 JS 脚本

```rust
// api/tiku.rs
api::tiku::search_questions(page, stem, subject_code, 50).await?
api::tiku::save_question(page, &question_data).await?
api::tiku::submit_paper(page, paper_id).await?

// api/llm.rs
api::llm::find_best_match(results, stem, imgs, api_key, api_base).await?
```

### 业务逻辑 (`processing.rs`)
**职责：** 协调 API 调用，实现业务流程

```rust
processing::process_paper(page, paper, index, config).await?
```

## 🚀 常用操作

### 1. 运行程序
```bash
cargo run
```

### 2. 检查编译
```bash
cargo check
```

### 3. 构建发布版
```bash
cargo build --release
```

### 4. 查看日志
日志保存在 `output.txt`

## 🔧 如何修改

### 添加新的题库 API

**位置：** `src/api/tiku.rs`

```rust
/// 你的新 API
pub async fn your_new_api(page: &Page, param: &str) -> Result<()> {
    let data = json!({"key": param});
    let script = build_api_call("your/endpoint", &data)?;
    page.evaluate(&script).await?.into_value()?;
    Ok(())
}
```

### 修改业务流程

**位置：** `src/processing.rs`

找到 `process_question` 函数，修改业务逻辑：

```rust
async fn process_question(...) -> Result<bool> {
    // 1. 搜索
    let results = api::tiku::search_questions(...).await?;
    
    // 2. 匹配
    let index = api::llm::find_best_match(...).await?;
    
    // 3. 保存
    api::tiku::save_question(...).await?;
    
    Ok(true)
}
```

### 添加配置项

**位置：** `src/config.rs`

```rust
pub struct Config {
    // 现有配置...
    pub your_new_config: String,  // ← 添加新字段
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // 现有默认值...
            your_new_config: "default_value".to_string(),  // ← 设置默认值
        }
    }
}
```

### 添加错误类型

**位置：** `src/error.rs`

```rust
#[derive(Error, Debug)]
pub enum AppError {
    // 现有错误...
    
    #[error("你的错误描述: {0}")]
    YourError(String),  // ← 添加新错误
}
```

## 💡 核心流程

### 处理一个试卷的流程

```
1. app.rs (加载试卷)
   ↓
2. processing::process_paper (遍历题目)
   ↓
3. 对每个题目：
   a. api::tiku::search_questions (搜索)
   b. api::llm::find_best_match (匹配)
   c. api::tiku::save_question (保存)
   ↓
4. api::tiku::submit_paper (提交试卷)
```

### 数据流向

```
TOML 文件 → models::QuestionPage → processing → api → 浏览器 JS
```

## 🐛 常见问题

### Q: 如何添加日志？
```rust
use tracing::{info, warn, error};

info!("正常信息");
warn!("警告信息");
error!("错误信息");
```

### Q: 如何处理错误？
```rust
// 使用 ? 传播错误
let result = some_function().await?;

// 使用 context 添加上下文
let result = some_function()
    .await
    .context("描述这个操作")?;

// 返回自定义错误
anyhow::bail!("自定义错误消息");
```

### Q: 如何在 page 上执行 JS？
```rust
let script = r#"
    (async () => {
        // 你的 JS 代码
        return result;
    })()
"#;

let result: serde_json::Value = page
    .evaluate(script)
    .await?
    .into_value()?;
```

### Q: 如何读取配置？
```rust
// 配置在 main.rs 中初始化
let config = Config::from_env();

// 在函数中使用
fn my_function(config: &Config) {
    let url = &config.target_url;
    let token = &config.tiku_token;
}
```

## 📚 依赖说明

| 包 | 用途 |
|---|---|
| `anyhow` | 简化错误处理 |
| `thiserror` | 定义错误类型 |
| `chromiumoxide` | 浏览器自动化 |
| `openai` | LLM API 调用 |
| `serde_json` | JSON 处理 |
| `tokio` | 异步运行时 |
| `tracing` | 日志记录 |

## 🎯 设计原则

1. **简单优先** - 不过度设计
2. **职责清晰** - 每个模块只做一件事
3. **直接传递** - `page` 作为参数，不包装
4. **错误清晰** - 使用 `thiserror` + `anyhow`

## 📝 代码风格

```rust
// ✅ 好的做法
pub async fn clear_function_name(
    page: &Page,
    param: &str,
) -> Result<ReturnType> {
    // 简单直接的实现
    Ok(result)
}

// ❌ 避免
pub async fn vague_name(p: &Page, x: &str) -> Result<Value> {
    // 复杂嵌套的实现
}
```

## 🔍 调试技巧

### 1. 查看详细日志
在 `config.rs` 中设置：
```rust
verbose_logging: true
```

### 2. 打印中间结果
```rust
dbg!(&search_results);  // 调试打印
println!("{:#?}", data);  // 格式化打印
```

### 3. 检查 JS 执行结果
```rust
let result: Value = page.evaluate(script).await?.into_value()?;
println!("JS 返回: {}", result);
```

## 🎓 学习路径

1. **先看** `processing.rs` - 了解业务流程
2. **再看** `api/tiku.rs` 和 `api/llm.rs` - 了解 API 调用
3. **最后看** `app.rs` - 了解整体架构

## 📞 需要帮助？

- 查看 `README_REFACTORING.md` 获取详细说明
- 阅读代码注释
- 使用 `cargo doc --open` 生成文档

---

**记住：** 代码应该简单、清晰、易于理解！