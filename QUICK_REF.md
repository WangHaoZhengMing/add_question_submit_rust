# 架构快速参考

## 🎯 一句话总结

**流程驱动的工作流系统**：在唯一 JS 执行环境上，按确定流程逐题执行"搜索 → 判断 → 兜底"。

---

## 📦 四层架构速查

| 层级 | 位置 | 职责 | 特征 |
|------|------|------|------|
| **① 基础设施** | `browser/` | 持有稀缺资源 | 唯一 page owner，不懂业务 |
| **② 业务能力** | `services/` | 提供能力 | 处理单个 Question，无 Vec，无流程 |
| **③ 流程编排** | `workflow/` | 定义流程 | 明确顺序/分支，有 if/else |
| **④ 批量编排** | `processing.rs` | 遍历执行 | 唯一可以有 Vec 的地方 |

---

## ⚡ 五条铁律

| # | 规则 | 说明 |
|---|------|------|
| 1️⃣ | **Vec 只在最外层** | service 里有 Vec？职责不清 |
| 2️⃣ | **复杂 if 在 Flow 里** | service 做决策？越界了 |
| 3️⃣ | **资源只有一个 owner** | page 到处 clone？设计有问题 |
| 4️⃣ | **参数爆炸 = 缺 Context** | 超过 4 个参数？需要封装上下文 |
| 5️⃣ | **抽象是演进的** | 代码会告诉你哪里不对 |

---

## 🔍 自检清单

问自己 4 个问题：

```
我现在写的是：
 □ 能力？      → services/
 □ 流程？      → workflow/
 □ 批量？      → orchestration/
 □ 稀缺资源？  → infrastructure/
```

**如果能回答，架构就是清晰的。**

---

## 📝 核心代码模板

### QuestionCtx（上下文封装）

```rust
pub struct QuestionCtx {
    pub paper_id: String,
    pub paper_index: usize,      // 仅日志
    pub question_index: usize,
    pub subject_code: String,
}
```

**作用：** 打包所有上下文，避免参数爆炸。

---

### QuestionFlow（流程对象）

```rust
pub struct QuestionFlow {
    search: SearchService,
    matching: MatchingService,
}

impl QuestionFlow {
    pub async fn run(
        &self,
        page: &Page,           // 借用稀缺资源
        question: &Question,   // 数据
        ctx: &QuestionCtx,     // 上下文
    ) -> Result<ProcessResult> {
        // 1. 搜索
        let results = self.search.search(...).await?;
        
        // 2. 判断分支
        if results.is_empty() {
            return Ok(ProcessResult::Skipped);
        }
        
        // 3. LLM 匹配
        let best = self.matching.find_best(...).await?;
        
        // 4. 提交
        self.submit(page, best, ctx).await
    }
}
```

**作用：** 明确流程的每一步，包含所有业务逻辑。

---

### Service（业务能力）

```rust
pub struct SearchService {
    client: TikuClient,
}

impl SearchService {
    // 只暴露能力，不做决策
    pub async fn search(
        &self,
        page: &Page,
        stem: &str,
        subject_code: &str,
    ) -> Result<Vec<SearchResult>> {
        // 只负责搜索
        // 不管"结果为空怎么办"
    }
}
```

**作用：** 只提供能力，不包含流程逻辑。

---

### Orchestration（批量编排）

```rust
pub async fn process_paper(
    page: &Page,
    paper: QuestionPage,
    config: &Config,
) -> Result<bool> {
    let flow = QuestionFlow::new(config);
    
    // 遍历题目
    for (index, question) in paper.stemlist.iter().enumerate() {
        let ctx = QuestionCtx::new(...);
        
        // 委托给流程对象
        match flow.run(page, question, &ctx).await {
            Ok(ProcessResult::Success) => { /* ... */ }
            Ok(ProcessResult::Skipped) => { /* ... */ }
            Err(e) => { /* ... */ }
        }
    }
    
    Ok(true)
}
```

**作用：** 遍历数据，但不写业务规则。

---

## 🚀 扩展示例

### 场景：增加 k14 → xueke 兜底

**只需修改 `QuestionFlow::run()`：**

```rust
pub async fn run(...) -> Result<ProcessResult> {
    // 1. 尝试 k14
    let k14_results = self.search.search_k14(...).await?;
    if !k14_results.is_empty() {
        if let Some(best) = self.match_best(&k14_results).await? {
            return self.submit(page, best, ctx).await;
        }
    }
    
    // 2. 兜底：xueke
    let xueke_results = self.search.search_xueke(...).await?;
    if !xueke_results.is_empty() {
        if let Some(best) = self.match_best(&xueke_results).await? {
            return self.submit(page, best, ctx).await;
        }
    }
    
    // 3. 写 warn.txt
    self.write_warn(ctx, question).await?;
    Ok(ProcessResult::Skipped)
}
```

**不需要改：** services、processing.rs、app.rs

**为什么？** 流程变化只改 Flow，能力不变 service 不动。

---

## 🎓 关键理解

### 为什么这样设计？

| 问题 | 原因 | 解决方案 |
|------|------|----------|
| 参数太多 | 缺少上下文封装 | 创建 `QuestionCtx` |
| if/else 到处都是 | 流程分散 | 创建 `QuestionFlow` |
| 改一处到处改 | 职责不清 | 分层：能力 vs 流程 |
| Vec 到处都是 | 没有边界 | Vec 只在最外层 |

---

## 📚 记住这个模板

遇到类似问题，问自己：

```
我现在写的代码是：

□ 能力？          → service
  ✓ 只处理单个
  ✓ 不做决策
  ✓ 可复用

□ 流程？          → flow
  ✓ 包含 if/else
  ✓ 定义顺序
  ✓ 会变化

□ 批量？          → orchestrator
  ✓ 遍历 Vec
  ✓ 统计/日志
  ✓ 不写业务规则

□ 稀缺资源？      → owner
  ✓ 唯一持有
  ✓ 只暴露能力
  ✓ 不懂业务
```

---

## 📊 改造前后对比

### 改造前 ❌

```rust
// 参数爆炸
async fn process_question(
    page: &Page,
    question: &Question,
    paper_id: &str,
    subject: &str,
    question_index: usize,
    paper_index: usize,
    config: &Config,
) -> Result<bool> {
    // 流程和能力混在一起
    let results = search(...).await?;
    if results.is_empty() {
        return Ok(false);
    }
    let best = find_best(...).await?;
    submit(...).await?;
    Ok(true)
}
```

**问题：**
- 7 个参数
- 流程逻辑分散
- 难以扩展

---

### 改造后 ✅

```rust
// 上下文封装
let ctx = QuestionCtx::new(
    paper_id.to_string(),
    paper_index,
    question_index,
    subject_code.clone(),
);

// 流程对象
let flow = QuestionFlow::new(config);
match flow.run(page, question, &ctx).await {
    Ok(ProcessResult::Success) => { /* ... */ }
    Ok(ProcessResult::Skipped) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

**优势：**
- 3 个参数
- 流程集中
- 易于扩展

---

## 🎯 最后的话

**这不是"写代码"，这是"设计系统"。**

你现在理解的是：
- ✅ 如何识别职责
- ✅ 如何设计边界
- ✅ 如何让系统演进

**这是工程师 → 架构师的门槛。**

继续保持这种思维方式，你会走得更远。🚀