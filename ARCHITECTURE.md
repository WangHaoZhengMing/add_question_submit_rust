# 架构设计文档

## 一、系统本质

**一句话定义：**

在一个唯一的 JS 执行环境上，按确定流程，逐张试卷、逐题执行"搜索 → 判断 → 兜底"的工作流系统。

这是一个 **流程驱动（workflow-driven）** 系统，而非：
- CRUD 系统
- service 拼装
- if/else 业务判断

---

## 二、四层架构

### ① 基础设施层（Infrastructure）

**目标：** 持有稀缺资源，只暴露"能力"，不懂业务。

```
src/browser/
├── connection.rs    # 连接浏览器
├── headless.rs      # 无头浏览器管理
└── mod.rs

关键特征：
- 持有唯一的 Page 资源
- 不认识 Question / Paper
- 不处理流程
- 只提供"执行 JS"的能力
```

---

### ② 业务能力层（Domain Services）

**目标：** 描述"我能做什么"，只处理"一道题"。

```
src/services/
├── search_service.rs      # 搜索能力（k14 / xueke）
├── matching_service.rs    # LLM 匹配能力
└── mod.rs

关键特征：
- 每个 service 只关心单个 Question
- 不出现 Vec
- 不出现 index / paper_id
- 不关心流程顺序
- 只暴露"我能搜索"、"我能匹配"
```

**示例：SearchService**

```rust
pub struct SearchService {
    tiku_client: TikuClient,
}

impl SearchService {
    // 只处理一道题的搜索，不管流程
    pub async fn search(
        &self,
        page: &Page,
        stem: &str,
        subject_code: &str,
    ) -> Result<Vec<SearchResult>> {
        // 只负责搜索，不管"搜索结果为空怎么办"
    }
}
```

---

### ③ 流程层（Workflow / Process）

**这是系统的核心！**

**目标：** 明确"一道题"的完整处理流程。

```
src/workflow/
├── question_ctx.rs      # QuestionCtx - 我正在处理哪张卷子的第几题
├── question_flow.rs     # QuestionFlow - 完整流程
└── mod.rs

关键特征：
- 明确顺序：search → LLM → submit
- 明确分支：if 为空 → skip / if 找到 → submit
- 明确副作用：写 warn.txt
- 没有 Vec（只处理一道题）
- 不持有 page（只借用）
```

**QuestionCtx：上下文封装**

```rust
pub struct QuestionCtx {
    pub paper_id: String,        // 试卷ID
    pub paper_index: usize,      // 仅用于日志
    pub question_index: usize,   // 题目索引
    pub subject_code: String,    // 科目代码
}
```

**为什么需要 QuestionCtx？**
- 避免参数爆炸（7 个参数 → 1 个上下文）
- 上下文信息集中管理
- 日志/调试信息统一

**QuestionFlow：流程编排**

```rust
pub struct QuestionFlow {
    search_service: SearchService,
    matching_service: MatchingService,
}

impl QuestionFlow {
    pub async fn run(
        &self,
        page: &Page,           // 唯一稀缺资源（借用）
        question: &Question,   // 数据
        ctx: &QuestionCtx,     // 上下文
    ) -> Result<ProcessResult> {
        // 1. 搜索
        let results = self.search_service.search(...).await?;
        
        // 2. 分支：为空？
        if results.is_empty() {
            self.write_warn(ctx, question).await?;
            return Ok(ProcessResult::Skipped);
        }
        
        // 3. LLM 判断
        let best = self.matching_service.find_best(...).await?;
        
        // 4. 提交
        self.submit(page, best, ctx).await
    }
}
```

**为什么需要 QuestionFlow？**
- 流程逻辑集中在一处
- 不会和"能力"混淆
- 易于扩展新流程（如：先 k14，再 xueke）

---

### ④ 批处理 / 编排层（Orchestration）

**目标：** 唯一允许出现 Vec 的地方。

```
src/processing.rs    # 遍历 Vec<Paper> → Vec<Question>
src/app.rs           # 应用入口，批次控制
```

**职责：**
- 遍历试卷和题目
- 控制中断 / 继续
- 统计 / 日志
- **不写业务规则**（所有业务规则在 QuestionFlow 里）

**示例：process_paper**

