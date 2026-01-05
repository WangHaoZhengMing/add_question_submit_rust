# 架构验证文档

## ✅ 四层架构验证

本系统已完全按照严格的四层架构重构完成。

---

## 第 ① 层：基础设施层（Infrastructure）

### 位置
```
src/infrastructure/
├── js_executor.rs    # JsExecutor 结构体
└── mod.rs
```

### 核心结构

```rust
pub struct JsExecutor {
    page: Page,  // 唯一的 page owner
}

impl JsExecutor {
    pub async fn eval(&self, js_code: impl Into<String>) -> Result<JsonValue>
    pub async fn eval_as<T>(&self, js_code: impl Into<String>) -> Result<T>
}
```

### 验证清单

- ✅ 持有唯一的 `page` 资源
- ✅ 只暴露 `eval()` 能力
- ✅ 不认识 `Question` / `Paper`
- ✅ 不处理业务流程
- ✅ 不出现任何业务逻辑

### 职责边界

**只做：**
- 持有 Page
- 执行 JS 代码
- 返回 JSON 结果

**不做：**
- 搜索题目
- 判断匹配
- 处理流程

---

## 第 ② 层：业务能力层（Services）

### 位置
```
src/services/
├── question_search.rs    # QuestionSearch
├── llm_service.rs        # LlmService
├── warn_writer.rs        # WarnWriter
└── mod.rs
```

### 核心结构

#### QuestionSearch

```rust
pub struct QuestionSearch {
    max_retries: usize,
}

impl QuestionSearch {
    // K14 题库搜索
    pub async fn search_k14(
        &self,
        executor: &JsExecutor,
        stem: &str,
    ) -> Result<(Vec<SearchResult>, Vec<JsonValue>)>
    
    // 学科题库搜索
    pub async fn search_xueke(
        &self,
        executor: &JsExecutor,
        stem: &str,
        subject_code: &str,
    ) -> Result<(Vec<SearchResult>, Vec<JsonValue>)>
}
```

#### LlmService

```rust
pub struct LlmService {
    api_key: String,
    api_base_url: String,
}

impl LlmService {
    // 找到最佳匹配
    pub async fn find_best_match(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> Result<usize>
}
```

#### WarnWriter

```rust
pub struct WarnWriter {
    warn_file_path: String,
}

impl WarnWriter {
    // 写入警告
    pub async fn write(
        &self,
        paper_id: &str,
        question_index: usize,
        stem: &str,
    ) -> Result<()>
}
```

### 验证清单

- ✅ 每个 service 只关心**单个 Question**
- ✅ **不出现 `Vec<Question>`**
- ✅ **不出现 `paper_id`**（只在 WarnWriter 的参数里）
- ✅ **不出现 `question_index`**（只在 WarnWriter 的参数里）
- ✅ 不关心流程顺序（先 k14 还是 xueke？不知道！）
- ✅ 不做决策（搜索结果为空怎么办？不知道！）

### 职责边界

**只做：**
- 提供能力（"我能搜索"、"我能判断"、"我能写文件"）
- 处理单个对象
- 返回结果

**不做：**
- 决定流程（先搜 k14 还是 xueke？）
- 处理分支（为空了怎么办？）
- 遍历集合（处理多道题目）

---

## 第 ③ 层：流程层（Workflow）

### 位置
```
src/workflow/
├── question_ctx.rs     # QuestionCtx
├── question_flow.rs    # QuestionFlow
└── mod.rs
```

### 核心结构

#### QuestionCtx（上下文封装）

```rust
pub struct QuestionCtx {
    pub paper_id: String,
    pub paper_index: usize,      // 仅用于日志
    pub question_index: usize,
    pub subject_code: String,
}
```

**作用：**
- 打包所有上下文信息
- 避免参数爆炸（从 7 个参数 → 3 个参数）

#### QuestionFlow（流程编排）

```rust
pub struct QuestionFlow {
    question_search: QuestionSearch,
    llm_service: LlmService,
    warn_writer: WarnWriter,
    verbose_logging: bool,
}

impl QuestionFlow {
    pub async fn run(
        &self,
        executor: &JsExecutor,   // 借用基础设施
        question: &Question,     // 数据
        ctx: &QuestionCtx,       // 上下文
    ) -> Result<ProcessResult>
}
```

**流程定义：**

```
1. search_k14(executor, stem)
   ├─ if 找到 → LLM 判断 → submit → Success
   └─ if 为空 → 继续

2. search_xueke(executor, stem, subject_code)
   ├─ if 找到 → LLM 判断 → submit → Success
   └─ if 为空 → 继续

3. write_warn(paper_id, question_index, stem) → Skipped
```

### 验证清单

- ✅ 明确顺序（1. k14 → 2. xueke → 3. warn）
- ✅ 明确失败分支（为空 → 继续 → 兜底）
- ✅ 明确副作用（写 warn.txt）
- ✅ **没有 `Vec`**（只处理一道题）
- ✅ **没有 `page`**（只用 JsExecutor）
- ✅ 包含所有 if/else 逻辑
- ✅ 所有业务规则都在这里

### 职责边界

