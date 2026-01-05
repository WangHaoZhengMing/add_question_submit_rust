//! # Add Question Submit
//!
//! 一个用于自动化题目提交的 Rust 应用程序
//!
//! ## 架构设计
//!
//! 本系统采用严格的四层架构：
//!
//! ### ① 基础设施层（Infrastructure）
//! - `infrastructure/` - 持有稀缺资源（Page），只暴露能力
//! - `JsExecutor` - 唯一的 page owner，提供 eval() 能力
//!
//! ### ② 业务能力层（Services）
//! - `services/` - 描述"我能做什么"，只处理单个 Question
//! - `QuestionSearch` - k14 / xueke 搜索能力
//! - `LlmService` - LLM 判断能力
//! - `WarnWriter` - 写 warn.txt 能力
//!
//! ### ③ 流程层（Workflow）
//! - `workflow/` - 定义"一道题"的完整处理流程
//! - `QuestionCtx` - 上下文封装（paper_id + question_index）
//! - `QuestionFlow` - 流程编排（search → LLM → submit → warn）
//!
//! ### ④ 编排层（Orchestration）
//! - `processing.rs` - 遍历 Vec<Paper> 和 Vec<Question>
//! - `app.rs` - 应用入口，批量控制
//!
//! ## 模块结构

pub mod api;
pub mod app;
pub mod browser;
pub mod clients;
pub mod config;
pub mod error;
pub mod infrastructure;
pub mod logger;
pub mod models;
pub mod processing;
pub mod services;
pub mod utils;
pub mod workflow;

// 重新导出常用类型
pub use app::App;
pub use browser::connect_to_browser_and_page;
pub use config::Config;
pub use error::{AppError, Result};
pub use infrastructure::JsExecutor;
pub use models::{Question, QuestionPage};
pub use processing::process_paper;
pub use workflow::{ProcessResult, QuestionCtx, QuestionFlow};
