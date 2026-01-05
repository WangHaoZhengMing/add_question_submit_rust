//! API 模块
//!
//! 负责所有与外部系统的交互

pub mod llm;
pub mod tiku;

// 重新导出常用函数
pub use llm::{chat, find_best_match};
pub use tiku::{save_question, save_title, search_questions_xueku, submit_paper};
