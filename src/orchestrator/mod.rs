//! 编排层（Orchestration Layer）
//!
//! ## 职责
//!
//! 本层负责批量处理和流程调度，是整个系统的"指挥中心"。
//!
//! ## 模块划分
//!
//! ### `batch_processor` - 批量试卷处理器
//! - 管理应用生命周期（初始化、运行、清理）
//! - 批量加载试卷（Vec<QuestionPage>）
//! - 控制并发数量（Semaphore）
//! - 管理浏览器资源（Browser、JsExecutor）
//! - 输出全局统计信息
//!
//! ### `paper_processor` - 单个试卷处理器
//! - 遍历单个试卷的所有题目（Vec<Question>）
//! - 创建并复用 QuestionFlow
//! - 处理标题和普通题目
//! - 提交试卷
//! - 清理文件
//! - 输出单个试卷的统计信息
//!
//! ## 层次关系
//!
//! ```text
//! batch_processor (处理 Vec<Paper>)
//!     ↓
//! paper_processor (处理 Vec<Question>)
//!     ↓
//! workflow::QuestionFlow (处理单个 Question)
//!     ↓
//! services (能力层：search / llm / warn)
//!     ↓
//! infrastructure (基础设施：JsExecutor)
//! ```
//!
//! ## 设计原则
//!
//! 1. **单一职责**：batch_processor 管批量，paper_processor 管单个
//! 2. **资源隔离**：只有编排层持有 Browser 和 JsExecutor
//! 3. **向下依赖**：编排层 → workflow → services → infrastructure
//! 4. **无业务逻辑**：只做调度和统计，不做具体业务判断

pub mod batch_processor;
pub mod paper_processor;

// 重新导出主要类型
pub use batch_processor::App;
pub use paper_processor::{process_paper, QuestionStats};