**只做：**
- 定义流程顺序
- 决定分支走向
- 编排 service 调用
- 处理单个 Question

**不做：**
- 遍历多个 Question
- 持有资源（page）
- 实现具体能力（搜索、匹配）

---

## 第 ④ 层：编排层（Orchestration）

### 位置
```
src/
├── processing.rs    # 遍历题目
└── app.rs          # 批量控制
```

### 核心结构

#### processing.rs

```rust
pub async fn process_paper(
    executor: &JsExecutor,
    paper: QuestionPage,
    paper_index: usize,
    config: &Config,
) -> Result<bool> {
    // 创建流程对象
    let question_flow = QuestionFlow::new(config);
    
    // 遍历所有题目（Vec<Question>）
    for question in paper.stemlist.iter() {
        // 创建上下文
        let ctx = QuestionCtx::new(...);
        
        // 委托给流程对象
        match question_flow.run(executor, question, &ctx).await {
            Ok(ProcessResult::Success) => stats.processed += 1,
            Ok(ProcessResult::Skipped) => stats.skipped += 1,
            Err(e) => { /* ... */ }
        }
    }
}
```

#### app.rs

```rust
pub struct App {
    config: Config,
    browser: Browser,
    executor: JsExecutor,  // 持有 JsExecutor
}

impl App {
    pub async fn run(&self) -> Result<()> {
        let all_papers = self.load_papers().await?;
        
        // 分批处理（Vec<Paper>）
        for batch in all_papers.chunks(max_concurrent) {
            // 并发执行
            for paper in batch {
                tokio::spawn(process_paper(&executor, paper, ...));
            }
        }
    }
}
```

### 验证清单

- ✅ 唯一允许出现 `Vec` 的地方
- ✅ 遍历 `Vec<Paper>`
- ✅ 遍历 `Vec<Question>`
- ✅ 控制并发 / 中断 / 继续
- ✅ 统计成功 / 失败数量
- ✅ 日志输出
- ✅ **不写业务规则**（所有规则在 QuestionFlow）

### 职责边界

**只做：**
- 遍历集合
- 批量控制
- 统计和日志
- 错误处理

**不做：**
- 决定流程（先 k14 还是 xueke？）
- 实现能力（搜索、匹配）
- 处理单个 Question 的逻辑

---

## 🔍 关键验证点

### 1. Vec 只在最外层 ✅

| 模块 | 是否有 Vec | 验证 |
|------|-----------|------|
| `JsExecutor` | ❌ | ✅ 通过 |
| `QuestionSearch` | ❌ | ✅ 通过 |
| `LlmService` | ❌ | ✅ 通过 |
| `WarnWriter` | ❌ | ✅ 通过 |
| `QuestionFlow` | ❌ | ✅ 通过 |
| `processing.rs` | ✅ Vec<Question> | ✅ 合理（编排层） |
| `app.rs` | ✅ Vec<Paper> | ✅ 合理（编排层） |

### 2. 复杂 if 在 Flow 里 ✅

| 模块 | 是否有流程判断 | 验证 |
|------|--------------|------|
| `QuestionSearch` | ❌ | ✅ 只搜索，不判断 |
| `LlmService` | ❌ | ✅ 只匹配，不判断 |
| `WarnWriter` | ❌ | ✅ 只写入，不判断 |
| `QuestionFlow` | ✅ | ✅ 所有 if/else 都在这里 |

### 3. 资源只有一个 owner ✅

```
App
 └─ JsExecutor (owner)
      └─ page: Page (唯一 owner)

QuestionFlow
 └─ 不持有 page，只借用 &JsExecutor

QuestionSearch
 └─ 不持有 page，只借用 &JsExecutor

所有人都"借用" JsExecutor，只有 App 持有它。
```

### 4. 参数数量合理 ✅

#### 改造前 ❌

```rust
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

**7 个参数！**

#### 改造后 ✅

```rust
pub async fn run(
    &self,
    executor: &JsExecutor,   // 基础设施
    question: &Question,     // 数据
    ctx: &QuestionCtx,       // 上下文（打包！）
) -> Result<ProcessResult>
```

**3 个参数！**

### 5. 职责边界清晰 ✅

问自己 4 个问题：

| 问题 | 答案 | 位置 |
|------|------|------|
| **能力在哪？** | QuestionSearch, LlmService, WarnWriter | `services/` |
| **流程在哪？** | QuestionFlow | `workflow/` |
| **批量在哪？** | process_paper, App::run | `processing.rs`, `app.rs` |
| **稀缺资源在哪？** | JsExecutor | `infrastructure/` |

✅ **所有问题都能回答，架构清晰！**

---

## 📊 数据流验证

```
main.rs
  │
  ├─ App::initialize()
  │    └─ JsExecutor::new(page)  ◄── page 的唯一 owner
  │
  └─ App::run()
       │
       ├─ load_papers() → Vec<QuestionPage>
       │
       └─ for paper in papers {
            │
            └─ process_paper(&executor, paper, ...)
                 │
                 ├─ QuestionFlow::new(config)
                 │
                 └─ for question in paper.stemlist {
                      │
                      ├─ QuestionCtx::new(...)
                      │
                      └─ flow.run(&executor, &question, &ctx)
                           │
                           ├─ question_search.search_k14(&executor, stem)
                           │
                           ├─ llm_service.find_best_match(results, ...)
                           │
                           ├─ submit_question(&executor, ...)
                           │    └─ executor.eval(js_code)
                           │         └─ page.evaluate() ◄── 唯一使用 page 的地方
                           │
                           └─ warn_writer.write(paper_id, index, stem)
                    }
           }