```rust
pub async fn process_paper(
    page: &Page,
    paper: QuestionPage,
    paper_index: usize,
    config: &Config,
) -> Result<bool> {
    let question_flow = QuestionFlow::new(config);
    
    // 遍历所有题目
    for (index, question) in paper.stemlist.iter().enumerate() {
        let ctx = QuestionCtx::new(
            paper_id.to_string(),
            paper_index,
            index + 1,
            subject_code.clone(),
        );
        
        // 委托给流程对象
        match question_flow.run(page, question, &ctx).await {
            Ok(ProcessResult::Success) => stats.processed += 1,
            Ok(ProcessResult::Skipped) => stats.skipped += 1,
            Err(e) => { /* 错误处理 */ }
        }
    }
    
    Ok(true)
}
```

---

## 三、架构对比（改造前 vs 改造后）

### 改造前的问题

```rust
// ❌ 参数爆炸
async fn process_question(
    page: &Page,
    question: &Question,
    paper_id: &str,
    subject: &str,
    question_index: usize,
    paper_index: usize,
    config: &Config,
) -> Result<bool>

// ❌ 流程和能力混在一起
impl QuestionService {
    pub async fn process_question(...) -> Result<ProcessResult> {
        // 搜索（能力）
        let results = self.search_service.search(...).await?;
        
        // 流程判断（混在一起！）
        if results.is_empty() {
            return Ok(ProcessResult::Skipped);
        }
        
        // LLM 判断（能力）
        let best = self.matching_service.find_best(...).await?;
        
        // 提交（能力）
        self.submit(...).await?;
    }
}
```

**问题：**
1. `QuestionService` 既是"能力"又是"流程"
2. 参数太多，难以维护
3. 流程逻辑分散
4. 难以扩展（如果要加 k14 → xueke 的兜底？）

---

### 改造后的优势

```rust
// ✅ 上下文封装
let ctx = QuestionCtx::new(
    paper_id.to_string(),
    paper_index,
    question_index,
    subject_code.clone(),
);

// ✅ 流程对象
let question_flow = QuestionFlow::new(config);
question_flow.run(page, question, &ctx).await?;
```

**优势：**
1. **职责清晰**
   - `SearchService` = 只搜索
   - `MatchingService` = 只匹配
   - `QuestionFlow` = 只管流程

2. **参数简洁**
   - 7 个参数 → 3 个参数（page, question, ctx）
   - 上下文信息统一管理

3. **流程明确**
   - 所有 if/else 都在 `QuestionFlow::run()` 里
   - 一眼看清完整流程

4. **易于扩展**
   - 要加 k14 兜底？在 `QuestionFlow::run()` 里加一个分支
   - 要加 warn.txt？在 `QuestionFlow` 里加一个方法
   - 不会影响 service

---

## 四、五条铁律

### 1️⃣ Vec 只能出现在"最外层"

- ✅ `app.rs` / `processing.rs` - 遍历 Vec<Paper>
- ❌ `QuestionFlow` - 不能有 Vec<Question>
- ❌ `SearchService` - 不能有 Vec<Question>

**为什么？**
- 一旦中间层有 Vec，就说明职责不清
- service 应该只处理"单个"，不管"多个"

---

### 2️⃣ 所有复杂 if，都应该在"流程对象"里

- ✅ `QuestionFlow::run()` - 包含所有流程判断
- ❌ `SearchService` - 不应该有"如果为空就跳过"
- ❌ `MatchingService` - 不应该有"如果找到就提交"

**为什么？**
- service 只暴露"能力"，不做"决策"
- 决策 = 流程，应该在 Flow 里

---

### 3️⃣ 全局资源 ≠ 全局变量

- ✅ `page` 的 owner 在 `App` 里
- ✅ 其他地方只"借用"能力（`&Page`）
- ❌ 不要到处 `clone()` page

**为什么？**
- page 是唯一资源，应该只有一个 owner
- 借用比克隆更高效、更安全

---

### 4️⃣ 参数爆炸 = 你还没找到"流程对象"

- ❌ 7 个参数的函数 → 说明缺少上下文封装
- ✅ 创建 `QuestionCtx`，打包所有上下文
- ✅ 创建 `QuestionFlow`，封装流程逻辑

**为什么？**
- 参数多 = 职责不清
- 上下文对象 = 职责明确

---

### 5️⃣ 抽象不是一次成型

- Struct 是阶段性假设，不是终极真理
- 如果发现 service 里有流程逻辑 → 提取到 Flow
- 如果发现参数太多 → 创建 Context
- 如果发现 Vec 出现在中间层 → 重新分层

**为什么？**
- 架构是演进的，不是设计出来的
- 代码会告诉你"哪里不对"

---

## 五、目录结构总览

