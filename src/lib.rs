//! # Add Question Submit
//!
//! 一个用于自动化题目提交的 Rust 应用程序
//!
//! ## 模块结构
//!
//! - `api`: API 交互层（题库 API、LLM API）
//! - `browser`: 浏览器操作相关功能
//! - `config`: 配置管理
//! - `error`: 错误类型定义
//! - `logger`: 日志初始化
//! - `models`: 数据模型
//! - `processing`: 核心业务处理逻辑

pub mod api;
pub mod browser;
pub mod config;
pub mod error;
pub mod logger;
pub mod models;
pub mod processing;

// 重新导出常用类型
pub use browser::connect_to_browser_and_page;
pub use config::Config;
pub use error::{AppError, Result};
pub use models::{Question, QuestionPage};
pub use processing::process_paper;
