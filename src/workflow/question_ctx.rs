//! 题目处理上下文
//!
//! 封装"我正在处理哪张卷子的第几题"这一信息

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