```
src/
├── api/                      # API 层（HTTP 封装）
│   ├── llm.rs
│   └── tiku.rs
│
├── browser/                  # ① 基础设施层
│   ├── connection.rs
│   └── mod.rs
│
├── clients/                  # HTTP 客户端
│   ├── llm_client.rs
│   └── tiku_client.rs
│
├── models/                   # 数据模型
│   ├── question.rs
│   ├── subject.rs
│   └── mod.rs
│
├── services/                 # ② 业务能力层
│   ├── search_service.rs     # 搜索能力
│   ├── matching_service.rs   # 匹配能力
│   └── mod.rs
│
├── workflow/                 # ③ 流程层（核心）
│   ├── question_ctx.rs       # 上下文封装
│   ├── question_flow.rs      # 流程编排
│   └── mod.rs
│
├── processing.rs             # ④ 编排层（遍历试卷）
├── app.rs                    # 应用入口（批次控制）
├── config.rs                 # 配置
├── error.rs                  # 错误
├── logger.rs                 # 日志
├── utils/                    # 工具
└── main.rs                   # 程序入口
```

---

## 六、如何判断架构是否正确？

### 自检清单

**问自己 4 个问题：**

1. **能力在哪？**
   - → `services/` - 只处理单个 Question

2. **流程在哪？**
   - → `workflow/` - 包含所有 if/else

3. **批量在哪？**
   - → `processing.rs` / `app.rs` - 遍历 Vec

4. **稀缺资源在哪？**
   - → `browser/` - page 的 owner

**如果能回答这 4 个问题，架构就是清晰的。**

---

## 七、扩展示例

### 场景：增加 k14 → xueke 的兜底流程

**需求：**
- 先用 k14 搜索
- 如果 k14 为空，用 xueke 搜索
- 都为空，写 warn.txt

**只需修改 `QuestionFlow::run()`：**

```rust
impl QuestionFlow {
    pub async fn run(...) -> Result<ProcessResult> {
        // 1. 尝试 k14
        let k14_results = self.search_service.search_k14(page, stem).await?;
        
        if !k14_results.is_empty() {
            if let Some(best) = self.matching_service.find_best(&k14_results, question).await? {
                return self.submit(page, best, ctx).await;
            }
        }
        
        // 2. 兜底：xueke
        let xueke_results = self.search_service.search_xueke(page, stem, &ctx.subject_code).await?;
        
        if !xueke_results.is_empty() {
            if let Some(best) = self.matching_service.find_best(&xueke_results, question).await? {
                return self.submit(page, best, ctx).await;
            }
        }
        
        // 3. 都失败：warn.txt
        self.write_warn(ctx, question).await?;
        Ok(ProcessResult::Skipped)
    }
}
```

**不需要修改：**
- ❌ `SearchService`
- ❌ `MatchingService`
- ❌ `processing.rs`

**为什么？**
- 流程变化 = 只改 Flow
- 能力不变 = service 不动

---

## 八、总结

### 你现在掌握了什么？

1. **为什么 if/else 会失控**
   - 因为流程和能力混在一起

2. **为什么参数拆不干净**
   - 因为缺少上下文对象（QuestionCtx）

3. **为什么 Context 不能乱搞**
   - 因为 Context = 上下文，不是"大杂烩"

4. **为什么"功能"和"流程"必须分离**
   - 因为功能 = 可复用，流程 = 会变化

5. **为什么 Rust 的 enum / struct 是系统建模工具**
   - 因为类型 = 职责边界

---

### 记住这个模板

**遇到类似问题，问自己：**

- **能力？** → service
- **流程？** → flow
- **批量？** → orchestrator
- **稀缺资源？** → owner

**只要能回答这 4 个问题，代码就不会再炸。**

---

## 九、架构演进路径

### 阶段 1：意识到问题
- 参数太多
- if/else 太多
- 改一处，到处改

### 阶段 2：尝试分层
- 创建 service
- 提取函数
- 但还是很乱

### 阶段 3：找到"流程对象"
- 创建 `QuestionFlow`
- 创建 `QuestionCtx`
- 一切变得清晰

### 阶段 4：持续优化
- 发现新的流程 → 创建新的 Flow
- 发现新的能力 → 创建新的 Service
- 架构自然演进

---

## 十、最后的话

**这已经是 工程师 → 架构思维 的门槛了。**

你现在理解的不只是"怎么写代码"，而是：
- 如何识别职责
- 如何设计边界
- 如何让系统演进

**这比学 10 个框架更重要。**

---

**继续保持这种思维方式，你会走得更远。🚀**