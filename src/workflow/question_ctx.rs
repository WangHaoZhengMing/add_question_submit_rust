//! 题目处理上下文
//!
//! 封装"我正在处理哪张卷子的第几题"这一信息

use std::fmt::Display;

/// 题目处理上下文
///
/// 包含处理单个题目所需的所有上下文信息
#[derive(Debug, Clone)]
pub struct QuestionCtx {
    /// 试卷ID
    pub paper_id: String,

    /// 试卷索引（仅用于日志显示）
    pub paper_index: usize,

    /// 题目在试卷中的索引（从1开始）
    pub question_index: usize,

    /// 科目代码
    pub subject_code: String,
}

impl QuestionCtx {
    /// 创建新的题目上下文
    pub fn new(
        paper_id: String,
        paper_index: usize,
        question_index: usize,
        subject_code: String,
    ) -> Self {
        Self {
            paper_id,
            paper_index,
            question_index,
            subject_code,
        }
    }
}

impl Display for QuestionCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[卷子 ID#{} 题目#{} 学科代码#{}]",
            self.paper_id, self.question_index, self.subject_code
        )
    }
}