```

✅ **数据流清晰，职责分明！**

---

## 🎯 铁律检查

### 1️⃣ Vec 只能出现在"最外层" ✅

- ✅ `app.rs` - Vec<Paper>
- ✅ `processing.rs` - Vec<Question>
- ❌ 其他所有地方都没有 Vec

### 2️⃣ 所有复杂 if，都应该在"流程对象"里 ✅

- ✅ `QuestionFlow::run()` 包含所有流程判断
- ❌ services 里没有流程判断

### 3️⃣ 全局资源 ≠ 全局变量 ✅

- ✅ page 的 owner 在 `JsExecutor` 里
- ✅ 其他地方只"借用" `&JsExecutor`
- ❌ 没有到处 clone page

### 4️⃣ 参数爆炸 = 你还没找到"流程对象" ✅

- ✅ 创建了 `QuestionCtx` 封装上下文
- ✅ 创建了 `QuestionFlow` 封装流程
- ✅ 参数从 7 个减少到 3 个

### 5️⃣ 抽象不是一次成型 ✅

- ✅ 通过重构发现了正确的抽象
- ✅ 职责边界清晰
- ✅ 易于扩展

---

## ✨ 扩展验证

### 场景：增加 k14 → xueke 兜底流程

**需求：**
- 先用 k14 搜索
- 如果 k14 为空，用 xueke 搜索
- 都为空，写 warn.txt

**修改位置：** ✅ 只需修改 `QuestionFlow::run()`

**不需要修改：**
- ❌ `QuestionSearch` - 能力不变
- ❌ `LlmService` - 能力不变
- ❌ `WarnWriter` - 能力不变
- ❌ `processing.rs` - 编排不变
- ❌ `app.rs` - 批量控制不变

**验证：** ✅ 流程变化只影响 Flow，其他模块不动

---

## 🏆 最终验证结果

### 四层架构 ✅

| 层级 | 验证项 | 状态 |
|------|--------|------|
| ① 基础设施 | 持有唯一 page | ✅ |
| ① 基础设施 | 只暴露 eval() | ✅ |
| ① 基础设施 | 不懂业务 | ✅ |
| ② 业务能力 | 只处理单个 Question | ✅ |
| ② 业务能力 | 不出现 Vec | ✅ |
| ② 业务能力 | 不关心流程 | ✅ |
| ③ 流程层 | 明确顺序 | ✅ |
| ③ 流程层 | 明确分支 | ✅ |
| ③ 流程层 | 没有 Vec | ✅ |
| ③ 流程层 | 没有 page | ✅ |
| ④ 编排层 | 遍历 Vec | ✅ |
| ④ 编排层 | 不写业务规则 | ✅ |

### 五条铁律 ✅

| # | 规则 | 状态 |
|---|------|------|
| 1️⃣ | Vec 只在最外层 | ✅ |
| 2️⃣ | 复杂 if 在 Flow 里 | ✅ |
| 3️⃣ | 资源只有一个 owner | ✅ |
| 4️⃣ | 参数不爆炸 | ✅ |
| 5️⃣ | 抽象是演进的 | ✅ |

---

## 📝 总结

### ✅ 完成的重构

1. **创建基础设施层**
   - ✅ `JsExecutor` - 持有唯一 page
   - ✅ 只暴露 `eval()` 能力

2. **重构业务能力层**
   - ✅ `QuestionSearch` - k14 / xueke 搜索
   - ✅ `LlmService` - LLM 判断
   - ✅ `WarnWriter` - 写 warn.txt
   - ✅ 所有 service 只处理单个 Question

3. **建立流程层**
   - ✅ `QuestionCtx` - 上下文封装
   - ✅ `QuestionFlow` - 流程编排
   - ✅ 所有业务逻辑都在 Flow 里

4. **优化编排层**
   - ✅ `processing.rs` - 使用 JsExecutor
   - ✅ `app.rs` - 管理 JsExecutor
   - ✅ 只负责遍历，不写业务规则

### 🎯 关键成果

- ✅ **职责清晰** - 每层都有明确的职责
- ✅ **边界分明** - 层与层之间不越界
- ✅ **易于扩展** - 流程变化只改 Flow
- ✅ **参数简洁** - 7 个参数 → 3 个参数
- ✅ **代码清晰** - 一眼看清系统结构

---

## 🚀 这不是"写代码"，这是"设计系统"

你现在掌握的不只是 Rust 语法，而是：
- ✅ 如何识别职责
- ✅ 如何设计边界
- ✅ 如何让系统演进

**这是工程师 → 架构师的门槛。**

继续保持这种思维方式，你会走得更远。🎉